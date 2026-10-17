//! Vehículos manejables (nuevo, no estaba en la web): los autos abandonados del Distrito Sur se
//! pueden manejar. Física arcade tipo "bicicleta": acelerador, freno, marcha atrás y dirección
//! que se endurece con la velocidad. Choca contra el mundo con tres círculos a lo largo del auto,
//! sube cordones y rampas (escalones de hasta 0,36 m) y cae con gravedad.

use crate::world::{ColliderId, World};
use glam::{Vec2, Vec3};

pub const LENGTH: f32 = 4.3;
pub const WIDTH: f32 = 1.9;
const WHEELBASE: f32 = 2.6;
const MAX_SPEED: f32 = 30.0;
const MAX_REVERSE: f32 = 9.0;
const ACCEL: f32 = 13.0;
const BRAKE: f32 = 28.0;
const MAX_STEER: f32 = 0.55;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DriveInput {
    /// Acelerador (+1) / marcha atrás (−1).
    pub throttle: f32,
    /// Derecha (+1) / izquierda (−1).
    pub steer: f32,
    pub brake: bool,
}

#[derive(Clone, Debug)]
pub struct Vehicle {
    pub id: u32,
    /// Centro del auto a nivel del piso.
    pub pos: Vec3,
    /// Rumbo: adelante es (sin yaw, cos yaw), como los soldados.
    pub yaw: f32,
    /// Velocidad hacia adelante (negativa en marcha atrás).
    pub speed: f32,
    pub steer: f32,
    vy: f32,
    pub color: u8,
    pub col: Option<ColliderId>,
    /// Vueltas de las ruedas, para el dibujo.
    pub wheel: f32,
    pub hp: f32,
    /// Destruido: queda como chatarra y no se puede manejar.
    pub wrecked: bool,
    /// Conducido por enemigos.
    pub ai: Option<CarAi>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AiMode {
    /// Recorre su ruta por las calles.
    Patrol,
    /// Va hacia el jugador.
    Chase,
    /// Frenó y bajó la tripulación.
    Parked,
}

/// Piloto enemigo: ruta, modo, tripulación a bordo y lógica para destrabarse.
#[derive(Clone, Debug)]
pub struct CarAi {
    pub route: Vec<Vec2>,
    pub idx: usize,
    pub mode: AiMode,
    /// Soldados que bajan al alcanzarte.
    pub crew: u32,
    stuck_t: f32,
    reverse_t: f32,
    pub see_t: f32,
    /// Convoy: al llegar al final de la ruta desaparece (y el evento se pierde).
    pub one_way: bool,
    pub event: Option<u32>,
    /// Tránsito civil: nunca persigue, frena si tiene algo adelante y, si le disparan, escapa.
    pub civil: bool,
    pub scared: f32,
}

impl CarAi {
    pub fn new(route: Vec<Vec2>, crew: u32, one_way: bool, event: Option<u32>) -> Self {
        Self { route, idx: 0, mode: AiMode::Patrol, crew, stuck_t: 0.0, reverse_t: 0.0, see_t: 0.0, one_way, event, civil: false, scared: 0.0 }
    }
}

/// Lo que decidió el piloto en este paso.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AiOut {
    None,
    /// Llegó cerca del jugador: bajar a la tripulación acá.
    Dismount,
    /// Terminó la ruta de ida (convoy que escapó).
    Arrived,
}

impl Vehicle {
    pub fn new(id: u32, pos: Vec3, yaw: f32, color: u8, world: &mut World) -> Self {
        let mut v = Self { id, pos, yaw, speed: 0.0, steer: 0.0, vy: 0.0, color, col: None, wheel: 0.0, hp: 60.0, wrecked: false, ai: None };
        v.place_collider(world);
        v
    }

    pub fn forward(&self) -> Vec3 {
        Vec3::new(self.yaw.sin(), 0.0, self.yaw.cos())
    }

    pub fn right(&self) -> Vec3 {
        // derecha mirando hacia adelante (con −Z al norte, girar a la derecha baja el yaw)
        Vec3::new(-self.yaw.cos(), 0.0, self.yaw.sin())
    }

    /// Colisionador en caja alineada a los ejes que envuelve al auto girado.
    fn place_collider(&mut self, world: &mut World) {
        if let Some(c) = self.col.take() {
            world.remove_collider(c);
        }
        let (s, c) = (self.yaw.sin().abs(), self.yaw.cos().abs());
        let (w, d) = (WIDTH * c + LENGTH * s, LENGTH * c + WIDTH * s);
        self.col = Some(world.add_collider(self.pos.x, self.pos.z, w * 0.92, 1.6, d * 0.92, self.pos.y + 0.1));
    }

    /// Centros de los tres círculos de colisión.
    fn circles(&self) -> [Vec3; 3] {
        let f = self.forward();
        [self.pos - f * 1.4, self.pos, self.pos + f * 1.4]
    }

