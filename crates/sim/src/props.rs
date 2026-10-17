//! Detalles de la ciudad (nuevo): todo lo que hace que una cuadra parezca vivida.
//! - **Vereda**: árboles con su cantero, faroles, bancos, cestos, contenedores de basura, buzones,
//!   hidrantes, maceteros, motos estacionadas, mesas de bar con sombrilla, verdulerías y
//!   bicicleteros.
//! - **Esquinas**: semáforos, carteles con el nombre de las calles, sendas peatonales, bolardos y
//!   kioscos de diarios.
//! - **Avenidas**: paradas de colectivo con refugio y algún colectivo parado.
//! - **Fachadas**: toldos, carteles salientes, aires acondicionados y rejas.
//! - **Terrazas y patios**: parrillas, reposeras, tendederos con ropa, antenas, plantas y
//!   piletas.
//! - **Plazas y parques**: juegos (hamacas, tobogán, subibaja), calesita, bebederos, florería y
//!   un food truck.
//!
//! Lo chico es solo visual; lo que se puede usar de cobertura o de escalón (bancos, kioscos,
//! contenedores, refugios, colectivos) tiene colisión.

use super::town::{Barrio, Block, St, VEREDA, XS, ZS};
use super::*;
use glam::Vec2;

/// Para qué lado de la calle mira algo que está en la vereda.
#[derive(Clone, Copy, Debug)]
pub enum Side {
    N,
    S,
    W,
    E,
}

impl Side {
    /// Vector hacia la calle.
    fn out(self) -> Vec2 {
        match self {
            Side::N => Vec2::new(0.0, -1.0),
            Side::S => Vec2::new(0.0, 1.0),
            Side::W => Vec2::new(-1.0, 0.0),
            Side::E => Vec2::new(1.0, 0.0),
        }
    }
    fn along_x(self) -> bool {
        matches!(self, Side::N | Side::S)
    }
}

/// Caja con frente `w` a lo largo de la vereda y fondo `d` hacia la calle.
#[allow(clippy::too_many_arguments)]
fn vbox(w: &mut World, s: Side, x: f32, y: f32, z: f32, len: f32, h: f32, dep: f32, m: Mat, shadow: bool) {
    if s.along_x() {
        w.add_visual(x, y, z, len, h, dep, m, shadow);
    } else {
        w.add_visual(x, y, z, dep, h, len, m, shadow);
    }
}

#[allow(clippy::too_many_arguments)]
fn cbox(w: &mut World, s: Side, x: f32, y0: f32, z: f32, len: f32, h: f32, dep: f32, m: Mat) {
    if s.along_x() {
        w.add_block(x, y0, z, len, h, dep, m);
    } else {
        w.add_block(x, y0, z, dep, h, len, m);
    }
}

const G: f32 = 0.16;

/* ---------- mobiliario ---------- */

pub fn bench(w: &mut World, s: Side, x: f32, z: f32) {
    let o = s.out();
    cbox(w, s, x, G, z, 1.8, 0.45, 0.5, Mat::Wood);
    vbox(w, s, x - o.x * 0.25, G + 0.75, z - o.y * 0.25, 1.8, 0.45, 0.08, Mat::Wood, false);
    for k in [-0.75f32, 0.75] {
        let (lx, lz) = if s.along_x() { (x + k, z) } else { (x, z + k) };
        vbox(w, s, lx, G + 0.2, lz, 0.08, 0.4, 0.45, Mat::Pole, false);
    }
}

pub fn trash_can(w: &mut World, x: f32, z: f32, color: u8) {
    w.add_visual(x, G + 0.45, z, 0.5, 0.9, 0.5, Mat::Awning(color % 4), true);
    w.add_visual(x, G + 0.92, z, 0.56, 0.06, 0.56, Mat::Pole, false);
}

pub fn dumpster(w: &mut World, s: Side, x: f32, z: f32) {
    cbox(w, s, x, G, z, 1.9, 1.25, 1.1, Mat::Container(3));
    vbox(w, s, x, G + 1.3, z, 2.0, 0.1, 1.2, Mat::Hedge, false);
}

pub fn mailbox(w: &mut World, x: f32, z: f32) {
    w.add_visual(x, G + 0.55, z, 0.42, 1.1, 0.42, Mat::Red, true);
    w.add_visual(x, G + 1.15, z, 0.46, 0.1, 0.46, Mat::Pole, false);
}

