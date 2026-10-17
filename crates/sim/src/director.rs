//! Director del mundo abierto (nuevo): hora del día, patrullas enemigas en auto y misiones
//! dinámicas que aparecen cerca del jugador: convoy, rehenes, dron de carga y contraataque a
//! un puesto liberado.

use crate::city::Role;
use crate::enemies::{Alert, DroneKind, Enemy, new_drone, new_soldier};
use crate::game::{Game, GameEvent, Snd};
use crate::maps::Level;
use crate::vehicle::{AiMode, AiOut, CarAi, Vehicle};
use glam::{Vec2, Vec3};

/// Una hora de juego dura 40 s: el día entero, 16 minutos.
pub const SECS_PER_HOUR: f32 = 40.0;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MissionKind {
    Convoy,
    Hostage,
    Cargo,
    Counter { op: usize },
}

#[derive(Clone, Debug)]
pub struct Mission {
    pub id: u32,
    pub kind: MissionKind,
    /// Dónde marcarla en la brújula y el mapa.
    pub pos: Vec3,
    pub t_left: f32,
    pub enemies: Vec<u32>,
    pub cars: Vec<u32>,
    pub reward: u32,
    /// Progreso de liberar al rehén (mantener F).
    pub hold: f32,
}

impl MissionKind {
    pub fn title(self) -> &'static str {
        match self {
            MissionKind::Convoy => "CONVOY",
            MissionKind::Hostage => "REHÉN",
            MissionKind::Cargo => "DRON DE CARGA",
            MissionKind::Counter { .. } => "CONTRAATAQUE",
        }
    }
    pub fn hint(self) -> &'static str {
        match self {
            MissionKind::Convoy => "Destruí los vehículos antes de que crucen la ciudad",
            MissionKind::Hostage => "Liberá al rehén (mantené F)",
            MissionKind::Cargo => "Derribá el dron y juntá su carga",
            MissionKind::Counter { .. } => "Defendé el puesto liberado",
        }
    }
}

/// Estado del director (vive dentro de `Game`).
#[derive(Clone, Debug)]
pub struct Director {
    pub hour: f32,
    next_t: f32,
    pub missions: Vec<Mission>,
}

impl Default for Director {
    fn default() -> Self {
        Self { hour: 10.0, next_t: 75.0, missions: Vec::new() }
    }
}

impl Director {
    /// 0 de día, 1 de noche, con transiciones al amanecer (5-7 h) y al atardecer (19-21 h).
    pub fn night(&self) -> f32 {
        let h = self.hour;
        let s = |x: f32| x * x * (3.0 - 2.0 * x);
        if (7.0..19.0).contains(&h) {
            0.0
        } else if (19.0..21.0).contains(&h) {
            s((h - 19.0) / 2.0)
        } else if (5.0..7.0).contains(&h) {
            1.0 - s((h - 5.0) / 2.0)
        } else {
            1.0
        }
    }
}

impl Game {
    fn street_points(level: &Level) -> Vec<f32> {
        level.city.as_ref().map(|c| c.streets.clone()).unwrap_or_default()
    }

    /// Ruta rectangular por calles alrededor de un punto (para patrullas).
    fn street_loop(&mut self, level: &Level, near: Vec2) -> Vec<Vec2> {
        let st = Self::street_points(level);
        let pick = |v: f32, rng: &mut crate::rng::Rng| -> (f32, f32) {
            let i = st.iter().enumerate().min_by(|a, b| (a.1 - v).abs().total_cmp(&(b.1 - v).abs())).map_or(0, |x| x.0);
            let j = (i + 1 + rng.idx(2)).min(st.len() - 1);
            let i0 = i.saturating_sub(1 + rng.idx(2));
            (st[i0], st[j])
        };
        let (x0, x1) = pick(near.x, &mut self.rng);
        let (z0, z1) = pick(near.y, &mut self.rng);
        vec![Vec2::new(x0, z0), Vec2::new(x1, z0), Vec2::new(x1, z1), Vec2::new(x0, z1)]
    }

    /// Patrullas en auto que recorren calles y persiguen al jugador.
    pub(crate) fn spawn_patrols(&mut self, level: &mut Level, n: usize) {
        for k in 0..n {
            let a = level.info.a - 40.0;
            let near = Vec2::new(self.rng.signed() * a, self.rng.signed() * a);
            let route = self.street_loop(level, near);
            let start = route[0];
            let id = self.next_id();
            let mut v = Vehicle::new(id, Vec3::new(start.x, 0.0, start.y), 0.0, 3, &mut level.world);
            v.hp = 18.0;
            v.ai = Some(CarAi::new(route, 2, false, None));
            let _ = k;
            self.vehicles.push(v);
        }
    }

