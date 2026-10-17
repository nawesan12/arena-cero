//! Jugador local: junta el input cada cuadro, corre la simulación a paso fijo (60 Hz) e
//! interpola la cámara entre pasos. El mismo esquema sirve después para predecir contra el servidor.

use crate::fx::Fx;
use crate::gun::GunRig;
use crate::world_render::LevelRes;
use arena_sim::player::{BASE_FOV, EYE};
use arena_sim::rng::Rng;
use arena_sim::weapons::{Arsenal, Nade, WEAPONS, WeaponEvent, WeaponInput, update_nades};
use arena_sim::{PlayerEvent, PlayerInput, PlayerState, TICK};
use bevy::camera::Exposure;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll};
use bevy::pbr::{DistanceFog, FogFalloff};
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions};

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Time::<Fixed>::from_seconds(TICK as f64))
            .init_resource::<Look>()
            .init_resource::<Latch>()
            .init_resource::<Settings>()
            .add_systems(Update, (grab_cursor, mouse_look, latch_input).chain())
            .add_systems(FixedUpdate, step_player)
            .add_systems(PostUpdate, place_camera.before(TransformSystems::Propagate));
    }
}

#[derive(Resource)]
pub struct Settings {
    pub sens: f32,
    pub input: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self { sens: 1.8, input: true }
    }
}

/// Vista: se actualiza cada cuadro (no cada paso) para que el mouse responda sin demora.
#[derive(Resource, Default)]
pub struct Look {
    pub yaw: f32,
    pub pitch: f32,
    pub fov: f32,
    /// Inercia del arma al mover el mouse.
    pub sway_x: f32,
    pub sway_y: f32,
}

/// Pulsaciones guardadas hasta el próximo paso fijo (si no, se perderían entre pasos).
#[derive(Resource, Default)]
struct Latch {
    jump: bool,
    dash: bool,
    hook: bool,
    reload: bool,
    nade: bool,
    select: Option<usize>,
    cycle: i32,
    /// El clic que toma el mouse no dispara.
    grabbed_at: f64,
}

#[derive(Resource)]
pub struct LocalPlayer {
    pub state: PlayerState,
    pub prev: Vec3,
    pub events: Vec<PlayerEvent>,
    pub arsenal: Arsenal,
    pub nades: Vec<Nade>,
    pub rng: Rng,
    pub hp: f32,
    pub max_hp: f32,
    pub last_hurt: f32,
    pub regen_mul: f32,
    /// Fase del balanceo al caminar.
    pub bob: f32,
    /// 0..1: viñeta roja al recibir daño.
    pub hurt_flash: f32,
    /// Hacia dónde vino el último golpe (radianes en pantalla) y cuánto queda del indicador.
    pub dmg_dir: f32,
    pub dmg_t: f32,
    pub start: Vec3,
}

impl LocalPlayer {
    pub fn hurt(&mut self, d: f32, from: Option<Vec3>) {
        if d <= 0.0 {
            return;
        }
        self.hp = (self.hp - d).max(0.0);
        self.last_hurt = 0.0;
        self.hurt_flash = (self.hurt_flash + 0.55).min(1.0);
        self.state.shake = (self.state.shake + 0.25).min(1.0);
        if let Some(f) = from {
            let (dx, dz) = (f.x - self.state.pos.x, f.z - self.state.pos.z);
            let (sy, cy) = self.state.yaw.sin_cos();
            let fwd = -dx * sy - dz * cy;
            let rgt = dx * cy - dz * sy;
            self.dmg_dir = rgt.atan2(fwd);
            self.dmg_t = 1.0;
        }
    }
}

#[derive(Component)]
pub struct MainCamera;