pub fn hydrant(w: &mut World, x: f32, z: f32) {
    w.add_visual(x, G + 0.3, z, 0.26, 0.6, 0.26, Mat::Red, false);
    w.add_visual(x, G + 0.45, z, 0.46, 0.12, 0.14, Mat::Red, false);
}

pub fn planter(w: &mut World, x: f32, z: f32) {
    w.add_block(x, G, z, 1.0, 0.55, 1.0, Mat::Jersey);
    w.add_visual(x, G + 0.75, z, 0.9, 0.4, 0.9, Mat::Hedge, false);
}

pub fn tree_pit(w: &mut World, x: f32, z: f32) {
    w.add_visual(x, G + 0.005, z, 1.2, 0.01, 1.2, Mat::Gravel, false);
}

pub fn moto(w: &mut World, s: Side, x: f32, z: f32, color: u8) {
    // perpendicular al cordón, como se estacionan en Buenos Aires
    let perp = !s.along_x();
    let (bw, bd) = if perp { (1.6, 0.36) } else { (0.36, 1.6) };
    w.add_visual(x, 0.6, z, bw, 0.45, bd, Mat::Car(color % 6), true);
    w.add_visual(x, 0.88, z, bw * 0.45, 0.12, bd.min(0.3).max(bd * 0.45), Mat::Tire, false);
    for k in [-0.62f32, 0.62] {
        let (wx, wz) = if perp { (x + k, z) } else { (x, z + k) };
        let (ww, wd) = if perp { (0.55, 0.12) } else { (0.12, 0.55) };
        w.add_visual(wx, 0.3, wz, ww, 0.55, wd, Mat::Tire, false);
    }
}

pub fn bike_rack(w: &mut World, s: Side, x: f32, z: f32) {
    for k in 0..3 {
        let t = (k as f32 - 1.0) * 0.8;
        let (px, pz) = if s.along_x() { (x + t, z) } else { (x, z + t) };
        vbox(w, s, px, G + 0.4, pz, 0.06, 0.8, 0.7, Mat::Pole, false);
    }
}

/// Mesa de bar con dos sillas y sombrilla del color del local.
pub fn cafe_table(w: &mut World, x: f32, z: f32, color: u8) {
    w.add_block(x, G, z, 0.75, 0.78, 0.75, Mat::Wood);
    for (dx, dz) in [(-0.7f32, 0.0f32), (0.7, 0.0)] {
        w.add_visual(x + dx, G + 0.23, z + dz, 0.42, 0.46, 0.42, Mat::Pole, false);
        w.add_visual(x + dx * 1.25, G + 0.7, z + dz, 0.06, 0.5, 0.42, Mat::Pole, false);
    }
    w.add_visual(x, G + 1.6, z, 0.06, 1.6, 0.06, Mat::Pole, false);
    w.add_visual(x, G + 2.4, z, 2.0, 0.12, 2.0, Mat::Awning(color % 4), true);
}

/// Cajones de fruta y verdura en la vereda de una verdulería.
pub fn fruit_stand(w: &mut World, s: Side, x: f32, z: f32) {
    cbox(w, s, x, G, z, 2.4, 0.7, 0.9, Mat::Wood);
    for k in 0..4 {
        let t = (k as f32 - 1.5) * 0.58;
        let (px, pz) = if s.along_x() { (x + t, z) } else { (x, z + t) };
        w.add_visual(px, G + 0.78, pz, 0.5, 0.14, 0.5, [Mat::Red, Mat::Mint, Mat::Yellow, Mat::Awning(2)][k], false);
    }
}

/// Kiosco de diarios y revistas: cubo verde con techito y tapas de colores.
pub fn kiosk(w: &mut World, s: Side, x: f32, z: f32) {
    let o = s.out();
    cbox(w, s, x, G, z, 2.4, 2.3, 1.6, Mat::Awning(3));
    vbox(w, s, x + o.x * 0.25, G + 2.45, z + o.y * 0.25, 2.8, 0.12, 2.2, Mat::Awning(3), true);
    for k in 0..6 {
        let t = (k as f32 - 2.5) * 0.36;
        let (px, pz) = if s.along_x() { (x + t, z + o.y * 0.82) } else { (x + o.x * 0.82, z + t) };
        let m = [Mat::Red, Mat::Yellow, Mat::Awning(1), Mat::StripW, Mat::Mint, Mat::Awning(0)][k];
        vbox(w, s, px, G + 1.2 + (k % 2) as f32 * 0.45, pz, 0.3, 0.4, 0.04, m, false);
    }
}

