//! Distrito Sur: mundo abierto (puerto de `03b-ciudad.js`).
//! Grilla de 7×7 manzanas de 32 m, calles de 12 m, muro a ±164 m y semilla fija 20261016.

use crate::mat::{self, Mat};

#[path = "city2.rs"]
pub mod city2;
use crate::rng::Rng;
use crate::world::{ColliderId, CylAxis, Cylinder, STEP, VisualBox, VisualGroup, World};
use glam::Vec3;

/// Alto de piso.
pub const FH: f32 = 3.4;
/// Espesor de pared.
pub const WT: f32 = 0.3;
pub const BLOCK: f32 = 32.0;
pub const BC: [f32; 7] = [-132.0, -88.0, -44.0, 0.0, 44.0, 88.0, 132.0];
pub const SEED: u32 = 20261016;
/// Semilla de los barrios exteriores (la ciudad original queda idéntica con `SEED`).
pub const SEED_OUTER: u32 = 20261017;
/// Medio lado del Distrito Sur original (el muro de la web).
pub const CORE_A: f32 = 164.0;
/// Centros de manzana de toda la ciudad ampliada: 13 × 13.
pub const BC_ALL: [f32; 13] = [-264.0, -220.0, -176.0, -132.0, -88.0, -44.0, 0.0, 44.0, 88.0, 132.0, 176.0, 220.0, 264.0];

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum OutpostId {
    Antena,
    Sub,
    Datos,
    Arsenal,
    Mando,
    Obra,
    Trenes,
    /// Campamento de los barrios nuevos.
    Camp,
}

impl OutpostId {
    /// Segundos que hay que mantener F en el objetivo.
    pub fn obj_time(self) -> f32 {
        match self {
            OutpostId::Antena | OutpostId::Trenes => 2.5,
            OutpostId::Sub | OutpostId::Arsenal => 3.0,
            OutpostId::Datos => 4.0,
            OutpostId::Obra => 3.5,
            OutpostId::Mando | OutpostId::Camp => 0.0,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Role {
    Patrol,
    Guard,
    Commander,
    Sniper,
}

#[derive(Clone, Copy, Debug)]
pub struct UnitSpec {
    pub x: f32,
    pub z: f32,
    pub y: f32,
    pub role: Role,
    /// Distancia máxima a su puesto (0: no se mueve).
    pub leash: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct TurretSpec {
    pub x: f32,
    pub z: f32,
    pub yaw: f32,
}

#[derive(Clone, Debug)]
pub struct Outpost {
    pub id: OutpostId,
    pub name: &'static str,
    pub full: &'static str,
    pub effect: &'static str,
    pub task: &'static str,
    pub icon: char,
    pub cx: f32,
    pub cz: f32,
    /// Puntos de cobertura alrededor de cajas y contenedores.
    pub cover: Vec<[f32; 2]>,
    pub patrol: Vec<[f32; 2]>,
    pub units: Vec<UnitSpec>,
    pub turrets: Vec<TurretSpec>,
    /// Dónde se interactúa con F (el mando no tiene: hay que eliminar al comandante).
    pub obj_pos: Option<Vec3>,
    pub extra_life: bool,
    /// Columnas de la antena (se sacan al volarla) y el grupo visual que se cae.
    pub antenna_cols: Vec<ColliderId>,
    pub antenna_group: Option<usize>,
    /// Campamento de los barrios nuevos: no es objetivo, pero despejarlo da reabastecimiento.
    pub camp: bool,
}

impl Outpost {
    fn new(id: OutpostId, name: &'static str, full: &'static str, effect: &'static str, task: &'static str, icon: char, cx: f32, cz: f32) -> Self {
        Self { id, name, full, effect, task, icon, cx, cz, cover: Vec::new(), patrol: Vec::new(), units: Vec::new(), turrets: Vec::new(), obj_pos: None, extra_life: false, antenna_cols: Vec::new(), antenna_group: None, camp: false }
    }
    fn unit(&mut self, x: f32, z: f32, role: Role) -> &mut Self {
        let leash = match role {
            Role::Patrol => 22.0,
            Role::Sniper => 0.0,
            _ => 12.0,
        };
        self.units.push(UnitSpec { x, z, y: 0.16, role, leash });
        self
    }
    fn unit_at(&mut self, x: f32, z: f32, y: f32, role: Role, leash: f32) -> &mut Self {
        self.units.push(UnitSpec { x, z, y, role, leash });
        self
    }
    fn sniper(&mut self, x: f32, z: f32, y: f32) -> &mut Self {
        self.unit_at(x, z, y, Role::Sniper, 0.0)
    }
    fn turret(&mut self, x: f32, z: f32, yaw: f32) -> &mut Self {
        self.turrets.push(TurretSpec { x, z, yaw });
        self
    }
    fn cover_around(&mut self, x: f32, z: f32, w: f32, d: f32) {
        self.cover.extend([[x - w / 2.0 - 0.9, z], [x + w / 2.0 + 0.9, z], [x, z - d / 2.0 - 0.9], [x, z + d / 2.0 + 0.9]]);
    }
}

/// Lo que el modo ciudad necesita además de la geometría.
#[derive(Clone, Debug)]
pub struct CityData {
    /// Los 7 puestos objetivo primero; después los campamentos de los barrios nuevos.
    pub outposts: Vec<Outpost>,
    /// Refugio: punto de reaparición y reabastecimiento.
    pub safe_pos: Vec3,
    /// Rutas de los drones de vigilancia, por las avenidas.
    pub drone_routes: Vec<Vec<[f32; 2]>>,
    /// Autos de la calle que se pueden manejar: (posición, rumbo, color).
    pub vehicles: Vec<(Vec3, f32, u8)>,
    /// Lugares posibles para cajas de botín (dentro de edificios y galpones).
    pub loot_spots: Vec<Vec3>,
    /// Calles (coordenada de cada calle, en X y en Z) para rutas de patrullas y convoyes.
    pub streets: Vec<f32>,
    /// Estaciones de subte.
    pub stations: Vec<city2::Station>,
}

#[derive(Clone, Copy)]
struct Opening {
    s: f32,
    w: f32,
    door: bool,
}

#[derive(Clone, Copy)]
struct BOpts<'a> {
    base: f32,
    doors: &'a [char],
    door_offset: f32,
    fire_escape: bool,
    roof_props: bool,
    balconies: bool,
    props: bool,
}

impl Default for BOpts<'_> {
    fn default() -> Self {
        Self { base: 0.16, doors: &['s'], door_offset: 0.0, fire_escape: false, roof_props: true, balconies: false, props: true }
    }
}

struct B<'w> {
    w: &'w mut World,
    r: Rng,
    /// Lugares para cajas de botín (pisos de edificios y galpones). No usan el `Rng`.
    spots: Vec<Vec3>,
}

fn free_spans(half: f32, spans: &[(f32, f32)]) -> Vec<(f32, f32)> {
    let mut s = spans.to_vec();
    s.sort_by(|p, q| p.0.total_cmp(&q.0));
    let mut out = Vec::new();
    let mut cur = -half;
    for (a, b) in s {
        if a > cur {
            out.push((cur, a));
        }
        cur = cur.max(b);
    }
    if cur < half {
        out.push((cur, half));
    }
    out
}

fn window_row(len: f32) -> Vec<Opening> {
    let (spacing, w) = (3.2, 1.4);
    let n = ((len - 1.6) / spacing).floor().max(0.0) as i32;
    (0..n).map(|i| Opening { s: -((n - 1) as f32 * spacing) / 2.0 + i as f32 * spacing, w, door: false }).collect()
}

impl B<'_> {
    fn mat_facade(&mut self) -> Mat {
        Mat::Facade(self.r.idx(mat::FACADES as usize) as u8)
    }
    fn mat_tower(&mut self) -> Mat {
        Mat::Tower(self.r.idx(mat::TOWERS as usize) as u8)
    }
    fn mat_car(&mut self) -> Mat {
        Mat::Car(self.r.idx(mat::CARS as usize) as u8)
    }

