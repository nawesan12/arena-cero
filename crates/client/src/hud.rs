//! HUD (puerto parcial de `19-hud...js` y del CSS de la web): cruz que se abre con la
//! dispersión, vida, postura, dash, munición, arma y ranuras, granadas, recarga, viñetas de
//! daño y de apuntado, y la mira del riel. Colores de la web (`--ink`, `--dim`, `--amber`…).

use crate::player::LocalPlayer;
use arena_sim::weapons::WEAPONS;
use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions};

const INK: Color = Color::srgb(0.902, 0.929, 0.910);
const DIM: Color = Color::srgb(0.498, 0.549, 0.529);
const AMBER: Color = Color::srgb(1.0, 0.690, 0.180);
const RED: Color = Color::srgb(1.0, 0.239, 0.353);
const MINT: Color = Color::srgb(0.365, 1.0, 0.690);
const CYAN: Color = Color::srgb(0.373, 0.847, 1.0);

pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(FrameTimeDiagnosticsPlugin::default()).add_systems(Startup, spawn_hud).add_systems(Update, update_hud);
    }
}

#[derive(Component, Clone, Copy, PartialEq, Eq)]
enum T {
    Stance,
    Info,
    Fps,
    Hp,
    Ammo,
    Mag,
    WName,
    Reload,
    Slot(usize),
}

#[derive(Component)]
struct Help;
#[derive(Component)]
struct DashFill;
#[derive(Component)]
struct HpFill;
#[derive(Component)]
struct CrossBar(usize);
#[derive(Component)]
struct CrossDot;
#[derive(Component)]
struct NadePip(u32);
#[derive(Component, Clone, Copy)]
enum Overlay {
    Hurt,
    Aim,
    Scope,
}

/// Fuentes de la web: Saira Condensed para títulos y Share Tech Mono para números.
#[derive(Resource, Clone)]
pub struct Fonts {
    pub display: Handle<Font>,
    pub mono: Handle<Font>,
}

fn label(text: &str, size: f32, color: Color, font: &Handle<Font>) -> impl Bundle {
    (Text::new(text), TextFont { font: FontSource::Handle(font.clone()), font_size: size.into(), ..default() }, TextColor(color))
}

/// Viñeta radial: transparente en el centro y del color dado en los bordes.
fn vignette(c: [f32; 3], inner: f32, alpha: f32) -> Image {
    let s = 256;
    let mut px = vec![[0.0f32; 4]; s * s];
    for y in 0..s {
        for x in 0..s {
            let (u, v) = ((x as f32 + 0.5) / s as f32 * 2.0 - 1.0, (y as f32 + 0.5) / s as f32 * 2.0 - 1.0);
            let d = (u * u + v * v).sqrt() / std::f32::consts::SQRT_2;
            let a = ((d - inner) / (1.0 - inner)).clamp(0.0, 1.0) * alpha;
            px[y * s + x] = [c[0], c[1], c[2], a];
        }
    }
    crate::textures::to_image(s, px, true, false)
}

/// Lente de la mira del riel: negro fuera del círculo, cruz cian cortada en el centro y punto rojo.
fn scope_lens() -> Image {
    let s = 512;
    let mut px = vec![[0.0f32; 4]; s * s];
    let h = s as f32 / 2.0;
    for y in 0..s {
        for x in 0..s {
            let (dx, dy) = (x as f32 + 0.5 - h, y as f32 + 0.5 - h);
            let d = dx.hypot(dy) / h;
            let c = if d > 1.0 {
                [0.016, 0.024, 0.027, 1.0]
            } else {
                // sombra interna del borde y tinte cian
                let edge = ((d - 0.8) / 0.2).clamp(0.0, 1.0);
                let mut c = [0.0, 0.0, 0.0, edge * edge * 0.8];
                if d > 0.62 {
                    c = [0.37 * 0.07, 0.85 * 0.07, 0.07, c[3].max((d - 0.62) / 0.38 * 0.07)];
                }
                let gap = (dx.abs() < 0.08 * h && dy.abs() < 0.08 * h) || dx.abs().max(dy.abs()) < 0.08 * h;
                if !gap && (dx.abs() < 0.6 || dy.abs() < 0.6) {
                    c = [0.373, 0.847, 1.0, 0.75];
                }
                if dx.hypot(dy) < 2.5 {
                    c = [1.0, 0.24, 0.35, 1.0];
                }
                c
            };
            px[y * s + x] = c;
        }
    }
    crate::textures::to_image(s, px, true, false)
}