/// Refugio de colectivo: techo, vidrio atrás, banco y cartel con el número.
pub fn bus_stop(w: &mut World, s: Side, x: f32, z: f32, line: u8) {
    let o = s.out();
    // vidrio del fondo (lado de la vereda) con colisión
    let back = Vec2::new(x, z) - o * 0.7;
    cbox(w, s, back.x, G, back.y, 3.8, 2.4, 0.1, Mat::Glass);
    vbox(w, s, x, G + 2.5, z, 4.2, 0.12, 1.7, Mat::Steel, true);
    for k in [-1.95f32, 1.95] {
        let (px, pz) = if s.along_x() { (back.x + k, back.y) } else { (back.x, back.y + k) };
        vbox(w, s, px, G + 1.25, pz, 0.1, 2.5, 0.1, Mat::Pole, false);
    }
    bench(w, s, x, z);
    // cartel con el número de la línea
    let sign = Vec2::new(x, z) + o * 0.6 + if s.along_x() { Vec2::new(2.4, 0.0) } else { Vec2::new(0.0, 2.4) };
    w.add_visual(sign.x, G + 1.4, sign.y, 0.08, 2.8, 0.08, Mat::Pole, false);
    vbox(w, s, sign.x, G + 2.7, sign.y, 0.7, 0.7, 0.05, Mat::Awning(line % 4), false);
}

/// Colectivo detenido junto al cordón (macizo: se puede usar de cobertura y subirse arriba).
pub fn colectivo(w: &mut World, along_x: bool, x: f32, z: f32, color: u8) {
    let (len, wid) = (11.0, 2.5);
    let (bw, bd) = if along_x { (len, wid) } else { (wid, len) };
    w.add_block(x, 0.35, z, bw, 2.6, bd, Mat::Car(color % 6));
    let (gw, gd) = if along_x { (len - 1.0, wid + 0.04) } else { (wid + 0.04, len - 1.0) };
    w.add_visual(x, 2.1, z, gw, 0.9, gd, Mat::CarGlass, false);
    w.add_visual(x, 3.0, z, bw - 0.4, 0.1, bd - 0.4, Mat::StripW, false);
    for k in [-3.6f32, 3.6] {
        let (wx, wz) = if along_x { (x + k, z) } else { (x, z + k) };
        let (ww, wd) = if along_x { (1.0, wid + 0.1) } else { (wid + 0.1, 1.0) };
        w.add_visual(wx, 0.45, wz, ww, 0.9, wd, Mat::Tire, false);
    }
}

/* ---------- esquinas ---------- */

/// Semáforo en la esquina, con las tres luces mirando a la calle.
pub fn traffic_light(w: &mut World, x: f32, z: f32, face: Vec2) {
    w.add_block(x, G, z, 0.16, 3.4, 0.16, Mat::Pole);
    let head = Vec2::new(x, z) + face * 0.25;
    w.add_visual(head.x, G + 3.0, head.y, 0.34, 1.0, 0.34, Mat::Pole, false);
    for (k, m) in [Mat::StripR, Mat::Strip, Mat::Mint].iter().enumerate() {
        let p = head + face * 0.18;
        w.add_visual(p.x, G + 3.32 - k as f32 * 0.3, p.y, 0.18, 0.18, 0.18, *m, false);
    }
}

/// Cartel con el nombre de las dos calles (placas azules cruzadas).
pub fn street_sign(w: &mut World, x: f32, z: f32) {
    w.add_block(x, G, z, 0.1, 2.9, 0.1, Mat::Pole);
    w.add_visual(x, G + 2.75, z + 0.0, 1.3, 0.28, 0.04, Mat::Awning(1), false);
    w.add_visual(x, G + 2.4, z, 0.04, 0.28, 1.3, Mat::Awning(1), false);
}

/// Senda peatonal (rayas blancas) cruzando la calle `st` a la altura `at` del otro eje.
fn crosswalk(w: &mut World, st: &St, along_x: bool, at: f32) {
    let n = ((st.w - 1.0) / 1.0) as i32;
    for k in 0..n {
        let u = st.c - st.w / 2.0 + 0.8 + k as f32 * 1.0;
        if k % 2 == 1 {
            continue;
        }
        let (x, z, ww, dd) = if along_x { (at, u, 3.2, 0.5) } else { (u, at, 0.5, 3.2) };
        w.add_strip(x, 0.02, z, ww, 0.02, dd, Mat::Line);
    }
}

