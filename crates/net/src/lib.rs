//! Online de Arena Cero: protocolo, fotos del mundo, anfitrión, cliente y LAN.
//!
//! - El **anfitrión** (o el **servidor dedicado**) corre la partida con autoridad: recibe los
//!   controles de cada jugador remoto, los simula en `arena_sim` y manda a cada cliente una foto
//!   comprimida del mundo cercano unas 20 veces por segundo, más los eventos (sonidos, efectos,
//!   golpes, radio) por un canal confiable.
//! - El **cliente** manda sus controles 60 veces por segundo, predice su propio movimiento con
//!   el mismo `arena_sim` y copia la foto en su `Game` para dibujar.
//! - En **LAN** el anfitrión se anuncia por broadcast UDP y los clientes ven la partida sola.

pub mod client;
pub mod host;
pub mod lan;

use arena_sim::campaign::OpResult;
use arena_sim::director::Mission;
use arena_sim::economy::{Decoy, Loot, Scout, Smoke};
use arena_sim::enemies::{Corpse, Enemy, OpState};
use arena_sim::game::{Game, GameEvent, Mark, Orb, Shot, Snd};
use arena_sim::maps::{Level, MapId};
use arena_sim::objectives::{Charge, Prompt};
use arena_sim::pawn::PawnInput;
use arena_sim::people::Person;
use arena_sim::vehicle::Vehicle;
use arena_sim::weapons::Nade;
use glam::Vec3;
use renet::{ChannelConfig, ConnectionConfig, SendType};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Cambia cuando cambia el protocolo: clientes y servidores de versiones distintas no se mezclan.
pub const PROTOCOL_ID: u64 = 0xA2E0_0004;
pub const PORT: u16 = 47800;
pub const LAN_PORT: u16 = 47801;
pub const MAX_PLAYERS: usize = 4;
/// Canal confiable (saludo, eventos) y no confiable (fotos del mundo, controles).
pub const CH_RELIABLE: u8 = 0;
pub const CH_FAST: u8 = 1;
/// Radio de interés: a cada cliente le llega lo que tiene a menos de esta distancia.
pub const INTEREST: f32 = 180.0;

pub fn connection_config() -> ConnectionConfig {
    let channels = vec![
        ChannelConfig { channel_id: CH_RELIABLE, max_memory_usage_bytes: 5 * 1024 * 1024, send_type: SendType::ReliableOrdered { resend_time: Duration::from_millis(200) } },
        ChannelConfig { channel_id: CH_FAST, max_memory_usage_bytes: 5 * 1024 * 1024, send_type: SendType::Unreliable },
    ];
    ConnectionConfig { available_bytes_per_tick: 120_000, server_channels_config: channels.clone(), client_channels_config: channels }
}

/// Qué se juega.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Mode {
    Campaign(u8),
    Free,
    /// Oleadas: índice en `MapId::ARENAS`.
    Waves(u8),
}

