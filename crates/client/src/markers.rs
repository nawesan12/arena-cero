//! Marcadores en pantalla (puerto de `updateMarkers` de la web): objetivos con distancia (fuera
//! de cuadro quedan en el borde), cuenta de la carga, reabastecimiento cercano, barra de
//! detección sobre los soldados, "!" cuando te ven y triángulo sobre los enemigos marcados.
//! También el cartel de interacción con F y la caída de la antena.

use crate::game::GameRes;
use crate::hud::Fonts;
use crate::player::{LocalPlayer, MainCamera};
use crate::world_render::{GroupVisual, LevelRes};
use arena_sim::enemies::Alert;
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions};

const INK: Color = Color::srgb(0.902, 0.929, 0.910);
const AMBER: Color = Color::srgb(1.0, 0.690, 0.180);
const RED: Color = Color::srgb(1.0, 0.239, 0.353);
const MINT: Color = Color::srgb(0.365, 1.0, 0.690);
const DARK: Color = Color::srgb(0.08, 0.06, 0.04);
const GOLD: Color = Color::srgb(1.0, 0.84, 0.3);
const POOL: usize = 64;

pub struct MarkersPlugin;

impl Plugin for MarkersPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostStartup, spawn).add_systems(Update, (markers, prompt, antenna));
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Style {
    /// Rombo con letra y texto (objetivos, carga).
    Obj(Color),
    /// Cruz verde (reabastecimiento).
    Supply,
    /// Barra de detección (0..1).
    Det(f32),
    /// "!" rojo: te está viendo.
    Seen,
    /// Triángulo rojo: marcado.
    Tag,
    /// Compañero online: rombo de su color con su número y el nombre.
    Mate(Color),
}

#[derive(Component)]
struct Mk(usize);
#[derive(Component)]
struct MkIcon(usize);
#[derive(Component)]
struct MkIconText(usize);
#[derive(Component)]
struct MkLabel(usize);
#[derive(Component)]
struct MkBar(usize);
#[derive(Component)]
struct MkBarFill(usize);
#[derive(Component)]
struct PromptRoot;
#[derive(Component)]
struct PromptText;
#[derive(Component)]
struct PromptFill;

fn spawn(mut commands: Commands, f: Res<Fonts>) {
    for i in 0..POOL {
        commands
            .spawn((Node { position_type: PositionType::Absolute, width: px(120), margin: UiRect { left: px(-60), top: px(-11), ..default() }, flex_direction: FlexDirection::Column, align_items: AlignItems::Center, row_gap: px(2), ..default() }, Visibility::Hidden, Mk(i)))
            .with_children(|p| {
                p.spawn((Node { width: px(20), height: px(20), justify_content: JustifyContent::Center, align_items: AlignItems::Center, ..default() }, MkIcon(i))).with_children(|d| {
                    d.spawn((Node { position_type: PositionType::Absolute, width: px(14), height: px(14), ..default() }, BackgroundColor(AMBER), UiTransform { rotation: Rot2::degrees(45.0), ..default() }));
                    d.spawn((Text::new(""), TextFont { font: FontSource::Handle(f.display.clone()), font_size: 12.0.into(), ..default() }, TextColor(DARK), MkIconText(i)));
                });
                p.spawn((Node { width: px(26), height: px(5), border: UiRect::all(px(1)), ..default() }, BorderColor::all(INK.with_alpha(0.8)), BackgroundColor(Color::BLACK.with_alpha(0.3)), Visibility::Hidden, MkBar(i))).with_children(|b| {
                    b.spawn((Node { width: percent(0), height: percent(100), ..default() }, BackgroundColor(AMBER), MkBarFill(i)));
                });
                p.spawn((Text::new(""), TextFont { font: FontSource::Handle(f.display.clone()), font_size: 12.0.into(), ..default() }, TextColor(INK), TextShadow::default(), MkLabel(i)));
            });
    }
    commands.spawn((Node { position_type: PositionType::Absolute, bottom: px(150), width: percent(100), flex_direction: FlexDirection::Column, align_items: AlignItems::Center, row_gap: px(6), ..default() }, Visibility::Hidden, PromptRoot)).with_children(|p| {
        p.spawn((Text::new(""), TextFont { font: FontSource::Handle(f.display.clone()), font_size: 17.0.into(), ..default() }, TextColor(INK), TextShadow::default(), PromptText));
        p.spawn((Node { width: px(180), height: px(4), ..default() }, BackgroundColor(Color::WHITE.with_alpha(0.15)))).with_children(|b| {
            b.spawn((Node { width: percent(0), height: percent(100), ..default() }, BackgroundColor(AMBER), PromptFill));
        });
    });
}

