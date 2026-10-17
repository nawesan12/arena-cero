//! Autos: modelos (carrocería, cabina, ruedas que giran y doblan, faros) y sonido del motor.

use crate::game::GameRes;
use bevy::audio::{AudioSinkPlayback, Volume};
use bevy::prelude::*;
use std::collections::HashMap;

const CAR: [u32; 6] = [0xb83a30, 0x2f5f8f, 0xd8d4c8, 0x2d2f31, 0x6f7f4a, 0xd9a43a];

pub struct VehiclesViewPlugin;

impl Plugin for VehiclesViewPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (sync, engine));
    }
}

#[derive(Component)]
struct CarRig {
    wheels: [Entity; 4],
}

#[derive(Component)]
pub struct EngineSound;

#[allow(clippy::type_complexity)]
fn sync(
    mut commands: Commands,
    game: Res<GameRes>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut mats: ResMut<Assets<StandardMaterial>>,
    mut known: Local<HashMap<u32, Entity>>,
    mut assets: Local<Option<(Handle<Mesh>, Handle<Mesh>, Vec<Handle<StandardMaterial>>)>>,
    mut seen_gen: Local<u32>,
    mut roots: Query<(&mut Transform, &CarRig)>,
    mut wheels: Query<&mut Transform, Without<CarRig>>,
    kids: Query<&Children, With<CarRig>>,
    mut car_mats: Query<&mut MeshMaterial3d<StandardMaterial>>,
    mut wrecked: Local<Vec<u32>>,
) {
    let g = &game.0;
    if *seen_gen != game.1 {
        *seen_gen = game.1;
        for (_, e) in known.drain() {
            commands.entity(e).despawn();
        }
        wrecked.clear();
    }
    let (cube, wheel, m) = assets.get_or_insert_with(|| {
        let pbr = |c: u32, metal: f32, rough: f32| StandardMaterial { base_color: crate::world_render::col(c), metallic: metal, perceptual_roughness: rough, ..default() };
        let mut list: Vec<Handle<StandardMaterial>> = CAR.iter().map(|&c| mats.add(pbr(c, 0.5, 0.4))).collect();
        list.push(mats.add(pbr(0x1d2a33, 0.8, 0.15))); // 6: vidrio
        list.push(mats.add(pbr(0x1a1a1a, 0.0, 0.9))); // 7: goma
        list.push(mats.add(StandardMaterial { base_color: crate::world_render::col(0xfff2d0), unlit: true, ..default() })); // 8: faro
        list.push(mats.add(StandardMaterial { base_color: crate::world_render::col(0xff3d5a), unlit: true, ..default() })); // 9: luz trasera
        list.push(mats.add(pbr(0x8a959c, 0.7, 0.3))); // 10: llanta
        list.push(mats.add(pbr(0x1b1a19, 0.2, 0.95))); // 11: chatarra quemada
        let mut w = Cylinder::new(0.36, 0.26).mesh().resolution(14).build();
        w.rotate_by(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2));
        (meshes.add(Cuboid::new(1.0, 1.0, 1.0)), meshes.add(w), list)
    });
    for v in &g.vehicles {
        if known.contains_key(&v.id) {
            continue;
        }
        let body = m[v.color as usize % 6].clone();
        let root = commands.spawn((Transform::from_translation(v.pos).with_rotation(Quat::from_rotation_y(v.yaw)), Visibility::default())).id();
        // el modelo mira a +Z (adelante del auto)
        let parts: [(usize, Vec3, Vec3); 9] = [
            (99, Vec3::new(0.0, 0.62, 0.0), Vec3::new(1.9, 0.7, 4.3)),
            (99, Vec3::new(0.0, 0.32, 0.0), Vec3::new(1.86, 0.2, 4.2)),
            (6, Vec3::new(0.0, 1.22, -0.2), Vec3::new(1.66, 0.56, 2.2)),
            (99, Vec3::new(0.0, 1.52, -0.25), Vec3::new(1.6, 0.06, 1.9)),
            (8, Vec3::new(-0.62, 0.68, 2.16), Vec3::new(0.36, 0.14, 0.04)),
            (8, Vec3::new(0.62, 0.68, 2.16), Vec3::new(0.36, 0.14, 0.04)),
            (9, Vec3::new(-0.66, 0.7, -2.16), Vec3::new(0.32, 0.12, 0.04)),
            (9, Vec3::new(0.66, 0.7, -2.16), Vec3::new(0.32, 0.12, 0.04)),
            (7, Vec3::new(0.0, 0.42, 2.17), Vec3::new(1.5, 0.16, 0.06)),
        ];
        for (mi, p, s) in parts {
            let mat = if mi == 99 { body.clone() } else { m[mi].clone() };
            let e = commands.spawn((Mesh3d(cube.clone()), MeshMaterial3d(mat), Transform::from_translation(p).with_scale(s))).id();
            commands.entity(root).add_child(e);
        }
        let mut ws = [Entity::PLACEHOLDER; 4];
        for (i, (x, z)) in [(-0.86, 1.35), (0.86, 1.35), (-0.86, -1.35), (0.86, -1.35)].into_iter().enumerate() {
            let e = commands.spawn((Mesh3d(wheel.clone()), MeshMaterial3d(m[7].clone()), Transform::from_xyz(x, 0.36, z))).id();
            let hub = commands.spawn((Mesh3d(cube.clone()), MeshMaterial3d(m[10].clone()), Transform::from_xyz(x.signum() * 0.14, 0.0, 0.0).with_scale(Vec3::new(0.02, 0.36, 0.36)))).id();
            commands.entity(e).add_child(hub);
            commands.entity(root).add_child(e);
            ws[i] = e;
        }
        commands.entity(root).insert(CarRig { wheels: ws });
        known.insert(v.id, root);
    }
    for v in &g.vehicles {
        let Some(&e) = known.get(&v.id) else { continue };
        if v.wrecked && !wrecked.contains(&v.id) {
            // destruido: carrocería quemada
            wrecked.push(v.id);
            if let Ok(children) = kids.get(e) {
                for c in children.iter() {
                    if let Ok(mut mm) = car_mats.get_mut(c) {
                        mm.0 = m[11].clone();
                    }
                }
            }
        }
        let Ok((mut t, rig)) = roots.get_mut(e) else { continue };
        t.translation = v.pos;
        t.rotation = Quat::from_rotation_y(v.yaw);
        for (i, &w) in rig.wheels.iter().enumerate() {
            if let Ok(mut wt) = wheels.get_mut(w) {
                // las delanteras doblan (girar a la derecha baja el yaw)
                let steer = if i < 2 { -v.steer } else { 0.0 };
                wt.rotation = Quat::from_rotation_y(steer) * Quat::from_rotation_x(v.wheel);
            }
        }
    }
}

/// Motor: un loop que sube de tono con la velocidad; solo suena manejando.
fn engine(mut commands: Commands, game: Res<GameRes>, sounds: Option<Res<crate::audio::Sounds>>, mut q: Query<&mut AudioSink, With<EngineSound>>, mut made: Local<bool>, time: Res<Time>, mut vol: Local<f32>) {
    if !*made {
        if let Some(s) = sounds {
            if let Some(h) = s.engine() {
                commands.spawn((AudioPlayer(h), PlaybackSettings::LOOP.with_volume(Volume::Linear(0.0)), EngineSound));
                *made = true;
            }
        }
        return;
    }
    let g = &game.0;
    let speed = g.driving.and_then(|i| g.vehicles.get(i)).map(|v| v.speed.abs());
    let target = if speed.is_some() { 0.12 } else { 0.0 };
    *vol += (target - *vol) * (time.delta_secs() * 4.0).min(1.0);
    for mut sink in &mut q {
        sink.set_volume(Volume::Linear(*vol));
        sink.set_speed(0.8 + speed.unwrap_or(0.0) / 30.0 * 0.7);
    }
}
