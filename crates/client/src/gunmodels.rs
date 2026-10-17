//! Modelos de las armas en primera persona (puerto de `05-viewmodel...js` y
//! `06-materiales-extra-y-utilidades.js`). Igual que en la web, no hay archivos de modelos:
//! cada arma se arma con cajas, cilindros, toros y perfiles laterales extruidos.
//!
//! Convención de la web: +X derecha, +Y arriba, −Z hacia adelante (el caño apunta a −Z).
//! Los perfiles `ex` se dibujan en (u = hacia adelante, v = arriba) y se extruyen a lo ancho.

use crate::textures::hex;
use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use std::f32::consts::{FRAC_PI_2, PI, TAU};

pub const GUN_LAYER: usize = 1;

/// Materiales del arma (`gm` en la web).
pub struct Gm {
    pub metal: Handle<StandardMaterial>,
    pub steel: Handle<StandardMaterial>,
    pub black: Handle<StandardMaterial>,
    pub fde: Handle<StandardMaterial>,
    pub rubber: Handle<StandardMaterial>,
    pub perf: Handle<StandardMaterial>,
    pub dark: Handle<StandardMaterial>,
    pub poly: Handle<StandardMaterial>,
    pub brass: Handle<StandardMaterial>,
    pub shell: Handle<StandardMaterial>,
    pub glass: Handle<StandardMaterial>,
    pub amber: Handle<StandardMaterial>,
    pub red: Handle<StandardMaterial>,
    /// Se aclara y oscurece con la carga del riel.
    pub cyan: Handle<StandardMaterial>,
    pub reticle: Handle<StandardMaterial>,
}

fn pbr(c: u32, metal: f32, rough: f32) -> StandardMaterial {
    StandardMaterial { base_color: crate::world_render::col(c), metallic: metal, perceptual_roughness: rough, ..default() }
}
fn flat(c: u32) -> StandardMaterial {
    StandardMaterial { base_color: crate::world_render::col(c), unlit: true, ..default() }
}

impl Gm {
    pub fn new(mats: &mut Assets<StandardMaterial>, images: &mut Assets<Image>) -> Self {
        Self {
            metal: mats.add(pbr(0x353c42, 0.85, 0.55)),
            steel: mats.add(pbr(0x5b646b, 1.0, 0.32)),
            black: mats.add(pbr(0x07090a, 0.3, 0.8)),
            fde: mats.add(pbr(0x5c4b37, 0.05, 0.72)),
            rubber: mats.add(pbr(0x111314, 0.0, 0.95)),
            perf: mats.add(StandardMaterial { double_sided: true, cull_mode: None, ..pbr(0x2a3035, 0.8, 0.45) }),
            dark: mats.add(StandardMaterial { double_sided: true, cull_mode: None, ..pbr(0x15181b, 0.7, 0.45) }),
            poly: mats.add(pbr(0x1c2023, 0.05, 0.75)),
            brass: mats.add(pbr(0xb8913a, 1.0, 0.3)),
            shell: mats.add(pbr(0x9a1e24, 0.2, 0.5)),
            glass: mats.add(StandardMaterial { alpha_mode: AlphaMode::Blend, base_color: crate::world_render::col(0x9fe9ff).with_alpha(0.16), metallic: 1.0, perceptual_roughness: 0.0, ..default() }),
            amber: mats.add(flat(0xffb02e)),
            red: mats.add(flat(0xff3d5a)),
            cyan: mats.add(flat(0x5fd8ff)),
            reticle: mats.add(StandardMaterial { base_color_texture: Some(images.add(reticle_tex())), unlit: true, alpha_mode: AlphaMode::Add, ..default() }),
        }
    }
}

/// Retícula roja de la mira holográfica.
fn reticle_tex() -> Image {
    let s = 128;
    let mut px = vec![[0.0f32; 4]; s * s];
    let c = (1.0, 0.24, 0.35);
    for y in 0..s {
        for x in 0..s {
            let (dx, dy) = (x as f32 + 0.5 - 64.0, y as f32 + 0.5 - 64.0);
            let d = dx.hypot(dy);
            let ring = (1.0 - ((d - 44.0).abs() - 2.0).max(0.0) / 3.0).clamp(0.0, 1.0);
            let dot = (1.0 - (d - 5.0).max(0.0) / 2.0).clamp(0.0, 1.0);
            let tick = if (dx.abs() < 2.0 && (dy.abs() > 46.0 && dy.abs() < 60.0)) || (dy.abs() < 2.0 && (dx.abs() > 46.0 && dx.abs() < 60.0)) { 1.0 } else { 0.0 };
            let a = ring.max(dot).max(tick);
            px[y * s + x] = [c.0 * a, c.1 * a, c.2 * a, a];
        }
    }
    crate::textures::to_image(s, px, true, false)
}

