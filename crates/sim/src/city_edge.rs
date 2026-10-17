//! Bordes naturales del Distrito Sur (nuevo): en vez del muro de 12 m, cada lado termina como
//! su barrio.
//! - **Norte (Puerto) y este (Centro Financiero): el río.** Costanera con faroles, bancos y
//!   árboles, borde de muelle con baranda y muelles que entran al agua. Más allá, agua hasta el
//!   horizonte, barcos de carga, islas y la otra orilla con sus torres.
//! - **Oeste (El Cerro): cerros.** Terrazas de roca con pasto y árboles que suben hasta los
//!   cerros del fondo.
//! - **Sur (Barrio Jardín): el bosque.** Pasto, árboles en filas desparejas, matorrales y lomas
//!   verdes que siguen afuera.
//!
//! No consume el `Rng` de la ciudad (usa uno propio), así el resto sale idéntico. La baranda del
//! río tiene un colisionador invisible alto: se ve una baranda, no un muro.

use super::*;

/// Línea del muelle (borde del agua) al norte y al este.
pub const QUAY: f32 = 284.5;
/// Donde empiezan los cerros al oeste.
pub const HILLS: f32 = 285.0;
/// Donde empieza el bosque al sur.
pub const FOREST: f32 = 280.0;
/// Hasta dónde se arma el paisaje de fondo (afuera del mapa).
const FAR: f32 = 470.0;
/// Nivel del agua.
const WATER_Y: f32 = -0.9;

/// Muelles: (posición a lo largo de la costa, ¿costa norte?).
const PIERS: [(f32, bool); 5] = [(-150.0, true), (30.0, true), (200.0, true), (-120.0, false), (120.0, false)];

pub(super) fn build(b: &mut B) {
    let mut r = Rng::new(20261018);
    river(b, &mut r);
    hills(b, &mut r);
    forest(b, &mut r);
}

/// Pieza de baranda a lo largo de x (norte) o de z (este), de `t0` a `t1`.
fn railing(w: &mut World, north: bool, t0: f32, t1: f32) {
    let len = t1 - t0;
    if len <= 0.2 {
        return;
    }
    let c = (t0 + t1) / 2.0;
    let q = QUAY + 0.15;
    let (x, z, ww, dd) = if north { (c, -q, len, 0.07) } else { (q, c, 0.07, len) };
    w.add_visual(x, 1.02, z, ww, 0.07, dd, Mat::Pole, false);
    w.add_visual(x, 0.6, z, ww, 0.05, dd, Mat::Pole, false);
    let mut t = t0;
    while t <= t1 + 0.01 {
        let (px, pz) = if north { (t, -q) } else { (q, t) };
        w.add_visual(px, 0.52, pz, 0.08, 1.04, 0.08, Mat::Pole, false);
        t += 2.0;
    }
    // la baranda tiene colisión (para autos y para no caerse caminando); saltándola, caés al río
    w.add_collider(x, z, ww.max(0.6), 3.3, dd.max(0.6), -2.0);
}