/// Esquinas: semáforos, carteles, sendas peatonales, bolardos y kioscos de diarios.
pub(super) fn intersections(b: &mut B, r: &mut Rng) {
    for (i, sx) in XS.iter().enumerate() {
        for (j, sz) in ZS.iter().enumerate() {
            let (x, z) = (sx.c, sz.c);
            if x.abs() > 270.0 || z.abs() > 270.0 {
                // las calles del borde llevan menos
                if r.next() < 0.5 {
                    continue;
                }
            }
            // sendas peatonales a los cuatro lados del cruce
            for s in [-1.0f32, 1.0] {
                crosswalk(b.w, sz, true, x + s * (sx.w / 2.0 + 2.2));
                crosswalk(b.w, sx, false, z + s * (sz.w / 2.0 + 2.2));
            }
            // en las cuatro esquinas de vereda
            for (k, (cx, cz)) in [(-1.0f32, -1.0f32), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)].into_iter().enumerate() {
                let corner = Vec2::new(x + cx * (sx.w / 2.0 + 0.7), z + cz * (sz.w / 2.0 + 0.7));
                if b.w.box_at(Vec3::new(corner.x, 1.0, corner.y)).is_some() || b.w.in_hole(corner.x, corner.y) || super::town::near_diag(corner, 0.5, 0.5, 0.5) {
                    continue;
                }
                if sx.avenue() || sz.avenue() || (i + j + k) % 2 == 0 {
                    traffic_light(b.w, corner.x, corner.y, Vec2::new(-cx, -cz).normalize());
                }
                if k == (i * 3 + j) % 4 {
                    street_sign(b.w, corner.x + cx * 0.6, corner.y);
                }
                // bolardos a los dos lados de la esquina, en el centro
                if (sx.avenue() || sz.avenue()) && x.abs() < 160.0 && z.abs() < 160.0 {
                    for t in [1.6f32, 2.6] {
                        b.w.add_visual(corner.x + cx * t, G + 0.4, corner.y, 0.18, 0.8, 0.18, Mat::Bollard, false);
                        b.w.add_visual(corner.x, G + 0.4, corner.y + cz * t, 0.18, 0.8, 0.18, Mat::Bollard, false);
                    }
                }
                // kiosco de diarios en algunas esquinas
                if r.next() < 0.12 {
                    let p = corner + Vec2::new(cx * 3.6, cz * 1.4);
                    let side = if cz < 0.0 { Side::S } else { Side::N };
                    if b.w.box_at(Vec3::new(p.x, 1.0, p.y)).is_none() && b.w.box_at(Vec3::new(p.x + 1.3, 1.0, p.y)).is_none() && b.w.box_at(Vec3::new(p.x - 1.3, 1.0, p.y)).is_none() {
                        kiosk(b.w, side, p.x, p.y);
                    }
                }
            }
        }
    }
}

/* ---------- veredas ---------- */

