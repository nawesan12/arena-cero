//! Personas (nuevo): esqueleto con cadera, columna, cabeza, hombros y codos, caderas y rodillas,
//! modelado con cápsulas y esferas en vez de cajas. Lo usan soldados, peatones y cadáveres.
//! La animación es procedural: caminar, correr (con el torso inclinado), agacharse rodilla en
//! tierra, apuntar con las dos manos, respirar quieto, huir con las manos arriba y caer.

use bevy::light::NotShadowCaster;
use bevy::prelude::*;
use std::collections::HashMap;

pub struct HumanoidPlugin;

impl Plugin for HumanoidPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PreStartup, setup);
    }
}

/// Mallas compartidas y paleta de materiales (piel, ropa, pelo).
#[derive(Resource)]
pub struct HumanAssets {
    pub capsule: Handle<Mesh>,
    pub sphere: Handle<Mesh>,
    pub cube: Handle<Mesh>,
    pal: HashMap<u32, Handle<StandardMaterial>>,
}

impl HumanAssets {
    /// Material de la paleta (todos los colores se crean al arrancar).
    pub fn m(&self, c: u32) -> Handle<StandardMaterial> {
        self.pal.get(&c).cloned().unwrap_or_else(|| self.pal[&0x808080].clone())
    }
}

pub const SKIN: [u32; 6] = [0xf1c8a8, 0xe0b090, 0xc8946c, 0xa87250, 0x8a5a3c, 0x5e3c28];
pub const SHIRT: [u32; 14] = [0x3d6fa8, 0xb8483e, 0xd8c070, 0x4f8a5a, 0x7a5a9a, 0xe8e6de, 0x2a2e36, 0xd07a38, 0x8ab0c8, 0xc86a8a, 0x5a6a3a, 0x9a3a3a, 0xf0d8a0, 0x30608a];
pub const PANTS: [u32; 7] = [0x2a3a5a, 0x1e2228, 0x8a7a5a, 0x4a4e54, 0x3a4a6a, 0x6a4a3a, 0xc8c0b0];
pub const SHOES: [u32; 4] = [0x1a1a1a, 0x5a3a28, 0xe8e8e8, 0x3a3a48];
pub const HAIR: [u32; 6] = [0x1a1410, 0x4a3020, 0x8a6a40, 0xc8a060, 0xb0b0b0, 0x6a2a1a];
/// Ropa de los soldados: chaleco, uniforme, detalles y arma.
pub const ARMY: [u32; 8] = [0x2b3238, 0x3a4234, 0x4a4436, 0x1f2427, 0xd9792b, 0x15181a, 0x3e2422, 0x8a1c1c];

fn setup(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>, mut mats: ResMut<Assets<StandardMaterial>>) {
    let mut pal = HashMap::new();
    let all = SKIN.iter().chain(SHIRT.iter()).chain(PANTS.iter()).chain(SHOES.iter()).chain(HAIR.iter()).chain(ARMY.iter()).chain([0x808080u32, 0x6a5a48, 0x2a2a2a].iter());
    for &c in all {
        let rough = if ARMY.contains(&c) { 0.6 } else { 0.85 };
        pal.entry(c).or_insert_with(|| mats.add(StandardMaterial { base_color: crate::world_render::col(c), perceptual_roughness: rough, ..default() }));
    }
    commands.insert_resource(HumanAssets {
        capsule: meshes.add(Capsule3d::new(0.5, 1.0).mesh().rings(2).latitudes(6).longitudes(8)),
        sphere: meshes.add(Sphere::new(0.5).mesh().ico(1).expect("esfera")),
        cube: meshes.add(Cuboid::new(1.0, 1.0, 1.0)),
        pal,
    });
}