/// Cubemap de un depósito con tiras de luz, para los reflejos del metal (como `makeEnv` de la web).
pub fn env_cubemap() -> Image {
    use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureViewDescriptor, TextureViewDimension};
    let s = 64usize;
    let stops = [(0.0, hex(0xd9e4ea)), (0.3, hex(0x55636c)), (0.55, hex(0x20282d)), (1.0, hex(0x0a0d0f))];
    let mut data = Vec::with_capacity(s * s * 6 * 4);
    // orden de caras: +X, −X, +Y, −Y, +Z, −Z
    for face in 0..6 {
        for y in 0..s {
            for x in 0..s {
                let (a, b) = ((x as f32 + 0.5) / s as f32 * 2.0 - 1.0, (y as f32 + 0.5) / s as f32 * 2.0 - 1.0);
                let d = match face {
                    0 => Vec3::new(1.0, -b, -a),
                    1 => Vec3::new(-1.0, -b, a),
                    2 => Vec3::new(a, 1.0, b),
                    3 => Vec3::new(a, -1.0, -b),
                    4 => Vec3::new(a, -b, 1.0),
                    _ => Vec3::new(-a, -b, -1.0),
                }
                .normalize();
                let t = d.y.clamp(-1.0, 1.0).acos() / PI;
                let az = d.z.atan2(d.x) / TAU + 0.5;
                let mut c = sample(&stops, t);
                if (t - 0.19).abs() < 0.012 && (az * 4.0).fract() < 0.53 {
                    c = hex(0xffd08a);
                }
                if (t - 0.52).abs() < 0.004 {
                    c = hex(0xffb02e);
                }
                let e = |v: f32| (v.clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
                data.extend_from_slice(&[e(c.0), e(c.1), e(c.2), 255]);
            }
        }
    }
    let mut img = Image::new(Extent3d { width: s as u32, height: s as u32, depth_or_array_layers: 6 }, TextureDimension::D2, data, TextureFormat::Rgba8UnormSrgb, RenderAssetUsages::RENDER_WORLD);
    img.texture_view_descriptor = Some(TextureViewDescriptor { dimension: Some(TextureViewDimension::Cube), ..default() });
    img
}

fn sample(stops: &[(f32, crate::textures::C)], t: f32) -> crate::textures::C {
    for w in stops.windows(2) {
        let ((a, ca), (b, cb)) = (w[0], w[1]);
        if t <= b {
            let k = ((t - a) / (b - a)).clamp(0.0, 1.0);
            return crate::textures::C(ca.0 + (cb.0 - ca.0) * k, ca.1 + (cb.1 - ca.1) * k, ca.2 + (cb.2 - ca.2) * k, 1.0);
        }
    }
    stops[stops.len() - 1].1
}

/* ---------- geometría ---------- */

#[derive(Default)]
struct Buf {
    pos: Vec<[f32; 3]>,
    nor: Vec<[f32; 3]>,
    idx: Vec<u32>,
}

impl Buf {
    /// Triángulo plano con la normal pedida (da vuelta el orden si hace falta).
    fn tri(&mut self, a: Vec3, b: Vec3, c: Vec3, n: Vec3) {
        let (b, c) = if (b - a).cross(c - a).dot(n) < 0.0 { (c, b) } else { (b, c) };
        let i = self.pos.len() as u32;
        for p in [a, b, c] {
            self.pos.push(p.to_array());
            self.nor.push(n.to_array());
        }
        self.idx.extend([i, i + 1, i + 2]);
    }
    fn mesh(self) -> Mesh {
        let n = self.pos.len();
        Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD)
            .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, self.pos)
            .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, self.nor)
            .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0.0f32, 0.0]; n])
            .with_inserted_indices(Indices::U32(self.idx))
    }
}

/// Punto de un perfil: recta o curva cuadrática (control, fin), como `tracePath` de la web.
#[derive(Clone, Copy)]
pub enum P {
    L(f32, f32),
    Q(f32, f32, f32, f32),
}
use P::{L, Q};

fn trace(pts: &[P]) -> Vec<[f32; 2]> {
    let mut out: Vec<[f32; 2]> = Vec::new();
    for p in pts {
        match *p {
            L(x, y) => out.push([x, y]),
            Q(cx, cy, x, y) => {
                let [x0, y0] = *out.last().unwrap_or(&[x, y]);
                for k in 1..=10 {
                    let t = k as f32 / 10.0;
                    let u = 1.0 - t;
                    out.push([u * u * x0 + 2.0 * u * t * cx + t * t * x, u * u * y0 + 2.0 * u * t * cy + t * t * y]);
                }
            }
        }
    }
    if out.len() > 1 && out[0] == out[out.len() - 1] {
        out.pop();
    }
    out
}

fn area(r: &[[f32; 2]]) -> f32 {
    (0..r.len()).map(|i| {
        let (a, b) = (r[i], r[(i + 1) % r.len()]);
        a[0] * b[1] - b[0] * a[1]
    }).sum::<f32>() / 2.0
}

