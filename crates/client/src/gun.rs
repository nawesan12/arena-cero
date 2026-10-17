//! Arma en primera persona: escena propia (capa 1) dibujada encima del mundo con su cámara,
//! luces y reflejos, más la animación de `18-ciclo-de-juego.js` (balanceo, retroceso, recarga,
//! cambio de arma, apuntado) y el destello de boca.

use crate::gunmodels::{self, GUN_LAYER, Gm, Kit, Parts};
use crate::player::{LocalPlayer, Look, MainCamera};
use arena_sim::weapons::WEAPONS;
use bevy::camera::visibility::RenderLayers;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::light::{GeneratedEnvironmentMapLight, NotShadowCaster};
use bevy::prelude::*;

pub const GUN_FOV: f32 = 60.0;
pub const GUN_FOV_AIM: f32 = 50.0;
/// Posición de reposo del arma respecto de la cámara.
pub const GUN_BASE: Vec3 = Vec3::new(0.17, -0.175, -0.39);
/// Por arma: (y, z) apuntando y (y, z) de la boca del caño.
const AIM: [[f32; 2]; 4] = [[-0.114, -0.27], [-0.08, -0.25], [-0.096, -0.2], [-0.1, -0.26]];
const MUZZLE: [[f32; 2]; 4] = [[0.004, -0.6], [0.02, -0.6], [0.0, -0.66], [0.01, -0.56]];
const FLASH_COLOR: [u32; 4] = [0xffc66a, 0xffc66a, 0x5fd8ff, 0xffb02e];

#[derive(Component)]
pub struct GunCamera;

#[derive(Resource)]
pub struct GunRig {
    root: Entity,
    models: [Entity; 4],
    parts: [Parts; 4],
    flash: Entity,
    flash_light: Entity,
    world_flash: Entity,
    pub gm: Gm,
    shown: usize,
    drum_angle: f32,
    pub drum_target: f32,
    /// Tiempo que queda de destello.
    pub flash_t: f32,
    /// Boca del caño en coordenadas de cámara (para las trazadoras).
    pub muzzle_cam: Vec3,
}

pub struct GunPlugin;

impl Plugin for GunPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_gun).add_systems(PostUpdate, animate_gun.before(TransformSystems::Propagate));
    }
}

fn spawn_gun(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>, mut mats: ResMut<Assets<StandardMaterial>>, mut images: ResMut<Assets<Image>>, main_cam: Query<Entity, With<MainCamera>>) {
    let layer = RenderLayers::layer(GUN_LAYER);
    // cámara del arma: no se mueve; el mundo no está en su capa
    commands.spawn((
        Camera3d::default(),
        Camera { order: 1, clear_color: ClearColorConfig::None, ..default() },
        Projection::from(PerspectiveProjection { fov: GUN_FOV.to_radians(), near: 0.01, far: 10.0, ..default() }),
        Tonemapping::None,
        GeneratedEnvironmentMapLight { environment_map: images.add(gunmodels::env_cubemap()), intensity: 900.0, ..default() },
        layer.clone(),
        GunCamera,
    ));
    // luz principal cálida y contraluz frío, como en la web
    for (pos, color, lux) in [(Vec3::new(-1.0, 2.0, 1.2), Color::srgb_u8(0xff, 0xf0, 0xdc), 3750.0), (Vec3::new(1.5, 0.4, -2.0), Color::srgb_u8(0x8f, 0xd8, 0xff), 1650.0)] {
        commands.spawn((DirectionalLight { color, illuminance: lux, ..default() }, Transform::from_translation(pos).looking_at(Vec3::ZERO, Vec3::Y), layer.clone()));
    }
    let gm = Gm::new(&mut mats, &mut images);
    let root = commands.spawn((Transform::from_translation(GUN_BASE), Visibility::default(), layer.clone())).id();
    let mut kit = Kit {
        cube: meshes.add(Cuboid::new(1.0, 1.0, 1.0)),
        cyl: meshes.add(Cylinder::new(1.0, 1.0).mesh().resolution(20)),
        tube: meshes.add(gunmodels::open_tube()),
        cmd: &mut commands,
        meshes: &mut meshes,
    };
    let builders: [fn(&mut Kit, Entity, &Gm) -> Parts; 4] = [gunmodels::rifle, gunmodels::shotgun, gunmodels::rail, gunmodels::launcher];
    let mut models = [Entity::PLACEHOLDER; 4];
    let mut parts: [Parts; 4] = Default::default();
    for (i, b) in builders.iter().enumerate() {
        let g = kit.group(root, Vec3::ZERO);
        kit.cmd.entity(g).insert(if i == 0 { Visibility::Inherited } else { Visibility::Hidden });
        parts[i] = b(&mut kit, g, &gm);
        models[i] = g;
    }
    let glow = mats.add(StandardMaterial { base_color_texture: Some(images.add(gunmodels::glow_tex())), unlit: true, alpha_mode: AlphaMode::Add, ..default() });
    let flash = commands.spawn((Mesh3d(meshes.add(Rectangle::new(0.42, 0.42))), MeshMaterial3d(glow), Transform::default(), Visibility::Hidden, layer.clone(), NotShadowCaster)).id();
    let flash_light = commands.spawn((PointLight { intensity: 0.0, range: 2.0, ..default() }, Transform::default(), layer.clone())).id();
    commands.entity(root).add_children(&[flash, flash_light]);
    // luz del disparo en el mundo, delante de la cámara
    let world_flash = commands.spawn((PointLight { intensity: 0.0, range: 10.0, ..default() }, Transform::from_xyz(0.0, 0.0, -1.0))).id();
    if let Ok(cam) = main_cam.single() {
        commands.entity(cam).add_child(world_flash);
    }
    commands.insert_resource(GunRig { root, models, parts, flash, flash_light, world_flash, gm, shown: 0, drum_angle: 0.0, drum_target: 0.0, flash_t: 0.0, muzzle_cam: GUN_BASE });
}

