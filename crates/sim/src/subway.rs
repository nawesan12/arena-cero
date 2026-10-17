//! Trenes del subte: en cada estación entra un tren por el túnel, frena en el andén, abre las
//! puertas, espera y sigue viaje por el otro túnel. Mientras está, tiene un colisionador (y si te
//! agarra en las vías, te pasa por encima). Subirse con las puertas abiertas te lleva a otra estación.

use crate::game::{Game, GameEvent};
use crate::maps::Level;
use crate::world::ColliderId;
use glam::Vec3;

/// Duración de una vuelta completa en una estación (s).
pub const CYCLE: f32 = 50.0;
const ARRIVE: f32 = 9.0;
const STOP: f32 = 13.0;
const LEAVE: f32 = 8.0;
/// Distancia a la que aparece y desaparece en el túnel.
const RUN: f32 = 95.0;
/// Tamaño del tren: tres coches.
pub const LEN: f32 = 40.0;
pub const WIDTH: f32 = 2.8;
pub const HEIGHT: f32 = 3.9;
pub const CARS: usize = 3;

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Train {
    pub t: f32,
    col: Option<ColliderId>,
}

/// Desplazamiento del tren respecto de su parada (None: no está en la estación).
pub fn offset(t: f32) -> Option<f32> {
    if t < ARRIVE {
        let s = 1.0 - t / ARRIVE;
        Some(-RUN * s * s)
    } else if t < ARRIVE + STOP {
        Some(0.0)
    } else if t < ARRIVE + STOP + LEAVE {
        let s = (t - ARRIVE - STOP) / LEAVE;
        Some(RUN * s * s)
    } else {
        None
    }
}

/// Puertas abiertas (con un margen para que se vean abrir y cerrar).
pub fn doors(t: f32) -> f32 {
    let a = t - ARRIVE - 0.6;
    let b = ARRIVE + STOP - 1.2 - t;
    (a.min(b) / 0.8).clamp(0.0, 1.0)
}

/// Segundos que faltan para que llegue el próximo (0 si ya está en el andén).
pub fn next_in(t: f32) -> f32 {
    if t < ARRIVE + STOP { (ARRIVE - t).max(0.0) } else { CYCLE - t + ARRIVE }
}

impl Game {
    pub(crate) fn init_trains(&mut self, n: usize) {
        self.trains = (0..n).map(|i| Train { t: (i as f32 * 17.0 + 4.0) % CYCLE, col: None }).collect();
    }

    /// Cliente online: pone los trenes en el momento que dice el servidor (con su colisionador).
    pub fn mirror_trains(&mut self, level: &mut Level, ts: &[f32]) {
        let Some(c) = level.city.as_ref() else { return };
        let tracks: Vec<Vec3> = c.stations.iter().map(|s| s.track).collect();
        for (i, (tr, t)) in self.trains.iter_mut().zip(ts).enumerate() {
            tr.t = *t;
            if let Some(col) = tr.col.take() {
                level.world.remove_collider(col);
            }
            if let Some(dx) = offset(tr.t) {
                let p = tracks[i] + Vec3::X * dx;
                tr.col = Some(level.world.add_collider(p.x, p.z, LEN, HEIGHT, WIDTH, p.y + 0.3));
            }
        }
    }

    pub(crate) fn teardown_trains(&mut self, world: &mut crate::world::World) {
        for tr in &mut self.trains {
            if let Some(c) = tr.col.take() {
                world.remove_collider(c);
            }
        }
    }