/// Extruye un contorno 2D con agujeros a lo largo de `depth`. `to3` lleva (x, y, z) del
/// perfil al espacio del arma.
fn extrude(outer: &[[f32; 2]], holes: &[Vec<[f32; 2]>], depth: f32, to3: impl Fn(f32, f32, f32) -> Vec3) -> Mesh {
    let mut b = Buf::default();
    let mut all: Vec<[f32; 2]> = outer.to_vec();
    let mut hole_idx = Vec::new();
    for h in holes {
        hole_idx.push(all.len() as u32);
        all.extend_from_slice(h);
    }
    let mut tris: Vec<u32> = Vec::new();
    earcut::Earcut::new().earcut(all.iter().copied(), &hole_idx, &mut tris);
    let hd = depth / 2.0;
    let n_front = (to3(0.0, 0.0, 1.0) - to3(0.0, 0.0, 0.0)).normalize();
    for t in tris.chunks(3) {
        let [p0, p1, p2] = [all[t[0] as usize], all[t[1] as usize], all[t[2] as usize]];
        b.tri(to3(p0[0], p0[1], hd), to3(p1[0], p1[1], hd), to3(p2[0], p2[1], hd), n_front);
        b.tri(to3(p0[0], p0[1], -hd), to3(p1[0], p1[1], -hd), to3(p2[0], p2[1], -hd), -n_front);
    }
    let rings = std::iter::once((outer.to_vec(), false)).chain(holes.iter().map(|h| (h.clone(), true)));
    for (r, hole) in rings {
        let sign = if area(&r) > 0.0 { 1.0 } else { -1.0 } * if hole { -1.0 } else { 1.0 };
        for i in 0..r.len() {
            let (a, c) = (r[i], r[(i + 1) % r.len()]);
            let (dx, dy) = (c[0] - a[0], c[1] - a[1]);
            let l = dx.hypot(dy);
            if l < 1e-6 {
                continue;
            }
            // normal hacia afuera del sólido, llevada al espacio del arma
            let n2 = (dy / l * sign, -dx / l * sign);
            let n = (to3(a[0] + n2.0, a[1] + n2.1, 0.0) - to3(a[0], a[1], 0.0)).normalize();
            let (a0, a1, c0, c1) = (to3(a[0], a[1], -hd), to3(a[0], a[1], hd), to3(c[0], c[1], -hd), to3(c[0], c[1], hd));
            b.tri(a0, c0, c1, n);
            b.tri(a0, c1, a1, n);
        }
    }
    b.mesh()
}

/// Constructor de un arma: agrega piezas como hijos de `root`.
pub struct Kit<'a, 'w, 's> {
    pub cmd: &'a mut Commands<'w, 's>,
    pub meshes: &'a mut Assets<Mesh>,
    pub cube: Handle<Mesh>,
    pub cyl: Handle<Mesh>,
    pub tube: Handle<Mesh>,
}

