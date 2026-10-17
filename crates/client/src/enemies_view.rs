//! Modelos y animación de los enemigos (puerto de `13-enemigos.js` y `15b-tactico.js`):
//! se crean, mueven y borran siguiendo la lista de la partida, por id.

use crate::game::GameRes;
use crate::player::LocalPlayer;
use arena_sim::enemies::{Alert, DroneKind, Enemy, Kind, SoldierKind, muzzle, turret_dir, turret_muzzle};
use bevy::light::NotShadowCaster;
use bevy::prelude::*;
use std::collections::HashMap;
use std::f32::consts::{FRAC_PI_2, PI};

pub struct EnemiesViewPlugin;

impl Plugin for EnemiesViewPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_assets).add_systems(Update, (sync_enemies, sync_corpses, sync_pools, draw_lasers));
    }
}

#[derive(Resource)]
struct Look {
    cube: Handle<Mesh>,
    octa: Handle<Mesh>,
    core: Handle<Mesh>,
    ring: Handle<Mesh>,
    kami: Handle<Mesh>,
    post: Handle<Mesh>,
    spot: Handle<Mesh>,
    shot: Handle<Mesh>,
    glow: Handle<Mesh>,
    orb: Handle<Mesh>,
    orb_in: Handle<Mesh>,
    m: HashMap<&'static str, Handle<StandardMaterial>>,
}

impl Look {
    fn mat(&self, k: &'static str) -> Handle<StandardMaterial> {
        self.m[k].clone()
    }
}

fn col(h: u32) -> Color {
    crate::world_render::col(h)
}

fn octahedron() -> Mesh {
    let v = [Vec3::X, Vec3::NEG_X, Vec3::Y, Vec3::NEG_Y, Vec3::Z, Vec3::NEG_Z];
    let faces = [(0, 2, 4), (4, 2, 1), (1, 2, 5), (5, 2, 0), (4, 3, 0), (1, 3, 4), (5, 3, 1), (0, 3, 5)];
    let (mut pos, mut nor) = (Vec::new(), Vec::new());
    for (a, b, c) in faces {
        let n = (v[b] - v[a]).cross(v[c] - v[a]).normalize();
        let n = if n.dot(v[a] + v[b] + v[c]) < 0.0 { -n } else { n };
        let (b, c) = if (v[b] - v[a]).cross(v[c] - v[a]).dot(n) < 0.0 { (c, b) } else { (b, c) };
        for i in [a, b, c] {
            pos.push((v[i] * 0.7).to_array());
            nor.push(n.to_array());
        }
    }
    let n = pos.len();
    Mesh::new(bevy::mesh::PrimitiveTopology::TriangleList, bevy::asset::RenderAssetUsages::RENDER_WORLD)
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, pos)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, nor)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0.0f32, 0.0]; n])
}

