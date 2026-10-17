//! Armas y granadas (puerto de `04-armas.js`, `15-granadas.js` y `16-disparo-del-jugador.js`).
//! La simulación decide cuándo y hacia dónde sale cada disparo; quién recibe el impacto lo
//! resuelve la capa de juego (enemigos), así esto sirve igual para el servidor.

use crate::player::{PlayerState, look_dir};
use crate::rng::Rng;
use crate::world::World;
use glam::Vec3;

pub struct WeaponDef {
    pub name: &'static str,
    pub short: &'static str,
    pub mag: u32,
    /// Segundos entre disparos.
    pub rate: f32,
    pub reload: f32,
    pub dmg: f32,
    pub pellets: u32,
    /// Dispersión quieto / en movimiento (en unidades de pantalla de la web).
    pub spread: [f32; 2],
    pub kick: f32,
    pub auto: bool,
    pub pierce: bool,
    pub launcher: bool,
    /// Ruido que hace al disparar (radio en m, modo ciudad).
    pub noise: f32,
}

pub const WEAPONS: [WeaponDef; 4] = [
    WeaponDef { name: "Rifle de pulso", short: "RIFLE", mag: 24, rate: 0.1, reload: 1.3, dmg: 1.0, pellets: 1, spread: [0.004, 0.012], kick: 0.009, auto: true, pierce: false, launcher: false, noise: 42.0 },
    WeaponDef { name: "Escopeta de dispersión", short: "ESCOPETA", mag: 6, rate: 0.7, reload: 1.7, dmg: 1.0, pellets: 9, spread: [0.075, 0.095], kick: 0.045, auto: false, pierce: false, launcher: false, noise: 55.0 },
    WeaponDef { name: "Riel de partículas", short: "RIEL", mag: 4, rate: 1.05, reload: 1.9, dmg: 6.0, pellets: 1, spread: [0.0, 0.006], kick: 0.03, auto: false, pierce: true, launcher: false, noise: 70.0 },
    WeaponDef { name: "Lanzagranadas", short: "LANZAG.", mag: 3, rate: 0.75, reload: 2.2, dmg: 0.0, pellets: 0, spread: [0.004, 0.02], kick: 0.05, auto: false, pierce: false, launcher: true, noise: 50.0 },
];

