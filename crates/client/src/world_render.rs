//! Convierte el `Level` de la simulación en entidades de Bevy: cajas fundidas por material y
//! por zona (así la cámara descarta lo que no ve), cilindros, árboles, piso, cielo y luces.

use crate::materials::Looks;
use arena_sim::maps::{Level, Sky};
use arena_sim::world::{CylAxis, VisualBox};
use arena_sim::{Mat, Vec3 as SVec3};
use bevy::asset::RenderAssetUsages;
use bevy::light::{CascadeShadowConfigBuilder, NotShadowCaster};
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use std::collections::BTreeMap;

/// Lado de las zonas en que se agrupan las cajas.
const CHUNK: f32 = 88.0;

#[derive(Resource)]
pub struct LevelRes(pub Level);

/// Todo lo que se borra al cambiar de mapa.
#[derive(Component)]
pub struct LevelEntity;

/// El domo de cielo y el sol siguen a la cámara.
#[derive(Component)]
pub struct SkyDome;

#[derive(Component)]
pub struct SunDisc(pub Vec3);

/// El sol del mapa (la luz que mueve el ciclo de día y noche).
#[derive(Component)]
pub struct WorldSun;

/// Grupo visual móvil (la antena), por índice en `World::groups`.
#[derive(Component)]
pub struct GroupVisual(pub usize);

#[derive(Default)]
struct MeshBuf {
    pos: Vec<[f32; 3]>,
    nor: Vec<[f32; 3]>,
    uv: Vec<[f32; 2]>,
    idx: Vec<u32>,
}

impl MeshBuf {
    fn push_box(&mut self, center: SVec3, size: SVec3, wuv: Option<f32>) {
        self.push_box_rot(center, size, 0.0, wuv);
    }

    /// Caja girada `yaw` sobre Y. La textura en metros se calcula sin girar (sigue a la caja).
    fn push_box_rot(&mut self, center: SVec3, size: SVec3, yaw: f32, wuv: Option<f32>) {
        let rot = Quat::from_rotation_y(yaw);
        let h = size * 0.5;
        // (normal, u, v) con u × v = normal: los vértices quedan antihorarios vistos desde afuera
        const FACES: [([f32; 3], [f32; 3], [f32; 3]); 6] = [
            ([1., 0., 0.], [0., 0., -1.], [0., 1., 0.]),
            ([-1., 0., 0.], [0., 0., 1.], [0., 1., 0.]),
            ([0., 0., 1.], [1., 0., 0.], [0., 1., 0.]),
            ([0., 0., -1.], [-1., 0., 0.], [0., 1., 0.]),
            ([0., 1., 0.], [1., 0., 0.], [0., 0., -1.]),
            ([0., -1., 0.], [1., 0., 0.], [0., 0., 1.]),
        ];
        for (n, u, v) in FACES {
            let (n, u, v) = (SVec3::from(n), SVec3::from(u), SVec3::from(v));
            let base = self.pos.len() as u32;
            for (su, sv, tu, tv) in [(-1.0, -1.0, 0.0, 1.0), (1.0, -1.0, 1.0, 1.0), (1.0, 1.0, 1.0, 0.0), (-1.0, 1.0, 0.0, 0.0)] {
                let p = center + n * (n.abs().dot(h)) + u * su * u.abs().dot(h) + v * sv * v.abs().dot(h);
                let world = center + rot * (p - center);
                self.pos.push(world.to_array());
                self.nor.push((rot * n).to_array());
                self.uv.push(match wuv {
                    // UV en metros: las fachadas repiten la textura por piso sin estirarse
                    Some(m) if n.x.abs() > 0.5 => [p.z / m, -p.y / m],
                    Some(m) if n.z.abs() > 0.5 => [p.x / m, -p.y / m],
                    Some(m) => [p.x / m, p.z / m],
                    None => [tu, tv],
                });
            }
            self.idx.extend([base, base + 1, base + 2, base, base + 2, base + 3]);
        }
    }

