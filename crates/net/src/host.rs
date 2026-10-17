//! Anfitrión (o servidor dedicado): acepta jugadores, simula sus controles y les manda el mundo.

use crate::lan::Beacon;
use crate::{CH_FAST, CH_RELIABLE, MAX_PLAYERS, Mode, NetEvent, PROTOCOL_ID, PlayerSnap, ToClient, ToServer, YouSnap, connection_config, pack, snapshot, unpack};
use arena_sim::game::Game;
use arena_sim::maps::Level;
use glam::Vec3;
use renet::{ClientId, RenetServer, ServerEvent};
use renet_netcode::{NetcodeServerTransport, ServerAuthentication, ServerConfig};
use std::collections::HashMap;
use std::net::{SocketAddr, UdpSocket};
use std::time::{Duration, Instant, SystemTime};

pub struct NetHost {
    server: RenetServer,
    transport: NetcodeServerTransport,
    /// Cliente de renet → número de jugador (1, 2, 3...; el 0 es el anfitrión que juega).
    pub clients: HashMap<ClientId, u8>,
    pub names: HashMap<u8, String>,
    pub mode: Mode,
    /// ¿El anfitrión juega? (no en el servidor dedicado)
    pub host_plays: bool,
    pub host_name: String,
    start: Instant,
    last: Instant,
    tick: u32,
    events: Vec<NetEvent>,
    beacon: Option<Beacon>,
    pub port: u16,
}

impl NetHost {
    pub fn new(port: u16, mode: Mode, host_name: &str, host_plays: bool, lan: bool) -> std::io::Result<Self> {
        let socket = UdpSocket::bind(SocketAddr::from(([0, 0, 0, 0], port)))?;
        let addr = socket.local_addr()?;
        let config = ServerConfig {
            current_time: SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap_or_default(),
            max_clients: MAX_PLAYERS,
            protocol_id: PROTOCOL_ID,
            public_addresses: vec![addr],
            authentication: ServerAuthentication::Unsecure,
        };
        let transport = NetcodeServerTransport::new(config, socket)?;
        let beacon = if lan { Beacon::new(host_name, addr.port()).ok() } else { None };
        Ok(Self { server: RenetServer::new(connection_config()), transport, clients: HashMap::new(), names: HashMap::new(), mode, host_plays, host_name: host_name.to_owned(), start: Instant::now(), last: Instant::now(), tick: 0, events: Vec::new(), beacon, port: addr.port() })
    }

    fn players(&self) -> Vec<(u8, String)> {
        let mut v: Vec<(u8, String)> = if self.host_plays { vec![(0, self.host_name.clone())] } else { Vec::new() };
        let mut rest: Vec<(u8, String)> = self.names.iter().map(|(k, v)| (*k, v.clone())).collect();
        rest.sort();
        v.extend(rest);
        v
    }

    /// Recibe conexiones, saludos y controles. Llamar una vez por paso, antes de simular.
    pub fn receive(&mut self, game: &mut Game, level: &Level) {
        let now = Instant::now();
        let dt = now - self.last;
        self.last = now;
        self.server.update(dt);
        let _ = self.transport.update(dt, &mut self.server);
        while let Some(ev) = self.server.get_event() {
            match ev {
                ServerEvent::ClientConnected { client_id } => {
                    let used: Vec<u8> = self.clients.values().copied().collect();
                    let id = (1..=MAX_PLAYERS as u8).find(|i| !used.contains(i)).unwrap_or(9);
                    self.clients.insert(client_id, id);
                    self.names.insert(id, format!("Jugador {id}"));
                    game.add_pawn(level, id, format!("Jugador {id}"));
                    let w = ToClient::Welcome { you: id, mode: self.mode, players: self.players() };
                    self.server.send_message(client_id, CH_RELIABLE, pack(&w));
                    self.broadcast_players();
                    game.banner("SE SUMÓ UN JUGADOR", format!("Jugador {id}"));
                }
                ServerEvent::ClientDisconnected { client_id, .. } => {
                    if let Some(id) = self.clients.remove(&client_id) {
                        let name = self.names.remove(&id).unwrap_or_default();
                        game.remove_pawn(id);
                        game.banner("SE FUE UN JUGADOR", name);
                        self.broadcast_players();
                    }
                }
            }
        }
        let ids: Vec<(ClientId, u8)> = self.clients.iter().map(|(c, i)| (*c, *i)).collect();
        for (cid, id) in ids {
            while let Some(b) = self.server.receive_message(cid, CH_RELIABLE) {
                if let Some(ToServer::Hello { name, .. }) = unpack::<ToServer>(&b) {
                    let name: String = name.chars().take(16).collect();
                    self.names.insert(id, name.clone());
                    if let Some(p) = game.pawns.iter_mut().find(|p| p.id == id) {
                        p.name = name;
                    }
                    self.broadcast_players();
                }
            }
            while let Some(b) = self.server.receive_message(cid, CH_FAST) {
                if let Some(ToServer::Input(inp)) = unpack::<ToServer>(&b) {
                    if let Some(p) = game.pawns.iter_mut().find(|p| p.id == id) {
                        p.input = inp;
                    }
                }
            }
        }
        if let Some(b) = self.beacon.as_mut() {
            b.tick(self.clients.len() + usize::from(self.host_plays), &self.mode.label());
        }
    }

