//! Los otros jugadores online: figuras con esqueleto (`humanoid.rs`) con chaleco y casco del
//! color de cada uno, que caminan, corren, se agachan y apuntan. Se mueven suavizados entre fotos
//! del servidor (llegan unas 20 por segundo).

use crate::humanoid::{ARMY, HumanAssets, Outfit, Pose, SKIN, Skel, TEAM, build, pose};
use crate::online::Online;
use arena_sim::player::EYE;
use bevy::prelude::*;
use std::collections::HashMap;

pub struct PlayersViewPlugin;

impl Plugin for PlayersViewPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, sync);
    }
}

#[derive(Component)]
struct Mate {
    skel: Skel,
    walk: f32,
    yaw: f32,
}

fn outfit(a: &HumanAssets, id: u8) -> Outfit {
    let team = a.m(TEAM[id as usize % TEAM.len()]);
    Outfit {
        skin: a.m(SKIN[(id as usize * 2 + 1) % SKIN.len()]),
        top: a.m(ARMY[3]),
        bottom: a.m(ARMY[1]),
        shoes: a.m(ARMY[5]),
        hair: None,
        long_hair: false,
        helmet: Some((team.clone(), a.m(ARMY[0]))),
        vest: Some((a.m(ARMY[0]), team)),
        pack: Some(a.m(ARMY[2])),
        hood: None,
        cap: None,
        coat: None,
        skirt: false,
        bag: None,
        gun: Some((a.m(ARMY[5]), 0.8, true)),
        build: 1.02,
    }
}

#[allow(clippy::type_complexity)]
fn sync(
    mut commands: Commands,
    time: Res<Time>,
    net: Res<Online>,
    ha: Res<HumanAssets>,
    mut known: Local<HashMap<u8, Entity>>,
    mut roots: Query<(&mut Transform, &mut Visibility, &mut Mate)>,
    mut parts: Query<&mut Transform, Without<Mate>>,
) {
    let dt = time.delta_secs();
    let t = time.elapsed_secs();
    // los que se fueron
    known.retain(|id, e| {
        let keep = net.others.iter().any(|o| o.id == *id);
        if !keep {
            commands.entity(*e).despawn();
        }
        keep
    });
    for o in &net.others {
        let feet = o.pos - Vec3::Y * EYE;
        let Some(&e) = known.get(&o.id) else {
            let root = commands.spawn((Transform::from_translation(feet), Visibility::default())).id();
            let skel = build(&mut commands, &ha, root, &outfit(&ha, o.id));
            commands.entity(root).insert(Mate { skel, walk: 0.0, yaw: o.yaw });
            known.insert(o.id, root);
            continue;
        };
        let Ok((mut tr, mut vis, mut m)) = roots.get_mut(e) else { continue };
        if !o.alive {
            // caído: tirado en el piso hasta que reaparezca
            tr.translation = feet + Vec3::Y * 0.15;
            tr.rotation = Quat::from_rotation_y(o.yaw) * Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2);
            pose(&m.skel, &Pose { dead: true, ..Default::default() }, &mut parts);
            continue;
        }
        *vis = Visibility::Inherited;
        // suavizado: se acerca rápido a la última posición conocida (si saltó lejos, se teletransporta)
        if tr.translation.distance(feet) > 6.0 {
            tr.translation = feet;
        } else {
            tr.translation = tr.translation.lerp(feet, (dt * 14.0).min(1.0));
        }
        // el yaw del jugador mira hacia −Z con 0; la figura mira hacia +Z
        let d = (o.yaw - m.yaw + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
        m.yaw += d * (dt * 14.0).min(1.0);
        tr.rotation = Quat::from_rotation_y(m.yaw + std::f32::consts::PI);
        let gait = if o.speed < 0.5 { 0.0 } else if o.speed > 8.5 { 2.0 } else { 1.0 };
        m.walk += dt * (4.0 + o.speed * 0.9) * f32::min(gait, 1.0);
        let pz = Pose { phase: m.walk, gait, crouch: o.crouch, aim: if o.aim { 1.0 } else { 0.4 }, pitch: o.pitch, t: t + f32::from(o.id), armed: true, ..Default::default() };
        pose(&m.skel, &pz, &mut parts);
    }
}
