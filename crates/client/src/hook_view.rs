//! Gancho visible: lanzador en la muñeca izquierda (en la capa del arma), cable metálico que
//! viaja hasta el punto de enganche, garra de tres puntas con brillo cian y anillo en la mira
//! cuando lo que apuntás está a tiro. La web solo dibujaba una línea y un cono.

use crate::gunmodels::GUN_LAYER;
use crate::player::LocalPlayer;
use crate::world_render::LevelRes;
use arena_sim::player::hook_cast;
use bevy::camera::visibility::RenderLayers;
use bevy::light::NotShadowCaster;
use bevy::prelude::*;

pub struct HookViewPlugin;

impl Plugin for HookViewPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Hookable>().add_systems(Startup, spawn).add_systems(PostUpdate, (aim_check, draw).chain().after(TransformSystems::Propagate));
    }
}

/// ¿Lo que está en la mira se puede enganchar? (lo lee el HUD para el anillo)
#[derive(Resource, Default)]
pub struct Hookable {
    pub can: bool,
    pub hooked: bool,
}

#[derive(Component)]
struct Cable;
#[derive(Component)]
struct Claw;
#[derive(Component)]
struct Bracer;
#[derive(Component)]
struct BracerLight;

const CYAN: u32 = 0x5fd8ff;

fn col(h: u32) -> Color {
    crate::world_render::col(h)
}

fn spawn(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>, mut mats: ResMut<Assets<StandardMaterial>>) {
    let steel = mats.add(StandardMaterial { base_color: col(0x8a959c), metallic: 0.6, perceptual_roughness: 0.35, ..default() });
    let dark = mats.add(StandardMaterial { base_color: col(0x1c2125), metallic: 0.4, perceptual_roughness: 0.5, ..default() });
    let amber = mats.add(StandardMaterial { base_color: col(0xffb02e), unlit: true, ..default() });
    let glow = mats.add(StandardMaterial { base_color: col(CYAN), unlit: true, ..default() });
    let cable_mat = mats.add(StandardMaterial { base_color: col(0xc9d4d0), metallic: 0.5, perceptual_roughness: 0.4, ..default() });
    let cube = meshes.add(Cuboid::new(1.0, 1.0, 1.0));
    let cyl = meshes.add(Cylinder::new(1.0, 1.0).mesh().resolution(10));

    // cable: un cilindro de 1 m que se estira entre la muñeca y la garra
    commands.spawn((Mesh3d(cyl.clone()), MeshMaterial3d(cable_mat), Transform::default(), Visibility::Hidden, NotShadowCaster, Cable));

    // garra: cuerpo, punta y tres uñas abiertas, con un aro cian
    commands
        .spawn((Transform::default(), Visibility::Hidden, Claw))
        .with_children(|c| {
            c.spawn((Mesh3d(cyl.clone()), MeshMaterial3d(dark.clone()), Transform::from_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)).with_scale(Vec3::new(0.06, 0.18, 0.06)), NotShadowCaster));
            c.spawn((Mesh3d(meshes.add(Cone::new(0.06, 0.14).mesh().resolution(8))), MeshMaterial3d(amber.clone()), Transform::from_xyz(0.0, 0.0, -0.16).with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)), NotShadowCaster));
            for k in 0..3 {
                let a = k as f32 / 3.0 * std::f32::consts::TAU;
                let dir = Vec3::new(a.cos(), a.sin(), 0.0);
                c.spawn((
                    Mesh3d(cube.clone()),
                    MeshMaterial3d(steel.clone()),
                    Transform::from_translation(dir * 0.08 + Vec3::new(0.0, 0.0, -0.06)).with_rotation(Quat::from_rotation_z(a) * Quat::from_rotation_y(0.5)).with_scale(Vec3::new(0.025, 0.025, 0.2)),
                    NotShadowCaster,
                ));
            }
            c.spawn((Mesh3d(meshes.add(Torus::new(0.07, 0.085).mesh().minor_resolution(6).major_resolution(16))), MeshMaterial3d(glow.clone()), Transform::from_xyz(0.0, 0.0, 0.05).with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)), NotShadowCaster));
        });

    // lanzador de muñeca, abajo a la izquierda en la vista del arma
    let layer = RenderLayers::layer(GUN_LAYER);
    commands
        .spawn((Transform::from_xyz(-0.22, -0.19, -0.42).with_scale(Vec3::splat(0.7)), Visibility::default(), layer.clone(), Bracer))
        .with_children(|b| {
            for (m, t) in [
                (dark.clone(), Transform::from_xyz(0.0, 0.0, 0.0).with_scale(Vec3::new(0.07, 0.05, 0.16))),
                (steel.clone(), Transform::from_xyz(0.0, 0.032, -0.02).with_scale(Vec3::new(0.05, 0.015, 0.12))),
                (amber.clone(), Transform::from_xyz(-0.036, 0.0, 0.02).with_scale(Vec3::new(0.004, 0.012, 0.06))),
            ] {
                b.spawn((Mesh3d(cube.clone()), MeshMaterial3d(m), t, layer.clone(), NotShadowCaster));
            }
            b.spawn((Mesh3d(cyl.clone()), MeshMaterial3d(steel.clone()), Transform::from_xyz(0.0, 0.0, -0.1).with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)).with_scale(Vec3::new(0.018, 0.08, 0.018)), layer.clone(), NotShadowCaster));
            b.spawn((Mesh3d(cube.clone()), MeshMaterial3d(glow.clone()), Transform::from_xyz(0.0, 0.028, 0.05).with_scale(Vec3::new(0.03, 0.006, 0.03)), layer.clone(), NotShadowCaster, BracerLight));
        });
}