fn spawn_hud(mut commands: Commands, mut font_assets: ResMut<Assets<Font>>, mut images: ResMut<Assets<Image>>) {
    let f = Fonts {
        display: font_assets.add(Font::from_bytes(include_bytes!("../assets/fonts/SairaCondensed-Bold.ttf").to_vec())),
        mono: font_assets.add(Font::from_bytes(include_bytes!("../assets/fonts/ShareTechMono-Regular.ttf").to_vec())),
    };
    commands.insert_resource(f.clone());
    let full = Node { position_type: PositionType::Absolute, width: percent(100), height: percent(100), ..default() };
    // viñetas (debajo de todo lo demás)
    commands.spawn((full.clone(), ImageNode::new(images.add(vignette([0.0, 0.0, 0.0], 0.55, 0.55))), Visibility::Hidden, Overlay::Aim));
    commands.spawn((full.clone(), ImageNode::new(images.add(vignette([1.0, 0.157, 0.275], 0.45, 0.55))), Visibility::Hidden, Overlay::Hurt));
    commands.spawn((Visibility::Hidden, Overlay::Scope, ZIndex(1), Node { justify_content: JustifyContent::Center, align_items: AlignItems::Center, ..full.clone() })).with_children(|p| {
        p.spawn((Node { width: vmin(80), height: vmin(80), ..default() }, ImageNode::new(images.add(scope_lens())), Outline { width: vmax(200), offset: px(0), color: Color::srgb(0.016, 0.024, 0.027) }));
    });
    // cruz: cuatro trazos que se abren con la dispersión
    commands.spawn(Node { position_type: PositionType::Absolute, left: percent(50), top: percent(50), ..default() }).with_children(|p| {
        for i in 0..4 {
            let (w, h) = if i < 2 { (2.0, 8.0) } else { (8.0, 2.0) };
            p.spawn((Node { position_type: PositionType::Absolute, width: px(w), height: px(h), ..default() }, BackgroundColor(INK.with_alpha(0.9)), CrossBar(i)));
        }
        p.spawn((Node { position_type: PositionType::Absolute, left: px(-1), top: px(-1), width: px(2), height: px(2), ..default() }, BackgroundColor(AMBER), CrossDot));
    });
    // arriba a la izquierda
    // posición y velocidad, chico abajo al centro (útil para probar)
    commands.spawn((Node { position_type: PositionType::Absolute, bottom: px(22), width: percent(100), justify_content: JustifyContent::Center, ..default() },)).with_children(|p| {
        p.spawn((label("", 11.0, DIM.with_alpha(0.7), &f.mono), T::Info, TextLayout::justify(Justify::Center)));
    });
    // abajo a la izquierda: vida, postura y dash
    commands.spawn(Node { position_type: PositionType::Absolute, left: px(16), bottom: px(16), flex_direction: FlexDirection::Column, row_gap: px(4), ..default() }).with_children(|p| {
        p.spawn(label("INTEGRIDAD", 12.0, DIM, &f.display));
        p.spawn((label("100", 44.0, INK, &f.mono), T::Hp));
        p.spawn((Node { width: px(220), height: px(10), ..default() }, BackgroundColor(Color::WHITE.with_alpha(0.08)))).with_children(|b| {
            b.spawn((Node { width: percent(100), height: percent(100), ..default() }, BackgroundColor(MINT), HpFill));
        });
        p.spawn((label("", 12.0, AMBER, &f.display), T::Stance));
        p.spawn(Node { flex_direction: FlexDirection::Row, column_gap: px(8), align_items: AlignItems::Center, ..default() }).with_children(|r| {
            r.spawn(label("Q DASH", 12.0, DIM, &f.display));
            r.spawn((Node { width: px(90), height: px(4), ..default() }, BackgroundColor(Color::WHITE.with_alpha(0.1)))).with_children(|b| {
                b.spawn((Node { width: percent(100), height: percent(100), ..default() }, BackgroundColor(CYAN), DashFill));
            });
        });
    });
    // abajo a la derecha: munición, arma, ranuras y granadas
    commands.spawn(Node { position_type: PositionType::Absolute, right: px(16), bottom: px(16), flex_direction: FlexDirection::Column, align_items: AlignItems::End, row_gap: px(4), ..default() }).with_children(|p| {
        p.spawn((label("", 14.0, AMBER, &f.display), T::Reload));
        p.spawn(Node { flex_direction: FlexDirection::Row, align_items: AlignItems::Baseline, column_gap: px(6), ..default() }).with_children(|r| {
            r.spawn((label("24", 52.0, INK, &f.mono), T::Ammo));
            r.spawn((label("/ 24", 22.0, DIM, &f.mono), T::Mag));
        });
        p.spawn((label("", 12.0, DIM, &f.display), T::WName));
        p.spawn(Node { flex_direction: FlexDirection::Row, column_gap: px(10), ..default() }).with_children(|r| {
            for (i, w) in WEAPONS.iter().enumerate() {
                r.spawn((label(&format!("{} {}", i + 1, w.short), 13.0, DIM, &f.display), T::Slot(i)));
            }
        });
        p.spawn(Node { flex_direction: FlexDirection::Row, column_gap: px(6), align_items: AlignItems::Center, ..default() }).with_children(|r| {
            r.spawn(label("G", 12.0, DIM, &f.display));
            for i in 0..4 {
                r.spawn((Node { width: px(10), height: px(10), border: UiRect::all(px(2)), ..default() }, BorderColor::all(AMBER), BackgroundColor(Color::NONE), NadePip(i)));
            }
        });
    });
    commands.spawn((Node { position_type: PositionType::Absolute, bottom: px(8), left: percent(50), ..default() }, label("", 11.0, DIM, &f.mono), T::Fps));
    commands.spawn((
        Node { position_type: PositionType::Absolute, top: percent(38), width: percent(100), justify_content: JustifyContent::Center, ..default() },
        label("Click para jugar · WASD moverse · Shift correr · C agacharse/deslizar · Espacio saltar (doble) · Q dash · E o Ctrl gancho\nClic izquierdo disparar · Clic derecho apuntar · 1-4 o rueda cambiar arma · R recargar · G granada · F1/F2/F3 calidad · Esc suelta el mouse", 15.0, INK, &f.display),
        TextLayout::justify(Justify::Center),
        Help,
    ));
}

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
fn update_hud(
    time: Res<Time>,
    lp: Res<LocalPlayer>,
    diag: Res<DiagnosticsStore>,
    cursor: Single<&CursorOptions>,
    mut texts: Query<(&mut Text, &mut TextColor, &T)>,
    mut fills: Query<(&mut Node, &mut BackgroundColor, Option<&DashFill>), (Or<(With<DashFill>, With<HpFill>)>, Without<CrossBar>, Without<NadePip>)>,
    mut bars: Query<(&mut Node, &mut Visibility, &CrossBar), (Without<DashFill>, Without<HpFill>, Without<NadePip>)>,
    mut dot: Query<&mut Visibility, (With<CrossDot>, Without<CrossBar>, Without<Help>, Without<Overlay>)>,
    mut pips: Query<(&mut Node, &mut BackgroundColor, &NadePip), (Without<DashFill>, Without<HpFill>, Without<CrossBar>)>,
    mut help: Query<&mut Visibility, (With<Help>, Without<CrossBar>, Without<CrossDot>, Without<Overlay>)>,
    mut overlays: Query<(&mut Visibility, &Overlay, Option<&mut ImageNode>), (Without<Help>, Without<CrossBar>, Without<CrossDot>)>,
) {
    let s = &lp.state;
    let a = &lp.arsenal;
    let wd = &WEAPONS[a.w];
    let stance = if a.overdrive > 0.0 {
        format!("SOBRECARGA {} s", a.overdrive.ceil())
    } else if s.dash_t > 0.0 {
        "IMPULSO".into()
    } else if s.slide > 0.0 {
        "DESLIZANDO".into()
    } else if s.hook.is_some() {
        "GANCHO".into()
    } else if s.crouch_t > 0.5 {
        "AGACHADO".into()
    } else {
        String::new()
    };
    let p = s.pos;
    let info = format!("x {:.0}  z {:.0}  altura {:.1} m  ·  {:.1} m/s", p.x, p.z, s.feet(), s.horizontal_speed());
    let fps = diag.get(&FrameTimeDiagnosticsPlugin::FPS).and_then(|d| d.smoothed()).map(|f| format!("{f:.0} FPS"));
    let mag = a.mag_of(a.w);
    let ammo = a.ammo[a.w];
    let low = ammo <= mag.div_ceil(4);
    let reload = if a.reload > 0.0 {
        "RECARGANDO"
    } else if low && ammo > 0 {
        "R PARA RECARGAR"
    } else {
        ""
    };
    for (mut t, mut c, k) in &mut texts {
        let v = match *k {
            T::Stance => stance.clone(),
            T::Info => info.clone(),
            T::Fps => match &fps {
                Some(f) => f.clone(),
                None => continue,
            },
            T::Hp => format!("{}", lp.hp.ceil()),
            T::Ammo => {
                c.0 = if low { RED } else { INK };
                format!("{ammo}")
            }
            T::Mag => format!("/ {mag}"),
            T::WName => wd.name.to_uppercase(),
            T::Reload => reload.into(),
            T::Slot(i) => {
                c.0 = if i == a.w { AMBER } else if !a.unlocked[i] { DIM.with_alpha(0.3) } else { DIM };
                continue;
            }
        };
        if **t != v {
            **t = v;
        }
    }
    let hr = lp.hp / lp.max_hp;
    let dk = (1.0 - s.dash_cd.max(0.0) / (3.0 * s.dash_cd_mul)).clamp(0.0, 1.0);
    for (mut n, mut bg, is_dash) in &mut fills {
        if is_dash.is_some() {
            n.width = percent(dk * 100.0);
            bg.0 = CYAN.with_alpha(if s.dash_cd <= 0.0 { 1.0 } else { 0.45 });
        } else {
            n.width = percent(hr * 100.0);
            bg.0 = if hr > 0.5 { MINT } else if hr > 0.25 { AMBER } else { RED };
        }
    }
    for (mut n, mut bg, pip) in &mut pips {
        n.display = if pip.0 < a.max_nades { Display::Flex } else { Display::None };
        bg.0 = if pip.0 < a.nades { AMBER } else { Color::NONE };
    }
    // la cruz se abre con el movimiento y el arma; al apuntar desaparece
    let moving = s.horizontal_speed() > 1.0 || !s.on_ground;
    let gap = ((if moving { wd.spread[1] } else { wd.spread[0] }) * 600.0 + a.recoil * 6.0) * (1.0 - s.crouch_t * 0.4);
    let aim_on = s.aim_t > 0.5;
    for (mut n, mut v, b) in &mut bars {
        let (l, t) = match b.0 {
            0 => (-1.0, -14.0 - gap),
            1 => (-1.0, 6.0 + gap),
            2 => (-14.0 - gap, -1.0),
            _ => (6.0 + gap, -1.0),
        };
        n.left = px(l);
        n.top = px(t);
        *v = if aim_on { Visibility::Hidden } else { Visibility::Inherited };
    }
    for mut v in &mut dot {
        *v = if aim_on { Visibility::Hidden } else { Visibility::Inherited };
    }
    let rail = if a.w == 2 { ((s.aim_t - 0.6) / 0.3).clamp(0.0, 1.0) } else { 0.0 };
    let low_hp = if lp.hp < 30.0 { 0.35 + (time.elapsed_secs() / 0.18).sin() * 0.1 } else { 0.0 };
    for (mut v, o, img) in &mut overlays {
        let alpha = match o {
            Overlay::Hurt => lp.hurt_flash.max(low_hp),
            Overlay::Aim => s.aim_t * (1.0 - rail),
            Overlay::Scope => rail,
        };
        *v = if alpha > 0.01 { Visibility::Inherited } else { Visibility::Hidden };
        if let Some(mut img) = img {
            img.color = Color::WHITE.with_alpha(alpha);
        }
    }
    let grabbed = cursor.grab_mode != CursorGrabMode::None;
    for mut v in &mut help {
        *v = if grabbed { Visibility::Hidden } else { Visibility::Visible };
    }
}
