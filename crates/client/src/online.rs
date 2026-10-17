//! Online desde el juego: ser anfitrión (la partida corre acá y los amigos se suman) o unirse a
//! otro (anfitrión o servidor dedicado). El paso del jugador (`player.rs`) llama a estas funciones:
//! el anfitrión simula a los remotos; el cliente predice su propio movimiento, manda los controles
//! y copia el mundo que le llega.

use crate::audio::{S, Sounds};
use crate::game::GameRes;
use crate::game::UiFeed;
use crate::player::LocalPlayer;
use crate::world_render::LevelRes;
use arena_net::client::NetClient;
use arena_net::host::NetHost;
use arena_net::lan::Scanner;
use arena_net::{Mode, NetEvent, PlayerSnap, YouSnap, apply};
use arena_sim::game::{Game, GameEvent};
use arena_sim::maps::{Level, MapId};
use arena_sim::pawn::PawnInput;
use arena_sim::PlayerInput;
use arena_sim::player::EYE;
use bevy::prelude::*;
use std::collections::VecDeque;
use std::net::SocketAddr;

pub struct OnlinePlugin;

impl Plugin for OnlinePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Online>().add_systems(Update, (scan, host_new_game));
    }
}

#[derive(Default)]
pub enum Role {
    #[default]
    Off,
    Host(Box<NetHost>),
    Client(Box<Remote>),
}

#[derive(Resource, Default)]
pub struct Online {
    pub role: Role,
    /// Los otros jugadores (para dibujarlos).
    pub others: Vec<PlayerSnap>,
    /// Partidas encontradas en la red local (mientras el menú online está abierto).
    pub scanner: Option<Scanner>,
    /// Texto de estado para el menú ("Conectando…", "No se pudo abrir el puerto"…).
    pub status: String,
    /// Nombre propio.
    pub name: String,
    /// IP escrita a mano.
    pub ip: String,
}

impl Online {
    pub fn active(&self) -> bool {
        !matches!(self.role, Role::Off)
    }
    pub fn is_client(&self) -> bool {
        matches!(self.role, Role::Client(_))
    }
    /// Nombre de un jugador por número.
    pub fn name_of(&self, id: u8) -> String {
        match &self.role {
            Role::Host(h) => h.names.get(&id).cloned().unwrap_or_else(|| format!("Jugador {id}")),
            Role::Client(r) => r.net.players.iter().find(|p| p.0 == id).map(|p| p.1.clone()).unwrap_or_else(|| format!("Jugador {id}")),
            Role::Off => String::new(),
        }
    }

    /// Cliente caído esperando reaparecer.
    pub fn down(&self) -> bool {
        matches!(&self.role, Role::Client(r) if r.down())
    }
    pub fn name(&self) -> String {
        if self.name.trim().is_empty() { "Jugador".into() } else { self.name.trim().to_owned() }
    }

    /// Abre la partida actual a otros jugadores.
    pub fn host(&mut self, game: &Game, level: &Level) {
        let mode = mode_of(game, level);
        match NetHost::new(arena_net::PORT, mode, &self.name(), true, true) {
            Ok(h) => {
                self.status = format!("Partida abierta · puerto UDP {} · {}", h.port, local_ip().map(|i| i.to_string()).unwrap_or_default());
                self.role = Role::Host(Box::new(h));
            }
            Err(e) => self.status = format!("No se pudo abrir el puerto {}: {e}", arena_net::PORT),
        }
    }

    pub fn join(&mut self, addr: SocketAddr) {
        match NetClient::connect(addr, &self.name()) {
            Ok(c) => {
                self.status = format!("Conectando con {addr}…");
                self.role = Role::Client(Box::new(Remote::new(c, addr)));
            }
            Err(e) => self.status = format!("No se pudo conectar: {e}"),
        }
    }

    pub fn stop(&mut self) {
        match &mut self.role {
            Role::Host(h) => h.shutdown(),
            Role::Client(c) => c.net.disconnect(),
            Role::Off => {}
        }
        self.role = Role::Off;
        self.others.clear();
        self.status.clear();
    }
}

/// IP de esta máquina en la red local (para decírsela a los amigos).
pub fn local_ip() -> Option<std::net::IpAddr> {
    let s = std::net::UdpSocket::bind("0.0.0.0:0").ok()?;
    s.connect("8.8.8.8:80").ok()?;
    s.local_addr().ok().map(|a| a.ip())
}

