//! Partida: junta enemigos, proyectiles, granadas, explosiones, puntaje, oleadas y el estado de
//! la ciudad (alarmas por puesto, refuerzos, cadáveres). Es lo que después corre el servidor.
//! El jugador entra como `Target` y el daño sale como eventos `Hurt` que aplica quien lo tenga.

use crate::city::{CityData, OutpostId};
use crate::enemies::*;
use crate::maps::Level;
use crate::nav::NavGrid;
use crate::rng::Rng;
use crate::weapons::{Explosion, Nade, Pellet, WeaponEvent, enemy_nade, update_nades};
use crate::world::{ColliderId, World};
use glam::Vec3;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mode {
    City,
    Waves,
}

/// Sonidos que pide la partida (el cliente los traduce a sus efectos).
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Snd {
    /// Disparo enemigo: distancia (volumen) y de dónde viene (paneo).
    EShot { dist: f32, big: bool, at: Vec3 },
    Whiz,
    Enemy { dist: f32, at: Vec3 },
    Boss,
    Siren,
    AlertBeep,
    Kill,
    Boom,
    Hit,
    Crit,
    Ricochet,
    Pickup,
    Plant,
    Takedown,
    Objective,
    Beep,
    Tag,
    Crash,
    Decoy,
    Toss,
    /// Grito de un civil.
    Scream,
    /// Tren que entra en la estación.
    Train,
    /// Bip de radio (operadora).
    Radio,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mark {
    Hit,
    Crit,
    Kill,
}

#[derive(Clone, Debug, PartialEq)]
pub enum GameEvent {
    /// Daño a un jugador (`to`: índice en la lista de objetivos del paso).
    Hurt { dmg: f32, from: Option<Vec3>, to: u8 },
    Sound(Snd),
    Burst { pos: Vec3, color: u32, n: u32, speed: f32, life: f32 },
    Tracer { a: Vec3, b: Vec3, color: u32, life: f32 },
    Boom { pos: Vec3, big: bool },
    Shake(f32),
    Banner { title: String, sub: String },
    Mark(Mark),
    /// Número de daño flotante (×10, como en la web).
    Num { pos: Vec3, value: f32, crit: bool },
    Heal(f32),
    Overdrive,
    /// Reabastecimiento: integridad, munición y granadas al máximo.
    Refill,
    /// Arsenal forzado: todas las armas desbloqueadas y llenas.
    Unlock,
    /// Distrito liberado.
    Victory { bonus: u32 },
    /// Elegir una mejora.
    OfferPerk,
    /// Oleada despejada (+1 granada).
    WaveCleared,
    /// El jugador se bajó del auto (el cliente lo ubica al costado).
    ExitVehicle,
    /// Créditos ganados.
    Credits { amount: u32, label: &'static str },
    /// Caja con munición o granadas: lo aplica el cliente.
    Loot(crate::economy::LootKind),
    /// Puntos ganados (para el "+120" junto a la mira).
    Score { points: u32, label: &'static str },
    /// Línea de radio de la campaña (quién habla y qué dice).
    Radio { who: &'static str, text: String },
    /// Campaña: volviste al último punto de control (el cliente restaura vida y munición).
    Respawn { pos: Vec3, yaw: f32 },
    /// Bala del jugador contra una pared o el piso: punto y normal (marca de impacto).
    Impact { pos: Vec3, normal: Vec3 },
    /// Racha: bajas seguidas (2 = doble, 3 = triple...).
    Streak(u32),
}

/// Qué arma `Game::new_with`: el mundo libre trae todo; la campaña despierta cada puesto cuando
/// empieza su operación y no tiene patrullas, botín ni misiones del director.
#[derive(Clone, Copy, Debug)]
pub struct Setup {
    pub outposts: bool,
    pub camps: bool,
    pub surveillance: bool,
    pub patrols: usize,
    pub traffic: usize,
    pub people: usize,
    pub loot: usize,
    pub director: bool,
}

impl Setup {
    pub const FREE: Setup = Setup { outposts: true, camps: true, surveillance: true, patrols: 6, traffic: 16, people: 80, loot: 80, director: true };
    pub const CAMPAIGN: Setup = Setup { outposts: false, camps: false, surveillance: false, patrols: 0, traffic: 16, people: 80, loot: 0, director: false };
}

#[derive(Clone, Debug)]
pub struct Shot {
    pub pos: Vec3,
    pub vel: Vec3,
    pub life: f32,
    pub dmg: f32,
    pub from: Vec3,
    pub heavy: bool,
}

#[derive(Clone, Debug)]
pub struct Orb {
    pub pos: Vec3,
    pub life: f32,
    pub od: bool,
}

/// Estado de la ciudad durante la partida.
#[derive(Debug)]
pub struct CityRun {
    pub ops: Vec<OpState>,
    pub powered: bool,
    pub antenna: bool,
    pub drones: bool,
    pub intel: bool,
    pub armory: bool,
    pub lives: u32,
    pub time: f32,
    pub done: Vec<OutpostId>,
    pub reinforce_t: f32,
    pub stealth_kills: u32,
    turret_cols: Vec<(u32, ColliderId)>,
    pub obj: crate::objectives::ObjState,
}

pub struct Game {
    pub mode: Mode,
    pub enemies: Vec<Enemy>,
    next_id: u32,
    pub shots: Vec<Shot>,
    /// Granadas del jugador y de los soldados.
    pub nades: Vec<Nade>,
    pub orbs: Vec<Orb>,
    pub corpses: Vec<Corpse>,
    pub city: Option<CityRun>,
    pub score: u32,
    pub kills: u32,
    pub combo: u32,
    pub combo_t: f32,
    pub wave: u32,
    pub spawn_queue: Vec<DroneKind>,
    spawn_timer: f32,
    pub intermission: f32,
    pub boss: Option<u32>,
    pub rng: Rng,
    pub time: f32,
    nav: Vec<NavGrid>,
    pub events: Vec<GameEvent>,
    /// Daño acumulado por enemigo en este paso, para el número flotante.
    nums: Vec<(u32, f32, bool)>,
    pub vehicles: Vec<crate::vehicle::Vehicle>,
    /// Auto que maneja el jugador.
    pub driving: Option<usize>,
    engine_noise_t: f32,
    pub credits: u32,
    pub loot: Vec<crate::economy::Loot>,
    /// Cantidad de humo, señuelo y dron explorador.
    pub gadgets: [u32; 3],
    pub gadget_sel: usize,
    pub smokes: Vec<crate::economy::Smoke>,
    pub decoys: Vec<crate::economy::Decoy>,
    pub scouts: Vec<crate::economy::Scout>,
    pub director: crate::director::Director,
    pub people: Vec<crate::people::Person>,
    /// Trenes del subte (uno por estación).
    pub trains: Vec<crate::subway::Train>,
    /// Campaña de operaciones (None en el mundo libre y en oleadas).
    pub campaign: Option<crate::campaign::Campaign>,
    /// Críticos y derribos del jugador (para el rango de la campaña).
    pub crits: u32,
    pub takedowns: u32,
    /// Apagón: los faroles no se prenden (lo lee el cliente).
    pub blackout: bool,
    /// Dentro de una explosión (los que mueren salen despedidos).
    blasting: bool,
    /// Jugadores de este paso (cooperativo: uno por jugador; en solitario, uno).
    pub targets: Vec<Target>,
    /// Jugadores remotos (online).
    pub pawns: Vec<crate::pawn::Pawn>,
}

impl Game {
    pub fn new(level: &mut Level, seed: u32) -> Self {
        Self::new_with(level, seed, Setup::FREE)
    }

    pub fn new_with(level: &mut Level, seed: u32, setup: Setup) -> Self {
        let mode = if level.info.city { Mode::City } else { Mode::Waves };
        let mut g = Game {
            mode,
            enemies: Vec::new(),
            next_id: 1,
            shots: Vec::new(),
            nades: Vec::new(),
            orbs: Vec::new(),
            corpses: Vec::new(),
            city: None,
            score: 0,
            kills: 0,
            combo: 0,
            combo_t: 0.0,
            wave: 0,
            spawn_queue: Vec::new(),
            spawn_timer: 0.0,
            intermission: 1.2,
            boss: None,
            rng: Rng::new(seed),
            time: 0.0,
            nav: Vec::new(),
            events: Vec::new(),
            nums: Vec::new(),
            vehicles: Vec::new(),
            driving: None,
            engine_noise_t: 0.0,
            credits: 0,
            loot: Vec::new(),
            gadgets: [1, 1, 0],
            gadget_sel: 0,
            smokes: Vec::new(),
            decoys: Vec::new(),
            scouts: Vec::new(),
            director: Default::default(),
            people: Vec::new(),
            trains: Vec::new(),
            campaign: None,
            crits: 0,
            takedowns: 0,
            blackout: false,
            blasting: false,
            targets: Vec::new(),
            pawns: Vec::new(),
        };
        if let Some(c) = level.city.as_ref() {
            let n_st = c.stations.len();
            g.init_city(c.clone(), &mut level.world, &setup);
            g.place_loot(level, setup.loot);
            g.spawn_patrols(level, setup.patrols);
            g.spawn_traffic(level, setup.traffic);
            g.spawn_people(level, setup.people);
            g.init_trains(n_st);
            g.director.enabled = setup.director;
        }
        g
    }

    /// Oleadas: la partida sigue en otra arena con el mismo puntaje, bajas y número de oleada;
    /// la próxima oleada arranca después de una pausa corta.
    pub fn next_arena(level: &mut Level, old: &Game) -> Game {
        let mut g = Game::new(level, 7 + old.wave);
        g.score = old.score;
        g.kills = old.kills;
        g.wave = old.wave;
        g.crits = old.crits;
        g.intermission = 3.0;
        g
    }

    pub(crate) fn next_id(&mut self) -> u32 {
        self.id()
    }

    fn id(&mut self) -> u32 {
        self.next_id += 1;
        self.next_id
    }

    fn ev(&mut self, e: GameEvent) {
        self.events.push(e);
    }

    fn burst(&mut self, pos: Vec3, color: u32, n: u32, speed: f32, life: f32) {
        self.events.push(GameEvent::Burst { pos, color, n, speed, life });
    }

    pub fn banner(&mut self, t: impl Into<String>, s: impl Into<String>) {
        self.events.push(GameEvent::Banner { title: t.into(), sub: s.into() });
    }

    /* ---------- ciudad ---------- */

    fn init_city(&mut self, c: CityData, world: &mut World, setup: &Setup) {
        let mut run = CityRun { ops: vec![OpState::default(); c.outposts.len()], powered: true, antenna: true, drones: true, intel: false, armory: false, lives: 3, time: 0.0, done: Vec::new(), reinforce_t: 20.0, stealth_kills: 0, turret_cols: Vec::new(), obj: Default::default() };
        for op in &c.outposts {
            self.nav.push(NavGrid::build(world, op.cx, op.cz, 36.0, 0.16));
        }
        self.city = Some(run);
        for (oi, op) in c.outposts.iter().enumerate() {
            let awake = if op.camp { setup.camps } else { setup.outposts };
            if awake {
                self.spawn_outpost(oi, op, world);
            } else {
                self.city.as_mut().unwrap().ops[oi].asleep = true;
            }
        }
        run = self.city.take().unwrap();
        for &(p, yaw, color) in &c.vehicles {
            let id = self.id();
            self.vehicles.push(crate::vehicle::Vehicle::new(id, p, yaw, color, world));
        }
        for (i, route) in c.drone_routes.iter().enumerate() {
            if !setup.surveillance {
                break;
            }
            let id = self.id();
            let p = route[i % route.len()];
            let mut e = new_drone(id, DroneKind::Normal, Vec3::new(p[0], 16.0, p[1]), 0, &mut self.rng);
            e.surv = Some(Surv { route: route.clone(), idx: i % route.len(), det: 0.0 });
            e.hp = 5.0;
            e.max_hp = 5.0;
            e.value = 200;
            self.enemies.push(e);
        }
        self.city = Some(run);
    }

    /// Despierta un puesto: aparecen sus soldados y torretas (la campaña lo hace al empezar cada
    /// operación; el mundo libre, al armar la ciudad).
    pub fn spawn_outpost(&mut self, oi: usize, op: &crate::city::Outpost, world: &mut World) {
        for u in &op.units {
            let id = self.id();
            let e = new_soldier(id, oi, u, &mut self.rng);
            self.enemies.push(e);
        }
        for t in &op.turrets {
            let id = self.id();
            let e = new_turret(id, oi, t, &mut self.rng);
            let col = world.add_collider(t.x, t.z, 0.9, 1.4, 0.9, 0.16);
            if let Some(run) = self.city.as_mut() {
                run.turret_cols.push((id, col));
            }
            self.enemies.push(e);
        }
        if let Some(run) = self.city.as_mut() {
            run.ops[oi].asleep = false;
        }
    }

    /// Alarma en un puesto: todas sus unidades entran en combate y llegan refuerzos.
    fn raise_alarm(&mut self, op: usize, known: Option<Vec3>, level: &Level) {
        let Some(run) = self.city.as_mut() else { return };
        let alive = self.enemies.iter().any(|u| u.op == Some(op) && u.alive());
        if run.ops[op].done && !alive {
            return;
        }
        let fresh = run.ops[op].alarm <= 0.0;
        run.ops[op].alarm = 32.0;
        run.ops[op].ever_alarm = true;
        let antenna = run.antenna;
        for u in self.enemies.iter_mut().filter(|u| u.op == Some(op) && u.alive()) {
            if let Some(k) = known {
                u.last_seen = k;
                if u.st != Alert::Combat {
                    u.see_t = 1.5;
                }
            }
            if u.is_soldier() && u.react <= 0.0 {
                u.react = 0.4 + self.rng.f() * 0.5;
            }
            u.st = Alert::Combat;
        }
        if fresh {
            self.ev(GameEvent::Sound(Snd::Siren));
            let full = level.city.as_ref().map_or("", |c| c.outposts[op].full).to_uppercase();
            self.banner(format!("ALERTA · {full}"), if antenna { "Pidieron refuerzos por radio" } else { "La antena está caída: no llegan refuerzos" });
        }
    }

    pub(crate) fn alarm_public(&mut self, op: usize, known: Option<Vec3>, level: &Level) {
        self.raise_alarm(op, known, level);
    }

    /// Ruido (disparos, explosiones): los soldados tranquilos que lo oyen van a buscar.
    pub fn make_noise(&mut self, pos: Vec3, radius: f32) {
        self.scare_people(pos, radius * 0.8);
        if self.mode != Mode::City {
            return;
        }
        for e in self.enemies.iter_mut().filter(|e| e.is_soldier() && e.alive()) {
            let d = e.pos.distance(pos);
            if d > radius {
                continue;
            }
            if e.st == Alert::Combat {
                if d < radius * 0.6 {
                    e.last_seen = pos;
                }
            } else {
                e.st = Alert::Search;
                e.target = Some(pos + Vec3::new((self.rng.f() - 0.5) * 6.0, 0.0, (self.rng.f() - 0.5) * 6.0));
                e.det = e.det.max(0.5);
                e.search_t = 9.0;
            }
        }
    }

    /// Nivel de alerta global: 0 oculto, 1 buscando, 2 detectado.
    pub fn city_alert(&self) -> u8 {
        let Some(run) = &self.city else { return 0 };
        if run.ops.iter().any(|o| o.alarm > 0.0) {
            2
        } else if self.enemies.iter().any(|e| e.is_soldier() && e.st == Alert::Search) {
            1
        } else {
            0
        }
    }

    /// Nivel de la música dinámica (0..3).
    pub fn music_level(&self, hp_frac: f32) -> u8 {
        match self.mode {
            Mode::City => {
                let cmd_combat = self.enemies.iter().any(|e| e.kind == Kind::Soldier(SoldierKind::Commander) && e.alive() && e.st == Alert::Combat);
                if cmd_combat { 3 } else { self.city_alert() }
            }
            Mode::Waves => {
                if self.enemies.is_empty() && self.spawn_queue.is_empty() {
                    0
                } else if self.boss.is_some() {
                    3
                } else if self.combo >= 3 || hp_frac < 0.35 || self.enemies.len() > 7 {
                    2
                } else {
                    1
                }
            }
        }
    }

    /// Saca del mundo lo que agregó la partida (colisionadores de torretas) antes de descartarla.
    pub fn teardown(&mut self, world: &mut World) {
        for v in &mut self.vehicles {
            if let Some(c) = v.col.take() {
                world.remove_collider(c);
            }
        }
        if let Some(c) = self.city.as_mut() {
            for (_, col) in c.turret_cols.drain(..) {
                world.remove_collider(col);
            }
        }
        self.teardown_trains(world);
    }

    /// Al caer en la ciudad: se calman las alarmas, se van los cazadores y los soldados vuelven a su puesto.
    pub fn on_player_down(&mut self) {
        self.driving = None;
        self.shots.clear();
        self.nades.retain(|n| !n.enemy);
        if let Some(run) = self.city.as_mut() {
            for o in &mut run.ops {
                o.alarm = 0.0;
            }
        }
        self.enemies.retain(|e| !e.hunting);
        for e in &mut self.enemies {
            if e.is_soldier() {
                e.st = Alert::Patrol;
                e.det = 0.0;
                e.see_t = 99.0;
                e.cover = None;
                e.pos = e.anchor;
                e.path.clear();
            }
            if e.is_turret() {
                e.st = Alert::Patrol;
                e.det = 0.0;
            }
        }
    }

    /* ---------- oleadas ---------- */

    fn start_wave(&mut self) {
        self.wave += 1;
        let w = self.wave;
        let boss_wave = w % 5 == 0;
        let n = ((3.0 + w as f32 * 1.5) * if boss_wave { 0.5 } else { 1.0 }).round() as u32;
        let heavies = if boss_wave { 0 } else { w / 3 };
        let kamis = if w >= 2 { w / 2 } else { 0 };
        let mut q: Vec<DroneKind> = vec![DroneKind::Normal; n as usize];
        for _ in 0..kamis {
            let i = 1 + self.rng.idx(q.len());
            q.insert(i.min(q.len()), DroneKind::Kami);
        }
        for _ in 0..heavies {
            let i = self.rng.idx(q.len());
            q.insert(i, DroneKind::Heavy);
        }
        if boss_wave {
            q.insert(0, DroneKind::Boss);
        }
        self.spawn_queue = q;
        self.spawn_timer = 0.4;
        let mut extra = Vec::new();
        if kamis > 0 {
            extra.push(format!("{kamis} kamikaze{}", if kamis > 1 { "s" } else { "" }));
        }
        if heavies > 0 {
            extra.push(format!("{heavies} pesado{}", if heavies > 1 { "s" } else { "" }));
        }
        if boss_wave {
            let mut parts = vec!["Jefe entrante".to_string(), format!("{n} drones")];
            parts.extend(extra);
            self.banner(format!("OLEADA {w} · COLOSO"), parts.join(" · "));
        } else {
            let mut parts = vec![format!("{n} drones")];
            parts.extend(extra);
            self.banner(format!("OLEADA {w}"), parts.join(" · "));
        }
    }

    fn spawn_drone(&mut self, d: DroneKind, world: &World, t: &Target) {
        let a = world.a;
        let mut x = 0.0;
        let mut z = 0.0;
        for _ in 0..60 {
            x = self.rng.signed() * (a - 6.0);
            z = self.rng.signed() * (a - 6.0);
            let dd = (x - t.pos.x).hypot(z - t.pos.z);
            if dd >= (a * 0.6).min(22.0) && dd <= 48.0 && world.box_at(Vec3::new(x, 3.0, z)).is_none() {
                break;
            }
        }
        let id = self.id();
        let e = new_drone(id, d, Vec3::new(x, if d == DroneKind::Boss { 12.0 } else { 9.0 }, z), self.wave, &mut self.rng);
        let color = drone_color(d);
        self.burst(e.pos, color, if d == DroneKind::Boss { 40 } else { 14 }, if d == DroneKind::Boss { 9.0 } else { 5.0 }, 0.5);
        if d == DroneKind::Boss {
            self.boss = Some(id);
        }
        self.enemies.push(e);
    }

    /* ---------- daño ---------- */

    /// Daña al enemigo `i`. Devuelve si lo mató. `dir` empuja a los drones.
    pub fn damage(&mut self, i: usize, dmg: f32, dir: Option<Vec3>, crit: bool, body: Vec3, level: &mut Level) -> bool {
        let e = &mut self.enemies[i];
        if !e.alive() {
            return false;
        }
        let shown = dmg.min(e.hp);
        if let Some(n) = self.nums.iter_mut().find(|n| n.0 == e.id) {
            n.1 += shown;
            n.2 |= crit;
        } else {
            self.nums.push((e.id, shown, crit));
        }
        e.hp -= dmg;
        e.flash = 0.07;
        e.hit_dir = dir.map_or(Vec3::ZERO, |d| d.with_y(0.0).normalize_or_zero());
        e.hit_force = if self.blasting { 7.0 } else if crit { 2.6 } else if dir.is_some() { 1.3 } else { 0.0 };
        if (e.is_soldier() || e.is_turret()) && e.hp > 0.0 {
            // herido: entra en combate y avisa a su puesto
            e.st = Alert::Combat;
            e.last_seen = body;
            e.see_t = 0.0;
            if e.is_soldier() {
                e.react = e.react.min(0.3);
            }
            if let Some(op) = e.op {
                self.raise_alarm(op, Some(body), level);
            }
        }
        let e = &mut self.enemies[i];
        if let Some(d) = dir {
            let k = match e.drone() {
                Some(DroneKind::Boss) => 0.15,
                Some(DroneKind::Heavy) => 0.8,
                Some(_) => 2.5,
                None => 0.0,
            };
            e.vel += d * k * dmg.min(3.0);
        }
        if e.hp <= 0.0 {
            self.kill(i, level);
            return true;
        }
        false
    }

    /// Racha de bajas en la ciudad: cada baja dentro de 4 s de la anterior suma y da 50 por nivel.
    fn streak(&mut self) -> u32 {
        self.combo = if self.combo_t > 0.0 { (self.combo + 1).min(9) } else { 1 };
        self.combo_t = 4.0;
        if self.combo >= 2 {
            self.events.push(GameEvent::Streak(self.combo));
        }
        50 * (self.combo - 1)
    }

    fn kill(&mut self, i: usize, level: &mut Level) {
        let e = self.enemies[i].clone();
        self.kills += 1;
        match e.kind {
            Kind::Soldier(sk) => {
                let stealth = e.op.and_then(|o| self.city.as_ref().map(|c| c.ops[o].alarm <= 0.0)).unwrap_or(false);
                let pts = e.value + if stealth { 60 } else { 0 } + self.streak();
                self.score += pts;
                let label = if stealth { "BAJA SILENCIOSA" } else if sk == SoldierKind::Commander { "COMANDANTE" } else { "BAJA" };
                self.events.push(GameEvent::Score { points: pts, label });
                if self.city.is_some() {
                    self.add_credits(crate::economy::bounty(label), label);
                }
                if stealth {
                    if let Some(c) = self.city.as_mut() {
                        c.stealth_kills += 1;
                    }
                }
                let cid = self.id();
                let fall = if self.rng.next() < 0.5 { 1.0 } else { -1.0 };
                let style = if e.hit_force >= 5.0 { 2 } else if e.hit_force <= 0.0 { 1 } else { 0 };
                self.corpses.push(Corpse { id: cid, pos: e.pos, yaw: e.yaw, fall, dir: e.hit_dir, push: e.hit_force, style, t: 0.0, seen: false, reported: false, kind: sk });
                if self.corpses.len() > 30 {
                    self.corpses.remove(0);
                }
                if sk == SoldierKind::Commander {
                    if let Some(op) = e.op {
                        self.enemies[i].hp = 0.0;
                        self.complete_objective(op, level);
                    }
                }
            }
            Kind::Turret => {
                let bonus = self.streak();
                self.score += e.value + bonus;
                self.events.push(GameEvent::Score { points: e.value + bonus, label: "TORRETA" });
                self.add_credits(50, "TORRETA");
                if let Some(c) = self.city.as_mut() {
                    if let Some(k) = c.turret_cols.iter().position(|t| t.0 == e.id) {
                        let (_, col) = c.turret_cols.remove(k);
                        level.world.remove_collider(col);
                    }
                }
                let p = e.pos + Vec3::Y * 1.9;
                self.burst(p, 0xffb02e, 30, 8.0, 0.7);
                self.burst(p, 0x2a2a2a, 14, 4.0, 1.2);
                self.ev(GameEvent::Sound(Snd::Boom));
                self.make_noise(e.pos, 60.0);
            }
            Kind::Drone(dk) => {
                self.burst(e.pos, drone_color(dk), if dk == DroneKind::Heavy { 50 } else { 28 }, 9.0, 0.8);
                self.burst(e.pos, 0x1b2226, 12, 6.0, 1.0);
                self.ev(GameEvent::Sound(Snd::Kill));
                self.combo = if self.combo_t > 0.0 { (self.combo + 1).min(5) } else { 1 };
                self.combo_t = 3.0;
                self.score += e.value * self.combo;
                self.events.push(GameEvent::Score { points: e.value * self.combo, label: if self.combo > 1 { "COMBO" } else { "DRON" } });
            }
        }
        self.enemies[i].hp = 0.0;
        if e.drone() == Some(DroneKind::Kami) {
            // reacción en cadena contra otros drones
            self.explode(Explosion { pos: e.pos, radius: 3.2, max_dmg: 3.0, self_dmg: 0.0, big: false, enemy: false }, None, level);
        }
        // puesto despejado: queda como punto de reabastecimiento
        if let Some(op) = e.op.filter(|_| e.is_soldier()) {
            let soldiers_left = self.enemies.iter().any(|u| u.op == Some(op) && u.alive() && u.is_soldier());
            let newly = match self.city.as_mut() {
                Some(run) if !run.ops[op].cleared && !soldiers_left => {
                    run.ops[op].cleared = true;
                    true
                }
                _ => false,
            };
            if newly {
                self.add_credits(250, "PUESTO DESPEJADO");
                let full = level.city.as_ref().map_or("", |c| c.outposts[op].full).to_string();
                self.banner("PUESTO DESPEJADO", format!("{full}: ahora es punto de reabastecimiento"));
            }
        }
        if self.mode == Mode::Waves {
            if let Some(dk) = e.drone() {
                if dk == DroneKind::Boss {
                    self.boss = None;
                    self.explode(Explosion { pos: e.pos, radius: 6.0, max_dmg: 4.0, self_dmg: 0.0, big: true, enemy: false }, None, level);
                    self.orbs.push(Orb { pos: e.pos, life: 20.0, od: false });
                    self.orbs.push(Orb { pos: e.pos + Vec3::X * 2.5, life: 20.0, od: true });
                    self.banner("COLOSO DERRIBADO", format!("+{} puntos", e.value * self.combo));
                } else if dk == DroneKind::Heavy || self.rng.next() < if dk == DroneKind::Kami { 0.1 } else { 0.3 } {
                    self.orbs.push(Orb { pos: e.pos, life: 20.0, od: false });
                } else if self.rng.next() < 0.05 {
                    self.orbs.push(Orb { pos: e.pos, life: 20.0, od: true });
                }
            }
        }
    }

    /// Explosión: daña enemigos según la distancia y, si toca, al jugador.
    pub fn explode(&mut self, x: Explosion, body: Option<Vec3>, level: &mut Level) {
        self.make_noise(x.pos, if x.big { 80.0 } else { 45.0 });
        self.burst(x.pos, 0xffb02e, if x.big { 60 } else { 30 }, if x.big { 12.0 } else { 8.0 }, 0.7);
        self.burst(x.pos, 0xff3d5a, if x.big { 24 } else { 12 }, 9.0, 0.6);
        self.burst(x.pos, 0x2a2a2a, if x.big { 20 } else { 8 }, 5.0, 1.2);
        self.ev(GameEvent::Boom { pos: x.pos, big: x.big });
        if x.big {
            self.ev(GameEvent::Sound(Snd::Boom));
        }
        let b = body.unwrap_or(Vec3::splat(1e6));
        self.blasting = true;
        for i in (0..self.enemies.len()).rev() {
            let e = &self.enemies[i];
            if !e.alive() {
                continue;
            }
            let d = e.pos.distance(x.pos);
            if d < x.radius + e.radius {
                let dmg = (x.max_dmg * (1.0 - d / (x.radius + e.radius))).round().max(1.0);
                let dir = (e.pos - x.pos).normalize_or_zero();
                self.damage(i, dmg, Some(dir), false, b, level);
            }
        }
        self.blasting = false;
        for pi in 0..self.people.len() {
            if self.people[pi].alive && self.people[pi].pos.distance(x.pos) < x.radius * 0.8 {
                self.hurt_person(pi);
            }
        }
        for vi in 0..self.vehicles.len() {
            let v = &self.vehicles[vi];
            let d = (v.pos + Vec3::Y).distance(x.pos);
            if !v.wrecked && d < x.radius + 2.0 {
                let dmg = x.max_dmg * 3.0 * (1.0 - d / (x.radius + 2.0));
                self.damage_vehicle(vi, dmg, level, body.unwrap_or(x.pos));
            }
        }
        if body.is_some() {
            // a todos los jugadores (en solitario, el único)
            let bodies: Vec<Vec3> = if self.targets.is_empty() { body.into_iter().collect() } else { self.targets.iter().map(|t| t.body).collect() };
            for (k, b) in bodies.into_iter().enumerate() {
                let pd = (b - Vec3::Y * 0.3).distance(x.pos);
                if k == 0 {
                    self.ev(GameEvent::Shake((if x.big { 0.9 } else { 0.4 }) * (1.0 - pd / 20.0).max(0.0)));
                }
                if x.self_dmg > 0.0 && pd < x.radius {
                    self.ev(GameEvent::Hurt { dmg: (x.self_dmg * (1.0 - pd / x.radius)).round(), from: Some(x.pos), to: k as u8 });
                }
            }
        }
    }

    /// Disparo hitscan del jugador. `muzzle` es de dónde salen las trazadoras.
    #[allow(clippy::too_many_arguments)]
    pub fn player_fire(&mut self, pellets: &[Pellet], dmg: f32, pierce: bool, crit_mul: f32, muzzle: Vec3, body: Vec3, level: &mut Level) {
        let (mut any_hit, mut any_kill, mut any_crit) = (false, false, false);
        let tracer_color = if pierce { 0x5fd8ff } else { 0xffc66a };
        for (k, p) in pellets.iter().enumerate() {
            let mut wt = level.world.ray(p.origin, p.dir, 120.0);
            // los autos tapan las balas (y el riel les pega igual)
            let mut car_hit: Option<(usize, f32)> = None;
            for (vi, v) in self.vehicles.iter().enumerate() {
                if Some(vi) == self.driving || (v.pos - p.origin).length() > wt + 4.0 {
                    continue;
                }
                if let Some(t) = ray_obb(p.origin, p.dir, v.pos + Vec3::Y * 0.8, Vec3::new(0.95, 0.8, 2.15), v.yaw) {
                    if t < wt && car_hit.is_none_or(|c| t < c.1) {
                        car_hit = Some((vi, t));
                    }
                }
            }
            if let Some((vi, t)) = car_hit {
                wt = t;
                let hp_before = self.vehicles[vi].hp;
                self.damage_vehicle(vi, dmg, level, body);
                if hp_before > 0.0 && !self.vehicles[vi].wrecked {
                    any_hit = true;
                }
            }
            let mut world_hit = car_hit.is_none();
            if let Some((pi, t)) = self.ray_people(p.origin, p.dir, wt) {
                let first_enemy = ray_enemies(&self.enemies, p.origin, p.dir, wt, p.origin).first().map_or(f32::MAX, |h| h.dist);
                if t < first_enemy {
                    self.hurt_person(pi);
                    wt = t;
                    world_hit = false;
                }
            }
            let hits = ray_enemies(&self.enemies, p.origin, p.dir, wt, p.origin);
            let mut end: Option<Vec3> = None;
            let mut seen: Vec<usize> = Vec::new();
            for h in hits {
                if h.zone == HitZone::Shield {
                    end = Some(h.point);
                    self.burst(h.point, 0x9fe9ff, 6, 4.0, 0.25);
                    self.ev(GameEvent::Sound(Snd::Ricochet));
                    break;
                }
                if seen.contains(&h.enemy) || !self.enemies[h.enemy].alive() {
                    continue;
                }
                seen.push(h.enemy);
                let crit = h.zone == HitZone::Crit;
                any_hit = true;
                any_crit |= crit;
                self.burst(h.point, if crit { 0xffb02e } else if pierce { 0x5fd8ff } else { 0xfff0e0 }, if crit { 8 } else if pierce { 14 } else { 4 }, 5.0, 0.25);
                if self.damage(h.enemy, dmg * if crit { crit_mul } else { 1.0 }, Some(p.dir), crit, body, level) {
                    any_kill = true;
                }
                if !pierce {
                    end = Some(h.point);
                    break;
                }
            }
            if end.is_none() && wt < 120.0 {
                let b = p.origin + p.dir * wt;
                self.burst(b, if pierce { 0x5fd8ff } else { 0xffb02e }, if pierce { 10 } else { 3 }, 4.0, 0.3);
                end = Some(b);
                if world_hit {
                    let normal = surface_normal(&level.world, b, p.dir);
                    self.ev(GameEvent::Impact { pos: b, normal });
                }
            }
            let end = end.unwrap_or(p.origin + p.dir * 80.0);
            if pierce || k % 3 == 0 {
                self.ev(GameEvent::Tracer { a: muzzle, b: end, color: tracer_color, life: if pierce { 0.18 } else { 0.05 } });
            }
        }
        if any_crit {
            self.crits += 1;
        }
        if any_kill {
            self.ev(GameEvent::Mark(Mark::Kill));
        } else if any_hit {
            self.ev(GameEvent::Sound(if any_crit { Snd::Crit } else { Snd::Hit }));
            self.ev(GameEvent::Mark(if any_crit { Mark::Crit } else { Mark::Hit }));
        }
    }

    /// Reacciona a los eventos de armas del jugador que le importan a la partida.
    pub fn weapon_events(&mut self, evs: &[WeaponEvent], crit_mul: f32, muzzle: Vec3, body: Vec3, level: &mut Level) {
        for e in evs {
            match e {
                WeaponEvent::Fire { pellets, dmg, pierce, .. } => self.player_fire(pellets, *dmg, *pierce, crit_mul, muzzle, body, level),
                WeaponEvent::Noise { pos, radius } => self.make_noise(*pos, *radius),
                _ => {}
            }
        }
    }

    /* ---------- paso ---------- */

    pub fn step(&mut self, level: &mut Level, t: &Target, hp_full: bool, dt: f32) {
        self.step_multi(level, &[*t], &[hp_full], dt);
    }

    /// Un paso con varios jugadores (cooperativo). `ts[0]` es el anfitrión.
    pub fn step_multi(&mut self, level: &mut Level, ts: &[Target], hp_full: &[bool], dt: f32) {
        self.targets = ts.to_vec();
        let t = &ts[0];
        self.time += dt;
        if self.combo_t > 0.0 {
            self.combo_t -= dt;
            if self.combo_t <= 0.0 {
                self.combo = 0;
            }
        }
        match self.mode {
            Mode::City => self.step_city(level, t, dt),
            Mode::Waves => self.step_waves(level, t, dt),
        }
        self.step_enemies(level, ts, dt);
        self.step_shots(level, ts, dt);
        // granadas (las del jugador las mete el arsenal en `nades`)
        let mut wev = Vec::new();
        let enemies = &self.enemies;
        update_nades(&mut self.nades, &level.world, dt, |p| enemies.iter().any(|e| e.alive() && e.pos.distance(p) < e.radius + 0.35), &mut wev);
        for e in wev {
            match e {
                WeaponEvent::Explosion(x) => self.explode(x, Some(t.body), level),
                WeaponEvent::Gadget { kind: crate::weapons::NadeKind::Smoke, pos } => {
                    self.smokes.push(crate::economy::Smoke { pos, r: 1.5, life: 12.0 });
                    self.events.push(GameEvent::Sound(Snd::Toss));
                }
                WeaponEvent::Gadget { pos, .. } => self.decoys.push(crate::economy::Decoy { pos, life: 10.0, beep: 0.0 }),
                _ => {}
            }
        }
        self.step_gadgets(dt);
        self.step_people(level, t.pos, dt);
        self.step_director(level, t.pos, dt);
        self.step_trains(level, dt);
        self.step_enemy_cars(level, dt);
        // cadáveres que caen: el envión del golpe los arrastra (sin atravesar paredes)
        for c in &mut self.corpses {
            if c.t < 0.7 {
                c.t += dt;
                if c.push > 0.0 && c.dir != Vec3::ZERO {
                    let step = c.dir * c.push * dt;
                    let next = c.pos + step;
                    if level.world.box_at(next + Vec3::Y * 0.6).is_none() {
                        c.pos.x = next.x;
                        c.pos.z = next.z;
                    } else {
                        c.push = 0.0;
                    }
                    c.push = (c.push - dt * if c.style == 2 { 9.0 } else { 6.0 }).max(0.0);
                }
                let k = (c.t / 0.5).min(1.0);
                c.pos.y = (c.pos.y - dt * 0.6).max(level.world.ground_at(c.pos.x, c.pos.z, c.pos.y + 0.2) + 0.15 * k);
            }
        }
        // orbes
        let mut picked = Vec::new();
        for (i, o) in self.orbs.iter_mut().enumerate() {
            o.life -= dt;
            let near = ts.iter().zip(hp_full).any(|(t, full)| (o.pos.x - t.pos.x).hypot(o.pos.z - t.pos.z) < 1.4 && (o.od || !full));
            if near {
                picked.push(i);
            }
        }
        for &i in picked.iter().rev() {
            let o = self.orbs.remove(i);
            self.ev(if o.od { GameEvent::Overdrive } else { GameEvent::Heal(25.0) });
            self.burst(o.pos, if o.od { 0xffb02e } else { 0x5dffb0 }, if o.od { 20 } else { 14 }, 5.0, 0.5);
            if o.od {
                self.banner("SOBRECARGA", "Cadencia y daño aumentados por 8 s");
            }
            self.ev(GameEvent::Sound(Snd::Pickup));
        }
        self.orbs.retain(|o| o.life > 0.0);
        // números de daño
        for (id, v, crit) in std::mem::take(&mut self.nums) {
            if let Some(e) = self.enemies.iter().find(|e| e.id == id) {
                self.events.push(GameEvent::Num { pos: e.pos + Vec3::Y * (e.radius + 0.3 + if e.is_soldier() { 1.6 } else { 0.0 }), value: v, crit });
            }
        }
        // limpiar muertos
        if let Some(b) = self.boss {
            if !self.enemies.iter().any(|e| e.id == b && e.alive()) {
                self.boss = None;
            }
        }
        self.enemies.retain(|e| e.alive());
    }

    fn step_city(&mut self, level: &mut Level, _t: &Target, dt: f32) {
        let Some(run) = self.city.as_mut() else { return };
        run.time += dt;
        // alarma por puesto: se apaga si nadie te ve durante 32 s
        let mut lost = Vec::new();
        for (oi, o) in run.ops.iter_mut().enumerate() {
            if o.alarm > 0.0 {
                o.ever_alarm = true;
                let seen = self.enemies.iter().any(|u| u.op == Some(oi) && u.alive() && u.visible && u.st == Alert::Combat);
                if !seen {
                    o.alarm -= dt;
                }
                if o.alarm <= 0.0 {
                    lost.push(oi);
                }
            }
        }
        for oi in lost {
            for u in self.enemies.iter_mut().filter(|u| u.op == Some(oi) && u.alive() && u.is_soldier() && u.st == Alert::Combat) {
                u.st = Alert::Search;
                u.target = Some(u.last_seen);
                u.search_t = 14.0;
                u.det = 0.5;
                u.cover = None;
            }
            let full = level.city.as_ref().map_or("", |c| c.outposts[oi].full).to_string();
            self.banner("BÚSQUEDA", format!("{full}: te perdieron el rastro"));
        }
        // refuerzos: drones cazadores cada 28 s mientras haya un puesto en alerta
        let run = self.city.as_mut().unwrap();
        let hot = run.ops.iter().position(|o| o.alarm > 0.0);
        if let (Some(op), true, true) = (hot, run.antenna, run.drones) {
            run.reinforce_t -= dt;
            if run.reinforce_t <= 0.0 {
                run.reinforce_t = 28.0;
                let hunters = self.enemies.iter().filter(|e| e.hunting).count();
                let (cx, cz) = level.city.as_ref().map(|c| (c.outposts[op].cx, c.outposts[op].cz)).unwrap_or((0.0, 0.0));
                for i in 0..2 {
                    if hunters + i >= 4 {
                        break;
                    }
                    let id = self.id();
                    let mut e = new_drone(id, DroneKind::Normal, Vec3::new(cx + (self.rng.f() - 0.5) * 30.0, 26.0, cz + (self.rng.f() - 0.5) * 30.0), 0, &mut self.rng);
                    e.hunting = true;
                    self.enemies.push(e);
                }
                self.banner("REFUERZOS", "Drones en camino. Destruí la antena para cortarlos.");
            }
        }
    }

    fn step_waves(&mut self, level: &mut Level, t: &Target, dt: f32) {
        if !self.spawn_queue.is_empty() {
            self.spawn_timer -= dt;
            let cap = (6 + self.wave / 2).min(12) as usize;
            if self.spawn_timer <= 0.0 && self.enemies.len() < cap {
                let d = self.spawn_queue.remove(0);
                self.spawn_drone(d, &level.world, t);
                self.spawn_timer = (0.9 - self.wave as f32 * 0.04).max(0.35);
            }
        } else if self.enemies.is_empty() {
            if self.intermission <= 0.0 {
                if self.wave > 0 {
                    let bonus = self.wave * 250;
                    self.score += bonus;
                    self.ev(GameEvent::Heal(20.0));
                    self.ev(GameEvent::WaveCleared);
                    self.banner("DESPEJADO", format!("+{bonus} bonus · +1 granada"));
                    self.ev(GameEvent::OfferPerk);
                }
                self.intermission = if self.wave > 0 { 3.2 } else { 1.2 };
            }
            self.intermission -= dt;
            if self.intermission <= 0.0 {
                self.start_wave();
                self.intermission = 0.0;
            }
        }
    }

    fn step_enemies(&mut self, level: &mut Level, ts: &[Target], dt: f32) {
        let outposts: &[crate::city::Outpost] = level.city.as_ref().map_or(&[], |c| &c.outposts);
        let smokes = self.smoke_spheres();
        let night = self.director.night();
        // cada enemigo se ocupa del jugador más cercano
        let pick: Vec<usize> = self.enemies.iter().map(|e| (0..ts.len()).min_by(|&a, &b| ts[a].pos.distance_squared(e.pos).total_cmp(&ts[b].pos.distance_squared(e.pos))).unwrap_or(0)).collect();
        let powered = self.city.as_ref().is_none_or(|c| c.powered);
        let lim = level.world.lim();
        let drone_pos: Vec<Vec3> = self.enemies.iter().map(|e| if e.drone().is_some() && e.surv.is_none() { e.pos } else { Vec3::splat(1e6) }).collect();
        let mut shooters = vec![0u32; outposts.len()];
        for e in &self.enemies {
            if let (Some(o), true) = (e.op, e.is_soldier() && e.alive() && e.burst > 0) {
                shooters[o] += 1;
            }
        }
        let mut ev = Vec::new();
        // a qué jugador va cada evento de la IA
        let mut who: Vec<u8> = Vec::new();
        let mut empty_ops: Vec<OpState> = Vec::new();
        let ops = self.city.as_mut().map_or(&mut empty_ops, |c| &mut c.ops);
        for i in 0..self.enemies.len() {
            let e = &mut self.enemies[i];
            if !e.alive() {
                continue;
            }
            let k = pick[i];
            let ctx = Ctx { world: &level.world, outposts, nav: &self.nav, t: &ts[k], time: self.time, dt, smokes: &smokes, night };
            let before = ev.len();
            match e.kind {
                Kind::Soldier(_) => {
                    let mine = e.op.map_or(0, |o| shooters[o]).saturating_sub(u32::from(e.burst > 0));
                    update_soldier(e, &ctx, ops, &mut self.corpses, mine, &mut self.rng, &mut ev);
                }
                Kind::Turret => update_turret(e, &ctx, powered, &mut self.rng, &mut ev),
                Kind::Drone(_) if e.surv.is_some() => {
                    update_surv(e, &ctx, &mut ev);
                }
                Kind::Drone(_) => update_drone(e, i, &ctx, &drone_pos, self.wave, lim, &mut self.rng, &mut ev),
            }
            who.resize(ev.len(), k as u8);
            let _ = before;
        }
        for (a, pk) in ev.into_iter().zip(who) {
            let t = &ts[pk as usize];
            match a {
                AiEvent::Shot { from, to, hit, dmg, big } => {
                    let d = from.distance(t.body);
                    if hit {
                        self.ev(GameEvent::Hurt { dmg, from: Some(from), to: pk });
                    } else if d < 30.0 {
                        self.ev(GameEvent::Sound(Snd::Whiz));
                    }
                    self.ev(GameEvent::Tracer { a: from, b: to, color: 0xffc66a, life: 0.05 });
                    self.ev(GameEvent::Sound(Snd::EShot { dist: d, big, at: from }));
                    self.burst(from, 0xffd27a, 2, 2.0, 0.08);
                }
                AiEvent::DroneFire { from, vel, dmg, heavy } => self.shots.push(Shot { pos: from, vel, life: 5.0, dmg, from, heavy }),
                AiEvent::DroneSfx { pos, boss } => self.ev(GameEvent::Sound(if boss { Snd::Boss } else { Snd::Enemy { dist: pos.distance(t.body), at: pos } })),
                AiEvent::Nade { from, target } => self.nades.push(enemy_nade(from, target)),
                AiEvent::Alarm { op, pos } => self.raise_alarm(op, pos, level),
                AiEvent::Spotted => {
                    let near = level.city.as_ref().and_then(|c| {
                        let done = self.city.as_ref().map(|r| r.ops.iter().map(|o| o.done || o.asleep).collect::<Vec<_>>()).unwrap_or_default();
                        c.outposts.iter().enumerate().filter(|(i, _)| !done.get(*i).copied().unwrap_or(false)).min_by(|a, b| (a.1.cx - t.pos.x).hypot(a.1.cz - t.pos.z).total_cmp(&(b.1.cx - t.pos.x).hypot(b.1.cz - t.pos.z))).map(|(i, _)| i)
                    });
                    if let Some(op) = near {
                        self.raise_alarm(op, Some(t.body), level);
                    }
                    self.ev(GameEvent::Sound(Snd::AlertBeep));
                }
                AiEvent::KamiBoom { idx } => {
                    if let Some(e) = self.enemies.get_mut(idx) {
                        if e.alive() {
                            e.hp = 0.0;
                            let p = e.pos;
                            self.explode(Explosion { pos: p, radius: 2.5, max_dmg: 2.0, self_dmg: 0.0, big: true, enemy: true }, Some(t.body), level);
                            self.ev(GameEvent::Hurt { dmg: 14.0, from: Some(p), to: pk });
                        }
                    }
                }
            }
        }
    }

    fn step_shots(&mut self, level: &Level, ts: &[Target], dt: f32) {
        let mut i = 0;
        while i < self.shots.len() {
            let s = &mut self.shots[i];
            s.pos += s.vel * dt;
            s.life -= dt;
            let (pos, from, dmg) = (s.pos, s.from, s.dmg);
            if s.life <= 0.0 {
                self.shots.swap_remove(i);
                continue;
            }
            if let Some(k) = ts.iter().position(|t| pos.distance_squared(t.body) < 0.45) {
                self.burst(pos, 0xff3d5a, 8, 4.0, 0.3);
                self.ev(GameEvent::Hurt { dmg, from: Some(from), to: k as u8 });
                self.shots.swap_remove(i);
                continue;
            }
            if pos.y < 0.0 || level.world.box_at(pos).is_some() {
                self.burst(pos, 0xff8060, 8, 4.0, 0.3);
                self.shots.swap_remove(i);
                continue;
            }
            i += 1;
        }
    }
}

/// Normal de la cara de caja que tocó un rayo: se mete un poco adentro y se prueba salir por
/// cada eje; gana el eje más alineado con el rayo entre los que dejan afuera.
pub fn surface_normal(world: &World, hit: Vec3, dir: Vec3) -> Vec3 {
    let inside = hit + dir * 0.03;
    if world.box_at(inside).is_none() && hit.y > 0.05 {
        return -dir;
    }
    if hit.y <= 0.05 && dir.y < 0.0 {
        return Vec3::Y;
    }
    let mut best: Option<(f32, Vec3)> = None;
    for axis in [Vec3::X, Vec3::Y, Vec3::Z] {
        let d = dir.dot(axis);
        if d.abs() < 1e-3 {
            continue;
        }
        let n = -axis * d.signum();
        if world.box_at(inside + n * 0.08).is_none() && best.is_none_or(|b| d.abs() > b.0) {
            best = Some((d.abs(), n));
        }
    }
    best.map_or(-dir, |b| b.1)
}

pub fn drone_color(d: DroneKind) -> u32 {
    match d {
        DroneKind::Kami => 0xff6a2e,
        DroneKind::Heavy => 0xffb02e,
        _ => 0xff3d5a,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::maps::{MapId, build_level};

    #[test]
    fn la_siguiente_arena_sigue_la_cuenta() {
        let mut l = build_level(MapId::Deposito);
        let mut g = Game::new(&mut l, 7);
        g.wave = 3;
        g.score = 1234;
        let mut l2 = build_level(MapId::Fundicion);
        let mut g2 = Game::next_arena(&mut l2, &g);
        assert_eq!((g2.wave, g2.score), (3, 1234));
        let t = target(0.0, 0.0);
        for _ in 0..(4.0 / crate::TICK) as usize {
            g2.step(&mut l2, &t, true, crate::TICK);
        }
        assert_eq!(g2.wave, 4, "arranca la oleada 4 en la arena nueva");
        assert!(!g2.events.iter().any(|e| matches!(e, GameEvent::WaveCleared)), "sin bono repetido");
    }

    #[test]
    fn normal_de_la_pared_y_el_piso() {
        let mut w = World::new(40.0);
        w.add_box(0.0, 0.0, 4.0, 3.0, 4.0, crate::mat::Mat::Concrete);
        // desde el este, contra la cara x = +2
        let o = Vec3::new(10.0, 1.5, 0.3);
        let d = Vec3::new(-1.0, -0.05, 0.1).normalize();
        let t = w.ray(o, d, 50.0);
        assert!(t < 50.0);
        assert!(surface_normal(&w, o + d * t, d).distance(Vec3::X) < 1e-3);
        // contra el piso
        let o = Vec3::new(10.0, 1.5, 10.0);
        let d = Vec3::new(0.3, -1.0, 0.0).normalize();
        let t = w.ray(o, d, 50.0);
        assert!(surface_normal(&w, o + d * t, d).distance(Vec3::Y) < 1e-3);
    }

    fn target(x: f32, z: f32) -> Target {
        Target { pos: Vec3::new(x, 1.6, z), body: Vec3::new(x, 1.1, z), vel: Vec3::ZERO, feet: 0.0, crouch_t: 0.0, evasive: false, light: false }
    }

    #[test]
    fn la_ciudad_arranca_con_sus_unidades() {
        let mut l = build_level(MapId::Ciudad);
        let g = Game::new(&mut l, 1);
        let soldiers = g.enemies.iter().filter(|e| e.is_soldier()).count();
        let turrets = g.enemies.iter().filter(|e| e.is_turret()).count();
        // antena 4+1 francotirador, subestación 4, oficinas 4+1, arsenal 4, mando 5+1, obra 4+2, trenes 4+1
        // más 6 campamentos de 4 soldados (3 con francotirador)
        assert_eq!(soldiers, 5 + 4 + 5 + 4 + 6 + 6 + 5 + 6 * 4 + 3);
        assert_eq!(turrets, 1 + 2 + 1 + 1 + 2 + 1 + 2 + 6);
        assert_eq!(g.enemies.iter().filter(|e| e.surv.is_some()).count(), l.city.as_ref().unwrap().drone_routes.len());
    }

    #[test]
    fn plantado_frente_al_mando_te_detectan_y_disparan() {
        let mut l = build_level(MapId::Ciudad);
        let mut g = Game::new(&mut l, 1);
        // en la puerta sur del mando, a la vista
        let m = l.city.as_ref().unwrap().outposts.iter().find(|o| o.id == crate::city::OutpostId::Mando).map(|o| (o.cx, o.cz)).unwrap();
        let t = target(m.0, m.1 + 14.0);
        let mut hurt = 0.0;
        for _ in 0..(60 * 20) {
            g.step(&mut l, &t, true, crate::TICK);
            for e in g.events.drain(..) {
                if let GameEvent::Hurt { dmg, .. } = e {
                    hurt += dmg;
                }
            }
        }
        assert!(g.city.as_ref().unwrap().ops.iter().any(|o| o.ever_alarm), "nadie dio la alarma");
        assert!(hurt > 0.0, "nadie disparó");
    }

    #[test]
    fn oleadas_arrancan_y_se_limpian() {
        let mut l = build_level(MapId::Deposito);
        let mut g = Game::new(&mut l, 3);
        let t = target(0.0, 24.0);
        for _ in 0..(60 * 3) {
            g.step(&mut l, &t, true, crate::TICK);
        }
        assert_eq!(g.wave, 1);
        assert!(!g.enemies.is_empty());
        // matar todo con explosiones
        for _ in 0..200 {
            if g.enemies.is_empty() && g.spawn_queue.is_empty() {
                break;
            }
            let idx: Vec<usize> = (0..g.enemies.len()).collect();
            for i in idx.into_iter().rev() {
                if i < g.enemies.len() {
                    g.damage(i, 100.0, None, false, t.body, &mut l);
                }
            }
            g.step(&mut l, &t, true, crate::TICK);
        }
        // la oleada 1 trae round(3 + 1,5) = 5 drones
        assert!(g.kills >= 5, "bajas: {}", g.kills);
        assert!(g.score > 0);
    }
}

impl Game {
    /// Un paso de manejo del auto que lleva el jugador: física, atropellos y ruido del motor.
    pub fn drive(&mut self, level: &mut Level, input: &crate::vehicle::DriveInput, dt: f32) {
        let Some(vi) = self.driving else { return };
        let impact = self.vehicles[vi].drive(input, &mut level.world, dt);
        let v = self.vehicles[vi].clone();
        if impact > 12.0 {
            self.events.push(GameEvent::Shake((impact / 30.0).min(0.8)));
            self.events.push(GameEvent::Sound(Snd::Crash));
            self.make_noise(v.pos, 40.0);
            if impact > 22.0 {
                self.events.push(GameEvent::Hurt { dmg: ((impact - 20.0) * 1.5).round(), from: None, to: 0 });
            }
        }
        // atropellos: soldados delante o al costado del auto, a más de 6 m/s
        if v.speed.abs() > 6.0 {
            let f = v.forward();
            for i in 0..self.enemies.len() {
                let e = &self.enemies[i];
                if !e.alive() || !e.is_soldier() || (e.pos.y - v.pos.y).abs() > 1.5 {
                    continue;
                }
                let rel = e.pos - v.pos;
                let along = rel.dot(f);
                let side = rel.dot(v.right());
                if along.abs() < 2.6 && side.abs() < 1.4 {
                    let dmg = v.speed.abs() * 0.8;
                    let dir = (f * v.speed.signum() + Vec3::Y * 0.3).normalize();
                    let p = e.pos + Vec3::Y;
                    self.events.push(GameEvent::Burst { pos: p, color: 0xff3d5a, n: 12, speed: 6.0, life: 0.5 });
                    if self.damage(i, dmg, Some(dir), false, v.pos, level) {
                        self.events.push(GameEvent::Mark(Mark::Kill));
                    }
                }
            }
        }
        if v.speed.abs() > 5.0 {
            let f = v.forward();
            for pi in 0..self.people.len() {
                let rel = self.people[pi].pos - v.pos;
                if self.people[pi].alive && rel.dot(f).abs() < 2.4 && rel.dot(v.right()).abs() < 1.3 {
                    self.hurt_person(pi);
                }
            }
        }
        // el motor se oye: los soldados cercanos van a ver qué pasa
        self.engine_noise_t -= dt;
        if self.engine_noise_t <= 0.0 && v.speed.abs() > 2.0 {
            self.engine_noise_t = 1.0;
            self.make_noise(v.pos, 22.0 + v.speed.abs());
        }
    }

    /// Auto cercano para subirse con F (a menos de 3,2 m del centro).
    pub fn vehicle_near(&self, pos: Vec3) -> Option<usize> {
        self.vehicles.iter().position(|v| (v.pos.x - pos.x).hypot(v.pos.z - pos.z) < 3.2 && (v.pos.y - (pos.y - 1.6)).abs() < 1.5)
    }
}

#[cfg(test)]
mod costo {
    use super::*;
    use crate::maps::{MapId, build_level};

    /// Cuánto tarda un paso de la partida en la ciudad (tiene que entrar holgado en 16 ms).
    #[test]
    fn costo_de_un_paso_en_la_ciudad() {
        let mut l = build_level(MapId::Ciudad);
        let mut g = Game::new(&mut l, 7);
        let p = Vec3::new(30.0, 1.6, -30.0);
        let t = Target { pos: p, body: p - Vec3::Y * 0.4, vel: Vec3::ZERO, feet: 0.16, crouch_t: 0.0, evasive: false, light: false };
        for _ in 0..60 {
            g.step(&mut l, &t, true, crate::TICK);
        }
        let n = 600;
        let t0 = std::time::Instant::now();
        for _ in 0..n {
            g.step(&mut l, &t, true, crate::TICK);
            g.events.clear();
        }
        let ms = t0.elapsed().as_secs_f64() * 1000.0 / n as f64;
        eprintln!("paso de la partida: {ms:.3} ms ({} enemigos, {} personas, {} autos)", g.enemies.len(), g.people.len(), g.vehicles.len());
        assert!(ms < 8.0);
        let t0 = std::time::Instant::now();
        let _ = build_level(MapId::Ciudad);
        eprintln!("armar la ciudad: {:.0} ms", t0.elapsed().as_secs_f64() * 1000.0);
    }
}