/// La web calculaba la dispersión en coordenadas de pantalla, que dependen del aspecto.
/// Acá se fija un aspecto de referencia para que todos los jugadores tengan la misma.
const REF_ASPECT: f32 = 1.6;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct WeaponInput {
    pub fire: bool,
    pub reload: bool,
    /// Elegir un arma exacta (teclas 1-4).
    pub select: Option<usize>,
    /// Rueda: +1 / −1.
    pub cycle: i32,
    pub nade: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Pellet {
    pub origin: Vec3,
    pub dir: Vec3,
}

#[derive(Clone, Debug, PartialEq)]
pub enum WeaponEvent {
    /// Disparo hitscan: un rayo por perdigón. `dmg` ya trae los multiplicadores.
    Fire { weapon: usize, pellets: Vec<Pellet>, dmg: f32, pierce: bool },
    Launch { weapon: usize },
    Toss,
    /// Retroceso a sumar a la vista (pitch, yaw).
    Kick { pitch: f32, yaw: f32 },
    Empty,
    Reload,
    Swap(usize),
    Locked,
    Noise { pos: Vec3, radius: f32 },
    Explosion(Explosion),
    Beep,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Explosion {
    pub pos: Vec3,
    pub radius: f32,
    /// Daño máximo a enemigos (en el centro).
    pub max_dmg: f32,
    /// Daño máximo al jugador si está dentro del radio.
    pub self_dmg: f32,
    pub big: bool,
    pub enemy: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Nade {
    pub pos: Vec3,
    pub vel: Vec3,
    pub fuse: f32,
    pub beep: f32,
    /// Proyectil del lanzagranadas: explota al tocar algo.
    pub impact: bool,
    pub gravity: f32,
    pub dmg: f32,
    /// Granada de un soldado.
    pub enemy: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Arsenal {
    pub w: usize,
    pub ammo: [u32; 4],
    pub reload: f32,
    pub reload_max: f32,
    pub fire_cd: f32,
    pub swap_t: f32,
    semi_lock: bool,
    pub recoil: f32,
    pub nades: u32,
    pub max_nades: u32,
    pub nade_cd: f32,
    /// En la ciudad el riel y el lanzagranadas arrancan bloqueados.
    pub unlocked: [bool; 4],
    pub mag_mul: f32,
    pub reload_mul: f32,
    pub dmg_mul: f32,
    pub crit_mul: f32,
    pub overdrive: f32,
}

impl Arsenal {
    pub fn new(city: bool) -> Self {
        Self {
            w: 0,
            ammo: WEAPONS.map(|w| w.mag),
            reload: 0.0,
            reload_max: 1.0,
            fire_cd: 0.0,
            swap_t: 0.0,
            semi_lock: false,
            recoil: 0.0,
            nades: 2,
            max_nades: 3,
            nade_cd: 0.0,
            unlocked: if city { [true, true, false, false] } else { [true; 4] },
            mag_mul: 1.0,
            reload_mul: 1.0,
            dmg_mul: 1.0,
            crit_mul: 2.0,
            overdrive: 0.0,
        }
    }

    pub fn def(&self) -> &'static WeaponDef {
        &WEAPONS[self.w]
    }

    pub fn mag_of(&self, w: usize) -> u32 {
        (WEAPONS[w].mag as f32 * self.mag_mul).round() as u32
    }

    pub fn refill(&mut self) {
        self.ammo = [0, 1, 2, 3].map(|i| self.mag_of(i));
        self.nades = self.max_nades;
    }

    /// Bloquea el apuntado mientras recarga o cambia de arma.
    pub fn busy(&self) -> bool {
        self.reload > 0.0 || self.swap_t > 0.0
    }

    fn start_reload(&mut self, ev: &mut Vec<WeaponEvent>) {
        if self.reload > 0.0 || self.ammo[self.w] == self.mag_of(self.w) || self.swap_t > 0.0 {
            return;
        }
        self.reload = WEAPONS[self.w].reload * self.reload_mul;
        self.reload_max = self.reload;
        ev.push(WeaponEvent::Reload);
    }

    fn switch(&mut self, target: i32, exact: bool, ev: &mut Vec<WeaponEvent>) {
        let n = WEAPONS.len() as i32;
        let mut w = target.rem_euclid(n);
        if !self.unlocked[w as usize] {
            if exact {
                ev.push(WeaponEvent::Locked);
                return;
            }
            let dir = if (w - self.w as i32).rem_euclid(n) <= n / 2 { 1 } else { -1 };
            for _ in 0..n {
                if self.unlocked[w as usize] {
                    break;
                }
                w = (w + dir).rem_euclid(n);
            }
            if !self.unlocked[w as usize] {
                return;
            }
        }
        let w = w as usize;
        if w == self.w {
            return;
        }
        self.w = w;
        self.reload = 0.0;
        self.swap_t = 0.32;
        self.fire_cd = self.fire_cd.max(0.1);
        self.semi_lock = false;
        ev.push(WeaponEvent::Swap(w));
    }

    /// Un paso: cambiar, recargar, disparar y tirar granadas.
    pub fn step(&mut self, input: &WeaponInput, p: &mut PlayerState, nades: &mut Vec<Nade>, rng: &mut Rng, dt: f32, ev: &mut Vec<WeaponEvent>) {
        if let Some(s) = input.select {
            self.switch(s as i32, true, ev);
        } else if input.cycle != 0 {
            self.switch(self.w as i32 + input.cycle.signum(), false, ev);
        }
        if input.reload {
            self.start_reload(ev);
        }
        self.fire_cd -= dt;
        self.nade_cd -= dt;
        if self.swap_t > 0.0 {
            self.swap_t -= dt;
        }
        if self.reload > 0.0 {
            self.reload -= dt;
            if self.reload <= 0.0 {
                self.reload = 0.0;
                self.ammo[self.w] = self.mag_of(self.w);
            }
        }
        if input.nade {
            self.throw_nade(p, nades, ev);
        }
        if input.fire {
            self.fire(p, nades, rng, ev);
        } else {
            self.semi_lock = false;
        }
        self.recoil = (self.recoil - dt * if self.w == 0 { 12.0 } else { 6.0 }).max(0.0);
        if self.overdrive > 0.0 {
            self.overdrive -= dt;
        }
        p.aim_blocked = self.busy();
    }

    fn fire(&mut self, p: &mut PlayerState, nades: &mut Vec<Nade>, rng: &mut Rng, ev: &mut Vec<WeaponEvent>) {
        let wd = &WEAPONS[self.w];
        if self.reload > 0.0 || self.fire_cd > 0.0 || self.swap_t > 0.0 || (self.semi_lock && !wd.auto) {
            return;
        }
        if self.ammo[self.w] == 0 {
            ev.push(WeaponEvent::Empty);
            self.fire_cd = 0.25;
            self.semi_lock = true;
            self.start_reload(ev);
            return;
        }
        let od = self.overdrive > 0.0;
        self.ammo[self.w] -= 1;
        self.fire_cd = wd.rate * if od { 0.6 } else { 1.0 };
        self.recoil = 1.0;
        p.slide = 0.0;
        if !wd.auto {
            self.semi_lock = true;
        }
        let eye = p.eye();
        let moving = p.horizontal_speed() > 1.0 || !p.on_ground;
        if wd.launcher {
            let sp = wd.spread[usize::from(p.horizontal_speed() > 1.0)] * (1.0 - p.aim_t * 0.8);
            let mut d = p.look_dir();
            d.x += (rng.f() - 0.5) * sp;
            d.y += (rng.f() - 0.5) * sp;
            let d = d.normalize();
            nades.push(Nade { pos: eye + d * 0.8, vel: d * 34.0 + p.vel * 0.3, fuse: 3.0, beep: 99.0, impact: true, gravity: 9.0, dmg: 7.0 * self.dmg_mul * if od { 1.5 } else { 1.0 }, enemy: false });
            let k = 1.0 - p.aim_t * 0.5;
            ev.push(WeaponEvent::Kick { pitch: wd.kick * k, yaw: 0.0 });
            p.shake = (p.shake + 0.25).min(1.0);
            ev.push(WeaponEvent::Launch { weapon: self.w });
        } else {
            let spread = wd.spread[usize::from(moving)] * (1.0 - p.aim_t * if self.w == 1 { 0.35 } else { 0.85 }) * (1.0 - p.crouch_t * 0.4);
            // radio angular equivalente a la dispersión en pantalla de la web
            let tan_half = (p.target_fov(self.w == 2).to_radians() / 2.0).tan() * REF_ASPECT;
            let (fwd, right, up) = basis(p.yaw, p.pitch);
            let pellets = (0..wd.pellets)
                .map(|_| {
                    let ang = rng.f() * std::f32::consts::TAU;
                    let rad = rng.f().sqrt() * spread * tan_half;
                    let dir = (fwd + right * ang.cos() * rad + up * ang.sin() * rad).normalize();
                    Pellet { origin: eye, dir }
                })
                .collect();
            let dmg = wd.dmg * self.dmg_mul * if od { 1.5 } else { 1.0 };
            ev.push(WeaponEvent::Fire { weapon: self.w, pellets, dmg, pierce: wd.pierce });
            let kick = (1.0 - p.aim_t * 0.5) * (1.0 - p.crouch_t * 0.35);
            ev.push(WeaponEvent::Kick { pitch: wd.kick * kick, yaw: (rng.f() - 0.5) * wd.kick * 0.45 * kick });
            if self.w > 0 {
                p.shake = (p.shake + 0.15).min(1.0);
            }
        }
        ev.push(WeaponEvent::Noise { pos: eye, radius: wd.noise });
        if self.ammo[self.w] == 0 {
            self.start_reload(ev);
        }
    }

    fn throw_nade(&mut self, p: &PlayerState, nades: &mut Vec<Nade>, ev: &mut Vec<WeaponEvent>) {
        if self.nades == 0 || self.nade_cd > 0.0 || self.swap_t > 0.0 {
            return;
        }
        self.nades -= 1;
        self.nade_cd = 0.6;
        let d = p.look_dir();
        nades.push(Nade { pos: p.eye() + d * 0.6, vel: d * 17.0 + Vec3::Y * 3.5 + p.vel * 0.6, fuse: 1.7, beep: 0.4, impact: false, gravity: 22.0, dmg: 9.0, enemy: false });
        ev.push(WeaponEvent::Toss);
    }
}

/// Adelante, derecha y arriba de la vista.
pub fn basis(yaw: f32, pitch: f32) -> (Vec3, Vec3, Vec3) {
    let f = look_dir(yaw, pitch);
    let (sy, cy) = yaw.sin_cos();
    let r = Vec3::new(cy, 0.0, -sy);
    (f, r, r.cross(f))
}

/// Granada enemiga: parábola hacia la última posición conocida.
pub fn enemy_nade(from: Vec3, target: Vec3) -> Nade {
    let pos = from + Vec3::Y * 1.7;
    let (dx, dz) = (target.x - from.x, target.z - from.z);
    let t = (dx.hypot(dz) / 13.0).max(0.8);
    let vel = Vec3::new(dx / t, (target.y - pos.y + 0.5 * 22.0 * t * t) / t, dz / t);
    Nade { pos, vel, fuse: t + 0.9, beep: 0.2, impact: false, gravity: 22.0, dmg: 9.0, enemy: true }
}

/// Avanza las granadas: rebotes contra cajas y piso, mecha y explosiones.
/// `touches_enemy` dice si un proyectil de impacto tocó un enemigo.
pub fn update_nades(nades: &mut Vec<Nade>, world: &World, dt: f32, mut touches_enemy: impl FnMut(Vec3) -> bool, ev: &mut Vec<WeaponEvent>) {
    let lim = world.lim();
    let mut i = 0;
    while i < nades.len() {
        let n = &mut nades[i];
        n.vel.y -= n.gravity * dt;
        let prev = n.pos;
        n.pos += n.vel * dt;
        if n.pos.y < 0.13 {
            n.pos.y = 0.13;
            n.vel.y = n.vel.y.abs() * 0.4;
            n.vel.x *= 0.7;
            n.vel.z *= 0.7;
        }
        let hit_box = world.box_at(n.pos);
        if let Some(b) = hit_box {
            if prev.y >= b.max.y - 0.05 {
                n.pos.y = b.max.y + 0.13;
                n.vel.y = n.vel.y.abs() * 0.4;
                n.vel.x *= 0.7;
                n.vel.z *= 0.7;
            } else {
                let was_in_x = prev.x > b.min.x && prev.x < b.max.x;
                n.pos.x = prev.x;
                n.pos.z = prev.z;
                if was_in_x {
                    n.vel.z *= -0.5;
                } else {
                    n.vel.x *= -0.5;
                }
            }
        }
        n.pos.x = n.pos.x.clamp(-lim, lim);
        n.pos.z = n.pos.z.clamp(-lim, lim);
        if n.impact {
            let hit = n.pos.y <= 0.14 || hit_box.is_some() || touches_enemy(n.pos);
            n.fuse -= dt;
            if hit || n.fuse <= 0.0 {
                ev.push(WeaponEvent::Explosion(Explosion { pos: n.pos, radius: 4.5, max_dmg: n.dmg, self_dmg: 16.0, big: true, enemy: n.enemy }));
                nades.swap_remove(i);
                continue;
            }
            i += 1;
            continue;
        }
        n.fuse -= dt;
        n.beep -= dt;
        if n.beep <= 0.0 {
            ev.push(WeaponEvent::Beep);
            n.beep = if n.fuse < 0.6 { 0.1 } else { 0.3 };
        }
        if n.fuse <= 0.0 {
            ev.push(WeaponEvent::Explosion(Explosion { pos: n.pos, radius: 6.5, max_dmg: n.dmg, self_dmg: 30.0, big: true, enemy: n.enemy }));
            nades.swap_remove(i);
            continue;
        }
        i += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tick(a: &mut Arsenal, inp: WeaponInput, p: &mut PlayerState, n: usize) -> Vec<WeaponEvent> {
        let (mut nades, mut rng, mut ev) = (Vec::new(), Rng::new(1), Vec::new());
        for _ in 0..n {
            a.step(&inp, p, &mut nades, &mut rng, crate::TICK, &mut ev);
        }
        ev
    }

    #[test]
    fn rifle_automatico_a_10_por_segundo() {
        let mut a = Arsenal::new(false);
        let mut p = PlayerState::new(0.0, 0.0);
        let ev = tick(&mut a, WeaponInput { fire: true, ..Default::default() }, &mut p, 60);
        let shots = ev.iter().filter(|e| matches!(e, WeaponEvent::Fire { .. })).count();
        assert!((9..=11).contains(&shots), "{shots} disparos");
        assert_eq!(a.ammo[0], 24 - shots as u32);
    }

    #[test]
    fn escopeta_semiautomatica_con_9_perdigones() {
        let mut a = Arsenal::new(false);
        let mut p = PlayerState::new(0.0, 0.0);
        tick(&mut a, WeaponInput { select: Some(1), ..Default::default() }, &mut p, 30);
        let ev = tick(&mut a, WeaponInput { fire: true, ..Default::default() }, &mut p, 120);
        let fires: Vec<_> = ev.iter().filter_map(|e| if let WeaponEvent::Fire { pellets, .. } = e { Some(pellets.len()) } else { None }).collect();
        assert_eq!(fires, vec![9], "mantener apretado dispara una sola vez");
    }

    #[test]
    fn recarga_al_vaciar() {
        let mut a = Arsenal::new(false);
        let mut p = PlayerState::new(0.0, 0.0);
        tick(&mut a, WeaponInput { fire: true, ..Default::default() }, &mut p, 60 * 3);
        assert!(a.ammo[0] == 24 || a.reload > 0.0);
        tick(&mut a, WeaponInput::default(), &mut p, 90);
        assert_eq!(a.ammo[0], 24);
    }

    #[test]
    fn en_la_ciudad_el_riel_arranca_bloqueado() {
        let mut a = Arsenal::new(true);
        let mut p = PlayerState::new(0.0, 0.0);
        let ev = tick(&mut a, WeaponInput { select: Some(2), ..Default::default() }, &mut p, 1);
        assert!(ev.contains(&WeaponEvent::Locked));
        assert_eq!(a.w, 0);
        // la rueda saltea las bloqueadas
        tick(&mut a, WeaponInput { cycle: -1, ..Default::default() }, &mut p, 1);
        assert_eq!(a.w, 1);
    }

    #[test]
    fn la_granada_rebota_y_explota() {
        let w = World::new(60.0);
        let mut nades = vec![Nade { pos: Vec3::new(0.0, 1.5, 0.0), vel: Vec3::new(5.0, 3.0, 0.0), fuse: 1.7, beep: 0.4, impact: false, gravity: 22.0, dmg: 9.0, enemy: false }];
        let mut ev = Vec::new();
        for _ in 0..120 {
            update_nades(&mut nades, &w, crate::TICK, |_| false, &mut ev);
        }
        assert!(nades.is_empty());
        assert!(ev.iter().any(|e| matches!(e, WeaponEvent::Explosion(x) if x.radius == 6.5)));
    }
}