/// Todo lo de la vereda de una manzana de lotes, lado por lado.
pub(super) fn sidewalks(b: &mut B, r: &mut Rng, blk: &Block) {
    let sides = [(Side::N, blk.z0, true), (Side::S, blk.z1, true), (Side::W, blk.x0, false), (Side::E, blk.x1, false)];
    for (k, (s, edge, along_x)) in sides.into_iter().enumerate() {
        let st = match k {
            0 => ZS[blk.j],
            1 => ZS[blk.j + 1],
            2 => XS[blk.i],
            _ => XS[blk.i + 1],
        };
        let (a0, a1) = if along_x { (blk.x0, blk.x1) } else { (blk.z0, blk.z1) };
        // hacia adentro de la manzana desde el cordón
        let inward = -s.out();
        let at = |t: f32, d: f32| -> Vec2 {
            let base = if along_x { Vec2::new(t, edge) } else { Vec2::new(edge, t) };
            base + inward * d
        };
        let free = |w: &World, p: Vec2| w.box_at(Vec3::new(p.x, 1.0, p.y)).is_none() && w.box_at(Vec3::new(p.x, 0.4, p.y)).is_none() && !super::town::near_diag(p, 0.6, 0.6, 0.5);
        let mut t = a0 + 3.5;
        let mut slot = 0u32;
        let bus_at = if st.avenue() && r.next() < 0.6 { Some(a0 + (a1 - a0) * (0.3 + r.f() * 0.4)) } else { None };
        while t < a1 - 3.5 {
            slot += 1;
            // línea de árboles y faroles a 0,9 m del cordón
            let p = at(t, 0.9);
            if let Some(bt) = bus_at {
                if (t - bt).abs() < 3.0 {
                    if (t - bt).abs() < 1.6 {
                        let q = at(bt, 1.6);
                        if free(b.w, q) {
                            bus_stop(b.w, s, q.x, q.y, (bt as i32).unsigned_abs() as u8);
                            // a veces un colectivo parado en la parada
                            if r.next() < 0.45 {
                                let c = at(bt, -2.2);
                                colectivo(b.w, along_x, c.x, c.y, r.idx(6) as u8);
                            }
                        }
                    }
                    t += 1.5;
                    continue;
                }
            }
            if free(b.w, p) && !b.w.in_hole(p.x, p.y) {
                if slot % 4 == 2 {
                    b.lamp(p.x, p.y);
                } else if slot % 2 == 1 && (blk.barrio != Barrio::Centro || slot % 4 == 1) {
                    let sc = 0.8 + r.f() * 0.4;
                    b.tree(p.x, p.y, sc);
                    tree_pit(b.w, p.x, p.y);
                } else {
                    // mobiliario chico pegado al cordón
                    let roll = r.f();
                    match blk.barrio {
                        Barrio::Centro if roll < 0.25 => trash_can(b.w, p.x, p.y, r.idx(4) as u8),
                        Barrio::Centro if roll < 0.35 => mailbox(b.w, p.x, p.y),
                        Barrio::Centro if roll < 0.55 => moto(b.w, s, p.x, p.y, r.idx(6) as u8),
                        Barrio::Centro if roll < 0.62 => planter(b.w, p.x, p.y),
                        Barrio::Centro if roll < 0.7 => bike_rack(b.w, s, p.x, p.y),
                        Barrio::Jardin if roll < 0.15 => trash_can(b.w, p.x, p.y, 3),
                        Barrio::Jardin if roll < 0.25 => hydrant(b.w, p.x, p.y),
                        Barrio::Cerro if roll < 0.25 => dumpster(b.w, s, p.x, p.y),
                        Barrio::Cerro if roll < 0.4 => moto(b.w, s, p.x, p.y, r.idx(6) as u8),
                        _ if roll < 0.06 => hydrant(b.w, p.x, p.y),
                        _ => {}
                    }
                }
            }
            // contra la fachada: mesas de bar, verdulerías y bancos
            let q = at(t, VEREDA - 0.9);
            if slot % 3 == 0 && free(b.w, q) {
                let roll = r.f();
                match blk.barrio {
                    Barrio::Centro if roll < 0.35 => cafe_table(b.w, q.x, q.y, r.idx(4) as u8),
                    Barrio::Centro if roll < 0.45 => bench(b.w, s, q.x, q.y),
                    Barrio::Jardin | Barrio::Cerro if roll < 0.18 => fruit_stand(b.w, s, q.x, q.y),
                    Barrio::Jardin if roll < 0.3 => bench(b.w, s, q.x, q.y),
                    Barrio::Cerro if roll < 0.3 => cafe_table(b.w, q.x, q.y, r.idx(4) as u8),
                    _ => {}
                }
            }
            t += 3.0;
        }
    }
}

/* ---------- fachadas ---------- */