    fn broadcast_players(&mut self) {
        let m = pack(&ToClient::Players(self.players()));
        self.server.broadcast_message(CH_RELIABLE, m);
    }

    /// Junta los eventos de este paso para mandarlos (llamar antes de que el anfitrión los consuma).
    pub fn collect(&mut self, game: &Game) {
        let offset = usize::from(self.host_plays);
        let ids: Vec<u8> = game.pawns.iter().map(|p| p.id).collect();
        let who = |k: u8| -> u8 {
            let k = k as usize;
            if k < offset { 0 } else { ids.get(k - offset).copied().unwrap_or(255) }
        };
        for e in &game.events {
            if let Some(n) = NetEvent::from_game(e, &who) {
                self.events.push(n);
            }
        }
    }

    /// Manda eventos (cada paso) y fotos (cada 3 pasos, 20 por segundo). `host` es el jugador del
    /// anfitrión para que los demás lo vean.
    pub fn send(&mut self, game: &mut Game, level: &Level, host: Option<PlayerSnap>) {
        self.tick += 1;
        if !self.events.is_empty() && !self.clients.is_empty() {
            let m = pack(&ToClient::Events(std::mem::take(&mut self.events)));
            self.server.broadcast_message(CH_RELIABLE, m);
        }
        self.events.clear();
        if self.tick % 3 == 0 {
            let mut others: Vec<PlayerSnap> = host.into_iter().collect();
            for p in &game.pawns {
                let s = &p.state;
                others.push(PlayerSnap { id: p.id, pos: s.pos, yaw: s.yaw, pitch: s.pitch, crouch: s.crouch_t, speed: s.horizontal_speed(), aim: p.input.aim, alive: p.alive(), weapon: p.arsenal.w as u8 });
            }
            let ids: Vec<(ClientId, u8)> = self.clients.iter().map(|(c, i)| (*c, *i)).collect();
            for (cid, id) in ids {
                let Some(p) = game.pawns.iter_mut().find(|p| p.id == id) else { continue };
                let you = YouSnap { pos: p.state.pos, vel: p.state.vel, hp: p.hp, max_hp: p.max_hp, ammo: p.arsenal.ammo, weapon: p.arsenal.w as u8, nades: p.arsenal.nades, down: p.down, marks: std::mem::take(&mut p.marks) };
                let at = p.state.pos;
                let players: Vec<PlayerSnap> = others.iter().filter(|o| o.id != id).cloned().collect();
                let snap = snapshot(game, level, at, self.tick, players, Some(you));
                self.server.send_message(cid, CH_FAST, pack(&ToClient::Snapshot(Box::new(snap))));
            }
        }
        self.transport.send_packets(&mut self.server);
    }

    /// Avisa a todos que la partida cambió (otra operación, otra arena).
    pub fn new_game(&mut self, mode: Mode) {
        self.mode = mode;
        self.server.broadcast_message(CH_RELIABLE, pack(&ToClient::NewGame(mode)));
    }

    pub fn player_count(&self) -> usize {
        self.clients.len()
    }

    pub fn rtt_ms(&self) -> Vec<(u8, f64)> {
        self.clients.iter().map(|(c, i)| (*i, self.server.rtt(*c) * 1000.0)).collect()
    }

    pub fn uptime(&self) -> Duration {
        self.start.elapsed()
    }

    pub fn shutdown(&mut self) {
        self.transport.disconnect_all(&mut self.server);
    }
}

/// Lo que ven los demás del jugador del anfitrión.
pub fn host_snap(pos: Vec3, yaw: f32, pitch: f32, crouch: f32, speed: f32, aim: bool, weapon: u8) -> PlayerSnap {
    PlayerSnap { id: 0, pos, yaw, pitch, crouch, speed, aim, alive: true, weapon }
}