impl Kit<'_, '_, '_> {
    fn spawn(&mut self, parent: Entity, mesh: Handle<Mesh>, m: &Handle<StandardMaterial>, t: Transform) -> Entity {
        let e = self.cmd.spawn((Mesh3d(mesh), MeshMaterial3d(m.clone()), t, bevy::camera::visibility::RenderLayers::layer(GUN_LAYER), bevy::light::NotShadowCaster)).id();
        self.cmd.entity(parent).add_child(e);
        e
    }
    pub fn group(&mut self, parent: Entity, pos: Vec3) -> Entity {
        let e = self.cmd.spawn((Transform::from_translation(pos), Visibility::default())).id();
        self.cmd.entity(parent).add_child(e);
        e
    }
    /// Caja (`B`).
    #[allow(clippy::too_many_arguments)]
    pub fn b(&mut self, p: Entity, w: f32, h: f32, d: f32, x: f32, y: f32, z: f32, m: &Handle<StandardMaterial>) -> Entity {
        let c = self.cube.clone();
        self.spawn(p, c, m, Transform::from_xyz(x, y, z).with_scale(Vec3::new(w, h, d)))
    }
    /// Cilindro con el eje en Z (`C`).
    #[allow(clippy::too_many_arguments)]
    pub fn c(&mut self, p: Entity, r: f32, len: f32, x: f32, y: f32, z: f32, m: &Handle<StandardMaterial>) -> Entity {
        let c = self.cyl.clone();
        self.spawn(p, c, m, Transform::from_xyz(x, y, z).with_rotation(Quat::from_rotation_x(FRAC_PI_2)).with_scale(Vec3::new(r, len, r)))
    }
    /// Cilindro vertical (`CY`).
    #[allow(clippy::too_many_arguments)]
    pub fn cy(&mut self, p: Entity, r: f32, len: f32, x: f32, y: f32, z: f32, m: &Handle<StandardMaterial>) -> Entity {
        let c = self.cyl.clone();
        self.spawn(p, c, m, Transform::from_xyz(x, y, z).with_scale(Vec3::new(r, len, r)))
    }
    /// Cilindro con el eje en X.
    #[allow(clippy::too_many_arguments)]
    pub fn cx(&mut self, p: Entity, r: f32, len: f32, x: f32, y: f32, z: f32, m: &Handle<StandardMaterial>) -> Entity {
        let c = self.cyl.clone();
        self.spawn(p, c, m, Transform::from_xyz(x, y, z).with_rotation(Quat::from_rotation_z(FRAC_PI_2)).with_scale(Vec3::new(r, len, r)))
    }
    /// Tubo abierto con el eje en Z (`OT`).
    #[allow(clippy::too_many_arguments)]
    pub fn ot(&mut self, p: Entity, r: f32, len: f32, x: f32, y: f32, z: f32, m: &Handle<StandardMaterial>) -> Entity {
        let c = self.tube.clone();
        self.spawn(p, c, m, Transform::from_xyz(x, y, z).with_rotation(Quat::from_rotation_x(FRAC_PI_2)).with_scale(Vec3::new(r, len, r)))
    }
    /// Toro en el plano XY (`T`).
    #[allow(clippy::too_many_arguments)]
    pub fn t(&mut self, p: Entity, r: f32, tube: f32, x: f32, y: f32, z: f32, m: &Handle<StandardMaterial>) -> Entity {
        let mesh = self.meshes.add(Torus::new(r - tube, r + tube).mesh().minor_resolution(8).major_resolution(24));
        self.spawn(p, mesh, m, Transform::from_xyz(x, y, z).with_rotation(Quat::from_rotation_x(FRAC_PI_2)))
    }
    pub fn sphere(&mut self, p: Entity, r: f32, x: f32, y: f32, z: f32, m: &Handle<StandardMaterial>) -> Entity {
        let mesh = self.meshes.add(Sphere::new(r).mesh().uv(10, 8));
        self.spawn(p, mesh, m, Transform::from_xyz(x, y, z))
    }
    /// Perfil lateral extruido a lo ancho (`EX`). El bisel de la web se omite: el ancho total es el mismo.
    #[allow(clippy::too_many_arguments)]
    pub fn ex(&mut self, p: Entity, pts: &[P], width: f32, x: f32, m: &Handle<StandardMaterial>, holes: &[&[P]]) -> Entity {
        let outer = trace(pts);
        let holes: Vec<_> = holes.iter().map(|h| trace(h)).collect();
        // (u adelante, v arriba, w ancho) → (x = w, y = v, z = −u)
        let mesh = self.meshes.add(extrude(&outer, &holes, width, |u, v, w| Vec3::new(w, v, -u)));
        self.spawn(p, mesh, m, Transform::from_xyz(x, 0.0, 0.0))
    }
    /// Marco visto de frente con ventana, extruido hacia atrás (`FRAME`): capuchas de mira.
    #[allow(clippy::too_many_arguments)]
    pub fn frame(&mut self, p: Entity, cy: f32, w: f32, h: f32, hw: f32, hh: f32, depth: f32, z: f32, m: &Handle<StandardMaterial>) -> Entity {
        let rect = |cx: f32, cy: f32, w: f32, h: f32, r: f32| {
            let (x, y) = (cx - w / 2.0, cy - h / 2.0);
            trace(&[L(x + r, y), L(x + w - r, y), Q(x + w, y, x + w, y + r), L(x + w, y + h - r), Q(x + w, y + h, x + w - r, y + h), L(x + r, y + h), Q(x, y + h, x, y + h - r), L(x, y + r), Q(x, y, x + r, y)])
        };
        let mesh = self.meshes.add(extrude(&rect(0.0, cy, w, h, 0.008), &[rect(0.0, cy + 0.002, hw, hh, 0.005)], depth, |x, y, zz| Vec3::new(x, y, zz)));
        self.spawn(p, mesh, m, Transform::from_xyz(0.0, 0.0, z))
    }
    pub fn quad(&mut self, p: Entity, w: f32, h: f32, x: f32, y: f32, z: f32, m: &Handle<StandardMaterial>) -> Entity {
        let mesh = self.meshes.add(Rectangle::new(w, h));
        self.spawn(p, mesh, m, Transform::from_xyz(x, y, z))
    }
}

/// Piezas animadas de cada arma.
#[derive(Default, Clone)]
pub struct Parts {
    pub mag: Option<Entity>,
    pub pump: Option<Entity>,
    pub coils: Vec<Entity>,
    pub drum: Option<Entity>,
}

