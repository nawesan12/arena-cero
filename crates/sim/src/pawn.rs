//! Jugadores remotos (nuevo, para el online): lo que el anfitrión o el servidor dedicado simulan
//! de cada jugador conectado a partir de los controles que manda su cliente. Movimiento, armas,
//! vida con regeneración, caer y reaparecer, y usar con F (derribos, objetivos, reabastecerse).
//!
//! El jugador del anfitrión sigue en el cliente (como en solitario) y es siempre el objetivo 0;
//! los remotos son los objetivos 1, 2, 3 (en el servidor dedicado, que no tiene jugador propio,
//! arrancan en 0).

use crate::enemies::Target;
use crate::game::{Game, GameEvent};
use crate::maps::Level;
use crate::player::{EYE, PlayerEvent, PlayerInput, PlayerState};
use crate::rng::Rng;
use crate::weapons::{Arsenal, WeaponEvent, WeaponInput};
use glam::Vec3;

/// Controles que manda un cliente. Las pulsaciones van como contadores: si cambió, hubo
/// pulsación (así no se pierden aunque se pierda un paquete).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PawnInput {
    pub forward: f32,
    pub right: f32,
    pub yaw: f32,
    pub pitch: f32,
    pub sprint: bool,
    pub crouch: bool,
    pub aim: bool,
    pub fire: bool,
    pub use_held: bool,
    pub light: bool,
    /// Arma elegida (0-3).
    pub weapon: u8,
    pub jump_n: u8,
    pub dash_n: u8,
    pub hook_n: u8,
    pub reload_n: u8,
    pub nade_n: u8,
    pub tag_n: u8,
}

/// Un jugador remoto en la partida.
#[derive(Clone, Debug)]
pub struct Pawn {
    pub id: u8,
    pub name: String,
    pub state: PlayerState,
    pub arsenal: Arsenal,
    pub rng: Rng,
    pub hp: f32,
    pub max_hp: f32,
    pub last_hurt: f32,
    /// Caído: segundos que faltan para reaparecer.
    pub down: f32,
    pub input: PawnInput,
    last: PawnInput,
    /// F sostenida en un objetivo.
    hold: f32,
    use_was: bool,
    /// Eventos de movimiento y armas de este paso (para que el cliente los muestre).
    pub fx: Vec<PlayerEvent>,
    pub wfx: Vec<WeaponEvent>,
    pub kills: u32,
}

impl Pawn {
    pub fn new(id: u8, name: String, at: Vec3, city: bool) -> Self {
        let mut state = PlayerState::new(at.x, at.z);
        state.pos.y = at.y + EYE;
        Self { id, name, state, arsenal: Arsenal::new(city), rng: Rng::new(100 + u32::from(id)), hp: 100.0, max_hp: 100.0, last_hurt: 10.0, down: 0.0, input: PawnInput::default(), last: PawnInput::default(), hold: 0.0, use_was: false, fx: Vec::new(), wfx: Vec::new(), kills: 0 }
    }

    /// Lo que la IA ve de este jugador.
    pub fn target(&self) -> Target {
        let s = &self.state;
        Target { pos: s.pos, body: s.body(), vel: s.vel, feet: s.feet(), crouch_t: s.crouch_t, evasive: s.hook.is_some() || s.dash_t > 0.0, light: self.input.light }
    }

    pub fn alive(&self) -> bool {
        self.down <= 0.0
    }

    pub fn hurt(&mut self, dmg: f32) {
        if !self.alive() || dmg <= 0.0 {
            return;
        }
        self.hp = (self.hp - dmg).max(0.0);
        self.last_hurt = 0.0;
        if self.hp <= 0.0 {
            self.down = 5.0;
        }
    }
}

impl Game {
    /// Dónde reaparece un jugador remoto: el punto de control de la campaña, el refugio o el inicio.
    fn pawn_spawn(&self, level: &Level) -> Vec3 {
        if let Some(cp) = &self.campaign {
            return cp.checkpoint + Vec3::new(1.5, 0.0, 1.5);
        }
        if let Some(c) = &level.city {
            return c.safe_pos + Vec3::new(2.5, -0.84, 2.5);
        }
        let s = level.info.start;
        Vec3::new(s[0] + 2.0, 0.16, s[1] + 2.0)
    }