fn col(h: u32) -> Color {
    crate::world_render::col(h)
}

#[allow(clippy::too_many_arguments)]
fn animate_gun(
    time: Res<Time>,
    lp: Res<LocalPlayer>,
    mut look: ResMut<Look>,
    mut rig: ResMut<GunRig>,
    mut tf: Query<&mut Transform>,
    mut vis: Query<&mut Visibility>,
    mut lights: Query<&mut PointLight>,
    mut cam: Query<&mut Projection, With<GunCamera>>,
    mut mats: ResMut<Assets<StandardMaterial>>,
) {
    let dt = time.delta_secs();
    let (p, a) = (&lp.state, &lp.arsenal);
    let w = a.w;
    // modelo visible (cambia a mitad de la animación de cambio, como en la web)
    if rig.shown != w && a.swap_t < 0.2 {
        let (old, new) = (rig.models[rig.shown], rig.models[w]);
        if let Ok(mut v) = vis.get_mut(old) {
            *v = Visibility::Hidden;
        }
        if let Ok(mut v) = vis.get_mut(new) {
            *v = Visibility::Inherited;
        }
        rig.shown = w;
    }
    let sw_i = rig.shown;
    let hs = p.horizontal_speed();
    let aim = p.aim_t;
    let rl = if a.reload > 0.0 { (((a.reload_max - a.reload) / a.reload_max).min(1.0) * std::f32::consts::PI).sin() } else { 0.0 };
    let sw = if a.swap_t > 0.0 { (a.swap_t / 0.32 * std::f32::consts::PI).sin() } else { 0.0 };
    let bob_amt = (hs / 7.0).min(1.0) * (1.0 - aim * 0.8);
    look.sway_x *= (-10.0 * dt).exp();
    look.sway_y *= (-10.0 * dt).exp();
    let sway_k = 1.0 - aim * 0.85;
    let bob = lp.bob;
    let aim_pos = AIM[sw_i];
    let pos = Vec3::new(
        GUN_BASE.x * (1.0 - aim) + bob.cos() * 0.01 * bob_amt - p.slide.max(0.0) * 0.05 - look.sway_x * 0.25 * sway_k,
        GUN_BASE.y + (aim_pos[0] - GUN_BASE.y) * aim + bob.sin().abs() * 0.014 * bob_amt - rl * 0.12 - sw * 0.25 + look.sway_y * 0.2 * sway_k,
        GUN_BASE.z + (aim_pos[1] - GUN_BASE.z) * aim + a.recoil * if w == 1 { 0.09 } else { 0.05 } * (1.0 - aim * 0.5),
    );
    let rot = Quat::from_euler(
        EulerRot::XYZ,
        a.recoil * 0.07 * (1.0 - aim * 0.6) - rl * 0.6 - sw * 0.5 + look.sway_y * sway_k,
        look.sway_x * sway_k + (1.0 - aim) * 0.07,
        rl * 0.4 + if p.slide > 0.0 { 0.25 } else { 0.0 } + look.sway_x * 0.6 * sway_k,
    );
    if let Ok(mut t) = tf.get_mut(rig.root) {
        t.translation = pos;
        t.rotation = rot;
    }
    let m = MUZZLE[sw_i];
    rig.muzzle_cam = pos + rot * Vec3::new(0.0, m[0], m[1] - 0.02);
    // animaciones por arma
    let parts = rig.parts[sw_i].clone();
    let wd = &WEAPONS[sw_i];
    if let Some(e) = parts.mag {
        if let Ok(mut t) = tf.get_mut(e) {
            t.translation.y = -rl * 0.12;
        }
    }
    if let Some(e) = parts.pump {
        let pk = if a.fire_cd > 0.0 { ((1.0 - (a.fire_cd - 0.2) / (wd.rate - 0.2)).min(1.0) * std::f32::consts::PI).sin().max(0.0) } else { 0.0 };
        if let Ok(mut t) = tf.get_mut(e) {
            t.translation.z = pk * 0.07;
        }
    }
    if sw_i == 2 {
        let charge = if a.reload > 0.0 { 0.0 } else { (1.0 - a.fire_cd.max(0.0) / wd.rate).min(1.0) };
        if let Some(mut mm) = mats.get_mut(&rig.gm.cyan) {
            mm.base_color = Color::srgb(0.37 * charge + 0.05, 0.85 * charge + 0.1, charge + 0.15);
        }
        let t0 = time.elapsed_secs();
        for (i, e) in parts.coils.iter().enumerate() {
            if let Ok(mut t) = tf.get_mut(*e) {
                t.scale = Vec3::splat(1.0 + (t0 / 0.12 + i as f32).sin() * 0.04 * charge);
            }
        }
        // con el visor puesto el arma se oculta
        if let Ok(mut v) = vis.get_mut(rig.models[2]) {
            *v = if aim < 0.8 { Visibility::Inherited } else { Visibility::Hidden };
        }
    }
    if let Some(e) = parts.drum {
        let k = (dt * 14.0).min(1.0);
        rig.drum_angle += (rig.drum_target - rig.drum_angle) * k;
        let ang = rig.drum_angle;
        if let Ok(mut t) = tf.get_mut(e) {
            t.rotation = Quat::from_rotation_z(ang);
        }
    }
    // destello
    rig.flash_t -= dt;
    let on = rig.flash_t > 0.0;
    let fpos = Vec3::new(0.0, m[0], m[1] - 0.02);
    if let Ok(mut v) = vis.get_mut(rig.flash) {
        *v = if on { Visibility::Inherited } else { Visibility::Hidden };
    }
    if let Ok(mut t) = tf.get_mut(rig.flash) {
        t.translation = fpos;
        if on {
            t.rotation = Quat::from_rotation_z((time.elapsed_secs() * 1234.567).sin() * 3.0);
        }
        t.scale = Vec3::splat(if sw_i == 1 || sw_i == 3 { 1.5 } else { 1.0 });
    }
    let pierce = sw_i == 2;
    if let Ok(mut l) = lights.get_mut(rig.flash_light) {
        l.intensity = if on { (if pierce { 4.0 } else { 3.0 }) * 2500.0 } else { 0.0 };
        l.color = col(FLASH_COLOR[sw_i]);
    }
    if let Ok(mut t) = tf.get_mut(rig.flash_light) {
        t.translation = fpos;
    }
    if let Ok(mut l) = lights.get_mut(rig.world_flash) {
        l.intensity = if on { (if pierce { 3.0 } else { 2.5 }) * 60000.0 } else { 0.0 };
        l.color = col(FLASH_COLOR[sw_i]);
    }
    if let Ok(mut pr) = cam.single_mut() {
        if let Projection::Perspective(pp) = &mut *pr {
            pp.fov = (GUN_FOV + (GUN_FOV_AIM - GUN_FOV) * aim).to_radians();
        }
    }
}