// PR-7: carabina modular, receptor de aluminio, guardamanos facetado y culata FDE, mira holográfica
pub fn rifle(k: &mut Kit, g: Entity, m: &Gm) -> Parts {
    let mut r = Parts::default();
    k.ex(g, &[L(-0.1, 0.0), L(0.19, 0.0), L(0.19, 0.046), L(0.172, 0.062), L(-0.1, 0.062)], 0.064, 0.0, &m.metal, &[]);
    k.b(g, 0.05, 0.012, 0.30, 0.0, 0.068, -0.045, &m.dark);
    for i in 0..12 {
        k.b(g, 0.054, 0.006, 0.01, 0.0, 0.077, 0.1 - i as f32 * 0.025, &m.black);
    }
    k.ex(g, &[L(0.18, -0.03), L(0.46, -0.03), L(0.476, -0.014), L(0.476, 0.046), L(0.46, 0.058), L(0.18, 0.058)], 0.072, 0.0, &m.fde, &[]);
    k.b(g, 0.046, 0.008, 0.27, 0.0, 0.066, -0.32, &m.dark);
    for i in 0..9 {
        k.b(g, 0.05, 0.005, 0.009, 0.0, 0.072, -0.2 - i as f32 * 0.03, &m.black);
    }
    for sx in [-1.0, 1.0] {
        for i in 0..3 {
            k.b(g, 0.004, 0.012, 0.05, sx * 0.0395, 0.012, -0.24 - i as f32 * 0.075, &m.black);
        }
    }
    k.b(g, 0.002, 0.005, 0.22, -0.0405, -0.016, -0.32, &m.amber);
    k.ex(g, &[L(-0.1, 0.0), L(0.17, 0.0), L(0.17, -0.024), L(0.125, -0.034), L(0.125, -0.062), L(0.05, -0.062), L(0.05, -0.03), L(-0.1, -0.03)], 0.058, 0.0, &m.metal, &[]);
    k.ex(g, &[L(-0.02, -0.03), L(-0.062, -0.142), L(-0.106, -0.142), L(-0.112, -0.128), L(-0.074, -0.03)], 0.044, 0.0, &m.poly, &[]);
    for i in 0..4 {
        let i = i as f32;
        k.b(g, 0.05, 0.003, 0.016, 0.0, -0.06 - i * 0.02, 0.052 + i * 0.009, &m.black);
    }
    k.ex(g, &[L(-0.016, -0.03), L(0.052, -0.03), L(0.042, -0.074), L(-0.012, -0.074)], 0.012, 0.0, &m.dark, &[&[L(-0.006, -0.036), L(0.04, -0.036), L(0.033, -0.066), L(-0.004, -0.066)]]);
    k.ex(g, &[L(0.012, -0.034), L(0.02, -0.034), L(0.017, -0.058), L(0.009, -0.057)], 0.006, 0.0, &m.steel, &[]);
    let mag = k.group(g, Vec3::ZERO);
    k.ex(mag, &[L(0.056, -0.056), L(0.12, -0.056), Q(0.136, -0.13, 0.16, -0.198), L(0.094, -0.212), Q(0.074, -0.13, 0.056, -0.056)], 0.046, 0.0, &m.poly, &[]);
    for i in 0..4 {
        let i = i as f32;
        k.b(mag, 0.052, 0.004, 0.05, 0.0, -0.09 - i * 0.028, -0.088 - i * 0.011, &m.black);
    }
    k.ex(mag, &[L(0.092, -0.206), L(0.164, -0.193), L(0.168, -0.206), L(0.094, -0.221)], 0.052, 0.0, &m.amber, &[]);
    r.mag = Some(mag);
    k.c(g, 0.016, 0.2, 0.0, 0.026, 0.2, &m.steel);
    k.ex(g, &[L(-0.12, 0.042), L(-0.36, 0.046), L(-0.372, 0.036), L(-0.372, -0.088), L(-0.34, -0.094), L(-0.2, -0.032), L(-0.12, -0.012)], 0.046, 0.0, &m.fde, &[&[L(-0.2, 0.026), L(-0.315, 0.029), L(-0.315, -0.05), L(-0.24, -0.022)]]);
    k.ex(g, &[L(-0.372, 0.048), L(-0.388, 0.048), L(-0.388, -0.094), L(-0.372, -0.094)], 0.05, 0.0, &m.rubber, &[]);
    k.c(g, 0.011, 0.14, 0.0, 0.012, -0.53, &m.steel);
    k.b(g, 0.032, 0.034, 0.022, 0.0, 0.012, -0.49, &m.dark);
    k.c(g, 0.019, 0.06, 0.0, 0.012, -0.6, &m.dark);
    for sx in [-1.0, 1.0] {
        for kk in 0..3 {
            k.b(g, 0.004, 0.009, 0.011, sx * 0.0192, 0.012, -0.58 - kk as f32 * 0.017, &m.black);
        }
    }
    k.b(g, 0.072, 0.012, 0.014, 0.0, 0.052, 0.093, &m.dark);
    k.b(g, 0.002, 0.02, 0.07, 0.0325, 0.03, -0.06, &m.black);
    k.b(g, 0.006, 0.01, 0.018, -0.032, -0.012, 0.03, &m.amber);
    k.b(g, 0.048, 0.012, 0.074, 0.0, 0.082, -0.062, &m.dark);
    k.frame(g, 0.114, 0.056, 0.052, 0.044, 0.036, 0.062, -0.068, &m.metal);
    k.b(g, 0.044, 0.036, 0.002, 0.0, 0.116, -0.096, &m.glass);
    k.quad(g, 0.026, 0.026, 0.0, 0.114, -0.092, &m.reticle);
    k.cx(g, 0.006, 0.01, 0.031, 0.118, -0.05, &m.steel);
    r
}

