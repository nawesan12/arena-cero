//! Movimiento del jugador (puerto de la parte de movimiento de `18-ciclo-de-juego.js` y del
//! gancho de `21-gancho-equipo-de-maniobras.js`). Es una función pura de (estado, entrada, mundo):
//! el servidor la corre con autoridad y el cliente la repite para predecir.

use crate::world::{R, World};
use glam::Vec3;

pub const EYE: f32 = 1.6;
pub const CROUCH_DROP: f32 = 0.65;
pub const HOOK_RANGE: f32 = 55.0;
pub const BASE_FOV: f32 = 75.0;
pub const AIM_FOV: f32 = 48.0;

/// Entrada de un paso. Los ángulos de vista viajan con la entrada (así los manda la red).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PlayerInput {
    /// Adelante (+1) / atrás (−1).
    pub forward: f32,
    /// Derecha (+1) / izquierda (−1).
    pub right: f32,
    pub yaw: f32,
    pub pitch: f32,
    pub sprint: bool,
    pub crouch: bool,
    pub aim: bool,
    /// Pulsaciones (flancos): valen solo en el paso en que llegan.
    pub jump: bool,
    pub dash: bool,
    pub hook: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hook {
    pub pt: Vec3,
    pub t: f32,
    /// Largo de la cuerda: te balanceás alrededor del punto sin alejarte más que esto.
    pub len: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlayerEvent {
    Slide,
    Dash,
    Jump,
    DoubleJump,
    HookFired,
    HookMiss,
    HookBoost,
    /// Daño recibido por el entorno (lava).
    Burn(u8),
    Land,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PlayerState {
    /// Posición de los ojos (de pie).
    pub pos: Vec3,
    pub vel: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub on_ground: bool,
    pub jumps: u32,
    pub max_jumps: u32,
    pub crouch_t: f32,
    pub was_crouch: bool,
    pub slide: f32,
    pub slide_cd: f32,
    pub aim_t: f32,
    /// Mientras recarga o cambia de arma no se puede apuntar (lo setea el sistema de armas).
    pub aim_blocked: bool,
    pub dash_cd: f32,
    pub dash_t: f32,
    pub dash_cd_mul: f32,
    pub speed_mul: f32,
    pub hook: Option<Hook>,
    pub hook_cd: f32,
    pub zone_acc: f32,
    /// Sacudón de cámara (0..1), lo consume el cliente.
    pub shake: f32,
}

impl PlayerState {
    pub fn new(x: f32, z: f32) -> Self {
        Self {
            pos: Vec3::new(x, EYE, z),
            vel: Vec3::ZERO,
            yaw: x.atan2(z),
            pitch: 0.0,
            on_ground: true,
            jumps: 0,
            max_jumps: 2,
            crouch_t: 0.0,
            was_crouch: false,
            slide: 0.0,
            slide_cd: 0.0,
            aim_t: 0.0,
            aim_blocked: false,
            dash_cd: 0.0,
            dash_t: 0.0,
            dash_cd_mul: 1.0,
            speed_mul: 1.0,
            hook: None,
            hook_cd: 0.0,
            zone_acc: 0.0,
            shake: 0.0,
        }
    }

    pub fn feet(&self) -> f32 {
        self.pos.y - EYE
    }

    /// Alto actual del cuerpo (se achica al agacharse).
    pub fn height(&self) -> f32 {
        1.8 - self.crouch_t * 0.7
    }

    /// Posición de la cámara.
    pub fn eye(&self) -> Vec3 {
        Vec3::new(self.pos.x, self.pos.y - self.crouch_t * CROUCH_DROP, self.pos.z)
    }

    /// Centro del cuerpo: a dónde apuntan los enemigos.
    pub fn body(&self) -> Vec3 {
        Vec3::new(self.pos.x, self.feet() + 1.1 - self.crouch_t * 0.5, self.pos.z)
    }

    pub fn look_dir(&self) -> Vec3 {
        look_dir(self.yaw, self.pitch)
    }

    pub fn horizontal_speed(&self) -> f32 {
        self.vel.x.hypot(self.vel.z)
    }

    /// FOV vertical en grados que pide el estado (el cliente lo suaviza).
    pub fn target_fov(&self, rail_scope: bool) -> f32 {
        let aim_fov = if rail_scope { AIM_FOV - 24.0 } else { AIM_FOV };
        // con velocidad (balanceo, caída, dash) se abre la vista: sensación de rapidez
        let speed = ((self.vel.length() - 11.0) * 0.55).clamp(0.0, 14.0);
        BASE_FOV + (aim_fov - BASE_FOV) * self.aim_t + if self.slide > 0.0 { 6.0 } else { 0.0 } + if self.dash_t > 0.0 { 10.0 } else { 0.0 } + speed * (1.0 - self.aim_t)
    }

    /// De dónde sale la cuerda (cintura del jugador).
    fn rope_origin(&self) -> Vec3 {
        self.pos - Vec3::Y * 0.5
    }

    pub fn release_hook(&mut self, boost: bool, ev: &mut Vec<PlayerEvent>) {
        if self.hook.take().is_none() {
            return;
        }
        self.hook_cd = 0.2;
        if boost {
            // salir despedido: se conserva toda la velocidad del balanceo y se suma un impulso
            // hacia arriba y hacia donde mirás
            let f = look_dir(self.yaw, 0.0);
            self.vel.y = self.vel.y.max(0.0) + 7.5;
            self.vel += f * 4.0;
            ev.push(PlayerEvent::HookBoost);
        }
    }

    fn fire_hook(&mut self, world: &World, ev: &mut Vec<PlayerEvent>) {
        if self.hook.is_some() {
            self.release_hook(false, ev);
            return;
        }
        if self.hook_cd > 0.0 {
            return;
        }
        self.hook_cd = 0.3;
        match hook_cast(world, self.eye(), self.look_dir()) {
            Some(pt) => {
                // la cuerda arranca un poco más corta que la distancia: el primer tirón te despega
                let len = (pt - self.rope_origin()).length() * 0.9;
                self.hook = Some(Hook { pt, t: 0.0, len });
                self.jumps = self.jumps.min(1);
                ev.push(PlayerEvent::HookFired);
            }
            None => ev.push(PlayerEvent::HookMiss),
        }
    }

    /// Gancho como péndulo: Shift recoge cuerda (y tira), C da cuerda, W/A/S/D dan envión
    /// perpendicular a la cuerda. La restricción de largo se aplica después de la gravedad.
    fn update_hook(&mut self, dt: f32, wish: Vec3, reel_in: bool, reel_out: bool, ev: &mut Vec<PlayerEvent>) {
        self.hook_cd -= dt;
        let origin = self.rope_origin();
        let Some(hk) = self.hook.as_mut() else { return };
        hk.t += dt;
        if hk.t < 0.08 {
            return; // el cable todavía viaja
        }
        let to = hk.pt - origin;
        let d = to.length().max(0.001);
        let r = to / d;
        if d < 1.8 || hk.t > 8.0 {
            self.release_hook(false, ev);
            return;
        }
        if reel_in {
            hk.len = (hk.len - 16.0 * dt).max(2.0);
            // recoger también tira: llegás rápido a una cornisa
            self.vel += r * 26.0 * dt;
        }
        if reel_out {
            hk.len = (hk.len + 9.0 * dt).min(super_len());
        }
        // envión: la parte del input perpendicular a la cuerda
        let tang = wish - r * wish.dot(r);
        self.vel += tang * 15.0 * dt;
        self.on_ground = false;
        self.slide = 0.0;
    }

    /// Mantiene el largo de la cuerda: si te pasás, te devuelve y saca la velocidad hacia afuera.
    fn rope_constraint(&mut self, dt: f32) {
        let origin = self.rope_origin();
        let Some(hk) = self.hook.as_ref() else { return };
        if hk.t < 0.08 {
            return;
        }
        let to = hk.pt - origin;
        let d = to.length().max(0.001);
        let r = to / d;
        if d > hk.len {
            let out = -self.vel.dot(r);
            if out > 0.0 {
                self.vel += r * out;
            }
            // corrección suave del exceso (sin tironear de golpe)
            self.vel += r * (d - hk.len) * 12.0 * dt;
        }
        let vs = self.vel.length();
        if vs > 45.0 {
            self.vel *= 45.0 / vs;
        }
    }

    /// Un paso de simulación.
    pub fn step(&mut self, input: &PlayerInput, world: &World, dt: f32, ev: &mut Vec<PlayerEvent>) {
        self.yaw = input.yaw;
        self.pitch = input.pitch.clamp(-1.5, 1.5);

        /* agacharse / deslizarse */
        let want_crouch = input.crouch;
        let hs0 = self.horizontal_speed();
        if want_crouch && !self.was_crouch && self.on_ground && hs0 > 8.0 && self.slide_cd <= 0.0 {
            self.slide = 0.7;
            self.slide_cd = 1.0;
            let k = 14.0 / hs0;
            self.vel.x *= k;
            self.vel.z *= k;
            ev.push(PlayerEvent::Slide);
        }
        self.was_crouch = want_crouch;
        self.slide_cd -= dt;
        if self.slide > 0.0 {
            self.slide -= dt;
        }
        let ct = if want_crouch || self.slide > 0.0 { 1.0 } else { 0.0 };
        self.crouch_t += (ct - self.crouch_t) * (1.0 - (-14.0 * dt).exp());

        /* apuntado */
        let want_aim = input.aim && !self.aim_blocked && self.slide <= 0.0;
        self.aim_t += (if want_aim { 1.0 } else { 0.0 } - self.aim_t) * (1.0 - (-14.0 * dt).exp());
        if self.aim_t < 0.001 {
            self.aim_t = 0.0;
        }

        /* movimiento */
        let (f, r) = (input.forward, input.right);
        let (sy, cy) = self.yaw.sin_cos();
        let (mut wx, mut wz) = (-sy * f + cy * r, -cy * f - sy * r);
        let wl = wx.hypot(wz);
        if wl > 1.0 {
            wx /= wl;
            wz /= wl;
        }
        let sprint = input.sprint && self.aim_t < 0.3 && self.crouch_t < 0.3;
        let spd = if sprint { 10.0 } else { 7.0 } * (1.0 - self.aim_t * 0.45) * (1.0 - self.crouch_t * 0.5) * self.speed_mul;
        self.dash_cd -= dt;
        if input.dash && self.dash_cd <= 0.0 {
            let (mut dx, mut dz) = (wx, wz);
            if dx.hypot(dz) < 0.1 {
                dx = -sy;
                dz = -cy;
            }
            let l = dx.hypot(dz);
            self.vel.x = dx / l * 24.0;
            self.vel.z = dz / l * 24.0;
            self.vel.y = self.vel.y.max(1.0);
            self.dash_t = 0.16;
            self.dash_cd = 3.0 * self.dash_cd_mul;
            self.slide = 0.0;
            self.shake = (self.shake + 0.2).min(1.0);
            ev.push(PlayerEvent::Dash);
        }
        if self.dash_t > 0.0 {
            self.dash_t -= dt;
            if self.dash_t <= 0.0 {
                self.vel.x *= 0.45;
                self.vel.z *= 0.45;
            }
        } else if self.slide > 0.0 && self.on_ground {
            let fr = 1.0 - (-1.6 * dt).exp();
            self.vel.x -= self.vel.x * fr;
            self.vel.z -= self.vel.z * fr;
        } else if self.on_ground && self.hook.is_none() {
            let k = 1.0 - (-12.0 * dt).exp();
            self.vel.x += (wx * spd - self.vel.x) * k;
            self.vel.z += (wz * spd - self.vel.z) * k;
        } else {
            // control aéreo: suma en la dirección del input sin frenar el impulso del gancho o el dash
            let along = self.vel.x * wx + self.vel.z * wz;
            if along < spd {
                self.vel.x += wx * spd * 2.4 * dt;
                self.vel.z += wz * spd * 2.4 * dt;
            }
            let dr = (-(if self.hook.is_some() { 0.1 } else { 0.6 }) * dt).exp();
            self.vel.x *= dr;
            self.vel.z *= dr;
        }
        if input.hook {
            self.fire_hook(world, ev);
        }
        let wish = Vec3::new(wx, 0.0, wz) + if input.forward > 0.0 && self.hook.is_some() { Vec3::Y * 0.25 } else { Vec3::ZERO };
        self.update_hook(dt, wish, input.sprint, input.crouch, ev);
        if input.jump {
            if self.hook.is_some() {
                self.release_hook(true, ev);
            } else if self.on_ground {
                self.vel.y = 8.2;
                self.on_ground = false;
                self.slide = 0.0;
                self.jumps = 1;
                ev.push(PlayerEvent::Jump);
            } else if self.jumps < self.max_jumps {
                // doble salto: impulso en el aire hacia donde apunta el input
                self.vel.y = 7.6;
                self.jumps += 1;
                if wl > 0.1 {
                    self.vel.x = wx * spd * 1.1;
                    self.vel.z = wz * spd * 1.1;
                }
                ev.push(PlayerEvent::DoubleJump);
            }
        }
        self.vel.y -= 24.0 * dt;
        self.rope_constraint(dt);

        let mut feet = self.feet();
        let n_sub = ((self.horizontal_speed() * dt / 0.3).ceil() as i32).clamp(1, 8);
        let h = self.height();
        for _ in 0..n_sub {
            self.pos.x += self.vel.x * dt / n_sub as f32;
            self.pos.z += self.vel.z * dt / n_sub as f32;
            world.collide_xz(&mut self.pos, R, feet, h);
        }
        let feet0 = feet;
        feet += self.vel.y * dt;
        if self.vel.y > 0.0 {
            // techo: no atravesar losas al saltar o con el gancho
            if let Some(c) = world.ceiling_at(self.pos.x, self.pos.z, feet0 + h, feet + h) {
                feet = c - h - 0.01;
                self.vel.y = 0.0;
            }
        }
        let was_ground = self.on_ground;
        let g = world.ground_at(self.pos.x, self.pos.z, self.feet());
        if feet <= g {
            feet = g;
            if !was_ground && self.vel.y < -6.0 {
                ev.push(PlayerEvent::Land);
            }
            self.vel.y = 0.0;
            self.on_ground = true;
            self.jumps = 0;
        } else if feet > g + 0.05 {
            self.on_ground = false;
        }
        self.pos.y = feet + EYE;
        if feet < 0.2 {
            for zn in &world.zones {
                if (self.pos.x - zn.x).abs() < zn.w / 2.0 && (self.pos.z - zn.z).abs() < zn.d / 2.0 {
                    self.zone_acc += zn.dps * dt;
                    if self.zone_acc >= 4.0 {
                        self.zone_acc -= 4.0;
                        ev.push(PlayerEvent::Burn(4));
                    }
                }
            }
        }
        let lim = world.lim();
        self.pos.x = self.pos.x.clamp(-lim, lim);
        self.pos.z = self.pos.z.clamp(-lim, lim);
        self.shake = (self.shake - dt * 2.2).max(0.0);
    }
}

/// Cuerda máxima: lo que alcanza el gancho.
fn super_len() -> f32 {
    HOOK_RANGE
}

/// Dirección de la mirada (convención de cámara: −Z adelante, rotación YXZ).
pub fn look_dir(yaw: f32, pitch: f32) -> Vec3 {
    let (sy, cy) = yaw.sin_cos();
    let (sp, cp) = pitch.sin_cos();
    Vec3::new(-sy * cp, sp, -cy * cp)
}

/// Punto donde engancharía el gancho (solo mundo; los drones se suman con los enemigos).
pub fn hook_cast(world: &World, eye: Vec3, dir: Vec3) -> Option<Vec3> {
    let t = world.ray(eye, dir, HOOK_RANGE);
    (t < HOOK_RANGE).then(|| eye + dir * t)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TICK;
    use crate::maps::{MapId, build_level};
    use crate::mat::Mat;

    fn run(p: &mut PlayerState, w: &World, input: PlayerInput, secs: f32) -> Vec<PlayerEvent> {
        let mut ev = Vec::new();
        let n = (secs / TICK).round() as usize;
        for i in 0..n {
            let mut inp = input;
            if i > 0 {
                inp.jump = false;
                inp.dash = false;
                inp.hook = false;
            }
            p.step(&inp, w, TICK, &mut ev);
        }
        ev
    }

    #[test]
    fn camina_a_7_y_corre_a_10() {
        let w = World::new(200.0);
        let mut p = PlayerState::new(0.0, 0.0);
        let yaw = p.yaw;
        run(&mut p, &w, PlayerInput { forward: 1.0, yaw, ..Default::default() }, 1.0);
        assert!((p.horizontal_speed() - 7.0).abs() < 0.05, "{}", p.horizontal_speed());
        run(&mut p, &w, PlayerInput { forward: 1.0, sprint: true, yaw, ..Default::default() }, 1.0);
        assert!((p.horizontal_speed() - 10.0).abs() < 0.05);
    }

    #[test]
    fn salto_y_doble_salto() {
        let w = World::new(60.0);
        let mut p = PlayerState::new(0.0, 0.0);
        run(&mut p, &w, PlayerInput { jump: true, ..Default::default() }, 0.2);
        let h1 = p.feet();
        assert!(!p.on_ground && h1 > 1.0);
        run(&mut p, &w, PlayerInput { jump: true, ..Default::default() }, 0.2);
        assert_eq!(p.jumps, 2);
        // un tercer salto en el aire no hace nada
        let vy = p.vel.y;
        run(&mut p, &w, PlayerInput { jump: true, ..Default::default() }, TICK);
        assert!(p.vel.y < vy);
        run(&mut p, &w, PlayerInput::default(), 3.0);
        assert!(p.on_ground && p.feet().abs() < 1e-4);
    }

    #[test]
    fn sube_escalones_y_no_atraviesa_paredes() {
        let mut w = World::new(60.0);
        let top = crate::world::STEP * 6.0;
        // plataforma en z ∈ [−4, 0] y escalera que baja hacia −z desde su borde
        w.add_box(0.0, -2.0, 4.0, top, 4.0, Mat::Concrete);
        w.add_stairs(0.0, -4.0, 0.0, -1.0, 4.0, top, Mat::Concrete);
        w.add_box(10.0, 0.0, 1.0, 5.0, 20.0, Mat::Wall);
        let mut p = PlayerState::new(0.0, -12.0);
        // yaw π mira hacia +z
        run(&mut p, &w, PlayerInput { forward: 1.0, yaw: std::f32::consts::PI, ..Default::default() }, 1.6);
        assert!((p.feet() - top).abs() < 0.01, "subió a {} (z = {})", p.feet(), p.pos.z);
        let mut p = PlayerState::new(0.0, 0.0);
        // yaw −π/2 mira hacia +X
        run(&mut p, &w, PlayerInput { forward: 1.0, yaw: -std::f32::consts::FRAC_PI_2, ..Default::default() }, 3.0);
        assert!(p.pos.x < 9.5 - R + 0.01, "x = {}", p.pos.x);
    }

    #[test]
    fn dash_y_deslizamiento() {
        let w = World::new(200.0);
        let mut p = PlayerState::new(0.0, 0.0);
        let ev = run(&mut p, &w, PlayerInput { dash: true, ..Default::default() }, TICK);
        assert!(ev.contains(&PlayerEvent::Dash));
        assert!(p.horizontal_speed() > 20.0);
        let mut p = PlayerState::new(0.0, 0.0);
        let yaw = p.yaw;
        run(&mut p, &w, PlayerInput { forward: 1.0, sprint: true, yaw, ..Default::default() }, 1.0);
        let ev = run(&mut p, &w, PlayerInput { forward: 1.0, crouch: true, yaw, ..Default::default() }, TICK);
        assert!(ev.contains(&PlayerEvent::Slide));
    }

    #[test]
    fn el_gancho_balancea_como_pendulo() {
        // punto alto, el jugador cuelga y se le da envión hacia adelante: pasa por debajo y sube
        let mut w = World::new(200.0);
        w.add_block(0.0, 30.0, -20.0, 4.0, 1.0, 4.0, Mat::Concrete);
        let mut p = PlayerState::new(0.0, 0.0);
        p.pos = Vec3::new(0.0, 22.0, 0.0);
        let pt = Vec3::new(0.0, 30.0, -20.0);
        let len = (pt - p.rope_origin()).length();
        p.hook = Some(Hook { pt, t: 1.0, len });
        let mut ev = Vec::new();
        let mut max_z: f32 = -1e9;
        let mut min_d: f32 = 1e9;
        for _ in 0..(60 * 3) {
            p.step(&PlayerInput { yaw: 0.0, ..Default::default() }, &w, crate::TICK, &mut ev);
            if p.hook.is_none() {
                break;
            }
            max_z = max_z.max(-p.pos.z);
            min_d = min_d.min((pt - p.rope_origin()).length());
            // nunca se aleja más que el largo de la cuerda (con un margen por la corrección suave)
            assert!((pt - p.rope_origin()).length() < len + 0.6);
        }
        // cruzó por debajo del punto hacia el otro lado (z < −20)
        assert!(max_z > 20.0, "no balanceó: llegó a z = {}", -max_z);
    }

    #[test]
    fn gancho_tira_hacia_la_pared() {
        let mut w = World::new(60.0);
        w.add_box(0.0, -30.0, 20.0, 20.0, 2.0, Mat::Wall);
        let mut p = PlayerState::new(0.0, 0.0);
        // con Shift se recoge la cuerda y te acerca
        let ev = run(&mut p, &w, PlayerInput { hook: true, sprint: true, yaw: 0.0, pitch: 0.3, ..Default::default() }, 0.6);
        assert!(ev.contains(&PlayerEvent::HookFired));
        assert!(p.pos.z < -5.0, "z = {}", p.pos.z);
    }

    #[test]
    fn camina_por_la_ciudad_sin_caerse_del_mapa() {
        let l = build_level(MapId::Ciudad);
        let mut p = PlayerState::new(l.info.start[0], l.info.start[1]);
        let yaw = p.yaw;
        run(&mut p, &l.world, PlayerInput { forward: 1.0, sprint: true, yaw, ..Default::default() }, 10.0);
        assert!(p.feet() >= 0.0);
        assert!(p.pos.x.abs() <= l.world.lim() && p.pos.z.abs() <= l.world.lim());
    }
}