impl Mode {
    pub fn map(self) -> MapId {
        match self {
            Mode::Waves(i) => MapId::ARENAS[i as usize % MapId::ARENAS.len()],
            _ => MapId::Ciudad,
        }
    }
    /// Arma la partida igual en el servidor y en los clientes.
    pub fn new_game(self, level: &mut Level) -> Game {
        match self {
            Mode::Campaign(op) => Game::new_campaign(level, op as usize),
            _ => Game::new(level, 7),
        }
    }
    pub fn label(self) -> String {
        match self {
            Mode::Campaign(op) => format!("Campaña · operación {}", op + 1),
            Mode::Free => "Mundo libre".into(),
            Mode::Waves(_) => format!("Oleadas · {}", arena_sim::maps::build_level(self.map()).info.name),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ToServer {
    Hello { name: String, version: u64 },
    Input(PawnInput),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ToClient {
    /// Te aceptaron: tu número de jugador, qué se juega y quiénes están.
    Welcome { you: u8, mode: Mode, players: Vec<(u8, String)> },
    Players(Vec<(u8, String)>),
    Events(Vec<NetEvent>),
    Snapshot(Box<Snapshot>),
    /// Cambió la partida (otra operación, otra arena): volver a armarla.
    NewGame(Mode),
}

/// Otro jugador, para dibujarlo.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlayerSnap {
    pub id: u8,
    pub pos: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub crouch: f32,
    pub speed: f32,
    pub aim: bool,
    pub alive: bool,
    pub weapon: u8,
}

/// Lo tuyo según el servidor (para corregir la predicción y mostrar vida y munición).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct YouSnap {
    pub pos: Vec3,
    pub vel: Vec3,
    pub hp: f32,
    pub max_hp: f32,
    pub ammo: [u32; 4],
    pub weapon: u8,
    pub nades: u32,
    pub down: f32,
    pub marks: Vec<Mark>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CampSnap {
    pub step: usize,
    pub t: f32,
    pub objective: String,
    pub marks: Vec<Vec3>,
    pub limit_left: Option<f32>,
    pub progress: Option<f32>,
    pub deaths: u32,
    pub checkpoint: Vec3,
    pub result: Option<OpResult>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CitySnap {
    pub ops: Vec<OpState>,
    pub flags: [bool; 5],
    pub lives: u32,
    pub time: f32,
    pub charge: Option<Charge>,
    pub antenna_fall: Option<f32>,
    pub prompt: Option<Prompt>,
}

/// Foto del mundo cercano a un jugador.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Snapshot {
    pub tick: u32,
    pub enemies: Vec<Enemy>,
    pub corpses: Vec<Corpse>,
    pub shots: Vec<Shot>,
    pub nades: Vec<Nade>,
    pub orbs: Vec<Orb>,
    pub vehicles: Vec<Vehicle>,
    pub people: Vec<Person>,
    pub trains: Vec<f32>,
    pub loot: Vec<Loot>,
    pub smokes: Vec<Smoke>,
    pub decoys: Vec<Decoy>,
    pub scouts: Vec<Scout>,
    pub hour: f32,
    pub missions: Vec<Mission>,
    pub city: Option<CitySnap>,
    pub camp: Option<CampSnap>,
    pub score: u32,
    pub kills: u32,
    pub combo: u32,
    pub credits: u32,
    pub wave: u32,
    pub boss: Option<u32>,
    pub blackout: bool,
    pub players: Vec<PlayerSnap>,
    pub you: Option<YouSnap>,
}

/// `GameEvent` con dueño (sin `&'static str`), para mandar por la red.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum NetEvent {
    /// Golpe a un jugador (por número de jugador, no por índice de objetivo).
    Hurt { dmg: f32, from: Option<Vec3>, to: u8 },
    Sound(Snd),
    Burst { pos: Vec3, color: u32, n: u32, speed: f32, life: f32 },
    Tracer { a: Vec3, b: Vec3, color: u32, life: f32 },
    Boom { pos: Vec3, big: bool },
    Banner { title: String, sub: String },
    Num { pos: Vec3, value: f32, crit: bool },
    Score { points: u32, label: String },
    Credits { amount: u32, label: String },
    Radio { who: String, text: String },
    Impact { pos: Vec3, normal: Vec3 },
    Streak(u32),
    Unlock,
    WaveCleared,
    Respawn { pos: Vec3, yaw: f32 },
    Victory { bonus: u32 },
}

impl NetEvent {
    /// Traduce un evento de la partida. `who` pasa de índice de objetivo a número de jugador.
    pub fn from_game(e: &GameEvent, who: &dyn Fn(u8) -> u8) -> Option<NetEvent> {
        Some(match e {
            GameEvent::Hurt { dmg, from, to } => NetEvent::Hurt { dmg: *dmg, from: *from, to: who(*to) },
            GameEvent::Sound(s) => NetEvent::Sound(*s),
            GameEvent::Burst { pos, color, n, speed, life } => NetEvent::Burst { pos: *pos, color: *color, n: *n, speed: *speed, life: *life },
            GameEvent::Tracer { a, b, color, life } => NetEvent::Tracer { a: *a, b: *b, color: *color, life: *life },
            GameEvent::Boom { pos, big } => NetEvent::Boom { pos: *pos, big: *big },
            GameEvent::Banner { title, sub } => NetEvent::Banner { title: title.clone(), sub: sub.clone() },
            GameEvent::Num { pos, value, crit } => NetEvent::Num { pos: *pos, value: *value, crit: *crit },
            GameEvent::Score { points, label } => NetEvent::Score { points: *points, label: (*label).into() },
            GameEvent::Credits { amount, label } => NetEvent::Credits { amount: *amount, label: (*label).into() },
            GameEvent::Radio { who, text } => NetEvent::Radio { who: (*who).into(), text: text.clone() },
            GameEvent::Impact { pos, normal } => NetEvent::Impact { pos: *pos, normal: *normal },
            GameEvent::Streak(n) => NetEvent::Streak(*n),
            GameEvent::Unlock => NetEvent::Unlock,
            GameEvent::WaveCleared => NetEvent::WaveCleared,
            GameEvent::Respawn { pos, yaw } => NetEvent::Respawn { pos: *pos, yaw: *yaw },
            GameEvent::Victory { bonus } => NetEvent::Victory { bonus: *bonus },
            // lo del jugador local del anfitrión (sacudidas, curar, botín, mejoras) no viaja
            _ => return None,
        })
    }

    /// De vuelta a `GameEvent` en el cliente (los textos fijos se guardan una sola vez).
    pub fn to_game(&self, me: u8) -> Option<GameEvent> {
        Some(match self {
            NetEvent::Hurt { dmg, from, to } => {
                if *to != me {
                    return None;
                }
                GameEvent::Hurt { dmg: *dmg, from: *from, to: 0 }
            }
            NetEvent::Sound(s) => GameEvent::Sound(*s),
            NetEvent::Burst { pos, color, n, speed, life } => GameEvent::Burst { pos: *pos, color: *color, n: *n, speed: *speed, life: *life },
            NetEvent::Tracer { a, b, color, life } => GameEvent::Tracer { a: *a, b: *b, color: *color, life: *life },
            NetEvent::Boom { pos, big } => GameEvent::Boom { pos: *pos, big: *big },
            NetEvent::Banner { title, sub } => GameEvent::Banner { title: title.clone(), sub: sub.clone() },
            NetEvent::Num { pos, value, crit } => GameEvent::Num { pos: *pos, value: *value, crit: *crit },
            NetEvent::Score { points, label } => GameEvent::Score { points: *points, label: intern(label) },
            NetEvent::Credits { amount, label } => GameEvent::Credits { amount: *amount, label: intern(label) },
            NetEvent::Radio { who, text } => GameEvent::Radio { who: intern(who), text: text.clone() },
            NetEvent::Impact { pos, normal } => GameEvent::Impact { pos: *pos, normal: *normal },
            NetEvent::Streak(n) => GameEvent::Streak(*n),
            NetEvent::Unlock => GameEvent::Unlock,
            NetEvent::WaveCleared => GameEvent::WaveCleared,
            NetEvent::Respawn { pos, yaw } => GameEvent::Respawn { pos: *pos, yaw: *yaw },
            NetEvent::Victory { bonus } => GameEvent::Victory { bonus: *bonus },
        })
    }
}

/// Guarda cada texto una vez y devuelve una referencia estática (son pocos y se repiten).
fn intern(s: &str) -> &'static str {
    use std::collections::HashSet;
    use std::sync::{Mutex, OnceLock};
    static SET: OnceLock<Mutex<HashSet<&'static str>>> = OnceLock::new();
    let mut set = SET.get_or_init(|| Mutex::new(HashSet::new())).lock().unwrap();
    if let Some(v) = set.get(s) {
        return v;
    }
    let v: &'static str = Box::leak(s.to_owned().into_boxed_str());
    set.insert(v);
    v
}

/// Serializa y comprime.
pub fn pack<T: Serialize>(m: &T) -> Vec<u8> {
    let raw = postcard::to_allocvec(m).expect("serializar");
    lz4_flex::compress_prepend_size(&raw)
}

pub fn unpack<T: for<'de> Deserialize<'de>>(b: &[u8]) -> Option<T> {
    let raw = lz4_flex::decompress_size_prepended(b).ok()?;
    postcard::from_bytes(&raw).ok()
}

/// Foto del mundo para alguien parado en `at`.
pub fn snapshot(game: &Game, level: &Level, at: Vec3, tick: u32, players: Vec<PlayerSnap>, you: Option<YouSnap>) -> Snapshot {
    let near = |p: Vec3| (p.x - at.x).hypot(p.z - at.z) < INTEREST;
    let mut enemies: Vec<Enemy> = game.enemies.iter().filter(|e| near(e.pos) || e.hunting || game.boss == Some(e.id)).cloned().collect();
    for e in &mut enemies {
        // el camino de la navegación no hace falta para dibujar
        e.path.clear();
    }
    let city = game.city.as_ref().map(|r| CitySnap {
        ops: r.ops.clone(),
        flags: [r.powered, r.antenna, r.drones, r.intel, r.armory],
        lives: r.lives,
        time: r.time,
        charge: r.obj.charge,
        antenna_fall: r.obj.antenna_fall,
        prompt: None,
    });
    let camp = game.campaign.as_ref().map(|c| CampSnap { step: c.step, t: c.t, objective: c.objective.clone(), marks: c.marks.clone(), limit_left: c.limit_left, progress: c.progress, deaths: c.deaths, checkpoint: c.checkpoint, result: c.result.clone() });
    let _ = level;
    Snapshot {
        tick,
        enemies,
        corpses: game.corpses.iter().filter(|c| near(c.pos)).cloned().collect(),
        shots: game.shots.iter().filter(|s| near(s.pos)).cloned().collect(),
        nades: game.nades.iter().filter(|n| near(n.pos)).cloned().collect(),
        orbs: game.orbs.clone(),
        vehicles: game.vehicles.iter().map(|v| {
            let mut v = v.clone();
            // la ruta del piloto no hace falta; los lejanos van igual (son pocos y se ven de lejos)
            if let Some(ai) = v.ai.as_mut() {
                ai.route.clear();
            }
            v
        }).collect(),
        people: game.people.iter().filter(|p| near(p.pos)).cloned().collect(),
        trains: game.trains.iter().map(|t| t.t).collect(),
        loot: game.loot.iter().filter(|l| near(l.pos)).cloned().collect(),
        smokes: game.smokes.clone(),
        decoys: game.decoys.clone(),
        scouts: game.scouts.clone(),
        hour: game.director.hour,
        missions: game.director.missions.clone(),
        city,
        camp,
        score: game.score,
        kills: game.kills,
        combo: game.combo,
        credits: game.credits,
        wave: game.wave,
        boss: game.boss,
        blackout: game.blackout,
        players,
        you,
    }
}

/// Copia la foto en el `Game` del cliente (lo que dibujan las vistas).
pub fn apply(game: &mut Game, s: Snapshot) {
    game.enemies = s.enemies;
    game.corpses = s.corpses;
    game.shots = s.shots;
    game.nades = s.nades;
    game.orbs = s.orbs;
    // los autos: se copian los datos pero se conservan los colisionadores locales (no los hay en el cliente)
    for v in &mut game.vehicles {
        v.col = None;
    }
    game.vehicles = s.vehicles;
    game.people = s.people;
    for (t, v) in game.trains.iter_mut().zip(&s.trains) {
        t.t = *v;
    }
    game.loot = s.loot;
    game.smokes = s.smokes;
    game.decoys = s.decoys;
    game.scouts = s.scouts;
    game.director.hour = s.hour;
    game.director.missions = s.missions;
    if let (Some(run), Some(c)) = (game.city.as_mut(), s.city) {
        run.ops = c.ops;
        [run.powered, run.antenna, run.drones, run.intel, run.armory] = c.flags;
        run.lives = c.lives;
        run.time = c.time;
        run.obj.charge = c.charge;
        run.obj.antenna_fall = c.antenna_fall;
    }
    if let (Some(cp), Some(c)) = (game.campaign.as_mut(), s.camp) {
        cp.step = c.step;
        cp.t = c.t;
        cp.objective = c.objective;
        cp.marks = c.marks;
        cp.limit_left = c.limit_left;
        cp.progress = c.progress;
        cp.deaths = c.deaths;
        cp.checkpoint = c.checkpoint;
        cp.result = c.result;
    }
    game.score = s.score;
    game.kills = s.kills;
    game.combo = s.combo;
    game.credits = s.credits;
    game.wave = s.wave;
    game.boss = s.boss;
    game.blackout = s.blackout;
}

#[cfg(test)]
mod tests {
    use super::*;
    use arena_sim::maps::build_level;