fn river(b: &mut B, r: &mut Rng) {
    let a = b.w.a;
    // agua: el piso de la calle se corta y el agua queda más abajo
    for h in [[-HILLS, -a - 1.0, a + 1.0, -QUAY], [QUAY, -QUAY, a + 1.0, a + 1.0]] {
        b.w.holes.push(h);
        b.w.water.push(h);
    }
    // lecho (para que lo que caiga no siga cayendo)
    b.w.add_collider(0.0, -(QUAY + a) / 2.0 - 0.5, a * 2.0 + 4.0, 0.3, a - QUAY + 3.0, -3.3);
    b.w.add_collider((QUAY + a) / 2.0 + 0.5, 0.0, a - QUAY + 3.0, 0.3, a * 2.0 + 4.0, -3.3);
    // agua hasta el horizonte (norte y este, se superponen en la esquina)
    let n_len = FAR - QUAY;
    b.w.add_visual(0.0, WATER_Y, -QUAY - n_len / 2.0, FAR * 2.0, 0.1, n_len, Mat::Sea, false);
    b.w.add_visual(QUAY + n_len / 2.0, WATER_Y, 0.0, n_len, 0.1, FAR * 2.0, Mat::Sea, false);
    for north in [true, false] {
        // costanera: vereda de 4,5 m entre las manzanas y el agua
        let (t0, t1) = if north { (-HILLS, QUAY) } else { (-QUAY, FOREST) };
        let c = (t0 + t1) / 2.0;
        let len = t1 - t0;
        let mid = (QUAY + 280.0) / 2.0;
        if north {
            b.w.add_box(c, -mid, len, 0.16, QUAY - 280.0, Mat::Sidewalk);
        } else {
            b.w.add_box(mid, c, QUAY - 280.0, 0.16, len, Mat::Sidewalk);
        }
        // borde del muelle y paredón que baja al agua
        let q = QUAY + 0.3;
        let (fx, fz, fw, fd) = if north { (c, -q, len, 0.6) } else { (q, c, 0.6, len) };
        b.w.add_visual(fx, -0.7, fz, fw, 1.8, fd, Mat::Concrete, false);
        b.w.add_visual(fx, 0.2, fz, fw, 0.12, fd, Mat::Jersey, false);
        // baranda con huecos en los muelles
        let piers: Vec<f32> = PIERS.iter().filter(|p| p.1 == north).map(|p| p.0).collect();
        let mut t = t0;
        for p in &piers {
            railing(b.w, north, t, p - 2.2);
            t = p + 2.2;
        }
        railing(b.w, north, t, t1);
        // faroles, bancos y árboles de la costanera
        let mut s = t0 + 8.0;
        let mut k = 0;
        while s < t1 - 6.0 {
            let near_pier = piers.iter().any(|p| (p - s).abs() < 5.0);
            let (lx, lz) = if north { (s, -QUAY + 1.0) } else { (QUAY - 1.0, s) };
            if !near_pier {
                match k % 3 {
                    0 => b.lamp(lx, lz),
                    1 => {
                        let (bw, bd) = if north { (1.8, 0.5) } else { (0.5, 1.8) };
                        b.w.add_box(lx, lz, bw, 0.45, bd, Mat::Wood);
                    }
                    _ => {
                        let (tx, tz) = if north { (s, -QUAY + 2.6) } else { (QUAY - 2.6, s) };
                        b.tree(tx, tz, 0.9 + r.f() * 0.4);
                    }
                }
            }
            k += 1;
            s += 8.0;
        }
        // muelles de madera que entran al agua, con baranda y farol en la punta
        for p in piers {
            let end = a - 2.0;
            let plen = end - QUAY;
            let pc = QUAY + plen / 2.0;
            let (x, z, w_, d_) = if north { (p, -pc, 4.0, plen) } else { (pc, p, plen, 4.0) };
            b.w.add_block(x, -1.2, z, w_, 1.5, d_, Mat::Wood);
            for side in [-1.0f32, 1.0] {
                let (sx, sz, sw, sd) = if north { (p + side * 2.0, -pc, 0.08, plen) } else { (pc, p + side * 2.0, plen, 0.08) };
                b.w.add_visual(sx, 1.3, sz, sw, 0.07, sd, Mat::Pole, false);
                b.w.add_collider(sx, sz, sw.max(0.5), 3.5, sd.max(0.5), -2.0);
                // pilotes
                let mut u = QUAY + 1.0;
                while u < end {
                    let (px, pz) = if north { (p + side * 2.1, -u) } else { (u, p + side * 2.1) };
                    b.w.add_visual(px, -0.4, pz, 0.3, 2.2, 0.3, Mat::Trunk, false);
                    u += 3.0;
                }
            }
            let (ex, ez) = if north { (p, -end) } else { (end, p) };
            b.w.add_collider(ex, ez, if north { 4.4 } else { 0.5 }, 3.5, if north { 0.5 } else { 4.4 }, -2.0);
            b.w.add_visual(ex, 1.3, ez, if north { 4.0 } else { 0.08 }, 0.07, if north { 0.08 } else { 4.0 }, Mat::Pole, false);
            let (lx, lz) = if north { (p + 1.6, -end + 0.6) } else { (end - 0.6, p + 1.6) };
            b.w.add_visual(lx, 1.8, lz, 0.14, 3.3, 0.14, Mat::Pole, false);
            b.w.add_visual(lx, 3.5, lz, 0.5, 0.14, 0.3, Mat::Lamp, false);
        }
    }
    // barcos de carga a lo lejos: casco, puente y contenedores
    for i in 0..7 {
        let north = i % 2 == 0;
        let along = -320.0 + r.f() * 640.0;
        let off = a + 40.0 + r.f() * 110.0;
        let len = 40.0 + r.f() * 50.0;
        let (x, z, w_, d_) = if north { (along, -off, len, 11.0) } else { (off, along, 11.0, len) };
        b.w.add_visual(x, WATER_Y + 2.0, z, w_, 5.0, d_, Mat::Steel, false);
        b.w.add_visual(x, WATER_Y + 4.6, z, w_ * 0.96, 0.3, d_ * 0.96, Mat::Red, false);
        let (bx, bz) = if north { (x - len * 0.4, z) } else { (x, z - len * 0.4) };
        b.w.add_visual(bx, WATER_Y + 9.0, bz, 6.0, 8.0, 6.0, Mat::Concrete, false);
        let stacks = (len / 7.0) as i32 - 2;
        for k in 0..stacks {
            let u = -len / 2.0 + 12.0 + k as f32 * 6.5;
            let hgt = 2.6 * (1 + r.idx(3)) as f32;
            let (cx, cz, cw, cd) = if north { (x + u, z, 6.0, 9.0) } else { (x, z + u, 9.0, 6.0) };
            b.w.add_visual(cx, WATER_Y + 4.6 + hgt / 2.0, cz, cw, hgt, cd, Mat::Container(r.idx(4) as u8), false);
        }
    }
    // islas verdes en el río
    for (x, z, s) in [(-210.0, -390.0, 30.0), (140.0, -420.0, 46.0), (400.0, -260.0, 34.0), (410.0, 300.0, 28.0)] {
        b.w.add_visual(x, WATER_Y + 1.0, z, s, 2.4, s * 0.7, Mat::Rock, false);
        b.w.add_visual(x, WATER_Y + 2.3, z, s * 0.86, 0.3, s * 0.6, Mat::Grass, false);
        for _ in 0..(s / 5.0) as i32 {
            b.w.trees.push([x + r.signed() * s * 0.35, WATER_Y + 4.8, z + r.signed() * s * 0.25, 1.6 + r.f()]);
        }
    }
    // la otra orilla: torres del otro lado del río, al este y al noreste
    for i in 0..26 {
        let t = -FAR + 30.0 + i as f32 * 36.0 + r.f() * 10.0;
        let (x, z) = if i % 3 == 0 { (t * 0.6, -FAR + 20.0 + r.f() * 20.0) } else { (FAR - 20.0 - r.f() * 30.0, t) };
        let h = 25.0 + r.f() * 70.0;
        let s = 14.0 + r.f() * 14.0;
        b.w.add_visual(x, h / 2.0 + WATER_Y, z, s, h, s, Mat::Tower(r.idx(4) as u8), false);
    }
    landmarks(b);
    // orilla de enfrente
    // orilla de enfrente: costa de hormigón y verde detrás, donde se paran las torres
    b.w.add_visual(FAR - 20.0, WATER_Y + 0.6, 0.0, 60.0, 1.4, FAR * 2.0, Mat::Grass, false);
    b.w.add_visual(FAR - 49.0, WATER_Y + 0.7, 0.0, 2.5, 1.6, FAR * 2.0, Mat::Concrete, false);
    let (n0, n1) = (-HILLS - 10.0, FAR);
    b.w.add_visual((n0 + n1) / 2.0, WATER_Y + 0.6, -FAR + 20.0, n1 - n0, 1.4, 60.0, Mat::Grass, false);
    b.w.add_visual((n0 + n1) / 2.0, WATER_Y + 0.7, -FAR + 49.0, n1 - n0, 1.6, 2.5, Mat::Concrete, false);
}

