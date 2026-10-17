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
}

impl Vehicle {
    pub fn new(id: u32, pos: Vec3, yaw: f32, color: u8, world: &mut World) -> Self {
        let mut v = Self { id, pos, yaw, speed: 0.0, steer: 0.0, vy: 0.0, color, col: None, wheel: 0.0 };
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
