//! Efectos: partículas (`11-particulas`), trazadoras (`12-trazadoras`), luz de explosiones y
//! granadas en vuelo. Los sistemas de juego encolan pedidos en `Fx` y acá se dibujan.

use crate::game::GameRes;
use bevy::light::NotShadowCaster;
use bevy::prelude::*;
use std::collections::HashMap;

const MAXP: usize = 600;

pub struct FxPlugin;

impl Plugin for FxPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Fx>().add_systems(Startup, setup_fx).add_systems(Update, (spawn_bursts, update_particles, draw_tracers, update_boom, sync_nades).chain());
    }
}

struct Burst {
    pos: Vec3,
    color: u32,
    n: u32,
    speed: f32,
    life: f32,
}

struct Tracer {
    a: Vec3,
    b: Vec3,
    color: Color,
    life: f32,
}

#[derive(Resource, Default)]
pub struct Fx {
    bursts: Vec<Burst>,
    tracers: Vec<Tracer>,
    boom: Option<(Vec3, f32)>,
    rng: u32,
}

impl Fx {
    pub fn burst(&mut self, pos: Vec3, color: u32, n: u32, speed: f32, life: f32) {
        self.bursts.push(Burst { pos, color, n, speed, life });
    }
    pub fn tracer(&mut self, a: Vec3, b: Vec3, color: u32, life: f32) {
        if self.tracers.len() >= 16 {
            self.tracers.remove(0);
        }
        self.tracers.push(Tracer { a, b, color: crate::world_render::col(color), life });
    }
    /// Destello de explosión: `big` es una granada, si no un kamikaze.
    pub fn boom(&mut self, pos: Vec3, big: bool) {
        self.boom = Some((pos, if big { 6.0 } else { 3.0 }));
    }
    fn rand(&mut self) -> f32 {
        self.rng = self.rng.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        (self.rng >> 8) as f32 / 16_777_216.0
    }
}

#[derive(Component)]
struct Particle {
    vel: Vec3,
    life: f32,
    max: f32,
}

#[derive(Component)]
struct BoomLight {
    t: f32,
    peak: f32,
}

#[derive(Component)]
struct NadeVis(usize);

#[derive(Resource)]
struct FxAssets {
    cube: Handle<Mesh>,
    mats: HashMap<u32, Handle<StandardMaterial>>,
}

fn setup_fx(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>, mut mats: ResMut<Assets<StandardMaterial>>) {
    commands.insert_resource(FxAssets { cube: meshes.add(Cuboid::new(0.09, 0.09, 0.09)), mats: HashMap::new() });
    // luz de explosiones: siempre presente (intensidad 0) para no recompilar shaders
    commands.spawn((PointLight { intensity: 0.0, range: 18.0, color: Color::srgb_u8(0xff, 0xa0, 0x40), ..default() }, Transform::default(), BoomLight { t: 0.0, peak: 0.0 }));
    // granadas en vuelo: un grupo fijo que se reubica cada cuadro
    let body = meshes.add(Sphere::new(0.13).mesh().uv(10, 8));
    let ring = meshes.add(Torus::new(0.115, 0.165).mesh().minor_resolution(6).major_resolution(16));
    let mb = mats.add(StandardMaterial { base_color: crate::world_render::col(0x1c2125), metallic: 0.6, perceptual_roughness: 0.4, ..default() });
    let ml = mats.add(StandardMaterial { base_color: crate::world_render::col(0xffb02e), unlit: true, ..default() });
    for i in 0..16 {
        commands.spawn((Mesh3d(body.clone()), MeshMaterial3d(mb.clone()), Transform::default(), Visibility::Hidden, NadeVis(i), NotShadowCaster)).with_child((Mesh3d(ring.clone()), MeshMaterial3d(ml.clone())));
    }
}

