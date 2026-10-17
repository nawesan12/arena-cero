//! Enemigos: drones (`13-enemigos.js` y el bucle de `18-ciclo-de-juego.js`) y la IA táctica de
//! la ciudad (`15b-tactico.js`): soldados, francotiradores, comandante, torretas, drones de
//! vigilancia y refuerzos. Los impactos se resuelven con rayos contra zonas de golpe (esferas y
//! cajas orientadas) en vez de contra mallas, así el servidor no necesita gráficos.

use crate::city::{Outpost, Role};
use crate::nav::NavGrid;
use crate::rng::Rng;
use crate::world::World;
use glam::{Vec2, Vec3};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DroneKind {
    Normal,
    Heavy,
    Kami,
    Boss,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SoldierKind {
    Rifle,
    Sniper,
    Commander,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    Drone(DroneKind),
    Soldier(SoldierKind),
    Turret,
}

/// Estado de alerta de soldados y torretas.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Alert {
    Patrol,
    Search,
    Combat,
}

#[derive(Clone, Debug)]
pub struct Surv {
    pub route: Vec<[f32; 2]>,
    pub idx: usize,
    pub det: f32,
}

#[derive(Clone, Debug)]
pub struct Enemy {
    pub id: u32,
    pub kind: Kind,
    pub pos: Vec3,
    pub vel: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub hp: f32,
    pub max_hp: f32,
    pub radius: f32,
    pub scale: f32,
    pub value: u32,
    pub phase: f32,
    pub flash: f32,
    pub fire_cd: f32,
    pub tagged: bool,
    /// Puesto al que pertenece (ciudad).
    pub op: Option<usize>,
    // drones
    pub speed: f32,
    pub flank: f32,
    pub surv: Option<Surv>,
    pub hunting: bool,
    // soldados y torretas
    pub st: Alert,
    pub det: f32,
    pub last_seen: Vec3,
    pub see_t: f32,
    pub visible: bool,
    pub vis_t: f32,
    pub burst: u32,
    pub burst_t: f32,
    pub react: f32,
    pub role: Role,
    pub wp: usize,
    pub wait: f32,
    pub anchor: Vec3,
    pub anchor_yaw: Option<f32>,
    pub leash: f32,
    pub target: Option<Vec3>,
    pub search_t: f32,
    pub cover: Option<[f32; 2]>,
    pub cover_t: f32,
    pub crouch: f32,
    pub want_crouch: bool,
    pub peek_t: f32,
    pub nade_cd: f32,
    pub soldier_flank: bool,
    pub aim_t: f32,
    pub floor_y: f32,
    pub moving: bool,
    pub move_yaw: f32,
    pub corpse: Option<usize>,
    pub path: Vec<Vec2>,
    pub path_goal: Option<Vec2>,
    // torretas
    pub base_yaw: f32,
    pub laser_len: f32,
    laser_t: f32,
}

impl Enemy {
    fn blank(id: u32, kind: Kind, pos: Vec3) -> Self {
        Self {
            id,
            kind,
            pos,
            vel: Vec3::ZERO,
            yaw: 0.0,
            pitch: 0.0,
            hp: 1.0,
            max_hp: 1.0,
            radius: 0.5,
            scale: 1.0,
            value: 100,
            phase: 0.0,
            flash: 0.0,
            fire_cd: 1.0,
            tagged: false,
            op: None,
            speed: 0.0,
            flank: 0.0,
            surv: None,
            hunting: false,
            st: Alert::Patrol,
            det: 0.0,
            last_seen: Vec3::ZERO,
            see_t: 99.0,
            visible: false,
            vis_t: 0.0,
            burst: 0,
            burst_t: 0.0,
            react: 0.0,
            role: Role::Guard,
            wp: 0,
            wait: 0.0,
            anchor: pos,
            anchor_yaw: None,
            leash: 12.0,
            target: None,
            search_t: 0.0,
            cover: None,
            cover_t: 0.0,
            crouch: 0.0,
            want_crouch: false,
            peek_t: 0.0,
            nade_cd: 14.0,
            soldier_flank: false,
            aim_t: 0.0,
            floor_y: pos.y,
            moving: false,
            move_yaw: 0.0,
            corpse: None,
            path: Vec::new(),
            path_goal: None,
            base_yaw: 0.0,
            laser_len: 40.0,
            laser_t: 0.0,
        }
    }

    pub fn alive(&self) -> bool {
        self.hp > 0.0
    }
    pub fn is_soldier(&self) -> bool {
        matches!(self.kind, Kind::Soldier(_))
    }
    pub fn is_turret(&self) -> bool {
        self.kind == Kind::Turret
    }
    pub fn drone(&self) -> Option<DroneKind> {
        if let Kind::Drone(d) = self.kind { Some(d) } else { None }
    }
    /// Alto del marcador sobre la cabeza.
    pub fn marker_h(&self) -> f32 {
        match self.kind {
            Kind::Soldier(_) => 2.25 * self.scale,
            Kind::Turret => 2.5,
            Kind::Drone(_) => 1.2,
        }
    }
    /// Altura del centro de masa (para marcar, apuntar el gancho, etc.).
    pub fn center(&self) -> Vec3 {
        self.pos
            + Vec3::Y
                * match self.kind {
                    Kind::Soldier(_) => 1.2,
                    Kind::Turret => 1.86,
                    Kind::Drone(_) => 0.0,
                }
    }
}

/* ---------- creación ---------- */