fn setup_assets(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>, mut mats: ResMut<Assets<StandardMaterial>>) {
    let pbr = |c: u32, metal: f32, rough: f32| StandardMaterial { base_color: col(c), metallic: metal, perceptual_roughness: rough, ..default() };
    let unlit = |c: u32| StandardMaterial { base_color: col(c), unlit: true, ..default() };
    let add = |c: u32, a: f32| StandardMaterial { base_color: col(c).with_alpha(a), unlit: true, alpha_mode: AlphaMode::Add, ..default() };
    let mut m = HashMap::new();
    for (k, v) in [
        ("shell", pbr(0x1b2226, 0.65, 0.35)),
        ("shellH", pbr(0x2b2418, 0.65, 0.35)),
        ("shellK", StandardMaterial { emissive: col(0x401010).to_linear(), ..pbr(0x3a1418, 0.5, 0.4) }),
        ("flash", unlit(0xffffff)),
        ("red", unlit(0xff3d5a)),
        ("amber", unlit(0xffb02e)),
        ("hot", unlit(0xfff0e0)),
        ("orange", unlit(0xff6a2e)),
        ("armor", pbr(0x2b3238, 0.45, 0.55)),
        ("armorC", pbr(0x3e2422, 0.5, 0.5)),
        ("cloth", pbr(0x1f2427, 0.0, 0.95)),
        ("accent", pbr(0xd9792b, 0.0, 0.6)),
        ("gun", pbr(0x15181a, 0.6, 0.4)),
        ("shield", StandardMaterial { alpha_mode: AlphaMode::Blend, base_color: col(0x8c9aa3).with_alpha(0.9), ..pbr(0x8c9aa3, 0.8, 0.3) }),
        ("turret", pbr(0x4b545b, 0.7, 0.4)),
        ("spot", StandardMaterial { cull_mode: None, double_sided: true, ..add(0xfff1b0, 0.12) }),
        ("glowRed", add(0xff3d5a, 0.45)),
        ("glowAmber", add(0xffb02e, 0.45)),
        ("mint", unlit(0x5dffb0)),
        ("cyan", unlit(0x5fd8ff)),
    ] {
        m.insert(k, mats.add(v));
    }
    let mut kami = Cone::new(0.42, 1.1).mesh().resolution(4).build();
    kami.rotate_by(Quat::from_rotation_x(FRAC_PI_2));
    let mut spot = Cone::new(6.0, 16.0).mesh().resolution(20).build();
    spot.translate_by(Vec3::new(0.0, -8.0, 0.0));
    commands.insert_resource(Look {
        cube: meshes.add(Cuboid::new(1.0, 1.0, 1.0)),
        octa: meshes.add(octahedron()),
        core: meshes.add(Sphere::new(0.3).mesh().uv(12, 8)),
        ring: meshes.add(Torus::new(0.92, 1.04).mesh().minor_resolution(6).major_resolution(32)),
        kami: meshes.add(kami),
        post: meshes.add(Cylinder::new(0.26, 1.3).mesh().resolution(10)),
        spot: meshes.add(spot),
        shot: meshes.add(Sphere::new(0.16).mesh().uv(8, 6)),
        glow: meshes.add(Sphere::new(0.34).mesh().uv(8, 6)),
        orb: meshes.add(Sphere::new(0.32).mesh().ico(0).unwrap()),
        orb_in: meshes.add(Sphere::new(0.14).mesh().uv(8, 6)),
        m,
    });
}

/// Piezas animadas de un enemigo.
#[derive(Component, Default)]
struct Rig {
    body: Option<Entity>,
    leg_l: Option<Entity>,
    leg_r: Option<Entity>,
    arms: Option<Entity>,
    visor: Option<Entity>,
    torso: Option<Entity>,
    ring: Option<Entity>,
    ring2: Option<Entity>,
    shell: Option<Entity>,
    head: Option<Entity>,
    eye: Option<Entity>,
    spot: Option<Entity>,
    base_mat: Option<Handle<StandardMaterial>>,
    last_visor: u8,
}

#[derive(Component)]
struct CorpseVis(u32);

struct B<'a, 'w, 's> {
    c: &'a mut Commands<'w, 's>,
    l: &'a Look,
}

impl B<'_, '_, '_> {
    #[allow(clippy::too_many_arguments)]
    fn bx(&mut self, p: Entity, w: f32, h: f32, d: f32, x: f32, y: f32, z: f32, m: Handle<StandardMaterial>) -> Entity {
        // solo el torso proyecta sombra: las piezas chicas no se notan y cuestan una pasada por cascada
        let e = self.c.spawn((Mesh3d(self.l.cube.clone()), MeshMaterial3d(m), Transform::from_xyz(x, y, z).with_scale(Vec3::new(w, h, d)), NotShadowCaster)).id();
        self.c.entity(p).add_child(e);
        e
    }
    fn grp(&mut self, p: Entity, x: f32, y: f32, z: f32) -> Entity {
        let e = self.c.spawn((Transform::from_xyz(x, y, z), Visibility::default())).id();
        self.c.entity(p).add_child(e);
        e
    }
    fn mesh(&mut self, p: Entity, mesh: Handle<Mesh>, m: Handle<StandardMaterial>, t: Transform) -> Entity {
        let e = self.c.spawn((Mesh3d(mesh), MeshMaterial3d(m), t, NotShadowCaster)).id();
        self.c.entity(p).add_child(e);
        e
    }
}

