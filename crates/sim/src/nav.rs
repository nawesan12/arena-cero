//! Navegación de los soldados: grilla de 0,5 m sobre el área de cada puesto (planta baja) y A*.
//! En la web los soldados iban en línea recta hacia su destino y se trababan contra las cajas.

use crate::world::World;
use glam::Vec2;
use std::cmp::Ordering;
use std::collections::BinaryHeap;

const CELL: f32 = 0.5;
/// Radio del soldado más un margen.
const CLEAR: f32 = 0.4;

pub struct NavGrid {
    origin: Vec2,
    n: i32,
    free: Vec<bool>,
}

#[derive(Clone, Copy, PartialEq)]
struct Node {
    f: f32,
    i: usize,
}
impl Eq for Node {}
impl Ord for Node {
    fn cmp(&self, o: &Self) -> Ordering {
        o.f.total_cmp(&self.f)
    }
}
impl PartialOrd for Node {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}

impl NavGrid {
    /// Grilla cuadrada de lado `size` centrada en (cx, cz), para pies a la altura `floor`.
    pub fn build(world: &World, cx: f32, cz: f32, size: f32, floor: f32) -> Self {
        let n = (size / CELL) as i32;
        let origin = Vec2::new(cx - size / 2.0, cz - size / 2.0);
        let mut free = vec![true; (n * n) as usize];
        for iz in 0..n {
            for ix in 0..n {
                let p = origin + Vec2::new((ix as f32 + 0.5) * CELL, (iz as f32 + 0.5) * CELL);
                let mut blocked = false;
                world.query(p.x - CLEAR, p.y - CLEAR, p.x + CLEAR, p.y + CLEAR, |b| {
                    // obstáculo si ocupa la franja del cuerpo (los escalones bajos se pueden subir)
                    if b.max.y > floor + 0.36 && b.min.y < floor + 1.8 && p.x > b.min.x - CLEAR && p.x < b.max.x + CLEAR && p.y > b.min.z - CLEAR && p.y < b.max.z + CLEAR {
                        blocked = true;
                    }
                });
                free[(iz * n + ix) as usize] = !blocked;
            }
        }
        Self { origin, n, free }
    }

    fn cell(&self, p: Vec2) -> Option<(i32, i32)> {
        let q = (p - self.origin) / CELL;
        let (ix, iz) = (q.x.floor() as i32, q.y.floor() as i32);
        (ix >= 0 && iz >= 0 && ix < self.n && iz < self.n).then_some((ix, iz))
    }

    fn ok(&self, ix: i32, iz: i32) -> bool {
        ix >= 0 && iz >= 0 && ix < self.n && iz < self.n && self.free[(iz * self.n + ix) as usize]
    }

    fn center(&self, ix: i32, iz: i32) -> Vec2 {
        self.origin + Vec2::new((ix as f32 + 0.5) * CELL, (iz as f32 + 0.5) * CELL)
    }

    /// Celda libre más cercana (por si el punto cae justo contra una pared).
    fn nearest_free(&self, c: (i32, i32)) -> Option<(i32, i32)> {
        for r in 0..6 {
            for dz in -r..=r {
                for dx in -r..=r {
                    if (dx as i32).abs().max((dz as i32).abs()) == r && self.ok(c.0 + dx, c.1 + dz) {
                        return Some((c.0 + dx, c.1 + dz));
                    }
                }
            }
        }
        None
    }

    /// ¿Se puede ir en línea recta entre dos puntos sin pisar celdas ocupadas?
    fn straight(&self, a: Vec2, b: Vec2) -> bool {
        let steps = (a.distance(b) / (CELL * 0.5)).ceil().max(1.0) as i32;
        (0..=steps).all(|k| {
            let p = a.lerp(b, k as f32 / steps as f32);
            self.cell(p).is_some_and(|(x, z)| self.ok(x, z))
        })
    }