/// Hitos para orientarse desde cualquier lado: puente colgante al este, vuelta al mundo sobre el
/// agua del Puerto y un faro en la escollera del noreste. Solo decorado.
fn landmarks(b: &mut B) {
    let w = &mut *b.w;
    // puente colgante: sale de la costanera este hacia la otra orilla
    let (bz, deck) = (-40.0, 11.0);
    let (x0, x1) = (QUAY + 6.0, FAR - 10.0);
    w.add_visual((x0 + x1) / 2.0, deck, bz, x1 - x0, 1.4, 14.0, Mat::Concrete, false);
    w.add_visual((x0 + x1) / 2.0, deck + 0.75, bz, x1 - x0, 0.1, 13.0, Mat::Line, false);
    for side in [-1.0f32, 1.0] {
        w.add_visual((x0 + x1) / 2.0, deck + 1.3, bz + side * 6.8, x1 - x0, 0.9, 0.3, Mat::Jersey, false);
    }
    let mut x = x0 + 10.0;
    while x < x1 {
        w.add_visual(x, (deck + WATER_Y) / 2.0, bz, 4.0, deck - WATER_Y, 10.0, Mat::Concrete, false);
        x += 42.0;
    }
    // dos torres y el cable en parábola con sus péndolas
    let towers = [335.0f32, 425.0];
    let top = deck + 46.0;
    for tx in towers {
        for side in [-1.0f32, 1.0] {
            w.add_visual(tx, (top + WATER_Y) / 2.0, bz + side * 7.5, 2.6, top - WATER_Y, 2.6, Mat::Red, false);
        }
        w.add_visual(tx, top - 3.0, bz, 2.6, 2.0, 17.6, Mat::Red, false);
        w.add_visual(tx, deck + 18.0, bz, 2.0, 1.6, 17.6, Mat::Red, false);
    }
    let cable = |x: f32| {
        // de la costanera a la primera torre, entre torres y de la segunda a la orilla
        let (a_, b_) = if x < towers[0] { (x0, towers[0]) } else if x < towers[1] { (towers[0], towers[1]) } else { (towers[1], x1) };
        let t = (x - a_) / (b_ - a_);
        let sag = if x < towers[0] || x >= towers[1] { 1.0 } else { 0.0 };
        let low = deck + 3.0;
        let h_a = if a_ == x0 { deck + 2.0 } else { top };
        let h_b = if b_ == x1 { deck + 2.0 } else { top };
        let lin = h_a + (h_b - h_a) * t;
        lin - (1.0 - sag) * (top - low) * 4.0 * t * (1.0 - t) - sag * 6.0 * 4.0 * t * (1.0 - t)
    };
    let mut x = x0;
    while x < x1 {
        let h = cable(x);
        for side in [-1.0f32, 1.0] {
            w.add_visual(x + 1.5, h, bz + side * 7.5, 3.1, 0.35, 0.35, Mat::Steel, false);
            if (x as i32) % 9 < 3 && h > deck + 2.5 {
                w.add_visual(x + 1.5, (h + deck) / 2.0, bz + side * 7.5, 0.12, h - deck, 0.12, Mat::Steel, false);
            }
        }
        x += 3.0;
    }
    // vuelta al mundo sobre una plataforma en el agua del Puerto
    let (cx, cz, r_, hub) = (110.0, -312.0, 17.0, 22.0);
    w.add_visual(cx, WATER_Y + 1.0, cz, 30.0, 2.4, 14.0, Mat::Concrete, false);
    for i in 0..40 {
        let ang = i as f32 / 40.0 * std::f32::consts::TAU;
        let (px, py) = (cx + ang.cos() * r_, hub + ang.sin() * r_);
        for dz in [-1.4f32, 1.4] {
            w.add_visual(px, py, cz + dz, 2.8, 0.3, 0.3, Mat::StripW, false);
        }
        if i % 4 == 0 {
            // rayo (en tramos cortos, porque las cajas no se pueden inclinar) y cabina
            let mut k = 1.5;
            while k < r_ - 0.5 {
                w.add_visual(cx + ang.cos() * k, hub + ang.sin() * k, cz, 0.25, 0.25, 0.25, Mat::Pole, false);
                k += 0.9;
            }
            w.add_visual(px, py - 1.6, cz, 2.0, 2.2, 2.0, Mat::Awning((i / 4 % 4) as u8), false);
        }
    }
    for side in [-1.0f32, 1.0] {
        w.add_visual(cx + side * 5.0, (hub + WATER_Y) / 2.0 + 1.0, cz, 1.0, hub - WATER_Y - 2.0, 1.0, Mat::Steel, false);
    }
    w.add_visual(cx, hub, cz, 2.4, 2.4, 3.4, Mat::Steel, false);
    // faro en la escollera
    let (fx, fz) = (330.0, -330.0);
    w.add_visual(fx - 20.0, WATER_Y + 0.8, fz + 20.0, 60.0, 2.0, 6.0, Mat::Rock, false);
    w.cylinders.push(Cylinder { center: Vec3::new(fx, 9.0, fz), radius: 2.4, length: 20.0, axis: CylAxis::Y, mat: Mat::StripW });
    for y in [4.0f32, 12.0] {
        w.cylinders.push(Cylinder { center: Vec3::new(fx, y, fz), radius: 2.45, length: 3.0, axis: CylAxis::Y, mat: Mat::StripR });
    }
    w.add_visual(fx, 20.5, fz, 3.4, 2.4, 3.4, Mat::Lamp, false);
    w.add_visual(fx, 22.2, fz, 4.0, 0.6, 4.0, Mat::Steel, false);
}