/// Qué se está jugando ahora.
pub fn mode_of(game: &Game, level: &Level) -> Mode {
    if let Some(c) = &game.campaign {
        Mode::Campaign(c.op as u8)
    } else if level.info.city {
        Mode::Free
    } else {
        Mode::Waves(MapId::ARENAS.iter().position(|m| *m == level.info.id).unwrap_or(0) as u8)
    }
}

/// Cliente: lo que hace falta para predecir y corregir.
pub struct Remote {
    pub net: NetClient,
    pub addr: SocketAddr,
    seq: u32,
    /// (secuencia, posición predicha después de aplicar esos controles)
    hist: VecDeque<(u32, Vec3)>,
    sent: PawnInput,
    pub you: Option<YouSnap>,
    /// La partida local ya está armada con el modo del servidor.
    pub ready: bool,
    was_down: bool,
    /// Pasos sin corregir la posición (después de un viaje en subte, hasta que el servidor lo hace).
    grace: u32,
}

impl Remote {
    fn new(net: NetClient, addr: SocketAddr) -> Self {
        Self { net, addr, seq: 0, hist: VecDeque::new(), sent: PawnInput::default(), you: None, ready: false, was_down: false, grace: 0 }
    }

    pub fn me(&self) -> u8 {
        self.net.you.unwrap_or(255)
    }

    pub fn down(&self) -> bool {
        self.you.as_ref().is_some_and(|y| y.down > 0.0)
    }

    /// Viaje en subte: lo pide al servidor y se deja de corregir la posición un rato.
    pub fn ride(&mut self, to: usize) {
        self.net.ride(to as u8);
        self.hist.clear();
        self.grace = 60;
    }
}

/// Cliente, al principio del paso: recibe, copia el mundo, corrige la predicción y deja los
/// eventos del servidor en `game.events` (el paso los procesa como si fueran locales).
pub fn client_pre(r: &mut Remote, others: &mut Vec<PlayerSnap>, game: &mut Game, level: &mut Level, lp: &mut LocalPlayer, ui: &mut UiFeed, snd: &mut Sounds) {
    r.net.receive();
    if !r.ready {
        return;
    }
    let me = r.me();
    for e in std::mem::take(&mut r.net.events) {
        match e {
            NetEvent::Shot { id, weapon, at } if id != me => snd.play_at(S::Shot(weapon.min(2)), 0.3, at),
            NetEvent::Shot { .. } => {}
            e => {
                if let Some(g) = e.to_game(me) {
                    game.events.push(g);
                }
            }
        }
    }
    r.grace = r.grace.saturating_sub(1);
    let Some(s) = r.net.snapshot.take() else { return };
    *others = s.players.clone();
    let you = s.you.clone();
    let was_driving = game.driving.is_some();
    apply(game, level, s);
    let Some(you) = you else { return };
    // cartel de F, calculado acá sobre la copia del mundo
    let a = arena_sim::objectives::Actor { pos: lp.state.pos, feet: lp.state.feet(), f_held: false };
    let prompt = game.client_prompt(level, &a, game.driving.is_some(), you.hold);
    if let Some(run) = game.city.as_mut() {
        run.obj.prompt = prompt;
    }
    if r.grace > 0 {
        r.hist.clear();
        r.you = Some(you);
        return;
    }
    for m in &you.marks {
        game.events.push(GameEvent::Mark(*m));
    }
    // corrección: dónde decía la predicción que estaba en el paso que el servidor ya procesó
    let predicted = r.hist.iter().find(|h| h.0 == you.ack).map(|h| h.1);
    while r.hist.front().is_some_and(|h| h.0 <= you.ack) {
        r.hist.pop_front();
    }
    let err = you.pos - predicted.unwrap_or(lp.state.pos);
    // al bajarse del auto, directo al costado donde lo dejó el servidor
    let got_off = was_driving && game.driving.is_none();
    if got_off || err.length() > 3.0 || (predicted.is_none() && err.length() > 0.5) {
        lp.state.pos = you.pos;
        lp.state.vel = you.vel;
        lp.state.hook = None;
        lp.prev = you.pos;
        r.hist.clear();
    } else if err.length() > 0.02 {
        let c = err * 0.25;
        lp.state.pos += c;
        for h in &mut r.hist {
            h.1 += c;
        }
    }
    lp.hp = you.hp;
    lp.max_hp = you.max_hp;
    lp.arsenal.nades = you.nades;
    if lp.arsenal.reload <= 0.0 {
        for (a, b) in lp.arsenal.ammo.iter_mut().zip(you.ammo) {
            if a.abs_diff(b) >= 3 {
                *a = b;
            }
        }
    }
    let down = you.down > 0.0;
    if down && !r.was_down {
        ui.banner("CAÍSTE", "Reaparecés en unos segundos");
    }
    if !down && r.was_down {
        lp.arsenal.refill();
        lp.hurt_flash = 0.0;
    }
    r.was_down = down;
    r.you = Some(you);
}

