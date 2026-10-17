//! Mundo estático: colisionadores en cajas alineadas a los ejes sobre una grilla de 8 m
//! (puerto de `10-colisiones.js` y de los helpers de `03-mapas.js`), más la lista de
//! piezas visuales que el cliente convierte en mallas.

use crate::mat::Mat;
use glam::Vec3;

pub const CELL: f32 = 8.0;
/// Alto de un escalón: las escaleras se arman con cajas de este alto.
pub const STEP: f32 = 0.34;
/// Radio del jugador (también lo usan `ground_at` y `ceiling_at`).
pub const R: f32 = 0.4;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Aabb {
    pub min: Vec3,
    pub max: Vec3,
}

#[derive(Clone, Copy, Debug)]
pub struct VisualBox {
    pub center: Vec3,
    pub size: Vec3,
    pub mat: Mat,
    pub shadow: bool,
    /// Giro sobre Y (0 en casi todo; las calles diagonales y curvas lo usan).
    pub yaw: f32,
}

/// Marco local de un colisionador girado: centro en XZ, seno y coseno del giro y la caja sin girar.
#[derive(Clone, Copy, Debug)]
struct Frame {
    cx: f32,
    cz: f32,
    c: f32,
    s: f32,
    local: Aabb,
}

impl Frame {
    /// Punto del mundo al marco local (el centro queda en el mismo lugar).
    fn to_local(&self, x: f32, z: f32) -> (f32, f32) {
        let (dx, dz) = (x - self.cx, z - self.cz);
        (self.cx + dx * self.c - dz * self.s, self.cz + dx * self.s + dz * self.c)
    }
    /// Desplazamiento local al mundo.
    fn delta_to_world(&self, dx: f32, dz: f32) -> (f32, f32) {
        (dx * self.c + dz * self.s, -dx * self.s + dz * self.c)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CylAxis {
    X,
    Y,
    Z,
}

#[derive(Clone, Copy, Debug)]
pub struct Cylinder {
    pub center: Vec3,
    pub radius: f32,
    pub length: f32,
    pub axis: CylAxis,
    pub mat: Mat,
}

/// Caja girada sobre Y, sin colisión (skyline de fondo).
#[derive(Clone, Copy, Debug)]
pub struct RotBox {
    pub center: Vec3,
    pub size: Vec3,
    pub yaw: f32,
    pub mat: Mat,
}

/// Zona que quema (piletas de metal fundido).
#[derive(Clone, Copy, Debug)]
pub struct Zone {
    pub x: f32,
    pub z: f32,
    pub w: f32,
    pub d: f32,
    pub dps: f32,
}

/// Conjunto de piezas que se mueve junto (la antena que se cae). Cajas en coordenadas locales.
#[derive(Clone, Debug, Default)]
pub struct VisualGroup {
    pub origin: Vec3,
    pub boxes: Vec<VisualBox>,
}

pub type ColliderId = u32;

pub struct World {
    /// Medio lado del mapa: el muro está en ±a.
    pub a: f32,
    boxes: Vec<Option<Aabb>>,
    /// Marco girado de cada caja (None: alineada a los ejes).
    frames: Vec<Option<Frame>>,
    /// Huecos en el piso (entradas al subte): ahí no hay plano en y = 0.
    pub holes: Vec<[f32; 4]>,
    /// Rango de celdas de cada caja, para no repetirla en las consultas.
    ranges: Vec<[i32; 4]>,
    cells: Vec<Vec<ColliderId>>,
    n: i32,
    off: i32,
    pub visuals: Vec<VisualBox>,
    pub cylinders: Vec<Cylinder>,
    pub rot_boxes: Vec<RotBox>,
    /// Copas de árboles: [x, y, z, escala].
    pub trees: Vec<[f32; 4]>,
    pub zones: Vec<Zone>,
    pub groups: Vec<VisualGroup>,
}

impl World {
    pub fn new(a: f32) -> Self {
        // la grilla cubre el mapa con margen; lo que cae afuera va a las celdas del borde
        let off = ((a + 2.0 * CELL) / CELL).ceil() as i32;
        let n = off * 2;
        Self {
            a,
            boxes: Vec::new(),
            frames: Vec::new(),
            holes: Vec::new(),
            ranges: Vec::new(),
            cells: vec![Vec::new(); (n * n) as usize],
            n,
            off,
            visuals: Vec::new(),
            cylinders: Vec::new(),
            rot_boxes: Vec::new(),
            trees: Vec::new(),
            zones: Vec::new(),
            groups: Vec::new(),
        }
    }

    /// Límite de movimiento dentro del muro.
    pub fn lim(&self) -> f32 {
        self.a - 1.5
    }

    fn cell_ix(&self, v: f32) -> i32 {
        ((v / CELL).floor() as i32 + self.off).clamp(0, self.n - 1)
    }

    fn cell(&self, ix: i32, iz: i32) -> &[ColliderId] {
        &self.cells[(ix * self.n + iz) as usize]
    }

    pub fn colliders(&self) -> impl Iterator<Item = &Aabb> {
        self.boxes.iter().flatten()
    }

    pub fn collider(&self, id: ColliderId) -> Option<&Aabb> {
        self.boxes.get(id as usize).and_then(|b| b.as_ref())
    }

    /* ---------- construcción ---------- */

    pub fn add_collider(&mut self, x: f32, z: f32, w: f32, h: f32, d: f32, y0: f32) -> ColliderId {
        let b = Aabb {
            min: Vec3::new(x - w / 2.0, y0, z - d / 2.0),
            max: Vec3::new(x + w / 2.0, y0 + h, z + d / 2.0),
        };
        let id = self.boxes.len() as ColliderId;
        let r = [self.cell_ix(b.min.x), self.cell_ix(b.min.z), self.cell_ix(b.max.x), self.cell_ix(b.max.z)];
        for ix in r[0]..=r[2] {
            for iz in r[1]..=r[3] {
                let n = self.n;
                self.cells[(ix * n + iz) as usize].push(id);
            }
        }
        self.boxes.push(Some(b));
        self.frames.push(None);
        self.ranges.push(r);
        id
    }

    /// Colisionador girado `yaw` sobre Y (caja de w × h × d con centro en x, z y base en y0).
    #[allow(clippy::too_many_arguments)]
    pub fn add_obb(&mut self, x: f32, z: f32, w: f32, h: f32, d: f32, y0: f32, yaw: f32) -> ColliderId {
        let (s, c) = yaw.sin_cos();
        if s.abs() < 1e-4 {
            return self.add_collider(x, z, w, h, d, y0);
        }
        // caja envolvente para la grilla
        let (bw, bd) = (w * c.abs() + d * s.abs(), w * s.abs() + d * c.abs());
        let id = self.add_collider(x, z, bw, h, bd, y0);
        let local = Aabb { min: Vec3::new(x - w / 2.0, y0, z - d / 2.0), max: Vec3::new(x + w / 2.0, y0 + h, z + d / 2.0) };
        self.frames[id as usize] = Some(Frame { cx: x, cz: z, c, s, local });
        id
    }

    /// Caja visual girada con colisión, apoyada en `y0`.
    #[allow(clippy::too_many_arguments)]
    pub fn add_block_rot(&mut self, x: f32, y0: f32, z: f32, w: f32, h: f32, d: f32, yaw: f32, mat: Mat) -> ColliderId {
        self.visuals.push(VisualBox { center: Vec3::new(x, y0 + h / 2.0, z), size: Vec3::new(w, h, d), mat, shadow: true, yaw });
        self.add_obb(x, z, w, h, d, y0, yaw)
    }

    /// Caja visual girada sin colisión.
    #[allow(clippy::too_many_arguments)]
    pub fn add_visual_rot(&mut self, x: f32, y: f32, z: f32, w: f32, h: f32, d: f32, yaw: f32, mat: Mat, shadow: bool) {
        self.visuals.push(VisualBox { center: Vec3::new(x, y, z), size: Vec3::new(w, h, d), mat, shadow, yaw });
    }

    /// ¿(x, z) cae en un hueco del piso?
    pub fn in_hole(&self, x: f32, z: f32) -> bool {
        self.holes.iter().any(|h| x > h[0] && x < h[2] && z > h[1] && z < h[3])
    }

    /// Caja de un colisionador en su marco local y el punto llevado a ese marco.
    fn local(&self, id: ColliderId, b: &Aabb, x: f32, z: f32) -> (Aabb, f32, f32, Option<Frame>) {
        match self.frames[id as usize] {
            Some(f) => {
                let (lx, lz) = f.to_local(x, z);
                (f.local, lx, lz, Some(f))
            }
            None => (*b, x, z, None),
        }
    }

    pub fn remove_collider(&mut self, id: ColliderId) {
        let Some(slot) = self.boxes.get_mut(id as usize) else { return };
        if slot.take().is_none() {
            return;
        }
        let r = self.ranges[id as usize];
        for ix in r[0]..=r[2] {
            for iz in r[1]..=r[3] {
                let n = self.n;
                self.cells[(ix * n + iz) as usize].retain(|&c| c != id);
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn add_visual(&mut self, x: f32, y: f32, z: f32, w: f32, h: f32, d: f32, mat: Mat, shadow: bool) {
        self.visuals.push(VisualBox { center: Vec3::new(x, y, z), size: Vec3::new(w, h, d), mat, shadow, yaw: 0.0 });
    }

    /// Caja apoyada en el piso, con colisión.
    pub fn add_box(&mut self, x: f32, z: f32, w: f32, h: f32, d: f32, mat: Mat) -> ColliderId {
        self.add_visual(x, h / 2.0, z, w, h, d, mat, true);
        self.add_collider(x, z, w, h, d, 0.0)
    }

    /// Caja que arranca en altura `y0`, con colisión.
    #[allow(clippy::too_many_arguments)]
    pub fn add_block(&mut self, x: f32, y0: f32, z: f32, w: f32, h: f32, d: f32, mat: Mat) -> ColliderId {
        self.add_visual(x, y0 + h / 2.0, z, w, h, d, mat, true);
        self.add_collider(x, z, w, h, d, y0)
    }

    /// Tira decorativa: sin colisión ni sombra.
    #[allow(clippy::too_many_arguments)]
    pub fn add_strip(&mut self, x: f32, y: f32, z: f32, w: f32, h: f32, d: f32, mat: Mat) {
        self.add_visual(x, y, z, w, h, d, mat, false);
    }

    #[allow(clippy::too_many_arguments)]
    pub fn add_stairs(&mut self, ex: f32, ez: f32, dx: f32, dz: f32, width: f32, top_h: f32, mat: Mat) {
        let n = (top_h / STEP).round() as i32;
        let depth = 0.75;
        for k in 1..n {
            let k = k as f32;
            let h = top_h - k * STEP;
            let cx = ex + dx * (k - 0.5) * depth;
            let cz = ez + dz * (k - 0.5) * depth;
            let w = if dx != 0.0 { depth } else { width };
            let d = if dz != 0.0 { depth } else { width };
            self.add_box(cx, cz, w, h, d, mat);
        }
    }

    pub fn add_container(&mut self, x: f32, z: f32, along_x: bool, stack: u32, ci: usize) {
        let (w, d, h) = if along_x { (6.2, 2.6, 2.7) } else { (2.6, 6.2, 2.7) };
        for s in 0..stack {
            let m = Mat::Container(((ci + s as usize) % crate::mat::CONTAINERS as usize) as u8);
            self.add_visual(x, h / 2.0 + s as f32 * h, z, w, h, d, m, true);
        }
        self.add_collider(x, z, w, h * stack as f32, d, 0.0);
    }

    pub fn add_perimeter(&mut self, h: f32, strip_y: Option<f32>) {
        let a = self.a;
        self.add_box(0.0, -a, a * 2.0 + 2.0, h, 2.0, Mat::Wall);
        self.add_box(0.0, a, a * 2.0 + 2.0, h, 2.0, Mat::Wall);
        self.add_box(-a, 0.0, 2.0, h, a * 2.0 + 2.0, Mat::Wall);
        self.add_box(a, 0.0, 2.0, h, a * 2.0 + 2.0, Mat::Wall);
        if let Some(y) = strip_y {
            self.add_strip(0.0, y, -a + 1.05, a * 2.0, 0.14, 0.1, Mat::Strip);
            self.add_strip(0.0, y, a - 1.05, a * 2.0, 0.14, 0.1, Mat::Strip);
            self.add_strip(-a + 1.05, y, 0.0, 0.1, 0.14, a * 2.0, Mat::Strip);
            self.add_strip(a - 1.05, y, 0.0, 0.1, 0.14, a * 2.0, Mat::Strip);
        }
    }

    pub fn add_lava(&mut self, x: f32, z: f32, w: f32, d: f32, dps: f32) {
        self.add_visual(x, 0.02, z, w, 0.004, d, Mat::Lava, false);
        self.add_strip(x, 0.03, z - d / 2.0 - 0.1, w + 0.4, 0.02, 0.2, Mat::Strip);
        self.add_strip(x, 0.03, z + d / 2.0 + 0.1, w + 0.4, 0.02, 0.2, Mat::Strip);
        self.add_strip(x - w / 2.0 - 0.1, 0.03, z, 0.2, 0.02, d, Mat::Strip);
        self.add_strip(x + w / 2.0 + 0.1, 0.03, z, 0.2, 0.02, d, Mat::Strip);
        self.zones.push(Zone { x, z, w, d, dps });
    }

    /* ---------- consultas ---------- */

    /// Recorre una vez cada caja que toca el rectángulo XZ (caja envolvente, sin girar).
    pub fn query(&self, min_x: f32, min_z: f32, max_x: f32, max_z: f32, mut f: impl FnMut(&Aabb)) {
        self.query_id(min_x, min_z, max_x, max_z, |_, b| f(b));
    }

    fn query_id(&self, min_x: f32, min_z: f32, max_x: f32, max_z: f32, mut f: impl FnMut(ColliderId, &Aabb)) {
        let (qx0, qz0, qx1, qz1) = (self.cell_ix(min_x), self.cell_ix(min_z), self.cell_ix(max_x), self.cell_ix(max_z));
        for ix in qx0..=qx1 {
            for iz in qz0..=qz1 {
                for &id in self.cell(ix, iz) {
                    let r = self.ranges[id as usize];
                    // una caja que ocupa varias celdas se procesa solo en la primera que comparte con la consulta
                    if ix != r[0].max(qx0) || iz != r[1].max(qz0) {
                        continue;
                    }
                    if let Some(b) = &self.boxes[id as usize] {
                        f(id, b);
                    }
                }
            }
        }
    }

    /// Empuja un cilindro vertical (pies a `feet`, alto `height`) fuera de las cajas.
    pub fn collide_xz(&self, p: &mut Vec3, r: f32, feet: f32, height: f32) {
        let mut q = *p;
        self.query_id(p.x - r, p.z - r, p.x + r, p.z + r, |id, bb| {
            let (b, lx, lz, frame) = self.local(id, bb, q.x, q.z);
            if feet >= b.max.y - 0.35 || feet + height <= b.min.y {
                return;
            }
            let cx = lx.clamp(b.min.x, b.max.x);
            let cz = lz.clamp(b.min.z, b.max.z);
            let (dx, dz) = (lx - cx, lz - cz);
            let d2 = dx * dx + dz * dz;
            if d2 >= r * r {
                return;
            }
            let (mut mx, mut mz) = (0.0, 0.0);
            if d2 > 1e-8 {
                let d = d2.sqrt();
                let k = (r - d) / d;
                mx = dx * k;
                mz = dz * k;
            } else {
                let (ox0, ox1) = (b.min.x - r - lx, b.max.x + r - lx);
                let (oz0, oz1) = (b.min.z - r - lz, b.max.z + r - lz);
                let px = if ox0.abs() < ox1.abs() { ox0 } else { ox1 };
                let pz = if oz0.abs() < oz1.abs() { oz0 } else { oz1 };
                if px.abs() < pz.abs() {
                    mx = px;
                } else {
                    mz = pz;
                }
            }
            let (wx, wz) = frame.map_or((mx, mz), |f| f.delta_to_world(mx, mz));
            q.x += wx;
            q.z += wz;
        });
        p.x = q.x;
        p.z = q.z;
    }

    /// Altura del piso bajo (x, z) para unos pies a `feet` (sube escalones de hasta 0,36 m).
    /// El plano y = 0 cuenta salvo en los huecos del subte y si los pies están muy por debajo.
    pub fn ground_at(&self, x: f32, z: f32, feet: f32) -> f32 {
        let e = R * 0.5;
        let mut g = if feet >= -0.36 && !self.in_hole(x, z) { 0.0f32 } else { -1000.0 };
        self.query_id(x - e, z - e, x + e, z + e, |id, bb| {
            let (b, lx, lz, _) = self.local(id, bb, x, z);
            if lx > b.min.x - e && lx < b.max.x + e && lz > b.min.z - e && lz < b.max.z + e && feet >= b.max.y - 0.36 && b.max.y > g {
                g = b.max.y;
            }
        });
        g
    }

    /// Techo: la primera losa que la cabeza cruzaría al subir de `head_old` a `head_new`.
    pub fn ceiling_at(&self, x: f32, z: f32, head_old: f32, head_new: f32) -> Option<f32> {
        let e = R * 0.7;
        let mut c: Option<f32> = None;
        // bajo tierra, el piso de la calle es techo (salvo en los huecos)
        if head_old < 0.0 && head_new >= 0.0 && !self.in_hole(x, z) {
            c = Some(0.0);
        }
        self.query_id(x - e, z - e, x + e, z + e, |id, bb| {
            let (b, lx, lz, _) = self.local(id, bb, x, z);
            if b.min.y.abs() > 0.05
                && b.min.y >= head_old - 0.02
                && b.min.y < head_new
                && lx > b.min.x - e
                && lx < b.max.x + e
                && lz > b.min.z - e
                && lz < b.max.z + e
                && c.is_none_or(|c| b.min.y < c)
            {
                c = Some(b.min.y);
            }
        });
        c
    }

    /// Caja que contiene el punto, si hay (para giradas devuelve la envolvente).
    pub fn box_at(&self, p: Vec3) -> Option<Aabb> {
        let mut hit = None;
        self.query_id(p.x, p.z, p.x, p.z, |id, bb| {
            let (b, lx, lz, _) = self.local(id, bb, p.x, p.z);
            if hit.is_none() && lx > b.min.x && lx < b.max.x && lz > b.min.z && lz < b.max.z && p.y < b.max.y && p.y > b.min.y {
                hit = Some(*bb);
            }
        });
        hit
    }

    /// Rayo contra el mundo (cajas y piso). Devuelve la distancia al primer impacto o `far`.
    pub fn ray(&self, o: Vec3, d: Vec3, far: f32) -> f32 {
        let mut best = far;
        if d.y.abs() > 1e-6 {
            // el plano de la calle tapa en los dos sentidos (desde abajo, salvo los huecos del subte)
            let t = -o.y / d.y;
            if t >= 0.0 && t < best && !self.in_hole(o.x + d.x * t, o.z + d.z * t) {
                best = t;
            }
        }
        let step = CELL * 0.35;
        // celdas ya revisadas (si se llena, se revisan de nuevo: el resultado es el mismo)
        let mut seen = [0u32; 128];
        let mut ns = 0usize;
        let mut t = 0.0f32;
        while t <= best + step {
            let tt = t.min(best);
            let (x, z) = (o.x + d.x * tt, o.z + d.z * tt);
            for (ox, oz) in [(-0.3, -0.3), (-0.3, 0.3), (0.3, -0.3), (0.3, 0.3)] {
                let (ix, iz) = (self.cell_ix(x + ox), self.cell_ix(z + oz));
                let key = (ix * self.n + iz) as u32;
                if seen[..ns].contains(&key) {
                    continue;
                }
                if ns < seen.len() {
                    seen[ns] = key;
                    ns += 1;
                }
                for &id in self.cell(ix, iz) {
                    if let Some(b) = &self.boxes[id as usize] {
                        let h = match self.frames[id as usize] {
                            Some(f) => {
                                let (lx, lz) = f.to_local(o.x, o.z);
                                let ld = Vec3::new(d.x * f.c - d.z * f.s, d.y, d.x * f.s + d.z * f.c);
                                ray_aabb(Vec3::new(lx, o.y, lz), ld, &f.local, best)
                            }
                            None => ray_aabb(o, d, b, best),
                        };
                        if let Some(h) = h {
                            if h < best {
                                best = h;
                            }
                        }
                    }
                }
            }
            t += step;
        }
        best
    }

    pub fn has_los(&self, from: Vec3, to: Vec3) -> bool {
        let v = to - from;
        let dist = v.length();
        if dist < 0.01 {
            return true;
        }
        self.ray(from, v / dist, dist) >= dist - 0.05
    }
}

/// Distancia de entrada del rayo a la caja, si la toca antes de `tmax`.
pub fn ray_aabb(o: Vec3, d: Vec3, b: &Aabb, tmax: f32) -> Option<f32> {
    let (mut t0, mut t1) = (0.0f32, tmax);
    for a in 0..3 {
        let (oa, da, mn, mx) = (o[a], d[a], b.min[a], b.max[a]);
        if da.abs() < 1e-9 {
            if oa < mn || oa > mx {
                return None;
            }
            continue;
        }
        let inv = 1.0 / da;
        let (mut tn, mut tf) = ((mn - oa) * inv, (mx - oa) * inv);
        if tn > tf {
            std::mem::swap(&mut tn, &mut tf);
        }
        t0 = t0.max(tn);
        t1 = t1.min(tf);
        if t0 > t1 {
            return None;
        }
    }
    Some(t0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mundo() -> World {
        let mut w = World::new(60.0);
        w.add_box(0.0, 0.0, 4.0, 2.0, 4.0, Mat::Crate); // caja de 4×2×4 en el origen
        w.add_block(10.0, 3.0, 0.0, 6.0, 0.2, 6.0, Mat::Roof); // losa en altura
        w
    }

    #[test]
    fn empuja_fuera_de_la_caja() {
        let w = mundo();
        let mut p = Vec3::new(2.1, 1.6, 0.0);
        w.collide_xz(&mut p, R, 0.0, 1.8);
        assert!(p.x >= 2.0 + R - 1e-4, "x = {}", p.x);
    }

    #[test]
    fn piso_y_escalon() {
        let w = mundo();
        assert_eq!(w.ground_at(0.0, 0.0, 2.0), 2.0); // arriba de la caja
        assert_eq!(w.ground_at(0.0, 0.0, 0.0), 0.0); // pies abajo: la caja es pared, no piso
        assert_eq!(w.ground_at(20.0, 20.0, 0.0), 0.0);
    }

    #[test]
    fn techo() {
        let w = mundo();
        assert_eq!(w.ceiling_at(10.0, 0.0, 2.0, 3.5), Some(3.0));
        assert_eq!(w.ceiling_at(10.0, 0.0, 3.5, 4.0), None);
    }

    #[test]
    fn rayo_y_linea_de_vision() {
        let w = mundo();
        let t = w.ray(Vec3::new(-10.0, 1.0, 0.0), Vec3::X, 100.0);
        assert!((t - 8.0).abs() < 1e-4, "t = {t}");
        assert!(!w.has_los(Vec3::new(-10.0, 1.0, 0.0), Vec3::new(10.0, 1.0, 0.0)));
        assert!(w.has_los(Vec3::new(-10.0, 3.0, 0.0), Vec3::new(5.0, 2.5, 0.0)));
        // hacia abajo pega en el piso
        let t = w.ray(Vec3::new(30.0, 5.0, 30.0), Vec3::NEG_Y, 100.0);
        assert!((t - 5.0).abs() < 1e-4);
    }

    #[test]
    fn caja_girada_choca_y_sostiene() {
        let mut w = World::new(60.0);
        // pared de 10 × 1 girada 45°
        w.add_block_rot(0.0, 0.0, 0.0, 10.0, 3.0, 1.0, std::f32::consts::FRAC_PI_4, Mat::Wall);
        // un punto en la diagonal libre de la envolvente no choca
        let mut p = Vec3::new(3.0, 1.0, 3.0);
        w.collide_xz(&mut p, 0.4, 0.0, 1.8);
        assert!((p - Vec3::new(3.0, 1.0, 3.0)).length() < 1e-4, "{p:?}");
        // sobre la pared se puede parar
        assert!((w.ground_at(2.0, -2.0, 3.0) - 3.0).abs() < 1e-4);
        // un rayo de frente pega a la distancia correcta (media diagonal de 0,5 m)
        let dir = Vec3::new(1.0, 0.0, 1.0).normalize();
        let t = w.ray(Vec3::new(-5.0, 1.0, -5.0), dir, 50.0);
        assert!((t - (50.0f32.sqrt() - 0.5)).abs() < 0.05, "t = {t}");
    }

    #[test]
    fn hueco_del_subte() {
        let mut w = World::new(60.0);
        w.holes.push([-2.0, -4.0, 2.0, 4.0]);
        w.add_block(0.0, -6.5, 10.0, 10.0, 0.5, 30.0, Mat::Concrete); // piso de la estación
        assert_eq!(w.ground_at(10.0, 10.0, 0.0), 0.0);
        assert!(w.ground_at(0.0, 0.0, 0.0) < -100.0 || (w.ground_at(0.0, 0.0, 0.0) + 6.0).abs() < 1e-4);
        // abajo, el piso es el de la estación
        assert!((w.ground_at(0.0, 10.0, -6.0) + 6.0).abs() < 1e-4);
        // desde abajo no se ve la calle
        assert!(!w.has_los(Vec3::new(0.0, -5.0, 10.0), Vec3::new(0.0, 2.0, 20.0)));
    }

    #[test]
    fn sacar_colisionador() {
        let mut w = World::new(20.0);
        let id = w.add_box(0.0, 0.0, 1.0, 5.0, 1.0, Mat::Steel);
        assert!(w.box_at(Vec3::new(0.0, 1.0, 0.0)).is_some());
        w.remove_collider(id);
        assert!(w.box_at(Vec3::new(0.0, 1.0, 0.0)).is_none());
    }

    #[test]
    fn caja_en_varias_celdas_se_procesa_una_vez() {
        let mut w = World::new(60.0);
        w.add_box(0.0, 0.0, 40.0, 1.0, 40.0, Mat::Wall);
        let mut n = 0;
        w.query(-30.0, -30.0, 30.0, 30.0, |_| n += 1);
        assert_eq!(n, 1);
    }
}
