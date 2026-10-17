//! Peatones: figuras simples con ropa de colores que caminan (piernas y brazos que se balancean),
//! corren cuando huyen y quedan en el piso si los hieren. Solo se dibujan los cercanos.

use crate::game::GameRes;
use crate::player::LocalPlayer;
use bevy::light::NotShadowCaster;
use bevy::prelude::*;
use std::collections::HashMap;

const SHIRT: [u32; 6] = [0x3d6fa8, 0xb8483e, 0xd8c070, 0x4f8a5a, 0x7a5a9a, 0xe0e0d8];
const PANTS: [u32; 3] = [0x2a2f3a, 0x4a3a2a, 0x3a4a5a];

pub struct PeopleViewPlugin;

impl Plugin for PeopleViewPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, sync);
    }
}

#[derive(Component)]
struct PersonRig {
    legs: [Entity; 2],
    arms: [Entity; 2],
}

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
fn sync(
    mut commands: Commands,
    time: Res<Time>,
    game: Res<GameRes>,
    lp: Res<LocalPlayer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut mats: ResMut<Assets<StandardMaterial>>,
    mut assets: Local<Option<(Handle<Mesh>, Vec<Handle<StandardMaterial>>, Vec<Handle<StandardMaterial>>, Handle<StandardMaterial>)>>,
    mut known: Local<HashMap<u32, Entity>>,
    mut seen_gen: Local<u32>,
    mut roots: Query<(&mut Transform, &mut Visibility, &PersonRig)>,
    mut parts: Query<&mut Transform, Without<PersonRig>>,
) {
    if *seen_gen != game.1 {
        *seen_gen = game.1;
        for (_, e) in known.drain() {
            commands.entity(e).despawn();
        }
    }
    let (cube, shirts, pants, skin) = assets.get_or_insert_with(|| {
        let m = |c: u32| StandardMaterial { base_color: crate::world_render::col(c), perceptual_roughness: 0.9, ..default() };
        (meshes.add(Cuboid::new(1.0, 1.0, 1.0)), SHIRT.iter().map(|&c| mats.add(m(c))).collect(), PANTS.iter().map(|&c| mats.add(m(c))).collect(), mats.add(m(0xc8a07c)))
    });
    let me = lp.state.pos;
    let t = time.elapsed_secs();
    for p in &game.0.people {
        let near = p.pos.distance(me) < 90.0;
        if !known.contains_key(&p.id) {
            if !near {
                continue;
            }
            let shirt = shirts[p.color as usize % shirts.len()].clone();
            let pant = pants[p.color as usize % pants.len()].clone();
            let root = commands.spawn((Transform::from_translation(p.pos), Visibility::default())).id();
            let piece = |commands: &mut Commands, m: Handle<StandardMaterial>, pos: Vec3, size: Vec3, parent: Entity| {
                let e = commands.spawn((Mesh3d(cube.clone()), MeshMaterial3d(m), Transform::from_translation(pos).with_scale(size), NotShadowCaster)).id();
                commands.entity(parent).add_child(e);
                e
            };
            piece(&mut commands, shirt.clone(), Vec3::new(0.0, 1.2, 0.0), Vec3::new(0.42, 0.55, 0.24), root);
            piece(&mut commands, skin.clone(), Vec3::new(0.0, 1.62, 0.0), Vec3::splat(0.24), root);
            let mut legs = [Entity::PLACEHOLDER; 2];
            let mut arms = [Entity::PLACEHOLDER; 2];
            for (i, x) in [-0.11f32, 0.11].into_iter().enumerate() {
                let hip = commands.spawn((Transform::from_xyz(x, 0.92, 0.0), Visibility::default())).id();
                commands.entity(root).add_child(hip);
                piece(&mut commands, pant.clone(), Vec3::new(0.0, -0.45, 0.0), Vec3::new(0.16, 0.9, 0.18), hip);
                legs[i] = hip;
                let sh = commands.spawn((Transform::from_xyz(x * 2.4, 1.44, 0.0), Visibility::default())).id();
                commands.entity(root).add_child(sh);
                piece(&mut commands, shirt.clone(), Vec3::new(0.0, -0.28, 0.0), Vec3::new(0.12, 0.56, 0.14), sh);
                arms[i] = sh;
            }
            commands.entity(root).insert(PersonRig { legs, arms });
            known.insert(p.id, root);
            continue;
        }
        let Some(&e) = known.get(&p.id) else { continue };
        let Ok((mut tr, mut vis, rig)) = roots.get_mut(e) else { continue };
        *vis = if near { Visibility::Inherited } else { Visibility::Hidden };
        if !near {
            continue;
        }
        tr.translation = p.pos;
        if p.alive {
            tr.rotation = Quat::from_rotation_y(p.yaw);
            let rate = if p.flee > 0.0 { 13.0 } else { 7.0 };
            let amp = if p.moving { if p.flee > 0.0 { 0.9 } else { 0.5 } } else { 0.0 };
            let sw = (t * rate + p.phase).sin() * amp;
            for (i, &l) in rig.legs.iter().enumerate() {
                if let Ok(mut lt) = parts.get_mut(l) {
                    lt.rotation = Quat::from_rotation_x(if i == 0 { sw } else { -sw });
                }
            }
            for (i, &a) in rig.arms.iter().enumerate() {
                if let Ok(mut at) = parts.get_mut(a) {
                    // huyendo, con las manos arriba
                    at.rotation = if p.flee > 0.0 { Quat::from_rotation_x(-2.6 + sw * 0.2) } else { Quat::from_rotation_x(if i == 0 { -sw } else { sw }) };
                }
            }
        } else {
            tr.rotation = Quat::from_rotation_y(p.yaw) * Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2);
            tr.translation.y = p.pos.y + 0.15;
        }
    }
}
