//! Ciudad 2.0: lo que hace que el Distrito Sur se sienta una ciudad y no una grilla.
//! Plaza Mayor, bulevar curvo, Diagonal Norte, rotondas, locales con vidriera, toldo y cartel,
//! puestos callejeros y cuatro estaciones de subte con escaleras desde la calle.
//! Es un submódulo de `city` para usar sus constructores (`B`).

use super::*;
use std::f32::consts::{FRAC_PI_2, FRAC_PI_4, PI, TAU};

/// Estación de subte: dónde está el andén (para viajar) y la boca en la calle.
#[derive(Clone, Debug)]
pub struct Station {
    pub name: &'static str,
    /// Punto del andén donde se toma el subte.
    pub platform: Vec3,
    /// Boca de la escalera en la calle.
    pub entrance: Vec3,
    /// Centro de las vías frente al andén (a la altura del foso): donde para el tren.
    pub track: Vec3,
}

/// Profundidad del andén (nivel del piso de la estación).
pub const STATION_FLOOR: f32 = -6.12;
/// Piso del foso de las vías.
pub const PIT: f32 = -7.4;
/// Techo de los túneles.
pub const TUNNEL_TOP: f32 = -2.4;
/// Largo de cada túnel a los costados de la estación.
pub const TUNNEL: f32 = 80.0;

/// Zonas que ocupan las piezas especiales: el anillo exterior no pone manzanas comunes ahí.
pub struct Reserved {
    pub blocks: Vec<(i32, i32)>,
}

/// Bulevar curvo: centro y radio del arco, y su rango de ángulos.
const ARC_C: Vec2 = Vec2::new(150.0, 0.0);
const ARC_R: f32 = 92.0;
const ARC_SPAN: f32 = 0.78;
/// Diagonal Norte: de la esquina del puerto hacia el centro.
const DIAG_A: Vec2 = Vec2::new(-286.0, -286.0);
const DIAG_B: Vec2 = Vec2::new(-168.0, -168.0);

use glam::Vec2;

fn dist_to_segment(p: Vec2, a: Vec2, b: Vec2) -> f32 {
    let ab = b - a;
    let t = ((p - a).dot(ab) / ab.length_squared()).clamp(0.0, 1.0);
    p.distance(a + ab * t)
}

/// Manzanas del anillo que quedan para la plaza, el bulevar, la diagonal y las estaciones.
pub fn reserved() -> Reserved {
    let mut blocks = vec![(176, 176), (220, 176), (176, 220), (220, 220)];
    for bx in BC_ALL {
        for bz in BC_ALL {
            let p = Vec2::new(bx, bz);
            let rel = p - ARC_C;
            let ang = rel.y.atan2(rel.x);
            let on_arc = ang.abs() < ARC_SPAN + 0.15 && (rel.length() - ARC_R).abs() < 34.0 && bx > CORE_A;
            let on_diag = dist_to_segment(p, DIAG_A, DIAG_B) < 30.0;
            if on_arc || on_diag {
                blocks.push((bx as i32, bz as i32));
            }
        }
    }
    // manzanas de estación
    for (x, z) in [(-176, 132), (264, -176)] {
        blocks.push((x, z));
    }
    Reserved { blocks }
}

