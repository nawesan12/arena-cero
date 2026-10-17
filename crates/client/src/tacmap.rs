//! Mapa táctico con Tab (puerto de `drawBigMap`): planta del distrito generada una vez a partir
//! de los colisionadores, y encima puestos, reabastecimiento, enemigos conocidos y el jugador.

use crate::game::GameRes;
use crate::hud::Fonts;
use crate::player::{LocalPlayer, Look};
use crate::world_render::LevelRes;
use arena_sim::city::{BC_ALL, BLOCK};
use arena_sim::enemies::Alert;
use bevy::prelude::*;

const SIZE: usize = 1024;
const INK: Color = Color::srgb(0.902, 0.929, 0.910);
const AMBER: Color = Color::srgb(1.0, 0.690, 0.180);
const RED: Color = Color::srgb(1.0, 0.239, 0.353);
const MINT: Color = Color::srgb(0.365, 1.0, 0.690);
const ORANGE: Color = Color::srgb(1.0, 0.416, 0.180);
const GREY: Color = Color::srgb(0.333, 0.376, 0.416);

pub struct TacMapPlugin;

impl Plugin for TacMapPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostStartup, spawn).add_systems(Update, update);
    }
}

#[derive(Component)]
struct Root;
#[derive(Component)]
struct MapBox;
#[derive(Component)]
struct OpBox(usize);
#[derive(Component)]
struct OpLabel(usize);
#[derive(Component)]
struct Dot(usize);
#[derive(Component)]
struct Me;

/// Planta: manzanas y edificios coloreados por altura.
fn plan(level: &arena_sim::Level) -> Image {
    let a = level.info.a;
    let k = SIZE as f32 / (a * 2.0);
    let mut px = vec![[0.125f32, 0.157, 0.173, 1.0]; SIZE * SIZE];
    let mut rect = |x0: f32, z0: f32, x1: f32, z1: f32, c: [f32; 4]| {
        let (ix0, iz0) = (((x0 + a) * k).max(0.0) as usize, ((z0 + a) * k).max(0.0) as usize);
        let (ix1, iz1) = ((((x1 + a) * k).ceil() as usize).min(SIZE), (((z1 + a) * k).ceil() as usize).min(SIZE));
        for z in iz0..iz1.max(iz0 + 1).min(SIZE) {
            for x in ix0..ix1.max(ix0 + 1).min(SIZE) {
                px[z * SIZE + x] = c;
            }
        }
    };
    if level.info.city {
        for bx in BC_ALL {
            for bz in BC_ALL {
                rect(bx - BLOCK / 2.0, bz - BLOCK / 2.0, bx + BLOCK / 2.0, bz + BLOCK / 2.0, [0.196, 0.231, 0.251, 1.0]);
            }
        }
    }
    for b in level.world.colliders() {
        let h = b.max.y;
        if h < 2.4 || b.max.x - b.min.x > 60.0 || b.max.z - b.min.z > 60.0 {
            continue;
        }
        let c = if h > 12.0 { [0.427, 0.471, 0.502, 1.0] } else if h > 5.0 { [0.333, 0.376, 0.416, 1.0] } else { [0.275, 0.314, 0.353, 1.0] };
        rect(b.min.x, b.min.z, b.max.x, b.max.z, c);
    }
    crate::textures::to_image(SIZE, px, false, false)
}

fn spawn(mut commands: Commands, level: Res<LevelRes>, f: Res<Fonts>, mut images: ResMut<Assets<Image>>) {
    let img = images.add(plan(&level.0));
    let t = |s: &str, size: f32, c: Color| (Text::new(s), TextFont { font: FontSource::Handle(f.display.clone()), font_size: size.into(), ..default() }, TextColor(c), TextShadow::default());
    commands
        .spawn((Node { position_type: PositionType::Absolute, width: percent(100), height: percent(100), justify_content: JustifyContent::Center, align_items: AlignItems::Center, flex_direction: FlexDirection::Column, row_gap: px(8), ..default() }, BackgroundColor(Color::srgba(0.03, 0.047, 0.055, 0.88)), GlobalZIndex(8), Visibility::Hidden, Root))
        .with_children(|p| {
            p.spawn((Node { width: vmin(84), height: vmin(84), ..default() }, ImageNode::new(img), MapBox)).with_children(|m| {
                let n = level.0.city.as_ref().map_or(0, |c| c.outposts.len());
                for i in 0..n {
                    m.spawn((Node { position_type: PositionType::Absolute, border: UiRect::all(px(2)), ..default() }, BorderColor::all(AMBER), OpBox(i)));
                    m.spawn((Node { position_type: PositionType::Absolute, width: px(220), margin: UiRect::left(px(-110)), justify_content: JustifyContent::Center, ..default() }, OpLabel(i))).with_children(|l| {
                        l.spawn(t("", 13.0, AMBER));
                    });
                }
                for i in 0..96 {
                    m.spawn((Node { position_type: PositionType::Absolute, width: px(6), height: px(6), margin: UiRect { left: px(-3), top: px(-3), ..default() }, border_radius: BorderRadius::MAX, ..default() }, BackgroundColor(RED), Visibility::Hidden, Dot(i)));
                }
                m.spawn((Node { position_type: PositionType::Absolute, width: px(0), height: px(0), ..default() }, UiTransform::default(), Me)).with_children(|a| {
                    // flecha: un triángulo armado con un rombo estirado y un punto
                    a.spawn((Node { position_type: PositionType::Absolute, left: px(-5), top: px(-11), width: px(10), height: px(16), ..default() }, BackgroundColor(INK), border_radius_arrow()));
                });
            });
            p.spawn(t("ÁMBAR: PUESTO ACTIVO · ROJO: EN ALERTA · VERDE: LIBERADO · CRUZ: REABASTECIMIENTO", 12.0, INK.with_alpha(0.6)));
        });
}