    fn into_mesh(self) -> Mesh {
        Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD)
            .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, self.pos)
            .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, self.nor)
            .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, self.uv)
            .with_inserted_indices(Indices::U32(self.idx))
    }
}

pub fn col(h: u32) -> Color {
    Color::srgb_u8((h >> 16) as u8, (h >> 8) as u8, h as u8)
}

/// Domo de cielo con UV v = 0 arriba (la fila de arriba de la textura queda en el cenit).
fn sky_mesh(r: f32) -> Mesh {
    let (seg, rings) = (32u32, 16u32);
    let mut b = MeshBuf::default();
    for i in 0..=rings {
        let v = i as f32 / rings as f32;
        let th = v * std::f32::consts::PI;
        for j in 0..=seg {
            let u = j as f32 / seg as f32;
            let ph = u * std::f32::consts::TAU;
            let p = SVec3::new(th.sin() * ph.cos(), th.cos(), th.sin() * ph.sin());
            b.pos.push((p * r).to_array());
            b.nor.push((-p).to_array());
            b.uv.push([u, v]);
        }
    }
    for i in 0..rings {
        for j in 0..seg {
            let (a, c) = (i * (seg + 1) + j, (i + 1) * (seg + 1) + j);
            b.idx.extend([a, a + 1, c, a + 1, c + 1, c]);
        }
    }
    b.into_mesh()
}

/// Resta un rectángulo [x0, z0, x1, z1] a otro: quedan hasta cuatro piezas alrededor.
fn cut(r: [f32; 4], h: &[f32; 4]) -> Vec<[f32; 4]> {
    if !(r[0] < h[2] && r[2] > h[0] && r[1] < h[3] && r[3] > h[1]) {
        return vec![r];
    }
    let mut out = Vec::new();
    // franjas de arriba y de abajo a todo el ancho, y los costados a la altura del hueco
    if r[1] < h[1] {
        out.push([r[0], r[1], r[2], h[1]]);
    }
    if r[3] > h[3] {
        out.push([r[0], h[3], r[2], r[3]]);
    }
    let (z0, z1) = (r[1].max(h[1]), r[3].min(h[3]));
    if r[0] < h[0] {
        out.push([r[0], z0, h[0], z1]);
    }
    if r[2] > h[2] {
        out.push([h[2], z0, r[2], z1]);
    }
    out
}

/// Piso en baldosas de 8 m; las que tocan un hueco (bocas de subte) se recortan justo alrededor
/// (antes se sacaba la baldosa entera y quedaba un agujero de 8 m).
/// Las UV son las de un plano único de 2a × 2a, para que la textura no cambie.
fn floor_mesh(a: f32, holes: &[[f32; 4]]) -> Mesh {
    let mut b = MeshBuf::default();
    let t = 8.0;
    let n = (a * 2.0 / t).ceil() as i32;
    for iz in 0..n {
        for ix in 0..n {
            let (x0, z0) = (-a + ix as f32 * t, -a + iz as f32 * t);
            let (x1, z1) = ((x0 + t).min(a), (z0 + t).min(a));
            let mut pieces = vec![[x0, z0, x1, z1]];
            for h in holes {
                pieces = pieces.iter().flat_map(|r| cut(*r, h)).collect();
            }
            for [x0, z0, x1, z1] in pieces {
                if x1 - x0 < 0.01 || z1 - z0 < 0.01 {
                    continue;
                }
                let base = b.pos.len() as u32;
                for (x, z) in [(x0, z0), (x0, z1), (x1, z1), (x1, z0)] {
                    b.pos.push([x, 0.0, z]);
                    b.nor.push([0.0, 1.0, 0.0]);
                    b.uv.push([(x + a) / (2.0 * a), (z + a) / (2.0 * a)]);
                }
                b.idx.extend([base, base + 1, base + 2, base, base + 2, base + 3]);
            }
        }
    }
    b.into_mesh()
}