pub fn new_drone(id: u32, d: DroneKind, pos: Vec3, wave: u32, rng: &mut Rng) -> Enemy {
    let s = match d {
        DroneKind::Boss => 2.8,
        DroneKind::Heavy => 1.6,
        _ => 1.0,
    };
    let w = wave as f32;
    let hp = match d {
        DroneKind::Boss => 40.0 + w * 5.0,
        DroneKind::Heavy => 16.0,
        DroneKind::Kami => 2.0,
        DroneKind::Normal => 4.0,
    };
    let mut e = Enemy::blank(id, Kind::Drone(d), pos);
    e.scale = s;
    e.hp = hp;
    e.max_hp = hp;
    e.phase = rng.f() * 10.0;
    e.speed = match d {
        DroneKind::Boss => 2.4,
        DroneKind::Kami => 7.5 + (w * 0.2).min(2.5),
        DroneKind::Heavy => 3.2 + (w * 0.2).min(2.5),
        DroneKind::Normal => 5.0 + (w * 0.2).min(2.5),
    };
    e.fire_cd = 2.0 + rng.f() * 1.5;
    e.radius = if d == DroneKind::Kami { 0.6 } else { 0.9 } * s;
    e.value = match d {
        DroneKind::Boss => 3000,
        DroneKind::Heavy => 500,
        DroneKind::Kami => 150,
        DroneKind::Normal => 100,
    };
    e.flank = if d == DroneKind::Normal && rng.next() < 0.45 { if rng.next() < 0.5 { -1.0 } else { 1.0 } } else { 0.0 };
    e
}

pub fn new_soldier(id: u32, op: usize, spec: &crate::city::UnitSpec, rng: &mut Rng) -> Enemy {
    let kind = match spec.role {
        Role::Commander => SoldierKind::Commander,
        Role::Sniper => SoldierKind::Sniper,
        _ => SoldierKind::Rifle,
    };
    let pos = Vec3::new(spec.x, spec.y, spec.z);
    let mut e = Enemy::blank(id, Kind::Soldier(kind), pos);
    e.op = Some(op);
    e.role = spec.role;
    e.hp = match kind {
        SoldierKind::Commander => 46.0,
        SoldierKind::Sniper => 4.0,
        SoldierKind::Rifle => 6.0,
    };
    e.max_hp = e.hp;
    e.radius = 0.5;
    e.scale = if kind == SoldierKind::Commander { 1.18 } else { 1.0 };
    e.vis_t = rng.f() * 0.2;
    e.fire_cd = 1.0 + rng.f();
    e.yaw = rng.f() * std::f32::consts::TAU;
    e.wp = rng.idx(8);
    e.wait = rng.f() * 2.0;
    e.leash = spec.leash;
    e.nade_cd = 14.0 + rng.f() * 10.0;
    e.phase = rng.f() * 6.0;
    e.soldier_flank = rng.next() < 0.35;
    e.value = match kind {
        SoldierKind::Commander => 1500,
        SoldierKind::Sniper => 200,
        SoldierKind::Rifle => 120,
    };
    e
}

pub fn new_turret(id: u32, op: usize, spec: &crate::city::TurretSpec, rng: &mut Rng) -> Enemy {
    let mut e = Enemy::blank(id, Kind::Turret, Vec3::new(spec.x, 0.16, spec.z));
    e.op = Some(op);
    e.hp = 12.0;
    e.max_hp = 12.0;
    e.radius = 0.6;
    e.base_yaw = spec.yaw;
    e.yaw = spec.yaw;
    e.vis_t = rng.f() * 0.2;
    e.fire_cd = 0.0;
    e.value = 250;
    e.tagged = true;
    e
}

/* ---------- zonas de golpe ---------- */

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum HitZone {
    Body,
    Crit,
    Shield,
}

#[derive(Clone, Copy, Debug)]
pub struct Hit {
    pub enemy: usize,
    pub dist: f32,
    pub zone: HitZone,
    pub point: Vec3,
}

fn ray_sphere(o: Vec3, d: Vec3, c: Vec3, r: f32) -> Option<f32> {
    let oc = o - c;
    let b = oc.dot(d);
    let cc = oc.length_squared() - r * r;
    let h = b * b - cc;
    if h < 0.0 {
        return None;
    }
    let t = -b - h.sqrt();
    if t >= 0.0 {
        Some(t)
    } else if cc < 0.0 {
        Some(0.0)
    } else {
        None
    }
}

/// Rayo contra una caja orientada por `yaw` (centro y medidas en mundo).
fn ray_obb(o: Vec3, d: Vec3, c: Vec3, half: Vec3, yaw: f32) -> Option<f32> {
    let (s, co) = yaw.sin_cos();
    // a coordenadas locales: rotar −yaw sobre Y
    let rot = |v: Vec3| Vec3::new(v.x * co - v.z * s, v.y, v.x * s + v.z * co);
    let (lo, ld) = (rot(o - c), rot(d));
    crate::world::ray_aabb(lo, ld, &crate::world::Aabb { min: -half, max: half }, f32::MAX)
}

/// Rotación de un punto local por `yaw` (convención de la web: +Z adelante del modelo).
fn local(e: &Enemy, x: f32, y: f32, z: f32) -> Vec3 {
    let (s, c) = e.yaw.sin_cos();
    e.pos + Vec3::new(x * c + z * s, y, -x * s + z * c) * e.scale
}