impl B<'_> {
    /// Local comercial macizo (para calles giradas): fachada, vidriera, toldo y cartel.
    #[allow(clippy::too_many_arguments)]
    fn shop_rot(&mut self, cx: f32, cz: f32, w: f32, d: f32, h: f32, yaw: f32, k: u32) {
        let facade = Mat::Facade((k % mat::FACADES as u32) as u8);
        self.w.add_block_rot(cx, 0.16, cz, w, h, d, yaw, facade);
        // el frente mira hacia −Z local
        let (s, c) = yaw.sin_cos();
        let front = |off: f32, dx: f32| -> (f32, f32) {
            let (lx, lz) = (dx, -d / 2.0 - off);
            (cx + lx * c + lz * s, cz - lx * s + lz * c)
        };
        let (gx, gz) = front(0.03, 0.0);
        self.w.add_visual_rot(gx, 1.5, gz, w * 0.8, 2.2, 0.06, yaw, Mat::CarGlass, false);
        let (ax, az) = front(0.7, 0.0);
        self.w.add_visual_rot(ax, 2.85, az, w * 0.86, 0.12, 1.4, yaw, Mat::Awning((k % mat::AWNINGS as u32) as u8), true);
        let (sx, sz) = front(0.08, 0.0);
        let sign = [Mat::Screen, Mat::Red, Mat::Mint, Mat::Strip][(k % 4) as usize];
        self.w.add_visual_rot(sx, 3.55, sz, w * 0.5, 0.55, 0.08, yaw, sign, false);
        self.w.add_visual_rot(cx, h + 0.16 + 0.5, cz, w - 0.6, 1.0, d - 0.6, yaw, Mat::Roof, true);
    }

    /// Vidriera, toldo y cartel en la planta baja de un edificio común (frente sur).
    pub(super) fn storefront(&mut self, cx: f32, cz: f32, w: f32, d: f32, k: u32) {
        let zf = cz + d / 2.0;
        self.w.add_visual(cx, 1.4, zf + 0.04, w * 0.7, 1.9, 0.05, Mat::CarGlass, false);
        self.w.add_visual(cx, 2.75, zf + 0.75, w * 0.78, 0.12, 1.5, Mat::Awning((k % mat::AWNINGS as u32) as u8), true);
        let sign = [Mat::Screen, Mat::Red, Mat::Mint, Mat::Strip][(k % 4) as usize];
        self.w.add_visual(cx, 3.35, zf + 0.06, w * 0.46, 0.5, 0.08, sign, false);
    }

    /// Puesto callejero en la vereda, mirando a la calle.
    pub(super) fn street_stall(&mut self, x: f32, z: f32, k: u32) {
        self.stall(x, z, Mat::Awning((k % mat::AWNINGS as u32) as u8));
        // mercadería de colores sobre el mostrador
        for i in 0..3 {
            let m = [Mat::Red, Mat::Yellow, Mat::Hedge][((k + i) % 3) as usize];
            self.w.add_visual(x - 0.8 + i as f32 * 0.8, 1.12, z, 0.5, 0.22, 0.5, m, false);
        }
    }

    /// Rotonda en un cruce: isla octogonal con césped, monumento y marcas de giro.
    fn roundabout(&mut self, x: f32, z: f32) {
        let r = 2.9;
        for yaw in [0.0, FRAC_PI_4] {
            self.w.add_block_rot(x, 0.0, z, r * 1.84, 0.3, r * 1.84, yaw, Mat::Jersey);
            self.w.add_visual_rot(x, 0.31, z, r * 1.7, 0.02, r * 1.7, yaw, Mat::Hedge, false);
        }
        self.w.add_block(x, 0.3, z, 0.9, 2.6, 0.9, Mat::Statue);
        self.w.add_strip(x, 3.0, z, 0.3, 0.3, 0.3, Mat::Lamp);
        // flechas de giro alrededor
        for k in 0..12 {
            let a = k as f32 / 12.0 * TAU;
            self.w.add_visual_rot(x + a.cos() * 5.3, 0.016, z + a.sin() * 5.3, 1.6, 0.02, 0.18, -a + FRAC_PI_2, Mat::Line, false);
        }
    }

    /// Plaza Mayor (2 × 2 manzanas): fuente, glorieta, senderos en diagonal, árboles, bancos y puestos.
    fn plaza_mayor(&mut self, stations: &mut Vec<Station>) {
        let (cx, cz) = (198.0, 198.0);
        let half = 38.0;
        // empedrado en cuatro piezas que dejan el hueco de la escalera del subte
        let hole = [cx + 18.0, cz + 18.0, cx + 22.0, cz + 26.6];
        self.paving_with_hole(cx, cz, half, hole);
        // senderos en diagonal y anillo
        for yaw in [FRAC_PI_4, -FRAC_PI_4] {
            self.w.add_visual_rot(cx, 0.165, cz, 3.0, 0.01, half * 2.6, yaw, Mat::InFloor, false);
        }
        for k in 0..24 {
            let a = k as f32 / 24.0 * TAU;
            self.w.add_visual_rot(cx + a.cos() * 14.0, 0.166, cz + a.sin() * 14.0, 4.0, 0.01, 2.6, -a + FRAC_PI_2, Mat::InFloor, false);
        }
        // fuente octogonal con estatua
        for yaw in [0.0, FRAC_PI_4] {
            self.w.add_block_rot(cx, 0.16, cz, 11.0, 0.7, 11.0, yaw, Mat::Jersey);
            self.w.add_visual_rot(cx, 0.88, cz, 10.0, 0.03, 10.0, yaw, Mat::Water, false);
        }
        self.w.add_block(cx, 0.86, cz, 1.6, 1.6, 1.6, Mat::Jersey);
        self.w.add_block(cx, 2.46, cz, 0.9, 2.4, 0.9, Mat::Statue);
        // glorieta: postes en octógono y techo
        let (gx, gz) = (cx - 22.0, cz - 22.0);
        for k in 0..8 {
            let a = k as f32 / 8.0 * TAU;
            self.w.add_block(gx + a.cos() * 3.6, 0.16, gz + a.sin() * 3.6, 0.2, 3.0, 0.2, Mat::Pole);
        }
        for yaw in [0.0, FRAC_PI_4] {
            self.w.add_block_rot(gx, 0.16, gz, 8.0, 0.5, 8.0, yaw, Mat::Wood);
            self.w.add_visual_rot(gx, 3.3, gz, 8.6, 0.25, 8.6, yaw, Mat::Awning(1), true);
        }
        // árboles en anillo y bancos mirando la fuente
        for k in 0..16 {
            let a = k as f32 / 16.0 * TAU + 0.2;
            self.tree(cx + a.cos() * 26.0, cz + a.sin() * 26.0, 1.1 + (k % 3) as f32 * 0.15);
        }
        for k in 0..8 {
            let a = k as f32 / 8.0 * TAU + 0.39;
            let (bx, bz) = (cx + a.cos() * 18.0, cz + a.sin() * 18.0);
            self.w.add_block_rot(bx, 0.16, bz, 2.6, 0.5, 0.7, -a + FRAC_PI_2, Mat::Wood);
        }
        // feria de puestos sobre un costado
        for k in 0..6 {
            self.street_stall(cx - 30.0 + k as f32 * 6.0, cz + 33.0, k);
        }
        for (dx, dz) in [(-34.0, -34.0), (34.0, -34.0), (-34.0, 34.0), (34.0, 34.0), (0.0, -35.0), (-35.0, 0.0)] {
            self.lamp(cx + dx, cz + dz);
        }
        // locales mirando a la plaza en el borde norte
        for k in 0..5 {
            self.shop_rot(cx - 28.0 + k as f32 * 14.0, cz - 33.0, 11.0, 7.0, 5.0 + (k % 3) as f32 * 2.0, PI, 3 + k);
        }
        stations.push(self.station("Plaza Mayor", hole));
    }

    /// Losa de vereda en un cuadrado con un hueco rectangular (boca de subte).
    fn paving_with_hole(&mut self, cx: f32, cz: f32, half: f32, h: [f32; 4]) {
        let (x0, z0, x1, z1) = (cx - half, cz - half, cx + half, cz + half);
        let mut slab = |a0: f32, a1: f32, b0: f32, b1: f32| {
            if a1 - a0 > 0.05 && b1 - b0 > 0.05 {
                self.w.add_box((a0 + a1) / 2.0, (b0 + b1) / 2.0, a1 - a0, 0.16, b1 - b0, Mat::Sidewalk);
            }
        };
        slab(x0, x1, z0, h[1]);
        slab(x0, x1, h[3], z1);
        slab(x0, h[0], h[1], h[3]);
        slab(h[2], x1, h[1], h[3]);
    }

    /// Estación de subte bajo un hueco: boca con techito en la calle, escalera que baja hacia +Z,
    /// hall con molinetes, andén, foso con vías a lo largo de X y túneles a los dos lados por donde
    /// entra y sale el tren (ver `subway.rs`).
    fn station(&mut self, name: &'static str, hole: [f32; 4]) -> Station {
        self.w.holes.push(hole);
        let (x0, z0, x1, z1) = (hole[0], hole[1], hole[2], hole[3]);
        let xc = (x0 + x1) / 2.0;
        let fl = STATION_FLOOR;
        let n = (-fl / STEP).round() as i32;
        let depth = (z1 - z0) / n as f32;
        // escalones: cada uno es una caja desde abajo hasta su altura; se entra por z0
        for k in 1..=n {
            let top = -(k as f32) * STEP;
            let zc = z0 + (k as f32 - 0.5) * depth;
            self.w.add_block(xc, fl - 0.5, zc, x1 - x0, top - (fl - 0.5), depth, Mat::Concrete);
            // nariz amarilla de cada escalón
            self.w.add_visual(xc, top + 0.005, zc - depth / 2.0 + 0.05, x1 - x0 - 0.1, 0.01, 0.08, Mat::LineY, false);
        }
        // paredes del pozo (bajo la vereda) con zócalo de azulejos
        for x in [x0 - 0.15, x1 + 0.15] {
            self.w.add_block(x, fl - 0.5, (z0 + z1) / 2.0, 0.3, -fl + 0.5, z1 - z0, Mat::Tile);
        }
        self.w.add_block(xc, fl - 0.5, z0 - 0.15, x1 - x0 + 0.6, -fl + 0.5, 0.3, Mat::Concrete);
        // en la calle: barandas a los costados y en el fondo (la entrada queda libre)
        for x in [x0 - 0.05, x1 + 0.05] {
            self.w.add_block(x, 0.0, (z0 + z1) / 2.0 + 0.4, 0.08, 1.05, z1 - z0 - 0.8, Mat::Pole);
        }
        self.w.add_block(xc, 0.0, z1 + 0.05, x1 - x0 + 0.2, 1.05, 0.08, Mat::Pole);
        // techito de vidrio sobre la boca y cartel "M" a un costado de la entrada
        for (x, z) in [(x0 - 0.1, z0 + 0.2), (x1 + 0.1, z0 + 0.2), (x0 - 0.1, z0 + 3.5), (x1 + 0.1, z0 + 3.5)] {
            self.w.add_block(x, 0.0, z, 0.12, 2.7, 0.12, Mat::Pole);
        }
        self.w.add_visual(xc, 2.75, z0 + 1.85, x1 - x0 + 0.6, 0.08, 3.8, Mat::CarGlass, true);
        self.w.add_strip(xc, 2.95, z0 + 0.1, x1 - x0 + 0.6, 0.35, 0.1, Mat::Screen);
        self.w.add_block(x0 - 0.8, 0.0, z0 - 0.4, 0.15, 3.4, 0.15, Mat::Pole);
        self.w.add_strip(x0 - 0.8, 3.6, z0 - 0.4, 0.9, 0.9, 0.12, Mat::Screen);
        self.w.add_strip(x0 - 0.8, 3.6, z0 - 0.47, 0.5, 0.5, 0.02, Mat::StripW);

        // hall: 30 × 14; el andén va de z1 a la línea amarilla y después está el foso
        let (hx, hw) = (xc, 30.0);
        let (hz0, edge, hz1) = (z1, z1 + 9.0, z1 + 13.6);
        let ceil = -1.0;
        self.w.add_block(hx, fl - 0.5, (hz0 + edge) / 2.0, hw, 0.5, edge - hz0, Mat::InFloor);
        self.w.add_block(hx, ceil, (hz0 + hz1) / 2.0, hw, 0.5, hz1 - hz0, Mat::Concrete);
        // pared del lado de la escalera (con el paso) y pared del fondo, detrás de las vías
        for (a0, a1) in [(hx - hw / 2.0, x0 - 0.3), (x1 + 0.3, hx + hw / 2.0)] {
            self.w.add_block((a0 + a1) / 2.0, fl, hz0 - 0.15, a1 - a0, ceil - fl, 0.3, Mat::Tile);
        }
        self.w.add_block(hx, PIT, hz1 + 0.15, hw, ceil - PIT, 0.3, Mat::Tile);
        // paredes de los extremos: abiertas a la altura del túnel
        for sx in [-1.0f32, 1.0] {
            let x = hx + sx * (hw / 2.0 + 0.15);
            self.w.add_block(x, fl, (hz0 + edge) / 2.0, 0.3, ceil - fl, edge - hz0, Mat::Tile);
            self.w.add_block(x, TUNNEL_TOP, (edge + hz1) / 2.0, 0.3, ceil - TUNNEL_TOP, hz1 - edge, Mat::Tile);
        }
        // foso de las vías: piso, durmientes y rieles a lo largo de toda la estación y los túneles
        let tz = (edge + hz1) / 2.0;
        let len = hw + TUNNEL * 2.0;
        self.w.add_block(hx, PIT - 0.5, tz, len, 0.5, hz1 - edge, Mat::Gravel);
        self.w.add_block(hx, PIT, edge + 0.15, hw, fl - PIT, 0.3, Mat::Concrete);
        let mut x = hx - len / 2.0 + 0.4;
        while x < hx + len / 2.0 {
            self.w.add_visual(x, PIT + 0.06, tz, 0.3, 0.12, 2.6, Mat::Wood, false);
            x += 0.9;
        }
        for dz in [-0.72, 0.72] {
            self.w.add_strip(hx, PIT + 0.17, tz + dz, len, 0.1, 0.1, Mat::Steel);
        }
        // túneles: paredes y techo oscuros con luces cada 10 m
        for sx in [-1.0f32, 1.0] {
            let cx = hx + sx * (hw / 2.0 + TUNNEL / 2.0);
            self.w.add_block(cx, PIT, edge - 0.15, TUNNEL, TUNNEL_TOP - PIT + 0.5, 0.3, Mat::Concrete);
            self.w.add_block(cx, PIT, hz1 + 0.15, TUNNEL, TUNNEL_TOP - PIT + 0.5, 0.3, Mat::Concrete);
            self.w.add_block(cx, TUNNEL_TOP, tz, TUNNEL, 0.4, hz1 - edge + 0.6, Mat::Concrete);
            let mut k = 5.0;
            while k < TUNNEL {
                self.w.add_strip(hx + sx * (hw / 2.0 + k), TUNNEL_TOP - 0.6, hz1 - 0.02, 0.6, 0.12, 0.04, Mat::Lamp);
                k += 10.0;
            }
        }
        // andén: borde con línea amarilla y botones de alerta, bancos, columnas, carteles y molinetes
        self.w.add_strip(hx, fl + 0.005, edge - 0.45, hw - 0.4, 0.01, 0.3, Mat::LineY);
        for k in 0..6 {
            self.w.add_strip(hx - 12.5 + k as f32 * 5.0, ceil - 0.03, (hz0 + hz1) / 2.0, 2.6, 0.04, 0.45, Mat::Lamp);
        }
        for k in 0..4 {
            let px = hx - 10.5 + k as f32 * 7.0;
            self.w.add_block(px, fl, edge - 3.2, 0.6, ceil - fl, 0.6, Mat::Pillar);
            if k < 3 {
                self.w.add_block(px + 3.5, fl, edge - 3.2, 2.2, 0.45, 0.6, Mat::Wood);
            }
        }
        // carteles con el nombre en la pared del fondo (se ven desde el andén, a través de las vías)
        for k in [-1.0f32, 1.0] {
            self.w.add_strip(hx + k * 7.0, -3.2, hz1 - 0.02, 7.0, 0.9, 0.04, Mat::Screen);
            self.w.add_strip(hx + k * 7.0, -3.2, hz1 - 0.05, 6.0, 0.35, 0.02, Mat::StripW);
        }
        // molinetes al pie de la escalera
        for k in 0..4 {
            let mx = x0 - 3.0 + k as f32 * 1.3 + if k >= 2 { 4.0 + (x1 - x0) - 2.6 } else { 0.0 };
            self.w.add_block(mx, fl, z1 + 2.2, 0.3, 1.0, 1.2, Mat::Steel);
        }
        self.w.add_block(x0 - 7.0, fl, z1 + 2.2, 6.0, 1.0, 0.3, Mat::Jersey);
        self.w.add_block(x1 + 7.0, fl, z1 + 2.2, 6.0, 1.0, 0.3, Mat::Jersey);
        Station { name, platform: Vec3::new(hx, fl, edge - 1.4), entrance: Vec3::new(xc, 0.0, z0 - 1.0), track: Vec3::new(hx, PIT, tz) }
    }

    /// Manzana con boca de subte y plazoleta.
    fn station_block(&mut self, bx: f32, bz: f32, name: &'static str, stations: &mut Vec<Station>) {
        let hole = [bx - 2.0, bz - 10.0, bx + 2.0, bz - 1.4];
        self.paving_with_hole(bx, bz, BLOCK / 2.0, hole);
        for (dx, dz) in [(-11.0, -11.0), (11.0, -11.0), (-11.0, 11.0), (11.0, 11.0)] {
            self.tree(bx + dx, bz + dz, 1.0);
        }
        for k in 0..2 {
            self.street_stall(bx - 8.0 + k as f32 * 16.0, bz + 12.0, k + 1);
        }
        for (lx, lz) in [(-15.0, -15.0), (15.0, 15.0)] {
            self.lamp(bx + lx, bz + lz);
        }
        stations.push(self.station(name, hole));
    }

    /// Bulevar curvo en el Centro Financiero: veredas giradas a los dos lados, torres que siguen
    /// la curva por fuera y un parque angosto por dentro.
    fn boulevard(&mut self) {
        let segs = 26;
        for k in 0..segs {
            let a = -ARC_SPAN + (k as f32 + 0.5) / segs as f32 * ARC_SPAN * 2.0;
            let da = ARC_SPAN * 2.0 / segs as f32;
            let dir = Vec2::new(a.cos(), a.sin());
            let yaw = -a; // el largo del tramo sigue la tangente
            for (r, w) in [(ARC_R - 9.0, 4.0), (ARC_R + 9.0, 4.0)] {
                let p = ARC_C + dir * r;
                self.w.add_block_rot(p.x, 0.0, p.y, w, 0.16, r * da + 0.3, yaw, Mat::Sidewalk);
            }
            // línea central
            if k % 2 == 0 {
                let p = ARC_C + dir * ARC_R;
                self.w.add_visual_rot(p.x, 0.016, p.y, 0.18, 0.02, ARC_R * da * 0.6, yaw, Mat::LineY, false);
            }
            // torres por fuera (una cada dos tramos)
            if k % 2 == 1 {
                let p = ARC_C + dir * (ARC_R + 21.0);
                let h = 24.0 + ((k * 7) % 5) as f32 * 7.0;
                self.w.add_block_rot(p.x, 0.0, p.y, 14.0, h, 12.0, yaw, Mat::Tower(((k / 2) % 4) as u8));
                self.w.add_visual_rot(p.x, h + 0.55, p.y, 13.4, 1.1, 11.4, yaw, Mat::Roof, true);
            } else {
                let p = ARC_C + dir * (ARC_R + 17.0);
                self.shop_rot(p.x, p.y, 10.0, 6.0, 4.5, yaw - FRAC_PI_2, k);
            }
            // parque y árboles por dentro
            if k % 2 == 0 {
                let p = ARC_C + dir * (ARC_R - 16.0);
                self.tree(p.x, p.y, 1.2);
            }
            if k % 4 == 0 {
                let p = ARC_C + dir * (ARC_R - 12.0);
                self.lamp(p.x, p.y);
            }
        }
    }

    /// Diagonal Norte: avenida a 45° por el puerto con locales a los dos lados.
    fn diagonal(&mut self) {
        let d = (DIAG_B - DIAG_A).normalize();
        let n = Vec2::new(-d.y, d.x);
        let len = DIAG_A.distance(DIAG_B);
        let yaw = d.x.atan2(d.y); // el largo sigue la diagonal
        let steps = (len / 8.0) as i32;
        for k in 0..steps {
            let t = (k as f32 + 0.5) * 8.0;
            let c = DIAG_A + d * t;
            for side in [-1.0, 1.0] {
                let p = c + n * side * 9.0;
                self.w.add_block_rot(p.x, 0.0, p.y, 4.0, 0.16, 8.3, yaw, Mat::Sidewalk);
            }
            if k % 2 == 0 {
                self.w.add_visual_rot(c.x, 0.016, c.y, 0.18, 0.02, 4.0, yaw, Mat::Line, false);
            }
            if k % 2 == 1 {
                for side in [-1.0f32, 1.0] {
                    let p = c + n * side * 16.5;
                    // el frente mira a la avenida
                    let shop_yaw = yaw + if side > 0.0 { FRAC_PI_2 } else { -FRAC_PI_2 };
                    self.shop_rot(p.x, p.y, 11.0, 7.0, 5.0 + (k % 4) as f32 * 2.5, shop_yaw, k as u32 + if side > 0.0 { 1 } else { 0 });
                }
            }
            if k % 3 == 0 {
                let p = c + n * 10.5;
                self.lamp(p.x, p.y);
            }
        }
    }
}

/// Arma todo lo de Ciudad 2.0. Devuelve las estaciones de subte.
pub(super) fn build(b: &mut B) -> Vec<Station> {
    let mut stations = Vec::new();
    b.plaza_mayor(&mut stations);
    b.boulevard();
    b.diagonal();
    b.station_block(-176.0, 132.0, "Refugio", &mut stations);
    b.station_block(264.0, -176.0, "Puerto Este", &mut stations);
    // rotondas en cruces del anillo exterior que no tocan las piezas especiales
    for (x, z) in [(-198.0, 198.0), (-242.0, 66.0), (242.0, -242.0 + 44.0), (66.0, 242.0), (-66.0, -198.0)] {
        b.roundabout(x, z);
    }
    stations
}

/// ¿La manzana va con locales? (barrios Jardín y Cerro, alrededor de la plaza)
pub fn shopping_block(bx: f32, bz: f32) -> bool {
    bz > CORE_A || bx < -CORE_A
}