    /// Pared con aberturas: alféizar (0-1 m), ventanas (1-2,4 m) y dintel hasta el techo.
    #[allow(clippy::too_many_arguments)]
    fn wall_side(&mut self, along_x: bool, fixed: f32, c0: f32, len: f32, y0: f32, wall_h: f32, openings: &[Opening], mat: Mat) {
        let put = |w: &mut World, s0: f32, s1: f32, ya: f32, yb: f32| {
            if s1 - s0 < 0.05 || yb - ya < 0.05 {
                return;
            }
            let (mid, l) = ((s0 + s1) / 2.0 + c0, s1 - s0);
            if along_x {
                w.add_block(mid, ya, fixed, l, yb - ya, WT, mat);
            } else {
                w.add_block(fixed, ya, mid, WT, yb - ya, l, mat);
            }
        };
        let half = len / 2.0;
        let doors: Vec<_> = openings.iter().filter(|o| o.door).map(|o| (o.s - o.w / 2.0, o.s + o.w / 2.0)).collect();
        let all: Vec<_> = openings.iter().map(|o| (o.s - o.w / 2.0, o.s + o.w / 2.0)).collect();
        let top = y0 + wall_h;
        for (a, b) in free_spans(half, &doors) {
            put(self.w, a, b, y0, y0 + 1.0);
        }
        for (a, b) in free_spans(half, &all) {
            put(self.w, a, b, y0 + 1.0, top.min(y0 + 2.4));
        }
        if top > y0 + 2.4 {
            put(self.w, -half, half, y0 + 2.4, top);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn slab_with_hole(&mut self, cx: f32, cz: f32, w: f32, d: f32, y: f32, t: f32, hx0: f32, hx1: f32, hz0: f32, hz1: f32, mat: Mat) {
        let (x0, x1, z0, z1) = (cx - w / 2.0, cx + w / 2.0, cz - d / 2.0, cz + d / 2.0);
        let mut r = |a0: f32, a1: f32, b0: f32, b1: f32| {
            if a1 - a0 > 0.02 && b1 - b0 > 0.02 {
                self.w.add_block((a0 + a1) / 2.0, y, (b0 + b1) / 2.0, a1 - a0, t, b1 - b0, mat);
            }
        };
        r(x0, x1, z0, hz0);
        r(x0, x1, hz1, z1);
        r(x0, hx0, hz0, hz1);
        r(hx1, x1, hz0, hz1);
    }

    /// Edificio transitable: plantas con ventanas, puertas en planta baja, escaleras alternadas
    /// y azotea con parapeto.
    #[allow(clippy::too_many_arguments)]
    fn building(&mut self, cx: f32, cz: f32, w: f32, d: f32, floors: u32, mat: Mat, o: BOpts) {
        let (base, stair_w, run) = (o.base, 2.0, 9.0 * 0.6);
        let stair = |f: u32| {
            let west = f % 2 == 0;
            let x = if west { cx - w / 2.0 + WT + 0.1 + stair_w / 2.0 } else { cx + w / 2.0 - WT - 0.1 - stair_w / 2.0 };
            let z0 = if west { cz - d / 2.0 + WT + 0.3 } else { cz + d / 2.0 - WT - 0.3 };
            (x, z0, if west { 1.0 } else { -1.0 })
        };
        for f in 0..=floors {
            let y0 = base + f as f32 * FH;
            if f > 0 {
                let (sx, sz0, dir) = stair(f - 1);
                let (za, zb) = (sz0, sz0 + dir * (run + 0.4));
                let m = if f == floors { Mat::Roof } else { Mat::InFloor };
                self.slab_with_hole(cx, cz, w, d, y0 - 0.2, 0.2, sx - stair_w / 2.0 - 0.1, sx + stair_w / 2.0 + 0.1, za.min(zb), za.max(zb), m);
            } else {
                self.w.add_visual(cx, base + 0.006, cz, w - WT * 2.0, 0.012, d - WT * 2.0, Mat::InFloor, false);
            }
            if f == floors {
                for [x, z, ww, dd] in [[cx, cz - d / 2.0 + 0.15, w, 0.3], [cx, cz + d / 2.0 - 0.15, w, 0.3], [cx - w / 2.0 + 0.15, cz, 0.3, d - 0.6], [cx + w / 2.0 - 0.15, cz, 0.3, d - 0.6]] {
                    self.w.add_block(x, y0, z, ww, 1.0, dd, mat);
                }
                if o.fire_escape {
                    self.fire_escape(cx + w / 2.0, cz, floors, base);
                }
                if o.roof_props {
                    self.w.add_block(cx + w * 0.2, y0, cz + d * 0.15, 2.2, 1.3, 1.6, Mat::Pillar);
                    self.w.add_block(cx - w * 0.15, y0, cz + d * 0.25, 1.2, 0.9, 1.2, Mat::Steel);
                }
                break;
            }
            let wall_h = FH - 0.2;
            let side = |key: char, len: f32| {
                let mut ops = window_row(len);
                if f == 0 && o.doors.contains(&key) {
                    let ds = o.door_offset;
                    ops.retain(|op| (op.s - ds).abs() >= 2.2);
                    ops.push(Opening { s: ds, w: 2.2, door: true });
                }
                ops
            };
            let inner = d - WT * 2.0;
            self.wall_side(true, cz - d / 2.0 + WT / 2.0, cx, w, y0, wall_h, &side('n', w), mat);
            self.wall_side(true, cz + d / 2.0 - WT / 2.0, cx, w, y0, wall_h, &side('s', w), mat);
            self.wall_side(false, cx - w / 2.0 + WT / 2.0, cz, inner, y0, wall_h, &side('w', inner), mat);
            self.wall_side(false, cx + w / 2.0 - WT / 2.0, cz, inner, y0, wall_h, &side('e', inner), mat);
            let (sx, sz0, dir) = stair(f);
            for k in 1..=9 {
                let k = k as f32;
                self.w.add_block(sx, y0, sz0 + dir * (k - 0.5) * 0.6, stair_w, k * STEP, 0.6, Mat::Concrete);
            }
            // botín: del lado opuesto a la escalera de este piso
            let side = if f % 2 == 0 { 1.0 } else { -1.0 };
            self.spots.push(Vec3::new(cx + side * (w * 0.5 - 2.2), y0, cz - side * (d * 0.5 - 2.0)));
            // tabique interior con puerta: arma habitaciones y pasillos
            if w >= 12.0 {
                let odd = f % 2 == 1;
                let door = [Opening { s: if odd { -1.0 } else { 1.0 } * (d * 0.18), w: 1.3, door: true }];
                self.wall_side(false, cx + if odd { -1.5 } else { 1.5 }, cz + 0.8, d - WT * 2.0 - 1.6, y0, wall_h, &door, Mat::InFloor);
            }
            // balcones en la fachada sur
            if o.balconies && f > 0 {
                for bxo in [-w * 0.25, w * 0.25] {
                    let zf = cz + d / 2.0;
                    self.w.add_block(cx + bxo, y0 - 0.2, zf + 0.65, 3.4, 0.2, 1.3, Mat::Roof);
                    self.w.add_block(cx + bxo, y0, zf + 1.25, 3.4, 0.95, 0.08, Mat::Pole);
                    self.w.add_block(cx + bxo - 1.66, y0, zf + 0.65, 0.08, 0.95, 1.3, Mat::Pole);
                    self.w.add_block(cx + bxo + 1.66, y0, zf + 0.65, 0.08, 0.95, 1.3, Mat::Pole);
                }
            }
            if o.props {
                for _ in 0..2 {
                    let px = cx + (self.r.f() - 0.5) * (w - 6.0);
                    let pz = cz + (self.r.f() - 0.5) * (d - 5.0);
                    if (px - sx).abs() < 2.5 {
                        continue;
                    }
                    let bw = 1.2 + self.r.f();
                    let bh = 0.9 + self.r.f() * 0.3;
                    let bd = 0.8 + self.r.f() * 0.6;
                    let m = if self.r.next() < 0.5 { Mat::Crate } else { Mat::Wood };
                    self.w.add_block(px, y0, pz, bw, bh, bd, m);
                }
            }
        }
    }

    /// Torre maciza con fachada vidriada; techo alcanzable con el gancho.
    #[allow(clippy::too_many_arguments)]
    fn tower(&mut self, cx: f32, cz: f32, w: f32, d: f32, h: f32, mat: Mat, gaps: &[char]) {
        self.w.add_box(cx, cz, w, h, d, mat);
        for (key, x, z, ww, dd) in [('n', cx, cz - d / 2.0 + 0.2, w, 0.4), ('s', cx, cz + d / 2.0 - 0.2, w, 0.4), ('w', cx - w / 2.0 + 0.2, cz, 0.4, d - 0.8), ('e', cx + w / 2.0 - 0.2, cz, 0.4, d - 0.8)] {
            if !gaps.contains(&key) {
                self.w.add_block(x, h, z, ww, 1.1, dd, Mat::Roof);
                continue;
            }
            let along_x = ww > dd;
            let l = if along_x { ww } else { dd };
            let seg = (l - 3.4) / 2.0;
            for sg in [-1.0, 1.0] {
                let off = sg * (1.7 + seg / 2.0);
                if along_x {
                    self.w.add_block(x + off, h, z, seg, 1.1, dd, Mat::Roof);
                } else {
                    self.w.add_block(x, h, z + off, ww, 1.1, seg, Mat::Roof);
                }
            }
        }
        self.w.add_block(cx, h, cz, 5.0f32.min(w * 0.4), 2.2, 4.0f32.min(d * 0.4), Mat::Pillar);
        self.w.add_strip(cx, h + 2.25, cz, 0.2, 0.5, 0.2, Mat::Red);
    }

    /// Galpón con portón y techo.
    #[allow(clippy::too_many_arguments)]
    fn hall(&mut self, cx: f32, cz: f32, w: f32, d: f32, h: f32, mat: Mat, door_sides: &[char]) {
        let base = 0.16;
        let ops = |key: char| if door_sides.contains(&key) { vec![Opening { s: 0.0, w: 5.0, door: true }] } else { Vec::new() };
        let inner = d - WT * 2.0;
        self.wall_side(true, cz - d / 2.0 + WT / 2.0, cx, w, base, h, &ops('n'), mat);
        self.wall_side(true, cz + d / 2.0 - WT / 2.0, cx, w, base, h, &ops('s'), mat);
        self.wall_side(false, cx - w / 2.0 + WT / 2.0, cz, inner, base, h, &ops('w'), mat);
        self.wall_side(false, cx + w / 2.0 - WT / 2.0, cz, inner, base, h, &ops('e'), mat);
        self.w.add_block(cx, base + h, cz, w, 0.3, d, Mat::Roof);
        self.w.add_visual(cx, base + 0.006, cz, w - WT * 2.0, 0.012, d - WT * 2.0, Mat::InFloor, false);
        self.spots.push(Vec3::new(cx - w * 0.3, base, cz - d * 0.3));
    }

    fn car(&mut self, x: f32, z: f32, along_x: bool, mat: Mat, y: f32) {
        let (l, wd) = (4.3, 1.9);
        let (w, d) = if along_x { (l, wd) } else { (wd, l) };
        let (gw, gd) = if along_x { (2.2, 1.7) } else { (1.7, 2.2) };
        self.w.add_block(x, y + 0.1, z, w, 0.9, d, mat);
        self.w.add_visual(x, y + 1.35, z, gw, 0.6, gd, Mat::CarGlass, true);
        self.w.add_collider(x, z, gw, 0.65, gd, y + 1.0);
        self.w.add_visual(x, y + 0.3, z, w + 0.06, 0.25, d + 0.06, Mat::Tire, false);
    }

    /// Escalera de incendio exterior: tramos alternados en dos carriles y descansos por piso.
    fn fire_escape(&mut self, wall_x: f32, cz: f32, floors: u32, base: f32) {
        let (in_l, out_l, za, zb) = (wall_x + 0.55, wall_x + 1.45, cz - 1.8, cz + 1.8);
        for f in 0..floors {
            let y0 = base + f as f32 * FH;
            let up = f % 2 == 0;
            let lane = if up { in_l } else { out_l };
            for k in 1..=9 {
                let k = k as f32;
                let z = if up { za + (k - 0.5) * 0.4 } else { zb - (k - 0.5) * 0.4 };
                self.w.add_block(lane, y0, z, 0.9, k * STEP, 0.4, Mat::Pole);
            }
            let y1 = y0 + FH;
            let lz = if up { zb + 0.6 } else { za - 0.6 };
            self.w.add_block(wall_x + 1.0, y1 - 0.14, lz, 1.9, 0.14, 1.2, Mat::Pole);
            self.w.add_block(wall_x + 1.97, y1, lz, 0.06, 0.95, 1.2, Mat::Pole);
        }
        let mut y = base + 1.0;
        while y < base + floors as f32 * FH {
            self.w.add_strip(wall_x + 1.97, y, cz, 0.05, 0.05, 3.6, Mat::Pole);
            y += 1.0;
        }
    }

    /// Pasarela entre azoteas.
    fn skybridge(&mut self, x0: f32, z0: f32, x1: f32, z1: f32, y: f32) {
        let along_x = (x1 - x0).abs() > (z1 - z0).abs();
        let l = if along_x { (x1 - x0).abs() } else { (z1 - z0).abs() };
        let (cx, cz) = ((x0 + x1) / 2.0, (z0 + z1) / 2.0);
        let mut t = -l / 2.0;
        if along_x {
            self.w.add_block(cx, y - 0.3, cz, l, 0.3, 3.2, Mat::Steel);
            for s in [-1.0, 1.0] {
                self.w.add_block(cx, y, cz + s * 1.55, l, 1.05, 0.1, Mat::Pole);
            }
            while t <= l / 2.0 {
                self.w.add_strip(cx + t, y - 0.9, cz, 0.2, 1.2, 3.2, Mat::Pole);
                t += 3.0;
            }
        } else {
            self.w.add_block(cx, y - 0.3, cz, 3.2, 0.3, l, Mat::Steel);
            for s in [-1.0, 1.0] {
                self.w.add_block(cx + s * 1.55, y, cz, 0.1, 1.05, l, Mat::Pole);
            }
            while t <= l / 2.0 {
                self.w.add_strip(cx, y - 0.9, cz + t, 3.2, 1.2, 0.2, Mat::Pole);
                t += 3.0;
            }
        }
    }

    /// Autopista elevada con rampas de bajada en ambos extremos.
    fn highway(&mut self, z: f32, x0: f32, x1: f32, deck_y: f32) {
        let (len, cx) = (x1 - x0, (x0 + x1) / 2.0);
        self.w.add_block(cx, deck_y - 0.6, z, len, 0.6, 11.0, Mat::Concrete);
        for s in [-1.0, 1.0] {
            self.w.add_block(cx, deck_y, z + s * 5.2, len, 0.95, 0.6, Mat::Jersey);
        }
        let mut x = x0 + 6.0;
        while x < x1 - 4.0 {
            self.w.add_block(x, 0.0, z - 3.0, 1.4, deck_y - 0.6, 1.4, Mat::Concrete);
            self.w.add_block(x, 0.0, z + 3.0, 1.4, deck_y - 0.6, 1.4, Mat::Concrete);
            self.w.add_visual(x, deck_y - 0.9, z, 1.6, 0.6, 9.0, Mat::Concrete, true);
            x += 14.0;
        }
        let mut x = x0 + 3.0;
        while x < x1 {
            self.w.add_strip(x, deck_y + 0.01, z, 3.5, 0.02, 0.18, Mat::Line);
            x += 8.0;
        }
        let n = (deck_y / STEP).round() as i32;
        for (edge, dir) in [(x0, -1.0), (x1, 1.0)] {
            for k in 1..n {
                let k = k as f32;
                self.w.add_block(edge + dir * (k - 0.5) * 0.9, 0.0, z, 0.9, deck_y - k * STEP, 10.0, Mat::Concrete);
            }
        }
        let mut x = x0 + 10.0;
        while x < x1 {
            self.w.add_block(x, deck_y, z - 5.6, 0.18, 6.0, 0.18, Mat::Pole);
            self.w.add_visual(x, deck_y + 6.05, z - 5.1, 0.35, 0.14, 0.9, Mat::Lamp, false);
            x += 30.0;
        }
    }

    fn tree(&mut self, x: f32, z: f32, s: f32) {
        self.w.add_block(x, 0.0, z, 0.35 * s, 2.4 * s, 0.35 * s, Mat::Trunk);
        self.w.trees.push([x, 2.4 * s + 1.2 * s, z, 1.7 * s]);
    }

    fn lamp(&mut self, x: f32, z: f32) {
        self.w.add_block(x, 0.0, z, 0.18, 6.0, 0.18, Mat::Pole);
        self.w.add_visual(x, 6.05, z, 0.8, 0.14, 0.35, Mat::Lamp, false);
    }

    fn sandbags(&mut self, x: f32, z: f32, len: f32, along_x: bool) {
        if along_x { self.w.add_box(x, z, len, 1.1, 0.9, Mat::Sandbag) } else { self.w.add_box(x, z, 0.9, 1.1, len, Mat::Sandbag) };
    }

    fn jersey(&mut self, x: f32, z: f32, len: f32, along_x: bool) {
        if along_x { self.w.add_box(x, z, len, 0.95, 0.6, Mat::Jersey) } else { self.w.add_box(x, z, 0.6, 0.95, len, Mat::Jersey) };
    }

    /// Estructura de obra: columnas, losas parciales y escaleras sin paredes.
    fn frame(&mut self, cx: f32, cz: f32, w: f32, d: f32, floors: u32) {
        let (base, stair_w, run) = (0.16, 2.0, 9.0 * 0.6);
        let mut x = -w / 2.0;
        while x <= w / 2.0 + 0.01 {
            let mut z = -d / 2.0;
            while z <= d / 2.0 + 0.01 {
                self.w.add_block(cx + x, base, cz + z, 0.4, floors as f32 * FH, 0.4, Mat::Steel);
                z += d / 4.0;
            }
            x += w / 4.0;
        }
        let stair = |f: u32| {
            let west = f % 2 == 0;
            (if west { cx - w / 2.0 + 1.6 } else { cx + w / 2.0 - 1.6 }, if west { cz - d / 2.0 + 0.6 } else { cz + d / 2.0 - 0.6 }, if west { 1.0 } else { -1.0 })
        };
        for f in 0..=floors {
            let y0 = base + f as f32 * FH;
            if f > 0 {
                let (sx, sz0, dir) = stair(f - 1);
                let (za, zb) = (sz0, sz0 + dir * (run + 0.4));
                if f == floors {
                    self.w.add_block(cx - w / 4.0, y0 - 0.2, cz, w / 2.0, 0.2, d, Mat::Concrete);
                } else {
                    self.slab_with_hole(cx, cz, w, d, y0 - 0.2, 0.2, sx - stair_w / 2.0 - 0.1, sx + stair_w / 2.0 + 0.1, za.min(zb), za.max(zb), Mat::Concrete);
                }
                for [x, z, ww, dd] in [[cx, cz - d / 2.0, w, 0.25], [cx, cz + d / 2.0, w, 0.25], [cx - w / 2.0, cz, 0.25, d], [cx + w / 2.0, cz, 0.25, d]] {
                    self.w.add_visual(x, y0 - 0.45, z, ww, 0.3, dd, Mat::Yellow, true);
                }
            }
            if f < floors {
                let (sx, sz0, dir) = stair(f);
                for k in 1..=9 {
                    let k = k as f32;
                    self.w.add_block(sx, y0, sz0 + dir * (k - 0.5) * 0.6, stair_w, k * STEP, 0.6, Mat::Steel);
                }
            }
        }
    }

    fn tank_car(&mut self, x: f32, z: f32) {
        self.w.cylinders.push(Cylinder { center: Vec3::new(x, 2.1, z), radius: 1.5, length: 11.0, axis: CylAxis::X, mat: Mat::Tank });
        self.w.add_collider(x, z, 11.0, 3.0, 3.0, 0.6);
        self.w.add_visual(x, 0.4, z, 11.0, 0.5, 2.4, Mat::Tire, false);
    }

    fn fuel_tank(&mut self, x: f32, z: f32, r: f32, h: f32) {
        self.w.cylinders.push(Cylinder { center: Vec3::new(x, 0.16 + h / 2.0, z), radius: r, length: h, axis: CylAxis::Y, mat: Mat::Tank });
        self.w.add_collider(x, z, r * 1.8, h, r * 1.8, 0.16);
        self.w.add_strip(x, 0.16 + h * 0.8, z, r * 2.02, 0.25, r * 2.02, Mat::LineY);
    }

    fn stall(&mut self, x: f32, z: f32, mat: Mat) {
        self.w.add_box(x, z, 2.6, 1.0, 1.2, Mat::Wood);
        for (dx, dz) in [(-1.3, -0.9), (1.3, -0.9), (-1.3, 0.9), (1.3, 0.9)] {
            self.w.add_block(x + dx, 0.0, z + dz, 0.1, 2.6, 0.1, Mat::Pole);
        }
        self.w.add_visual(x, 2.65, z, 3.0, 0.12, 2.2, mat, true);
    }

    /* --- puestos enemigos --- */

    fn outpost_walls(&mut self, cx: f32, cz: f32, gates: &[char]) {
        let h = BLOCK / 2.0 - 0.6;
        for (key, ax, fx) in [('n', true, cz - h), ('s', true, cz + h), ('w', false, cx - h), ('e', false, cx + h)] {
            let segs: &[(f32, f32)] = if gates.contains(&key) { &[(-h, -3.0), (3.0, h)] } else { &[(-h, h)] };
            for &(a, b) in segs {
                let (c, l) = ((a + b) / 2.0, b - a);
                if ax {
                    self.sandbags(cx + c, fx, l, true);
                } else {
                    self.sandbags(fx, cz + c, l, false);
                }
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn op_crate(&mut self, op: &mut Outpost, x: f32, z: f32, w: f32, h: f32, d: f32) {
        self.w.add_box(x, z, w, h, d, Mat::Crate);
        op.cover_around(x, z, w, d);
    }

    fn outposts(&mut self) -> Vec<Outpost> {
        use std::f32::consts::PI;
        use OutpostId::*;
        use Role::*;
        let mut ops = Vec::new();

        // Antena de comunicaciones
        {
            let (cx, cz) = (-132.0, -44.0);
            let mut op = Outpost::new(Antena, "Antena", "Antena de comunicaciones", "Sin antena no llegan drones de refuerzo", "Plantar carga en la antena", 'A', cx, cz);
            self.outpost_walls(cx, cz, &['e', 's']);
            let mut g = VisualGroup { origin: Vec3::new(cx, 0.0, cz), boxes: Vec::new() };
            let mut am = |w: f32, h: f32, d: f32, x: f32, y: f32, z: f32, m: Mat| g.boxes.push(VisualBox { center: Vec3::new(x, y, z), size: Vec3::new(w, h, d), mat: m, shadow: true, yaw: 0.0 });
            for (lx, lz) in [(-1.6, -1.6), (1.6, -1.6), (-1.6, 1.6), (1.6, 1.6)] {
                am(0.35, 24.0, 0.35, lx, 12.0, lz, Mat::Steel);
            }
            let mut y = 3.0;
            while y < 24.0 {
                am(3.2, 0.12, 0.12, 0.0, y, -1.6, Mat::Pole);
                am(3.2, 0.12, 0.12, 0.0, y, 1.6, Mat::Pole);
                am(0.12, 0.12, 3.2, -1.6, y, 0.0, Mat::Pole);
                am(0.12, 0.12, 3.2, 1.6, y, 0.0, Mat::Pole);
                y += 3.0;
            }
            am(0.6, 0.6, 0.6, 0.0, 24.3, 0.0, Mat::Red);
            am(2.4, 0.2, 0.2, 0.0, 22.0, 0.0, Mat::Pole);
            am(0.2, 0.2, 2.4, 0.0, 22.0, 0.0, Mat::Pole);
            for (lx, lz) in [(-1.6, -1.6), (1.6, -1.6), (-1.6, 1.6), (1.6, 1.6)] {
                op.antenna_cols.push(self.w.add_collider(cx + lx, cz + lz, 0.35, 24.0, 0.35, 0.0));
            }
            op.antenna_group = Some(self.w.groups.len());
            self.w.groups.push(g);
            op.obj_pos = Some(Vec3::new(cx, 1.0, cz + 2.4));
            self.w.add_block(cx, 0.16, cz + 2.6, 1.2, 0.9, 0.6, Mat::Pillar);
            self.w.add_strip(cx, 0.7, cz + 2.93, 0.8, 0.3, 0.02, Mat::Screen);
            self.w.add_container(cx - 9.0, cz - 9.0, true, 2, 1);
            self.w.add_container(cx + 9.0, cz - 10.0, false, 1, 0);
            self.w.add_container(cx - 10.0, cz + 8.0, false, 1, 2);
            op.cover_around(cx - 9.0, cz - 9.0, 6.2, 2.6);
            op.cover_around(cx + 9.0, cz - 10.0, 2.6, 6.2);
            op.cover_around(cx - 10.0, cz + 8.0, 2.6, 6.2);
            self.op_crate(&mut op, cx + 6.0, cz + 6.0, 2.0, 1.2, 2.0);
            self.op_crate(&mut op, cx - 5.0, cz + 1.0, 1.4, 1.1, 3.0);
            self.op_crate(&mut op, cx + 4.0, cz - 4.0, 3.0, 1.1, 1.2);
            op.patrol = vec![[cx - 6.0, cz + 10.0], [cx + 10.0, cz + 10.0], [cx + 10.0, cz - 2.0], [cx - 2.0, cz - 6.0], [cx - 12.0, cz]];
            op.unit(cx + 8.0, cz + 9.0, Patrol).unit(cx - 6.0, cz - 4.0, Patrol).unit(cx + 2.0, cz + 4.0, Guard).unit(cx - 12.0, cz + 12.0, Guard);
            op.sniper(cx - 9.0, cz - 9.0, 5.4);
            op.turret(cx + 12.0, cz - 12.0, PI * 0.75);
            ops.push(op);
        }
        // Subestación
        {
            let (cx, cz) = (132.0, 44.0);
            let mut op = Outpost::new(Sub, "Subestación", "Subestación eléctrica", "Sin energía, todas las torretas se apagan", "Sabotear el generador", 'S', cx, cz);
            self.outpost_walls(cx, cz, &['w', 'n']);
            for (tx, tz) in [(-8.0, -7.0), (-2.0, -7.0), (4.0, -7.0), (-8.0, 6.0), (-2.0, 6.0)] {
                self.w.add_box(cx + tx, cz + tz, 3.0, 2.6, 2.4, Mat::Steel);
                op.cover_around(cx + tx, cz + tz, 3.0, 2.4);
                for i in -1..=1 {
                    self.w.add_strip(cx + tx + i as f32 * 0.9, 3.0, cz + tz, 0.14, 0.8, 0.14, Mat::Pole);
                }
            }
            self.hall(cx + 8.0, cz + 6.0, 9.0, 8.0, 4.0, Mat::Facade(1), &['w']);
            op.obj_pos = Some(Vec3::new(cx + 10.0, 1.0, cz + 7.0));
            self.w.add_block(cx + 10.6, 0.16, cz + 7.0, 0.7, 1.4, 1.4, Mat::Pillar);
            self.w.add_strip(cx + 10.24, 1.1, cz + 7.0, 0.02, 0.5, 0.9, Mat::Screen);
            self.op_crate(&mut op, cx + 2.0, cz + 1.0, 3.0, 1.1, 1.2);
            self.op_crate(&mut op, cx - 5.0, cz, 1.2, 1.1, 3.0);
            op.patrol = vec![[cx - 11.0, cz - 1.0], [cx + 1.0, cz - 2.0], [cx + 2.0, cz + 11.0], [cx - 9.0, cz + 11.0]];
            op.unit(cx - 10.0, cz, Patrol).unit(cx + 1.0, cz + 10.0, Patrol).unit(cx + 6.0, cz + 6.0, Guard).unit(cx - 4.0, cz - 11.0, Guard);
            op.turret(cx - 12.0, cz - 12.0, PI * 1.25).turret(cx - 12.0, cz + 12.0, -PI * 0.25);
            ops.push(op);
        }
        // Oficinas
        {
            let (cx, cz) = (88.0, -132.0);
            let mut op = Outpost::new(Datos, "Oficinas", "Torre de oficinas", "Los datos marcan a todos los enemigos", "Descargar los datos (último piso)", 'D', cx, cz);
            self.outpost_walls(cx, cz, &['s', 'w']);
            self.building(cx + 2.0, cz - 3.0, 16.0, 13.0, 3, Mat::Facade(5), BOpts { doors: &['s', 'w'], roof_props: false, fire_escape: true, ..Default::default() });
            op.obj_pos = Some(Vec3::new(cx + 7.0, 0.16 + FH * 2.0 + 1.0, cz - 6.0));
            self.w.add_block(cx + 7.5, 0.16 + FH * 2.0, cz - 6.0, 1.6, 0.8, 0.9, Mat::Pillar);
            self.w.add_strip(cx + 7.5, 0.16 + FH * 2.0 + 0.95, cz - 6.2, 1.1, 0.5, 0.06, Mat::Screen);
            self.op_crate(&mut op, cx - 9.0, cz + 9.0, 2.0, 1.2, 2.0);
            self.op_crate(&mut op, cx + 8.0, cz + 10.0, 3.0, 1.1, 1.2);
            self.op_crate(&mut op, cx - 10.0, cz - 6.0, 1.2, 1.1, 3.0);
            op.patrol = vec![[cx - 12.0, cz + 6.0], [cx - 4.0, cz + 12.0], [cx + 10.0, cz + 12.0], [cx + 12.0, cz + 6.0]];
            op.unit(cx - 6.0, cz + 9.0, Patrol).unit(cx + 9.0, cz + 7.0, Guard);
            op.unit_at(cx + 4.0, cz - 1.0, 0.16 + FH, Guard, 3.0).unit_at(cx - 2.0, cz - 6.0, 0.16 + FH * 2.0, Guard, 3.0);
            op.sniper(cx + 2.0, cz - 6.0, 0.16 + FH * 3.0);
            op.turret(cx + 13.0, cz + 13.0, -PI * 0.25);
            ops.push(op);
        }
        // Arsenal
        {
            let (cx, cz) = (0.0, 132.0);
            let mut op = Outpost::new(Arsenal, "Arsenal", "Arsenal", "Desbloquea el lanzagranadas", "Forzar el arsenal", 'R', cx, cz);
            self.outpost_walls(cx, cz, &['n', 'w']);
            self.hall(cx + 2.0, cz + 4.0, 16.0, 11.0, 5.0, Mat::Facade(3), &['n', 'w']);
            op.obj_pos = Some(Vec3::new(cx + 7.0, 1.0, cz + 7.0));
            self.w.add_block(cx + 7.0, 0.16, cz + 7.5, 2.4, 1.2, 1.2, Mat::Container(3));
            for (x, z) in [(-3.0, 6.0), (1.0, 2.0), (5.0, 2.0)] {
                self.w.add_box(cx + x, cz + z, 2.0, 1.2, 1.0, Mat::Crate);
                op.cover_around(cx + x, cz + z, 2.0, 1.0);
            }
            self.op_crate(&mut op, cx - 9.0, cz - 8.0, 2.0, 1.2, 2.0);
            self.op_crate(&mut op, cx + 8.0, cz - 9.0, 3.0, 1.1, 1.2);
            self.op_crate(&mut op, cx - 10.0, cz + 4.0, 1.2, 1.1, 3.0);
            op.patrol = vec![[cx - 11.0, cz - 10.0], [cx + 11.0, cz - 11.0], [cx + 11.0, cz - 4.0], [cx - 11.0, cz - 3.0]];
            op.unit(cx - 8.0, cz - 11.0, Patrol).unit(cx + 9.0, cz - 5.0, Patrol);
            op.unit_at(cx + 4.0, cz + 6.0, 0.16, Guard, 4.0).unit_at(cx - 3.0, cz + 2.0, 0.16, Guard, 4.0);
            op.turret(cx + 12.0, cz - 13.0, PI * 0.75);
            ops.push(op);
        }
        // Puesto de mando
        {
            let (cx, cz) = (0.0, 0.0);
            let mut op = Outpost::new(Mando, "Mando", "Puesto de mando", "Eliminar al comandante descabeza al ejército", "Eliminar al comandante", 'M', cx, cz);
            self.outpost_walls(cx, cz, &['n', 's', 'e', 'w']);
            self.hall(cx, cz - 6.0, 12.0, 8.0, 3.6, Mat::Jersey, &['s']);
            for [x, z, w, d] in [[-8.0, 6.0, 4.0, 1.0], [8.0, 6.0, 4.0, 1.0], [0.0, 10.0, 6.0, 1.0], [-10.0, -2.0, 1.0, 4.0], [10.0, -2.0, 1.0, 4.0]] {
                self.w.add_box(cx + x, cz + z, w, 1.1, d, Mat::Sandbag);
                op.cover_around(cx + x, cz + z, w, d);
            }
            self.op_crate(&mut op, cx - 5.0, cz + 2.0, 2.0, 1.2, 2.0);
            self.op_crate(&mut op, cx + 5.0, cz + 2.0, 2.0, 1.2, 2.0);
            op.patrol = vec![[cx - 12.0, cz + 12.0], [cx + 12.0, cz + 12.0], [cx + 12.0, cz - 12.0], [cx - 12.0, cz - 12.0]];
            op.unit_at(cx, cz - 6.0, 0.16, Commander, 8.0);
            op.unit(cx - 10.0, cz + 10.0, Patrol).unit(cx + 10.0, cz - 12.0, Patrol).unit(cx - 4.0, cz - 4.0, Guard).unit(cx + 4.0, cz - 4.0, Guard);
            op.sniper(cx, cz - 6.0, 0.16 + 3.9);
            op.turret(cx - 13.0, cz - 13.0, PI * 1.25).turret(cx + 13.0, cz + 13.0, -PI * 0.25);
            ops.push(op);
        }
        // Obra en construcción: la cabina de la grúa está a 40 m
        {
            let (cx, cz) = (-44.0, -132.0);
            let mut op = Outpost::new(Obra, "Obra", "Obra en construcción", "+1 intento y francotiradores y torretas marcados", "Hackear la grúa (cabina, a 40 m)", 'O', cx, cz);
            self.outpost_walls(cx, cz, &['s', 'e']);
            self.frame(cx - 4.0, cz - 3.0, 20.0, 16.0, 5);
            let (mx, mz) = (cx + 10.0, cz + 9.0);
            self.w.add_box(mx, mz, 1.8, 40.0, 1.8, Mat::Yellow);
            let mut y = 2.0;
            while y < 40.0 {
                self.w.add_strip(mx, y, mz, 1.9, 0.1, 1.9, Mat::Pole);
                y += 2.5;
            }
            self.w.add_block(mx + 6.0, 40.0, mz, 34.0, 1.2, 1.6, Mat::Yellow);
            self.w.add_block(mx - 9.0, 38.6, mz, 3.0, 1.4, 2.4, Mat::Concrete);
            self.w.add_block(mx + 2.4, 37.4, mz, 2.4, 2.4, 2.2, Mat::Yellow);
            self.w.add_visual(mx + 2.4, 38.7, mz + 1.12, 2.0, 1.0, 0.04, Mat::CarGlass, false);
            let mut x = mx - 9.0;
            while x <= mx + 22.0 {
                self.w.add_strip(x, 41.25, mz, 0.12, 0.1, 1.6, Mat::Pole);
                x += 2.0;
            }
            self.w.add_strip(mx + 18.0, 30.0, mz, 0.06, 20.0, 0.06, Mat::Pole);
            self.w.add_block(mx + 18.0, 19.4, mz, 0.8, 0.8, 0.8, Mat::Steel);
            op.obj_pos = Some(Vec3::new(mx + 2.4, 41.2 + 1.0, mz));
            op.extra_life = true;
            self.w.add_container(cx + 10.0, cz - 12.0, true, 1, 0);
            self.w.add_container(cx - 11.0, cz + 11.0, true, 2, 2);
            op.cover_around(cx + 10.0, cz - 12.0, 6.2, 2.6);
            op.cover_around(cx - 11.0, cz + 11.0, 6.2, 2.6);
            self.op_crate(&mut op, cx + 4.0, cz + 9.0, 3.0, 1.2, 1.6);
            self.op_crate(&mut op, cx - 4.0, cz + 11.0, 2.0, 1.2, 2.0);
            self.op_crate(&mut op, cx + 12.0, cz + 2.0, 1.4, 1.1, 3.0);
            for i in 0..4 {
                let i = i as f32;
                self.w.add_box(cx + 9.0 + i * 1.2, cz - 2.0, 1.0, 0.6 + i * 0.3, 3.0, Mat::Wood);
            }
            op.patrol = vec![[cx - 12.0, cz + 6.0], [cx + 4.0, cz + 12.0], [cx + 12.0, cz + 5.0], [cx + 12.0, cz - 12.0]];
            op.unit(cx - 10.0, cz + 6.0, Patrol).unit(cx + 6.0, cz + 12.0, Patrol);
            op.unit_at(cx - 4.0, cz - 3.0, 0.16, Guard, 5.0).unit_at(cx - 8.0, cz - 6.0, 0.16 + FH * 2.0, Guard, 3.0);
            op.sniper(cx - 10.0, cz - 3.0, 0.16 + FH * 5.0).sniper(cx + 1.0, cz - 9.0, 0.16 + FH * 3.0);
            op.turret(cx + 13.0, cz + 13.0, -PI * 0.25);
            ops.push(op);
        }
        // Patio de trenes y depósito de combustible
        {
            let (cx, cz) = (132.0, 132.0);
            let mut op = Outpost::new(Trenes, "Trenes", "Depósito de combustible", "Sin combustible, los drones no vuelan", "Plantar carga en el depósito", 'T', cx, cz);
            self.outpost_walls(cx, cz, &['n', 'w']);
            for tz in [-9.0, -3.0] {
                self.w.add_strip(cx, 0.19, tz + cz - 0.7, 31.0, 0.06, 0.12, Mat::Steel);
                self.w.add_strip(cx, 0.19, tz + cz + 0.7, 31.0, 0.06, 0.12, Mat::Steel);
                let mut x = -15.0;
                while x <= 15.0 {
                    self.w.add_strip(cx + x, 0.17, cz + tz, 0.25, 0.03, 2.2, Mat::Wood);
                    x += 1.2;
                }
            }
            for (x, ci) in [(-9.0, 0u8), (5.0, 2)] {
                self.w.add_block(cx + x, 0.55, cz - 9.0, 12.0, 3.4, 3.0, Mat::Container(ci));
                self.w.add_visual(cx + x, 0.3, cz - 9.0, 11.0, 0.5, 2.4, Mat::Tire, false);
                op.cover_around(cx + x, cz - 9.0, 12.0, 3.0);
            }
            self.tank_car(cx - 4.0, cz - 3.0);
            op.cover_around(cx - 4.0, cz - 3.0, 11.0, 3.0);
            self.fuel_tank(cx + 9.0, cz + 9.0, 3.0, 7.0);
            self.fuel_tank(cx + 1.0, cz + 10.0, 2.6, 6.0);
            let mut x = -1.0;
            while x <= 9.0 {
                self.w.add_strip(cx + x, 1.2, cz + 5.6, 2.0, 0.3, 0.3, Mat::Pole);
                x += 2.0;
            }
            op.obj_pos = Some(Vec3::new(cx + 5.0, 1.0, cz + 5.0));
            self.w.add_block(cx + 5.0, 0.16, cz + 5.4, 1.4, 1.2, 0.7, Mat::Pillar);
            self.w.add_strip(cx + 5.0, 0.9, cz + 5.03, 0.9, 0.4, 0.04, Mat::Screen);
            self.building(cx - 9.5, cz + 9.0, 10.0, 9.0, 2, Mat::Facade(6), BOpts { doors: &['n', 'e'], roof_props: false, ..Default::default() });
            self.op_crate(&mut op, cx + 12.0, cz + 1.0, 2.0, 1.2, 2.0);
            self.op_crate(&mut op, cx - 2.0, cz + 4.0, 3.0, 1.1, 1.2);
            op.patrol = vec![[cx - 13.0, cz - 13.0], [cx + 13.0, cz - 13.0], [cx + 13.0, cz + 2.0], [cx - 2.0, cz + 1.0]];
            op.unit(cx - 12.0, cz - 13.0, Patrol).unit(cx + 12.0, cz + 2.0, Patrol).unit(cx + 6.0, cz + 2.0, Guard).unit(cx - 3.0, cz - 6.0, Guard);
            op.sniper(cx - 10.0, cz + 9.0, 0.16 + FH * 2.0);
            op.turret(cx + 13.0, cz + 13.0, -PI * 0.25).turret(cx - 13.0, cz - 13.0, PI * 0.75);
            ops.push(op);
        }
        ops
    }

    /* --- manzanas civiles --- */

    fn block_apts(&mut self, bx: f32, bz: f32, fire_escape: bool, balconies: bool) {
        let floors = 2 + self.r.idx(2) as u32;
        let m = self.mat_facade();
        self.building(bx - 6.0, bz - 5.0, 14.0, 12.0, floors, m, BOpts { doors: &['s', 'e'], fire_escape, balconies, ..Default::default() });
        let floors = 2 + self.r.idx(2) as u32;
        let m = self.mat_facade();
        self.building(bx + 8.0, bz + 7.0, 12.0, 11.0, floors, m, BOpts { doors: &['w', 'n'], balconies, ..Default::default() });
        self.tree(bx - 11.0, bz + 11.0, 1.0);
        self.tree(bx + 12.0, bz - 11.0, 1.0);
        self.w.add_box(bx + 6.0, bz - 8.0, 3.0, 1.1, 1.0, Mat::Hedge);
    }

    fn block_courtyard(&mut self, bx: f32, bz: f32) {
        let m = self.mat_facade();
        self.building(bx, bz - 11.0, 30.0, 9.0, 3, m, BOpts { doors: &['s', 'n'], balconies: true, ..Default::default() });
        self.building(bx, bz + 11.0, 30.0, 9.0, 3, m, BOpts { doors: &['n', 's'], ..Default::default() });
        self.building(bx - 11.0, bz, 8.0, 13.0, 2, m, BOpts { doors: &['e'], props: false, ..Default::default() });
        self.building(bx + 11.0, bz, 8.0, 13.0, 2, m, BOpts { doors: &['w'], props: false, ..Default::default() });
        self.tree(bx, bz, 1.2);
        self.w.add_box(bx - 3.0, bz + 3.0, 2.0, 0.5, 0.6, Mat::Wood);
        self.w.add_box(bx + 3.0, bz - 3.0, 2.0, 0.5, 0.6, Mat::Wood);
    }

    fn block_terrace_park(&mut self, bx: f32, bz: f32) {
        for k in 1..=6 {
            let s = 30.0 - k as f32 * 4.0;
            self.w.add_box(bx, bz, s, k as f32 * STEP, s, if k % 2 == 1 { Mat::Hedge } else { Mat::Sidewalk });
        }
        for (dx, dz) in [(-1.5, -1.5), (1.5, -1.5), (-1.5, 1.5), (1.5, 1.5)] {
            self.w.add_block(bx + dx, 6.0 * STEP, bz + dz, 0.2, 2.6, 0.2, Mat::Pole);
        }
        self.w.add_block(bx, 6.0 * STEP + 2.6, bz, 4.0, 0.2, 4.0, Mat::Roof);
        for (dx, dz) in [(-14.0, -14.0), (14.0, -14.0), (-14.0, 14.0), (14.0, 14.0), (0.0, -14.5), (-14.5, 0.0)] {
            self.tree(bx + dx, bz + dz, 1.1);
        }
    }

    fn block_market(&mut self, bx: f32, bz: f32) {
        for i in -1..=1 {
            for j in -1..=1 {
                if i != 0 || j != 0 {
                    let m = Mat::Awning(self.r.idx(mat::AWNINGS as usize) as u8);
                    self.stall(bx + i as f32 * 8.0, bz + j as f32 * 7.0, m);
                }
            }
        }
        self.w.add_box(bx, bz, 4.0, 3.2, 4.0, Mat::Wood);
        self.w.add_visual(bx, 3.4, bz, 5.0, 0.2, 5.0, Mat::Awning(0), true);
        self.tree(bx - 13.0, bz - 13.0, 1.0);
        self.tree(bx + 13.0, bz + 13.0, 1.0);
    }

    fn block_plaza(&mut self, bx: f32, bz: f32) {
        self.w.add_box(bx, bz, 9.0, 0.7, 9.0, Mat::Jersey);
        self.w.add_visual(bx, 0.72, bz, 8.0, 0.04, 8.0, Mat::Water, false);
        self.w.add_box(bx, bz, 1.2, 2.4, 1.2, Mat::Jersey);
        self.w.add_block(bx, 2.4, bz, 0.8, 1.8, 0.8, Mat::Statue);
        for (dx, dz) in [(-10.0f32, 0.0), (10.0, 0.0), (0.0, -10.0), (0.0, 10.0)] {
            let (w, d) = if dx != 0.0 { (0.6, 3.0) } else { (3.0, 0.6) };
            self.w.add_box(bx + dx, bz + dz, w, 0.5, d, Mat::Wood);
        }
        for (dx, dz) in [(-12.0, -12.0), (12.0, -12.0), (-12.0, 12.0), (12.0, 12.0)] {
            self.tree(bx + dx, bz + dz, 1.2);
        }
    }

    fn block_park(&mut self, bx: f32, bz: f32) {
        for _ in 0..8 {
            let x = bx + self.r.signed() * 12.0;
            let z = bz + self.r.signed() * 12.0;
            let s = 0.9 + self.r.f() * 0.5;
            self.tree(x, z, s);
        }
        self.w.add_box(bx, bz, 6.0, 0.6, 6.0, Mat::Jersey);
        self.w.add_visual(bx, 0.62, bz, 5.0, 0.04, 5.0, Mat::Water, false);
        for [x, z, w, d] in [[-8.0, 0.0, 1.0, 6.0], [8.0, 0.0, 1.0, 6.0], [0.0, -9.0, 6.0, 1.0], [0.0, 9.0, 6.0, 1.0]] {
            self.w.add_box(bx + x, bz + z, w, 0.8, d, Mat::Hedge);
        }
    }

    fn block_hall(&mut self, bx: f32, bz: f32) {
        let m = self.mat_facade();
        self.hall(bx, bz - 2.0, 22.0, 16.0, 6.0, m, &['s', 'e']);
        for i in 0..4 {
            let z = bz - 2.0 + (self.r.f() - 0.5) * 6.0;
            let h = 1.2 + self.r.f();
            self.w.add_box(bx - 6.0 + i as f32 * 4.0, z, 2.0, h, 2.0, Mat::Crate);
        }
        let ci = self.r.idx(4);
        self.w.add_container(bx + 10.0, bz + 12.0, true, 1, ci);
    }

    fn block_containers(&mut self, bx: f32, bz: f32) {
        for _ in 0..7 {
            let ax = self.r.next() < 0.5;
            let x = bx + self.r.signed() * 10.0;
            let z = bz + self.r.signed() * 10.0;
            let stack = 1 + u32::from(self.r.next() < 0.35);
            let ci = self.r.idx(4);
            self.w.add_container(x, z, ax, stack, ci);
        }
    }

    fn block_industrial(&mut self, bx: f32, bz: f32) {
        self.hall(bx - 4.0, bz - 5.0, 18.0, 14.0, 7.0, Mat::Facade(3), &['s']);
        self.fuel_tank(bx + 10.0, bz + 8.0, 2.6, 9.0);
        self.fuel_tank(bx + 10.0, bz - 4.0, 2.6, 9.0);
        let mut y = 2.0;
        while y < 9.0 {
            self.w.add_strip(bx + 10.0, y, bz + 2.0, 0.5, 0.5, 10.0, Mat::Pole);
            y += 3.0;
        }
        let ci = self.r.idx(4);
        self.w.add_container(bx - 6.0, bz + 11.0, true, 2, ci);
    }

    fn block_lot(&mut self, bx: f32, bz: f32) {
        for i in 0..6 {
            let m = self.mat_car();
            self.car(bx - 9.0 + (i % 3) as f32 * 9.0, bz - 9.0 + (i / 3) as f32 * 9.0, false, m, 0.0);
        }
        let m = self.mat_facade();
        self.building(bx + 9.0, bz + 9.0, 10.0, 9.0, 1, m, BOpts { doors: &['w'], ..Default::default() });
    }

    fn block_tower(&mut self, bx: f32, bz: f32, h: f32, gaps: &[char]) {
        let m = self.mat_tower();
        self.tower(bx, bz, 18.0, 18.0, h, m, gaps);
        self.tree(bx - 13.0, bz + 13.0, 1.0);
        self.tree(bx + 13.0, bz - 13.0, 1.0);
        self.jersey(bx, bz + 14.0, 4.0, true);
    }

    fn block_skyscraper(&mut self, bx: f32, bz: f32) {
        self.tower(bx, bz, 20.0, 20.0, 58.0, Mat::Tower(3), &[]);
        self.building(bx, bz + 13.0, 26.0, 6.0, 1, Mat::Facade(5), BOpts { doors: &['s'], props: false, roof_props: false, ..Default::default() });
        for [x, z, w, d] in [[0.0, -4.0, 6.0, 0.4], [0.0, 4.0, 6.0, 0.4], [-3.0, 0.0, 0.4, 8.4], [3.0, 0.0, 0.4, 8.4]] {
            self.w.add_strip(bx + x, 59.2, bz + z, w, 0.04, d, Mat::StripW);
        }
        self.w.add_block(bx + 7.0, 59.1, bz - 7.0, 0.5, 9.0, 0.5, Mat::Steel);
        self.w.add_strip(bx + 7.0, 68.2, bz - 7.0, 0.5, 0.5, 0.5, Mat::Red);
    }
}

/// Arma el Distrito Sur en `w` (que tiene que haberse creado con a = 164).
pub fn build(w: &mut World) -> CityData {
    let full_a = w.a;
    let a = CORE_A;
    let mut b = B { w, r: Rng::new(SEED), spots: Vec::new() };
    // El muro y el horizonte de la web quedaban a ±164 m; ahora ahí siguen los barrios nuevos.
    // Se consumen los mismos números aleatorios para que el resto del distrito salga idéntico.
    for _ in 0..40 {
        for _ in 0..4 {
            b.r.next();
        }
    }
    // veredas y líneas de calle
    for bx in BC {
        for bz in BC {
            b.w.add_box(bx, bz, BLOCK, 0.16, BLOCK, Mat::Sidewalk);
        }
    }
    let streets = [-110.0f32, -66.0, -22.0, 22.0, 66.0, 110.0];
    let avenue = 156.0f32;
    for s in streets.iter().copied().chain([-avenue, avenue, -198.0, 198.0, -242.0, 242.0]) {
        let m = if (s.abs() - 156.0).abs() < 1.0 { Mat::LineY } else { Mat::Line };
        let mut t = -full_a + 4.0;
        while t < full_a - 4.0 {
            b.w.add_strip(s, 0.015, t, 0.18, 0.02, 3.0, m);
            b.w.add_strip(t, 0.015, s, 3.0, 0.02, 0.18, m);
            t += 7.0;
        }
    }
    // autos abandonados en las calles
    let mut vehicles = Vec::new();
    for _ in 0..70 {
        let s = streets[b.r.idx(streets.len())];
        let t = b.r.signed() * (a - 14.0);
        let ax = b.r.next() < 0.5;
        let lane = if b.r.next() < 0.5 { -4.2 } else { 4.2 };
        let (x, z) = if ax { (t, s + lane) } else { (s + lane, t) };
        if (z + 22.0).abs() < 7.0 && ax {
            continue;
        }
        // en la web eran decorado; acá se pueden manejar (los de la autopista siguen fijos)
        let Mat::Car(c) = b.mat_car() else { unreachable!() };
        vehicles.push((Vec3::new(x, 0.0, z), if ax { std::f32::consts::FRAC_PI_2 } else { 0.0 }, c));
    }
    // autopista elevada sobre la calle z = -22, con autos arriba
    b.highway(-22.0, -118.0, 118.0, 7.14);
    for i in 0..9 {
        let x = -100.0 + i as f32 * 24.0 + b.r.f() * 6.0;
        let z = -22.0 + if b.r.next() < 0.5 { -2.6 } else { 2.6 };
        let m = b.mat_car();
        b.car(x, z, true, m, 7.14);
    }
    let outposts = b.outposts();
    let mut used: Vec<(i32, i32)> = outposts.iter().map(|o| (o.cx as i32, o.cz as i32)).collect();
    for bx in BC {
        for bz in BC {
            if !used.contains(&(bx as i32, bz as i32)) {
                for (lx, lz) in [(-15.0, -15.0), (15.0, 15.0), (15.0, -15.0), (-15.0, 15.0)] {
                    b.lamp(bx + lx, bz + lz);
                }
            }
        }
    }
    // refugio
    let safe_pos;
    {
        let (cx, cz) = (-132.0, 132.0);
        used.push((cx as i32, cz as i32));
        b.building(cx, cz, 14.0, 11.0, 2, Mat::Facade(0), BOpts { doors: &['n', 'e'], ..Default::default() });
        safe_pos = Vec3::new(cx - 3.0, 1.0, cz + 2.0);
        b.w.add_block(cx - 3.0, 0.16, cz + 2.6, 1.6, 0.9, 0.9, Mat::Container(1));
        b.w.add_strip(cx - 3.0, 1.1, cz + 2.6, 1.2, 0.06, 0.7, Mat::Mint);
        for (x, z) in [(-12.0, -12.0), (12.0, 12.0), (-12.0, 12.0)] {
            b.tree(cx + x, cz + z, 1.1);
        }
    }
    // distrito por zonas: residencial al oeste, centro de torres con pasarelas, industrial al este
    #[rustfmt::skip]
    let plan: &[((i32, i32), &str)] = &[
        ((-132, -132), "aptsFE"), ((-88, -132), "park"), ((0, -132), "hall"), ((44, -132), "apts"), ((132, -132), "containers"),
        ((-132, -88), "court"), ((-88, -88), "sky"), ((-44, -88), "aptsB"), ((0, -88), "tower40"), ((44, -88), "market"), ((88, -88), "aptsFE"), ((132, -88), "industrial"),
        ((-88, -44), "apts"), ((-44, -44), "tower30"), ((0, -44), "twinA1"), ((44, -44), "twinA2"), ((88, -44), "court"), ((132, -44), "hall"),
        ((-132, 0), "aptsFE"), ((-88, 0), "terraces"), ((-44, 0), "twinB2"), ((44, 0), "plaza"), ((88, 0), "market"), ((132, 0), "aptsB"),
        ((-132, 44), "hall"), ((-88, 44), "aptsB"), ((-44, 44), "twinB1"), ((0, 44), "tower24"), ((44, 44), "court"), ((88, 44), "lot"),
        ((-132, 88), "aptsFE"), ((-88, 88), "park"), ((-44, 88), "court"), ((0, 88), "apts"), ((44, 88), "tower2"), ((88, 88), "hall"), ((132, 88), "containers"),
        ((-88, 132), "lot"), ((-44, 132), "apts"), ((44, 132), "hall"), ((88, 132), "industrial"),
    ];
    for bx in BC {
        for bz in BC {
            let key = (bx as i32, bz as i32);
            if used.contains(&key) {
                continue;
            }
            let kind = plan.iter().find(|(k, _)| *k == key).map_or("apts", |(_, v)| *v);
            match kind {
                "apts" => b.block_apts(bx, bz, false, false),
                "aptsB" => b.block_apts(bx, bz, false, true),
                "aptsFE" => b.block_apts(bx, bz, true, true),
                "court" => b.block_courtyard(bx, bz),
                "park" => b.block_park(bx, bz),
                "terraces" => b.block_terrace_park(bx, bz),
                "market" => b.block_market(bx, bz),
                "plaza" => b.block_plaza(bx, bz),
                "hall" => b.block_hall(bx, bz),
                "containers" => b.block_containers(bx, bz),
                "industrial" => b.block_industrial(bx, bz),
                "lot" => b.block_lot(bx, bz),
                "sky" => b.block_skyscraper(bx, bz),
                "tower40" => b.block_tower(bx, bz, 40.0, &[]),
                "tower30" => b.block_tower(bx, bz, 30.0, &[]),
                "tower24" => {
                    b.block_tower(bx, bz, 24.0, &[]);
                    b.tower(bx + 12.5, bz + 12.5, 5.0, 5.0, 12.0, Mat::Tower(1), &[]);
                }
                "tower2" => {
                    let m = b.mat_tower();
                    b.tower(bx - 4.0, bz - 4.0, 14.0, 14.0, 26.0, m, &[]);
                    let m = b.mat_tower();
                    b.tower(bx + 9.0, bz + 9.0, 8.0, 8.0, 14.0, m, &[]);
                }
                "twinA1" => b.block_tower(bx, bz, 34.0, &['e']),
                "twinA2" => b.block_tower(bx, bz, 34.0, &['w']),
                "twinB1" => b.block_tower(bx, bz, 26.0, &['n']),
                "twinB2" => b.block_tower(bx, bz, 26.0, &['s']),
                _ => unreachable!("manzana desconocida: {kind}"),
            }
        }
    }
    // pasarelas entre torres gemelas, por encima de las calles
    b.skybridge(9.0, -44.0, 35.0, -44.0, 34.0);
    b.skybridge(-44.0, 9.0, -44.0, 35.0, 26.0);

    // barrios nuevos alrededor del distrito original
    let mut outposts = outposts;
    let mut stations = Vec::new();
    outer(&mut b, &mut vehicles, &mut outposts, &mut stations);

    let drone_routes = vec![
        vec![[-286.0, -286.0], [286.0, -286.0], [286.0, -198.0], [-286.0, -198.0]],
        vec![[198.0, -286.0], [286.0, -286.0], [286.0, 286.0], [198.0, 286.0]],
        vec![[-286.0, 198.0], [286.0, 198.0], [286.0, 286.0], [-286.0, 286.0]],
        vec![[-286.0, -286.0], [-198.0, -286.0], [-198.0, 286.0], [-286.0, 286.0]],
        vec![[-156.0, -156.0], [156.0, -156.0], [156.0, -66.0], [-156.0, -66.0]],
        vec![[-110.0, -156.0], [-110.0, 156.0], [-22.0, 156.0], [-22.0, -110.0]],
        vec![[156.0, 156.0], [22.0, 156.0], [22.0, 22.0], [156.0, 22.0]],
        vec![[66.0, -110.0], [66.0, 110.0], [110.0, 110.0], [110.0, -110.0]],
        vec![[-66.0, 66.0], [-156.0, 66.0], [-156.0, 110.0], [-66.0, 110.0]],
    ];
    let loot_spots = std::mem::take(&mut b.spots);
    let streets = vec![-242.0, -198.0, -156.0, -110.0, -66.0, -22.0, 22.0, 66.0, 110.0, 156.0, 198.0, 242.0];
    CityData { outposts, safe_pos, drone_routes, vehicles, loot_spots, streets, stations }
}

/// Barrios exteriores: un anillo de manzanas alrededor del Distrito Sur con cuatro temas,
/// campamentos enemigos, una segunda autopista, más autos y el muro y horizonte nuevos.
fn outer(b: &mut B, vehicles: &mut Vec<(Vec3, f32, u8)>, outposts: &mut Vec<Outpost>, stations: &mut Vec<city2::Station>) {
    use std::f32::consts::{FRAC_PI_2, PI};
    let a = b.w.a;
    b.r = Rng::new(SEED_OUTER);
    let is_outer = |x: f32, z: f32| x.abs() > CORE_A || z.abs() > CORE_A;
    // muro nuevo y horizonte más allá
    for [x, z, ww, d] in [[0.0, -a, a * 2.0 + 2.0, 2.0], [0.0, a, a * 2.0 + 2.0, 2.0], [-a, 0.0, 2.0, a * 2.0 + 2.0], [a, 0.0, 2.0, a * 2.0 + 2.0]] {
        b.w.add_box(x, z, ww, 12.0, d, Mat::Concrete);
    }
    for i in 0..64 {
        let t = b.r.signed() * a;
        let o = a + 14.0 + b.r.f() * 40.0;
        let ww = 14.0 + b.r.f() * 18.0;
        let h = 22.0 + b.r.f() * 60.0;
        let (x, z) = match i % 4 {
            0 => (t, -o),
            1 => (t, o),
            2 => (-o, t),
            _ => (o, t),
        };
        b.w.add_visual(x, h / 2.0, z, ww, h, ww, Mat::Tower((i % 4) as u8), false);
    }
    // campamentos: (centro, nombre, portones)
    let camps: [(f32, f32, &str, &[char]); 6] = [
        (-220.0, -264.0, "Campamento del Puerto", &['s', 'e']),
        (176.0, -220.0, "Campamento de los Silos", &['w', 's']),
        (264.0, 220.0, "Campamento del Este", &['w', 'n']),
        (88.0, 264.0, "Campamento del Parque", &['n', 'w']),
        (-176.0, 220.0, "Campamento Jardín", &['n', 'e']),
        (-264.0, -44.0, "Campamento del Cerro", &['e', 's']),
    ];
    let mut used: Vec<(i32, i32)> = Vec::new();
    for (k, &(cx, cz, name, gates)) in camps.iter().enumerate() {
        used.push((cx as i32, cz as i32));
        b.w.add_box(cx, cz, BLOCK, 0.16, BLOCK, Mat::Sidewalk);
        let mut op = Outpost::new(OutpostId::Camp, name, name, "", "", 'C', cx, cz);
        op.camp = true;
        b.outpost_walls(cx, cz, gates);
        // galpón o contenedores y cajas de cobertura
        if k % 2 == 0 {
            b.hall(cx + 4.0, cz - 5.0, 12.0, 9.0, 4.0, Mat::Facade((3 + k) as u8 % mat::FACADES), &['s']);
        } else {
            b.w.add_container(cx - 6.0, cz - 7.0, true, 2, k % 4);
            b.w.add_container(cx + 7.0, cz - 4.0, false, 1, (k + 1) % 4);
            op.cover_around(cx - 6.0, cz - 7.0, 6.2, 2.6);
            op.cover_around(cx + 7.0, cz - 4.0, 2.6, 6.2);
        }
        for (x, z) in [(-8.0, 6.0), (6.0, 8.0), (-2.0, 1.0), (9.0, -10.0)] {
            b.op_crate(&mut op, cx + x, cz + z, 2.0, 1.2, 2.0);
        }
        op.patrol = vec![[cx - 11.0, cz - 11.0], [cx + 11.0, cz - 11.0], [cx + 11.0, cz + 11.0], [cx - 11.0, cz + 11.0]];
        op.unit(cx - 10.0, cz + 10.0, Role::Patrol).unit(cx + 9.0, cz - 9.0, Role::Patrol).unit(cx + 2.0, cz + 4.0, Role::Guard).unit(cx - 5.0, cz - 3.0, Role::Guard);
        if k % 2 == 1 {
            op.sniper(cx - 6.0, cz - 7.0, 5.4);
        }
        op.turret(cx + 12.0, cz + 12.0, -PI * 0.25);
        outposts.push(op);
    }
    // Ciudad 2.0: plaza, bulevar, diagonal, rotondas y subte ocupan sus manzanas
    used.extend(city2::reserved().blocks);
    stations.extend(city2::build(b));
    // manzanas por barrio
    for bx in BC_ALL {
        for bz in BC_ALL {
            if !is_outer(bx, bz) || used.contains(&(bx as i32, bz as i32)) {
                continue;
            }
            b.w.add_box(bx, bz, BLOCK, 0.16, BLOCK, Mat::Sidewalk);
            for (lx, lz) in [(-15.0, -15.0), (15.0, 15.0), (15.0, -15.0), (-15.0, 15.0)] {
                b.lamp(bx + lx, bz + lz);
            }
            let r = b.r.next();
            if bz < -CORE_A {
                // Puerto: industria, contenedores, tanques y galpones
                match (r * 6.0) as u32 {
                    0 | 1 => b.block_containers(bx, bz),
                    2 | 3 => b.block_industrial(bx, bz),
                    4 => b.block_hall(bx, bz),
                    _ => b.block_lot(bx, bz),
                }
            } else if bz > CORE_A {
                // Barrio Jardín: casas bajas, patios y parques
                let k = ((bx * 7.0 + bz * 3.0).abs() as u32) % 17;
                b.street_stall(bx + 9.0, bz + 14.2, k);
                match (r * 6.0) as u32 {
                    0 | 1 => {
                        b.block_apts(bx, bz, false, true);
                        b.storefront(bx - 6.0, bz - 5.0, 14.0, 12.0, k);
                    }
                    2 => b.block_courtyard(bx, bz),
                    3 => b.block_park(bx, bz),
                    4 => b.block_plaza(bx, bz),
                    _ => b.block_market(bx, bz),
                }
            } else if bx > CORE_A {
                // Centro Financiero: torres altas
                match (r * 5.0) as u32 {
                    0 => b.block_skyscraper(bx, bz),
                    1 => b.block_tower(bx, bz, 48.0, &[]),
                    2 => b.block_tower(bx, bz, 36.0, &[]),
                    3 => b.block_plaza(bx, bz),
                    _ => b.block_courtyard(bx, bz),
                }
            } else {
                // El Cerro: terrazas, mercados, departamentos y estacionamientos
                let k = ((bx * 5.0 + bz * 11.0).abs() as u32) % 13;
                match (r * 6.0) as u32 {
                    0 => b.block_terrace_park(bx, bz),
                    1 => b.block_market(bx, bz),
                    2 | 3 => {
                        b.block_apts(bx, bz, true, true);
                        b.storefront(bx - 6.0, bz - 5.0, 14.0, 12.0, k);
                        b.street_stall(bx - 9.0, bz + 14.2, k + 1);
                    }
                    4 => b.block_lot(bx, bz),
                    _ => b.block_park(bx, bz),
                }
            }
        }
    }
    // segunda autopista, sobre la calle del puerto
    b.highway(-242.0, -280.0, 280.0, 7.14);
    for i in 0..10 {
        let x = -250.0 + i as f32 * 52.0 + b.r.f() * 8.0;
        let z = -242.0 + if b.r.next() < 0.5 { -2.6 } else { 2.6 };
        let m = b.mat_car();
        b.car(x, z, true, m, 7.14);
    }
    // más autos manejables en las calles de los barrios nuevos
    let streets = [-242.0f32, -198.0, -156.0, 156.0, 198.0, 242.0, -110.0, -66.0, -22.0, 22.0, 66.0, 110.0];
    let mut placed = 0;
    while placed < 60 {
        let s = streets[b.r.idx(streets.len())];
        let t = b.r.signed() * (a - 14.0);
        let ax = b.r.next() < 0.5;
        let lane = if b.r.next() < 0.5 { -4.2 } else { 4.2 };
        let (x, z) = if ax { (t, s + lane) } else { (s + lane, t) };
        let Mat::Car(c) = b.mat_car() else { unreachable!() };
        // solo afuera del distrito original y lejos de la autopista nueva
        if !is_outer(x, z) || ((z + 242.0).abs() < 7.0 && ax) {
            continue;
        }
        vehicles.push((Vec3::new(x, 0.0, z), if ax { FRAC_PI_2 } else { 0.0 }, c));
        placed += 1;
    }
}

#[cfg(test)]
mod tests {
    use crate::maps::{MapId, build_level};

    #[test]
    fn siete_puestos_y_refugio() {
        let l = build_level(MapId::Ciudad);
        let c = l.city.unwrap();
        assert_eq!(c.outposts.iter().filter(|o| !o.camp).count(), 7);
        assert_eq!(c.outposts.iter().filter(|o| o.camp).count(), 6);
        assert_eq!(c.stations.len(), 3);
        // del andén se puede caminar: hay piso a la profundidad de la estación
        for st in &c.stations {
            let g = l.world.ground_at(st.platform.x, st.platform.z, st.platform.y + 0.2);
            assert!((g - st.platform.y).abs() < 0.05, "{}: piso {g}", st.name);
        }
        assert!(c.outposts.iter().all(|o| !o.patrol.is_empty() && !o.units.is_empty()));
        assert_eq!(c.outposts.iter().filter(|o| o.obj_pos.is_none() && !o.camp).count(), 1); // el mando
        // se reaparece corrido 1,5 m de la caja de reabastecimiento, sobre la vereda
        let g = l.world.ground_at(c.safe_pos.x + 1.5, c.safe_pos.z + 1.5, 1.0);
        assert!(g < 1.0, "piso del refugio {g}");
    }

    #[test]
    fn cabina_de_la_grua_alcanzable() {
        let l = build_level(MapId::Ciudad);
        let c = l.city.unwrap();
        let obra = c.outposts.iter().find(|o| o.icon == 'O').unwrap();
        let p = obra.obj_pos.unwrap();
        // hay piso a ~41 m debajo del objetivo
        let g = l.world.ground_at(p.x, p.z, p.y);
        assert!((g - 41.2).abs() < 0.3, "piso de la grúa {g}");
    }
}
