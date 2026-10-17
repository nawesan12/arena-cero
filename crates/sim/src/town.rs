//! Ciudad 3.0 (nuevo): trazado porteño hecho a mano. Reemplaza la grilla de 13 × 13 manzanas
//! iguales de la versión web.
//!
//! - **Calles**: 10 de norte a sur y 10 de este a oeste con anchos distintos: calles de 12 m,
//!   avenidas de 26 m con bulevar arbolado y la Av. 9 de Julio de 40 m con el Obelisco.
//! - **Manzanas llenas**: lotes de frente angosto entre medianeras sobre la línea de la vereda,
//!   con ochava en cada esquina y pulmón de manzana adentro (patios y árboles). Las alturas
//!   saltan de un lote al otro, como en Buenos Aires.
//! - **Diagonales Norte y Sur**: salen de las esquinas de la Plaza Mayor y llegan a la 9 de Julio,
//!   con los edificios girados siguiendo la diagonal.
//! - **Barrios con tipología propia**: microcentro de 5 a 14 pisos, torres de vidrio en el
//!   financiero, docks de ladrillo en el Puerto, casas bajas con jardín en Barrio Jardín y casas de
//!   chapa de colores en El Cerro.
//!
//! El decorado usa su propio `Rng`, así el resto sigue determinista con la semilla de la ciudad.

use super::*;
use glam::Vec2;
use std::f32::consts::{FRAC_PI_2, FRAC_PI_4};

/// Una calle recta de punta a punta del mapa.
#[derive(Clone, Copy, Debug)]
pub struct St {
    /// Coordenada del eje (x para las que van de norte a sur, z para las otras).
    pub c: f32,
    pub w: f32,
    pub name: &'static str,
}

impl St {
    pub fn avenue(&self) -> bool {
        self.w >= 20.0
    }
}

/// Calles que van de norte a sur (x fija), de oeste a este.
pub const XS: [St; 10] = [
    St { c: -273.0, w: 12.0, name: "Calle de los Cerros" },
    St { c: -215.0, w: 12.0, name: "Calle Lavalle" },
    St { c: -155.0, w: 26.0, name: "Av. Callao" },
    St { c: -95.0, w: 12.0, name: "Calle Tucumán" },
    St { c: -30.0, w: 40.0, name: "Av. 9 de Julio" },
    St { c: 35.0, w: 12.0, name: "Calle Viamonte" },
    St { c: 95.0, w: 12.0, name: "Calle Florida" },
    St { c: 155.0, w: 26.0, name: "Av. Leandro Alem" },
    St { c: 215.0, w: 12.0, name: "Calle Juncal" },
    St { c: 272.0, w: 14.0, name: "Av. Costanera" },
];

/// Calles que van de este a oeste (z fija), de norte a sur.
pub const ZS: [St; 10] = [
    St { c: -272.0, w: 14.0, name: "Av. del Puerto" },
    St { c: -212.0, w: 12.0, name: "Calle Reconquista" },
    St { c: -152.0, w: 26.0, name: "Av. Corrientes" },
    St { c: -92.0, w: 12.0, name: "Calle San Martín" },
    St { c: -30.0, w: 12.0, name: "Calle Belgrano" },
    St { c: 30.0, w: 26.0, name: "Av. de Mayo" },
    St { c: 90.0, w: 12.0, name: "Calle Moreno" },
    St { c: 150.0, w: 12.0, name: "Calle Alsina" },
    St { c: 210.0, w: 26.0, name: "Av. San Juan" },
    St { c: 268.0, w: 12.0, name: "Calle Garay" },
];

/// Ancho de vereda entre el cordón y las fachadas.
pub const VEREDA: f32 = 3.0;