/// Cómo está vestida una persona.
#[derive(Clone)]
pub struct Outfit {
    pub skin: Handle<StandardMaterial>,
    pub top: Handle<StandardMaterial>,
    pub bottom: Handle<StandardMaterial>,
    pub shoes: Handle<StandardMaterial>,
    pub hair: Option<Handle<StandardMaterial>>,
    pub long_hair: bool,
    /// Casco con visor (soldados).
    pub helmet: Option<(Handle<StandardMaterial>, Handle<StandardMaterial>)>,
    /// Chaleco con placas y una franja de color.
    pub vest: Option<(Handle<StandardMaterial>, Handle<StandardMaterial>)>,
    pub pack: Option<Handle<StandardMaterial>>,
    pub hood: Option<Handle<StandardMaterial>>,
    /// Gorra o boina.
    pub cap: Option<Handle<StandardMaterial>>,
    /// Sobretodo hasta las rodillas.
    pub coat: Option<Handle<StandardMaterial>>,
    pub skirt: bool,
    pub bag: Option<Handle<StandardMaterial>>,
    /// Arma: material, largo y mira.
    pub gun: Option<(Handle<StandardMaterial>, f32, bool)>,
    /// Ancho de hombros (1 normal).
    pub build: f32,
}

/// Articulaciones de una persona, para animarla.
#[derive(Component, Clone)]
pub struct Skel {
    pub hips: Entity,
    pub chest: Entity,
    pub head: Entity,
    pub shoulder: [Entity; 2],
    pub elbow: [Entity; 2],
    pub hip: [Entity; 2],
    pub knee: [Entity; 2],
    pub gun: Option<Entity>,
    pub visor: Option<Entity>,
    pub torso: Entity,
    pub hip_y: f32,
}

struct Kit<'a, 'w, 's> {
    c: &'a mut Commands<'w, 's>,
    a: &'a HumanAssets,
}

impl Kit<'_, '_, '_> {
    fn joint(&mut self, p: Entity, at: Vec3) -> Entity {
        let e = self.c.spawn((Transform::from_translation(at), Visibility::default())).id();
        self.c.entity(p).add_child(e);
        e
    }
    fn part(&mut self, p: Entity, mesh: &Handle<Mesh>, m: &Handle<StandardMaterial>, at: Vec3, scale: Vec3) -> Entity {
        let e = self.c.spawn((Mesh3d(mesh.clone()), MeshMaterial3d(m.clone()), Transform::from_translation(at).with_scale(scale), NotShadowCaster)).id();
        self.c.entity(p).add_child(e);
        e
    }
    /// Cápsula de largo total `len` y radio `r`, colgando desde la articulación hacia abajo.
    fn limb(&mut self, p: Entity, m: &Handle<StandardMaterial>, len: f32, r: f32) -> Entity {
        let cap = self.a.capsule.clone();
        self.part(p, &cap, m, Vec3::new(0.0, -len / 2.0, 0.0), Vec3::new(r * 2.0, len / 2.0, r * 2.0))
    }
    fn ball(&mut self, p: Entity, m: &Handle<StandardMaterial>, at: Vec3, size: Vec3) -> Entity {
        let s = self.a.sphere.clone();
        self.part(p, &s, m, at, size)
    }
    fn block(&mut self, p: Entity, m: &Handle<StandardMaterial>, at: Vec3, size: Vec3) -> Entity {
        let s = self.a.cube.clone();
        self.part(p, &s, m, at, size)
    }
}