pub fn spawn_level(commands: &mut Commands, meshes: &mut Assets<Mesh>, mats: &mut Assets<StandardMaterial>, looks: &mut Looks, level: &Level) {
    let w = &level.world;
    let info = &level.info;
    let a = info.a;

    // cajas: una malla por (material, sombra, zona)
    let mut batches: BTreeMap<(Mat, bool, i32, i32), MeshBuf> = BTreeMap::new();
    for vb in &w.visuals {
        let key = (vb.mat, vb.shadow, (vb.center.x / CHUNK).floor() as i32, (vb.center.z / CHUNK).floor() as i32);
        let wuv = looks.get(vb.mat, mats).wuv;
        batches.entry(key).or_default().push_box_rot(vb.center, vb.size, vb.yaw, wuv);
    }
    for ((m, shadow, _, _), buf) in batches {
        let look = looks.get(m, mats).mat.clone();
        let mut e = commands.spawn((Mesh3d(meshes.add(buf.into_mesh())), MeshMaterial3d(look), Transform::IDENTITY, LevelEntity));
        if !shadow {
            e.insert(NotShadowCaster);
        }
    }

    // grupos móviles (la antena)
    for (gi, g) in w.groups.iter().enumerate() {
        let mut by_mat: BTreeMap<Mat, MeshBuf> = BTreeMap::new();
        for VisualBox { center, size, mat, .. } in &g.boxes {
            by_mat.entry(*mat).or_default().push_box(*center, *size, None);
        }
        commands
            .spawn((Transform::from_translation(g.origin), Visibility::default(), GroupVisual(gi), LevelEntity))
            .with_children(|p| {
                for (m, buf) in by_mat {
                    p.spawn((Mesh3d(meshes.add(buf.into_mesh())), MeshMaterial3d(looks.get(m, mats).mat.clone())));
                }
            });
    }

    // cilindros: tanques, vagón cisterna y ruedas
    for cy in &w.cylinders {
        let mesh = meshes.add(Cylinder::new(cy.radius, cy.length).mesh().resolution(24));
        let rot = match cy.axis {
            CylAxis::Y => Quat::IDENTITY,
            CylAxis::X => Quat::from_rotation_z(std::f32::consts::FRAC_PI_2),
            CylAxis::Z => Quat::from_rotation_x(std::f32::consts::FRAC_PI_2),
        };
        commands.spawn((Mesh3d(mesh), MeshMaterial3d(looks.get(cy.mat, mats).mat.clone()), Transform::from_translation(cy.center).with_rotation(rot), LevelEntity));
    }

    // skyline girado de la azotea
    let unit = meshes.add(Cuboid::new(1.0, 1.0, 1.0));
    for rb in &w.rot_boxes {
        commands.spawn((
            Mesh3d(unit.clone()),
            MeshMaterial3d(looks.get(rb.mat, mats).mat.clone()),
            Transform::from_translation(rb.center).with_rotation(Quat::from_rotation_y(rb.yaw)).with_scale(rb.size),
            NotShadowCaster,
            LevelEntity,
        ));
    }

    // copas de árboles: fundidas en una malla por zona (cada entidad le cuesta CPU a Bevy)
    if !w.trees.is_empty() {
        let mut ico = Sphere::new(1.0).mesh().ico(1).expect("icosfera");
        ico.duplicate_vertices();
        ico.compute_flat_normals();
        let mut zones: BTreeMap<(i32, i32), Mesh> = BTreeMap::new();
        for [x, y, z, s] in &w.trees {
            let m = ico.clone().transformed_by(Transform::from_xyz(*x, *y, *z).with_scale(Vec3::new(*s, s * 0.9, *s)));
            let key = ((x / CHUNK).floor() as i32, (z / CHUNK).floor() as i32);
            match zones.get_mut(&key) {
                Some(acc) => {
                    let _ = acc.merge(&m);
                }
                None => {
                    zones.insert(key, m);
                }
            }
        }
        for (_, m) in zones {
            commands.spawn((Mesh3d(meshes.add(m)), MeshMaterial3d(looks.foliage.clone()), Transform::IDENTITY, LevelEntity));
        }
    }

    // piso
    let floor = looks.floor(info.floor_tex, info.floor, a, mats);
    commands.spawn((Mesh3d(meshes.add(floor_mesh(a, &w.holes))), MeshMaterial3d(floor), Transform::IDENTITY, NotShadowCaster, LevelEntity));

    // cielo y sol
    let sun_dir = level.sun_pos().normalize();
    if info.sky != Sky::Flat {
        let city = info.sky == Sky::City;
        let r = if city { 330.0 } else { 190.0 };
        let sky = mats.add(StandardMaterial { base_color_texture: Some(looks.sky(city)), unlit: true, fog_enabled: false, cull_mode: None, ..default() });
        commands.spawn((Mesh3d(meshes.add(sky_mesh(r))), MeshMaterial3d(sky), Transform::IDENTITY, NotShadowCaster, SkyDome, LevelEntity));
        let disc = mats.add(StandardMaterial { base_color: col(if city { 0xfff1d0 } else { 0xfff4d6 }), unlit: true, fog_enabled: false, ..default() });
        let (dr, dd) = if city { (10.0, 320.0) } else { (7.0, 180.0) };
        commands.spawn((Mesh3d(meshes.add(Sphere::new(dr).mesh().uv(16, 12))), MeshMaterial3d(disc), Transform::from_translation(sun_dir * dd), NotShadowCaster, SunDisc(sun_dir * dd), LevelEntity));
    }

    // luces. La web usaba luces "de artista" de three.js r128; con la exposición por defecto de
    // Bevy (EV100 9,7) una intensidad 1 equivale a ~3000 lux directos y ~1000 cd/m² de ambiente.
    let (sun_c, sun_i) = info.sun;
    let cascades = CascadeShadowConfigBuilder { num_cascades: 4, first_cascade_far_bound: 12.0, maximum_distance: if info.city { 160.0 } else { a * 2.5 }, ..default() }.build();
    commands.spawn((
        DirectionalLight { color: col(sun_c), illuminance: sun_i * 3000.0, shadow_maps_enabled: !crate::off("shadows"), ..default() },
        Transform::from_translation(level.sun_pos()).looking_at(Vec3::ZERO, Vec3::Y),
        cascades,
        WorldSun,
        LevelEntity,
    ));
    let (sky_c, ground_c, hemi_i) = info.hemi;
    let (s, g) = (col(sky_c).to_linear(), col(ground_c).to_linear());
    let mix = LinearRgba::rgb(s.red * 0.6 + g.red * 0.4, s.green * 0.6 + g.green * 0.4, s.blue * 0.6 + g.blue * 0.4);
    commands.insert_resource(GlobalAmbientLight { color: mix.into(), brightness: hemi_i * 1000.0, ..default() });
    commands.insert_resource(ClearColor(col(info.bg)));
}

/// El cielo se queda centrado en la cámara para no recortarse nunca.
pub fn follow_sky(cam: Query<&GlobalTransform, With<Camera3d>>, mut sky: Query<&mut Transform, (With<SkyDome>, Without<SunDisc>)>, mut sun: Query<(&mut Transform, &SunDisc), Without<SkyDome>>) {
    let Some(cam) = cam.iter().next() else { return };
    let p = cam.translation();
    for mut t in &mut sky {
        t.translation = p;
    }
    for (mut t, d) in &mut sun {
        t.translation = p + d.0;
    }
}

/// Metal fundido que corre despacio.
pub fn animate_lava(time: Res<Time>, looks: Res<Looks>, mut mats: ResMut<Assets<StandardMaterial>>) {
    if let Some(mut m) = mats.get_mut(&looks.lava) {
        let t = time.elapsed_secs();
        m.uv_transform.translation = Vec2::new(t * 0.02, t * 0.012);
    }
}