// KS-12: escopeta de corredera, guardamano perforado, cartuchera lateral, mira de anillo y fibra óptica
pub fn shotgun(k: &mut Kit, g: Entity, m: &Gm) -> Parts {
    let mut r = Parts::default();
    k.ex(g, &[L(-0.12, -0.035), L(0.14, -0.035), L(0.14, 0.046), L(0.105, 0.056), L(-0.06, 0.056), Q(-0.112, 0.056, -0.12, 0.02)], 0.064, 0.0, &m.metal, &[]);
    k.b(g, 0.002, 0.022, 0.07, 0.0335, 0.015, -0.02, &m.black);
    k.b(g, 0.006, 0.05, 0.12, -0.036, 0.002, 0.0, &m.poly);
    for i in 0..4 {
        let z = 0.045 - i as f32 * 0.03;
        k.cy(g, 0.0095, 0.054, -0.048, 0.004, z, &m.shell);
        k.cy(g, 0.0099, 0.013, -0.048, 0.036, z, &m.brass);
    }
    k.c(g, 0.02, 0.48, 0.0, 0.02, -0.38, &m.steel);
    k.ot(g, 0.029, 0.24, 0.0, 0.02, -0.3, &m.perf);
    k.b(g, 0.012, 0.006, 0.17, 0.0, 0.046, -0.535, &m.dark);
    for i in 0..3 {
        k.b(g, 0.004, 0.006, 0.004, 0.0, 0.04, -0.47 - i as f32 * 0.06, &m.dark);
    }
    k.c(g, 0.017, 0.36, 0.0, -0.025, -0.32, &m.dark);
    k.c(g, 0.019, 0.02, 0.0, -0.025, -0.505, &m.steel);
    k.b(g, 0.03, 0.066, 0.016, 0.0, -0.002, -0.49, &m.metal);
    let pump = k.group(g, Vec3::ZERO);
    k.ex(pump, &[L(0.2, -0.06), L(0.4, -0.06), Q(0.42, -0.046, 0.42, -0.032), L(0.42, -0.01), L(0.2, -0.01), Q(0.19, -0.034, 0.2, -0.06)], 0.07, 0.0, &m.poly, &[]);
    for i in 0..7 {
        k.b(pump, 0.084, 0.046, 0.005, 0.0, -0.035, -0.225 - i as f32 * 0.025, &m.black);
    }
    r.pump = Some(pump);
    k.ex(g, &[L(-0.12, 0.046), L(-0.25, 0.03), L(-0.42, 0.008), L(-0.432, 0.0), L(-0.432, -0.098), L(-0.384, -0.11), L(-0.2, -0.068), L(-0.13, -0.056), L(-0.104, -0.138), L(-0.06, -0.134), L(-0.058, -0.035), L(-0.12, -0.035)], 0.046, 0.0, &m.poly, &[&[L(-0.22, 0.014), L(-0.37, 0.0), L(-0.37, -0.07), L(-0.24, -0.045)]]);
    k.ex(g, &[L(-0.432, 0.008), L(-0.448, 0.008), L(-0.448, -0.1), L(-0.432, -0.1)], 0.05, 0.0, &m.rubber, &[]);
    for sx in [-1.0, 1.0] {
        k.b(g, 0.002, 0.005, 0.18, sx * 0.0242, 0.021, 0.29, &m.red);
    }
    for i in 0..4 {
        let i = i as f32;
        k.b(g, 0.048, 0.003, 0.014, 0.0, -0.07 - i * 0.017, 0.09 + i * 0.008, &m.black);
    }
    k.ex(g, &[L(-0.04, -0.035), L(0.03, -0.035), L(0.02, -0.078), L(-0.036, -0.078)], 0.012, 0.0, &m.dark, &[&[L(-0.03, -0.04), L(0.018, -0.04), L(0.012, -0.07), L(-0.028, -0.07)]]);
    k.ex(g, &[L(-0.012, -0.039), L(-0.004, -0.039), L(-0.007, -0.064), L(-0.015, -0.063)], 0.006, 0.0, &m.steel, &[]);
    k.b(g, 0.012, 0.03, 0.014, 0.0, 0.068, 0.06, &m.dark);
    k.t(g, 0.013, 0.0032, 0.0, 0.08, 0.06, &m.steel);
    k.b(g, 0.004, 0.028, 0.006, 0.0, 0.063, -0.53, &m.dark);
    k.sphere(g, 0.0032, 0.0, 0.08, -0.53, &m.red);
    r
}