fn hills(b: &mut B, r: &mut Rng) {
    let a = b.w.a;
    // terrazas de roca con pasto: tres escalones que suben hacia el borde, en tramos desparejos
    let mut z = -QUAY;
    while z < a + 2.0 {
        let d = 6.0 + r.f() * 6.0;
        let zc = z + d / 2.0;
        let tiers = [(HILLS + 2.0, 4.0, 1.0 + r.f() * 1.0), (HILLS + 5.8, 3.8, 2.8 + r.f() * 2.0), (HILLS + 9.6, 4.4, 5.5 + r.f() * 4.0)];
        for (cx, w_, h) in tiers {
            let jx = r.signed() * 0.6;
            b.w.add_box(-cx + jx, zc, w_, h, d + 0.4, Mat::Rock);
            b.w.add_visual(-cx + jx, h + 0.06, zc, w_ - 0.3, 0.14, d + 0.1, Mat::Grass, true);
            if r.next() < 0.35 {
                let s = 0.8 + r.f() * 0.5;
                let (tx, tz) = (-cx + jx + r.signed() * 1.0, zc + r.signed() * (d / 3.0));
                b.w.add_visual(tx, h + 1.2 * s, tz, 0.35 * s, 2.4 * s, 0.35 * s, Mat::Trunk, true);
                b.w.trees.push([tx, h + 3.6 * s, tz, 1.7 * s]);
            }
        }
        z += d;
    }
    // cerros del fondo, afuera del mapa: lomas de roca con pasto y árboles
    let mut z = -FAR;
    while z < FAR {
        let d = 30.0 + r.f() * 40.0;
        for k in 0..3 {
            let x = -(a + 18.0 + k as f32 * 45.0 + r.f() * 20.0);
            let h = 12.0 + k as f32 * 16.0 + r.f() * 22.0;
            let w_ = 40.0 + r.f() * 30.0;
            b.w.add_visual(x, h / 2.0 - 0.5, z, w_, h, d, Mat::Rock, false);
            b.w.add_visual(x, h - 0.3, z, w_ * 0.92, 0.8, d * 0.92, Mat::Grass, false);
            for _ in 0..3 {
                b.w.trees.push([x + r.signed() * w_ * 0.35, h + 2.6, z + r.signed() * d * 0.35, 2.0 + r.f() * 1.5]);
            }
        }
        z += d * 0.8;
    }
}