/// Todos los impactos de un rayo contra los enemigos vivos, ordenados por distancia.
pub fn ray_enemies(enemies: &[Enemy], o: Vec3, d: Vec3, far: f32, toward: Vec3) -> Vec<Hit> {
    let mut hits = Vec::new();
    for (i, e) in enemies.iter().enumerate() {
        if !e.alive() || (e.pos - o).length() > far + 4.0 {
            continue;
        }
        let mut test = |t: Option<f32>, zone: HitZone| {
            if let Some(t) = t {
                if t <= far {
                    hits.push(Hit { enemy: i, dist: t, zone, point: o + d * t });
                }
            }
        };
        match e.kind {
            Kind::Drone(dk) => {
                // el núcleo mira al jugador
                let to = (toward - e.pos).normalize_or_zero();
                if dk == DroneKind::Kami {
                    test(ray_sphere(o, d, e.pos, 0.5 * e.scale), HitZone::Body);
                } else {
                    test(ray_sphere(o, d, e.pos + to * 0.55 * e.scale, 0.3 * e.scale), HitZone::Crit);
                    test(ray_sphere(o, d, e.pos, 0.72 * e.scale), HitZone::Body);
                }
            }
            Kind::Soldier(sk) => {
                let dy = -e.crouch * 0.48;
                let sc = e.scale;
                test(ray_obb(o, d, local(e, 0.0, 1.3 + dy, 0.0), Vec3::new(0.27, 0.31, 0.16) * sc, e.yaw), HitZone::Body);
                test(ray_obb(o, d, local(e, 0.0, 1.8 + dy, 0.0), Vec3::new(0.135, 0.145, 0.145) * sc, e.yaw), HitZone::Crit);
                test(ray_obb(o, d, local(e, 0.0, 1.96 + dy, -0.01), Vec3::new(0.16, 0.075, 0.17) * sc, e.yaw), HitZone::Crit);
                for lx in [-0.13, 0.13] {
                    test(ray_obb(o, d, local(e, lx, 0.49 + dy * 0.5, 0.0), Vec3::new(0.1, 0.46, 0.12) * sc, e.yaw), HitZone::Body);
                }
                if sk == SoldierKind::Commander {
                    test(ray_obb(o, d, local(e, -0.12, 1.0 + dy, 0.62), Vec3::new(0.5, 0.675, 0.045) * sc, e.yaw), HitZone::Shield);
                }
            }
            Kind::Turret => {
                test(ray_obb(o, d, e.pos + Vec3::Y * 1.7, Vec3::new(0.35, 0.225, 0.4), e.yaw), HitZone::Body);
            }
        }
    }
    hits.sort_by(|a, b| a.dist.total_cmp(&b.dist));
    hits
}

/* ---------- objetivo (jugador) visto por la IA ---------- */

/// Lo que la IA necesita saber de un jugador.
#[derive(Clone, Copy, Debug)]
pub struct Target {
    pub pos: Vec3,
    pub body: Vec3,
    pub vel: Vec3,
    pub feet: f32,
    pub crouch_t: f32,
    /// Gancho o dash activos: muy difícil de acertar.
    pub evasive: bool,
}

impl Target {
    /// Qué tan visible es según la postura: agachado ×0,5; corriendo ×1,45; quieto ×0,75.
    pub fn stance_factor(&self) -> f32 {
        let hs = self.vel.x.hypot(self.vel.z);
        (if self.crouch_t > 0.5 { 0.5 } else { 1.0 }) * if hs > 8.5 { 1.45 } else if hs < 0.5 { 0.75 } else { 1.0 }
    }
}

/// Eventos de la IA para el juego (daño, sonido, efectos).
#[derive(Clone, Debug, PartialEq)]
pub enum AiEvent {
    /// Disparo hitscan de soldado o torreta.
    Shot { from: Vec3, to: Vec3, hit: bool, dmg: f32, big: bool },
    /// Proyectil de dron.
    DroneFire { from: Vec3, vel: Vec3, dmg: f32, heavy: bool },
    DroneSfx { pos: Vec3, boss: bool },
    Nade { from: Vec3, target: Vec3 },
    Alarm { op: usize, pos: Option<Vec3> },
    KamiBoom { idx: usize },
    Spotted,
}

/* ---------- percepción ---------- */

/// Distancia si ve al objetivo (0 si no): rango, cono de visión y línea de vista.
fn can_see(world: &World, eye: Vec3, t: &Target, range: f32, cos_fov: f32, fwd: Vec2) -> f32 {
    let d3 = t.body - eye;
    let d = d3.length();
    if d > range {
        return 0.0;
    }
    if d > 2.5 && cos_fov > -1.0 {
        let h = d3.x.hypot(d3.z).max(1e-3);
        if (d3.x * fwd.x + d3.z * fwd.y) / h < cos_fov {
            return 0.0;
        }
    }
    if !world.has_los(eye, t.body) {
        return 0.0;
    }
    d.max(0.01)
}

/// Disparo enemigo: probabilidad de acierto según precisión base, distancia y movimiento.
fn enemy_shot(t: &Target, from: Vec3, dmg: f32, acc: f32, big: bool, rng: &mut Rng, ev: &mut Vec<AiEvent>) {
    let d = from.distance(t.body);
    let hs = t.vel.x.hypot(t.vel.z);
    let mut p = acc - d * 0.009 - if hs > 8.0 { 0.2 } else if hs > 3.0 { 0.12 } else { 0.0 } - if t.evasive { 0.35 } else { 0.0 } - if t.crouch_t > 0.5 { 0.08 } else { 0.0 };
    p = p.clamp(0.04, 0.6);
    let hit = rng.next() < f64::from(p);
    let mut to = t.body;
    if !hit {
        to.x += (rng.f() - 0.5) * 2.2;
        to.y += (rng.f() - 0.3) * 1.4;
        to.z += (rng.f() - 0.5) * 2.2;
    }
    ev.push(AiEvent::Shot { from, to, hit, dmg, big });
}

/* ---------- soldados ---------- */

pub struct Ctx<'a> {
    pub world: &'a World,
    pub outposts: &'a [Outpost],
    pub nav: &'a [NavGrid],
    pub t: &'a Target,
    pub time: f32,
    pub dt: f32,
}