    /// Soldados que bajan de un auto enemigo, ya en combate.
    fn dismount(&mut self, vi: usize, crew: u32, body: Vec3, mission: Option<u32>) {
        let v = &self.vehicles[vi];
        let (p, r, f) = (v.pos, v.right(), v.forward());
        for k in 0..crew {
            let side = if k % 2 == 0 { -1.0 } else { 1.0 };
            let at = p + r * side * 1.8 + f * (k as f32 * 0.8 - 0.5);
            let id = self.next_id();
            let mut e = new_soldier(id, usize::MAX, &crate::city::UnitSpec { x: at.x, z: at.z, y: p.y, role: Role::Patrol, leash: 45.0 }, &mut self.rng);
            e.op = None;
            e.st = Alert::Combat;
            e.last_seen = body;
            e.see_t = 0.0;
            e.react = 0.6 + self.rng.f() * 0.4;
            self.enemies.push(e);
            if let Some(mid) = mission {
                if let Some(m) = self.director.missions.iter_mut().find(|m| m.id == mid) {
                    m.enemies.push(id);
                }
            }
        }
    }

    /// Autos enemigos: manejar, ver al jugador, bajar tripulación, convoyes que llegan.
    pub(crate) fn step_enemy_cars(&mut self, level: &mut Level, player: Vec3, body: Vec3, light: bool, dt: f32) {
        let night = self.director.night();
        let range = 35.0 * (1.0 - 0.45 * night * if light { 0.2 } else { 1.0 });
        for vi in 0..self.vehicles.len() {
            if self.vehicles[vi].ai.is_none() || self.vehicles[vi].wrecked || Some(vi) == self.driving {
                continue;
            }
            let v = &self.vehicles[vi];
            let eye = v.pos + Vec3::Y * 1.5;
            let sees = eye.distance(body) < range && level.world.has_los(eye, body) && !crate::economy::smoke_blocks(&self.smoke_spheres(), eye, body);
            let out = self.vehicles[vi].ai_step(&mut level.world, player, sees, dt);
            match out {
                AiOut::Dismount => {
                    let ai = self.vehicles[vi].ai.as_mut().unwrap();
                    let crew = std::mem::take(&mut ai.crew);
                    let ev = ai.event;
                    self.dismount(vi, crew, body, ev);
                    self.events.push(GameEvent::Sound(Snd::AlertBeep));
                }
                AiOut::Arrived => {
                    // convoy que escapó: desaparece
                    if let Some(mid) = self.vehicles[vi].ai.as_ref().and_then(|a| a.event) {
                        if let Some(m) = self.director.missions.iter_mut().find(|m| m.id == mid) {
                            m.t_left = -1.0;
                        }
                    }
                    if let Some(c) = self.vehicles[vi].col.take() {
                        level.world.remove_collider(c);
                    }
                    self.vehicles[vi].wrecked = true;
                    self.vehicles[vi].pos.y = -50.0;
                }
                AiOut::None => {}
            }
            // atropella al jugador
            let v = &self.vehicles[vi];
            if v.speed.abs() > 6.0 && self.driving.is_none() {
                let rel = player - v.pos;
                if rel.dot(v.forward()).abs() < 2.6 && rel.dot(v.right()).abs() < 1.4 && (player.y - 1.6 - v.pos.y).abs() < 1.5 {
                    self.events.push(GameEvent::Hurt { dmg: 25.0, from: Some(v.pos) });
                    self.events.push(GameEvent::Shake(0.7));
                    let s = v.speed;
                    self.vehicles[vi].speed = s * 0.3;
                }
            }
        }
    }