fn forest(b: &mut B, r: &mut Rng) {
    let a = b.w.a;
    // pasto entre las manzanas y el borde
    let (x0, x1) = (-HILLS, QUAY);
    b.w.add_visual((x0 + x1) / 2.0, 0.03, (FOREST + a) / 2.0, x1 - x0, 0.06, a - FOREST, Mat::Grass, false);
    // filas desparejas de árboles, cada vez más tupidas hacia el borde
    for (row, dens) in [(284.0, 9.0), (287.5, 6.5), (291.0, 5.0), (294.5, 4.2)] {
        let mut x = x0 + 3.0 + r.f() * 4.0;
        while x < x1 - 2.0 {
            let s = 0.9 + r.f() * 0.6 + (row - 284.0) * 0.03;
            let z = row + r.signed() * 1.2;
            b.tree(x, z, s);
            x += dens * (0.7 + r.f() * 0.6);
        }
    }
    // matorrales entre los árboles (tapan el paso)
    let mut x = x0 + 2.0;
    while x < x1 - 2.0 {
        let w_ = 2.5 + r.f() * 3.0;
        let h = 1.4 + r.f() * 1.2;
        b.w.add_box(x + w_ / 2.0, 290.5 + r.signed() * 1.5, w_, h, 1.6 + r.f() * 1.4, Mat::Hedge);
        x += w_ + 1.0 + r.f() * 3.0;
    }
    // un sendero de tierra que entra al bosque cada tanto
    for px in [-200.0f32, -60.0, 90.0, 230.0] {
        b.w.add_visual(px, 0.07, 286.0, 2.2, 0.02, 12.0, Mat::InFloor, false);
    }
    // afuera: lomas verdes con bosque (hasta el río)
    let mut x = -FAR;
    while x < QUAY - 20.0 {
        let w_ = 40.0 + r.f() * 40.0;
        for k in 0..3 {
            let z = a + 14.0 + k as f32 * 40.0 + r.f() * 20.0;
            let h = 4.0 + k as f32 * 9.0 + r.f() * 10.0;
            let d = 34.0 + r.f() * 20.0;
            // la última loma no pasa del borde del río
            let w_ = w_.min((QUAY - 2.0 - x) * 2.0).max(4.0);
            b.w.add_visual(x, h / 2.0 - 0.5, z, w_, h, d, Mat::Grass, false);
            for _ in 0..5 {
                b.w.trees.push([x + r.signed() * w_ * 0.4, h + 2.4, z + r.signed() * d * 0.4, 1.8 + r.f() * 1.4]);
            }
        }
        x += w_ * 0.85;
    }
    // piso verde de fondo, debajo de todo lo de afuera (oeste y sur)
    b.w.add_visual(-(a + FAR) / 2.0, -0.6, 0.0, FAR - a, 0.2, FAR * 2.0, Mat::Grass, false);
    b.w.add_visual((QUAY - FAR) / 2.0, -0.6, (a + FAR) / 2.0, QUAY + FAR, 0.2, FAR - a, Mat::Grass, false);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::maps::{MapId, build_level};
    use crate::player::{PlayerInput, PlayerState};

    /// Camina `secs` segundos hacia `yaw` (0 = norte, −Z), saltando cada tanto, y devuelve dónde quedó.
    fn walk(w: &World, x: f32, z: f32, yaw: f32, secs: f32) -> Vec3 {
        let mut p = PlayerState::new(x, z);
        let mut ev = Vec::new();
        let n = (secs / crate::TICK) as usize;
        for i in 0..n {
            let input = PlayerInput { forward: 1.0, yaw, sprint: true, jump: i % 50 == 0, ..Default::default() };
            p.step(&input, w, crate::TICK, &mut ev);
        }
        p.pos
    }

    #[test]
    fn el_rio_frena_en_la_baranda() {
        let l = build_level(MapId::Ciudad);
        // al norte, lejos de los muelles
        let p = walk(&l.world, -60.0, -270.0, 0.0, 6.0);
        assert!(p.z > -QUAY - 0.2 && p.z < -QUAY + 1.5, "quedó en z = {}", p.z);
        assert!(p.y > 1.0, "no se cayó al agua");
        // al este (yaw −π/2 mira a +X)
        let p = walk(&l.world, 270.0, 60.0, -std::f32::consts::FRAC_PI_2, 6.0);
        assert!(p.x < QUAY + 0.2 && p.x > QUAY - 1.5, "quedó en x = {}", p.x);
    }

    #[test]
    fn si_saltas_al_rio_te_devuelve_a_la_orilla() {
        let l = build_level(MapId::Ciudad);
        let mut p = PlayerState::new(-60.0, -QUAY - 6.0);
        p.pos.y = 1.6;
        let mut ev = Vec::new();
        for _ in 0..120 {
            p.step(&PlayerInput::default(), &l.world, crate::TICK, &mut ev);
        }
        assert!(ev.contains(&crate::player::PlayerEvent::Splash));
        assert!(p.pos.z > -QUAY && p.pos.y > 1.0, "volvió a la costanera: {}", p.pos);
    }

    #[test]
    fn los_muelles_se_caminan_hasta_la_punta() {
        let l = build_level(MapId::Ciudad);
        let p = walk(&l.world, 30.0, -270.0, 0.0, 6.0);
        assert!(p.z < -l.world.a + 3.5, "llegó hasta z = {}", p.z);
        assert!(p.y > 1.5, "sobre la madera, no en el agua (y = {})", p.y);
    }

    #[test]
    fn los_cerros_y_el_bosque_cortan_el_paso_sin_muro() {
        let l = build_level(MapId::Ciudad);
        let w = &l.world;
        // ya no hay muro de 12 m en el borde
        for t in [-200.0f32, 0.0, 150.0] {
            assert!(w.box_at(Vec3::new(t, 11.0, w.a)).is_none());
            assert!(w.box_at(Vec3::new(-w.a, 11.0, t)).is_none());
        }
        // cerros: hay roca subiendo hacia el borde
        assert!(w.ground_at(-HILLS - 2.0, 0.0, 30.0) > 0.8);
        assert!(w.ground_at(-HILLS - 10.0, 0.0, 30.0) > w.ground_at(-HILLS - 2.0, 0.0, 30.0));
        // bosque: matorrales con colisión
        let hedges = (-250..250).step_by(5).filter(|x| (288..294).any(|z| w.box_at(Vec3::new(*x as f32, 0.8, z as f32)).is_some())).count();
        assert!(hedges > 40, "matorrales: {hedges}");
    }
}