// RP-3: riel de partículas, cuerpo facetado, horquilla de rieles, bobinas, cápsula de energía y mira tubular
pub fn rail(k: &mut Kit, g: Entity, m: &Gm) -> Parts {
    let mut r = Parts::default();
    k.ex(g, &[L(-0.14, -0.04), L(0.26, -0.04), L(0.3, -0.014), L(0.3, 0.034), L(0.22, 0.06), L(-0.06, 0.06), L(-0.14, 0.044)], 0.074, 0.0, &m.metal, &[]);
    k.ex(g, &[L(-0.05, 0.06), L(0.2, 0.06), L(0.18, 0.076), L(-0.03, 0.076)], 0.05, 0.0, &m.dark, &[]);
    for i in 0..7 {
        k.b(g, 0.052, 0.004, 0.009, 0.0, 0.08, 0.02 - i as f32 * 0.028, &m.black);
    }
    k.b(g, 0.003, 0.034, 0.26, -0.0395, 0.004, -0.07, &m.dark);
    k.b(g, 0.003, 0.005, 0.24, -0.041, 0.02, -0.07, &m.cyan);
    k.b(g, 0.003, 0.005, 0.12, -0.041, -0.012, -0.01, &m.cyan);
    k.c(g, 0.019, 0.2, 0.0, -0.062, -0.06, &m.glass);
    k.c(g, 0.009, 0.19, 0.0, -0.062, -0.06, &m.cyan);
    for z in [0.04, -0.16] {
        k.c(g, 0.021, 0.016, 0.0, -0.062, z, &m.steel);
    }
    for sx in [-1.0, 1.0] {
        k.ex(g, &[L(0.26, -0.03), L(0.72, -0.012), L(0.72, 0.012), L(0.26, 0.04)], 0.016, sx * 0.031, &m.steel, &[]);
        k.b(g, 0.003, 0.008, 0.44, sx * 0.021, 0.0, -0.49, &m.cyan);
    }
    for i in 0..6 {
        let e = k.t(g, 0.044, 0.007, 0.0, 0.0, -0.33 - i as f32 * 0.066, &m.cyan);
        r.coils.push(e);
    }
    k.b(g, 0.078, 0.018, 0.022, 0.0, 0.0, -0.71, &m.dark);
    k.ex(g, &[L(-0.03, -0.04), L(-0.066, -0.15), L(-0.112, -0.15), L(-0.082, -0.04)], 0.046, 0.0, &m.poly, &[]);
    k.ex(g, &[L(0.0, -0.04), L(0.06, -0.04), L(0.05, -0.08), L(0.0, -0.08)], 0.012, 0.0, &m.dark, &[&[L(0.008, -0.045), L(0.048, -0.045), L(0.042, -0.073), L(0.008, -0.073)]]);
    for kk in 0..3 {
        let kk = kk as f32;
        k.c(g, 0.012, 0.055, -0.05, -0.022, 0.02 - kk * 0.07, &m.dark);
        k.t(g, 0.0125, 0.002, -0.05, -0.022, -kk * 0.07, &m.cyan);
    }
    k.ex(g, &[L(-0.14, 0.044), L(-0.38, 0.03), L(-0.392, 0.02), L(-0.392, -0.09), L(-0.352, -0.096), L(-0.14, -0.03)], 0.044, 0.0, &m.dark, &[&[L(-0.18, 0.022), L(-0.34, 0.014), L(-0.34, -0.06), L(-0.2, -0.02)]]);
    k.ex(g, &[L(-0.392, 0.032), L(-0.406, 0.032), L(-0.406, -0.096), L(-0.392, -0.096)], 0.048, 0.0, &m.rubber, &[]);
    k.b(g, 0.02, 0.025, 0.03, 0.0, 0.072, -0.02, &m.dark);
    k.b(g, 0.02, 0.025, 0.03, 0.0, 0.072, -0.13, &m.dark);
    k.ot(g, 0.024, 0.2, 0.0, 0.096, -0.07, &m.dark);
    k.ot(g, 0.031, 0.04, 0.0, 0.096, -0.19, &m.metal);
    k.ot(g, 0.028, 0.025, 0.0, 0.096, 0.035, &m.metal);
    k.cy(g, 0.009, 0.018, 0.0, 0.126, -0.07, &m.steel);
    k.cx(g, 0.009, 0.018, 0.03, 0.096, -0.07, &m.steel);
    k.b(g, 0.046, 0.046, 0.002, 0.0, 0.096, -0.205, &m.glass);
    k.sphere(g, 0.0018, 0.0, 0.096, -0.2, &m.cyan);
    r
}

