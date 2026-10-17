//! Marcas de impacto (nuevo): cada bala del jugador que pega en una pared o en el piso deja un
//! agujero oscuro con un halo de polvo. Pool fijo de 64 que se recicla del más viejo al más nuevo.

use crate::game::GameRes;
use bevy::prelude::*;

const POOL: usize = 64;

pub struct DecalsPlugin;

impl Plugin for DecalsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Decals>().add_systems(Update, place);
    }
}

/// Impactos nuevos que todavía no se pusieron.
#[derive(Resource, Default)]
pub struct Decals {
    pending: Vec<(Vec3, Vec3)>,
}

impl Decals {
    pub fn add(&mut self, pos: Vec3, normal: Vec3) {
        if self.pending.len() < 32 {
            self.pending.push((pos, normal));
        }
    }
}

#[derive(Component)]
struct Mark;

#[allow(clippy::too_many_arguments)]
fn place(
    mut commands: Commands,
    mut decals: ResMut<Decals>,
    game: Res<GameRes>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut mats: ResMut<Assets<StandardMaterial>>,
    mut pool: Local<Vec<Entity>>,
    mut next: Local<usize>,
    mut seen_gen: Local<u32>,
    mut seed: Local<u32>,
    mut q: Query<(&mut Transform, &mut Visibility), With<Mark>>,
) {
    if pool.is_empty() {
        let mesh = meshes.add(Plane3d::new(Vec3::Y, Vec2::splat(0.5)).mesh());
        let hole = mats.add(StandardMaterial { base_color: Color::srgba(0.03, 0.03, 0.03, 0.85), alpha_mode: AlphaMode::Blend, unlit: true, ..default() });
        let dust = mats.add(StandardMaterial { base_color: Color::srgba(0.35, 0.33, 0.3, 0.35), alpha_mode: AlphaMode::Blend, unlit: true, ..default() });
        for i in 0..POOL {
            // uno de cada dos con halo de polvo más grande abajo
            let m = commands
                .spawn((Mesh3d(mesh.clone()), MeshMaterial3d(hole.clone()), Transform::default(), Visibility::Hidden, Mark, bevy::light::NotShadowCaster, crate::world_render::LevelEntity))
                .with_children(|p| {
                    if i % 2 == 0 {
                        p.spawn((Mesh3d(mesh.clone()), MeshMaterial3d(dust.clone()), Transform::from_xyz(0.0, -0.002, 0.0).with_scale(Vec3::splat(2.2)), bevy::light::NotShadowCaster));
                    }
                })
                .id();
            pool.push(m);
        }
    }
    // partida nueva: se borran todas
    if *seen_gen != game.1 {
        *seen_gen = game.1;
        for e in pool.iter() {
            if let Ok((_, mut v)) = q.get_mut(*e) {
                *v = Visibility::Hidden;
            }
        }
    }
    for (pos, n) in decals.pending.drain(..) {
        let e = pool[*next % POOL];
        *next += 1;
        *seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
        let r = (*seed >> 8) as f32 / (1u32 << 24) as f32;
        if let Ok((mut t, mut v)) = q.get_mut(e) {
            let rot = Quat::from_rotation_arc(Vec3::Y, n.normalize_or(Vec3::Y)) * Quat::from_rotation_y(r * std::f32::consts::TAU);
            *t = Transform::from_translation(pos + n * (0.012 + r * 0.004)).with_rotation(rot).with_scale(Vec3::splat(0.12 + r * 0.08));
            *v = Visibility::Inherited;
        }
    }
}