/// Toldo, cartel saliente y aires acondicionados para el frente de un lote.
/// (fx, fz) es el centro de la fachada sobre la vereda, `len` su ancho y `out` hacia la calle.
#[allow(clippy::too_many_arguments)]
pub fn facade_details(w: &mut World, r: &mut Rng, fx: f32, fz: f32, len: f32, out: Vec2, floors: u32, shop: bool) {
    let along_x = out.y != 0.0;
    let at = |d: f32| Vec2::new(fx, fz) + out * d;
    if shop && r.next() < 0.55 && len > 3.0 {
        // toldo con faldón
        let p = at(0.65);
        let c = r.idx(4) as u8;
        let (ww, dd) = if along_x { (len - 0.6, 1.3) } else { (1.3, len - 0.6) };
        w.add_visual(p.x, G + 3.0, p.y, ww, 0.12, dd, Mat::Awning(c), true);
        let f = at(1.28);
        let (fw, fd) = if along_x { (len - 0.6, 0.04) } else { (0.04, len - 0.6) };
        w.add_visual(f.x, G + 2.82, f.y, fw, 0.35, fd, Mat::Awning(c), false);
    }
    if shop && r.next() < 0.25 {
        // cartel saliente, perpendicular a la fachada
        let p = at(0.5);
        let side = if along_x { Vec2::new(len * 0.35, 0.0) } else { Vec2::new(0.0, len * 0.35) };
        let q = p + side;
        let (ww, dd) = if along_x { (0.1, 0.9) } else { (0.9, 0.1) };
        w.add_visual(q.x, G + 4.1, q.y, ww, 1.1, dd, Mat::Neon(r.idx(4) as u8), false);
    }
    // aires acondicionados colgados de algunas ventanas
    for f in 1..floors {
        if r.next() < 0.22 {
            let p = at(0.22);
            let t = r.signed() * (len * 0.35);
            let (x, z) = if along_x { (p.x + t, p.y) } else { (p.x, p.y + t) };
            let (ww, dd) = if along_x { (0.8, 0.4) } else { (0.4, 0.8) };
            w.add_visual(x, G + f as f32 * FH + 0.5, z, ww, 0.5, dd, Mat::Jersey, false);
        }
    }
}

/// Rejas de hierro frente a la planta baja de una casa.
pub fn house_railing(w: &mut World, fx: f32, fz: f32, len: f32, out: Vec2) {
    let along_x = out.y != 0.0;
    let p = Vec2::new(fx, fz) + out * 0.06;
    let n = (len / 0.25) as i32;
    for k in 0..n {
        let t = -len / 2.0 + 0.12 + k as f32 * 0.25;
        let (x, z) = if along_x { (p.x + t, p.y) } else { (p.x, p.y + t) };
        w.add_visual(x, G + 1.1, z, 0.03, 1.6, 0.03, Mat::Pole, false);
    }
}

/* ---------- terrazas y patios ---------- */

/// Terraza porteña: parrilla, reposeras, tendedero con ropa, antena parabólica y plantas.
pub fn rooftop(w: &mut World, r: &mut Rng, cx: f32, cz: f32, wid: f32, dep: f32, top: f32) {
    if wid < 5.0 || dep < 5.0 {
        return;
    }
    let pick = |r: &mut Rng, k: f32| (cx + r.signed() * (wid / 2.0 - k), cz + r.signed() * (dep / 2.0 - k));
    if r.next() < 0.35 {
        // parrilla de ladrillo con chimenea
        let (x, z) = pick(r, 1.5);
        w.add_block(x, top, z, 1.4, 1.0, 0.8, Mat::Facade(3));
        w.add_visual(x, top + 1.6, z + 0.2, 0.35, 1.2, 0.35, Mat::Facade(3), false);
        w.add_visual(x, top + 1.02, z, 1.2, 0.04, 0.6, Mat::Pole, false);
    }
    if r.next() < 0.35 {
        // tendedero con ropa de colores
        let (x, z) = pick(r, 2.0);
        for k in [-1.5f32, 1.5] {
            w.add_visual(x + k, top + 0.9, z, 0.06, 1.8, 0.06, Mat::Pole, false);
        }
        w.add_visual(x, top + 1.75, z, 3.0, 0.02, 0.02, Mat::Line, false);
        for k in 0..4 {
            let m = [Mat::Awning(0), Mat::StripW, Mat::Awning(1), Mat::Yellow][k];
            w.add_visual(x - 1.1 + k as f32 * 0.7, top + 1.4, z, 0.5, 0.65, 0.02, m, false);
        }
    }
    if r.next() < 0.25 {
        let (x, z) = pick(r, 1.0);
        for k in 0..2 {
            w.add_visual(x + k as f32 * 0.8, top + 0.2, z, 0.6, 0.12, 1.6, Mat::Wood, false);
        }
    }
    if r.next() < 0.2 {
        let (x, z) = pick(r, 1.0);
        w.add_visual(x, top + 0.8, z, 0.08, 1.6, 0.08, Mat::Pole, false);
        w.add_visual(x, top + 1.7, z + 0.2, 0.9, 0.9, 0.1, Mat::StripW, false);
    }
    if r.next() < 0.3 {
        for _ in 0..3 {
            let (x, z) = pick(r, 0.8);
            w.add_visual(x, top + 0.25, z, 0.5, 0.5, 0.5, Mat::Facade(6), false);
            w.add_visual(x, top + 0.7, z, 0.6, 0.5, 0.6, Mat::Hedge, false);
        }
    }
}