// LG-6: lanzagranadas revólver, placas laterales caladas, tambor estriado, culata plegable
pub fn launcher(k: &mut Kit, g: Entity, m: &Gm) -> Parts {
    let mut r = Parts::default();
    k.ot(g, 0.042, 0.36, 0.0, 0.02, -0.36, &m.dark);
    k.c(g, 0.036, 0.34, 0.0, 0.02, -0.36, &m.black);
    for z in [-0.24, -0.36, -0.48] {
        k.t(g, 0.044, 0.004, 0.0, 0.02, z, &m.steel);
    }
    k.ot(g, 0.048, 0.03, 0.0, 0.02, -0.535, &m.metal);
    k.b(g, 0.03, 0.01, 0.3, 0.0, 0.066, -0.36, &m.dark);
    for i in 0..8 {
        k.b(g, 0.034, 0.005, 0.008, 0.0, 0.073, -0.24 - i as f32 * 0.032, &m.dark);
    }
    for sx in [-1.0, 1.0] {
        k.ex(g, &[L(-0.08, -0.086), L(0.16, -0.086), L(0.182, -0.04), L(0.182, 0.072), L(-0.08, 0.072)], 0.01, sx * 0.076, &m.metal, &[&[L(-0.045, -0.062), L(0.105, -0.062), L(0.105, 0.048), L(-0.045, 0.048)]]);
    }
    k.b(g, 0.11, 0.012, 0.27, 0.0, 0.077, -0.05, &m.dark);
    for i in 0..8 {
        k.b(g, 0.114, 0.004, 0.01, 0.0, 0.084, 0.06 - i as f32 * 0.032, &m.black);
    }
    k.b(g, 0.162, 0.012, 0.27, 0.0, -0.09, -0.05, &m.metal);
    let drum = k.group(g, Vec3::new(0.0, -0.01, -0.02));
    k.c(drum, 0.07, 0.12, 0.0, 0.0, 0.0, &m.steel);
    for i in 0..6 {
        let a = i as f32 / 6.0 * TAU;
        let a2 = a + PI / 6.0;
        k.c(drum, 0.017, 0.124, a.cos() * 0.044, a.sin() * 0.044, 0.0, &m.black);
        k.c(drum, 0.015, 0.012, a.cos() * 0.044, a.sin() * 0.044, -0.058, &m.amber);
        let fl = k.b(drum, 0.014, 0.01, 0.1, a2.cos() * 0.068, a2.sin() * 0.068, 0.0, &m.black);
        k.cmd.entity(fl).insert(Transform::from_xyz(a2.cos() * 0.068, a2.sin() * 0.068, 0.0).with_rotation(Quat::from_rotation_z(a2)).with_scale(Vec3::new(0.014, 0.01, 0.1)));
    }
    r.drum = Some(drum);
    k.ex(g, &[L(0.24, -0.022), L(0.3, -0.022), L(0.292, -0.135), L(0.252, -0.135)], 0.036, 0.0, &m.poly, &[]);
    k.ex(g, &[L(-0.03, -0.09), L(-0.066, -0.195), L(-0.112, -0.195), L(-0.082, -0.09)], 0.044, 0.0, &m.poly, &[]);
    k.ex(g, &[L(-0.012, -0.09), L(0.05, -0.09), L(0.04, -0.13), L(-0.01, -0.13)], 0.012, 0.0, &m.dark, &[&[L(-0.003, -0.095), L(0.04, -0.095), L(0.034, -0.124), L(-0.002, -0.124)]]);
    for sx in [-1.0, 1.0] {
        k.b(g, 0.008, 0.012, 0.22, sx * 0.022, 0.03, 0.19, &m.steel);
    }
    k.ex(g, &[L(-0.29, 0.052), L(-0.312, 0.052), L(-0.312, -0.082), L(-0.29, -0.082)], 0.056, 0.0, &m.rubber, &[]);
    k.b(g, 0.044, 0.012, 0.012, 0.0, -0.03, 0.29, &m.steel);
    k.b(g, 0.006, 0.014, 0.006, 0.0, 0.089, -0.02, &m.dark);
    k.t(g, 0.012, 0.003, 0.0, 0.1, -0.02, &m.steel);
    k.b(g, 0.004, 0.036, 0.006, 0.0, 0.081, -0.5, &m.dark);
    k.sphere(g, 0.003, 0.0, 0.1, -0.5, &m.amber);
    r
}

/// Malla de tubo abierto (radio 1, alto 1, eje Y) para `ot`.
pub fn open_tube() -> Mesh {
    let mut b = Buf::default();
    let n = 24;
    for i in 0..n {
        let (a0, a1) = (i as f32 / n as f32 * TAU, (i + 1) as f32 / n as f32 * TAU);
        let (p0, p1) = (Vec3::new(a0.cos(), 0.0, a0.sin()), Vec3::new(a1.cos(), 0.0, a1.sin()));
        let n0 = ((p0 + p1) / 2.0).normalize();
        let (lo0, hi0, lo1, hi1) = (p0 - Vec3::Y * 0.5, p0 + Vec3::Y * 0.5, p1 - Vec3::Y * 0.5, p1 + Vec3::Y * 0.5);
        b.tri(lo0, lo1, hi1, n0);
        b.tri(lo0, hi1, hi0, n0);
    }
    b.mesh()
}

/// Destello de boca: textura radial ámbar (`glowTex`).
pub fn glow_tex() -> Image {
    let s = 64;
    let mut px = vec![[0.0f32; 4]; s * s];
    for y in 0..s {
        for x in 0..s {
            let d = ((x as f32 + 0.5 - 32.0).hypot(y as f32 + 0.5 - 32.0) / 32.0).min(1.0);
            let (c, a) = if d < 0.3 {
                let k = d / 0.3;
                ((1.0, 0.94 - 0.27 * k, 0.78 - 0.58 * k), 1.0 - 0.2 * k)
            } else {
                let k = (d - 0.3) / 0.7;
                ((1.0, 0.67 - 0.2 * k, 0.2 * (1.0 - k)), 0.8 * (1.0 - k))
            };
            px[y * s + x] = [c.0 * a, c.1 * a, c.2 * a, a];
        }
    }
    crate::textures::to_image(s, px, true, false)
}
