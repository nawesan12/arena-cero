//! Objetivos del modo ciudad (puerto de `17b-ciudad-juego.js`): interacción con F, derribo
//! silencioso por la espalda, cargas explosivas, efectos de cada puesto, reabastecimiento,
//! marcado de enemigos (T), antena que se cae y victoria.

use crate::city::OutpostId;
use crate::enemies::{Alert, Kind, SoldierKind};
use crate::game::{Game, GameEvent, Mark, Snd};
use crate::maps::Level;
use crate::weapons::Explosion;
use glam::Vec3;

/// Lo que el jugador tiene delante para usar con F.
#[derive(Clone, Debug, PartialEq)]
pub enum Interact {
    Takedown { enemy: usize },
    Objective { op: usize, time: f32 },
    Supply { ready: bool, cd: f32 },
    /// Subirse a un auto.
    Drive { vehicle: usize },
    Loot { idx: usize },
    /// Rehén de una misión (mantener F).
    Hostage { mission: usize },
}

/// Cartel de interacción para la interfaz.
#[derive(Clone, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Prompt {
    pub label: String,
    /// Progreso de "mantené F" (0..1), si corresponde.
    pub frac: Option<f32>,
}

#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Charge {
    pub op: usize,
    pub t: f32,
    beep: f32,
}

/// Estado de la interacción en la ciudad (vive dentro de `CityRun`).
#[derive(Clone, Debug, Default)]
pub struct ObjState {
    hold_t: f32,
    f_was: bool,
    pub charge: Option<Charge>,
    supply_cd: f32,
    /// Tiempo desde que voló la antena (para la animación de caída).
    pub antenna_fall: Option<f32>,
    pub(crate) fell: bool,
    pub prompt: Option<Prompt>,
    pub victory: bool,
    perk_t: Option<f32>,
}

/// Lo que el juego necesita saber del jugador para interactuar.
#[derive(Clone, Copy, Debug)]
pub struct Actor {
    pub pos: Vec3,
    pub feet: f32,
    pub f_held: bool,
}

impl Game {
    /// Puntos de reabastecimiento activos: el refugio y los puestos despejados.
    pub fn supplies(&self, level: &Level) -> Vec<(Vec3, &'static str)> {
        let (Some(c), Some(run)) = (level.city.as_ref(), self.city.as_ref()) else { return Vec::new() };
        let mut v = vec![(c.safe_pos, "Refugio")];
        for (i, op) in c.outposts.iter().enumerate() {
            if run.ops[i].cleared {
                v.push((Vec3::new(op.cx, 1.0, op.cz), op.name));
            }
        }
        v
    }

    fn find_interact(&self, level: &Level, a: &Actor) -> Option<Interact> {
        let c = level.city.as_ref()?;
        let run = self.city.as_ref()?;
        // derribo sigiloso: por la espalda y sin que te haya visto
        for (i, e) in self.enemies.iter().enumerate() {
            if !e.alive() || !e.is_soldier() || e.st == Alert::Combat || e.kind == Kind::Soldier(SoldierKind::Commander) {
                continue;
            }
            let d = (e.pos.x - a.pos.x).hypot(e.pos.z - a.pos.z);
            if d > 2.3 || (e.pos.y - a.feet).abs() > 1.2 {
                continue;
            }
            let (fx, fz) = (e.yaw.sin(), e.yaw.cos());
            let (tx, tz) = ((a.pos.x - e.pos.x) / d.max(1e-3), (a.pos.z - e.pos.z) / d.max(1e-3));
            if fx * tx + fz * tz < -0.15 {
                return Some(Interact::Takedown { enemy: i });
            }
        }
        for (i, op) in c.outposts.iter().enumerate() {
            let Some(p) = op.obj_pos else { continue };
            if run.ops[i].done || run.ops[i].asleep || run.obj.charge.is_some_and(|ch| ch.op == i) {
                continue;
            }
            if (p.x - a.pos.x).hypot(p.z - a.pos.z) < 2.6 && (p.y - 1.0 - a.feet).abs() < 1.4 {
                return Some(Interact::Objective { op: i, time: op.id.obj_time() });
            }
        }
        if let Some(mission) = self.hostage_near(a.pos) {
            return Some(Interact::Hostage { mission });
        }
        if let Some(idx) = self.loot_near(a.pos, a.feet) {
            return Some(Interact::Loot { idx });
        }
        for (p, _) in self.supplies(level) {
            if (p.x - a.pos.x).hypot(p.z - a.pos.z) < 2.4 && (p.y - 1.0 - a.feet).abs() < 1.4 {
                return Some(Interact::Supply { ready: run.obj.supply_cd <= 0.0, cd: run.obj.supply_cd });
            }
        }
        self.vehicle_near(a.pos).map(|vehicle| Interact::Drive { vehicle })
    }