    /// Un paso de manejo. Devuelve la fuerza del choque (velocidad perdida) si chocó.
    pub fn drive(&mut self, input: &DriveInput, world: &mut World, dt: f32) -> f32 {
        // el propio colisionador no cuenta mientras se mueve
        if let Some(c) = self.col.take() {
            world.remove_collider(c);
        }
        let thr = input.throttle.clamp(-1.0, 1.0);
        if input.brake {
            self.speed -= self.speed.signum() * (BRAKE * dt).min(self.speed.abs());
        } else if thr > 0.0 {
            // acelerar (si iba para atrás, primero frena)
            self.speed += if self.speed < 0.0 { BRAKE } else { ACCEL * (1.0 - self.speed / MAX_SPEED).max(0.15) } * thr * dt;
        } else if thr < 0.0 {
            self.speed += if self.speed > 0.0 { BRAKE } else { ACCEL * 0.7 } * thr * dt;
        } else {
            // rozamiento y resistencia del aire
            let drag = 2.5 + self.speed.abs() * 0.05;
            self.speed -= self.speed.signum() * (drag * dt).min(self.speed.abs());
        }
        self.speed = self.speed.clamp(-MAX_REVERSE, MAX_SPEED);
        let target = input.steer.clamp(-1.0, 1.0) * MAX_STEER / (1.0 + self.speed.abs() * 0.05);
        self.steer += (target - self.steer) * (dt * 6.0).min(1.0);
        self.yaw -= self.speed * self.steer.tan() / WHEELBASE * dt;
        self.pos += self.forward() * self.speed * dt;
        self.wheel += self.speed * dt / 0.36;
        // choques: cada círculo se empuja fuera de las cajas y el auto se lleva el promedio
        let mut push = Vec2::ZERO;
        for c in self.circles() {
            let mut q = c;
            world.collide_xz(&mut q, 1.0, self.pos.y + 0.05, 1.4);
            push += Vec2::new(q.x - c.x, q.z - c.z);
        }
        let mut impact = 0.0;
        if push.length() > 0.01 {
            self.pos.x += push.x;
            self.pos.z += push.y;
            impact = self.speed.abs();
            // rebote: pierde casi toda la velocidad
            self.speed *= -0.2;
        }
        // piso: el más alto bajo los tres círculos (sube cordones y escalones de rampa)
        let g = self.circles().iter().map(|c| world.ground_at(c.x, c.z, self.pos.y)).fold(0.0f32, f32::max);
        if self.pos.y > g + 0.01 {
            self.vy -= 24.0 * dt;
            self.pos.y = (self.pos.y + self.vy * dt).max(g);
            if self.pos.y <= g {
                self.vy = 0.0;
            }
        } else {
            self.pos.y = g;
            self.vy = 0.0;
        }
        let lim = world.lim() - 2.0;
        self.pos.x = self.pos.x.clamp(-lim, lim);
        self.pos.z = self.pos.z.clamp(-lim, lim);
        self.place_collider(world);
        impact
    }

    /// Piloto enemigo: elige acelerador y dirección según la ruta o el jugador, maneja y se
    /// destraba yendo marcha atrás si se queda pegado contra algo.
    pub fn ai_step(&mut self, world: &mut World, player: Vec3, sees_player: bool, dt: f32) -> AiOut {
        let Some(mut ai) = self.ai.take() else { return AiOut::None };
        let mut out = AiOut::None;
        let here = Vec2::new(self.pos.x, self.pos.z);
        let p2 = Vec2::new(player.x, player.z);
        let sees_player = sees_player && !ai.civil;
        ai.scared = (ai.scared - dt).max(0.0);
        if sees_player {
            ai.see_t = 0.0;
            if ai.mode == AiMode::Patrol && !ai.one_way {
                ai.mode = AiMode::Chase;
            }
        } else {
            ai.see_t += dt;
            if ai.mode == AiMode::Chase && ai.see_t > 20.0 {
                ai.mode = AiMode::Patrol;
            }
        }
        let (target, max_speed) = match ai.mode {
            AiMode::Patrol => {
                let w = ai.route[ai.idx % ai.route.len()];
                if here.distance(w) < 6.0 {
                    ai.idx += 1;
                    if ai.one_way && ai.idx >= ai.route.len() {
                        out = AiOut::Arrived;
                    }
                }
                (ai.route[ai.idx % ai.route.len()], if ai.civil { if ai.scared > 0.0 { 22.0 } else { 10.0 } } else { 13.0 })
            }
            AiMode::Chase => {
                if here.distance(p2) < 16.0 && ai.crew > 0 {
                    ai.mode = AiMode::Parked;
                    out = AiOut::Dismount;
                }
                (p2, 22.0)
            }
            AiMode::Parked => (here, 0.0),
        };
        let mut input = DriveInput::default();
        // el tránsito frena si tiene al jugador adelante
        let ahead = {
            let rel = player - self.pos;
            let f = self.forward();
            let along = rel.dot(f);
            ai.civil && ai.scared <= 0.0 && along > 0.0 && along < 9.0 && rel.dot(self.right()).abs() < 2.2 && (player.y - 1.6 - self.pos.y).abs() < 2.0
        };
        if ahead {
            input.brake = true;
        } else if ai.mode == AiMode::Parked {
            input.brake = true;
        } else if ai.reverse_t > 0.0 {
            ai.reverse_t -= dt;
            input.throttle = -1.0;
            input.steer = 1.0;
        } else {
            let to = target - here;
            let want = to.x.atan2(to.y);
            let mut diff = (want - self.yaw).rem_euclid(std::f32::consts::TAU);
            if diff > std::f32::consts::PI {
                diff -= std::f32::consts::TAU;
            }
            // girar a la derecha baja el yaw: dirección opuesta a la diferencia
            input.steer = (-diff * 2.0).clamp(-1.0, 1.0);
            let slow = diff.abs() > 0.9 || self.speed > max_speed;
            input.throttle = if slow { 0.25 } else { 1.0 };
            if self.speed > max_speed + 2.0 {
                input.throttle = 0.0;
            }
        }
        let impact = self.drive(&input, world, dt);
        // ¿trabado? (choca o casi no avanza queriendo avanzar)
        if ai.mode != AiMode::Parked && ai.reverse_t <= 0.0 && !ahead {
            if impact > 0.0 || (self.speed.abs() < 1.0 && input.throttle > 0.0) {
                ai.stuck_t += dt;
            } else {
                ai.stuck_t = (ai.stuck_t - dt).max(0.0);
            }
            if ai.stuck_t > 1.2 {
                ai.stuck_t = 0.0;
                ai.reverse_t = 1.0;
            }
        }
        self.ai = Some(ai);
        out
    }