/// Estado de alerta compartido por puesto.
#[derive(Clone, Debug, Default)]
pub struct OpState {
    pub alarm: f32,
    pub done: bool,
    pub cleared: bool,
    pub ever_alarm: bool,
    /// Índices (en `enemies`) de sus unidades vivas se recalculan al vuelo con `op`.
    pub shooters: u32,
}

/// Movimiento de soldado hacia (tx, tz) con correa, colisión y navegación. Devuelve si llegó.
fn soldier_move(e: &mut Enemy, ctx: &Ctx, tx: f32, tz: f32, speed: f32) -> bool {
    let goal = Vec2::new(tx, tz);
    let here = Vec2::new(e.pos.x, e.pos.z);
    if here.distance(goal) < 0.3 {
        e.path.clear();
        return true;
    }
    // ¿hace falta rodear? (la web iba en línea recta y se trababa contra las cajas)
    let mut next = goal;
    if let Some(grid) = e.op.and_then(|o| ctx.nav.get(o)) {
        if e.floor_y < 0.5 {
            let knee = |p: Vec2| Vec3::new(p.x, e.floor_y + 0.7, p.y);
            let direct = ctx.world.has_los(knee(here), knee(goal));
            if direct {
                e.path.clear();
            } else {
                let stale = e.path_goal.is_none_or(|g| g.distance(goal) > 1.5) || e.path.is_empty();
                if stale {
                    e.path = grid.find(here, goal).unwrap_or_default();
                    e.path_goal = Some(goal);
                }
                while e.path.len() > 1 && here.distance(e.path[0]) < 0.6 {
                    e.path.remove(0);
                }
                if let Some(p) = e.path.first() {
                    next = *p;
                }
            }
        }
    }
    let (dx, dz) = (next.x - e.pos.x, next.y - e.pos.z);
    let d = dx.hypot(dz).max(1e-4);
    let s = (speed * ctx.dt).min(d);
    e.pos.x += dx / d * s;
    e.pos.z += dz / d * s;
    let mut p = e.pos;
    ctx.world.collide_xz(&mut p, 0.38, e.pos.y, 1.8);
    e.pos.x = p.x;
    e.pos.z = p.z;
    // correa: no se alejan de su puesto
    let (ax, az) = (e.pos.x - e.anchor.x, e.pos.z - e.anchor.z);
    let ad = ax.hypot(az);
    let lim = e.leash.max(1.0);
    if e.leash > 0.0 && ad > lim {
        e.pos.x = e.anchor.x + ax / ad * lim;
        e.pos.z = e.anchor.z + az / ad * lim;
    }
    if e.leash <= 0.0 {
        e.pos.x = e.anchor.x;
        e.pos.z = e.anchor.z;
    }
    e.pos.y = ctx.world.ground_at(e.pos.x, e.pos.z, e.pos.y + 0.3);
    e.moving = true;
    e.move_yaw = (tx - e.pos.x).atan2(tz - e.pos.z);
    false
}

fn pick_cover(e: &Enemy, ctx: &Ctx, rng: &mut Rng) -> Option<[f32; 2]> {
    let op = &ctx.outposts[e.op?];
    let mut best = None;
    let mut bs = f32::MAX;
    for c in &op.cover {
        let d_self = (c[0] - e.pos.x).hypot(c[1] - e.pos.z);
        if d_self > 16.0 {
            continue;
        }
        let d_pl = (c[0] - ctx.t.pos.x).hypot(c[1] - ctx.t.pos.z);
        if !(6.0..=40.0).contains(&d_pl) {
            continue;
        }
        // agachado no lo ven
        if ctx.world.has_los(Vec3::new(c[0], e.floor_y + 0.7, c[1]), ctx.t.body) {
            continue;
        }
        // parado puede disparar
        let peek = ctx.world.has_los(Vec3::new(c[0], e.floor_y + 1.6, c[1]), ctx.t.body);
        let sc = d_self + if peek { 0.0 } else { 8.0 } + rng.f() * 2.0;
        if sc < bs {
            bs = sc;
            best = Some(*c);
        }
    }
    best
}

/// Cadáveres visibles: el juego le pasa la lista para que los soldados los descubran.
#[derive(Clone, Debug)]
pub struct Corpse {
    pub id: u32,
    pub pos: Vec3,
    pub yaw: f32,
    pub fall: f32,
    pub t: f32,
    pub seen: bool,
    pub reported: bool,
    pub kind: SoldierKind,
}