    /// Un paso de la interacción y de las cargas. Llamar después de `step`.
    pub fn step_objectives(&mut self, level: &mut Level, a: &Actor, dt: f32) {
        if self.city.is_none() {
            return;
        }
        let driving = self.driving.is_some();
        let found = if driving { None } else { self.find_interact(level, a) };
        let run = self.city.as_mut().unwrap();
        run.obj.supply_cd -= dt;
        let pressed = a.f_held && !run.obj.f_was;
        run.obj.f_was = a.f_held;
        if driving {
            run.obj.prompt = Some(Prompt { label: "F · BAJAR".into(), frac: None });
            if pressed {
                self.driving = None;
                self.events.push(GameEvent::ExitVehicle);
            }
            self.step_charge(level, a, dt);
            return;
        }
        let c = level.city.as_ref().unwrap();
        run.obj.prompt = found.as_ref().map(|i| match i {
            Interact::Takedown { .. } => Prompt { label: "F · NEUTRALIZAR EN SILENCIO".into(), frac: None },
            Interact::Objective { op, time } => Prompt { label: format!("MANTENÉ F · {}", c.outposts[*op].task.to_uppercase()), frac: Some((run.obj.hold_t / time).min(1.0)) },
            Interact::Supply { ready, cd } => Prompt { label: if *ready { "F · REABASTECERSE".into() } else { format!("REABASTECIMIENTO LISTO EN {} S", cd.ceil()) }, frac: None },
            Interact::Drive { .. } => Prompt { label: "F · MANEJAR".into(), frac: None },
            Interact::Loot { .. } => Prompt { label: "F · ABRIR LA CAJA".into(), frac: None },
            Interact::Hostage { mission } => Prompt { label: "MANTENÉ F · LIBERAR AL REHÉN".into(), frac: Some((self.director.missions[*mission].hold / 2.5).min(1.0)) },
        });
        // mantener F para liberar al rehén
        if let Some(Interact::Hostage { mission }) = found {
            if a.f_held {
                self.director.missions[mission].hold += dt;
            }
        }
        let run = self.city.as_mut().unwrap();
        // mantener F en un objetivo
        match found {
            Some(Interact::Objective { op, time }) if a.f_held => {
                run.obj.hold_t += dt;
                if run.obj.hold_t >= time {
                    run.obj.hold_t = 0.0;
                    self.objective_reached(op, a.pos, level);
                }
            }
            _ => run.obj.hold_t = 0.0,
        }
        if pressed {
            match found {
                Some(Interact::Takedown { enemy }) => {
                    let p = self.enemies[enemy].pos + Vec3::Y * 1.3;
                    self.events.push(GameEvent::Sound(Snd::Takedown));
                    self.events.push(GameEvent::Burst { pos: p, color: 0xfff0e0, n: 6, speed: 3.0, life: 0.3 });
                    let hp = self.enemies[enemy].hp;
                    self.takedowns += 1;
                    self.damage(enemy, hp + 1.0, None, true, a.pos, level);
                    self.events.push(GameEvent::Mark(Mark::Kill));
                }
                Some(Interact::Loot { idx }) => self.take_loot(idx),
                Some(Interact::Drive { vehicle }) => {
                    // robar un auto del tránsito: el conductor se baja corriendo
                    if self.vehicles[vehicle].ai.as_ref().is_some_and(|a| a.civil) {
                        self.vehicles[vehicle].ai = None;
                        let p = self.vehicles[vehicle].pos;
                        self.scare_people(p, 12.0);
                    }
                    self.driving = Some(vehicle);
                    self.events.push(GameEvent::Sound(Snd::Pickup));
                }
                Some(Interact::Supply { ready: true, .. }) => {
                    self.city.as_mut().unwrap().obj.supply_cd = 20.0;
                    self.events.push(GameEvent::Refill);
                    self.events.push(GameEvent::Sound(Snd::Pickup));
                    self.banner("REABASTECIDO", "Integridad, munición y granadas al máximo");
                }
                _ => {}
            }
        }
        self.step_charge(level, a, dt);
        // antena cayendo
        let run = self.city.as_mut().unwrap();
        if let Some(t) = run.obj.antenna_fall.as_mut() {
            *t += dt;
            if *t >= 2.4 && !run.obj.fell {
                run.obj.fell = true;
                let op = c_pos(level, OutpostId::Antena);
                self.events.push(GameEvent::Burst { pos: op + Vec3::new(20.0, 1.0, 0.0), color: 0x6a6a6a, n: 50, speed: 9.0, life: 1.4 });
                self.events.push(GameEvent::Sound(Snd::Boom));
                self.events.push(GameEvent::Shake(0.5));
            }
        }
        // mejora tras completar un objetivo (como en la web, 1,6 s después)
        let run = self.city.as_mut().unwrap();
        if let Some(t) = run.obj.perk_t.as_mut() {
            *t -= dt;
            if *t <= 0.0 {
                run.obj.perk_t = None;
                self.events.push(GameEvent::OfferPerk);
            }
        }
    }