fn border_radius_arrow() -> Outline {
    Outline { width: px(1), offset: px(0), color: Color::BLACK.with_alpha(0.6) }
}

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
fn update(
    key: Res<ButtonInput<KeyCode>>,
    game: Res<GameRes>,
    level: Res<LevelRes>,
    lp: Res<LocalPlayer>,
    look: Res<Look>,
    mut root: Query<&mut Visibility, (With<Root>, Without<Dot>)>,
    mapbox: Query<&ComputedNode, With<MapBox>>,
    mut ops: Query<(&OpBox, &mut Node, &mut BorderColor), (Without<Dot>, Without<OpLabel>, Without<Me>)>,
    mut labels: Query<(&OpLabel, &mut Node, &Children), (Without<Dot>, Without<OpBox>, Without<Me>)>,
    mut label_txt: Query<(&mut Text, &mut TextColor)>,
    mut dots: Query<(&Dot, &mut Node, &mut Visibility, &mut BackgroundColor), (Without<OpBox>, Without<OpLabel>, Without<Me>)>,
    mut me: Query<(&mut Node, &mut UiTransform), (With<Me>, Without<Dot>, Without<OpBox>, Without<OpLabel>)>,
) {
    let open = (key.pressed(KeyCode::Tab) || std::env::var_os("ARENA_TAB").is_some()) && level.0.info.city;
    for mut v in &mut root {
        *v = if open { Visibility::Inherited } else { Visibility::Hidden };
    }
    if !open {
        return;
    }
    let Ok(cn) = mapbox.single() else { return };
    let side = cn.size().x * cn.inverse_scale_factor();
    let a = level.0.info.a;
    let k = side / (a * 2.0);
    let to = |x: f32, z: f32| Vec2::new((x + a) * k, (z + a) * k);
    let g = &game.0;
    let (Some(c), Some(run)) = (level.0.city.as_ref(), g.city.as_ref()) else { return };
    for (o, mut n, mut bc) in &mut ops {
        let op = &c.outposts[o.0];
        let st = &run.ops[o.0];
        let p = to(op.cx - 16.0, op.cz - 16.0);
        n.left = px(p.x);
        n.top = px(p.y);
        n.width = px(32.0 * k);
        n.height = px(32.0 * k);
        *bc = BorderColor::all(if st.done { MINT } else if st.alarm > 0.0 { RED } else { AMBER });
    }
    for (l, mut n, children) in &mut labels {
        let op = &c.outposts[l.0];
        let st = &run.ops[l.0];
        let p = to(op.cx, op.cz - 21.0);
        n.left = px(p.x);
        n.top = px(p.y);
        if let Some(&ch) = children.first() {
            if let Ok((mut t, mut tc)) = label_txt.get_mut(ch) {
                let v = format!("{} · {}", op.icon, op.full.to_uppercase());
                if **t != v {
                    **t = v;
                }
                tc.0 = if st.done { MINT } else if st.alarm > 0.0 { RED } else { AMBER };
            }
        }
    }
    // reabastecimiento y enemigos conocidos
    let mut pts: Vec<(Vec2, Color, f32)> = g.supplies(&level.0).iter().map(|(p, _)| (to(p.x, p.z), MINT, 9.0)).collect();
    for e in &g.enemies {
        if !e.alive() {
            continue;
        }
        let known = e.is_turret() || e.surv.is_some() || e.hunting || e.tagged || run.intel || (e.is_soldier() && e.st == Alert::Combat && e.see_t < 3.0);
        if !known {
            continue;
        }
        let col = if e.is_turret() { if run.powered { AMBER } else { GREY } } else if e.surv.is_some() || e.hunting { ORANGE } else { RED };
        let big = e.kind == arena_sim::enemies::Kind::Soldier(arena_sim::enemies::SoldierKind::Commander);
        pts.push((to(e.pos.x, e.pos.z), col, if big { 10.0 } else { 6.0 }));
    }
    for (d, mut n, mut v, mut bg) in &mut dots {
        match pts.get(d.0) {
            Some(&(p, col, s)) => {
                n.left = px(p.x);
                n.top = px(p.y);
                n.width = px(s);
                n.height = px(s);
                n.margin = UiRect { left: px(-s / 2.0), top: px(-s / 2.0), ..default() };
                bg.0 = col;
                *v = Visibility::Inherited;
            }
            None => *v = Visibility::Hidden,
        }
    }
    for (mut n, mut tr) in &mut me {
        let p = to(lp.state.pos.x, lp.state.pos.z);
        n.left = px(p.x);
        n.top = px(p.y);
        // el norte del mapa es −Z; yaw 0 mira al norte
        tr.rotation = Rot2::radians(-look.yaw);
    }
}
