//! Objetos nuevos del mundo: cajas de botín, nubes de humo, señuelos, drones exploradores y
//! rehenes de las misiones.

use crate::game::GameRes;
use arena_sim::director::MissionKind;
use bevy::light::NotShadowCaster;
use bevy::prelude::*;
use std::collections::HashMap;

pub struct ExtrasViewPlugin;

impl Plugin for ExtrasViewPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup).add_systems(Update, (loot, pools));
    }
}

#[derive(Resource)]
struct A {
    cube: Handle<Mesh>,
    sphere: Handle<Mesh>,
    crate_m: Handle<StandardMaterial>,
    mint: Handle<StandardMaterial>,
    amber: Handle<StandardMaterial>,
}

#[derive(Component)]
struct SmokeVis(usize, usize);
#[derive(Component)]
struct DecoyVis(usize);
#[derive(Component)]
struct ScoutVis(usize);
#[derive(Component)]
struct HostageVis(usize);

fn col(h: u32) -> Color {
    crate::world_render::col(h)
}

fn setup(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>, mut mats: ResMut<Assets<StandardMaterial>>) {
    let cube = meshes.add(Cuboid::new(1.0, 1.0, 1.0));
    let sphere = meshes.add(Sphere::new(1.0).mesh().ico(2).unwrap());
    let smoke = mats.add(StandardMaterial { base_color: Color::srgba(0.62, 0.64, 0.66, 0.55), alpha_mode: AlphaMode::Blend, perceptual_roughness: 1.0, ..default() });
    let dark = mats.add(StandardMaterial { base_color: col(0x1c2125), metallic: 0.5, perceptual_roughness: 0.4, ..default() });
    let red = mats.add(StandardMaterial { base_color: col(0xff3d5a), unlit: true, ..default() });
    let cyan = mats.add(StandardMaterial { base_color: col(0x5fd8ff), unlit: true, ..default() });
    let civ = mats.add(StandardMaterial { base_color: col(0x3d6fa8), perceptual_roughness: 0.9, ..default() });
    let skin = mats.add(StandardMaterial { base_color: col(0xc8a07c), perceptual_roughness: 0.9, ..default() });
    // humo: 4 nubes de 5 esferas cada una
    for i in 0..4 {
        for k in 0..5 {
            commands.spawn((Mesh3d(sphere.clone()), MeshMaterial3d(smoke.clone()), Transform::default(), Visibility::Hidden, NotShadowCaster, SmokeVis(i, k)));
        }
    }
    for i in 0..4 {
        commands.spawn((Mesh3d(cube.clone()), MeshMaterial3d(dark.clone()), Transform::from_scale(Vec3::new(0.25, 0.3, 0.25)), Visibility::Hidden, NotShadowCaster, DecoyVis(i))).with_child((Mesh3d(sphere.clone()), MeshMaterial3d(red.clone()), Transform::from_xyz(0.0, 0.7, 0.0).with_scale(Vec3::splat(0.35))));
    }
    for i in 0..3 {
        commands
            .spawn((Transform::default(), Visibility::Hidden, ScoutVis(i)))
            .with_children(|d| {
                d.spawn((Mesh3d(cube.clone()), MeshMaterial3d(dark.clone()), Transform::from_scale(Vec3::new(0.5, 0.15, 0.5)), NotShadowCaster));
                for (x, z) in [(-0.4, -0.4), (0.4, -0.4), (-0.4, 0.4), (0.4, 0.4)] {
                    d.spawn((Mesh3d(cube.clone()), MeshMaterial3d(cyan.clone()), Transform::from_xyz(x, 0.08, z).with_scale(Vec3::new(0.35, 0.02, 0.06)), NotShadowCaster));
                }
            });
    }
    // rehén: persona sentada con las manos atrás (cuerpo azul)
    for i in 0..2 {
        commands
            .spawn((Transform::default(), Visibility::Hidden, HostageVis(i)))
            .with_children(|h| {
                h.spawn((Mesh3d(cube.clone()), MeshMaterial3d(civ.clone()), Transform::from_xyz(0.0, 0.75, 0.0).with_scale(Vec3::new(0.5, 0.6, 0.3))));
                h.spawn((Mesh3d(cube.clone()), MeshMaterial3d(skin.clone()), Transform::from_xyz(0.0, 1.22, 0.0).with_scale(Vec3::splat(0.26))));
                h.spawn((Mesh3d(cube.clone()), MeshMaterial3d(civ.clone()), Transform::from_xyz(0.0, 0.3, 0.2).with_scale(Vec3::new(0.45, 0.25, 0.6))));
                h.spawn((Mesh3d(sphere.clone()), MeshMaterial3d(cyan.clone()), Transform::from_xyz(0.0, 1.8, 0.0).with_scale(Vec3::splat(0.12)), NotShadowCaster));
            });
    }
    let crate_m = mats.add(StandardMaterial { base_color: col(0x3a4a3e), metallic: 0.3, perceptual_roughness: 0.6, ..default() });
    let mint = mats.add(StandardMaterial { base_color: col(0x5dffb0), unlit: true, ..default() });
    let amber = mats.add(StandardMaterial { base_color: col(0xffb02e), unlit: true, ..default() });
    commands.insert_resource(A { cube, sphere, crate_m, mint, amber });
}