    /// Se terminó de mantener F en un objetivo: carga con cuenta regresiva (antena, trenes) o listo.
    pub(crate) fn objective_reached(&mut self, op: usize, _by: Vec3, level: &mut Level) {
        let Some(c) = level.city.as_ref() else { return };
        let id = c.outposts[op].id;
        if matches!(id, OutpostId::Antena | OutpostId::Trenes) {
            let p = c.outposts[op].obj_pos.unwrap_or(Vec3::ZERO);
            if let Some(run) = self.city.as_mut() {
                run.obj.charge = Some(Charge { op, t: 4.0, beep: 0.0 });
            }
            self.events.push(GameEvent::Sound(Snd::Plant));
            self.banner("CARGA PLANTADA", "Alejate: explota en 4 segundos");
            self.make_noise(p, 18.0);
        } else {
            self.complete_objective(op, level);
        }
    }

    /// `find_interact` para los jugadores remotos.
    pub(crate) fn find_interact_pub(&self, level: &Level, a: &Actor) -> Option<Interact> {
        self.find_interact(level, a)
    }

    fn step_charge(&mut self, level: &mut Level, a: &Actor, dt: f32) {
        let run = self.city.as_mut().unwrap();
        let Some(mut ch) = run.obj.charge else { return };
        ch.t -= dt;
        ch.beep -= dt;
        if ch.beep <= 0.0 {
            self.events.push(GameEvent::Sound(Snd::Beep));
            ch.beep = if ch.t < 1.5 { 0.12 } else { 0.4 };
        }
        if ch.t > 0.0 {
            self.city.as_mut().unwrap().obj.charge = Some(ch);
            return;
        }
        self.city.as_mut().unwrap().obj.charge = None;
        let c = level.city.as_ref().unwrap();
        let op = &c.outposts[ch.op];
        let p = op.obj_pos.unwrap_or(Vec3::new(op.cx, 1.0, op.cz));
        let (id, cx, cz, cols) = (op.id, op.cx, op.cz, op.antenna_cols.clone());
        self.explode(Explosion { pos: p, radius: 7.0, max_dmg: 10.0, self_dmg: 40.0, big: true, enemy: false }, Some(a.pos - Vec3::Y * 0.5), level);
        self.make_noise(p, 90.0);
        if id == OutpostId::Antena {
            let mut h = 1.0;
            while h < 24.0 {
                self.events.push(GameEvent::Burst { pos: Vec3::new(cx, h, cz), color: 0xffb02e, n: 18, speed: 8.0, life: 0.8 });
                h += 4.0;
            }
            let run = self.city.as_mut().unwrap();
            run.antenna = false;
            run.obj.antenna_fall = Some(0.0);
            for col in cols {
                level.world.remove_collider(col);
            }
        } else {
            for i in 0..6 {
                let q = Vec3::new(cx + 1.0 + self.rng.f() * 10.0, 2.0 + self.rng.f() * 7.0, cz + 7.0 + self.rng.f() * 5.0);
                self.events.push(GameEvent::Burst { pos: q, color: if i % 2 == 1 { 0xff6a2e } else { 0x2a2a2a }, n: 30, speed: 9.0, life: 1.4 });
            }
            self.events.push(GameEvent::Shake(0.6));
        }
        self.alarm_public(ch.op, None, level);
        self.complete_objective(ch.op, level);
    }