#[allow(clippy::too_many_arguments)]
pub fn update_soldier(e: &mut Enemy, ctx: &Ctx, ops: &mut [OpState], corpses: &mut [Corpse], shooters: u32, rng: &mut Rng, ev: &mut Vec<AiEvent>) {
    let dt = ctx.dt;
    let t = ctx.t;
    let sk = match e.kind {
        Kind::Soldier(k) => k,
        _ => return,
    };
    e.moving = false;
    if e.flash > 0.0 {
        e.flash -= dt;
    }
    // percepción (cada ~0,15 s)
    e.vis_t -= dt;
    let kind_range = if sk == SoldierKind::Sniper { 95.0 } else { 42.0 };
    if e.vis_t <= 0.0 {
        e.vis_t = 0.15;
        let eye = Vec3::new(e.pos.x, e.pos.y + if e.crouch > 0.5 { 1.1 } else { 1.72 } * e.scale, e.pos.z);
        let fwd = Vec2::new(e.yaw.sin(), e.yaw.cos());
        let combat = e.st == Alert::Combat;
        let d = can_see(ctx.world, eye, t, if combat { kind_range * 1.25 } else { kind_range }, if combat { 0.05 } else { 0.42 }, fwd);
        let was = e.visible;
        e.visible = d > 0.0;
        // cada vez que vuelve a verte tarda en reaccionar (asomarse no es un suicidio)
        if e.visible && !was && combat {
            e.react = e.react.max(0.8 + rng.f() * 0.8 + d * 0.012);
        }
        if e.visible {
            e.last_seen = t.body;
            e.see_t = 0.0;
            if !combat {
                let rate = (if d < 4.0 { 3.5 } else { 0.8 * (1.0 - d / kind_range) + 0.12 }) * t.stance_factor();
                e.det = (e.det + rate * 0.15).min(1.0);
                if e.det >= 1.0 {
                    e.st = Alert::Combat;
                    e.react = 0.45 + rng.f() * 0.4;
                    if let Some(o) = e.op {
                        ev.push(AiEvent::Alarm { op: o, pos: Some(t.body) });
                    }
                } else if e.det > 0.45 && e.st == Alert::Patrol {
                    e.st = Alert::Search;
                    e.target = Some(t.body);
                    e.search_t = 8.0;
                }
            }
        } else {
            if e.st != Alert::Combat {
                e.det = (e.det - 0.04).max(0.0);
            }
            // cadáveres a la vista
            if e.st == Alert::Patrol {
                for (ci, c) in corpses.iter_mut().enumerate() {
                    if c.seen || c.pos.distance(e.pos) > 20.0 {
                        continue;
                    }
                    let (cx, cz) = (c.pos.x - e.pos.x, c.pos.z - e.pos.z);
                    let h = cx.hypot(cz).max(1e-3);
                    if (cx * fwd.x + cz * fwd.y) / h < 0.3 {
                        continue;
                    }
                    if ctx.world.has_los(eye, c.pos + Vec3::Y * 0.4) {
                        c.seen = true;
                        e.st = Alert::Search;
                        e.target = Some(c.pos);
                        e.search_t = 10.0;
                        e.corpse = Some(ci);
                        e.det = e.det.max(0.6);
                        break;
                    }
                }
            }
        }
    }
    if e.st == Alert::Combat {
        e.see_t += dt;
    }
    // máquina de estados
    let mut face: Option<f32> = None;
    match e.st {
        Alert::Patrol => {
            let op = e.op.map(|o| &ctx.outposts[o]);
            if e.role == Role::Patrol && op.is_some_and(|o| !o.patrol.is_empty()) {
                let patrol = &op.unwrap().patrol;
                let w = patrol[e.wp % patrol.len()];
                if e.wait > 0.0 {
                    e.wait -= dt;
                    face = Some(e.yaw + (ctx.time * 0.8 + e.phase).sin() * 0.02);
                } else if soldier_move(e, ctx, w[0], w[1], 1.9) {
                    e.wp += 1;
                    e.wait = 1.5 + rng.f() * 2.5;
                }
            } else {
                let ay = *e.anchor_yaw.get_or_insert(e.yaw);
                face = Some(ay + (ctx.time * 0.35 + e.phase).sin() * 0.9);
            }
        }
        Alert::Search => {
            e.search_t -= dt;
            if let Some(tg) = e.target {
                if soldier_move(e, ctx, tg.x, tg.z, 3.2) {
                    face = Some(e.yaw + dt * 1.4);
                    if let Some(ci) = e.corpse {
                        if let Some(c) = corpses.get_mut(ci) {
                            if !c.reported {
                                c.reported = true;
                                if let Some(o) = e.op {
                                    ev.push(AiEvent::Alarm { op: o, pos: None });
                                }
                            }
                        }
                    }
                }
            }
            if e.search_t <= 0.0 {
                e.st = Alert::Patrol;
                e.target = None;
                e.corpse = None;
            }
        }
        Alert::Combat => {
            e.react -= dt;
            e.nade_cd -= dt;
            e.cover_t -= dt;
            let lp = e.last_seen;
            face = Some((lp.x - e.pos.x).atan2(lp.z - e.pos.z));
            if sk == SoldierKind::Sniper {
                if e.visible && e.react <= 0.0 {
                    e.aim_t += dt;
                } else {
                    e.aim_t = (e.aim_t - dt * 2.0).max(0.0);
                }
                if e.aim_t >= 2.6 {
                    e.aim_t = 0.0;
                    e.react = 3.0;
                    enemy_shot(t, muzzle(e), 24.0, 0.75, true, rng, ev);
                }
            } else {
                // cobertura: buscar, ir, asomarse y disparar
                if e.cover_t <= 0.0 && e.leash > 0.0 {
                    e.cover = pick_cover(e, ctx, rng);
                    e.cover_t = 6.0 + rng.f() * 4.0;
                }
                let mut at_cover = true;
                if let Some(c) = e.cover {
                    at_cover = soldier_move(e, ctx, c[0], c[1], 4.2);
                } else if !e.visible && e.see_t > 2.5 {
                    // avanzar hacia la última posición conocida (los flanqueadores abren por un costado)
                    let (mut tx, mut tz) = (lp.x, lp.z);
                    if e.soldier_flank {
                        let a = (lp.x - e.pos.x).atan2(lp.z - e.pos.z) + std::f32::consts::FRAC_PI_2;
                        tx += a.sin() * 7.0;
                        tz += a.cos() * 7.0;
                    }
                    at_cover = soldier_move(e, ctx, tx, tz, 3.6);
                }
                if at_cover && e.cover.is_some() {
                    e.peek_t -= dt;
                    if e.peek_t <= 0.0 {
                        e.peek_t = if e.crouch > 0.5 { 1.6 + rng.f() } else { 1.4 + rng.f() * 1.2 };
                        e.want_crouch = e.crouch <= 0.5;
                    }
                } else {
                    e.want_crouch = false;
                }
                // granadas cuando el jugador se esconde
                if !e.visible && e.see_t > 2.5 && e.see_t < 9.0 && e.nade_cd <= 0.0 {
                    let dd = lp.distance(e.pos);
                    if dd > 6.0 && dd < 26.0 && rng.next() < 0.5 {
                        ev.push(AiEvent::Nade { from: e.pos, target: lp });
                    }
                    e.nade_cd = 16.0 + rng.f() * 10.0;
                }
                // ráfagas; como mucho dos tiradores por puesto al mismo tiempo
                e.fire_cd -= dt;
                let can_shoot = e.visible && e.react <= 0.0 && e.crouch < 0.5;
                let cmd = sk == SoldierKind::Commander;
                if e.burst > 0 {
                    e.burst_t -= dt;
                    if e.burst_t <= 0.0 {
                        e.burst -= 1;
                        e.burst_t = 0.11;
                        if can_shoot {
                            enemy_shot(t, muzzle(e), if cmd { 5.0 } else { 4.0 }, if cmd { 0.48 } else { 0.4 }, false, rng, ev);
                        }
                    }
                } else if can_shoot && e.fire_cd <= 0.0 && shooters < 2 {
                    e.burst = if cmd { 4 } else { 3 };
                    e.burst_t = 0.25;
                    e.fire_cd = 1.8 + rng.f() * 1.6;
                }
                let alarm = e.op.map_or(0.0, |o| ops[o].alarm);
                if e.see_t > 25.0 && alarm <= 0.0 {
                    e.st = Alert::Search;
                    e.target = Some(lp);
                    e.search_t = 10.0;
                }
            }
        }
    }
    // agacharse y girar
    let wc = if e.want_crouch && e.st == Alert::Combat { 1.0 } else { 0.0 };
    e.crouch += (wc - e.crouch) * (dt * 8.0).min(1.0);
    if face.is_none() && e.moving {
        face = Some(e.move_yaw);
    }
    if let Some(fy) = face {
        let dy = fy - e.yaw;
        let dy = dy.sin().atan2(dy.cos());
        e.yaw += dy * (dt * if e.st == Alert::Combat { 9.0 } else { 4.0 }).min(1.0);
    }
    e.pitch = if e.st == Alert::Combat { (e.last_seen.y - (e.pos.y + 1.45)).atan2((e.last_seen.x - e.pos.x).hypot(e.last_seen.z - e.pos.z)) } else { -0.35 };
}