pub fn spawn_player(commands: &mut Commands, level: &LevelRes, start: Option<[f32; 4]>, weapon: Option<usize>) {
    let info = &level.0.info;
    let mut state = PlayerState::new(info.start[0], info.start[1]);
    if let Some([x, z, yaw, pitch]) = start {
        state.pos.x = x;
        state.pos.z = z;
        state.pos.y = level.0.world.ground_at(x, z, 200.0) + EYE;
        state.yaw = yaw;
        state.pitch = pitch;
    }
    let mut arsenal = Arsenal::new(info.city);
    if let Some(w) = weapon {
        arsenal.unlocked[w] = true;
        arsenal.w = w;
    }
    commands.insert_resource(Look { yaw: state.yaw, pitch: state.pitch, fov: BASE_FOV, ..default() });
    commands.insert_resource(LocalPlayer {
        prev: state.pos,
        start: state.pos,
        state,
        events: Vec::new(),
        arsenal,
        nades: Vec::new(),
        rng: Rng::new(7),
        hp: 100.0,
        max_hp: 100.0,
        last_hurt: 10.0,
        regen_mul: 1.0,
        bob: 0.0,
        hurt_flash: 0.0,
        dmg_dir: 0.0,
        dmg_t: 0.0,
    });
    let fog_end = info.fog[1].min(350.0);
    commands.spawn((
        Camera3d::default(),
        MainCamera,
        Projection::from(PerspectiveProjection { fov: BASE_FOV.to_radians(), near: 0.05, far: 360.0, ..default() }),
        Tonemapping::None,
        Exposure::default(),
        DistanceFog { color: crate::world_render::col(info.bg), falloff: FogFalloff::Linear { start: info.fog[0].min(fog_end * 0.4), end: fog_end }, ..default() },
        Transform::from_translation(Vec3::new(info.start[0], 1.6, info.start[1])),
        crate::world_render::LevelEntity,
    ));
}

fn grab_cursor(mut cursor: Single<&mut CursorOptions>, mouse: Res<ButtonInput<MouseButton>>, key: Res<ButtonInput<KeyCode>>, settings: Res<Settings>, time: Res<Time<Real>>, mut latch: ResMut<Latch>) {
    if !settings.input {
        return;
    }
    if mouse.just_pressed(MouseButton::Left) && cursor.grab_mode == CursorGrabMode::None {
        cursor.visible = false;
        cursor.grab_mode = CursorGrabMode::Locked;
        latch.grabbed_at = time.elapsed_secs_f64();
    }
    if key.just_pressed(KeyCode::Escape) {
        cursor.visible = true;
        cursor.grab_mode = CursorGrabMode::None;
    }
}

fn mouse_look(motion: Res<AccumulatedMouseMotion>, cursor: Single<&CursorOptions>, settings: Res<Settings>, mut look: ResMut<Look>) {
    if cursor.grab_mode == CursorGrabMode::None {
        return;
    }
    let d = motion.delta;
    // picos al tomar el puntero
    if d.x.abs() > 180.0 || d.y.abs() > 180.0 {
        return;
    }
    let s = 0.0022 * settings.sens * (look.fov / BASE_FOV);
    look.yaw -= d.x * s;
    look.pitch = (look.pitch - d.y * s).clamp(-1.5, 1.5);
    look.sway_x = (look.sway_x + d.x * 0.00012).clamp(-0.06, 0.06);
    look.sway_y = (look.sway_y + d.y * 0.0001).clamp(-0.05, 0.05);
}

fn latch_input(key: Res<ButtonInput<KeyCode>>, scroll: Res<AccumulatedMouseScroll>, cursor: Single<&CursorOptions>, mut latch: ResMut<Latch>) {
    if cursor.grab_mode == CursorGrabMode::None {
        return;
    }
    latch.jump |= key.just_pressed(KeyCode::Space);
    latch.dash |= key.just_pressed(KeyCode::KeyQ);
    latch.hook |= key.just_pressed(KeyCode::KeyE) || key.just_pressed(KeyCode::ControlLeft) || key.just_pressed(KeyCode::ControlRight);
    latch.reload |= key.just_pressed(KeyCode::KeyR);
    latch.nade |= key.just_pressed(KeyCode::KeyG);
    for (i, k) in [KeyCode::Digit1, KeyCode::Digit2, KeyCode::Digit3, KeyCode::Digit4].into_iter().enumerate() {
        if key.just_pressed(k) {
            latch.select = Some(i);
        }
    }
    if scroll.delta.y != 0.0 {
        latch.cycle = if scroll.delta.y < 0.0 { 1 } else { -1 };
    }
}