    /// Objetivo cumplido: puntaje, efecto global y mejora (o victoria si era el último).
    pub fn complete_objective(&mut self, op: usize, level: &mut Level) {
        let Some(c) = level.city.as_ref() else { return };
        let Some(run) = self.city.as_mut() else { return };
        if run.ops[op].done {
            return;
        }
        run.ops[op].done = true;
        let o = &c.outposts[op];
        run.done.push(o.id);
        let ghost = !run.ops[op].ever_alarm;
        let pts = 1500 + if ghost { 1000 } else { 0 };
        match o.id {
            OutpostId::Sub => run.powered = false,
            OutpostId::Datos => run.intel = true,
            OutpostId::Obra => run.lives = (run.lives + 1).min(4),
            OutpostId::Trenes => run.drones = false,
            OutpostId::Arsenal => run.armory = true,
            _ => {}
        }
        // en la campaña el final lo decide la operación, no la cuenta de puestos
        let all = run.done.len() >= c.outposts.iter().filter(|o| !o.camp).count() && self.campaign.is_none();
        let (id, full, effect, obj_pos) = (o.id, o.full, o.effect, o.obj_pos);
        self.score += pts;
        self.events.push(GameEvent::Score { points: pts, label: if ghost { "OBJETIVO SIN SER VISTO" } else { "OBJETIVO" } });
        self.add_credits(if ghost { 600 } else { 400 }, "OBJETIVO");
        self.events.push(GameEvent::Sound(Snd::Objective));
        match id {
            OutpostId::Sub => {
                if let Some(p) = obj_pos {
                    self.events.push(GameEvent::Burst { pos: p, color: 0x5fd8ff, n: 40, speed: 7.0, life: 0.8 });
                }
            }
            OutpostId::Obra => {
                for e in &mut self.enemies {
                    if e.is_turret() || e.kind == Kind::Soldier(SoldierKind::Sniper) {
                        e.tagged = true;
                    }
                }
            }
            OutpostId::Trenes => {
                for i in 0..self.enemies.len() {
                    let e = &self.enemies[i];
                    if e.alive() && (e.surv.is_some() || e.hunting) {
                        let p = e.pos;
                        self.events.push(GameEvent::Burst { pos: p, color: 0xff6a2e, n: 24, speed: 7.0, life: 0.7 });
                        let hp = e.hp;
                        self.damage(i, hp + 1.0, None, false, Vec3::ZERO, level);
                    }
                }
            }
            OutpostId::Arsenal => self.events.push(GameEvent::Unlock),
            _ => {}
        }
        self.banner(full.to_uppercase(), format!("{effect}{}", if ghost { " · +1000 sin ser visto" } else { "" }));
        if all {
            let run = self.city.as_mut().unwrap();
            let tb = (8000.0 - run.time * 4.0).round().max(0.0) as u32;
            let bonus = 5000 + tb + run.lives * 1000;
            run.obj.victory = true;
            self.score += bonus;
            self.events.push(GameEvent::Victory { bonus });
        } else {
            self.city.as_mut().unwrap().obj.perk_t = Some(1.6);
        }
    }