/// Boca del arma de un soldado, aproximada (brazos a 1,45 m, caño hacia adelante).
pub fn muzzle(e: &Enemy) -> Vec3 {
    let len = if e.kind == Kind::Soldier(SoldierKind::Sniper) { 1.15 } else { 0.84 };
    let (sp, cp) = e.pitch.sin_cos();
    local(e, 0.05, 1.41 - e.crouch * 0.48, 0.05) + Vec3::new(e.yaw.sin() * cp, sp, e.yaw.cos() * cp) * len * e.scale
}

/* ---------- torretas ---------- */

pub fn update_turret(e: &mut Enemy, ctx: &Ctx, powered: bool, rng: &mut Rng, ev: &mut Vec<AiEvent>) {
    let dt = ctx.dt;
    let t = ctx.t;
    if e.flash > 0.0 {
        e.flash -= dt;
    }
    if !powered {
        e.pitch += (0.5 - e.pitch) * (dt * 2.0).min(1.0);
        return;
    }
    let eye = e.pos + Vec3::Y * 1.7;
    e.vis_t -= dt;
    if e.vis_t <= 0.0 {
        e.vis_t = 0.12;
        let fwd = Vec2::new(e.yaw.sin(), e.yaw.cos());
        let combat = e.st == Alert::Combat;
        let d = can_see(ctx.world, eye, t, if combat { 50.0 } else { 40.0 }, if combat { 0.5 } else { 0.86 }, fwd);
        let was = e.visible;
        e.visible = d > 0.0;
        if e.visible && !was {
            e.react = 0.9 + rng.f() * 0.5;
        }
        if e.visible {
            e.last_seen = t.body;
            e.see_t = 0.0;
            if !combat {
                e.det += (1.4 * (1.0 - d / 40.0) + 0.3) * t.stance_factor() * 0.12;
                if e.det >= 1.0 {
                    e.st = Alert::Combat;
                    if let Some(o) = e.op {
                        ev.push(AiEvent::Alarm { op: o, pos: Some(t.body) });
                    }
                }
            }
        } else {
            e.det = (e.det - 0.03).max(0.0);
        }
    }
    e.see_t += dt;
    let ty;
    if e.st == Alert::Combat && e.see_t < 4.0 {
        ty = (e.last_seen.x - e.pos.x).atan2(e.last_seen.z - e.pos.z);
        let pitch = -(e.last_seen.y - eye.y).atan2((e.last_seen.x - e.pos.x).hypot(e.last_seen.z - e.pos.z));
        e.pitch += (pitch - e.pitch) * (dt * 6.0).min(1.0);
        e.fire_cd -= dt;
        e.react -= dt;
        if e.burst > 0 {
            e.burst_t -= dt;
            if e.burst_t <= 0.0 {
                e.burst -= 1;
                e.burst_t = 0.1;
                if e.visible {
                    enemy_shot(t, turret_muzzle(e), 3.0, 0.3, false, rng, ev);
                }
            }
        } else if e.visible && e.fire_cd <= 0.0 && e.react <= 0.0 {
            e.burst = 3;
            e.fire_cd = 2.2;
        }
    } else {
        if e.st == Alert::Combat && e.see_t > 10.0 {
            e.st = Alert::Patrol;
        }
        ty = e.base_yaw + (ctx.time * 0.45 + e.base_yaw).sin() * 0.95;
        e.pitch += (0.0 - e.pitch) * (dt * 3.0).min(1.0);
    }
    let dy = ty - e.yaw;
    let dy = dy.sin().atan2(dy.cos());
    e.yaw += dy * (dt * if e.st == Alert::Combat { 5.0 } else { 2.0 }).min(1.0);
    // láser: hasta la primera superficie
    e.laser_t -= dt;
    if e.laser_t <= 0.0 {
        e.laser_t = 0.1;
        e.laser_len = ctx.world.ray(turret_muzzle(e), turret_dir(e), 45.0);
    }
}