/// Patio del pulmón de manzana: tendederos, parrilla, mesa, pileta en Jardín y alguna bici.
pub fn backyard(w: &mut World, r: &mut Rng, x0: f32, z0: f32, x1: f32, z1: f32, barrio: Barrio) {
    let (wid, dep) = (x1 - x0, z1 - z0);
    if wid < 4.0 || dep < 4.0 {
        return;
    }
    let pick = |r: &mut Rng| (x0 + 1.2 + r.f() * (wid - 2.4), z0 + 1.2 + r.f() * (dep - 2.4));
    for _ in 0..((wid * dep) / 120.0).ceil() as i32 {
        let (x, z) = pick(r);
        match r.idx(5) {
            0 => {
                w.add_block(x, G, z, 1.4, 1.0, 0.8, Mat::Facade(3));
                w.add_visual(x, G + 1.6, z + 0.2, 0.35, 1.2, 0.35, Mat::Facade(3), false);
            }
            1 => {
                w.add_visual(x, G + 1.7, z, 3.0, 0.02, 0.02, Mat::Line, false);
                for k in 0..4 {
                    let m = [Mat::Awning(2), Mat::StripW, Mat::Awning(0), Mat::Mint][k];
                    w.add_visual(x - 1.1 + k as f32 * 0.7, G + 1.35, z, 0.5, 0.65, 0.02, m, false);
                }
                for k in [-1.5f32, 1.5] {
                    w.add_visual(x + k, G + 0.9, z, 0.06, 1.8, 0.06, Mat::Pole, false);
                }
            }
            2 => {
                w.add_block(x, G, z, 1.4, 0.75, 0.9, Mat::Wood);
                for k in [-1.0f32, 1.0] {
                    w.add_visual(x + k, G + 0.23, z, 0.42, 0.46, 0.42, Mat::Pole, false);
                }
            }
            3 if barrio == Barrio::Jardin && wid > 7.0 && dep > 6.0 => {
                w.add_block(x.clamp(x0 + 3.0, x1 - 3.0), G, z.clamp(z0 + 2.5, z1 - 2.5), 4.0, 0.35, 3.0, Mat::Tile);
                w.add_visual(x.clamp(x0 + 3.0, x1 - 3.0), G + 0.36, z.clamp(z0 + 2.5, z1 - 2.5), 3.4, 0.02, 2.4, Mat::Water, false);
            }
            _ => {
                w.add_visual(x, 0.6, z, 1.6, 0.06, 0.06, Mat::Car(r.idx(6) as u8), false);
                for k in [-0.55f32, 0.55] {
                    w.add_visual(x + k, 0.5, z, 0.06, 0.65, 0.65, Mat::Tire, false);
                }
            }
        }
    }
}

/* ---------- plazas y parques ---------- */

/// Juegos para chicos: hamacas, tobogán y subibaja, sobre arena.
pub fn playground(w: &mut World, x: f32, z: f32) {
    w.add_visual(x, G + 0.02, z, 9.0, 0.04, 7.0, Mat::Gravel, false);
    // hamacas
    for k in [-1.6f32, 1.6] {
        w.add_block(x - 2.5 + k, G, z - 1.5, 0.12, 2.6, 0.12, Mat::Red);
    }
    w.add_visual(x - 2.5, G + 2.6, z - 1.5, 3.4, 0.12, 0.12, Mat::Red, false);
    for k in [-0.7f32, 0.7] {
        w.add_visual(x - 2.5 + k, G + 1.5, z - 1.5, 0.02, 1.9, 0.02, Mat::Pole, false);
        w.add_visual(x - 2.5 + k, G + 0.55, z - 1.5, 0.5, 0.06, 0.25, Mat::Yellow, false);
    }
    // tobogán: escalera, plataforma y la bajada en escalones finos
    w.add_block(x + 2.5, G, z + 1.0, 1.0, 1.8, 1.0, Mat::Awning(1));
    for k in 0..6 {
        let h = 1.8 - k as f32 * 0.3;
        w.add_visual(x + 3.4 + k as f32 * 0.35, G + h - 0.05, z + 1.0, 0.36, 0.1, 0.7, Mat::Yellow, false);
    }
    // subibaja
    w.add_visual(x, G + 0.4, z + 2.5, 3.2, 0.1, 0.3, Mat::Awning(0), false);
    w.add_visual(x, G + 0.2, z + 2.5, 0.2, 0.4, 0.2, Mat::Pole, false);
}