    #[test]
    fn la_foto_de_la_ciudad_entra_comoda_en_la_red() {
        let mut l = build_level(MapId::Ciudad);
        let g = Game::new(&mut l, 7);
        let at = Vec3::new(0.0, 1.6, 0.0);
        let s = snapshot(&g, &l, at, 1, Vec::new(), None);
        let b = pack(&ToClient::Snapshot(Box::new(s)));
        eprintln!("foto comprimida: {} bytes", b.len());
        // 20 por segundo por cliente: menos de 40 KB cada una (< 0,8 MB/s)
        assert!(b.len() < 40_000, "{} bytes", b.len());
        let back: ToClient = unpack(&b).unwrap();
        let ToClient::Snapshot(s) = back else { panic!() };
        let mut g2 = Game::new(&mut build_level(MapId::Ciudad), 7);
        let n = s.enemies.len();
        apply(&mut g2, *s);
        assert_eq!(g2.enemies.len(), n);
    }

    #[test]
    fn los_eventos_van_y_vuelven() {
        let e = GameEvent::Score { points: 120, label: "BAJA" };
        let n = NetEvent::from_game(&e, &|k| k).unwrap();
        let b = pack(&ToClient::Events(vec![n]));
        let ToClient::Events(v) = unpack::<ToClient>(&b).unwrap() else { panic!() };
        assert_eq!(v[0].to_game(1), Some(e));
        // el golpe llega solo a quien va
        let h = NetEvent::Hurt { dmg: 4.0, from: None, to: 2 };
        assert!(h.to_game(1).is_none());
        assert!(h.to_game(2).is_some());
    }
}