/// Cajas de botín: una caja verde oscura con franja luminosa (dorada la del cargamento).
fn loot(mut commands: Commands, game: Res<GameRes>, a: Res<A>, mut known: Local<HashMap<u32, Entity>>, mut seen_gen: Local<u32>) {
    if *seen_gen != game.1 {
        *seen_gen = game.1;
        for (_, e) in known.drain() {
            commands.entity(e).despawn();
        }
    }
    for l in &game.0.loot {
        if known.contains_key(&l.id) {
            continue;
        }
        let e = commands
            .spawn((Mesh3d(a.cube.clone()), MeshMaterial3d(a.crate_m.clone()), Transform::from_translation(l.pos + Vec3::Y * 0.3).with_scale(Vec3::new(0.9, 0.6, 0.6))))
            .with_child((Mesh3d(a.cube.clone()), MeshMaterial3d(if l.rich { a.amber.clone() } else { a.mint.clone() }), Transform::from_xyz(0.0, 0.0, 0.0).with_scale(Vec3::new(1.02, 0.12, 1.02)), NotShadowCaster))
            .id();
        known.insert(l.id, e);
    }
    let ids: std::collections::HashSet<u32> = game.0.loot.iter().map(|l| l.id).collect();
    known.retain(|id, e| {
        let keep = ids.contains(id);
        if !keep {
            commands.entity(*e).despawn();
        }
        keep
    });
    let _ = &a.sphere;
}

#[allow(clippy::type_complexity)]
fn pools(
    time: Res<Time>,
    game: Res<GameRes>,
    mut smoke: Query<(&SmokeVis, &mut Transform, &mut Visibility), (Without<DecoyVis>, Without<ScoutVis>, Without<HostageVis>)>,
    mut decoy: Query<(&DecoyVis, &mut Transform, &mut Visibility), (Without<SmokeVis>, Without<ScoutVis>, Without<HostageVis>)>,
    mut scout: Query<(&ScoutVis, &mut Transform, &mut Visibility), (Without<SmokeVis>, Without<DecoyVis>, Without<HostageVis>)>,
    mut hostage: Query<(&HostageVis, &mut Transform, &mut Visibility), (Without<SmokeVis>, Without<DecoyVis>, Without<ScoutVis>)>,
) {
    let g = &game.0;
    let t = time.elapsed_secs();
    for (s, mut tr, mut v) in &mut smoke {
        match g.smokes.get(s.0) {
            Some(sm) => {
                let k = s.1 as f32;
                let off = Vec3::new((k * 2.1).cos(), 0.4 + (k * 1.3).sin() * 0.4, (k * 2.1).sin()) * sm.r * 0.45;
                tr.translation = sm.pos + Vec3::Y * 1.2 + off + Vec3::Y * (t * 0.3 + k).sin() * 0.2;
                tr.scale = Vec3::splat(sm.r * (0.55 + 0.1 * k / 4.0) * (sm.life / 1.5).min(1.0));
                *v = Visibility::Inherited;
            }
            None => *v = Visibility::Hidden,
        }
    }
    for (d, mut tr, mut v) in &mut decoy {
        match g.decoys.get(d.0) {
            Some(dc) => {
                tr.translation = dc.pos + Vec3::Y * 0.15;
                *v = if dc.beep > 1.2 || (t * 4.0).sin() > -0.5 { Visibility::Inherited } else { Visibility::Hidden };
            }
            None => *v = Visibility::Hidden,
        }
    }
    for (s, mut tr, mut v) in &mut scout {
        match g.scouts.get(s.0) {
            Some(sc) => {
                tr.translation = sc.pos;
                tr.rotation = Quat::from_rotation_y(t * 3.0);
                *v = Visibility::Inherited;
            }
            None => *v = Visibility::Hidden,
        }
    }
    let hostages: Vec<_> = g.director.missions.iter().filter(|m| m.kind == MissionKind::Hostage).collect();
    for (h, mut tr, mut v) in &mut hostage {
        match hostages.get(h.0) {
            Some(m) => {
                tr.translation = m.pos;
                *v = Visibility::Inherited;
            }
            None => *v = Visibility::Hidden,
        }
    }
}