struct Item {
    pos: Vec3,
    style: Style,
    icon: String,
    label: String,
    clamp: bool,
}

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
fn markers(
    game: Res<GameRes>,
    level: Res<LevelRes>,
    lp: Res<LocalPlayer>,
    cursor: Single<&CursorOptions>,
    win: Single<&Window>,
    target: Option<Res<crate::render_scale::SceneTarget>>,
    cam: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    net: Res<crate::online::Online>,
    mut roots: Query<(&Mk, &mut Node, &mut Visibility), (Without<MkBar>, Without<MkBarFill>, Without<MkIcon>)>,
    mut icons: Query<(&MkIcon, &mut Visibility, &Children), (Without<Mk>, Without<MkBar>)>,
    mut icon_bg: Query<(&mut BackgroundColor, &mut UiTransform), (Without<MkBarFill>, Without<MkBar>)>,
    mut icon_txt: Query<(&MkIconText, &mut Text), Without<MkLabel>>,
    mut labels: Query<(&MkLabel, &mut Text, &mut TextColor), Without<MkIconText>>,
    mut bars: Query<(&MkBar, &mut Visibility, &mut BorderColor), (Without<Mk>, Without<MkIcon>)>,
    mut fills: Query<(&MkBarFill, &mut Node, &mut BackgroundColor), (Without<Mk>, Without<MkBar>)>,
) {
    let g = &game.0;
    let mut items: Vec<Item> = Vec::new();
    let playing = cursor.grab_mode != CursorGrabMode::None;
    let me = lp.state.pos;
    if playing {
        if let (Some(c), Some(run)) = (level.0.city.as_ref(), g.city.as_ref()) {
            for (i, op) in c.outposts.iter().enumerate() {
                let st = &run.ops[i];
                if st.done || op.camp || st.asleep || g.campaign.is_some() {
                    continue;
                }
                let cmd = g.enemies.iter().find(|e| e.op == Some(i) && e.kind == arena_sim::enemies::Kind::Soldier(arena_sim::enemies::SoldierKind::Commander));
                let (pos, up) = match (op.obj_pos, cmd) {
                    (Some(p), _) => (p, 1.6),
                    (None, Some(e)) => (e.pos, 3.0),
                    _ => (Vec3::new(op.cx, 0.0, op.cz), 3.0),
                };
                let d = (pos.x - me.x).hypot(pos.z - me.z);
                items.push(Item { pos: pos + Vec3::Y * up, style: Style::Obj(if st.alarm > 0.0 { RED } else { AMBER }), icon: op.icon.to_string(), label: format!("{} · {} m", op.name.to_uppercase(), d.round()), clamp: true });
            }
            if let Some(cp) = &g.campaign {
                for (k, p) in cp.marks.iter().enumerate() {
                    let d = (p.x - me.x).hypot(p.z - me.z);
                    let label = if k == 0 { format!("{} · {} m", cp.objective.to_uppercase(), d.round()) } else { format!("{} m", d.round()) };
                    items.push(Item { pos: *p + Vec3::Y * 0.8, style: Style::Obj(GOLD), icon: format!("{}", cp.step + 1), label, clamp: true });
                }
            }
            if let Some(ch) = run.obj.charge {
                if let Some(p) = c.outposts[ch.op].obj_pos {
                    items.push(Item { pos: p + Vec3::Y, style: Style::Obj(RED), icon: "!".into(), label: format!("{:.1} S", ch.t.max(0.0)), clamp: true });
                }
            }
            for (p, name) in g.supplies(&level.0) {
                let d = p.distance(me);
                if d < 70.0 {
                    items.push(Item { pos: p + Vec3::Y * 1.2, style: Style::Supply, icon: "+".into(), label: if d < 8.0 { name.to_uppercase() } else { format!("{} m", d.round()) }, clamp: false });
                }
            }
        }
        // compañeros online: nombre y distancia (caído, en rojo)
        for o in &net.others {
            let d = o.pos.distance(me);
            let c = crate::world_render::col(crate::humanoid::TEAM[o.id as usize % crate::humanoid::TEAM.len()]);
            let name = net.name_of(o.id).to_uppercase();
            let label = if o.alive { if d < 25.0 { name } else { format!("{name} · {} m", d.round()) } } else { format!("{name} · CAÍDO") };
            items.push(Item { pos: o.pos + Vec3::Y * 0.75, style: Style::Mate(if o.alive { c } else { RED }), icon: format!("{}", o.id + 1), label, clamp: true });
        }
        // misiones activas y botín cercano
        for m in &g.director.missions {
            let d = (m.pos.x - me.x).hypot(m.pos.z - me.z);
            let up = if m.kind == arena_sim::director::MissionKind::Cargo { 1.5 } else { 2.6 };
            items.push(Item { pos: m.pos + Vec3::Y * up, style: Style::Obj(Color::srgb(1.0, 0.42, 0.18)), icon: "!".into(), label: format!("{} · {} m · {}:{:02}", m.kind.title(), d.round(), (m.t_left.max(0.0) / 60.0) as u32, (m.t_left.max(0.0) % 60.0) as u32), clamp: true });
        }
        for l in &g.loot {
            let d = l.pos.distance(me);
            if d < 14.0 {
                items.push(Item { pos: l.pos + Vec3::Y * 1.1, style: Style::Supply, icon: "$".into(), label: String::new(), clamp: false });
            }
        }
        let intel = g.city.as_ref().is_some_and(|r| r.intel);
        for e in &g.enemies {
            if !e.alive() {
                continue;
            }
            let d = (e.pos.x - me.x).hypot(e.pos.z - me.z);
            let p = e.pos + Vec3::Y * e.marker_h();
            if e.is_soldier() && e.st != Alert::Combat && e.det > 0.04 && d < 60.0 {
                items.push(Item { pos: p, style: Style::Det(e.det), icon: String::new(), label: String::new(), clamp: false });
            } else if e.is_soldier() && e.st == Alert::Combat && e.visible && d < 80.0 {
                items.push(Item { pos: p, style: Style::Seen, icon: "!".into(), label: String::new(), clamp: false });
            } else if (e.tagged || intel) && d < 110.0 && !e.is_turret() {
                items.push(Item { pos: p, style: Style::Tag, icon: String::new(), label: String::new(), clamp: false });
            } else if let Some(s) = e.surv.as_ref().filter(|s| s.det > 0.05) {
                items.push(Item { pos: e.pos + Vec3::Y * 1.5, style: Style::Det(s.det), icon: String::new(), label: String::new(), clamp: false });
            }
        }
    }
    items.truncate(POOL);
    let Ok((camera, ct)) = cam.single() else { return };
    let k = target.map_or(1.0, |t| win.width() / t.size.x.max(1) as f32);
    let (w, h) = (win.width(), win.height());
    let view = ct.compute_transform();
    let mut placed: Vec<Option<(Vec2, bool)>> = Vec::with_capacity(items.len());
    for it in &items {
        let local = view.rotation.inverse() * (it.pos - view.translation);
        let behind = local.z > 0.0;
        let sp = camera.world_to_viewport(ct, it.pos).ok().map(|v| v * k);
        let on = !behind && sp.is_some_and(|v| v.x >= 0.0 && v.y >= 0.0 && v.x <= w && v.y <= h);
        if on {
            placed.push(Some((sp.unwrap(), false)));
        } else if it.clamp {
            // fuera de cuadro: sobre una elipse alrededor de la mira, según el rumbo
            let th = local.x.atan2(-local.z);
            placed.push(Some((Vec2::new(w / 2.0 + th.sin() * w * 0.4, h / 2.0 - th.cos() * h * 0.3), true)));
        } else {
            placed.push(None);
        }
    }
    for (m, mut n, mut v) in &mut roots {
        match (items.get(m.0), placed.get(m.0).copied().flatten()) {
            (Some(_), Some((p, _))) => {
                n.left = px(p.x);
                n.top = px(p.y);
                *v = Visibility::Inherited;
            }
            _ => *v = Visibility::Hidden,
        }
    }
    for (ic, mut v, children) in &mut icons {
        let Some(it) = items.get(ic.0) else { continue };
        let show = !matches!(it.style, Style::Det(_));
        *v = if show { Visibility::Inherited } else { Visibility::Hidden };
        if let Some(&d) = children.first() {
            if let Ok((mut bg, mut tr)) = icon_bg.get_mut(d) {
                let (c, round) = match it.style {
                    Style::Obj(c) => (c, false),
                    Style::Mate(c) => (c, true),
                    Style::Supply => (MINT, true),
                    Style::Seen => (RED, true),
                    Style::Tag => (RED, false),
                    Style::Det(_) => (AMBER, false),
                };
                bg.0 = c;
                tr.rotation = Rot2::degrees(if round { 0.0 } else { 45.0 });
                tr.scale = Vec2::splat(if it.style == Style::Tag { 0.55 } else { 1.0 });
            }
        }
    }
    for (t, mut tx) in &mut icon_txt {
        if let Some(it) = items.get(t.0) {
            if **tx != it.icon {
                **tx = it.icon.clone();
            }
        }
    }
    for (l, mut tx, mut c) in &mut labels {
        let Some(it) = items.get(l.0) else { continue };
        // fuera de cuadro solo el ícono
        let label = if placed.get(l.0).copied().flatten().is_some_and(|p| p.1) { String::new() } else { it.label.clone() };
        if **tx != label {
            **tx = label;
        }
        c.0 = INK;
    }
    for (b, mut v, mut bc) in &mut bars {
        let Some(it) = items.get(b.0) else { continue };
        match it.style {
            Style::Det(f) => {
                *v = Visibility::Inherited;
                *bc = BorderColor::all(if f > 0.45 { AMBER } else { INK.with_alpha(0.8) });
            }
            _ => *v = Visibility::Hidden,
        }
    }
    for (b, mut n, mut bg) in &mut fills {
        if let Some(Item { style: Style::Det(f), .. }) = items.get(b.0) {
            n.width = percent(f * 100.0);
            bg.0 = if *f > 0.45 { AMBER } else { INK };
        }
    }
}