    /// T o rueda: marca al enemigo más cercano a la mira que esté a la vista.
    pub fn try_tag(&mut self, level: &Level, eye: Vec3, dir: Vec3) {
        if self.city.is_none() {
            return;
        }
        let mut best = None;
        let mut bs = 0.06f32;
        for (i, e) in self.enemies.iter().enumerate() {
            if e.tagged || !e.alive() {
                continue;
            }
            let tp = e.center();
            let v = tp - eye;
            let d = v.length();
            if d > 140.0 {
                continue;
            }
            let ang = (v / d).dot(dir).min(1.0).acos();
            if ang < bs && level.world.has_los(eye, tp) {
                bs = ang;
                best = Some(i);
            }
        }
        if let Some(i) = best {
            self.enemies[i].tagged = true;
            self.events.push(GameEvent::Sound(Snd::Tag));
        }
    }
}

fn c_pos(level: &Level, id: OutpostId) -> Vec3 {
    level.city.as_ref().and_then(|c| c.outposts.iter().find(|o| o.id == id)).map_or(Vec3::ZERO, |o| Vec3::new(o.cx, 0.0, o.cz))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::enemies::Target;
    use crate::maps::{MapId, build_level};

    fn actor(p: Vec3, f: bool) -> Actor {
        Actor { pos: p + Vec3::Y * 1.6, feet: p.y, f_held: f }
    }

    #[test]
    fn hackear_la_subestacion_apaga_las_torretas() {
        let mut l = build_level(MapId::Ciudad);
        let mut g = Game::new(&mut l, 1);
        let c = l.city.clone().unwrap();
        let (si, sub) = c.outposts.iter().enumerate().find(|(_, o)| o.id == OutpostId::Sub).unwrap();
        // sacar a los soldados para que no molesten
        for e in &mut g.enemies {
            if e.op == Some(si) {
                e.hp = 0.0;
            }
        }
        let p = sub.obj_pos.unwrap() - Vec3::Y;
        let t = Target { pos: p + Vec3::Y * 1.6, body: p + Vec3::Y, vel: Vec3::ZERO, feet: p.y, crouch_t: 0.0, evasive: false, light: false };
        for _ in 0..(60 * 4) {
            g.step(&mut l, &t, true, crate::TICK);
            g.step_objectives(&mut l, &actor(p, true), crate::TICK);
        }
        assert!(!g.city.as_ref().unwrap().powered);
        assert!(g.city.as_ref().unwrap().done.contains(&OutpostId::Sub));
    }

    #[test]
    fn la_carga_de_la_antena_explota_y_la_tira() {
        let mut l = build_level(MapId::Ciudad);
        let mut g = Game::new(&mut l, 1);
        let c = l.city.clone().unwrap();
        let (ai, ant) = c.outposts.iter().enumerate().find(|(_, o)| o.id == OutpostId::Antena).unwrap();
        for e in &mut g.enemies {
            if e.op == Some(ai) {
                e.hp = 0.0;
            }
        }
        let p = ant.obj_pos.unwrap() - Vec3::Y;
        let far = Vec3::new(p.x + 30.0, 0.16, p.z + 30.0);
        let t = Target { pos: far + Vec3::Y * 1.6, body: far + Vec3::Y, vel: Vec3::ZERO, feet: far.y, crouch_t: 0.0, evasive: false, light: false };
        for _ in 0..(60 * 3) {
            g.step_objectives(&mut l, &actor(p, true), crate::TICK);
        }
        assert!(g.city.as_ref().unwrap().obj.charge.is_some(), "no se plantó la carga");
        for _ in 0..(60 * 5) {
            g.step(&mut l, &t, true, crate::TICK);
            g.step_objectives(&mut l, &actor(far, false), crate::TICK);
        }
        let run = g.city.as_ref().unwrap();
        assert!(!run.antenna && run.done.contains(&OutpostId::Antena));
        assert!(ant.antenna_cols.iter().all(|&c| l.world.collider(c).is_none()));
    }
}