/// Soldado (`buildSoldierModel`): piernas, torso con placa naranja, casco con visor, brazos y arma.
fn soldier(b: &mut B, root: Entity, kind: SoldierKind, rig: &mut Rig) {
    let l = b.l;
    let armor = l.mat(if kind == SoldierKind::Commander { "armorC" } else { "armor" });
    let (cloth, accent, gun) = (l.mat("cloth"), l.mat("accent"), l.mat("gun"));
    let body = b.grp(root, 0.0, 0.0, 0.0);
    let leg_l = b.grp(body, -0.13, 0.95, 0.0);
    let leg_r = b.grp(body, 0.13, 0.95, 0.0);
    for lg in [leg_l, leg_r] {
        b.bx(lg, 0.2, 0.92, 0.24, 0.0, -0.46, 0.0, cloth.clone());
        b.bx(lg, 0.22, 0.2, 0.3, 0.0, -0.86, 0.04, armor.clone());
        b.bx(lg, 0.23, 0.14, 0.08, 0.0, -0.5, 0.13, armor.clone());
    }
    let torso = b.bx(body, 0.54, 0.62, 0.32, 0.0, 1.3, 0.0, armor.clone());
    b.c.entity(torso).remove::<NotShadowCaster>();
    b.bx(body, 0.4, 0.3, 0.06, 0.0, 1.36, 0.17, accent.clone());
    b.bx(body, 0.42, 0.46, 0.2, 0.0, 1.32, -0.24, cloth.clone());
    let head = b.grp(body, 0.0, 1.66, 0.0);
    b.bx(head, 0.27, 0.29, 0.29, 0.0, 0.14, 0.0, cloth.clone());
    b.bx(head, 0.32, 0.15, 0.34, 0.0, 0.3, -0.01, armor.clone());
    let visor_mat = b.l.mat("flash");
    let visor = b.bx(head, 0.25, 0.075, 0.03, 0.0, 0.16, 0.15, visor_mat);
    b.bx(head, 0.33, 0.04, 0.35, 0.0, 0.24, -0.01, accent.clone());
    for sx in [-1.0, 1.0] {
        b.bx(body, 0.18, 0.12, 0.34, sx * 0.33, 1.56, 0.0, armor.clone());
        b.bx(body, 0.19, 0.04, 0.35, sx * 0.33, 1.6, 0.0, accent.clone());
    }
    b.bx(body, 0.56, 0.08, 0.34, 0.0, 1.0, 0.0, gun.clone());
    let arms = b.grp(body, 0.0, 1.45, 0.05);
    b.bx(arms, 0.14, 0.14, 0.5, -0.2, -0.08, 0.2, armor.clone());
    b.bx(arms, 0.14, 0.14, 0.5, 0.2, -0.08, 0.2, armor.clone());
    let sniper = kind == SoldierKind::Sniper;
    b.bx(arms, 0.08, 0.1, if sniper { 1.1 } else { 0.75 }, 0.05, -0.04, if sniper { 0.6 } else { 0.45 }, gun.clone());
    if sniper {
        b.bx(arms, 0.06, 0.06, 0.26, 0.05, 0.05, 0.3, gun.clone());
    }
    if kind == SoldierKind::Commander {
        b.bx(body, 1.0, 1.35, 0.09, -0.12, 1.0, 0.62, b.l.mat("shield"));
        b.bx(body, 0.14, 0.5, 0.1, -0.12, 1.7, 0.62, accent);
    }
    rig.body = Some(body);
    rig.leg_l = Some(leg_l);
    rig.leg_r = Some(leg_r);
    rig.arms = Some(arms);
    rig.visor = Some(visor);
    rig.torso = Some(torso);
    rig.base_mat = Some(armor);
}

