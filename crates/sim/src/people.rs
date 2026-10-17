//! Vida en la ciudad (nuevo): peatones que caminan por las veredas de su manzana y huyen de los
//! tiros, y tránsito civil que circula por las calles. Herir civiles cuesta créditos.

use crate::city::{BC_ALL, CORE_A};
use crate::game::{Game, GameEvent, Snd};
use crate::maps::Level;
use crate::vehicle::{CarAi, Vehicle};
use glam::{Vec2, Vec3};

#[derive(Clone, Debug)]
pub struct Person {
    pub id: u32,
    pub pos: Vec3,
    pub yaw: f32,
    /// Centro de la manzana que rodea.
    home: Vec2,
    /// Esquina a la que va (0..4) y sentido de la vuelta.
    corner: usize,
    dir: i32,
    pub speed: f32,
    /// Huyendo: tiempo que le queda y de dónde se aleja.
    pub flee: f32,
    from: Vec2,
    pub color: u8,
    pub alive: bool,
    pub phase: f32,
    /// Para la animación: si se está moviendo.
    pub moving: bool,
}

/// Esquinas de la vereda de una manzana (a 14,8 m del centro).
fn corner(home: Vec2, i: usize) -> Vec2 {
    let r = 14.8;
    home + [Vec2::new(-r, -r), Vec2::new(r, -r), Vec2::new(r, r), Vec2::new(-r, r)][i % 4]
}

impl Game {
    /// Peatones repartidos en manzanas sin puestos enemigos.
    pub(crate) fn spawn_people(&mut self, level: &Level, n: usize) {
        let Some(c) = level.city.as_ref() else { return };
        let busy: Vec<(i32, i32)> = c.outposts.iter().map(|o| (o.cx as i32, o.cz as i32)).collect();
        let mut tries = 0;
        while self.people.len() < n && tries < n * 10 {
            tries += 1;
            let home = Vec2::new(BC_ALL[self.rng.idx(BC_ALL.len())], BC_ALL[self.rng.idx(BC_ALL.len())]);
            if busy.contains(&(home.x as i32, home.y as i32)) {
                continue;
            }
            let k = self.rng.idx(4);
            let start = corner(home, k).lerp(corner(home, k + 1), self.rng.f());
            let id = self.next_id();
            let dir = if self.rng.next() < 0.5 { 1 } else { -1 };
            let p = Person {
                id,
                pos: Vec3::new(start.x, 0.16, start.y),
                yaw: 0.0,
                home,
                corner: if dir > 0 { (k + 1) % 4 } else { k },
                dir,
                speed: 1.1 + self.rng.f() * 0.6,
                flee: 0.0,
                from: Vec2::ZERO,
                color: self.rng.idx(6) as u8,
                alive: true,
                phase: self.rng.f() * 6.0,
                moving: true,
            };
            self.people.push(p);
        }
    }

    /// Tránsito civil: autos que dan vueltas por las calles del anillo exterior.
    pub(crate) fn spawn_traffic(&mut self, level: &mut Level, n: usize) {
        let st: Vec<f32> = level.city.as_ref().map(|c| c.streets.clone()).unwrap_or_default();
        if st.len() < 3 {
            return;
        }
        for _ in 0..n {
            let i = self.rng.idx(st.len() - 2);
            let j = i + 1 + self.rng.idx(2).min(st.len() - 2 - i);
            let k = self.rng.idx(st.len() - 2);
            let l = k + 1 + self.rng.idx(2).min(st.len() - 2 - k);
            let lane = 4.2;
            // vuelta en sentido horario, por el carril de la derecha
            let route = vec![Vec2::new(st[i] + lane, st[k] + lane), Vec2::new(st[j] - lane, st[k] + lane), Vec2::new(st[j] - lane, st[l] - lane), Vec2::new(st[i] + lane, st[l] - lane)];
            let s = route[0];
            let id = self.next_id();
            let mut v = Vehicle::new(id, Vec3::new(s.x, 0.0, s.y), std::f32::consts::FRAC_PI_2, self.rng.idx(6) as u8, &mut level.world);
            v.hp = 25.0;
            let mut ai = CarAi::new(route, 0, false, None);
            ai.civil = true;
            v.ai = Some(ai);
            self.vehicles.push(v);
        }
    }