    /// Lugar libre para bajarse: a la izquierda y, si no hay lugar, a la derecha.
    pub fn exit_spot(&self, world: &World) -> Vec3 {
        for side in [-1.0, 1.0] {
            let p = self.pos + self.right() * side * 2.0;
            if world.box_at(p + Vec3::Y * 1.0).is_none() {
                return p;
            }
        }
        self.pos + Vec3::Y * 2.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mat::Mat;

    #[test]
    fn el_piloto_sigue_su_ruta() {
        let mut w = World::new(200.0);
        let mut v = Vehicle::new(1, Vec3::ZERO, 0.0, 0, &mut w);
        v.ai = Some(CarAi::new(vec![Vec2::new(0.0, 60.0), Vec2::new(60.0, 60.0)], 2, true, None));
        let mut arrived = false;
        for _ in 0..(60 * 30) {
            if v.ai_step(&mut w, Vec3::splat(1000.0), false, crate::TICK) == AiOut::Arrived {
                arrived = true;
                break;
            }
        }
        assert!(arrived, "no llegó: {:?}", v.pos);
    }

    #[test]
    fn acelera_frena_y_dobla() {
        let mut w = World::new(200.0);
        let mut v = Vehicle::new(1, Vec3::ZERO, 0.0, 0, &mut w);
        for _ in 0..180 {
            v.drive(&DriveInput { throttle: 1.0, ..Default::default() }, &mut w, crate::TICK);
        }
        assert!(v.speed > 20.0, "velocidad {}", v.speed);
        assert!(v.pos.z > 20.0, "avanza hacia +z: {}", v.pos.z);
        let yaw0 = v.yaw;
        for _ in 0..60 {
            v.drive(&DriveInput { throttle: 1.0, steer: 1.0, ..Default::default() }, &mut w, crate::TICK);
        }
        assert!(v.yaw < yaw0, "girar a la derecha baja el yaw");
        for _ in 0..120 {
            v.drive(&DriveInput { brake: true, ..Default::default() }, &mut w, crate::TICK);
        }
        assert!(v.speed.abs() < 0.01);
    }

    #[test]
    fn choca_contra_una_pared() {
        let mut w = World::new(200.0);
        w.add_box(0.0, 30.0, 20.0, 4.0, 1.0, Mat::Wall);
        let mut v = Vehicle::new(1, Vec3::ZERO, 0.0, 0, &mut w);
        let mut hit = 0.0f32;
        for _ in 0..300 {
            hit = hit.max(v.drive(&DriveInput { throttle: 1.0, ..Default::default() }, &mut w, crate::TICK));
        }
        assert!(hit > 5.0);
        assert!(v.pos.z < 30.0 - 0.5 - 1.4, "atravesó la pared: z = {}", v.pos.z);
    }

    #[test]
    fn sube_la_vereda() {
        let mut w = World::new(200.0);
        w.add_box(0.0, 20.0, 40.0, 0.16, 20.0, Mat::Sidewalk);
        let mut v = Vehicle::new(1, Vec3::ZERO, 0.0, 0, &mut w);
        for _ in 0..240 {
            v.drive(&DriveInput { throttle: 1.0, ..Default::default() }, &mut w, crate::TICK);
            if v.pos.z > 16.0 {
                break;
            }
        }
        assert!((v.pos.y - 0.16).abs() < 0.01, "altura {}", v.pos.y);
    }
}