#[cfg(test)]
mod dump {
    /// `ARENA_DUMP=archivo cargo test -p arena_sim volcar_mapa`: guarda todas las piezas del mapa
    /// (cajas, cilindros, árboles, huecos) en texto, para dibujar una maqueta sin abrir el juego.
    #[test]
    fn volcar_mapa() {
        use std::fmt::Write;
        let Some(path) = std::env::var_os("ARENA_DUMP") else { return };
        let l = crate::maps::build_level(crate::maps::MapId::Ciudad);
        let w = &l.world;
        let mut s = String::new();
        for v in &w.visuals {
            let _ = writeln!(s, "B {:?} {} {} {} {} {} {} {}", v.mat, v.center.x, v.center.y, v.center.z, v.size.x, v.size.y, v.size.z, v.yaw);
        }
        for g in &w.groups {
            for v in &g.boxes {
                let c = g.origin + v.center;
                let _ = writeln!(s, "B {:?} {} {} {} {} {} {} 0", v.mat, c.x, c.y, c.z, v.size.x, v.size.y, v.size.z);
            }
        }
        for c in &w.cylinders {
            let _ = writeln!(s, "C {:?} {} {} {} {} {} {:?}", c.mat, c.center.x, c.center.y, c.center.z, c.radius, c.length, c.axis);
        }
        for t in &w.trees {
            let _ = writeln!(s, "T {} {} {} {}", t[0], t[1], t[2], t[3]);
        }
        for h in &w.holes {
            let _ = writeln!(s, "H {} {} {} {}", h[0], h[1], h[2], h[3]);
        }
        std::fs::write(path, s).unwrap();
    }
}