    /// Caminar, doblar en las esquinas y huir del peligro.
    pub(crate) fn step_people(&mut self, level: &Level, player: Vec3, dt: f32) {
        let _ = CORE_A;
        for p in &mut self.people {
            if !p.alive {
                continue;
            }
            p.moving = true;
            let here = Vec2::new(p.pos.x, p.pos.z);
            let target = if p.flee > 0.0 {
                p.flee -= dt;
                here + (here - p.from).normalize_or_zero() * 10.0
            } else {
                let c = corner(p.home, p.corner);
                if here.distance(c) < 0.6 {
                    p.corner = ((p.corner as i32 + p.dir).rem_euclid(4)) as usize;
                }
                corner(p.home, p.corner)
            };
            let to = target - here;
            let d = to.length();
            let speed = if p.flee > 0.0 { 4.5 } else { p.speed };
            // se corren si el jugador les camina encima
            let me = Vec2::new(player.x, player.z);
            if here.distance(me) < 1.2 && p.flee <= 0.0 {
                p.yaw = (here - me).x.atan2((here - me).y);
                continue;
            }
            if d > 0.05 {
                let step = to / d * (speed * dt).min(d);
                let mut np = p.pos + Vec3::new(step.x, 0.0, step.y);
                level.world.collide_xz(&mut np, 0.3, p.pos.y, 1.7);
                np.y = level.world.ground_at(np.x, np.z, p.pos.y + 0.3);
                p.pos = np;
                p.yaw = to.x.atan2(to.y);
            } else {
                p.moving = false;
            }
        }
    }

    /// Ruido fuerte (tiros, explosiones): los civiles cercanos salen corriendo.
    pub(crate) fn scare_people(&mut self, pos: Vec3, radius: f32) {
        for p in &mut self.people {
            if p.alive && p.pos.distance(pos) < radius {
                p.flee = 8.0;
                p.from = Vec2::new(pos.x, pos.z);
            }
        }
    }

    /// Civil herido (bala, explosión o atropello).
    pub(crate) fn hurt_person(&mut self, i: usize) {
        let p = &mut self.people[i];
        if !p.alive {
            return;
        }
        p.alive = false;
        let pos = p.pos + Vec3::Y;
        self.events.push(GameEvent::Burst { pos, color: 0xff3d5a, n: 10, speed: 4.0, life: 0.5 });
        self.credits = self.credits.saturating_sub(100);
        self.events.push(GameEvent::Banner { title: "CIVIL HERIDO".into(), sub: "−100 créditos · cuidado con la gente".into() });
        self.events.push(GameEvent::Sound(Snd::Scream));
    }

    /// Distancia del rayo al primer civil que toca (caja de 0,5 × 1,8 m).
    pub(crate) fn ray_people(&self, o: Vec3, d: Vec3, far: f32) -> Option<(usize, f32)> {
        let mut best: Option<(usize, f32)> = None;
        for (i, p) in self.people.iter().enumerate() {
            if !p.alive || (p.pos - o).length() > far + 2.0 {
                continue;
            }
            let c = p.pos + Vec3::Y * 0.9;
            if let Some(t) = crate::world::ray_aabb(o, d, &crate::world::Aabb { min: c - Vec3::new(0.25, 0.9, 0.25), max: c + Vec3::new(0.25, 0.9, 0.25) }, far) {
                if best.is_none_or(|b| t < b.1) {
                    best = Some((i, t));
                }
            }
        }
        best
    }
}

#[cfg(test)]
mod tests {
    use crate::enemies::Target;
    use crate::maps::{MapId, build_level};
    use glam::Vec3;

    #[test]
    fn hay_gente_y_transito_y_se_mueven() {
        let mut l = build_level(MapId::Ciudad);
        let mut g = crate::game::Game::new(&mut l, 3);
        assert!(g.people.len() >= 60);
        let civ: Vec<usize> = (0..g.vehicles.len()).filter(|&i| g.vehicles[i].ai.as_ref().is_some_and(|a| a.civil)).collect();
        assert!(civ.len() >= 10);
        let p0: Vec<Vec3> = g.people.iter().map(|p| p.pos).collect();
        let c0: Vec<Vec3> = civ.iter().map(|&i| g.vehicles[i].pos).collect();
        let t = Target { pos: Vec3::new(-132.0, 1.76, 150.0), body: Vec3::new(-132.0, 1.26, 150.0), vel: Vec3::ZERO, feet: 0.16, crouch_t: 0.0, evasive: false, light: false };
        for _ in 0..(60 * 5) {
            g.step(&mut l, &t, true, crate::TICK);
            g.events.clear();
        }
        let moved = g.people.iter().zip(&p0).filter(|(p, q)| p.pos.distance(**q) > 2.0).count();
        assert!(moved > g.people.len() / 2, "solo caminaron {moved}");
        let drove = civ.iter().zip(&c0).filter(|(i, q)| g.vehicles[**i].pos.distance(**q) > 10.0).count();
        assert!(drove > civ.len() / 2, "solo circularon {drove}");
    }
}
