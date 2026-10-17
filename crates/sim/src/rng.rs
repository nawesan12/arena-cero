/// Generador mulberry32, idéntico al `rng(seed)` de la versión web: con la misma semilla
/// produce la misma secuencia, así el Distrito Sur sale igual que en `03b-ciudad.js`.
#[derive(Clone, Debug)]
pub struct Rng(u32);

impl Rng {
    pub fn new(seed: u32) -> Self {
        Self(seed)
    }

    /// Número en [0, 1), como `Math.random()`.
    pub fn next(&mut self) -> f64 {
        self.0 = self.0.wrapping_add(0x6D2B_79F5);
        let s = self.0;
        let mut t = (s ^ (s >> 15)).wrapping_mul(1 | s);
        t = t.wrapping_add((t ^ (t >> 7)).wrapping_mul(61 | t)) ^ t;
        f64::from(t ^ (t >> 14)) / 4_294_967_296.0
    }

    pub fn f(&mut self) -> f32 {
        self.next() as f32
    }

    /// Entero en [0, n), como `r() * n | 0`.
    pub fn idx(&mut self, n: usize) -> usize {
        (self.next() * n as f64) as usize
    }

    /// Valor en [-1, 1).
    pub fn signed(&mut self) -> f32 {
        (self.next() * 2.0 - 1.0) as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn misma_semilla_misma_secuencia() {
        let mut a = Rng::new(20261016);
        let mut b = Rng::new(20261016);
        for _ in 0..100 {
            assert_eq!(a.next().to_bits(), b.next().to_bits());
        }
    }

    #[test]
    fn rango() {
        let mut r = Rng::new(1);
        for _ in 0..10_000 {
            let v = r.next();
            assert!((0.0..1.0).contains(&v));
        }
    }
}
