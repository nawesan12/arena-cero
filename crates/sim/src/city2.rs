//! Piezas de la ciudad que usa `town`: estaciones de subte (boca, escalera, andén, vías y
//! túneles), vereda con hueco, puestos callejeros y el decorado de azoteas (carteles y neón).
//! Es un submódulo de `city` para usar sus constructores (`B`).

use super::*;

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


impl B<'_> {
    /// Puesto callejero en la vereda, mirando a la calle.
    pub(super) fn street_stall(&mut self, x: f32, z: f32, k: u32) {
        self.stall(x, z, Mat::Awning((k % mat::AWNINGS as u32) as u8));
        // mercadería de colores sobre el mostrador
        for i in 0..3 {
            let m = [Mat::Red, Mat::Yellow, Mat::Hedge][((k + i) % 3) as usize];
            self.w.add_visual(x - 0.8 + i as f32 * 0.8, 1.12, z, 0.5, 0.22, 0.5, m, false);
        }
    }

    /// Losa de vereda en un cuadrado con un hueco rectangular (boca de subte).
    pub(super) fn paving_with_hole(&mut self, cx: f32, cz: f32, half: f32, h: [f32; 4]) {
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
    pub(super) fn station(&mut self, name: &'static str, hole: [f32; 4]) -> Station {
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

}

/// Número pseudoaleatorio 0..1 a partir de una posición: el decorado no consume el RNG de la
/// ciudad, así el resto del distrito sale idéntico.
fn hash(x: f32, z: f32, k: f32) -> f32 {
    ((x * 12.9898 + z * 78.233 + k * 37.719).sin() * 43_758.547).fract().abs()
}

/// Decorado (nuevo, solo visual): carteles en azoteas, neón en bordes de techos y objetos de
/// calle según el barrio (pallets en el Puerto, bolardos y macetas en el Centro Financiero,
/// cercos en el Barrio Jardín, tachos y cajones en El Cerro).
pub(super) fn decor(w: &mut World, streets: &[f32]) {
    // azoteas (también las giradas del bulevar y la diagonal)
    let roofs: Vec<(Vec3, Vec3, f32)> = w.visuals.iter().filter(|v| v.mat == Mat::Roof && v.size.x >= 6.0 && v.size.z >= 6.0 && v.size.y <= 1.2 && v.center.y >= 5.0).map(|v| (v.center, v.size, v.yaw)).collect();
    let mut used: Vec<Vec3> = Vec::new();
    for (c, sz, yaw) in roofs {
        if used.iter().any(|u| u.distance(c) < 14.0) {
            continue;
        }
        let top = c.y + sz.y / 2.0;
        // del marco local del techo (eje x a lo ancho, z a lo hondo) al mundo
        let (sy, cy) = yaw.sin_cos();
        let at = |lx: f32, lz: f32| Vec3::new(c.x + lx * cy + lz * sy, 0.0, c.z - lx * sy + lz * cy);
        let h = hash(c.x, c.z, 1.0);
        // pocos: un cartel cada tanto en los techos más grandes
        if h < 0.1 && sz.x.max(sz.z) >= 10.0 {
            used.push(c);
            let k = (hash(c.x, c.z, 2.0) * 4.0) as u8;
            let along_x = sz.x >= sz.z;
            let side = if hash(c.x, c.z, 3.0) < 0.5 { -1.0 } else { 1.0 };
            let width = (if along_x { sz.x } else { sz.z } * 0.7).min(11.0);
            let ph = width * 0.45;
            let (lx, lz) = if along_x { (0.0, side * (sz.z / 2.0 - 1.2)) } else { (side * (sz.x / 2.0 - 1.2), 0.0) };
            let p = at(lx, lz);
            let (w_, d_) = if along_x { (width, 0.18) } else { (0.18, width) };
            w.add_visual_rot(p.x, top + 1.3 + ph / 2.0, p.z, w_, ph, d_, yaw, Mat::Billboard(k), true);
            for e in [-0.4f32, 0.4] {
                let q = if along_x { at(e * width, lz) } else { at(lx, e * width) };
                w.add_visual_rot(q.x, top + 0.65, q.z, 0.18, 1.3, 0.18, yaw, Mat::Pole, true);
            }
        } else if h > 0.93 {
            used.push(c);
            let k = (hash(c.x, c.z, 4.0) * 4.0) as u8;
            let y = top + 0.08;
            for (lx, lz, ww, dd) in [(0.0, -sz.z / 2.0, sz.x, 0.12), (0.0, sz.z / 2.0, sz.x, 0.12), (-sz.x / 2.0, 0.0, 0.12, sz.z), (sz.x / 2.0, 0.0, 0.12, sz.z)] {
                let p = at(lx, lz);
                w.add_visual_rot(p.x, y, p.z, ww, 0.12, dd, yaw, Mat::Neon(k), false);
            }
        }
    }
    // objetos de calle por barrio, sobre la vereda a 7 m del eje
    let a = w.a - 6.0;
    for &s in streets {
        let mut t = -a;
        while t < a {
            for side in [-7.0f32, 7.0] {
                for along_x in [true, false] {
                    let (x, z) = if along_x { (t, s + side) } else { (s + side, t) };
                    // ni en las esquinas ni en el centro original
                    if streets.iter().any(|o| (o - t).abs() < 9.0) || (x.abs() < CORE_A && z.abs() < CORE_A) {
                        continue;
                    }
                    let r = hash(x, z, 5.0);
                    if r > 0.45 {
                        continue;
                    }
                    let free = |p: Vec3, w: &World| w.box_at(p).is_none() && w.ground_at(p.x, p.z, 1.0) < 0.4 && !w.in_hole(p.x, p.z);
                    if !free(Vec3::new(x, 0.6, z), w) || !free(Vec3::new(x + 0.8, 0.6, z + 0.8), w) || !free(Vec3::new(x - 0.8, 0.6, z - 0.8), w) {
                        continue;
                    }
                    let g = w.ground_at(x, z, 1.0);
                    if z < -CORE_A {
                        // Puerto: pallets apilados
                        let n = 1 + (r * 8.0) as u32 % 3;
                        for i in 0..n {
                            w.add_visual(x, g + 0.08 + i as f32 * 0.16, z, 1.2, 0.14, 1.0, Mat::Wood, true);
                        }
                    } else if x > CORE_A {
                        // Centro Financiero: bolardos y macetas
                        if r < 0.25 {
                            for k in -1..=1 {
                                let o = k as f32 * 1.4;
                                let (bx, bz) = if along_x { (x + o, z) } else { (x, z + o) };
                                w.add_visual(bx, g + 0.45, bz, 0.22, 0.9, 0.22, Mat::Bollard, true);
                            }
                        } else {
                            w.add_visual(x, g + 0.3, z, 1.2, 0.6, 1.2, Mat::Jersey, true);
                            w.add_visual(x, g + 0.75, z, 1.0, 0.4, 1.0, Mat::Hedge, true);
                        }
                    } else if z > CORE_A {
                        // Barrio Jardín: cerco bajo de madera
                        let (lw, ld) = if along_x { (3.0, 0.08) } else { (0.08, 3.0) };
                        w.add_visual(x, g + 0.5, z, lw, 0.08, ld, Mat::Wood, true);
                        w.add_visual(x, g + 0.25, z, lw, 0.08, ld, Mat::Wood, true);
                        for k in [-1.4f32, 0.0, 1.4] {
                            let (px, pz) = if along_x { (x + k, z) } else { (x, z + k) };
                            w.add_visual(px, g + 0.35, pz, 0.1, 0.7, 0.1, Mat::Wood, true);
                        }
                    } else {
                        // El Cerro: tacho y cajones
                        w.add_visual(x, g + 0.5, z, 0.7, 1.0, 0.7, Mat::Bollard, true);
                        if r < 0.2 {
                            w.add_visual(x + 1.0, g + 0.25, z, 0.6, 0.5, 0.5, Mat::Crate, true);
                        }
                    }
                }
            }
            t += 12.0;
        }
    }
}
