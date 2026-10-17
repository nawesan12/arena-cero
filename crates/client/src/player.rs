//! Jugador local: junta el input cada cuadro, corre la simulación a paso fijo (60 Hz) e
//! interpola la cámara entre pasos. El mismo esquema sirve después para predecir contra el servidor.

use crate::world_render::LevelRes;
use arena_sim::player::BASE_FOV;
use arena_sim::{PlayerEvent, PlayerInput, PlayerState, TICK};
use bevy::camera::Exposure;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::input::mouse::AccumulatedMouseMotion;
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
}

/// Pulsaciones guardadas hasta el próximo paso fijo (si no, se perderían entre pasos).
#[derive(Resource, Default)]
struct Latch {
    jump: bool,
    dash: bool,
    hook: bool,
}

#[derive(Resource)]
pub struct LocalPlayer {
    pub state: PlayerState,
    pub prev: Vec3,
    pub events: Vec<PlayerEvent>,
}

#[derive(Component)]
pub struct MainCamera;

pub fn spawn_player(commands: &mut Commands, level: &LevelRes, start: Option<[f32; 4]>) {
    let info = &level.0.info;
    let mut state = PlayerState::new(info.start[0], info.start[1]);
    if let Some([x, z, yaw, pitch]) = start {
        state.pos.x = x;
        state.pos.z = z;
        state.pos.y = level.0.world.ground_at(x, z, 200.0) + arena_sim::player::EYE;
        state.yaw = yaw;
        state.pitch = pitch;
    }
    commands.insert_resource(Look { yaw: state.yaw, pitch: state.pitch, fov: BASE_FOV });
    commands.insert_resource(LocalPlayer { prev: state.pos, state, events: Vec::new() });
    let fog_end = info.fog[1].min(350.0);
    commands.spawn((
        Camera3d::default(),
        MainCamera,
        Projection::from(PerspectiveProjection { fov: BASE_FOV.to_radians(), near: 0.05, far: 360.0, ..default() }),
        Tonemapping::None,
        Exposure::default(),
        DistanceFog {
            color: crate::world_render::col(info.bg),
            falloff: FogFalloff::Linear { start: info.fog[0].min(fog_end * 0.4), end: fog_end },
            ..default()
        },
        Transform::from_translation(Vec3::new(info.start[0], 1.6, info.start[1])),
        crate::world_render::LevelEntity,
    ));
}

fn grab_cursor(mut cursor: Single<&mut CursorOptions>, mouse: Res<ButtonInput<MouseButton>>, key: Res<ButtonInput<KeyCode>>, settings: Res<Settings>) {
    if !settings.input {
        return;
    }
    if mouse.just_pressed(MouseButton::Left) && cursor.grab_mode == CursorGrabMode::None {
        cursor.visible = false;
        cursor.grab_mode = CursorGrabMode::Locked;
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
}

fn latch_input(key: Res<ButtonInput<KeyCode>>, mut latch: ResMut<Latch>) {
    latch.jump |= key.just_pressed(KeyCode::Space);
    latch.dash |= key.just_pressed(KeyCode::KeyQ);
    latch.hook |= key.just_pressed(KeyCode::KeyE) || key.just_pressed(KeyCode::ControlLeft) || key.just_pressed(KeyCode::ControlRight);
}

fn step_player(key: Res<ButtonInput<KeyCode>>, mouse: Res<ButtonInput<MouseButton>>, look: Res<Look>, mut latch: ResMut<Latch>, level: Res<LevelRes>, mut lp: ResMut<LocalPlayer>) {
    let axis = |pos: KeyCode, neg: KeyCode| f32::from(u8::from(key.pressed(pos))) - f32::from(u8::from(key.pressed(neg)));
    let input = PlayerInput {
        forward: axis(KeyCode::KeyW, KeyCode::KeyS),
        right: axis(KeyCode::KeyD, KeyCode::KeyA),
        yaw: look.yaw,
        pitch: look.pitch,
        sprint: key.pressed(KeyCode::ShiftLeft) || key.pressed(KeyCode::ShiftRight),
        crouch: key.pressed(KeyCode::KeyC),
        aim: mouse.pressed(MouseButton::Right),
        jump: std::mem::take(&mut latch.jump),
        dash: std::mem::take(&mut latch.dash),
        hook: std::mem::take(&mut latch.hook),
    };
    let lp = &mut *lp;
    lp.prev = lp.state.eye();
    lp.state.step(&input, &level.0.world, TICK, &mut lp.events);
}

fn place_camera(time: Res<Time>, fixed: Res<Time<Fixed>>, mut look: ResMut<Look>, lp: Res<LocalPlayer>, mut cam: Query<(&mut Transform, &mut Projection), With<MainCamera>>) {
    let Ok((mut t, mut proj)) = cam.single_mut() else { return };
    let s = &lp.state;
    let a = fixed.overstep_fraction();
    let eye = lp.prev.lerp(s.eye(), a);
    let dt = time.delta_secs();
    let target = s.target_fov(false);
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