/// Las dos diagonales: de las esquinas oeste de la Plaza Mayor a la 9 de Julio.
pub const DIAG_W: f32 = 16.0;
pub fn diagonals() -> [(Vec2, Vec2, &'static str); 2] {
    let (px0, pz0, _, pz1) = block_rect(6, 4);
    [(Vec2::new(px0, pz0), Vec2::new(XS[4].c, ZS[2].c), "Diagonal Norte"), (Vec2::new(px0, pz1), Vec2::new(XS[4].c, ZS[7].c), "Diagonal Sur")]
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Barrio {
    Centro,
    Financiero,
    Puerto,
    Jardin,
    Cerro,
}

/// Manzana: rectángulo entre cuatro calles.
#[derive(Clone, Copy, Debug)]
pub struct Block {
    pub x0: f32,
    pub z0: f32,
    pub x1: f32,
    pub z1: f32,
    pub barrio: Barrio,
    /// Índices en `XS` / `ZS` de la calle de la izquierda y la de arriba.
    pub i: usize,
    pub j: usize,
}

impl Block {
    pub fn center(&self) -> Vec2 {
        Vec2::new((self.x0 + self.x1) / 2.0, (self.z0 + self.z1) / 2.0)
    }
}

/// Rectángulo de la manzana (i, j): entre las calles i e i+1 (x) y j y j+1 (z).
pub fn block_rect(i: usize, j: usize) -> (f32, f32, f32, f32) {
    (XS[i].c + XS[i].w / 2.0, ZS[j].c + ZS[j].w / 2.0, XS[i + 1].c - XS[i + 1].w / 2.0, ZS[j + 1].c - ZS[j + 1].w / 2.0)
}

pub fn block_center(i: usize, j: usize) -> Vec2 {
    let (x0, z0, x1, z1) = block_rect(i, j);
    Vec2::new((x0 + x1) / 2.0, (z0 + z1) / 2.0)
}

pub fn barrio(i: usize, j: usize) -> Barrio {
    if j == 0 {
        Barrio::Puerto
    } else if i >= 7 && j <= 4 {
        Barrio::Financiero
    } else if j >= 6 || (i >= 7 && j == 5) {
        Barrio::Jardin
    } else if i <= 1 {
        Barrio::Cerro
    } else {
        Barrio::Centro
    }
}

/// Qué va en cada manzana especial (el resto son lotes, docks o torres según el barrio).
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Use {
    Lots,
    Docks,
    Yard,
    Towers,
    /// Plaza con estación de subte (nombre de la estación).
    Plaza(&'static str),
    Park,
    Outpost(OutpostId),
    Camp(usize),
    Refugio,
}

/// Puestos en el orden de `outposts()`: Antena, Subestación, Oficinas, Arsenal, Mando, Obra, Trenes.
pub const OUTPOST_BLOCKS: [(usize, usize); 7] = [(0, 3), (8, 7), (8, 2), (4, 7), (7, 4), (5, 1), (2, 0)];
pub const CAMP_BLOCKS: [(usize, usize, &str, &[char]); 6] = [
    (0, 0, "Campamento del Puerto", &['s', 'e']),
    (8, 0, "Campamento de los Silos", &['w', 's']),
    (8, 5, "Campamento del Este", &['w', 'n']),
    (6, 8, "Campamento del Parque", &['n', 'w']),
    (0, 8, "Campamento Jardín", &['n', 'e']),
    (0, 5, "Campamento del Cerro", &['e', 's']),
];

pub fn block_use(i: usize, j: usize) -> Use {
    let ids = [OutpostId::Antena, OutpostId::Sub, OutpostId::Datos, OutpostId::Arsenal, OutpostId::Mando, OutpostId::Obra, OutpostId::Trenes];
    if let Some(k) = OUTPOST_BLOCKS.iter().position(|b| *b == (i, j)) {
        return Use::Outpost(ids[k]);
    }
    if let Some(k) = CAMP_BLOCKS.iter().position(|b| (b.0, b.1) == (i, j)) {
        return Use::Camp(k);
    }
    match (i, j) {
        (6, 4) => Use::Plaza("Plaza Mayor"),
        (3, 2) => Use::Plaza("Obelisco"),
        (1, 7) => Use::Refugio,
        (2, 8) | (6, 7) => Use::Park,
        (3, 5) => Use::Park,
        _ => match barrio(i, j) {
            Barrio::Puerto => {
                if i % 2 == 1 {
                    Use::Docks
                } else {
                    Use::Yard
                }
            }
            Barrio::Financiero => Use::Towers,
            _ => Use::Lots,
        },
    }
}

/// Distancia de un punto a un segmento.
fn seg_dist(p: Vec2, a: Vec2, b: Vec2) -> f32 {
    let ab = b - a;
    let t = ((p - a).dot(ab) / ab.length_squared()).clamp(0.0, 1.0);
    p.distance(a + ab * t)
}

/// ¿Un rectángulo (centro, medio ancho, medio fondo) toca el corredor de alguna diagonal (más un margen)?
pub fn near_diag(c: Vec2, hw: f32, hd: f32, margin: f32) -> bool {
    let r = hw.hypot(hd);
    diagonals().iter().any(|(a, b, _)| seg_dist(c, *a, *b) < DIAG_W / 2.0 + margin + r * 0.75)
}

/// Lo que devuelve el armado de la ciudad nueva.
pub struct Town {
    pub outpost_pos: [(f32, f32); 7],
    pub camps: Vec<(f32, f32, &'static str, &'static [char])>,
    pub stations: Vec<super::city2::Station>,
    pub safe_pos: Vec3,
    pub blocks: Vec<Block>,
    pub vehicles: Vec<(Vec3, f32, u8)>,
}

/// Armado completo de calles y manzanas (los puestos los arma `outposts()` en `outpost_pos`).
pub(super) fn build(b: &mut B) -> Town {
    let mut r = Rng::new(20261020);
    let mut town = Town { outpost_pos: [(0.0, 0.0); 7], camps: Vec::new(), stations: Vec::new(), safe_pos: Vec3::ZERO, blocks: Vec::new(), vehicles: Vec::new() };
    for (k, &(i, j)) in OUTPOST_BLOCKS.iter().enumerate() {
        let c = block_center(i, j);
        town.outpost_pos[k] = (c.x, c.y);
    }
    streets(b, &mut r, &mut town);
    for i in 0..XS.len() - 1 {
        for j in 0..ZS.len() - 1 {
            let (x0, z0, x1, z1) = block_rect(i, j);
            let blk = Block { x0, z0, x1, z1, barrio: barrio(i, j), i, j };
            town.blocks.push(blk);
            let u = block_use(i, j);
            // vereda de toda la manzana (salvo donde pasa una diagonal o hay subte)
            if !matches!(u, Use::Plaza(_) | Use::Refugio) {
                sidewalk(b, &blk);
            }
            match u {
                Use::Lots => lots(b, &mut r, &blk),
                Use::Docks => docks(b, &mut r, &blk),
                Use::Yard => yard(b, &mut r, &blk),
                Use::Towers => towers(b, &mut r, &blk),
                Use::Plaza(name) => {
                    let st = plaza(b, &mut r, &blk, name);
                    town.stations.push(st);
                }
                Use::Park => park(b, &mut r, &blk),
                Use::Refugio => {
                    let (st, safe) = refugio(b, &mut r, &blk);
                    town.stations.push(st);
                    town.safe_pos = safe;
                }
                Use::Outpost(_) => outpost_ring(b, &mut r, &blk),
                Use::Camp(k) => {
                    let c = blk.center();
                    let (_, _, name, gates) = CAMP_BLOCKS[k];
                    town.camps.push((c.x, c.y, name, gates));
                    outpost_ring(b, &mut r, &blk);
                }
            }
            if !matches!(u, Use::Plaza(_) | Use::Refugio | Use::Park) {
                super::props::sidewalks(b, &mut r, &blk);
            }
        }
    }
    diagonal_buildings(b, &mut r);
    obelisco(b);
    super::props::intersections(b, &mut r);
    // estaciones en el orden de antes: Plaza Mayor primero (la campaña la usa)
    town.stations.sort_by_key(|s| if s.name == "Plaza Mayor" { 0 } else if s.name == "Refugio" { 1 } else { 2 });
    // autos estacionados a lo largo de las calles
    for _ in 0..90 {
        let ax = r.next() < 0.5;
        let list = if ax { &ZS } else { &XS };
        let s = list[1 + r.idx(list.len() - 2)];
        let t = r.signed() * 250.0;
        let side = if r.next() < 0.5 { -1.0 } else { 1.0 };
        let lane = side * (s.w / 2.0 - 1.4);
        let (x, z) = if ax { (t, s.c + lane) } else { (s.c + lane, t) };
        // ni en los cruces ni sobre una diagonal
        let cross = if ax { &XS } else { &ZS };
        if cross.iter().any(|o| (o.c - t).abs() < o.w / 2.0 + 5.0) || near_diag(Vec2::new(x, z), 1.0, 2.5, 1.0) {
            continue;
        }
        town.vehicles.push((Vec3::new(x, 0.0, z), if ax { FRAC_PI_2 } else { 0.0 }, r.idx(6) as u8));
    }
    town
}

/* ---------- calles ---------- */

fn streets(b: &mut B, r: &mut Rng, _town: &mut Town) {
    let ext = 280.0;
    for (list, along_x) in [(&ZS, true), (&XS, false)] {
        for s in list.iter() {
            // líneas: amarilla doble al medio en avenidas, blanca cortada en calles
            let mut t = -ext;
            while t < ext {
                let crossing = (if along_x { &XS } else { &ZS }).iter().any(|o| (o.c - t).abs() < o.w / 2.0 + 1.0);
                if !crossing {
                    for off in if s.avenue() { vec![-s.w / 2.0 + 6.4, s.w / 2.0 - 6.4] } else { vec![0.0] } {
                        let (x, z) = if along_x { (t, s.c + off) } else { (s.c + off, t) };
                        let (w, d) = if along_x { (3.0, 0.15) } else { (0.15, 3.0) };
                        b.w.add_strip(x, 0.015, z, w, 0.02, d, Mat::Line);
                    }
                }
                t += 7.0;
            }
            // bulevar arbolado al medio de las avenidas
            if s.avenue() {
                let mw = if s.w >= 40.0 { 6.0 } else { 3.0 };
                let cross = if along_x { &XS } else { &ZS };
                let mut spans: Vec<(f32, f32)> = Vec::new();
                let mut a = -ext;
                for o in cross.iter() {
                    let (c0, c1) = (o.c - o.w / 2.0 - 1.0, o.c + o.w / 2.0 + 1.0);
                    if c0 > a {
                        spans.push((a, c0));
                    }
                    a = c1;
                }
                if a < ext {
                    spans.push((a, ext));
                }
                for (a0, a1) in spans {
                    let (cx, cz, w, d) = if along_x { ((a0 + a1) / 2.0, s.c, a1 - a0, mw) } else { (s.c, (a0 + a1) / 2.0, mw, a1 - a0) };
                    // las diagonales cortan el bulevar
                    if near_diag(Vec2::new(cx, cz), 0.5, 0.5, 6.0) && (a1 - a0) < 30.0 {
                        continue;
                    }
                    b.w.add_box(cx, cz, w, 0.22, d, Mat::Jersey);
                    b.w.add_visual(cx, 0.24, cz, w - 0.4, 0.04, d - 0.4, Mat::Grass, false);
                    let mut u = a0 + 5.0;
                    while u < a1 - 4.0 {
                        let (tx, tz) = if along_x { (u, s.c) } else { (s.c, u) };
                        if !near_diag(Vec2::new(tx, tz), 1.0, 1.0, 2.0) {
                            b.tree(tx, tz, 0.9 + r.f() * 0.4);
                        }
                        u += 11.0 + r.f() * 4.0;
                    }
                }
            }
        }
    }
    // diagonales: asfalto girado con líneas
    for (a, c, _) in diagonals() {
        let d = c - a;
        let len = d.length();
        let mid = (a + c) / 2.0;
        let yaw = (-d.y).atan2(d.x);
        b.w.add_visual_rot(mid.x, 0.18, mid.y, len, 0.03, DIAG_W, yaw, Mat::Asphalt, false);
        let nrm = Vec2::new(-d.y, d.x) / len;
        for side in [-1.0f32, 1.0] {
            let e = mid + nrm * side * (DIAG_W / 2.0 + 0.15);
            b.w.add_visual_rot(e.x, 0.2, e.y, len, 0.06, 0.3, yaw, Mat::Jersey, false);
        }
        let n = (len / 7.0) as i32;
        for k in 0..n {
            let p = a + d * ((k as f32 + 0.5) / n as f32);
            b.w.add_visual_rot(p.x, 0.2, p.y, 3.0, 0.02, 0.15, yaw, Mat::Line, false);
        }
    }
}

/// Vereda de la manzana: una losa. Donde la cruza una diagonal, el asfalto de la diagonal va
/// encima (a la misma altura que la vereda), con cordones girados a los dos lados.
fn sidewalk(b: &mut B, blk: &Block) {
    let c = blk.center();
    let (w, d) = (blk.x1 - blk.x0, blk.z1 - blk.z0);
    b.w.add_box(c.x, c.y, w, 0.16, d, Mat::Sidewalk);
}

/* ---------- edificios entre medianeras ---------- */

/// Para qué lado mira el frente: 'n' (−Z), 's' (+Z), 'w' (−X), 'e' (+X).
type Face = char;

/// Cuántos pisos según el barrio (y si da a una avenida, más).
fn floors(r: &mut Rng, barrio: Barrio, avenue: bool) -> u32 {
    let base = match barrio {
        Barrio::Centro => 4 + r.idx(7) as u32 + if r.next() < 0.12 { 5 } else { 0 },
        Barrio::Financiero => 6 + r.idx(6) as u32,
        Barrio::Puerto => 3 + r.idx(2) as u32,
        Barrio::Jardin => 1 + r.idx(3) as u32,
        Barrio::Cerro => 2 + r.idx(3) as u32,
    };
    base + if avenue && barrio != Barrio::Jardin { 2 + r.idx(4) as u32 } else { 0 }
}

/// Edificio de un lote: volumen macizo con medianeras lisas, fachada con ventanas sobre la calle,
/// planta baja con local o puerta, cornisa, balcones y techo con tanque de agua.
/// (x0, z0)-(x1, z1) es el lote; `face` el lado que da a la calle.
#[allow(clippy::too_many_arguments)]
fn lot_building(b: &mut B, r: &mut Rng, x0: f32, z0: f32, x1: f32, z1: f32, face: Face, n: u32, barrio: Barrio) {
    let (w, d) = (x1 - x0, z1 - z0);
    if w < 2.5 || d < 2.5 {
        return;
    }
    let (cx, cz) = ((x0 + x1) / 2.0, (z0 + z1) / 2.0);
    let base = 0.16;
    let h = n as f32 * FH + 0.4;
    let style = match barrio {
        Barrio::Cerro => 3,
        Barrio::Puerto => 2,
        Barrio::Jardin => if r.next() < 0.6 { 0 } else { 1 },
        _ => r.idx(3) as u8 % 2 + if r.next() < 0.2 { 1 } else { 0 },
    };
    // volumen (medianeras lisas)
    let side_mat = match style {
        2 => Mat::Facade(3),
        3 => Mat::Chapa(r.idx(6) as u8),
        _ => Mat::Facade([0u8, 2, 4, 7][r.idx(4)]),
    };
    b.w.add_block(cx, base, cz, w, h, d, side_mat);
    // techo
    b.w.add_visual(cx, base + h + 0.03, cz, w - 0.2, 0.06, d - 0.2, Mat::Roof, false);
    // fachada sobre la calle: piel con ventanas, planta baja aparte
    let skin = 0.08;
    let front = |along: bool, at: f32| if along { (cx, at, w, skin) } else { (at, cz, skin, d) };
    let (fx, fz, fw, fd) = match face {
        'n' => front(true, z0 - skin / 2.0),
        's' => front(true, z1 + skin / 2.0),
        'w' => front(false, x0 - skin / 2.0),
        _ => front(false, x1 + skin / 2.0),
    };
    let front_mat = match style {
        3 => Mat::Chapa(r.idx(6) as u8),
        s => Mat::Front(s * 4 + r.idx(4) as u8),
    };
    if n > 1 {
        b.w.add_visual(fx, base + FH + (h - FH) / 2.0, fz, fw, h - FH, fd, front_mat, true);
    }
    let pb = if barrio == Barrio::Jardin && r.next() < 0.6 { Mat::Front(r.idx(4) as u8) } else { Mat::Shop(r.idx(6) as u8) };
    b.w.add_visual(fx, base + FH / 2.0, fz, fw, FH, fd, pb, true);
    // toldos, carteles, aires acondicionados y rejas
    let out_dir = match face {
        'n' => Vec2::new(0.0, -1.0),
        's' => Vec2::new(0.0, 1.0),
        'w' => Vec2::new(-1.0, 0.0),
        _ => Vec2::new(1.0, 0.0),
    };
    let front_len = if matches!(face, 'n' | 's') { w } else { d };
    let shop = matches!(pb, Mat::Shop(_));
    super::props::facade_details(b.w, r, fx, fz, front_len, out_dir, n, shop);
    if !shop && r.next() < 0.6 {
        super::props::house_railing(b.w, fx, fz, front_len, out_dir);
    }
    // cornisa y, a veces, balcones corridos
    let out = |dist: f32| match face {
        'n' => (0.0, -dist),
        's' => (0.0, dist),
        'w' => (-dist, 0.0),
        _ => (dist, 0.0),
    };
    let (ox, oz) = out(0.25);
    let (cwid, cdep) = if matches!(face, 'n' | 's') { (w, 0.5) } else { (0.5, d) };
    b.w.add_visual(fx + ox, base + h - 0.2, fz + oz, cwid, 0.4, cdep, Mat::Jersey, false);
    if n >= 3 && style < 2 && r.next() < 0.55 {
        let (bx, bz) = out(0.5);
        let (bw, bd) = if matches!(face, 'n' | 's') { (w - 1.0, 1.0) } else { (1.0, d - 1.0) };
        for f in 1..n {
            let y = base + f as f32 * FH;
            b.w.add_visual(fx + bx, y - 0.1, fz + bz, bw, 0.2, bd, Mat::Concrete, true);
            let (rx, rz) = out(0.98);
            let (rw, rd) = if matches!(face, 'n' | 's') { (w - 1.0, 0.06) } else { (0.06, d - 1.0) };
            b.w.add_visual(fx + rx, y + 0.45, fz + rz, rw, 0.9, rd, Mat::Pole, false);
        }
    }
    // tanque de agua y antena (techo porteño)
    let top = base + h;
    if r.next() < 0.45 && w > 4.0 && d > 4.0 {
        let (tx, tz) = (cx + r.signed() * (w * 0.2), cz + r.signed() * (d * 0.2));
        for (lx, lz) in [(-0.6, -0.6), (0.6, -0.6), (-0.6, 0.6), (0.6, 0.6)] {
            b.w.add_visual(tx + lx, top + 0.6, tz + lz, 0.12, 1.2, 0.12, Mat::Pole, false);
        }
        b.w.add_block(tx, top + 1.2, tz, 1.8, 1.4, 1.8, Mat::Tank);
    }
    // a veces hay una caja en la terraza (botín para los que suben con el gancho)
    if r.next() < 0.08 && w > 5.0 && d > 5.0 {
        b.spots.push(Vec3::new(cx - w * 0.2, top, cz - d * 0.2));
    }
    if r.next() < 0.3 {
        b.w.add_visual(cx + r.signed() * w * 0.3, top + 1.5, cz + r.signed() * d * 0.3, 0.06, 3.0, 0.06, Mat::Pole, false);
    }
    super::props::rooftop(b.w, r, cx, cz, w - 1.0, d - 1.0, top);
    // parapeto
    for (px, pz, pw, pd) in [(cx, z0 + 0.1, w, 0.2), (cx, z1 - 0.1, w, 0.2), (x0 + 0.1, cz, 0.2, d), (x1 - 0.1, cz, 0.2, d)] {
        b.w.add_visual(px, top + 0.45, pz, pw, 0.9, pd, side_mat, false);
    }
}

/// Esquina con ochava: dos volúmenes en L que dejan el corte a 45° y una fachada girada que lo cierra.
/// (cx, cz) es la esquina de la manzana; (sx, sz) hacia adentro de la manzana (±1).
#[allow(clippy::too_many_arguments)]
fn corner_building(b: &mut B, r: &mut Rng, cx: f32, cz: f32, sx: f32, sz: f32, depth: f32, n: u32, barrio: Barrio) {
    let o = 3.0;
    // L: un brazo sobre cada calle, sin el triángulo de la esquina
    let (ax0, ax1) = (cx + sx * o, cx + sx * depth);
    let (az0, az1) = (cz, cz + sz * depth);
    let fz = if sz > 0.0 { 'n' } else { 's' };
    lot_building(b, r, ax0.min(ax1), az0.min(az1), ax0.max(ax1), az0.max(az1), fz, n, barrio);
    let (bx0, bx1) = (cx, cx + sx * o);
    let (bz0, bz1) = (cz + sz * o, cz + sz * depth);
    let fx = if sx > 0.0 { 'w' } else { 'e' };
    lot_building(b, r, bx0.min(bx1), bz0.min(bz1), bx0.max(bx1), bz0.max(bz1), fx, n, barrio);
    // fachada girada de la ochava (con colisión) y su planta baja
    let h = n as f32 * FH + 0.4;
    let mid = Vec2::new(cx + sx * o / 2.0, cz + sz * o / 2.0);
    let yaw = if sx * sz > 0.0 { -FRAC_PI_4 } else { FRAC_PI_4 };
    b.w.add_block_rot(mid.x, 0.16, mid.y, o * std::f32::consts::SQRT_2, h, 0.35, yaw, Mat::Facade(2));
    b.w.add_visual_rot(mid.x - sx * 0.2, 0.16 + FH / 2.0, mid.y - sz * 0.2, o * std::f32::consts::SQRT_2, FH, 0.1, yaw, Mat::Shop(r.idx(6) as u8), true);
    if n > 1 {
        b.w.add_visual_rot(mid.x - sx * 0.2, 0.16 + FH + (h - FH) / 2.0, mid.y - sz * 0.2, o * std::f32::consts::SQRT_2, h - FH, 0.1, yaw, Mat::Front(r.idx(8) as u8), true);
    }
    // relleno del triángulo interior de la ochava (para que no quede un hueco en el techo)
    b.w.add_visual(cx + sx * (o * 0.75), 0.16 + h / 2.0, cz + sz * (o * 0.75), o * 0.5, h, o * 0.5, Mat::Facade(2), false);
}

/// Manzana de lotes: esquinas con ochava, lotes de frente angosto, un pasaje al pulmón y patios.
fn lots(b: &mut B, r: &mut Rng, blk: &Block) {
    let barrio = blk.barrio;
    let depth: f32 = match barrio {
        Barrio::Jardin => 12.0,
        Barrio::Cerro => 11.0,
        _ => 13.5,
    };
    let setback = if barrio == Barrio::Jardin { 3.0 } else { 0.0 };
    let (x0, z0, x1, z1) = (blk.x0 + VEREDA, blk.z0 + VEREDA, blk.x1 - VEREDA, blk.z1 - VEREDA);
    let av = |side: usize| match side {
        0 => ZS[blk.j].avenue(),
        1 => ZS[blk.j + 1].avenue(),
        2 => XS[blk.i].avenue(),
        _ => XS[blk.i + 1].avenue(),
    };
    // esquinas
    let corners = [(x0, z0, 1.0, 1.0, 0usize), (x1, z0, -1.0, 1.0, 0), (x0, z1, 1.0, -1.0, 1), (x1, z1, -1.0, -1.0, 1)];
    for (cx, cz, sx, sz, side) in corners {
        let dd = depth.min((x1 - x0) / 2.0 - 1.0).min((z1 - z0) / 2.0 - 1.0);
        let c = Vec2::new(cx + sx * dd / 2.0, cz + sz * dd / 2.0);
        if near_diag(c, dd / 2.0, dd / 2.0, 1.0) {
            continue;
        }
        let n = floors(r, barrio, av(side)) + if barrio != Barrio::Jardin { 1 } else { 0 };
        if barrio == Barrio::Jardin {
            lot_building(b, r, c.x - dd / 2.0, c.y - dd / 2.0, c.x + dd / 2.0, c.y + dd / 2.0, if sz > 0.0 { 'n' } else { 's' }, n, barrio);
        } else {
            corner_building(b, r, cx, cz, sx, sz, dd, n, barrio);
        }
    }
    // lotes a lo largo de cada lado (entre las esquinas); un pasaje por manzana
    let pasaje_side = r.idx(4);
    for side in 0..4 {
        let along_x = side < 2;
        let (a0, a1) = if along_x { (x0 + depth, x1 - depth) } else { (z0 + depth, z1 - depth) };
        if a1 - a0 < 4.0 {
            continue;
        }
        let pasaje_at = a0 + (a1 - a0) * (0.3 + r.f() * 0.4);
        let mut a = a0;
        while a < a1 - 0.5 {
            let (fmin, fmax) = match barrio {
                Barrio::Jardin => (8.0, 12.0),
                Barrio::Cerro => (5.0, 8.0),
                _ => (7.0, 13.0),
            };
            let mut f = fmin + r.f() * (fmax - fmin);
            if a1 - (a + f) < fmin * 0.7 {
                f = a1 - a;
            }
            let e = (a + f).min(a1);
            // pasaje angosto hacia el pulmón de manzana
            if side == pasaje_side && a <= pasaje_at && e > pasaje_at {
                let p0 = pasaje_at - 1.6;
                let p1 = pasaje_at + 1.6;
                if p0 - a > 2.5 {
                    lot_side(b, r, blk, side, a, p0, depth, setback, av(side));
                }
                if e - p1 > 2.5 {
                    lot_side(b, r, blk, side, p1, e, depth, setback, av(side));
                }
                a = e;
                continue;
            }
            // en Jardín, cada tanto un baldío con árboles
            if barrio == Barrio::Jardin && r.next() < 0.12 {
                let (cx, cz) = side_center(blk, side, (a + e) / 2.0, depth);
                b.tree(cx, cz, 1.0 + r.f() * 0.4);
            } else {
                lot_side(b, r, blk, side, a, e, depth, setback, av(side));
            }
            a = e;
        }
    }
    // pulmón de manzana: piso de patio, árboles y alguna pared baja
    let (ix0, iz0, ix1, iz1) = (x0 + depth, z0 + depth, x1 - depth, z1 - depth);
    if ix1 - ix0 > 3.0 && iz1 - iz0 > 3.0 {
        let c = Vec2::new((ix0 + ix1) / 2.0, (iz0 + iz1) / 2.0);
        if !near_diag(c, (ix1 - ix0) / 2.0, (iz1 - iz0) / 2.0, 0.0) {
            b.w.add_visual(c.x, 0.17, c.y, ix1 - ix0, 0.02, iz1 - iz0, if barrio == Barrio::Centro { Mat::InFloor } else { Mat::Grass }, false);
            b.spots.push(Vec3::new(c.x + 1.0, 0.16, c.y + 1.0));
            for _ in 0..(((ix1 - ix0) * (iz1 - iz0)) / 90.0) as i32 {
                let (tx, tz) = (ix0 + 1.5 + r.f() * (ix1 - ix0 - 3.0), iz0 + 1.5 + r.f() * (iz1 - iz0 - 3.0));
                b.tree(tx, tz, 0.8 + r.f() * 0.5);
            }
            if r.next() < 0.6 {
                b.w.add_box(c.x, c.y, ix1 - ix0 - 2.0, 1.8, 0.25, Mat::Facade(0));
            }
            super::props::backyard(b.w, r, ix0, iz0, ix1, iz1, barrio);
        }
    }
}

/// Centro del frente de un lote en el lado `side` a la coordenada `t`.
fn side_center(blk: &Block, side: usize, t: f32, depth: f32) -> (f32, f32) {
    match side {
        0 => (t, blk.z0 + VEREDA + depth / 2.0),
        1 => (t, blk.z1 - VEREDA - depth / 2.0),
        2 => (blk.x0 + VEREDA + depth / 2.0, t),
        _ => (blk.x1 - VEREDA - depth / 2.0, t),
    }
}

/// Un lote sobre el lado `side` entre `a` y `e` (coordenadas a lo largo del lado).
#[allow(clippy::too_many_arguments)]
fn lot_side(b: &mut B, r: &mut Rng, blk: &Block, side: usize, a: f32, e: f32, depth: f32, setback: f32, avenue: bool) {
    let (x0, z0, x1, z1) = (blk.x0 + VEREDA, blk.z0 + VEREDA, blk.x1 - VEREDA, blk.z1 - VEREDA);
    let gap = 0.05;
    let (lx0, lz0, lx1, lz1, face) = match side {
        0 => (a + gap, z0 + setback, e - gap, z0 + depth, 'n'),
        1 => (a + gap, z1 - depth, e - gap, z1 - setback, 's'),
        2 => (x0 + setback, a + gap, x0 + depth, e - gap, 'w'),
        _ => (x1 - depth, a + gap, x1 - setback, e - gap, 'e'),
    };
    let c = Vec2::new((lx0 + lx1) / 2.0, (lz0 + lz1) / 2.0);
    if near_diag(c, (lx1 - lx0) / 2.0, (lz1 - lz0) / 2.0, 1.0) {
        // lo que la diagonal deja libre se vuelve plazoleta (si no cae sobre el asfalto)
        if !near_diag(c, 0.5, 0.5, 2.0) {
            plazoleta(b, r, c, lx1 - lx0, lz1 - lz0);
        }
        return;
    }
    let n = floors(r, blk.barrio, avenue);
    // algunos lotes son edificios con interior (escaleras, pisos, botín)
    let w = lx1 - lx0;
    let d = lz1 - lz0;
    if blk.barrio != Barrio::Jardin && w >= 10.0 && d >= 10.0 && n <= 6 && r.next() < 0.22 {
        let mat = match blk.barrio {
            Barrio::Puerto => Mat::Facade(3),
            Barrio::Cerro => Mat::Facade(6),
            _ => Mat::Facade([0u8, 1, 2, 4, 5, 7][r.idx(6)]),
        };
        b.building(c.x, c.y, w, d, n.min(5), mat, BOpts { doors: &[face], balconies: false, fire_escape: false, ..Default::default() });
        return;
    }
    // jardín delantero con cerco en Barrio Jardín
    if setback > 0.0 {
        let (gx, gz, gw, gd) = match side {
            0 => (c.x, z0 + setback / 2.0, w, setback),
            1 => (c.x, z1 - setback / 2.0, w, setback),
            2 => (x0 + setback / 2.0, c.y, setback, d),
            _ => (x1 - setback / 2.0, c.y, setback, d),
        };
        b.w.add_visual(gx, 0.18, gz, gw, 0.03, gd, Mat::Grass, false);
        let (hx, hz, hw, hd) = match side {
            0 => (c.x, z0 + 0.2, w - 1.4, 0.3),
            1 => (c.x, z1 - 0.2, w - 1.4, 0.3),
            2 => (x0 + 0.2, c.y, 0.3, d - 1.4),
            _ => (x1 - 0.2, c.y, 0.3, d - 1.4),
        };
        b.w.add_box(hx, hz, hw, 0.9, hd, Mat::Hedge);
    }
    lot_building(b, r, lx0, lz0, lx1, lz1, face, n, blk.barrio);
}

/// Plazoleta chica en un lote libre: pasto, un árbol, un banco y a veces una escultura.
fn plazoleta(b: &mut B, r: &mut Rng, c: Vec2, w: f32, d: f32) {
    b.w.add_visual(c.x, 0.18, c.y, w - 0.6, 0.03, d - 0.6, Mat::Grass, false);
    b.tree(c.x + r.signed() * w * 0.2, c.y + r.signed() * d * 0.2, 1.0 + r.f() * 0.4);
    if r.next() < 0.6 {
        let s = if w > d { super::props::Side::N } else { super::props::Side::W };
        super::props::bench(b.w, s, c.x + r.signed() * w * 0.15, c.y + r.signed() * d * 0.15);
    }
    if r.next() < 0.2 {
        b.w.add_box(c.x, c.y, 1.4, 0.9, 1.4, Mat::Jersey);
        b.w.add_block(c.x, 1.06, c.y, 0.7, 2.0, 0.7, Mat::Statue);
    }
}

/// Edificios girados que acompañan las diagonales, de los dos lados.
fn diagonal_buildings(b: &mut B, r: &mut Rng) {
    for (a, c, _) in diagonals() {
        let d = c - a;
        let len = d.length();
        let dir = d / len;
        let nrm = Vec2::new(-dir.y, dir.x);
        let yaw = (-d.y).atan2(d.x);
        for side in [-1.0f32, 1.0] {
            let mut t = 8.0;
            while t < len - 8.0 {
                let f = 9.0 + r.f() * 5.0;
                let depth = 13.0;
                let p = a + dir * (t + f / 2.0) + nrm * side * (DIAG_W / 2.0 + VEREDA + depth / 2.0);
                t += f;
                // solo dentro de una manzana (no sobre calles ni plazas)
                let inside = |q: Vec2| {
                    (0..XS.len() - 1).any(|i| {
                        (0..ZS.len() - 1).any(|j| {
                            let (x0, z0, x1, z1) = block_rect(i, j);
                            q.x > x0 + 0.5 && q.x < x1 - 0.5 && q.y > z0 + 0.5 && q.y < z1 - 0.5 && matches!(block_use(i, j), Use::Lots | Use::Towers)
                        })
                    })
                };
                let hw = f / 2.0 - 0.1;
                let hd = depth / 2.0;
                let corners = [p + dir * hw + nrm * hd, p - dir * hw + nrm * hd, p + dir * hw - nrm * hd, p - dir * hw - nrm * hd];
                if !corners.iter().all(|q| inside(*q)) {
                    continue;
                }
                let barrio = (0..XS.len() - 1).flat_map(|i| (0..ZS.len() - 1).map(move |j| (i, j))).find(|&(i, j)| {
                    let (x0, z0, x1, z1) = block_rect(i, j);
                    p.x > x0 && p.x < x1 && p.y > z0 && p.y < z1
                });
                let br = barrio.map_or(Barrio::Centro, |(i, j)| self::barrio(i, j));
                let n = floors(r, br, true);
                let h = n as f32 * FH + 0.4;
                let m = Mat::Facade([0u8, 2, 4, 7][r.idx(4)]);
                b.w.add_block_rot(p.x, 0.16, p.y, f - 0.1, h, depth, yaw, m);
                b.w.add_visual_rot(p.x, 0.16 + h + 0.03, p.y, f - 0.3, 0.06, depth - 0.2, yaw, Mat::Roof, false);
                // fachada hacia la diagonal
                let fp = p - nrm * side * (depth / 2.0 + 0.05);
                b.w.add_visual_rot(fp.x, 0.16 + FH / 2.0, fp.y, f - 0.1, FH, 0.1, yaw, Mat::Shop(r.idx(6) as u8), true);
                if n > 1 {
                    b.w.add_visual_rot(fp.x, 0.16 + FH + (h - FH) / 2.0, fp.y, f - 0.1, h - FH, 0.1, yaw, Mat::Front(r.idx(8) as u8), true);
                }
                let cp = p - nrm * side * (depth / 2.0 + 0.25);
                b.w.add_visual_rot(cp.x, 0.16 + h - 0.2, cp.y, f - 0.1, 0.4, 0.5, yaw, Mat::Jersey, false);
            }
        }
    }
}

/* ---------- manzanas especiales ---------- */

/// Docks de ladrillo junto al río: dos naves largas de 4 pisos con arcos abajo.
fn docks(b: &mut B, r: &mut Rng, blk: &Block) {
    let (w, d) = (blk.x1 - blk.x0 - 2.0, blk.z1 - blk.z0 - 2.0);
    let c = blk.center();
    let half = (d - 6.0) / 2.0;
    for s in [-1.0f32, 1.0] {
        let cz = c.y + s * (half / 2.0 + 1.5);
        let n = 3 + r.idx(2) as u32;
        let h = n as f32 * FH;
        b.w.add_block(c.x, 0.16, cz, w, h, half, Mat::Facade(3));
        b.w.add_visual(c.x, 0.16 + h + 0.03, cz, w - 0.2, 0.06, half - 0.2, Mat::Roof, false);
        for face in [-1.0f32, 1.0] {
            let fz = cz + face * (half / 2.0 + 0.04);
            b.w.add_visual(c.x, 0.16 + FH + (h - FH) / 2.0, fz, w, h - FH, 0.08, Mat::Front(8 + r.idx(4) as u8), true);
            b.w.add_visual(c.x, 0.16 + FH / 2.0, fz, w, FH, 0.08, Mat::Front(12), true);
        }
        // grúas portuarias viejas sobre el techo de las naves del lado del río
        if s < 0.0 {
            for k in [-0.3f32, 0.3] {
                let x = c.x + k * w;
                b.w.add_visual(x, 0.16 + h + 4.0, cz, 1.0, 8.0, 1.0, Mat::Red, true);
                b.w.add_visual(x + 3.0, 0.16 + h + 8.0, cz, 8.0, 0.8, 0.8, Mat::Red, true);
            }
        }
    }
}

/// Playa de contenedores con galpón y grúa pórtico.
fn yard(b: &mut B, r: &mut Rng, blk: &Block) {
    let c = blk.center();
    let (w, d) = (blk.x1 - blk.x0, blk.z1 - blk.z0);
    b.hall(c.x - w * 0.22, c.y - d * 0.2, w * 0.4, d * 0.4, 7.0, Mat::Facade(3), &['s', 'e']);
    for k in 0..6 {
        let x = c.x + w * 0.05 + (k % 3) as f32 * 7.0 - 4.0;
        let z = c.y + d * 0.2 + (k / 3) as f32 * 4.0 - 2.0;
        b.w.add_container(x, z, true, 1 + r.idx(3) as u32, r.idx(4));
    }
    // pórtico
    for s in [-1.0f32, 1.0] {
        b.w.add_block(c.x + w * 0.3, 0.16, c.y + s * d * 0.35, 1.0, 14.0, 1.0, Mat::Yellow);
    }
    b.w.add_block(c.x + w * 0.3, 14.16, c.y, 1.4, 1.2, d * 0.75, Mat::Yellow);
}

/// Torres de vidrio sobre una plaza seca (Catalinas): una alta y una o dos más bajas.
fn towers(b: &mut B, r: &mut Rng, blk: &Block) {
    let c = blk.center();
    let (w, d) = (blk.x1 - blk.x0, blk.z1 - blk.z0);
    let tw = (w * 0.42).min(20.0);
    let td = (d * 0.42).min(20.0);
    let h = 55.0 + r.f() * 65.0;
    let m = Mat::Tower(r.idx(4) as u8);
    let (tx, tz) = (c.x - w * 0.18, c.y - d * 0.18);
    b.tower(tx, tz, tw, td, h, m, &[]);
    b.w.add_visual(tx, h + 1.0, tz, tw * 0.5, 2.0, td * 0.5, Mat::Steel, true);
    let h2 = 25.0 + r.f() * 30.0;
    let (sx, sz) = (c.x + w * 0.22, c.y + d * 0.2);
    b.tower(sx, sz, tw * 0.75, td * 0.75, h2, Mat::Tower(r.idx(4) as u8), &[]);
    // plaza seca con bancos y árboles en maceta
    for k in 0..4 {
        let x = c.x + w * 0.25 - k as f32 * 3.5;
        let z = c.y - d * 0.3;
        b.w.add_box(x, z, 1.2, 0.6, 1.2, Mat::Jersey);
        b.w.trees.push([x, 2.6, z, 0.9]);
    }
}

/// Plaza con estación de subte: senderos, fuente o monumento, bancos y árboles.
fn plaza(b: &mut B, r: &mut Rng, blk: &Block, name: &'static str) -> super::city2::Station {
    let c = blk.center();
    let (w, d) = (blk.x1 - blk.x0, blk.z1 - blk.z0);
    let half = w.min(d) / 2.0;
    // boca del subte en el borde sur, y la vereda alrededor del hueco
    let hole = [c.x + half * 0.45, c.y + half * 0.35, c.x + half * 0.45 + 4.0, c.y + half * 0.35 + 8.6];
    b.paving_with_hole(c.x, c.y, half, hole);
    // si la manzana no es cuadrada, completar con vereda
    if w > d + 0.5 {
        for s in [-1.0f32, 1.0] {
            b.w.add_box(c.x + s * (half + (w / 2.0 - half) / 2.0), c.y, w / 2.0 - half, 0.16, d, Mat::Sidewalk);
        }
    } else if d > w + 0.5 {
        for s in [-1.0f32, 1.0] {
            b.w.add_box(c.x, c.y + s * (half + (d / 2.0 - half) / 2.0), w, 0.16, d / 2.0 - half, Mat::Sidewalk);
        }
    }
    // canteros de pasto en cuatro cuadrantes y senderos en cruz y en diagonal
    for (qx, qz) in [(-1.0f32, -1.0f32), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
        let q = Vec2::new(c.x + qx * half * 0.5, c.y + qz * half * 0.5);
        if (q.x - hole[0]).abs() < half * 0.5 && (q.y - hole[1]).abs() < half * 0.5 {
            continue;
        }
        b.w.add_visual(q.x, 0.18, q.y, half * 0.8, 0.03, half * 0.8, Mat::Grass, false);
        for _ in 0..3 {
            b.tree(q.x + r.signed() * half * 0.3, q.y + r.signed() * half * 0.3, 1.0 + r.f() * 0.5);
        }
        b.w.add_box(q.x - qx * half * 0.42, q.y, 1.8, 0.45, 0.5, Mat::Wood);
    }
    if name == "Plaza Mayor" {
        // Pirámide: obelisco blanco chico sobre basamento, con rejas
        b.w.add_box(c.x, c.y, 6.0, 1.0, 6.0, Mat::Jersey);
        b.w.add_block(c.x, 1.0, c.y, 2.4, 9.0, 2.4, Mat::StripW);
        b.w.add_visual(c.x, 10.6, c.y, 1.4, 1.2, 1.4, Mat::StripW, true);
        b.w.add_visual(c.x, 11.8, c.y, 0.6, 1.4, 0.6, Mat::Statue, true);
        // palmeras
        for (px, pz) in [(-0.7f32, -0.7f32), (0.7, -0.7), (-0.7, 0.7), (0.7, 0.7)] {
            let (x, z) = (c.x + px * half * 0.3, c.y + pz * half * 0.3);
            b.w.add_block(x, 0.16, z, 0.4, 7.0, 0.4, Mat::Trunk);
            b.w.trees.push([x, 7.6, z, 1.5]);
        }
    } else {
        // fuente
        b.w.add_box(c.x, c.y, 7.0, 0.7, 7.0, Mat::Jersey);
        b.w.add_visual(c.x, 0.72, c.y, 6.0, 0.04, 6.0, Mat::Water, false);
        b.w.add_block(c.x, 0.7, c.y, 0.8, 2.2, 0.8, Mat::Statue);
    }
    for k in 0..3 {
        b.street_stall(c.x - half * 0.6 + k as f32 * 4.0, c.y - half + 2.5, (k + name.len()) as u32);
    }
    super::props::benches_ring(b.w, c.x, c.y, 7.5);
    super::props::flower_stand(b.w, c.x - half * 0.55, c.y + half * 0.75);
    super::props::fountain_drink(b.w, c.x + half * 0.2, c.y - half * 0.85);
    for k in 0..6 {
        let a = k as f32 / 6.0 * std::f32::consts::TAU;
        b.lamp(c.x + a.cos() * half * 0.85, c.y + a.sin() * half * 0.85);
    }
    b.station(name, hole)
}

/// Parque: pasto, muchos árboles, laguna, glorieta y senderos.
fn park(b: &mut B, r: &mut Rng, blk: &Block) {
    let c = blk.center();
    let (w, d) = (blk.x1 - blk.x0, blk.z1 - blk.z0);
    b.w.add_visual(c.x, 0.18, c.y, w - 2.0, 0.03, d - 2.0, Mat::Grass, false);
    b.w.add_visual(c.x, 0.2, c.y, 2.5, 0.02, d - 2.0, Mat::InFloor, false);
    b.w.add_visual(c.x, 0.2, c.y, w - 2.0, 0.02, 2.5, Mat::InFloor, false);
    b.w.add_box(c.x - w * 0.2, c.y + d * 0.2, 10.0, 0.5, 7.0, Mat::Jersey);
    b.w.add_visual(c.x - w * 0.2, 0.52, c.y + d * 0.2, 9.2, 0.04, 6.2, Mat::Water, false);
    // glorieta
    let (gx, gz) = (c.x + w * 0.22, c.y - d * 0.22);
    b.w.add_box(gx, gz, 6.0, 0.4, 6.0, Mat::Concrete);
    for (px, pz) in [(-2.5f32, -2.5f32), (2.5, -2.5), (-2.5, 2.5), (2.5, 2.5)] {
        b.w.add_block(gx + px, 0.4, gz + pz, 0.25, 3.0, 0.25, Mat::StripW);
    }
    b.w.add_block(gx, 3.4, gz, 6.6, 0.3, 6.6, Mat::Red);
    super::props::playground(b.w, c.x + w * 0.22, c.y + d * 0.24);
    super::props::carousel(b.w, c.x - w * 0.24, c.y - d * 0.22);
    super::props::food_truck(b.w, c.x + w * 0.05, blk.z0 + 3.0);
    super::props::benches_ring(b.w, c.x, c.y, 4.5);
    super::props::fountain_drink(b.w, c.x + 3.0, c.y + 3.0);
    for _ in 0..((w * d) / 70.0) as i32 {
        let (x, z) = (blk.x0 + 2.0 + r.f() * (w - 4.0), blk.z0 + 2.0 + r.f() * (d - 4.0));
        if (x - c.x).abs() < 2.5 || (z - c.y).abs() < 2.5 || b.w.box_at(Vec3::new(x, 1.0, z)).is_some() {
            continue;
        }
        b.tree(x, z, 1.0 + r.f() * 0.7);
    }
}

/// Refugio: casona de dos plantas en una plaza, con boca de subte.
fn refugio(b: &mut B, r: &mut Rng, blk: &Block) -> (super::city2::Station, Vec3) {
    let c = blk.center();
    let half = (blk.x1 - blk.x0).min(blk.z1 - blk.z0) / 2.0;
    let hole = [c.x + half * 0.5, c.y + half * 0.3, c.x + half * 0.5 + 4.0, c.y + half * 0.3 + 8.6];
    b.paving_with_hole(c.x, c.y, half, hole);
    let (w, d) = (blk.x1 - blk.x0, blk.z1 - blk.z0);
    if w > d + 0.5 {
        for s in [-1.0f32, 1.0] {
            b.w.add_box(c.x + s * (half + (w / 2.0 - half) / 2.0), c.y, w / 2.0 - half, 0.16, d, Mat::Sidewalk);
        }
    } else if d > w + 0.5 {
        for s in [-1.0f32, 1.0] {
            b.w.add_box(c.x, c.y + s * (half + (d / 2.0 - half) / 2.0), w, 0.16, d / 2.0 - half, Mat::Sidewalk);
        }
    }
    let (bx, bz) = (c.x - 4.0, c.y - 5.0);
    b.building(bx, bz, 14.0, 11.0, 2, Mat::Facade(0), BOpts { doors: &['s', 'e'], ..Default::default() });
    let safe = Vec3::new(bx - 3.0, 1.0, bz + 2.0);
    b.w.add_block(bx - 3.0, 0.16, bz + 2.6, 1.6, 0.9, 0.9, Mat::Container(1));
    b.w.add_strip(bx - 3.0, 1.1, bz + 2.6, 1.2, 0.06, 0.7, Mat::Mint);
    for (x, z) in [(-14.0, 10.0), (12.0, -12.0), (-14.0, -14.0), (14.0, 10.0), (-2.0, 14.0), (-16.0, 2.0)] {
        b.tree(c.x + x, c.y + z, 1.1 + r.f() * 0.3);
    }
    // jardín de la casona, bancos, bebedero, farolas y una huerta comunitaria
    b.w.add_visual(c.x - 4.0, 0.18, c.y + 4.5, 14.0, 0.03, 6.0, Mat::Grass, false);
    for k in 0..4 {
        b.w.add_box(c.x - 9.0 + k as f32 * 3.2, c.y + 5.0, 2.2, 0.4, 3.6, Mat::Wood);
        b.w.add_visual(c.x - 9.0 + k as f32 * 3.2, 0.62, c.y + 5.0, 2.0, 0.08, 3.4, Mat::Hedge, false);
    }
    super::props::benches_ring(b.w, c.x + 9.0, c.y + 8.0, 3.2);
    super::props::fountain_drink(b.w, c.x + 9.0, c.y + 8.0);
    super::props::food_truck(b.w, c.x + 8.0, c.y - 14.0);
    for k in 0..4 {
        let a = k as f32 / 4.0 * std::f32::consts::TAU + 0.4;
        b.lamp(c.x + a.cos() * half * 0.8, c.y + a.sin() * half * 0.8);
    }
    (b.station("Refugio", hole), safe)
}

/// Alrededor de un puesto o campamento: vereda y árboles (el puesto arma sus muros adentro).
fn outpost_ring(b: &mut B, r: &mut Rng, blk: &Block) {
    let c = blk.center();
    for _ in 0..4 {
        let ang = r.f() * std::f32::consts::TAU;
        let rr = 18.5;
        let (x, z) = (c.x + ang.cos() * rr, c.y + ang.sin() * rr);
        if x > blk.x0 + 1.5 && x < blk.x1 - 1.5 && z > blk.z0 + 1.5 && z < blk.z1 - 1.5 {
            b.tree(x, z, 0.9);
        }
    }
}

/// El Obelisco, en el cruce de la 9 de Julio y Corrientes.
fn obelisco(b: &mut B) {
    let (x, z) = (XS[4].c, ZS[2].c);
    b.w.add_box(x, z, 4.4, 1.2, 4.4, Mat::Jersey);
    let mut y = 1.2;
    let mut s = 3.4;
    for _ in 0..8 {
        let h = 7.0;
        b.w.add_block(x, y, z, s, h, s, Mat::StripW);
        y += h;
        s -= 0.25;
    }
    // punta
    b.w.add_block(x, y, z, s * 0.7, 2.0, s * 0.7, Mat::StripW);
    b.w.add_visual(x, y + 2.6, z, s * 0.35, 1.2, s * 0.35, Mat::StripW, true);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manzanas_de_buen_tamano() {
        for i in 0..XS.len() - 1 {
            for j in 0..ZS.len() - 1 {
                let (x0, z0, x1, z1) = block_rect(i, j);
                assert!(x1 - x0 > 36.0 && z1 - z0 > 36.0, "manzana ({i}, {j}) chica: {} × {}", x1 - x0, z1 - z0);
            }
        }
    }

    #[test]
    fn diagonales_a_45_grados() {
        for (a, b, _) in diagonals() {
            let d = b - a;
            assert!((d.x.abs() / d.y.abs() - 1.0).abs() < 0.1);
        }
    }
}