fn prompt(game: Res<GameRes>, level: Res<LevelRes>, lp: Res<LocalPlayer>, cursor: Single<&CursorOptions>, mut root: Query<&mut Visibility, With<PromptRoot>>, mut text: Query<&mut Text, With<PromptText>>, mut fill: Query<(&mut Node, &mut Visibility), (With<PromptFill>, Without<PromptRoot>)>) {
    let mut p = game.0.city.as_ref().and_then(|r| r.obj.prompt.clone()).filter(|_| cursor.grab_mode != CursorGrabMode::None);
    // en el andén se toma el subte
    if game.0.driving.is_none() && cursor.grab_mode != CursorGrabMode::None && crate::subway::platform_near(&level.0, lp.state.pos).is_some() {
        p = Some(arena_sim::objectives::Prompt { label: "F · TOMAR EL SUBTE".into(), frac: None });
    }
    // en un punto de reabastecimiento también está la tienda
    if game.0.driving.is_none() && cursor.grab_mode != CursorGrabMode::None && game.0.shop_near(&level.0, lp.state.pos) {
        let shop = "B · TIENDA";
        p = Some(match p {
            Some(mut q) => {
                q.label = format!("{}   ·   {shop}", q.label);
                q
            }
            None => arena_sim::objectives::Prompt { label: shop.into(), frac: None },
        });
    }
    for mut v in &mut root {
        *v = if p.is_some() { Visibility::Inherited } else { Visibility::Hidden };
    }
    if let Some(p) = p {
        for mut t in &mut text {
            if **t != p.label {
                **t = p.label.clone();
            }
        }
        for (mut n, mut v) in &mut fill {
            n.width = percent(p.frac.unwrap_or(0.0) * 100.0);
            *v = if p.frac.is_some() { Visibility::Inherited } else { Visibility::Hidden };
        }
    }
}

/// La antena volada se inclina y cae en 2,4 s.
fn antenna(game: Res<GameRes>, mut q: Query<&mut Transform, With<GroupVisual>>) {
    let t = game.0.city.as_ref().and_then(|r| r.obj.antenna_fall).unwrap_or(0.0);
    let k = (t / 2.4).min(1.0);
    for mut tr in &mut q {
        tr.rotation = Quat::from_rotation_z(-k * k * std::f32::consts::FRAC_PI_2 * 0.96);
    }
}