    /// Daña un vehículo; al destruirse explota y queda como chatarra.
    pub fn damage_vehicle(&mut self, vi: usize, dmg: f32, level: &mut Level, body: Vec3) {
        let v = &mut self.vehicles[vi];
        if v.wrecked {
            return;
        }
        v.hp -= dmg;
        if v.hp > 0.0 {
            return;
        }
        v.wrecked = true;
        v.speed = 0.0;
        let p = v.pos + Vec3::Y;
        let enemy = v.ai.is_some();
        let crew = v.ai.as_ref().map_or(0, |a| a.crew);
        if let Some(a) = v.ai.as_mut() {
            a.crew = 0;
            a.mode = AiMode::Parked;
        }
        self.explode(crate::weapons::Explosion { pos: p, radius: 5.0, max_dmg: 6.0, self_dmg: 30.0, big: true, enemy: false }, Some(body), level);
        if enemy {
            self.kills += crew;
            self.score += 150 + crew * 120;
            self.events.push(GameEvent::Score { points: 150 + crew * 120, label: "VEHÍCULO" });
        }
        if self.driving == Some(vi) {
            self.driving = None;
            self.events.push(GameEvent::ExitVehicle);
        }
    }

    /// Hora del día, misiones nuevas y estado de las activas.
    pub(crate) fn step_director(&mut self, level: &mut Level, player: Vec3, dt: f32) {
        self.director.hour = (self.director.hour + dt / SECS_PER_HOUR) % 24.0;
        let h = self.director.hour;
        if (19.0..19.0 + dt / SECS_PER_HOUR * 1.5).contains(&h) {
            self.banner("CAE LA NOCHE", "L prende la linterna · de noche cuesta más verte");
        }
        if (6.0..6.0 + dt / SECS_PER_HOUR * 1.5).contains(&h) {
            self.banner("AMANECE", "Con luz te ven de más lejos");
        }
        if level.city.is_none() {
            return;
        }
        self.director.next_t -= dt;
        if self.director.next_t <= 0.0 && self.director.missions.len() < 2 {
            self.director.next_t = 70.0 + self.rng.f() * 60.0;
            self.start_mission(level, player);
        }
        // seguimiento de misiones
        let mut finished: Vec<(usize, bool)> = Vec::new();
        for (i, m) in self.director.missions.iter_mut().enumerate() {
            m.t_left -= dt;
            let alive = |id: &u32| self.enemies.iter().any(|e| e.id == *id && e.alive());
            let cars_alive = |id: &u32| self.vehicles.iter().any(|v| v.id == *id && !v.wrecked);
            if let Some(&cid) = m.cars.first() {
                if let Some(v) = self.vehicles.iter().find(|v| v.id == cid && !v.wrecked) {
                    m.pos = v.pos;
                }
            }
            if m.kind == MissionKind::Cargo {
                if let Some(e) = m.enemies.first().and_then(|id| self.enemies.iter().find(|e| e.id == *id)) {
                    m.pos = e.pos;
                }
            }
            let won = match m.kind {
                MissionKind::Convoy => !m.cars.iter().any(cars_alive) && !m.enemies.iter().any(alive),
                MissionKind::Hostage => m.hold >= 2.5,
                MissionKind::Cargo => !m.enemies.first().is_some_and(alive),
                MissionKind::Counter { .. } => !m.enemies.iter().any(alive),
            };
            if won {
                finished.push((i, true));
            } else if m.t_left <= 0.0 {
                finished.push((i, false));
            }
        }
        for &(i, won) in finished.iter().rev() {
            let m = self.director.missions.remove(i);
            if won {
                self.add_credits(m.reward, "MISIÓN");
                self.score += m.reward * 2;
                self.events.push(GameEvent::Sound(Snd::Objective));
                self.banner(format!("{} · COMPLETADO", m.kind.title()), format!("+{} créditos", m.reward));
                if m.kind == MissionKind::Cargo {
                    let g = level.world.ground_at(m.pos.x, m.pos.z, m.pos.y);
                    let id = self.next_id();
                    self.loot.push(crate::economy::Loot { id, pos: Vec3::new(m.pos.x, g, m.pos.z), kind: crate::economy::LootKind::Ammo, rich: true });
                }
            } else {
                // los que quedaron se van
                for id in &m.enemies {
                    if let Some(e) = self.enemies.iter_mut().find(|e| e.id == *id && e.drone().is_some()) {
                        e.hp = 0.0;
                    }
                }
                let lose = match m.kind {
                    MissionKind::Convoy => "El convoy escapó",
                    MissionKind::Hostage => "Se llevaron al rehén",
                    MissionKind::Cargo => "El dron de carga se fue",
                    MissionKind::Counter { op } => {
                        if let Some(run) = self.city.as_mut() {
                            run.ops[op].cleared = false;
                        }
                        "Perdiste el puesto: ya no reabastece"
                    }
                };
                self.banner(format!("{} · FALLIDO", m.kind.title()), lose);
            }
        }
    }