    /// Suma un jugador remoto al lado del punto de reaparición.
    pub fn add_pawn(&mut self, level: &Level, id: u8, name: String) {
        let at = self.pawn_spawn(level) + Vec3::new(f32::from(id) * 1.2, 0.0, 0.0);
        let g = level.world.ground_at(at.x, at.z, at.y + 2.0);
        self.pawns.push(Pawn::new(id, name, Vec3::new(at.x, g, at.z), level.info.city));
    }

    pub fn remove_pawn(&mut self, id: u8) {
        self.pawns.retain(|p| p.id != id);
    }

    /// Objetivos de la IA: el anfitrión (si juega) y después los remotos vivos.
    pub fn all_targets(&self, host: Option<Target>) -> Vec<Target> {
        let mut v: Vec<Target> = host.into_iter().collect();
        for p in &self.pawns {
            // caído: lejos, para que nadie le apunte
            v.push(if p.alive() { p.target() } else { Target { pos: Vec3::splat(1e5), body: Vec3::splat(1e5), ..p.target() } });
        }
        v
    }

    /// Un paso de los jugadores remotos (antes de `step_multi`): moverse, disparar, regenerar.
    pub fn step_pawns(&mut self, level: &mut Level, dt: f32) {
        for i in 0..self.pawns.len() {
            let p = &mut self.pawns[i];
            p.fx.clear();
            p.wfx.clear();
            if !p.alive() {
                p.down -= dt;
                if p.down <= 0.0 {
                    let at = self.pawn_spawn(level);
                    let p = &mut self.pawns[i];
                    let g = level.world.ground_at(at.x, at.z, at.y + 2.0);
                    let (yaw, sm) = (p.state.yaw, p.state.speed_mul);
                    p.state = PlayerState::new(at.x, at.z);
                    p.state.pos.y = g + EYE;
                    p.state.yaw = yaw;
                    p.state.speed_mul = sm;
                    p.hp = p.max_hp;
                    p.arsenal.refill();
                }
                continue;
            }
            let (inp, last) = (p.input, p.last);
            let pulse = |a: u8, b: u8| a != b;
            let mv = PlayerInput {
                forward: inp.forward.clamp(-1.0, 1.0),
                right: inp.right.clamp(-1.0, 1.0),
                yaw: inp.yaw,
                pitch: inp.pitch.clamp(-1.5, 1.5),
                sprint: inp.sprint,
                crouch: inp.crouch,
                aim: inp.aim,
                jump: pulse(inp.jump_n, last.jump_n),
                dash: pulse(inp.dash_n, last.dash_n),
                hook: pulse(inp.hook_n, last.hook_n),
            };
            let mut pev = std::mem::take(&mut p.fx);
            p.state.step(&mv, &level.world, dt, &mut pev);
            p.fx = pev;
            let win = WeaponInput { fire: inp.fire, reload: pulse(inp.reload_n, last.reload_n), select: (inp.weapon != p.arsenal.w as u8).then_some(inp.weapon as usize), cycle: 0, nade: pulse(inp.nade_n, last.nade_n) };
            let mut wev = Vec::new();
            let (arsenal, state, rng) = (&mut p.arsenal, &mut p.state, &mut p.rng);
            arsenal.step(&win, state, &mut self.nades, rng, dt, &mut wev);
            let tag = pulse(inp.tag_n, last.tag_n);
            p.last = inp;
            p.last_hurt += dt;
            if p.last_hurt > 4.0 && p.hp < p.max_hp {
                p.hp = (p.hp + 4.0 * dt).min(p.max_hp);
            }
            let (eye, dir, body, crit) = (p.state.eye(), p.state.look_dir(), p.state.body(), p.arsenal.crit_mul);
            let right = Vec3::new(-p.state.yaw.cos(), 0.0, p.state.yaw.sin());
            let muzzle = eye + dir * 0.6 - right * 0.25 - Vec3::Y * 0.2;
            let kills_before = self.kills;
            self.weapon_events(&wev, crit, muzzle, body, level);
            let p = &mut self.pawns[i];
            p.kills += self.kills - kills_before;
            p.wfx = wev;
            if tag {
                self.try_tag(level, eye, dir);
            }
        }
    }

    /// Aplica a los remotos los golpes de este paso. `offset` es el índice del primer remoto en la
    /// lista de objetivos (1 si el anfitrión juega, 0 en el servidor dedicado).
    pub fn apply_pawn_hits(&mut self, offset: usize) {
        for e in &self.events {
            if let GameEvent::Hurt { dmg, to, .. } = e {
                let k = *to as usize;
                if k >= offset {
                    if let Some(p) = self.pawns.get_mut(k - offset) {
                        p.hurt(*dmg);
                    }
                }
            }
        }
    }