    /// Camino de `from` a `to` (sin incluir el punto de partida), suavizado.
    pub fn find(&self, from: Vec2, to: Vec2) -> Option<Vec<Vec2>> {
        let s = self.nearest_free(self.cell(from)?)?;
        let g = self.nearest_free(self.cell(to)?)?;
        let idx = |c: (i32, i32)| (c.1 * self.n + c.0) as usize;
        let h = |c: (i32, i32)| (((c.0 - g.0) as f32).powi(2) + ((c.1 - g.1) as f32).powi(2)).sqrt();
        let total = (self.n * self.n) as usize;
        let mut cost = vec![f32::MAX; total];
        let mut prev = vec![usize::MAX; total];
        let mut open = BinaryHeap::new();
        cost[idx(s)] = 0.0;
        open.push(Node { f: h(s), i: idx(s) });
        let mut found = false;
        let mut expanded = 0;
        while let Some(Node { i, .. }) = open.pop() {
            if i == idx(g) {
                found = true;
                break;
            }
            expanded += 1;
            if expanded > 6000 {
                break;
            }
            let (cx, cz) = ((i as i32) % self.n, (i as i32) / self.n);
            for (dx, dz) in [(1, 0), (-1, 0), (0, 1), (0, -1), (1, 1), (1, -1), (-1, 1), (-1, -1)] {
                let (nx, nz) = (cx + dx, cz + dz);
                if !self.ok(nx, nz) {
                    continue;
                }
                // no cortar esquinas en diagonal
                if dx != 0 && dz != 0 && (!self.ok(cx + dx, cz) || !self.ok(cx, cz + dz)) {
                    continue;
                }
                let step = if dx != 0 && dz != 0 { std::f32::consts::SQRT_2 } else { 1.0 };
                let ni = idx((nx, nz));
                let c = cost[i] + step;
                if c < cost[ni] {
                    cost[ni] = c;
                    prev[ni] = i;
                    open.push(Node { f: c + h((nx, nz)), i: ni });
                }
            }
        }
        if !found {
            return None;
        }
        let mut cells = vec![idx(g)];
        while let Some(&last) = cells.last() {
            let p = prev[last];
            if p == usize::MAX {
                break;
            }
            cells.push(p);
        }
        cells.reverse();
        let pts: Vec<Vec2> = cells.iter().map(|&i| self.center((i as i32) % self.n, (i as i32) / self.n)).collect();
        // suavizado: saltear puntos mientras haya línea recta libre
        let mut out = Vec::new();
        let mut cur = from;
        let mut k = 0;
        while k < pts.len() {
            let mut j = pts.len() - 1;
            while j > k && !self.straight(cur, pts[j]) {
                j -= 1;
            }
            out.push(pts[j]);
            cur = pts[j];
            k = j + 1;
        }
        if let Some(last) = out.last_mut() {
            if self.straight(*last, to) {
                *last = to;
            }
        }
        Some(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mat::Mat;

    #[test]
    fn rodea_una_pared() {
        let mut w = World::new(60.0);
        // pared de 10 m entre el inicio y el destino
        w.add_box(0.0, 0.0, 10.0, 2.0, 0.6, Mat::Wall);
        let g = NavGrid::build(&w, 0.0, 0.0, 30.0, 0.0);
        let path = g.find(Vec2::new(0.0, -5.0), Vec2::new(0.0, 5.0)).expect("camino");
        assert!(path.len() >= 2, "{path:?}");
        // ningún tramo atraviesa la pared
        let mut a = Vec2::new(0.0, -5.0);
        for p in &path {
            let mid = (a + *p) / 2.0;
            assert!(!(mid.x.abs() < 5.0 && mid.y.abs() < 0.3), "corta la pared: {a:?} → {p:?}");
            a = *p;
        }
        assert!(path.last().unwrap().distance(Vec2::new(0.0, 5.0)) < 0.01);
    }
}