/// Arma la persona debajo de `root` (los pies en y = 0, mirando a +Z).
pub fn build(c: &mut Commands, a: &HumanAssets, root: Entity, o: &Outfit) -> Skel {
    let mut k = Kit { c, a };
    let bw = o.build;
    let hip_y = 0.95;
    let hips = k.joint(root, Vec3::new(0.0, hip_y, 0.0));
    // pelvis y piernas
    k.ball(hips, &o.bottom, Vec3::new(0.0, 0.02, 0.0), Vec3::new(0.36 * bw, 0.24, 0.24));
    let mut hip = [Entity::PLACEHOLDER; 2];
    let mut knee = [Entity::PLACEHOLDER; 2];
    for (i, sx) in [-1.0f32, 1.0].into_iter().enumerate() {
        let h = k.joint(hips, Vec3::new(sx * 0.1 * bw, -0.04, 0.0));
        let leg = if o.skirt { o.skin.clone() } else { o.bottom.clone() };
        k.limb(h, &leg, 0.48, 0.085);
        let kn = k.joint(h, Vec3::new(0.0, -0.45, 0.0));
        k.limb(kn, &leg, 0.46, 0.07);
        k.block(kn, &o.shoes, Vec3::new(0.0, -0.45, 0.05), Vec3::new(0.12, 0.09, 0.27));
        if o.vest.is_some() {
            // rodilleras
            k.ball(kn, &o.vest.as_ref().unwrap().0, Vec3::new(0.0, -0.02, 0.06), Vec3::new(0.13, 0.12, 0.08));
        }
        hip[i] = h;
        knee[i] = kn;
    }
    if o.skirt {
        k.ball(hips, &o.bottom, Vec3::new(0.0, -0.18, 0.0), Vec3::new(0.44 * bw, 0.42, 0.32));
    }
    // columna, torso y cabeza
    let chest = k.joint(hips, Vec3::new(0.0, 0.1, 0.0));
    let cap = a.capsule.clone();
    let torso = k.part(chest, &cap, &o.top, Vec3::new(0.0, 0.26, 0.0), Vec3::new(0.4 * bw, 0.27, 0.25));
    k.c.entity(torso).remove::<NotShadowCaster>();
    if let Some((vest, band)) = &o.vest {
        k.block(chest, vest, Vec3::new(0.0, 0.3, 0.02), Vec3::new(0.42 * bw, 0.4, 0.3));
        k.block(chest, band, Vec3::new(0.0, 0.36, 0.17), Vec3::new(0.3 * bw, 0.14, 0.02));
        for sx in [-0.12f32, 0.0, 0.12] {
            k.block(chest, vest, Vec3::new(sx * bw, 0.12, 0.17), Vec3::new(0.09, 0.12, 0.07));
        }
        // hombreras
        for sx in [-1.0f32, 1.0] {
            k.ball(chest, vest, Vec3::new(sx * 0.21 * bw, 0.47, 0.0), Vec3::new(0.17, 0.12, 0.2));
        }
    }
    if let Some(coat) = &o.coat {
        k.part(chest, &cap, coat, Vec3::new(0.0, -0.05, 0.0), Vec3::new(0.46 * bw, 0.5, 0.3));
    }
    if let Some(pack) = &o.pack {
        k.block(chest, pack, Vec3::new(0.0, 0.3, -0.2), Vec3::new(0.32 * bw, 0.42, 0.18));
        k.block(chest, pack, Vec3::new(0.0, 0.12, -0.26), Vec3::new(0.26 * bw, 0.14, 0.1));
    }
    if let Some(bag) = &o.bag {
        k.block(chest, bag, Vec3::new(0.24 * bw, 0.05, 0.02), Vec3::new(0.08, 0.26, 0.3));
    }
    let neck = k.joint(chest, Vec3::new(0.0, 0.54, 0.0));
    k.limb(neck, &o.skin, 0.12, 0.05);
    let head = k.joint(chest, Vec3::new(0.0, 0.6, 0.0));
    k.ball(head, &o.skin, Vec3::new(0.0, 0.12, 0.01), Vec3::new(0.21, 0.26, 0.23));
    k.ball(head, &o.skin, Vec3::new(0.0, 0.1, 0.12), Vec3::new(0.05, 0.06, 0.05));
    let mut visor = None;
    if let Some((helm, vis)) = &o.helmet {
        k.ball(head, helm, Vec3::new(0.0, 0.19, -0.005), Vec3::new(0.27, 0.22, 0.29));
        k.block(head, helm, Vec3::new(0.0, 0.14, -0.02), Vec3::new(0.29, 0.04, 0.3));
        visor = Some(k.block(head, vis, Vec3::new(0.0, 0.13, 0.12), Vec3::new(0.2, 0.06, 0.03)));
    } else if let Some(hood) = &o.hood {
        k.ball(head, hood, Vec3::new(0.0, 0.15, -0.02), Vec3::new(0.27, 0.3, 0.29));
    } else {
        if let Some(h) = &o.hair {
            k.ball(head, h, Vec3::new(0.0, 0.2, -0.015), Vec3::new(0.23, 0.15, 0.24));
            if o.long_hair {
                k.block(head, h, Vec3::new(0.0, 0.05, -0.1), Vec3::new(0.2, 0.28, 0.06));
            }
        }
        if let Some(capm) = &o.cap {
            k.ball(head, capm, Vec3::new(0.0, 0.22, 0.0), Vec3::new(0.24, 0.1, 0.25));
            k.block(head, capm, Vec3::new(0.0, 0.2, 0.13), Vec3::new(0.16, 0.02, 0.1));
        }
    }
    // brazos
    let mut shoulder = [Entity::PLACEHOLDER; 2];
    let mut elbow = [Entity::PLACEHOLDER; 2];
    for (i, sx) in [-1.0f32, 1.0].into_iter().enumerate() {
        let s = k.joint(chest, Vec3::new(sx * 0.23 * bw, 0.46, 0.0));
        k.limb(s, &o.top, 0.3, 0.06);
        let el = k.joint(s, Vec3::new(0.0, -0.29, 0.0));
        let forearm = if o.vest.is_some() { o.top.clone() } else { o.skin.clone() };
        k.limb(el, &forearm, 0.27, 0.05);
        k.ball(el, &o.skin, Vec3::new(0.0, -0.29, 0.0), Vec3::splat(0.09));
        shoulder[i] = s;
        elbow[i] = el;
    }
    // arma (va en el pecho; la pose la sube a la altura del hombro)
    let gun = o.gun.as_ref().map(|(gm, len, scope)| {
        let g = k.joint(chest, Vec3::new(0.08, 0.3, 0.3));
        k.block(g, gm, Vec3::new(0.0, 0.0, len * 0.35), Vec3::new(0.07, 0.11, *len));
        k.block(g, gm, Vec3::new(0.0, -0.08, len * 0.15), Vec3::new(0.05, 0.14, 0.06));
        k.block(g, gm, Vec3::new(0.0, -0.02, -len * 0.18), Vec3::new(0.06, 0.09, 0.16));
        if *scope {
            k.block(g, gm, Vec3::new(0.0, 0.08, len * 0.25), Vec3::new(0.05, 0.05, 0.24));
        }
        g
    });
    Skel { hips, chest, head, shoulder, elbow, hip, knee, gun, visor, torso, hip_y }
}