/// Mira cada tanto si lo apuntado está a tiro del gancho.
fn aim_check(time: Res<Time>, mut t: Local<f32>, lp: Res<LocalPlayer>, level: Res<LevelRes>, mut hk: ResMut<Hookable>) {
    hk.hooked = lp.state.hook.is_some();
    *t -= time.delta_secs();
    if *t > 0.0 {
        return;
    }
    *t = 0.08;
    hk.can = !hk.hooked && lp.state.hook_cd <= 0.0 && hook_cast(&level.0.world, lp.state.eye(), lp.state.look_dir()).is_some();
}

#[allow(clippy::type_complexity)]
fn draw(
    time: Res<Time>,
    lp: Res<LocalPlayer>,
    hk: Res<Hookable>,
    cam: Query<&GlobalTransform, With<crate::player::MainCamera>>,
    mut cable: Query<(&mut Transform, &mut Visibility), (With<Cable>, Without<Claw>, Without<Bracer>, Without<BracerLight>)>,
    mut claw: Query<(&mut Transform, &mut Visibility), (With<Claw>, Without<Cable>, Without<Bracer>, Without<BracerLight>)>,
    mut bracer: Query<&mut Transform, (With<Bracer>, Without<Cable>, Without<Claw>, Without<BracerLight>)>,
    mut light: Query<&mut Transform, (With<BracerLight>, Without<Cable>, Without<Claw>, Without<Bracer>)>,
    mut kick: Local<f32>,
) {
    let Ok(ct) = cam.single() else { return };
    let dt = time.delta_secs();
    // culatazo del lanzador al disparar
    if lp.events.iter().any(|e| matches!(e, arena_sim::PlayerEvent::HookFired)) {
        *kick = 1.0;
    }
    *kick = (*kick - dt * 6.0).max(0.0);
    for mut t in &mut bracer {
        t.translation = Vec3::new(-0.22, -0.19 - *kick * 0.015, -0.42 + *kick * 0.04);
        t.rotation = Quat::from_rotation_x(*kick * 0.25) * Quat::from_rotation_y(0.12);
    }
    let pulse = if hk.can { 1.0 + (time.elapsed_secs() * 10.0).sin() * 0.25 } else { 0.6 };
    for mut t in &mut light {
        t.scale = Vec3::new(0.03 * pulse, 0.006, 0.03 * pulse);
    }
    let Some(h) = lp.state.hook else {
        for (_, mut v) in &mut cable {
            *v = Visibility::Hidden;
        }
        for (_, mut v) in &mut claw {
            *v = Visibility::Hidden;
        }
        return;
    };
    // de la muñeca a la punta, que viaja 0,08 s. El lanzador está en la cámara del arma (FOV 60);
    // en la cámara del mundo (FOV 75) el mismo lugar de pantalla queda ×1,33 más afuera.
    let start = ct.transform_point(Vec3::new(-0.29, -0.24, -0.45));
    let tip = start.lerp(h.pt, (h.t / 0.08).min(1.0));
    let d = tip - start;
    let len = d.length().max(0.01);
    let dir = d / len;
    for (mut t, mut v) in &mut cable {
        t.translation = start + d * 0.5;
        t.rotation = Quat::from_rotation_arc(Vec3::Y, dir);
        t.scale = Vec3::new(0.012, len, 0.012);
        *v = Visibility::Inherited;
    }
    for (mut t, mut v) in &mut claw {
        t.translation = tip;
        // la garra mira hacia adelante (−Z) en la dirección del tiro
        t.rotation = Quat::from_rotation_arc(Vec3::NEG_Z, dir);
        *v = Visibility::Inherited;
    }
}