    fn start_mission(&mut self, level: &mut Level, player: Vec3) {
        let held: Vec<usize> = self.city.as_ref().map(|r| r.ops.iter().enumerate().filter(|(_, o)| o.cleared || o.done).map(|(i, _)| i).collect()).unwrap_or_default();
        let roll = self.rng.idx(if held.is_empty() { 3 } else { 4 });
        let kind = match roll {
            0 => MissionKind::Convoy,
            1 => MissionKind::Hostage,
            2 => MissionKind::Cargo,
            _ => MissionKind::Counter { op: held[self.rng.idx(held.len())] },
        };
        let id = self.next_id();
        let a = level.info.a - 24.0;
        let st = Self::street_points(level);
        let mut m = Mission { id, kind, pos: player, t_left: 240.0, enemies: Vec::new(), cars: Vec::new(), reward: 500, hold: 0.0 };
        match kind {
            MissionKind::Convoy => {
                // por una calle que cruza toda la ciudad, cerca del jugador
                let along_x = self.rng.next() < 0.5;
                let near = if along_x { player.z } else { player.x };
                let s = st.iter().copied().min_by(|x, y| (x - near).abs().total_cmp(&(y - near).abs())).unwrap_or(0.0) + 4.2;
                let dir = if self.rng.next() < 0.5 { 1.0 } else { -1.0 };
                let p = |t: f32| if along_x { Vec2::new(t, s) } else { Vec2::new(s, t) };
                let route = vec![p(-a * dir), p(a * dir)];
                for k in 0..3 {
                    let start = route[0] - (route[1] - route[0]).normalize() * (k as f32 * 14.0);
                    let vid = self.next_id();
                    let yaw = { let d = route[1] - route[0]; d.x.atan2(d.y) };
                    let mut v = Vehicle::new(vid, Vec3::new(start.x, 0.0, start.y), yaw, if k == 1 { 0 } else { 3 }, &mut level.world);
                    v.hp = 22.0;
                    v.ai = Some(CarAi::new(route.clone(), 2, true, Some(id)));
                    m.cars.push(vid);
                    self.vehicles.push(v);
                }
                m.reward = 700;
                m.t_left = 200.0;
            }
            MissionKind::Hostage => {
                // en una esquina a 90-200 m
                let mut best = Vec2::new(st[0], st[0]);
                for _ in 0..40 {
                    let c = Vec2::new(st[self.rng.idx(st.len())], st[self.rng.idx(st.len())]);
                    let d = c.distance(Vec2::new(player.x, player.z));
                    if (90.0..200.0).contains(&d) {
                        best = c;
                        break;
                    }
                }
                m.pos = Vec3::new(best.x, 0.0, best.y);
                for k in 0..3 {
                    let ang = k as f32 * 2.1;
                    let at = Vec3::new(best.x + ang.cos() * 5.0, 0.0, best.y + ang.sin() * 5.0);
                    let eid = self.next_id();
                    let mut e = new_soldier(eid, usize::MAX, &crate::city::UnitSpec { x: at.x, z: at.z, y: 0.0, role: Role::Guard, leash: 10.0 }, &mut self.rng);
                    e.op = None;
                    m.enemies.push(eid);
                    self.enemies.push(e);
                }
                m.reward = 600;
            }
            MissionKind::Cargo => {
                let side = self.rng.idx(4);
                let t = self.rng.signed() * a * 0.7;
                let (from, to) = match side {
                    0 => (Vec2::new(-a, t), Vec2::new(a, -t)),
                    1 => (Vec2::new(a, t), Vec2::new(-a, -t)),
                    2 => (Vec2::new(t, -a), Vec2::new(-t, a)),
                    _ => (Vec2::new(t, a), Vec2::new(-t, -a)),
                };
                let eid = self.next_id();
                let mut d = new_drone(eid, DroneKind::Heavy, Vec3::new(from.x, 30.0, from.y), 3, &mut self.rng);
                d.hp = 30.0;
                d.max_hp = 30.0;
                d.surv = Some(crate::enemies::Surv { route: vec![[to.x, to.y]], idx: 0, det: 0.0 });
                d.value = 400;
                m.enemies.push(eid);
                self.enemies.push(d);
                for k in 0..2 {
                    let eid = self.next_id();
                    let mut esc = new_drone(eid, DroneKind::Normal, Vec3::new(from.x + k as f32 * 4.0, 28.0, from.y + 4.0), 3, &mut self.rng);
                    esc.hunting = true;
                    self.enemies.push(esc);
                }
                m.pos = Vec3::new(from.x, 30.0, from.y);
                m.reward = 400;
                m.t_left = (to - from).length() / 5.0;
            }
            MissionKind::Counter { op } => {
                let c = level.city.as_ref().unwrap();
                let o = &c.outposts[op];
                let center = Vec3::new(o.cx, 0.16, o.cz);
                let ang = self.rng.f() * std::f32::consts::TAU;
                for k in 0..5 {
                    let at = center + Vec3::new((ang + k as f32 * 0.2).cos(), 0.0, (ang + k as f32 * 0.2).sin()) * 38.0;
                    let eid = self.next_id();
                    let mut e: Enemy = new_soldier(eid, op, &crate::city::UnitSpec { x: at.x, z: at.z, y: 0.0, role: Role::Patrol, leash: 60.0 }, &mut self.rng);
                    e.anchor = center;
                    e.st = Alert::Combat;
                    e.last_seen = center + Vec3::Y;
                    e.see_t = 3.0;
                    m.enemies.push(eid);
                    self.enemies.push(e);
                }
                m.pos = center;
                m.reward = 650;
                m.t_left = 150.0;
            }
        }
        self.banner(format!("{} · NUEVA MISIÓN", kind.title()), kind.hint());
        self.events.push(GameEvent::Sound(Snd::AlertBeep));
        self.director.missions.push(m);
    }