pub fn turret_dir(e: &Enemy) -> Vec3 {
    let cp = e.pitch.cos();
    Vec3::new(e.yaw.sin() * cp, -e.pitch.sin(), e.yaw.cos() * cp)
}

pub fn turret_muzzle(e: &Enemy) -> Vec3 {
    e.pos + Vec3::Y * 1.66 + turret_dir(e) * 0.95
}

/* ---------- drones ---------- */

/// Dron de vigilancia: ruta por las avenidas con un cono de luz; si te ilumina, alerta al puesto más cercano.
pub fn update_surv(e: &mut Enemy, ctx: &Ctx, ev: &mut Vec<AiEvent>) {
    let dt = ctx.dt;
    let Some(s) = e.surv.as_mut() else { return };
    let w = s.route[s.idx];
    let (dx, dz) = (w[0] - e.pos.x, w[1] - e.pos.z);
    let d = dx.hypot(dz);
    if d < 1.5 {
        s.idx = (s.idx + 1) % s.route.len();
    } else {
        e.pos.x += dx / d * 6.5 * dt;
        e.pos.z += dz / d * 6.5 * dt;
    }
    e.pos.y += (16.0 + (ctx.time + e.phase).sin() * 0.6 - e.pos.y) * (dt * 2.0).min(1.0);
    e.yaw = dx.atan2(dz);
    if e.flash > 0.0 {
        e.flash -= dt;
    }
    let (sx, sz) = (e.pos.x + e.yaw.sin() * 5.0, e.pos.z + e.yaw.cos() * 5.0);
    let t = ctx.t;
    let hd = (t.pos.x - sx).hypot(t.pos.z - sz);
    if hd < 6.5 && t.pos.y < 9.0 && ctx.world.has_los(e.pos, t.body) {
        s.det += dt * 1.6 * t.stance_factor();
        if s.det >= 1.0 {
            e.surv = None;
            e.hunting = true;
            ev.push(AiEvent::Spotted);
        }
    } else {
        s.det = (s.det - dt * 0.5).max(0.0);
    }
}