    pub(crate) fn step_trains(&mut self, level: &mut Level, dt: f32) {
        let Some(c) = level.city.as_ref() else { return };
        let feet: Vec<Vec3> = self.targets.iter().map(|t| Vec3::new(t.pos.x, t.feet, t.pos.z)).collect();
        let tracks: Vec<Vec3> = c.stations.iter().map(|s| s.track).collect();
        for (i, tr) in self.trains.iter_mut().enumerate() {
            let before = offset(tr.t);
            tr.t = (tr.t + dt) % CYCLE;
            if let Some(col) = tr.col.take() {
                level.world.remove_collider(col);
            }
            let Some(dx) = offset(tr.t) else { continue };
            let p = tracks[i] + Vec3::X * dx;
            tr.col = Some(level.world.add_collider(p.x, p.z, LEN, HEIGHT, WIDTH, p.y + 0.3));
            // pitido al entrar en la estación
            if before.is_none() && feet.first().is_some_and(|f| p.distance(*f) < 90.0) {
                self.events.push(GameEvent::Sound(crate::game::Snd::Train));
            }
            // si te agarra en las vías
            let moving = before.is_some_and(|b| (b - dx).abs() > 0.01);
            for (k, f) in feet.iter().enumerate() {
                let rel = *f - p;
                if moving && rel.x.abs() < LEN / 2.0 + 0.5 && rel.z.abs() < WIDTH / 2.0 + 0.4 && rel.y < HEIGHT {
                    self.events.push(GameEvent::Hurt { dmg: 200.0, from: Some(p), to: k as u8 });
                }
            }
        }
    }

    /// Estación en cuyo andén está el jugador con el tren parado y las puertas abiertas.
    pub fn train_boardable(&self, level: &Level, feet: Vec3) -> Option<usize> {
        let c = level.city.as_ref()?;
        c.stations.iter().enumerate().position(|(i, s)| {
            let near = (s.platform.x - feet.x).abs() < LEN / 2.0 - 1.0 && (s.platform.z - feet.z).abs() < 2.4 && (feet.y - s.platform.y).abs() < 0.6;
            near && self.trains.get(i).is_some_and(|t| doors(t.t) > 0.9)
        })
    }

    /// Viaje: el tren de la estación de origen sale y en el destino te espera uno con las
    /// puertas abiertas. Devuelve dónde aparecés (en el andén, frente a una puerta).
    pub fn ride_train(&mut self, level: &Level, from: usize, to: usize) -> Option<Vec3> {
        let c = level.city.as_ref()?;
        let st = c.stations.get(to)?;
        if let Some(t) = self.trains.get_mut(from) {
            t.t = ARRIVE + STOP;
        }
        if let Some(t) = self.trains.get_mut(to) {
            t.t = ARRIVE + 1.4;
        }
        Some(st.platform + Vec3::new(0.0, 0.0, 0.2))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn un_remoto_toma_el_subte() {
        let mut l = crate::maps::build_level(crate::maps::MapId::Ciudad);
        let mut g = Game::new(&mut l, 7);
        g.add_pawn(&l, 1, "Amigo".into());
        let (p0, p1) = {
            let c = l.city.as_ref().unwrap();
            (c.stations[0].platform, c.stations[1].platform)
        };
        g.pawns[0].state.pos = p0 + Vec3::new(0.0, crate::player::EYE, 0.2);
        // sin tren no viaja
        g.trains[0].t = 0.0;
        assert!(!g.pawn_ride(&l, 1, 1));
        // con las puertas abiertas, sí
        g.trains[0].t = ARRIVE + 5.0;
        assert!(g.pawn_ride(&l, 1, 1));
        assert!(g.pawns[0].state.pos.distance(p1) < 2.5, "llegó a la otra estación");
    }

    #[test]
    fn el_tren_llega_para_y_se_va() {
        assert!(offset(0.0).unwrap() < -90.0);
        assert_eq!(offset(ARRIVE + 1.0), Some(0.0));
        assert!(doors(ARRIVE + 5.0) > 0.99);
        assert!(doors(ARRIVE + 0.2) < 0.01);
        assert!(offset(ARRIVE + STOP + LEAVE - 0.01).unwrap() > 90.0);
        assert_eq!(offset(CYCLE - 1.0), None);
        assert!((next_in(CYCLE - 1.0) - (1.0 + ARRIVE)).abs() < 1e-4);
    }
}