fn drone(b: &mut B, root: Entity, dk: DroneKind, surv: bool, rig: &mut Rig) {
    let l = b.l;
    if dk == DroneKind::Kami {
        let shell = b.mesh(root, l.kami.clone(), l.mat("shellK"), Transform::IDENTITY);
        b.bx(root, 1.2, 0.04, 0.3, 0.0, 0.0, 0.25, l.mat("orange"));
        rig.shell = Some(shell);
        rig.base_mat = Some(l.mat("shellK"));
    } else {
        let base = if matches!(dk, DroneKind::Heavy | DroneKind::Boss) { "shellH" } else { "shell" };
        let colm = if dk == DroneKind::Heavy { "amber" } else { "red" };
        let shell = b.mesh(root, l.octa.clone(), l.mat(base), Transform::from_scale(Vec3::new(1.0, 1.15, 1.0)));
        b.mesh(root, l.core.clone(), l.mat(if dk == DroneKind::Boss { "hot" } else { colm }), Transform::from_xyz(0.0, 0.0, 0.55));
        rig.ring = Some(b.mesh(root, l.ring.clone(), l.mat(colm), Transform::IDENTITY));
        if dk == DroneKind::Boss {
            rig.ring2 = Some(b.mesh(root, l.ring.clone(), l.mat("amber"), Transform::from_scale(Vec3::splat(1.3))));
        }
        rig.shell = Some(shell);
        rig.base_mat = Some(l.mat(base));
    }
    if surv {
        let spot = b.c.spawn((Mesh3d(l.spot.clone()), MeshMaterial3d(l.mat("spot")), Transform::default(), NotShadowCaster)).id();
        rig.spot = Some(spot);
    }
}

fn turret(b: &mut B, root: Entity, rig: &mut Rig) {
    let l = b.l;
    let m = l.mat("turret");
    b.bx(root, 0.9, 0.3, 0.9, 0.0, 0.15, 0.0, m.clone());
    b.mesh(root, l.post.clone(), m.clone(), Transform::from_xyz(0.0, 0.9, 0.0));
    let head = b.grp(root, 0.0, 1.7, 0.0);
    let shell = b.bx(head, 0.7, 0.45, 0.8, 0.0, 0.0, 0.0, m.clone());
    b.bx(head, 0.5, 0.1, 0.5, 0.0, 0.27, -0.05, l.mat("accent"));
    for sx in [-0.14, 0.14] {
        b.bx(head, 0.08, 0.08, 0.7, sx, -0.04, 0.6, l.mat("gun"));
    }
    let eye = b.bx(head, 0.16, 0.1, 0.04, 0.0, 0.06, 0.41, l.mat("amber"));
    rig.head = Some(head);
    rig.shell = Some(shell);
    rig.eye = Some(eye);
    rig.base_mat = Some(m);
}

fn spawn_enemy(c: &mut Commands, l: &Look, e: &Enemy) -> Entity {
    let root = c.spawn((Transform::from_translation(e.pos).with_scale(Vec3::splat(e.scale)), Visibility::Hidden)).id();
    let mut rig = Rig { last_visor: 255, ..default() };
    let mut b = B { c, l };
    match e.kind {
        Kind::Soldier(k) => soldier(&mut b, root, k, &mut rig),
        Kind::Drone(d) => drone(&mut b, root, d, e.surv.is_some(), &mut rig),
        Kind::Turret => turret(&mut b, root, &mut rig),
    }
    c.entity(root).insert(rig);
    root
}

fn visor_code(e: &Enemy) -> u8 {
    match e.st {
        Alert::Combat => 2,
        Alert::Search => 1,
        Alert::Patrol => 0,
    }
}