    /// F de los remotos: derribo por la espalda, objetivos (mantener) y reabastecerse.
    pub fn step_pawn_use(&mut self, level: &mut Level, dt: f32) {
        use crate::objectives::Interact;
        for i in 0..self.pawns.len() {
            let p = &self.pawns[i];
            if !p.alive() {
                continue;
            }
            let a = crate::objectives::Actor { pos: p.state.pos, feet: p.state.feet(), f_held: p.input.use_held };
            let pressed = a.f_held && !p.use_was;
            let found = self.find_interact_pub(level, &a);
            let p = &mut self.pawns[i];
            p.use_was = a.f_held;
            match found {
                Some(Interact::Objective { op, time }) if a.f_held => {
                    p.hold += dt;
                    if p.hold >= time {
                        p.hold = 0.0;
                        self.objective_reached(op, a.pos, level);
                    }
                }
                Some(Interact::Takedown { enemy }) if pressed => {
                    let hp = self.enemies[enemy].hp;
                    self.takedowns += 1;
                    self.damage(enemy, hp + 1.0, None, true, a.pos, level);
                    self.pawns[i].kills += 1;
                }
                Some(Interact::Supply { ready: true, .. }) if pressed => {
                    let p = &mut self.pawns[i];
                    p.hp = p.max_hp;
                    p.arsenal.refill();
                }
                _ => p.hold = 0.0,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::maps::{MapId, build_level};

    #[test]
    fn un_remoto_camina_dispara_cae_y_reaparece() {
        let mut l = build_level(MapId::Deposito);
        let mut g = Game::new(&mut l, 7);
        g.add_pawn(&l, 1, "Amigo".into());
        let start = g.pawns[0].state.pos;
        g.pawns[0].input = PawnInput { forward: 1.0, ..Default::default() };
        for _ in 0..60 {
            g.step_pawns(&mut l, crate::TICK);
        }
        assert!(g.pawns[0].state.pos.distance(start) > 3.0, "camina");
        // dispara: gasta munición
        let ammo = g.pawns[0].arsenal.ammo[0];
        g.pawns[0].input.fire = true;
        for _ in 0..30 {
            g.step_pawns(&mut l, crate::TICK);
        }
        assert!(g.pawns[0].arsenal.ammo[0] < ammo, "dispara");
        // le pegan hasta tirarlo y reaparece con la vida llena
        g.events.push(GameEvent::Hurt { dmg: 150.0, from: None, to: 1 });
        g.apply_pawn_hits(1);
        assert!(!g.pawns[0].alive());
        for _ in 0..(6.0 / crate::TICK) as usize {
            g.step_pawns(&mut l, crate::TICK);
        }
        assert!(g.pawns[0].alive() && g.pawns[0].hp >= 99.0);
    }

    #[test]
    fn los_enemigos_van_contra_el_remoto_si_esta_mas_cerca() {
        let mut l = build_level(MapId::Ciudad);
        let mut g = Game::new(&mut l, 1);
        g.add_pawn(&l, 1, "Amigo".into());
        // el remoto plantado frente al mando y el anfitrión lejos, en el refugio
        let m = l.city.as_ref().unwrap().outposts.iter().find(|o| o.id == crate::city::OutpostId::Mando).map(|o| (o.cx, o.cz)).unwrap();
        g.pawns[0].state.pos = Vec3::new(m.0, 1.76, m.1 + 14.0);
        let host = Target { pos: Vec3::new(-180.0, 1.76, 176.0), body: Vec3::new(-180.0, 1.26, 176.0), vel: Vec3::ZERO, feet: 0.16, crouch_t: 0.0, evasive: false, light: false };
        let mut to_remote = 0.0;
        let mut to_host = 0.0;
        for _ in 0..(60 * 20) {
            let ts = g.all_targets(Some(host));
            g.step_multi(&mut l, &ts, &[true, true], crate::TICK);
            for e in g.events.drain(..) {
                if let GameEvent::Hurt { dmg, to, .. } = e {
                    if to == 1 { to_remote += dmg } else { to_host += dmg }
                }
            }
        }
        assert!(to_remote > 0.0, "le disparan al remoto");
        assert_eq!(to_host, 0.0, "el anfitrión está lejos");
    }
}