/// Lo que hace la persona en este cuadro.
#[derive(Clone, Copy, Default)]
pub struct Pose {
    /// Fase del paso (radianes).
    pub phase: f32,
    /// 0 quieto, 1 caminando, 2 corriendo.
    pub gait: f32,
    /// 0..1: agachado (rodilla en tierra).
    pub crouch: f32,
    /// 0..1: apuntando con el arma.
    pub aim: f32,
    /// Inclinación de la mira (radianes, + hacia arriba).
    pub pitch: f32,
    /// Tiempo (para respirar).
    pub t: f32,
    /// 0..1: manos arriba (huyendo).
    pub hands_up: f32,
    /// Cuerpo tirado (brazos sueltos).
    pub dead: bool,
    /// Lleva un arma: sin apuntar la sostiene cruzada adelante con las dos manos.
    pub armed: bool,
}

fn lerp(a: f32, b: f32, k: f32) -> f32 {
    a + (b - a) * k
}

fn rot<F: bevy::ecs::query::QueryFilter>(tf: &mut Query<&mut Transform, F>, e: Entity, q: Quat) {
    if let Ok(mut t) = tf.get_mut(e) {
        t.rotation = q;
    }
}

/// Aplica la pose a las articulaciones.
pub fn pose<F: bevy::ecs::query::QueryFilter>(s: &Skel, p: &Pose, tf: &mut Query<&mut Transform, F>) {
    if p.dead {
        rot(tf, s.shoulder[0], Quat::from_rotation_z(-1.2) * Quat::from_rotation_x(-0.3));
        rot(tf, s.shoulder[1], Quat::from_rotation_z(1.3) * Quat::from_rotation_x(0.2));
        rot(tf, s.elbow[0], Quat::from_rotation_x(-0.4));
        rot(tf, s.elbow[1], Quat::from_rotation_x(-0.2));
        rot(tf, s.hip[0], Quat::from_rotation_z(-0.15));
        rot(tf, s.hip[1], Quat::from_rotation_z(0.2) * Quat::from_rotation_x(-0.3));
        rot(tf, s.knee[0], Quat::IDENTITY);
        rot(tf, s.knee[1], Quat::from_rotation_x(0.5));
        rot(tf, s.chest, Quat::IDENTITY);
        rot(tf, s.head, Quat::from_rotation_y(0.5));
        return;
    }
    let (sn, cs) = p.phase.sin_cos();
    let walk = p.gait.min(1.0);
    let run = (p.gait - 1.0).clamp(0.0, 1.0);
    let amp = lerp(0.0, 0.5, walk) + run * 0.4;
    let cr = p.crouch;
    // piernas: paso alternado, la rodilla se dobla cuando la pierna va hacia atrás
    let knee_bend = |x: f32| (x.max(0.0)) * (0.6 + run * 0.6) * walk.max(run);
    let thigh_l = -sn * amp;
    let thigh_r = sn * amp;
    // agachado: rodilla izquierda adelante, derecha en el piso
    rot(tf, s.hip[0], Quat::from_rotation_x(lerp(thigh_l, -1.35, cr)));
    rot(tf, s.hip[1], Quat::from_rotation_x(lerp(thigh_r, -0.15, cr)));
    rot(tf, s.knee[0], Quat::from_rotation_x(lerp(knee_bend(-cs) + 0.05, 1.6, cr)));
    rot(tf, s.knee[1], Quat::from_rotation_x(lerp(knee_bend(cs) + 0.05, 1.75, cr)));
    if let Ok(mut t) = tf.get_mut(s.hips) {
        let bob = (sn * 2.0).abs() * 0.035 * p.gait.min(2.0);
        t.translation.y = s.hip_y - cr * 0.48 + bob - 0.02 * walk;
    }
    // torso: se inclina al correr y al apuntar hacia abajo; respira quieto
    let breathe = (p.t * 1.7).sin() * 0.015 * (1.0 - walk);
    let lean = run * 0.28 + walk * 0.05 + cr * 0.15 - p.pitch * 0.35 * p.aim;
    rot(tf, s.chest, Quat::from_rotation_x(lean + breathe) * Quat::from_rotation_y(sn * 0.12 * walk * (1.0 - p.aim)));
    rot(tf, s.head, Quat::from_rotation_x(-lean * 0.6 - p.pitch * 0.4 * p.aim));
    // brazos: balanceo al caminar; apuntando, el arma sube al hombro con las dos manos
    let swing = sn * amp * 0.9;
    let aim = p.aim;
    let up = p.hands_up;
    // con arma y sin apuntar: "en guardia", el arma cruzada adelante con las dos manos
    let (rest_r, rest_l, rest_er, rest_el, rest_zl) = if p.armed { (-0.45 + swing * 0.15, -0.75 - swing * 0.15, -1.25, -1.15, 0.45) } else { (swing, -swing, -0.25 - run * 1.0, -0.25 - run * 1.0, -0.08) };
    let r_sh = lerp(lerp(rest_r, -1.35 - p.pitch, aim), -2.9, up);
    let l_sh = lerp(lerp(rest_l, -1.15 - p.pitch, aim), -2.8, up);
    rot(tf, s.shoulder[1], Quat::from_rotation_x(r_sh) * Quat::from_rotation_z(lerp(0.08, 0.15, aim)));
    rot(tf, s.shoulder[0], Quat::from_rotation_x(l_sh) * Quat::from_rotation_z(lerp(rest_zl, 0.55, aim)));
    rot(tf, s.elbow[1], Quat::from_rotation_x(lerp(rest_er, -0.35, aim.max(up))));
    rot(tf, s.elbow[0], Quat::from_rotation_x(lerp(rest_el, -0.9, aim.max(up))));
    if let Some(g) = s.gun {
        if let Ok(mut t) = tf.get_mut(g) {
            // bajo, cruzado sobre el cuerpo · en el hombro, apuntando
            let low = (Vec3::new(0.02, 0.2, 0.32), Quat::from_rotation_y(-0.75) * Quat::from_rotation_x(0.3));
            let high = (Vec3::new(0.1, 0.42, 0.32), Quat::from_rotation_x(-p.pitch));
            t.translation = low.0.lerp(high.0, aim);
            t.rotation = low.1.slerp(high.1, aim);
        }
    }
}