#[allow(clippy::too_many_arguments)]
fn sync_enemies(
    mut commands: Commands,
    time: Res<Time>,
    game: Res<GameRes>,
    lp: Res<LocalPlayer>,
    look: Res<Look>,
    mut roots: Query<(Entity, &mut Rig)>,
    mut tf: Query<&mut Transform>,
    mut mats: Query<&mut MeshMaterial3d<StandardMaterial>>,
    mut vis: Query<&mut Visibility, With<Rig>>,
    mut spot_vis: Query<&mut Visibility, Without<Rig>>,
    mut known: Local<HashMap<u32, Entity>>,
    mut seen_gen: Local<u32>,
) {
    let dt = time.delta_secs();
    let t = time.elapsed_secs();
    let g = &game.0;
    // partida nueva: los ids vuelven a empezar, se descarta todo lo anterior
    if *seen_gen != game.1 {
        *seen_gen = game.1;
        for (_, ent) in known.drain() {
            if let Ok((_, rig)) = roots.get(ent) {
                if let Some(s) = rig.spot {
                    commands.entity(s).despawn();
                }
            }
            commands.entity(ent).despawn();
        }
    }
    let eye = lp.state.eye();
    // nuevos
    for e in &g.enemies {
        if !known.contains_key(&e.id) {
            let ent = spawn_enemy(&mut commands, &look, e);
            known.insert(e.id, ent);
        }
    }
    // borrados
    let alive: std::collections::HashSet<u32> = g.enemies.iter().map(|e| e.id).collect();
    known.retain(|id, ent| {
        if alive.contains(id) {
            return true;
        }
        if let Ok((_, rig)) = roots.get(*ent) {
            if let Some(s) = rig.spot {
                commands.entity(s).despawn();
            }
        }
        commands.entity(*ent).despawn();
        false
    });
    for e in &g.enemies {
        let Some(&ent) = known.get(&e.id) else { continue };
        let Ok((_, mut rig)) = roots.get_mut(ent) else { continue };
        // lejos no se dibujan ni se animan (la niebla y el tamaño los vuelven invisibles igual)
        let far = if e.drone().is_some() { 170.0 } else { 100.0 };
        let near = e.pos.distance(eye) < far && !crate::off("enemies");
        if let Ok(mut v) = vis.get_mut(ent) {
            let want = if near { Visibility::Inherited } else { Visibility::Hidden };
            if *v != want {
                *v = want;
            }
        }
        if let Some(sp) = rig.spot {
            if let Ok(mut v) = spot_vis.get_mut(sp) {
                *v = if near { Visibility::Inherited } else { Visibility::Hidden };
            }
        }
        if !near {
            continue;
        }
        let k = 1.0 - (-30.0 * dt).exp();
        if let Ok(mut rt) = tf.get_mut(ent) {
            let to = e.pos;
            if rt.translation.distance(to) > 4.0 {
                rt.translation = to;
            } else {
                rt.translation = rt.translation.lerp(to, k);
            }
            match e.kind {
                Kind::Drone(dk) => {
                    // mira al jugador (los de vigilancia, hacia donde van)
                    let target = if e.surv.is_some() { e.pos + Vec3::new(e.yaw.sin(), 0.0, e.yaw.cos()) } else if dk == DroneKind::Kami { lp.state.body() } else { eye };
                    let dir = target - rt.translation;
                    if dir.length() > 0.01 {
                        rt.rotation = Transform::IDENTITY.looking_to(-dir, Vec3::Y).rotation;
                    }
                }
                Kind::Turret => rt.rotation = Quat::IDENTITY,
                Kind::Soldier(_) => rt.rotation = Quat::from_rotation_y(e.yaw),
            }
        }
        let flashing = e.flash > 0.0;
        if let (Some(s), Some(base)) = (rig.shell.or(rig.torso), rig.base_mat.clone()) {
            if let Ok(mut m) = mats.get_mut(s) {
                let want = if flashing { look.mat("flash") } else { base };
                if m.0 != want {
                    m.0 = want;
                }
            }
        }
        match e.kind {
            Kind::Drone(dk) => {
                if let Some(r) = rig.ring {
                    if let Ok(mut rt) = tf.get_mut(r) {
                        rt.rotation = Quat::from_rotation_x((t * 2.0 + e.phase).sin() * 0.4) * Quat::from_rotation_z(t * if dk == DroneKind::Heavy { 1.5 } else { 3.0 });
                    }
                }
                if let Some(r) = rig.ring2 {
                    if let Ok(mut rt) = tf.get_mut(r) {
                        rt.rotation = Quat::from_rotation_y(t * 1.2) * Quat::from_rotation_x((t * 0.7).cos() * 0.5);
                    }
                }
                if dk == DroneKind::Kami {
                    if let Some(s) = rig.shell {
                        if let Ok(mut st) = tf.get_mut(s) {
                            st.rotation = Quat::from_rotation_z(t * 8.0);
                        }
                    }
                }
                if let Some(sp) = rig.spot {
                    if let Ok(mut st) = tf.get_mut(sp) {
                        // el cono apunta a un punto del piso 5 m adelante
                        let ground = Vec3::new(e.pos.x + e.yaw.sin() * 5.0, 0.0, e.pos.z + e.yaw.cos() * 5.0);
                        let dir = (ground - e.pos).normalize();
                        st.translation = e.pos;
                        st.rotation = Quat::from_rotation_arc(Vec3::NEG_Y, dir);
                    }
                }
            }
            Kind::Soldier(_) => {
                let sw = if e.moving { (t * 9.0 + e.phase).sin() * 0.55 } else { 0.0 };
                if let Some(x) = rig.leg_l.and_then(|x| tf.get_mut(x).ok()).as_mut() {
                    x.rotation = Quat::from_rotation_x(sw - e.crouch * 1.2);
                }
                if let Some(x) = rig.leg_r.and_then(|x| tf.get_mut(x).ok()).as_mut() {
                    x.rotation = Quat::from_rotation_x(-sw - e.crouch * 0.2);
                }
                if let Some(x) = rig.body.and_then(|x| tf.get_mut(x).ok()).as_mut() {
                    x.translation.y = -e.crouch * 0.48;
                }
                if let Some(x) = rig.arms.and_then(|x| tf.get_mut(x).ok()).as_mut() {
                    x.rotation = Quat::from_rotation_x(-e.pitch);
                }
                let vc = visor_code(e);
                if vc != rig.last_visor {
                    rig.last_visor = vc;
                    if let Some(v) = rig.visor {
                        if let Ok(mut m) = mats.get_mut(v) {
                            m.0 = look.mat(["cyan", "amber", "red"][vc as usize]);
                        }
                    }
                }
            }
            Kind::Turret => {
                if let Some(h) = rig.head {
                    if let Ok(mut ht) = tf.get_mut(h) {
                        ht.rotation = Quat::from_rotation_y(e.yaw) * Quat::from_rotation_x(e.pitch);
                    }
                }
                let powered = g.city.as_ref().is_none_or(|c| c.powered);
                let code = if !powered { 3 } else if e.st == Alert::Combat { 2 } else { 1 };
                if code != rig.last_visor {
                    rig.last_visor = code;
                    if let Some(v) = rig.eye {
                        if let Ok(mut m) = mats.get_mut(v) {
                            m.0 = look.mat(match code {
                                2 => "red",
                                1 => "amber",
                                _ => "cloth",
                            });
                        }
                    }
                }
            }
        }
    }
}