#[allow(clippy::too_many_arguments)]
fn step_player(
    key: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    cursor: Single<&CursorOptions>,
    time: Res<Time<Real>>,
    mut look: ResMut<Look>,
    mut latch: ResMut<Latch>,
    level: Res<LevelRes>,
    mut lp: ResMut<LocalPlayer>,
    mut rig: ResMut<GunRig>,
    mut fx: ResMut<Fx>,
) {
    let playing = cursor.grab_mode != CursorGrabMode::None;
    let axis = |pos: KeyCode, neg: KeyCode| if playing { f32::from(u8::from(key.pressed(pos))) - f32::from(u8::from(key.pressed(neg))) } else { 0.0 };
    let input = PlayerInput {
        forward: axis(KeyCode::KeyW, KeyCode::KeyS),
        right: axis(KeyCode::KeyD, KeyCode::KeyA),
        yaw: look.yaw,
        pitch: look.pitch,
        sprint: playing && (key.pressed(KeyCode::ShiftLeft) || key.pressed(KeyCode::ShiftRight)),
        crouch: playing && key.pressed(KeyCode::KeyC),
        aim: playing && mouse.pressed(MouseButton::Right),
        jump: std::mem::take(&mut latch.jump),
        dash: std::mem::take(&mut latch.dash),
        hook: std::mem::take(&mut latch.hook),
    };
    let winput = WeaponInput {
        fire: playing && mouse.pressed(MouseButton::Left) && time.elapsed_secs_f64() - latch.grabbed_at > 0.25,
        reload: std::mem::take(&mut latch.reload),
        select: latch.select.take(),
        cycle: std::mem::take(&mut latch.cycle),
        nade: std::mem::take(&mut latch.nade),
    };
    let world = &level.0.world;
    let lp = &mut *lp;
    lp.prev = lp.state.eye();
    let mut pev = std::mem::take(&mut lp.events);
    pev.clear();
    lp.state.step(&input, world, TICK, &mut pev);
    for e in &pev {
        match *e {
            PlayerEvent::Burn(d) => {
                lp.hurt(f32::from(d), None);
                fx.burst(Vec3::new(lp.state.pos.x, 0.1, lp.state.pos.z), 0xff8030, 2, 3.0, 0.4);
            }
            PlayerEvent::Dash => fx.burst(lp.state.pos - Vec3::Y, 0x5fd8ff, 18, 3.0, 0.35),
            PlayerEvent::DoubleJump | PlayerEvent::HookBoost => fx.burst(lp.state.pos - Vec3::Y * EYE, 0x5fd8ff, 16, 4.0, 0.4),
            _ => {}
        }
    }
    if lp.state.slide > 0.0 && lp.state.on_ground && lp.rng.next() < 0.5 {
        fx.burst(Vec3::new(lp.state.pos.x, 0.05, lp.state.pos.z), 0x3a4246, 1, 2.0, 0.4);
    }
    lp.events = pev;

    // armas
    let mut wev = Vec::new();
    let (arsenal, state, nades, rng) = (&mut lp.arsenal, &mut lp.state, &mut lp.nades, &mut lp.rng);
    arsenal.step(&winput, state, nades, rng, TICK, &mut wev);
    update_nades(&mut lp.nades, world, TICK, |_| false, &mut wev);
    // boca del caño en el mundo: la posición del arma en espacio de cámara llevada al mundo
    let cam_rot = Quat::from_euler(EulerRot::YXZ, lp.state.yaw, lp.state.pitch, 0.0);
    let muzzle = lp.state.eye() + cam_rot * rig.muzzle_cam;
    for e in wev {
        match e {
            WeaponEvent::Fire { weapon, pellets, pierce, .. } => {
                // el destello dura casi todo el cuadro de disparo (en la web, hasta rate − 0,04)
                rig.flash_t = (WEAPONS[weapon].rate - 0.04).min(0.06);
                let color = if pierce { 0x5fd8ff } else { 0xffc66a };
                for (k, p) in pellets.iter().enumerate() {
                    let t = world.ray(p.origin, p.dir, 120.0);
                    let end = p.origin + p.dir * if t < 120.0 { t } else { 80.0 };
                    if t < 120.0 {
                        fx.burst(end, if pierce { 0x5fd8ff } else { 0xffb02e }, if pierce { 10 } else { 3 }, 4.0, 0.3);
                    }
                    if pierce || k % 3 == 0 {
                        fx.tracer(muzzle, end, color, if pierce { 0.18 } else { 0.05 });
                    }
                }
            }
            WeaponEvent::Launch { .. } => {
                rig.flash_t = 0.06;
                rig.drum_target += std::f32::consts::PI / 3.0;
            }
            WeaponEvent::Kick { pitch, yaw } => {
                look.pitch = (look.pitch + pitch).min(1.5);
                look.yaw += yaw;
            }
            WeaponEvent::Explosion(x) => {
                fx.burst(x.pos, 0xffb02e, if x.big { 60 } else { 30 }, if x.big { 12.0 } else { 8.0 }, 0.7);
                fx.burst(x.pos, 0xff3d5a, if x.big { 24 } else { 12 }, 9.0, 0.6);
                fx.burst(x.pos, 0x2a2a2a, if x.big { 20 } else { 8 }, 5.0, 1.2);
                fx.boom(x.pos, x.big);
                let body = Vec3::new(lp.state.pos.x, lp.state.pos.y - 0.8, lp.state.pos.z);
                let pd = body.distance(x.pos);
                lp.state.shake = (lp.state.shake + if x.big { 0.9 } else { 0.4 } * (1.0 - pd / 20.0).max(0.0)).min(1.0);
                if x.self_dmg > 0.0 && pd < x.radius {
                    lp.hurt((x.self_dmg * (1.0 - pd / x.radius)).round(), Some(x.pos));
                }
            }
            _ => {}
        }
    }

    // vida: regeneración tras 4 s sin daño; al morir, de vuelta al inicio (todavía no hay enemigos)
    lp.last_hurt += TICK;
    if lp.last_hurt > 4.0 && lp.hp < lp.max_hp {
        lp.hp = (lp.hp + 4.0 * lp.regen_mul * TICK).min(lp.max_hp);
    }
    if lp.hp <= 0.0 {
        let start = lp.start;
        lp.state = PlayerState::new(start.x, start.z);
        lp.state.pos = start;
        lp.prev = start;
        lp.hp = lp.max_hp;
        lp.nades.clear();
    }
    if lp.state.on_ground && lp.state.slide <= 0.0 {
        lp.bob += TICK * lp.state.horizontal_speed() * 1.6;
    }
    lp.hurt_flash = (lp.hurt_flash - TICK * 1.6).max(0.0);
    lp.dmg_t = (lp.dmg_t - TICK * 1.2).max(0.0);
}

fn place_camera(time: Res<Time>, fixed: Res<Time<Fixed>>, mut look: ResMut<Look>, lp: Res<LocalPlayer>, mut cam: Query<(&mut Transform, &mut Projection), With<MainCamera>>) {
    let Ok((mut t, mut proj)) = cam.single_mut() else { return };
    let s = &lp.state;
    let a = fixed.overstep_fraction();
    let eye = lp.prev.lerp(s.eye(), a);
    let dt = time.delta_secs();
    let target = s.target_fov(lp.arsenal.w == 2);
    look.fov += (target - look.fov) * (dt * 20.0).min(1.0);
    let sh = s.shake * s.shake * 0.035;
    let jitter = |k: f32| (((time.elapsed_secs() * 97.0 + k).sin() * 43758.547).fract() - 0.5) * sh;
    let roll = if s.slide > 0.0 { -0.06 } else { 0.0 } * s.crouch_t;
    t.translation = eye;
    t.rotation = Quat::from_euler(EulerRot::YXZ, look.yaw + jitter(1.3), look.pitch + jitter(7.1), roll);
    if let Projection::Perspective(p) = &mut *proj {
        p.fov = look.fov.to_radians();
    }
}