/// Número pseudoaleatorio estable a partir de un id (para vestir siempre igual a la misma persona).
pub fn hash(id: u32, k: u32) -> u32 {
    let mut x = id.wrapping_mul(0x9E37_79B9) ^ k.wrapping_mul(0x85EB_CA6B);
    x ^= x >> 15;
    x = x.wrapping_mul(0x2C1B_3C6D);
    x ^= x >> 12;
    x
}

/// Ropa de civil al azar (estable por id).
pub fn civilian(a: &HumanAssets, id: u32) -> Outfit {
    let h = |k: u32, n: usize| hash(id, k) as usize % n;
    let woman = h(1, 2) == 0;
    let old = h(2, 7) == 0;
    let hair = if old { 0xb0b0b0 } else { HAIR[h(3, HAIR.len())] };
    Outfit {
        skin: a.m(SKIN[h(4, SKIN.len())]),
        top: a.m(SHIRT[h(5, SHIRT.len())]),
        bottom: a.m(PANTS[h(6, PANTS.len())]),
        shoes: a.m(SHOES[h(7, SHOES.len())]),
        hair: if !woman && h(8, 6) == 0 { None } else { Some(a.m(hair)) },
        long_hair: woman && h(9, 3) != 0,
        helmet: None,
        vest: None,
        pack: if h(10, 6) == 0 { Some(a.m(SHIRT[h(11, SHIRT.len())])) } else { None },
        hood: None,
        cap: if !woman && h(12, 5) == 0 { Some(a.m(SHIRT[h(13, SHIRT.len())])) } else { None },
        coat: if old || h(14, 6) == 0 { Some(a.m([0x4a4e54, 0x6a4a3a, 0x2a2e36, 0x8a7a5a][h(15, 4)])) } else { None },
        skirt: woman && h(16, 3) == 0,
        bag: if woman && h(17, 2) == 0 { Some(a.m(SHOES[h(18, SHOES.len())])) } else { None },
        gun: None,
        build: if woman { 0.88 } else { 1.0 + (h(19, 10) as f32) * 0.012 },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::RunSystemOnce;

    /// `ARENA_HUMAN=archivo cargo test -p arena_client figuras`: arma personas en distintas poses y
    /// guarda cada pieza (forma, posición, rotación, escala y color) para dibujarlas sin abrir el juego.
    #[test]
    fn figuras() {
        let Some(path) = std::env::var_os("ARENA_HUMAN") else { return };
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default(), bevy::transform::TransformPlugin));
        app.init_asset::<Mesh>().init_asset::<StandardMaterial>();
        app.add_systems(Startup, setup);
        app.update();
        // poses: (ropa, pose, x)
        let poses: Vec<(u8, Pose)> = vec![
            (0, Pose { armed: true, ..Default::default() }),
            (0, Pose { aim: 1.0, armed: true, ..Default::default() }),
            (0, Pose { gait: 2.0, phase: 1.2, armed: true, ..Default::default() }),
            (0, Pose { crouch: 1.0, aim: 1.0, armed: true, ..Default::default() }),
            (1, Pose { aim: 1.0, armed: true, ..Default::default() }),
            (2, Pose { gait: 1.0, phase: 0.6, ..Default::default() }),
            (3, Pose { gait: 1.0, phase: 2.2, ..Default::default() }),
            (4, Pose { gait: 2.0, phase: 0.4, hands_up: 0.7, ..Default::default() }),
            (5, Pose { armed: true, ..Default::default() }),
            (6, Pose { dead: true, ..Default::default() }),
        ];
        let skels: Vec<Skel> = app.world_mut().run_system_once(move |mut c: Commands, a: Res<HumanAssets>| {
            let mut out = Vec::new();
            for (k, (kind, _)) in [(0u8, ()), (0, ()), (0, ()), (0, ()), (1, ()), (2, ()), (3, ()), (4, ()), (5, ()), (6, ())].iter().enumerate() {
                let root = c.spawn((Transform::from_xyz(k as f32 * 1.4, 0.0, 0.0), Visibility::default())).id();
                let army = |h: u32| a.m(h);
                let o = match kind {
                    0 | 6 => Outfit { skin: a.m(SKIN[1]), top: army(ARMY[3]), bottom: army(ARMY[1]), shoes: army(ARMY[5]), hair: None, long_hair: false, helmet: Some((army(ARMY[0]), army(ARMY[4]))), vest: Some((army(ARMY[0]), army(ARMY[4]))), pack: Some(army(ARMY[2])), hood: None, cap: None, coat: None, skirt: false, bag: None, gun: Some((army(ARMY[5]), 0.8, false)), build: 1.0 },
                    1 => Outfit { skin: a.m(SKIN[2]), top: army(ARMY[3]), bottom: army(ARMY[2]), shoes: army(ARMY[5]), hair: None, long_hair: false, helmet: None, vest: Some((army(ARMY[1]), army(ARMY[4]))), pack: None, hood: Some(army(ARMY[2])), cap: None, coat: None, skirt: false, bag: None, gun: Some((army(ARMY[5]), 1.25, true)), build: 1.0 },
                    5 => Outfit { skin: a.m(SKIN[0]), top: army(ARMY[6]), bottom: army(ARMY[3]), shoes: army(ARMY[5]), hair: Some(a.m(HAIR[0])), long_hair: false, helmet: None, vest: None, pack: None, hood: None, cap: Some(army(ARMY[7])), coat: Some(army(ARMY[6])), skirt: false, bag: None, gun: Some((army(ARMY[5]), 0.6, false)), build: 1.12 },
                    _ => civilian(&a, 11 + *kind as u32 * 7),
                };
                out.push(build(&mut c, &a, root, &o));
            }
            out
        }).unwrap();
        app.update();
        for (sk, (_, p)) in skels.iter().zip(poses.iter()) {
            let (sk, p) = (sk.clone(), *p);
            app.world_mut().run_system_once(move |mut tf: Query<&mut Transform>| pose(&sk, &p, &mut tf)).unwrap();
        }
        app.update();
        app.update();
        let a = app.world().resource::<HumanAssets>();
        let (cap, sph, cube) = (a.capsule.id(), a.sphere.id(), a.cube.id());
        let colors: HashMap<AssetId<StandardMaterial>, Color> = app.world().resource::<Assets<StandardMaterial>>().iter().map(|(id, m)| (id, m.base_color)).collect();
        let mut s = String::new();
        let world = app.world_mut();
        let mut q = world.query::<(&GlobalTransform, &Mesh3d, &MeshMaterial3d<StandardMaterial>)>();
        for (g, m, mm) in q.iter(world) {
            let shape = if m.0.id() == cap { "C" } else if m.0.id() == sph { "S" } else if m.0.id() == cube { "B" } else { continue };
            let (sc, r, t) = g.to_scale_rotation_translation();
            let c = colors.get(&mm.0.id()).copied().unwrap_or(Color::WHITE).to_srgba();
            s += &format!("{shape} {} {} {} {} {} {} {} {} {} {} {} {} {}\n", t.x, t.y, t.z, r.x, r.y, r.z, r.w, sc.x, sc.y, sc.z, c.red, c.green, c.blue);
        }
        std::fs::write(path, s).unwrap();
    }
}
