//! Peatones: personas con esqueleto (ver `humanoid.rs`) y ropa propia de cada uno (piel, ropa,
//! pelo, gorra, sobretodo, pollera, cartera, mochila), siempre la misma para el mismo peatón.
//! Caminan, corren con las manos arriba cuando huyen y quedan tirados si los hieren.
//! Solo se dibujan los cercanos.

use crate::game::GameRes;
use crate::humanoid::{HumanAssets, Pose, Skel, build, civilian, pose};
use crate::player::LocalPlayer;
use bevy::prelude::*;
use std::collections::HashMap;

pub struct PeopleViewPlugin;

impl Plugin for PeopleViewPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, sync);
    }
}

#[derive(Component)]
struct PersonRig {
    skel: Skel,
    walk: f32,
}

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
fn sync(
    mut commands: Commands,
    time: Res<Time>,
    game: Res<GameRes>,
    lp: Res<LocalPlayer>,
    ha: Res<HumanAssets>,
    mut known: Local<HashMap<u32, Entity>>,
    mut seen_gen: Local<u32>,
    mut roots: Query<(&mut Transform, &mut Visibility, &mut PersonRig)>,
    mut parts: Query<&mut Transform, Without<PersonRig>>,
) {
    if *seen_gen != game.1 {
        *seen_gen = game.1;
        for (_, e) in known.drain() {
            commands.entity(e).despawn();
        }
    }
    let me = lp.state.pos;
    let t = time.elapsed_secs();
    let dt = time.delta_secs();
    for p in &game.0.people {
        let d = p.pos.distance(me);
        let near = d < 90.0;
        if !known.contains_key(&p.id) {
            if !near {
                continue;
            }
            let root = commands.spawn((Transform::from_translation(p.pos), Visibility::default())).id();
            let o = civilian(&ha, p.id);
            let skel = build(&mut commands, &ha, root, &o);
            commands.entity(root).insert(PersonRig { skel, walk: p.phase });
            known.insert(p.id, root);
            continue;
        }
        let Some(&e) = known.get(&p.id) else { continue };
        let Ok((mut tr, mut vis, mut rig)) = roots.get_mut(e) else { continue };
        *vis = if near { Visibility::Inherited } else { Visibility::Hidden };
        if !near {
            continue;
        }
        tr.translation = p.pos;
        if p.alive {
            tr.rotation = Quat::from_rotation_y(p.yaw);
            let fleeing = p.flee > 0.0;
            let gait: f32 = if p.moving { if fleeing { 2.0 } else { 1.0 } } else { 0.0 };
            rig.walk += dt * if fleeing { 11.0 } else { 6.0 + p.speed } * gait.min(1.0);
            // lejos se anima menos seguido (no se nota y ahorra)
            if d < 45.0 || (t * 10.0) as u32 % 3 == p.id % 3 {
                let pz = Pose { phase: rig.walk, gait, t: t + p.phase, hands_up: if fleeing { 0.7 } else { 0.0 }, ..Default::default() };
                pose(&rig.skel, &pz, &mut parts);
            }
        } else {
            tr.rotation = Quat::from_rotation_y(p.yaw) * Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2);
            tr.translation.y = p.pos.y + 0.15;
            pose(&rig.skel, &Pose { dead: true, ..Default::default() }, &mut parts);
        }
    }
}