/// Calesita: base redonda, caballitos y techo a rayas.
pub fn carousel(w: &mut World, x: f32, z: f32) {
    w.cylinders.push(Cylinder { center: Vec3::new(x, G + 0.2, z), radius: 4.0, length: 0.4, axis: CylAxis::Y, mat: Mat::Wood });
    w.add_collider(x, z, 7.0, 0.4, 7.0, G);
    w.cylinders.push(Cylinder { center: Vec3::new(x, G + 2.0, z), radius: 0.3, length: 3.6, axis: CylAxis::Y, mat: Mat::Yellow });
    w.cylinders.push(Cylinder { center: Vec3::new(x, G + 3.9, z), radius: 4.3, length: 0.3, axis: CylAxis::Y, mat: Mat::Red });
    w.cylinders.push(Cylinder { center: Vec3::new(x, G + 4.3, z), radius: 2.5, length: 0.5, axis: CylAxis::Y, mat: Mat::StripW });
    for k in 0..8 {
        let a = k as f32 / 8.0 * std::f32::consts::TAU;
        let (px, pz) = (x + a.cos() * 3.0, z + a.sin() * 3.0);
        w.add_visual(px, G + 2.2, pz, 0.06, 3.4, 0.06, Mat::Yellow, false);
        w.add_visual(px, G + 1.2, pz, 0.35, 0.6, 0.9, [Mat::StripW, Mat::Awning(2), Mat::Awning(1), Mat::Red][k % 4], false);
    }
}

pub fn fountain_drink(w: &mut World, x: f32, z: f32) {
    w.add_visual(x, G + 0.45, z, 0.4, 0.9, 0.4, Mat::Concrete, false);
    w.add_visual(x, G + 0.95, z, 0.6, 0.1, 0.45, Mat::Steel, false);
}

/// Puesto de flores con baldes de colores.
pub fn flower_stand(w: &mut World, x: f32, z: f32) {
    w.add_block(x, G, z, 2.4, 0.8, 1.2, Mat::Awning(3));
    w.add_visual(x, G + 2.3, z, 2.8, 0.1, 1.8, Mat::Awning(3), true);
    for k in 0..6 {
        let t = (k as f32 - 2.5) * 0.38;
        w.add_visual(x + t, G + 1.0, z + 0.2, 0.3, 0.4, 0.3, [Mat::Red, Mat::Yellow, Mat::StripW, Mat::Awning(1), Mat::Neon(0), Mat::Mint][k], false);
    }
    for k in [-1.3f32, 1.3] {
        w.add_visual(x + k, G + 1.15, z - 0.8, 0.08, 2.3, 0.08, Mat::Pole, false);
    }
}

/// Food truck estacionado con su toldo y dos mesitas.
pub fn food_truck(w: &mut World, x: f32, z: f32) {
    w.add_block(x, 0.35, z, 5.0, 2.6, 2.2, Mat::Car(5));
    w.add_visual(x, 2.0, z + 1.12, 2.6, 1.0, 0.04, Mat::CarGlass, false);
    w.add_visual(x, 3.05, z + 1.6, 3.2, 0.1, 1.2, Mat::Awning(0), true);
    for k in [-1.8f32, 1.8] {
        w.add_visual(x + k, 0.45, z, 0.9, 0.9, 2.3, Mat::Tire, false);
    }
    cafe_table(w, x - 1.6, z + 3.4, 2);
    cafe_table(w, x + 1.6, z + 3.4, 1);
}

/// Bancos alrededor de un punto (para plazas).
pub fn benches_ring(w: &mut World, x: f32, z: f32, r: f32) {
    for (s, dx, dz) in [(Side::S, 0.0, -r), (Side::N, 0.0, r), (Side::E, -r, 0.0), (Side::W, r, 0.0)] {
        bench(w, s, x + dx, z + dz);
    }
}