/// Partículas en un pool fijo de `MAXP` entidades ocultas que se reciclan (antes cada partícula
/// era una entidad nueva que se creaba y se borraba: cientos por explosión).
fn spawn_bursts(
    mut commands: Commands,
    mut fx: ResMut<Fx>,
    mut assets: ResMut<FxAssets>,
    mut mats: ResMut<Assets<StandardMaterial>>,
    mut pool: Local<Vec<Entity>>,
    mut next: Local<usize>,
    mut q: Query<(&mut Transform, &mut Particle, &mut Visibility, &mut MeshMaterial3d<StandardMaterial>)>,
) {
    if pool.is_empty() {
        let mat = assets.mats.entry(0xffffff).or_insert_with(|| mats.add(StandardMaterial { base_color: Color::WHITE, unlit: true, ..default() })).clone();
        for _ in 0..MAXP {
            pool.push(commands.spawn((Mesh3d(assets.cube.clone()), MeshMaterial3d(mat.clone()), Transform::default(), Visibility::Hidden, Particle { vel: Vec3::ZERO, life: 0.0, max: 1.0 }, NotShadowCaster)).id());
        }
        return;
    }
    let bursts = std::mem::take(&mut fx.bursts);
    for b in bursts {
        let mat = assets.mats.entry(b.color).or_insert_with(|| mats.add(StandardMaterial { base_color: crate::world_render::col(b.color), unlit: true, ..default() })).clone();
        for _ in 0..b.n {
            // busca un lugar libre; si están todos ocupados, pisa al más viejo de la ronda
            let mut slot = *next % MAXP;
            for k in 0..MAXP {
                let i = (*next + k) % MAXP;
                if q.get(pool[i]).is_ok_and(|(_, p, _, _)| p.life <= 0.0) {
                    slot = i;
                    break;
                }
            }
            *next = slot + 1;
            let v = Vec3::new(fx.rand() - 0.5, fx.rand() - 0.2, fx.rand() - 0.5);
            let s = b.speed * (0.4 + fx.rand() * 0.8) / v.length().max(1e-3);
            let life = b.life * (0.6 + fx.rand() * 0.6);
            if let Ok((mut t, mut p, mut vis, mut m)) = q.get_mut(pool[slot]) {
                *t = Transform::from_translation(b.pos);
                *p = Particle { vel: v * s, life, max: life };
                *vis = Visibility::Inherited;
                if m.0 != mat {
                    m.0 = mat.clone();
                }
            }
        }
    }
}

fn update_particles(time: Res<Time>, mut q: Query<(&mut Transform, &mut Particle, &mut Visibility)>) {
    let dt = time.delta_secs();
    for (mut t, mut p, mut vis) in &mut q {
        if p.life <= 0.0 {
            continue;
        }
        p.life -= dt;
        if p.life <= 0.0 {
            *vis = Visibility::Hidden;
            continue;
        }
        p.vel.y -= 14.0 * dt;
        t.translation += p.vel * dt;
        if t.translation.y < 0.05 {
            t.translation.y = 0.05;
            p.vel.y *= -0.3;
            p.vel.x *= 0.6;
            p.vel.z *= 0.6;
        }
        t.scale = Vec3::splat((p.life / p.max).max(0.01));
    }
}

fn draw_tracers(time: Res<Time>, mut fx: ResMut<Fx>, mut gizmos: Gizmos) {
    let dt = time.delta_secs();
    for t in &mut fx.tracers {
        gizmos.line(t.a, t.b, t.color);
        t.life -= dt;
    }
    fx.tracers.retain(|t| t.life > 0.0);
}

fn update_boom(time: Res<Time>, mut fx: ResMut<Fx>, mut q: Query<(&mut PointLight, &mut Transform, &mut BoomLight)>) {
    let Ok((mut l, mut t, mut b)) = q.single_mut() else { return };
    if let Some((pos, i)) = fx.boom.take() {
        t.translation = pos;
        b.t = 0.25;
        b.peak = i * 150_000.0;
    }
    if b.t > 0.0 {
        b.t -= time.delta_secs();
        l.intensity = (b.t / 0.25).max(0.0) * b.peak;
    } else {
        l.intensity = 0.0;
    }
}

fn sync_nades(game: Res<GameRes>, time: Res<Time>, mut q: Query<(&NadeVis, &mut Transform, &mut Visibility)>) {
    for (n, mut t, mut v) in &mut q {
        match game.0.nades.get(n.0) {
            Some(nd) => {
                t.translation = nd.pos;
                t.rotation = Quat::from_rotation_x(time.elapsed_secs() * nd.vel.z * 3.0) * Quat::from_rotation_z(-time.elapsed_secs() * nd.vel.x * 3.0);
                t.scale = Vec3::splat(if nd.impact { 0.8 } else { 1.0 });
                *v = Visibility::Inherited;
            }
            None => *v = Visibility::Hidden,
        }
    }
}
