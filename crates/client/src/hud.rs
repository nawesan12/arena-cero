//! HUD mínimo de esta etapa: cruz, postura, barra de dash, posición y FPS.
//! Colores de la web (`--ink`, `--dim`, `--amber`, `--cyan`).

use crate::player::{LocalPlayer, Look};
use crate::world_render::LevelRes;
use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions};

const INK: Color = Color::srgb(0.902, 0.929, 0.910);
const DIM: Color = Color::srgb(0.498, 0.549, 0.529);
const AMBER: Color = Color::srgb(1.0, 0.690, 0.180);
const CYAN: Color = Color::srgb(0.373, 0.847, 1.0);

pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(FrameTimeDiagnosticsPlugin::default()).add_systems(Startup, spawn_hud).add_systems(Update, update_hud);
    }
}

#[derive(Component)]
struct Stance;
#[derive(Component)]
struct DashFill;
#[derive(Component)]
struct Info;
#[derive(Component)]
struct Fps;
#[derive(Component)]
struct Help;
#[derive(Component)]
struct CrossBar(usize);

/// Fuentes de la web: Saira Condensed para títulos y Share Tech Mono para números.
#[derive(Resource, Clone)]
pub struct Fonts {
    pub display: Handle<Font>,
    pub mono: Handle<Font>,
}

fn label(text: &str, size: f32, color: Color, font: &Handle<Font>) -> impl Bundle {
    (Text::new(text), TextFont { font: FontSource::Handle(font.clone()), font_size: size.into(), ..default() }, TextColor(color))
}

fn spawn_hud(mut commands: Commands, mut font_assets: ResMut<Assets<Font>>) {
    let f = Fonts {
        display: font_assets.add(Font::from_bytes(include_bytes!("../assets/fonts/SairaCondensed-Bold.ttf").to_vec())),
        mono: font_assets.add(Font::from_bytes(include_bytes!("../assets/fonts/ShareTechMono-Regular.ttf").to_vec())),
    };
    commands.insert_resource(f.clone());
    // cruz: cuatro trazos que se abren con el movimiento
    commands.spawn((Node { position_type: PositionType::Absolute, left: percent(50), top: percent(50), ..default() },)).with_children(|p| {
        for i in 0..4 {
            let (w, h) = if i < 2 { (2.0, 8.0) } else { (8.0, 2.0) };
            p.spawn((Node { position_type: PositionType::Absolute, width: px(w), height: px(h), ..default() }, BackgroundColor(INK.with_alpha(0.9)), CrossBar(i)));
        }
        p.spawn((Node { position_type: PositionType::Absolute, left: px(-1), top: px(-1), width: px(2), height: px(2), ..default() }, BackgroundColor(AMBER)));
    });
    // arriba a la izquierda
    commands.spawn(Node { position_type: PositionType::Absolute, left: px(16), top: px(14), flex_direction: FlexDirection::Column, ..default() }).with_children(|p| {
        p.spawn((label("", 14.0, DIM, &f.mono), Info));
    });
    // abajo a la izquierda: postura y dash
    commands.spawn(Node { position_type: PositionType::Absolute, left: px(16), bottom: px(16), flex_direction: FlexDirection::Column, row_gap: px(6), ..default() }).with_children(|p| {
        p.spawn(label("INTEGRIDAD", 12.0, DIM, &f.display));
        p.spawn(label("100", 44.0, INK, &f.mono));
        p.spawn((label("", 12.0, AMBER, &f.display), Stance));
        p.spawn(Node { flex_direction: FlexDirection::Row, column_gap: px(8), align_items: AlignItems::Center, ..default() }).with_children(|r| {
            r.spawn(label("Q DASH", 12.0, DIM, &f.display));
            r.spawn((Node { width: px(90), height: px(4), ..default() }, BackgroundColor(Color::WHITE.with_alpha(0.1)))).with_children(|b| {
                b.spawn((Node { width: percent(100), height: percent(100), ..default() }, BackgroundColor(CYAN), DashFill));
            });
        });
    });
    commands.spawn((Node { position_type: PositionType::Absolute, bottom: px(8), left: percent(50), ..default() }, label("", 11.0, DIM, &f.mono), Fps));
    commands.spawn((
        Node { position_type: PositionType::Absolute, top: percent(38), width: percent(100), justify_content: JustifyContent::Center, ..default() },
        label("Click para jugar · WASD moverse · Shift correr · C agacharse/deslizar · Espacio saltar (doble) · Q dash · E o Ctrl gancho · Clic derecho apuntar · Esc suelta el mouse", 15.0, INK, &f.display),
        TextLayout::justify(Justify::Center),
        Help,
    ));
}

#[allow(clippy::type_complexity)]
fn update_hud(
    lp: Res<LocalPlayer>,
    look: Res<Look>,
    level: Res<LevelRes>,
    diag: Res<DiagnosticsStore>,
    cursor: Single<&CursorOptions>,
    mut texts: ParamSet<(Query<&mut Text, With<Stance>>, Query<&mut Text, With<Info>>, Query<&mut Text, With<Fps>>)>,
    mut dash: Query<(&mut Node, &mut BackgroundColor), (With<DashFill>, Without<CrossBar>)>,
    mut bars: Query<(&mut Node, &CrossBar), Without<DashFill>>,
    mut help: Query<&mut Visibility, With<Help>>,
) {
    let s = &lp.state;
    let stance = if s.dash_t > 0.0 {
        "IMPULSO"
    } else if s.slide > 0.0 {
        "DESLIZANDO"
    } else if s.hook.is_some() {
        "GANCHO"
    } else if s.crouch_t > 0.5 {
        "AGACHADO"
    } else {
        ""
    };
    for mut t in &mut texts.p0() {
        **t = stance.to_string();
    }
    let p = s.pos;
    let info = format!("{} · {} m\nx {:.0}  z {:.0}  altura {:.1} m  ·  {:.1} m/s", level.0.info.name, level.0.info.size, p.x, p.z, s.feet(), s.horizontal_speed());
    for mut t in &mut texts.p1() {
        **t = info.clone();
    }
    if let Some(fps) = diag.get(&FrameTimeDiagnosticsPlugin::FPS).and_then(|d| d.smoothed()) {
        for mut t in &mut texts.p2() {
            **t = format!("{fps:.0} FPS");
        }
    }
    let k = (1.0 - s.dash_cd.max(0.0) / (3.0 * s.dash_cd_mul)).clamp(0.0, 1.0);
    for (mut n, mut bg) in &mut dash {
        n.width = percent(k * 100.0);
        bg.0 = CYAN.with_alpha(if s.dash_cd <= 0.0 { 1.0 } else { 0.45 });
    }
    // la cruz se abre con el movimiento (dispersión del rifle: 0,004 quieto, 0,012 en movimiento)
    let moving = s.horizontal_speed() > 1.0 || !s.on_ground;
    let gap = ((if moving { 0.012 } else { 0.004 }) * 600.0 * (1.0 - s.crouch_t * 0.4)) * (1.0 - s.aim_t);
    for (mut n, b) in &mut bars {
        let (l, t) = match b.0 {
            0 => (-1.0, -14.0 - gap),
            1 => (-1.0, 6.0 + gap),
            2 => (-14.0 - gap, -1.0),
            _ => (6.0 + gap, -1.0),
        };
        n.left = px(l);
        n.top = px(t);
    }
    let _ = look;
    let grabbed = cursor.grab_mode != CursorGrabMode::None;
    for mut v in &mut help {
        *v = if grabbed { Visibility::Hidden } else { Visibility::Visible };
    }
}