/// Pulsaciones de este paso (para mandarlas como contadores).
pub struct Pulses {
    pub reload: bool,
    pub nade: bool,
    pub tag: bool,
    pub fire: bool,
    pub use_held: bool,
    pub light: bool,
    pub brake: bool,
}

/// Cliente, después de moverse: manda los controles y guarda la posición predicha.
pub fn client_send(r: &mut Remote, input: &PlayerInput, p: &Pulses, lp: &LocalPlayer) {
    if !r.ready {
        r.net.send_input(PawnInput::default());
        return;
    }
    r.seq = r.seq.wrapping_add(1);
    let s = &mut r.sent;
    let bump = |n: &mut u8, on: bool| {
        if on {
            *n = n.wrapping_add(1);
        }
    };
    bump(&mut s.jump_n, input.jump);
    bump(&mut s.dash_n, input.dash);
    bump(&mut s.hook_n, input.hook);
    bump(&mut s.reload_n, p.reload);
    bump(&mut s.nade_n, p.nade);
    bump(&mut s.tag_n, p.tag);
    s.forward = input.forward;
    s.right = input.right;
    s.yaw = input.yaw;
    s.pitch = input.pitch;
    s.sprint = input.sprint;
    s.crouch = input.crouch;
    s.aim = input.aim;
    s.fire = p.fire;
    s.use_held = p.use_held;
    s.light = p.light;
    s.brake = p.brake;
    s.weapon = lp.arsenal.w as u8;
    s.seq = r.seq;
    let inp = *s;
    r.net.send_input(inp);
    r.hist.push_back((r.seq, lp.state.pos));
    if r.hist.len() > 240 {
        r.hist.pop_front();
    }
}

/// Anfitrión: los remotos como los ven los demás.
pub fn pawn_snaps(game: &Game) -> Vec<PlayerSnap> {
    game.pawns.iter().map(|p| PlayerSnap { id: p.id, pos: p.state.pos, yaw: p.state.yaw, pitch: p.state.pitch, crouch: p.state.crouch_t, speed: p.state.horizontal_speed(), aim: p.input.aim, alive: p.alive(), weapon: p.arsenal.w as u8 }).collect()
}

pub fn host_self(lp: &LocalPlayer) -> PlayerSnap {
    let s = &lp.state;
    arena_net::host::host_snap(s.pos, s.yaw, s.pitch, s.crouch_t, s.horizontal_speed(), s.aim_t > 0.5, lp.arsenal.w as u8)
}

/// Cuando el anfitrión arranca otra partida (otra operación, mundo libre), avisa y vuelve a sumar
/// a los remotos.
fn host_new_game(mut net: ResMut<Online>, mut game: ResMut<GameRes>, level: Res<LevelRes>, mut seen: Local<u32>) {
    let g = game.1;
    if *seen == g {
        return;
    }
    *seen = g;
    let Role::Host(h) = &mut net.role else { return };
    let mode = mode_of(&game.0, &level.0);
    if game.0.pawns.is_empty() && h.player_count() > 0 {
        let mut ids: Vec<(u8, String)> = h.names.iter().map(|(k, v)| (*k, v.clone())).collect();
        ids.sort();
        for (id, n) in ids {
            game.0.add_pawn(&level.0, id, n);
        }
    }
    if h.mode != mode {
        h.new_game(mode);
    }
}

/// Lista de partidas en la red local, mientras haga falta.
fn scan(mut net: ResMut<Online>) {
    if let Some(s) = net.scanner.as_mut() {
        s.poll();
    }
}

/// Dónde aparece el jugador local al unirse (el servidor lo corrige con la primera foto).
pub fn join_spot(game: &Game, level: &Level) -> (Vec3, f32) {
    if let Some((at, yaw)) = game.campaign_start() {
        return (at, yaw);
    }
    let s = level.info.start;
    (Vec3::new(s[0], EYE, s[1]), 0.0)
}