/// Dron de combate (oleadas, cazadores y refuerzos). `others` son las posiciones de los demás drones.
#[allow(clippy::too_many_arguments)]
pub fn update_drone(e: &mut Enemy, idx: usize, ctx: &Ctx, others: &[Vec3], wave: u32, lim: f32, rng: &mut Rng, ev: &mut Vec<AiEvent>) {
    let dt = ctx.dt;
    let t = ctx.t;
    let dk = e.drone().unwrap_or(DroneKind::Normal);
    let (dx, dz) = (t.pos.x - e.pos.x, t.pos.z - e.pos.z);
    let dist = dx.hypot(dz).max(0.001);
    let (nx, nz) = (dx / dist, dz / dist);
    let (mut mx, mut mz, ty);
    if dk == DroneKind::Kami {
        let wob = (ctx.time * 4.0 + e.phase).sin() * 0.35;
        mx = nx - nz * wob;
        mz = nz + nx * wob;
        ty = if dist > 6.0 { 2.2 } else { t.body.y };
    } else {
        if e.flank != 0.0 {
            // flanqueador: busca un punto al costado o la espalda del jugador
            let fa = t_yaw(t) + e.flank * 1.9;
            let (tx, tz) = (t.pos.x - fa.sin() * 10.0, t.pos.z - fa.cos() * 10.0);
            let (fx, fz) = (tx - e.pos.x, tz - e.pos.z);
            let fd = fx.hypot(fz).max(0.001);
            let fs = (fd * 0.25).min(1.25);
            let wob = (ctx.time * 1.1 + e.phase).sin() * 0.3;
            mx = fx / fd * fs - nz * wob;
            mz = fz / fd * fs + nx * wob;
        } else {
            let keep = match dk {
                DroneKind::Boss => 16.0,
                DroneKind::Heavy => 8.0,
                _ => 11.0,
            };
            let approach = ((dist - keep) * 0.4).clamp(-1.0, 1.0);
            let side = (ctx.time * 0.8 + e.phase).sin() * if dk == DroneKind::Heavy { 0.5 } else { 0.9 };
            mx = nx * approach - nz * side;
            mz = nz * approach + nx * side;
        }
        ty = match dk {
            DroneKind::Boss => 5.0,
            DroneKind::Heavy => 3.2,
            _ => 2.6,
        } + (ctx.time * 1.3 + e.phase).sin() * 0.6;
    }
    // separación entre drones
    for (oi, o) in others.iter().enumerate() {
        if oi == idx {
            continue;
        }
        let (ox, oz) = (e.pos.x - o.x, e.pos.z - o.z);
        let od2 = ox * ox + oz * oz;
        if od2 < 5.76 && od2 > 1e-4 {
            let od = od2.sqrt();
            mx += ox / od * (2.4 - od);
            mz += oz / od * (2.4 - od);
        }
    }
    let ka = 1.0 - (-(if dk == DroneKind::Kami { 5.0 } else { 3.0 }) * dt).exp();
    e.vel.x += (mx * e.speed - e.vel.x) * ka;
    e.vel.z += (mz * e.speed - e.vel.z) * ka;
    e.vel.y += ((ty - e.pos.y) * if dk == DroneKind::Kami { 4.0 } else { 2.0 } - e.vel.y) * ka;
    e.pos += e.vel * dt;
    let mut p = e.pos;
    ctx.world.collide_xz(&mut p, e.radius, e.pos.y - e.radius, e.radius * 2.0);
    e.pos.x = p.x.clamp(-lim + 0.5, lim - 0.5);
    e.pos.z = p.z.clamp(-lim + 0.5, lim - 0.5);
    e.yaw = (t.pos.x - e.pos.x).atan2(t.pos.z - e.pos.z);
    if e.flash > 0.0 {
        e.flash -= dt;
    }
    if dk == DroneKind::Kami {
        if e.pos.distance(t.body) < 1.4 {
            ev.push(AiEvent::KamiBoom { idx });
        }
        return;
    }
    e.fire_cd -= dt;
    if e.fire_cd <= 0.0 {
        let d3 = t.body.distance(e.pos);
        if d3 < if dk == DroneKind::Boss { 55.0 } else { 38.0 } && ctx.world.has_los(e.pos, t.body) {
            let lead = if rng.next() < 0.4 { d3 / 32.0 } else { 0.0 };
            let mut aim = (t.body + t.vel * lead - e.pos).normalize();
            aim.x += (rng.f() - 0.5) * 0.06;
            aim.y += (rng.f() - 0.5) * 0.04;
            aim.z += (rng.f() - 0.5) * 0.06;
            let aim = aim.normalize();
            let w = wave as f32;
            let sp = if dk == DroneKind::Boss { 9.0 } else { (11.0 + w * 0.35).min(17.0) * if dk == DroneKind::Heavy { 0.85 } else { 1.0 } };
            let dmg = match dk {
                DroneKind::Boss => 10.0,
                DroneKind::Heavy => 11.0,
                _ => 7.0,
            };
            let mut fire = |dir: Vec3| ev.push(AiEvent::DroneFire { from: e.pos + dir * (e.radius + 0.2), vel: dir * sp, dmg, heavy: dk != DroneKind::Normal });
            if dk == DroneKind::Boss {
                let n = 14;
                for q in 0..n {
                    let ang = q as f32 / n as f32 * std::f32::consts::TAU + ctx.time;
                    fire(Vec3::new(ang.cos(), -0.12 + (rng.f() - 0.5) * 0.06, ang.sin()).normalize());
                }
                for ang in [-0.07f32, 0.0, 0.07] {
                    fire(glam::Quat::from_rotation_y(ang) * aim);
                }
                e.fire_cd = 3.4;
            } else {
                if dk == DroneKind::Heavy {
                    for ang in [-0.12f32, 0.0, 0.12] {
                        fire(glam::Quat::from_rotation_y(ang) * aim);
                    }
                } else {
                    fire(aim);
                }
                e.fire_cd = (if dk == DroneKind::Heavy { 3.0 } else { 2.2 + rng.f() * 1.6 }) / (1.0 + (w.min(15.0)) * 0.03);
            }
            ev.push(AiEvent::DroneSfx { pos: e.pos, boss: dk == DroneKind::Boss });
        } else {
            e.fire_cd = 0.4;
        }
    }
}

/// Rumbo del jugador visto desde su velocidad (los flanqueadores de la web usaban el yaw de la cámara).
fn t_yaw(t: &Target) -> f32 {
    if t.vel.x.hypot(t.vel.z) > 0.5 { (-t.vel.x).atan2(-t.vel.z) } else { 0.0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn soldier_at(x: f32, z: f32, yaw: f32) -> Enemy {
        let mut e = Enemy::blank(1, Kind::Soldier(SoldierKind::Rifle), Vec3::new(x, 0.0, z));
        e.hp = 6.0;
        e.yaw = yaw;
        e
    }

    #[test]
    fn cabeza_es_critico_y_torso_no() {
        let e = vec![soldier_at(0.0, -10.0, 0.0)];
        let h = ray_enemies(&e, Vec3::new(0.0, 1.8, 0.0), Vec3::NEG_Z, 50.0, Vec3::ZERO);
        assert_eq!(h[0].zone, HitZone::Crit);
        let h = ray_enemies(&e, Vec3::new(0.0, 1.3, 0.0), Vec3::NEG_Z, 50.0, Vec3::ZERO);
        assert_eq!(h[0].zone, HitZone::Body);
        let h = ray_enemies(&e, Vec3::new(3.0, 1.3, 0.0), Vec3::NEG_Z, 50.0, Vec3::ZERO);
        assert!(h.is_empty());
    }

    #[test]
    fn escudo_del_comandante_adelante() {
        let mut c = soldier_at(0.0, -10.0, 0.0);
        c.kind = Kind::Soldier(SoldierKind::Commander);
        c.scale = 1.18;
        // mira hacia +z (al jugador en el origen): el escudo queda entre los dos
        let h = ray_enemies(&[c], Vec3::new(-0.14, 1.2, 0.0), Vec3::NEG_Z, 50.0, Vec3::ZERO);
        assert_eq!(h[0].zone, HitZone::Shield);
    }

    #[test]
    fn nucleo_del_dron_mira_al_jugador() {
        let mut rng = Rng::new(1);
        let d = new_drone(1, DroneKind::Normal, Vec3::new(0.0, 2.0, -10.0), 1, &mut rng);
        let h = ray_enemies(&[d], Vec3::new(0.0, 2.0, 0.0), Vec3::NEG_Z, 50.0, Vec3::new(0.0, 2.0, 0.0));
        assert_eq!(h[0].zone, HitZone::Crit);
    }
}