/// Cadáveres: el mismo modelo de soldado, caído y con el visor apagado.
fn sync_corpses(mut commands: Commands, game: Res<GameRes>, look: Res<Look>, mut q: Query<(Entity, &CorpseVis, &mut Transform)>, mut known: Local<HashMap<u32, Entity>>, mut seen_gen: Local<u32>) {
    let g = &game.0;
    if *seen_gen != game.1 {
        *seen_gen = game.1;
        for (_, e) in known.drain() {
            commands.entity(e).despawn();
        }
    }
    for c in &g.corpses {
        if known.contains_key(&c.id) {
            continue;
        }
        let root = commands.spawn((Transform::from_translation(c.pos).with_rotation(Quat::from_rotation_y(c.yaw)), Visibility::default(), CorpseVis(c.id))).id();
        let mut rig = Rig::default();
        let mut b = B { c: &mut commands, l: &look };
        soldier(&mut b, root, c.kind, &mut rig);
        if let Some(v) = rig.visor {
            commands.entity(v).insert(MeshMaterial3d(look.mat("cloth")));
        }
        if c.kind == SoldierKind::Commander {
            commands.entity(root).insert(Transform::from_translation(c.pos).with_rotation(Quat::from_rotation_y(c.yaw)).with_scale(Vec3::splat(1.18)));
        }
        known.insert(c.id, root);
    }
    let ids: std::collections::HashSet<u32> = g.corpses.iter().map(|c| c.id).collect();
    known.retain(|id, e| {
        let keep = ids.contains(id);
        if !keep {
            commands.entity(*e).despawn();
        }
        keep
    });
    for (_, cv, mut t) in &mut q {
        if let Some(c) = g.corpses.iter().find(|c| c.id == cv.0) {
            let k = (c.t / 0.5).min(1.0);
            t.translation = c.pos;
            t.rotation = Quat::from_rotation_y(c.yaw) * Quat::from_rotation_x(-k * PI / 2.0 * c.fall * 0.98);
        }
    }
}