    /// Rehén cerca para liberar (índice de misión).
    pub fn hostage_near(&self, pos: Vec3) -> Option<usize> {
        self.director.missions.iter().position(|m| m.kind == MissionKind::Hostage && (m.pos.x - pos.x).hypot(m.pos.z - pos.z) < 2.2)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::economy::{Smoke, smoke_blocks, ShopItem};
    use crate::enemies::Target;
    use crate::maps::{MapId, build_level};

    fn target(p: Vec3) -> Target {
        Target { pos: p + Vec3::Y * 1.6, body: p + Vec3::Y * 1.1, vel: Vec3::ZERO, feet: p.y, crouch_t: 0.0, evasive: false, light: false }
    }

    #[test]
    fn hay_patrullas_botin_y_aparecen_misiones() {
        let mut l = build_level(MapId::Ciudad);
        let mut g = Game::new(&mut l, 5);
        assert_eq!(g.vehicles.iter().filter(|v| v.ai.is_some()).count(), 6);
        assert!(g.loot.len() >= 60);
        // el refugio queda lejos de todo: esperar a la primera misión
        let t = target(Vec3::new(-132.0, 0.16, 150.0));
        for _ in 0..(60 * 90) {
            g.step(&mut l, &t, true, crate::TICK);
            g.events.clear();
        }
        assert!(!g.director.missions.is_empty() || g.director.next_t > 60.0, "no apareció ninguna misión");
    }

    #[test]
    fn el_humo_tapa_la_vista() {
        let s = [(Vec3::new(0.0, 1.5, 10.0), 4.0)];
        assert!(smoke_blocks(&s, Vec3::new(0.0, 1.5, 0.0), Vec3::new(0.0, 1.5, 20.0)));
        assert!(!smoke_blocks(&s, Vec3::new(10.0, 1.5, 0.0), Vec3::new(10.0, 1.5, 20.0)));
        let _ = Smoke { pos: Vec3::ZERO, r: 1.0, life: 1.0 };
    }

    #[test]
    fn la_tienda_cobra() {
        let mut l = build_level(MapId::Deposito);
        let mut g = Game::new(&mut l, 1);
        assert!(!g.buy(ShopItem::Smoke));
        g.credits = 500;
        assert!(g.buy(ShopItem::Smoke));
        assert_eq!(g.credits, 500 - ShopItem::Smoke.price());
        assert_eq!(g.gadgets[0], 1 + 2);
    }

    #[test]
    fn de_noche_cuesta_mas_que_te_vean() {
        let d = Director { hour: 23.0, ..Default::default() };
        assert!(d.night() > 0.99);
        let d = Director { hour: 12.0, ..Default::default() };
        assert_eq!(d.night(), 0.0);
        let d = Director { hour: 20.0, ..Default::default() };
        assert!((0.3..0.7).contains(&d.night()));
    }
}