#[derive(Component)]
struct ShotVis(usize);
#[derive(Component)]
struct OrbVis(usize);

/// Proyectiles de los drones y orbes: grupos fijos que se reubican cada cuadro.
fn sync_pools(mut commands: Commands, time: Res<Time>, game: Res<GameRes>, look: Res<Look>, mut shots: Query<(&ShotVis, &mut Transform, &mut Visibility), Without<OrbVis>>, mut orbs: Query<(&OrbVis, &mut Transform, &mut Visibility), Without<ShotVis>>, mut made: Local<bool>) {
    if !*made {
        *made = true;
        for i in 0..64 {
            commands.spawn((Mesh3d(look.shot.clone()), MeshMaterial3d(look.mat("hot")), Transform::default(), Visibility::Hidden, NotShadowCaster, ShotVis(i))).with_child((Mesh3d(look.glow.clone()), MeshMaterial3d(look.mat("glowRed")), NotShadowCaster));
        }
        for i in 0..16 {
            commands.spawn((Mesh3d(look.orb.clone()), MeshMaterial3d(look.mat("mint")), Transform::default(), Visibility::Hidden, NotShadowCaster, OrbVis(i))).with_child((Mesh3d(look.orb_in.clone()), MeshMaterial3d(look.mat("mint"))));
        }
        return;
    }
    let g = &game.0;
    for (s, mut t, mut v) in &mut shots {
        match g.shots.get(s.0) {
            Some(sh) => {
                t.translation = sh.pos;
                *v = Visibility::Inherited;
            }
            None => *v = Visibility::Hidden,
        }
    }
    let tt = time.elapsed_secs();
    for (o, mut t, mut v) in &mut orbs {
        match g.orbs.get(o.0) {
            Some(orb) => {
                t.translation = orb.pos + Vec3::Y * (0.8 + (tt * 3.0 + o.0 as f32).sin() * 0.15);
                t.rotation = Quat::from_rotation_y(tt * 2.0) * Quat::from_rotation_x(tt);
                t.scale = Vec3::splat(if orb.od { 1.25 } else { 1.0 });
                // parpadea los últimos 4 s
                *v = if orb.life > 4.0 || (tt * 20.0).sin() > 0.0 { Visibility::Inherited } else { Visibility::Hidden };
            }
            None => *v = Visibility::Hidden,
        }
    }
}

/// Láseres de torretas y francotiradores.
fn draw_lasers(game: Res<GameRes>, lp: Res<LocalPlayer>, mut gizmos: Gizmos) {
    let g = &game.0;
    let powered = g.city.as_ref().is_none_or(|c| c.powered);
    for e in &g.enemies {
        match e.kind {
            Kind::Turret if powered => {
                let m = turret_muzzle(e);
                gizmos.line(m, m + turret_dir(e) * e.laser_len, col(0xff3d5a).with_alpha(0.55));
            }
            Kind::Soldier(SoldierKind::Sniper) if e.st == Alert::Combat && e.aim_t > 0.05 => {
                let a = 0.3 + e.aim_t / 2.6 * 0.7;
                gizmos.line(muzzle(e), lp.state.body() + Vec3::Y * 0.2, col(0xff2a3a).with_alpha(a));
            }
            _ => {}
        }
    }
}
