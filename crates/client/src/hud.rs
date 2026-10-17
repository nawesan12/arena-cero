//! Interfaz en juego, rediseñada con estética de shooter: brújula con objetivos arriba, vitales y
//! habilidades abajo a la izquierda, arma abajo a la derecha, mira dinámica con marcas de impacto,
//! indicador de dirección del daño y pantalla de pausa con los controles.
//! Paleta y fuentes de la web (Saira Condensed + Share Tech Mono).

use crate::game::{GameRes, UiFeed};
use crate::player::{LocalPlayer, MainCamera};
use crate::world_render::LevelRes;
use arena_sim::game::{Mark, Mode};
use arena_sim::weapons::WEAPONS;
use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions};
use std::f32::consts::{PI, TAU};

const INK: Color = Color::srgb(0.902, 0.929, 0.910);
const DIM: Color = Color::srgb(0.498, 0.549, 0.529);
const AMBER: Color = Color::srgb(1.0, 0.690, 0.180);
const RED: Color = Color::srgb(1.0, 0.239, 0.353);
const MINT: Color = Color::srgb(0.365, 1.0, 0.690);
const CYAN: Color = Color::srgb(0.373, 0.847, 1.0);
const PANEL: Color = Color::srgba(0.04, 0.06, 0.07, 0.55);
const DARK: Color = Color::srgb(0.08, 0.06, 0.04);
/// Objetivo de la campaña.
const GOLD: Color = Color::srgb(1.0, 0.84, 0.3);

const COMPASS_W: f32 = 520.0;
/// Grados visibles en la brújula.
const COMPASS_FOV: f32 = 130.0;
const HP_SEGS: usize = 10;

pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(FrameTimeDiagnosticsPlugin::default())
            .insert_resource(ShowFps(crate::config::get("fps").is_some_and(|v| v == "1")))
            .add_systems(Startup, spawn_hud)
            .add_systems(Update, (tick_feed, vitals, weapon, top, compass, crosshair, overlays, numbers, pause_screen, fps, hook_ring));
    }
}

#[derive(Resource)]
struct ShowFps(bool);

/// Fuentes de la web: Saira Condensed para títulos y Share Tech Mono para números.
#[derive(Resource, Clone)]
pub struct Fonts {
    pub display: Handle<Font>,
    pub mono: Handle<Font>,
}

/// Textos que se actualizan.
#[derive(Component, Clone, Copy, PartialEq, Eq)]
enum T {
    Hp,
    Ammo,
    Mag,
    WName,
    RKey,
    Wave,
    WaveSub,
    Lives,
    Score,
    Combo,
    BTitle,
    BSub,
    Alert,
    Fps,
    Popup,
    PopupLabel,
    Slot(usize),
    Credits,
    CreditPop,
    Clock,
    Gadget,
}

#[derive(Component)]
struct HpSeg(usize);
#[derive(Component)]
struct ChipFill(u8);
#[derive(Component)]
struct ReloadFill;
#[derive(Component)]
struct NadePip(u32);
#[derive(Component)]
struct Slots;
#[derive(Component)]
struct CrossBar(usize);
#[derive(Component)]
struct CrossDot;
#[derive(Component)]
struct HookRing;
#[derive(Component)]
struct HitRoot;
#[derive(Component)]
struct HitBar;
#[derive(Component)]
struct DmgArc;
#[derive(Component)]
struct BossRoot;
#[derive(Component)]
struct BossFill;
#[derive(Component)]
struct AlertPill;
#[derive(Component)]
struct ObjPip(usize);
#[derive(Component)]
struct ObjPipText(usize);
#[derive(Component)]
struct CompassTick(usize);
#[derive(Component)]
struct CompassMark(usize);
#[derive(Component)]
struct CompassMarkText(usize);
#[derive(Component)]
struct Pause;
#[derive(Component)]
struct PauseMap;
#[derive(Component)]
struct NumText(usize);
#[derive(Component)]
struct PopupRoot;
#[derive(Component, Clone, Copy)]
enum Overlay {
    Hurt,
    Aim,
    Scope,
}

fn txt(text: &str, size: f32, color: Color, font: &Handle<Font>) -> impl Bundle {
    (Text::new(text), TextFont { font: FontSource::Handle(font.clone()), font_size: size.into(), ..default() }, TextColor(color))
}

/// Imagen RGBA a partir de una función por píxel (u, v en −1..1).
fn paint(s: usize, f: impl Fn(f32, f32) -> [f32; 4]) -> Image {
    let mut px = vec![[0.0f32; 4]; s * s];
    for y in 0..s {
        for x in 0..s {
            px[y * s + x] = f((x as f32 + 0.5) / s as f32 * 2.0 - 1.0, (y as f32 + 0.5) / s as f32 * 2.0 - 1.0);
        }
    }
    crate::textures::to_image(s, px, true, false)
}

fn vignette(c: [f32; 3], inner: f32, alpha: f32) -> Image {
    paint(256, |u, v| {
        let d = (u * u + v * v).sqrt() / std::f32::consts::SQRT_2;
        [c[0], c[1], c[2], ((d - inner) / (1.0 - inner)).clamp(0.0, 1.0) * alpha]
    })
}

/// Arco rojo arriba del centro: se gira hacia de dónde vino el golpe.
fn dmg_arc() -> Image {
    paint(256, |u, v| {
        let r = (u * u + v * v).sqrt();
        let ang = u.atan2(-v);
        let band = (1.0 - ((r - 0.86) / 0.07).abs()).clamp(0.0, 1.0);
        let spread = (1.0 - (ang.abs() / 0.42)).clamp(0.0, 1.0);
        [1.0, 0.24, 0.35, band * spread.powf(0.7)]
    })
}

fn scope_lens() -> Image {
    paint(512, |u, v| {
        let d = (u * u + v * v).sqrt();
        if d > 1.0 {
            return [0.016, 0.024, 0.027, 1.0];
        }
        let edge = ((d - 0.8) / 0.2).clamp(0.0, 1.0);
        let mut c = [0.0, 0.0, 0.0, edge * edge * 0.8];
        let px = 1.0 / 256.0;
        let gap = u.abs().max(v.abs()) < 0.08;
        if !gap && (u.abs() < px * 1.2 || v.abs() < px * 1.2) {
            c = [0.373, 0.847, 1.0, 0.75];
        }
        if d < px * 5.0 {
            c = [1.0, 0.24, 0.35, 1.0];
        }
        c
    })
}

/// Panel oscuro con una esquina redondeada de más (como los botones de la web).
fn panel() -> impl Bundle {
    BackgroundColor(PANEL)
}

fn panel_radius() -> BorderRadius {
    BorderRadius::new(px(2), px(12), px(2), px(2))
}

fn spawn_hud(mut commands: Commands, mut font_assets: ResMut<Assets<Font>>, mut images: ResMut<Assets<Image>>) {
    let f = Fonts {
        display: font_assets.add(Font::from_bytes(include_bytes!("../assets/fonts/SairaCondensed-Bold.ttf").to_vec())),
        mono: font_assets.add(Font::from_bytes(include_bytes!("../assets/fonts/ShareTechMono-Regular.ttf").to_vec())),
    };
    commands.insert_resource(f.clone());
    let full = Node { position_type: PositionType::Absolute, width: percent(100), height: percent(100), ..default() };
    let centered = Node { justify_content: JustifyContent::Center, align_items: AlignItems::Center, ..full.clone() };

    // viñetas y mira del riel
    commands.spawn((full.clone(), ImageNode::new(images.add(vignette([0.0, 0.0, 0.0], 0.55, 0.55))).with_mode(NodeImageMode::Stretch), Visibility::Hidden, Overlay::Aim));
    commands.spawn((full.clone(), ImageNode::new(images.add(vignette([1.0, 0.157, 0.275], 0.45, 0.55))).with_mode(NodeImageMode::Stretch), Visibility::Hidden, Overlay::Hurt));
    commands.spawn((centered.clone(), Visibility::Hidden, Overlay::Scope)).with_children(|p| {
        p.spawn((Node { width: vmin(80), height: vmin(80), ..default() }, ImageNode::new(images.add(scope_lens())), Outline { width: vmax(200), offset: px(0), color: Color::srgb(0.016, 0.024, 0.027) }));
    });

    // indicador de dirección del daño
    commands.spawn(centered.clone()).with_children(|p| {
        p.spawn((Node { width: px(300), height: px(300), ..default() }, ImageNode::new(images.add(dmg_arc())), UiTransform::default(), Visibility::Hidden, DmgArc));
    });

    // mira
    let center = Node { position_type: PositionType::Absolute, left: percent(50), top: percent(50), ..default() };
    commands.spawn(center.clone()).with_children(|p| {
        for i in 0..4 {
            let (w, h) = if i < 2 { (2.0, 9.0) } else { (9.0, 2.0) };
            p.spawn((Node { position_type: PositionType::Absolute, width: px(w), height: px(h), ..default() }, BackgroundColor(INK.with_alpha(0.92)), Outline { width: px(1), offset: px(0), color: Color::BLACK.with_alpha(0.35) }, CrossBar(i)));
        }
        p.spawn((Node { position_type: PositionType::Absolute, left: px(-1), top: px(-1), width: px(2), height: px(2), ..default() }, BackgroundColor(AMBER), CrossDot));
        // anillo cian: lo apuntado se puede enganchar
        p.spawn((Node { position_type: PositionType::Absolute, left: px(-20), top: px(-20), width: px(40), height: px(40), border: UiRect::all(px(1)), border_radius: BorderRadius::MAX, ..default() }, BorderColor::all(CYAN.with_alpha(0.6)), Visibility::Hidden, HookRing));
    });
    commands.spawn((center, UiTransform::default(), Visibility::Hidden, HitRoot)).with_children(|p| {
        for (a, x, y) in [(-45.0f32, 1.0, 1.0), (-45.0, -1.0, -1.0), (45.0, 1.0, -1.0), (45.0, -1.0, 1.0)] {
            p.spawn((
                Node { position_type: PositionType::Absolute, left: px(x * 10.0 - 1.0), top: px(y * 10.0 - 5.0), width: px(2), height: px(10), ..default() },
                BackgroundColor(INK),
                UiTransform { rotation: Rot2::degrees(a), ..default() },
                HitBar,
            ));
        }
    });
    // "+120" debajo de la mira
    commands.spawn((Node { position_type: PositionType::Absolute, top: percent(50), margin: UiRect::top(px(30)), width: percent(100), flex_direction: FlexDirection::Column, align_items: AlignItems::Center, ..default() }, PopupRoot, Visibility::Hidden)).with_children(|p| {
        p.spawn((txt("", 20.0, AMBER, &f.mono), T::Popup));
        p.spawn((txt("", 11.0, INK, &f.display), T::PopupLabel));
    });

    // arriba al centro: brújula, alerta y jefe
    commands.spawn(Node { position_type: PositionType::Absolute, top: px(12), width: percent(100), flex_direction: FlexDirection::Column, align_items: AlignItems::Center, row_gap: px(6), ..default() }).with_children(|p| {
        p.spawn(Node { width: px(COMPASS_W), height: px(34), overflow: Overflow::clip(), ..default() }).with_children(|c| {
            c.spawn((Node { position_type: PositionType::Absolute, left: px(0), bottom: px(4), width: px(COMPASS_W), height: px(1), ..default() }, BackgroundColor(INK.with_alpha(0.25))));
            c.spawn((Node { position_type: PositionType::Absolute, left: px(COMPASS_W / 2.0 - 1.0), bottom: px(0), width: px(2), height: px(10), ..default() }, BackgroundColor(AMBER)));
            for i in 0..24 {
                let lbl = match i * 15 {
                    0 => "N",
                    45 => "NE",
                    90 => "E",
                    135 => "SE",
                    180 => "S",
                    225 => "SO",
                    270 => "O",
                    315 => "NO",
                    _ => "",
                };
                c.spawn((Node { position_type: PositionType::Absolute, bottom: px(5), flex_direction: FlexDirection::Column, align_items: AlignItems::Center, width: px(30), margin: UiRect::left(px(-15)), ..default() }, CompassTick(i))).with_children(|t| {
                    if !lbl.is_empty() {
                        t.spawn(txt(lbl, if lbl.len() == 1 { 14.0 } else { 11.0 }, if lbl == "N" { AMBER } else { INK }, &f.display));
                    }
                    t.spawn((Node { width: px(if lbl.is_empty() { 1.0 } else { 2.0 }), height: px(if lbl.is_empty() { 5.0 } else { 7.0 }), ..default() }, BackgroundColor(INK.with_alpha(if lbl.is_empty() { 0.4 } else { 0.8 }))));
                });
            }
            for i in 0..12 {
                c.spawn((Node { position_type: PositionType::Absolute, top: px(1), width: px(18), height: px(18), margin: UiRect::left(px(-9)), justify_content: JustifyContent::Center, align_items: AlignItems::Center, ..default() }, Visibility::Hidden, CompassMark(i))).with_children(|m| {
                    m.spawn((Node { position_type: PositionType::Absolute, width: px(12), height: px(12), ..default() }, BackgroundColor(AMBER), UiTransform { rotation: Rot2::degrees(45.0), ..default() }));
                    m.spawn((txt("", 10.0, DARK, &f.display), CompassMarkText(i)));
                });
            }
        });
        p.spawn((Node { padding: UiRect::axes(px(10), px(2)), border: UiRect::all(px(1)), ..default() }, BorderColor::all(MINT), BackgroundColor(PANEL), AlertPill, Visibility::Hidden)).with_children(|b| {
            b.spawn((txt("OCULTO", 12.0, MINT, &f.display), T::Alert));
        });
        p.spawn((Node { flex_direction: FlexDirection::Column, align_items: AlignItems::Center, ..default() }, BossRoot, Visibility::Hidden)).with_children(|b| {
            b.spawn(txt("COLOSO", 13.0, RED, &f.display));
            b.spawn((Node { width: px(420), height: px(8), margin: UiRect::top(px(3)), ..default() }, BackgroundColor(Color::WHITE.with_alpha(0.08)))).with_children(|bb| {
                bb.spawn((Node { width: percent(100), height: percent(100), ..default() }, BackgroundColor(RED), BossFill));
            });
        });
    });

    // arriba a la izquierda: objetivos (ciudad) u oleada
    commands.spawn(Node { position_type: PositionType::Absolute, left: px(18), top: px(16), flex_direction: FlexDirection::Column, row_gap: px(6), ..default() }).with_children(|p| {
        p.spawn(Node { flex_direction: FlexDirection::Row, align_items: AlignItems::Baseline, column_gap: px(8), ..default() }).with_children(|r| {
            r.spawn((txt("", 28.0, AMBER, &f.display), T::Wave));
            r.spawn((txt("", 22.0, INK, &f.mono), T::Clock));
            r.spawn((txt("", 13.0, DIM, &f.display), T::WaveSub));
        });
        p.spawn(Node { flex_direction: FlexDirection::Row, column_gap: px(9), ..default() }).with_children(|r| {
            for i in 0..7 {
                r.spawn((Node { width: px(22), height: px(22), justify_content: JustifyContent::Center, align_items: AlignItems::Center, ..default() }, Visibility::Hidden, ObjPip(i))).with_children(|d| {
                    d.spawn((Node { position_type: PositionType::Absolute, width: px(15), height: px(15), border: UiRect::all(px(2)), ..default() }, BorderColor::all(AMBER), BackgroundColor(Color::NONE), UiTransform { rotation: Rot2::degrees(45.0), ..default() }));
                    d.spawn((txt("", 11.0, AMBER, &f.display), ObjPipText(i)));
                });
            }
        });
        p.spawn((txt("", 12.0, DIM, &f.display), T::Lives));
    });

    // arriba a la derecha: puntaje
    commands.spawn(Node { position_type: PositionType::Absolute, right: px(18), top: px(14), flex_direction: FlexDirection::Column, align_items: AlignItems::End, ..default() }).with_children(|p| {
        p.spawn((txt("0", 26.0, INK, &f.mono), T::Score));
        p.spawn((txt("", 18.0, AMBER, &f.display), T::Combo));
        p.spawn((txt("", 18.0, AMBER, &f.mono), T::Credits));
        p.spawn((txt("", 12.0, MINT, &f.display), T::CreditPop));
    });

    // cartel
    commands.spawn(Node { position_type: PositionType::Absolute, top: percent(22), width: percent(100), flex_direction: FlexDirection::Column, align_items: AlignItems::Center, row_gap: px(4), ..default() }).with_children(|p| {
        p.spawn((txt("", 46.0, INK, &f.display), T::BTitle));
        p.spawn((txt("", 13.0, AMBER, &f.display), T::BSub));
    });

    // abajo a la izquierda: vida y habilidades
    commands.spawn((Node { position_type: PositionType::Absolute, left: px(18), bottom: px(18), flex_direction: FlexDirection::Column, row_gap: px(8), padding: UiRect::all(px(10)), border_radius: panel_radius(), ..default() }, panel())).with_children(|p| {
        p.spawn(Node { flex_direction: FlexDirection::Row, align_items: AlignItems::Center, column_gap: px(10), ..default() }).with_children(|r| {
            r.spawn((txt("100", 34.0, INK, &f.mono), T::Hp));
            r.spawn(Node { flex_direction: FlexDirection::Row, column_gap: px(3), ..default() }).with_children(|bar| {
                for i in 0..HP_SEGS {
                    bar.spawn((Node { width: px(16), height: px(12), ..default() }, BackgroundColor(MINT), HpSeg(i)));
                }
            });
        });
        p.spawn(Node { flex_direction: FlexDirection::Row, column_gap: px(8), ..default() }).with_children(|r| {
            r.spawn((Node { height: px(32), padding: UiRect::axes(px(8), px(0)), flex_direction: FlexDirection::Column, justify_content: JustifyContent::Center, border: UiRect::all(px(1)), ..default() }, BorderColor::all(AMBER.with_alpha(0.6)))).with_children(|c| {
                c.spawn((txt("V", 13.0, AMBER, &f.mono), T::Gadget));
            });
            for (k, key, name) in [(0u8, "Q", "DASH"), (1, "E", "GANCHO"), (2, "C", "DESLIZAR")] {
                r.spawn((Node { width: px(62), height: px(32), flex_direction: FlexDirection::Column, justify_content: JustifyContent::Center, align_items: AlignItems::Center, border: UiRect::all(px(1)), overflow: Overflow::clip(), ..default() }, BorderColor::all(CYAN.with_alpha(0.6)))).with_children(|c| {
                    c.spawn((Node { position_type: PositionType::Absolute, left: px(0), bottom: px(0), width: percent(100), height: percent(100), ..default() }, BackgroundColor(CYAN.with_alpha(0.22)), ChipFill(k)));
                    c.spawn(txt(key, 13.0, CYAN, &f.mono));
                    c.spawn(txt(name, 9.0, DIM, &f.display));
                });
            }
        });
    });

    // abajo a la derecha: arma
    commands.spawn((Node { position_type: PositionType::Absolute, right: px(18), bottom: px(18), flex_direction: FlexDirection::Column, align_items: AlignItems::End, row_gap: px(4), padding: UiRect::all(px(10)), border_radius: panel_radius(), ..default() }, panel())).with_children(|p| {
        p.spawn((Node { flex_direction: FlexDirection::Row, column_gap: px(12), ..default() }, Visibility::Hidden, Slots)).with_children(|r| {
            for (i, w) in WEAPONS.iter().enumerate() {
                r.spawn((txt(&format!("{} {}", i + 1, w.short), 12.0, DIM, &f.display), T::Slot(i)));
            }
        });
        p.spawn(Node { flex_direction: FlexDirection::Row, align_items: AlignItems::Center, column_gap: px(6), ..default() }).with_children(|r| {
            r.spawn((txt("R", 13.0, DARK, &f.mono), BackgroundColor(AMBER), Node { padding: UiRect::axes(px(5), px(0)), margin: UiRect::right(px(6)), ..default() }, T::RKey));
            r.spawn((txt("24", 46.0, INK, &f.mono), T::Ammo));
            r.spawn((txt("/24", 18.0, DIM, &f.mono), T::Mag));
        });
        p.spawn((Node { width: px(150), height: px(3), ..default() }, BackgroundColor(Color::WHITE.with_alpha(0.08)))).with_children(|b| {
            b.spawn((Node { width: percent(0), height: percent(100), ..default() }, BackgroundColor(AMBER), ReloadFill));
        });
        p.spawn(Node { flex_direction: FlexDirection::Row, align_items: AlignItems::Center, column_gap: px(10), ..default() }).with_children(|r| {
            r.spawn((txt("", 13.0, INK, &f.display), T::WName));
            r.spawn(Node { flex_direction: FlexDirection::Row, column_gap: px(6), align_items: AlignItems::Center, ..default() }).with_children(|g| {
                for i in 0..4 {
                    g.spawn((Node { width: px(9), height: px(9), border: UiRect::all(px(2)), ..default() }, BorderColor::all(AMBER), BackgroundColor(Color::NONE), UiTransform { rotation: Rot2::degrees(45.0), ..default() }, NadePip(i)));
                }
            });
        });
    });

    // números de daño y FPS
    for i in 0..28 {
        commands.spawn((Node { position_type: PositionType::Absolute, ..default() }, txt("", 17.0, INK, &f.mono), Visibility::Hidden, NumText(i)));
    }
    commands.spawn((Node { position_type: PositionType::Absolute, left: px(8), bottom: px(4), ..default() }, txt("", 11.0, DIM, &f.mono), T::Fps));

    // pausa: controles y "clic para jugar"
    commands.spawn((centered, BackgroundColor(Color::srgba(0.02, 0.03, 0.035, 0.8)), GlobalZIndex(10), Pause)).with_children(|p| {
        p.spawn(Node { flex_direction: FlexDirection::Column, row_gap: px(18), width: px(780), ..default() }).with_children(|c| {
            c.spawn(Node { flex_direction: FlexDirection::Row, column_gap: px(14), align_items: AlignItems::Baseline, ..default() }).with_children(|t| {
                t.spawn(txt("ARENA", 76.0, INK, &f.display));
                t.spawn(txt("CERO", 76.0, AMBER, &f.display));
            });
            c.spawn((txt("", 15.0, DIM, &f.display), PauseMap));
            c.spawn((Node { padding: UiRect::axes(px(24), px(8)), align_self: AlignSelf::Start, border_radius: BorderRadius::new(px(0), px(14), px(0), px(0)), ..default() }, BackgroundColor(AMBER))).with_children(|b| {
                b.spawn(txt("CLIC PARA JUGAR", 20.0, DARK, &f.display));
            });
            c.spawn(Node { display: Display::Grid, grid_template_columns: vec![GridTrack::auto(), GridTrack::flex(1.0), GridTrack::auto(), GridTrack::flex(1.0)], column_gap: px(14), row_gap: px(7), ..default() }).with_children(|g| {
                for (k, d) in [
                    ("W A S D", "Moverse"),
                    ("SHIFT", "Correr"),
                    ("CLIC IZQ.", "Disparar"),
                    ("CLIC DER.", "Apuntar"),
                    ("E / CTRL", "Gancho · otra vez suelta"),
                    ("ESPACIO", "Saltar · con gancho, impulso"),
                    ("C", "Agacharse · corriendo, deslizar"),
                    ("Q", "Dash"),
                    ("1-4 / RUEDA", "Cambiar arma"),
                    ("G", "Granada"),
                    ("R", "Recargar"),
                    ("F", "Usar · derribo por la espalda"),
                    ("T / RUEDA CLIC", "Marcar enemigo"),
                    ("TAB", "Mapa táctico (ciudad)"),
                    ("B", "Tienda (en reabastecimiento)"),
                    ("V / Z", "Usar / cambiar gadget"),
                    ("L", "Linterna"),
                    ("M", "Música"),
                    ("F1 F2 F3", "Calidad baja · media · alta"),
                    ("F4", "Mostrar FPS"),
                    ("F5", "Brillo (bloom) sí/no"),
                    ("ESC", "Pausa"),
                ] {
                    g.spawn((Node { padding: UiRect::axes(px(7), px(2)), border: UiRect::all(px(1)), justify_self: JustifySelf::Start, ..default() }, BorderColor::all(INK.with_alpha(0.25)))).with_children(|kb| {
                        kb.spawn(txt(k, 12.0, INK, &f.mono));
                    });
                    g.spawn(txt(d, 13.0, DIM, &f.display));
                }
            });
        });
    });
}

fn tick_feed(time: Res<Time>, mut ui: ResMut<UiFeed>) {
    let dt = time.delta_secs();
    ui.banner_t -= dt;
    ui.next_banner();
    ui.mark_t -= dt;
    ui.popup_t -= dt;
    ui.credit_t -= dt;
    for n in &mut ui.nums {
        n.3 += dt;
    }
    ui.nums.retain(|n| n.3 < 0.8);
    if ui.nums.len() > 28 {
        let extra = ui.nums.len() - 28;
        ui.nums.drain(..extra);
    }
}

fn set(t: &mut Text, v: String) {
    if **t != v {
        **t = v;
    }
}

#[allow(clippy::type_complexity)]
fn vitals(lp: Res<LocalPlayer>, time: Res<Time>, mut texts: Query<(&mut Text, &mut TextColor, &T)>, mut segs: Query<(&HpSeg, &mut BackgroundColor), Without<ChipFill>>, mut chips: Query<(&ChipFill, &mut Node, &mut BackgroundColor), Without<HpSeg>>) {
    let s = &lp.state;
    let hr = (lp.hp / lp.max_hp).clamp(0.0, 1.0);
    let col = if hr > 0.5 { MINT } else if hr > 0.25 { AMBER } else { RED };
    let blink = if hr <= 0.25 { 0.6 + (time.elapsed_secs() * 8.0).sin() * 0.4 } else { 1.0 };
    for (mut t, mut c, k) in &mut texts {
        if *k == T::Hp {
            set(&mut t, format!("{}", lp.hp.ceil()));
            c.0 = if hr > 0.25 { INK } else { RED };
        }
    }
    for (seg, mut bg) in &mut segs {
        let full = (seg.0 as f32 + 0.5) / HP_SEGS as f32 <= hr;
        bg.0 = if full { col.with_alpha(blink) } else { Color::WHITE.with_alpha(0.08) };
    }
    for (c, mut n, mut bg) in &mut chips {
        let (k, active) = match c.0 {
            0 => ((1.0 - s.dash_cd.max(0.0) / (3.0 * s.dash_cd_mul)).clamp(0.0, 1.0), s.dash_t > 0.0),
            1 => ((1.0 - s.hook_cd.max(0.0) / 0.3).clamp(0.0, 1.0), s.hook.is_some()),
            _ => ((1.0 - s.slide_cd.max(0.0)).clamp(0.0, 1.0), s.slide > 0.0),
        };
        n.height = percent(k * 100.0);
        bg.0 = if active { CYAN.with_alpha(0.6) } else if k >= 1.0 { CYAN.with_alpha(0.22) } else { CYAN.with_alpha(0.1) };
    }
}

#[allow(clippy::type_complexity)]
fn weapon(
    game: Res<GameRes>,
    lp: Res<LocalPlayer>,
    time: Res<Time>,
    mut swap_seen: Local<(usize, f32)>,
    mut texts: Query<(&mut Text, &mut TextColor, &T, Option<&mut BackgroundColor>)>,
    mut fill: Query<&mut Node, With<ReloadFill>>,
    mut pips: Query<(&NadePip, &mut Node, &mut BackgroundColor), (Without<ReloadFill>, Without<T>)>,
    mut slots: Query<&mut Visibility, With<Slots>>,
) {
    let a = &lp.arsenal;
    let wd = &WEAPONS[a.w];
    let mag = a.mag_of(a.w);
    let ammo = a.ammo[a.w];
    let low = ammo <= mag.div_ceil(4);
    let now = time.elapsed_secs();
    if swap_seen.0 != a.w {
        *swap_seen = (a.w, now);
    }
    let blink = (now * 6.0).sin() > 0.0;
    let car = game.0.driving.and_then(|i| game.0.vehicles.get(i));
    if let Some(v) = car {
        for (mut t, mut c, k, bg) in &mut texts {
            match *k {
                T::Ammo => {
                    set(&mut t, format!("{}", (v.speed.abs() * 3.6).round()));
                    c.0 = INK;
                }
                T::Mag => set(&mut t, "KM/H".into()),
                T::WName => set(&mut t, format!("AUTO · {}%", (v.hp / 60.0 * 100.0).clamp(0.0, 100.0).round())),
                T::RKey => {
                    c.0 = DARK.with_alpha(0.0);
                    if let Some(mut b) = bg {
                        b.0 = AMBER.with_alpha(0.0);
                    }
                }
                _ => {}
            }
        }
        return;
    }
    for (mut t, mut c, k, bg) in &mut texts {
        match *k {
            T::Ammo => {
                set(&mut t, format!("{ammo}"));
                c.0 = if ammo == 0 { RED } else if low { AMBER } else { INK };
            }
            T::Mag => set(&mut t, format!("/{mag}")),
            T::WName => set(&mut t, wd.name.to_uppercase()),
            T::RKey => {
                let show = a.reload <= 0.0 && low && blink;
                c.0 = DARK.with_alpha(if show { 1.0 } else { 0.0 });
                if let Some(mut b) = bg {
                    b.0 = AMBER.with_alpha(if show { 1.0 } else { 0.0 });
                }
            }
            T::Slot(i) => c.0 = if i == a.w { AMBER } else if !a.unlocked[i] { DIM.with_alpha(0.3) } else { INK.with_alpha(0.7) },
            _ => {}
        }
    }
    let k = if a.reload > 0.0 { 1.0 - a.reload / a.reload_max } else { 0.0 };
    for mut n in &mut fill {
        n.width = percent(k * 100.0);
    }
    for (p, mut n, mut bg) in &mut pips {
        n.display = if p.0 < a.max_nades { Display::Flex } else { Display::None };
        bg.0 = if p.0 < a.nades { AMBER } else { Color::NONE };
    }
    // las ranuras aparecen un momento al cambiar de arma
    let show = now - swap_seen.1 < 1.6 && now > 2.0;
    for mut v in &mut slots {
        *v = if show { Visibility::Inherited } else { Visibility::Hidden };
    }
}

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
fn top(
    game: Res<GameRes>,
    level: Res<LevelRes>,
    ui: Res<UiFeed>,
    mut texts: Query<(&mut Text, &mut TextColor, &T), Without<ObjPipText>>,
    mut boss: Query<&mut Visibility, (With<BossRoot>, Without<AlertPill>, Without<ObjPip>, Without<PopupRoot>)>,
    mut boss_fill: Query<&mut Node, With<BossFill>>,
    mut pill: Query<(&mut Visibility, &mut BorderColor), (With<AlertPill>, Without<BossRoot>, Without<ObjPip>, Without<PopupRoot>)>,
    mut pips: Query<(&ObjPip, &mut Visibility, &Children), (Without<BossRoot>, Without<AlertPill>, Without<PopupRoot>)>,
    mut pip_border: Query<(&mut BorderColor, &mut BackgroundColor), (Without<AlertPill>, Without<ObjPip>)>,
    mut pip_text: Query<(&ObjPipText, &mut Text, &mut TextColor), Without<T>>,
    mut popup: Query<&mut Visibility, (With<PopupRoot>, Without<BossRoot>, Without<AlertPill>, Without<ObjPip>)>,
) {
    let g = &game.0;
    let city = g.mode == Mode::City;
    let (alert_txt, alert_col) = [("OCULTO", MINT), ("BUSCANDO", AMBER), ("DETECTADO", RED)][g.city_alert() as usize];
    let banner_a = if ui.banner_t > 0.0 { 1.0 } else { (1.0 + ui.banner_t / 0.4).max(0.0) };
    let popup_a = (ui.popup_t / 0.4).clamp(0.0, 1.0);
    for (mut t, mut c, k) in &mut texts {
        match *k {
            T::Wave => set(&mut t, match (&g.campaign, city) {
                (Some(cp), _) => format!("OP {}", cp.op + 1),
                (None, true) => String::new(),
                (None, false) => format!("OLEADA {:02}", g.wave),
            }),
            T::WaveSub if g.campaign.is_some() => {
                let cp = g.campaign.as_ref().unwrap();
                set(&mut t, format!("{} · {}:{:02}", cp.def.name.to_uppercase(), (cp.t / 60.0) as u32, (cp.t % 60.0) as u32));
            }
            T::Lives if g.campaign.is_some() => {
                let d = g.campaign.as_ref().map_or(0, |c| c.deaths);
                set(&mut t, if d == 0 { "SIN CAÍDAS".into() } else { format!("CAÍDAS {d}") });
            }
            T::WaveSub => set(&mut t, match &g.city {
                Some(run) => format!("{}:{:02}", (run.time / 60.0) as u32, (run.time % 60.0) as u32),
                None => {
                    let left = g.enemies.len() + g.spawn_queue.len();
                    if g.wave > 0 { format!("{left} HOSTIL{}", if left == 1 { "" } else { "ES" }) } else { "PREPARATE".into() }
                }
            }),
            T::Lives => set(&mut t, match &g.city {
                Some(run) => format!("INTENTOS {}{}", "◆".repeat(run.lives as usize), "◇".repeat(3usize.saturating_sub(run.lives as usize))),
                None => String::new(),
            }),
            T::Score => set(&mut t, fmt_thousands(g.score)),
            T::Combo => set(&mut t, match (g.combo > 1, city) {
                (true, true) => format!("RACHA ×{}", g.combo),
                (true, false) => format!("COMBO ×{}", g.combo),
                _ => String::new(),
            }),
            T::BTitle => {
                c.0 = INK.with_alpha(banner_a);
                set(&mut t, ui.banner.as_ref().map(|b| b.0.clone()).unwrap_or_default());
            }
            T::BSub => {
                c.0 = AMBER.with_alpha(banner_a);
                set(&mut t, ui.banner.as_ref().map(|b| b.1.to_uppercase()).unwrap_or_default());
            }
            T::Alert => {
                c.0 = alert_col;
                set(&mut t, alert_txt.into());
            }
            T::Credits => set(&mut t, if city { format!("¢ {}", fmt_thousands(g.credits)) } else { String::new() }),
            T::CreditPop => {
                let a = (ui.credit_t / 0.4).clamp(0.0, 1.0);
                c.0 = MINT.with_alpha(a);
                set(&mut t, match (ui.credit, ui.credit_t > 0.0) {
                    (Some((n, l)), true) => format!("+{n} ¢ · {l}"),
                    _ => String::new(),
                });
            }
            T::Clock => set(&mut t, if city {
                let h = g.director.hour;
                format!("{:02}:{:02}", h as u32, ((h.fract()) * 60.0) as u32 / 10 * 10)
            } else {
                String::new()
            }),
            T::Gadget => {
                let gi = g.gadget_sel % 3;
                let gd = arena_sim::economy::GADGETS[gi];
                set(&mut t, if city { format!("V {} ×{}", gd.name().to_uppercase(), g.gadgets[gi]) } else { String::new() });
                c.0 = if g.gadgets[gi] > 0 { AMBER } else { DIM };
            }
            T::Popup => {
                c.0 = AMBER.with_alpha(popup_a);
                set(&mut t, ui.popup.as_ref().map(|p| format!("+{}", p.0)).unwrap_or_default());
            }
            T::PopupLabel => {
                c.0 = INK.with_alpha(popup_a * 0.9);
                set(&mut t, ui.popup.as_ref().map(|p| p.1.to_string()).unwrap_or_default());
            }
            _ => {}
        }
    }
    for mut v in &mut popup {
        *v = if ui.popup_t > 0.0 { Visibility::Inherited } else { Visibility::Hidden };
    }
    let boss_e = g.boss.and_then(|id| g.enemies.iter().find(|e| e.id == id));
    for mut v in &mut boss {
        *v = if boss_e.is_some() { Visibility::Inherited } else { Visibility::Hidden };
    }
    if let Some(b) = boss_e {
        for mut n in &mut boss_fill {
            n.width = percent((b.hp / b.max_hp).max(0.0) * 100.0);
        }
    }
    for (mut v, mut b) in &mut pill {
        *v = if city { Visibility::Inherited } else { Visibility::Hidden };
        *b = BorderColor::all(alert_col);
    }
    // rombos de objetivos: ámbar pendiente, rojo en alerta, verde hecho
    let ops = level.0.city.as_ref().map(|c| &c.outposts);
    for (p, mut v, children) in &mut pips {
        let Some(op) = ops.and_then(|o| o.get(p.0)).filter(|_| g.campaign.is_none()) else {
            *v = Visibility::Hidden;
            continue;
        };
        *v = Visibility::Inherited;
        let st = g.city.as_ref().map(|r| &r.ops[p.0]);
        let done = st.is_some_and(|s| s.done);
        let hot = st.is_some_and(|s| s.alarm > 0.0);
        let c = if done { MINT } else if hot { RED } else { AMBER };
        if let Some(&d) = children.first() {
            if let Ok((mut bc, mut bg)) = pip_border.get_mut(d) {
                *bc = BorderColor::all(c);
                bg.0 = if done { c } else { Color::NONE };
            }
        }
        for (pt, mut t, mut tc) in &mut pip_text {
            if pt.0 == p.0 {
                set(&mut t, op.icon.to_string());
                tc.0 = if done { DARK } else { c };
            }
        }
    }
}

/// Brújula: rumbo de la cámara, puntos cardinales, objetivos pendientes y reabastecimiento.
#[allow(clippy::type_complexity)]
fn compass(
    lp: Res<LocalPlayer>,
    look: Res<crate::player::Look>,
    game: Res<GameRes>,
    level: Res<LevelRes>,
    mut ticks: Query<(&CompassTick, &mut Node, &mut Visibility), Without<CompassMark>>,
    mut marks: Query<(&CompassMark, &mut Node, &mut Visibility, &Children), Without<CompassTick>>,
    mut mark_bg: Query<&mut BackgroundColor>,
    mut mark_txt: Query<(&CompassMarkText, &mut Text)>,
) {
    let heading = -look.yaw;
    let place = |bearing: f32| -> Option<f32> {
        let mut rel = (bearing - heading).rem_euclid(TAU);
        if rel > PI {
            rel -= TAU;
        }
        let half = COMPASS_FOV.to_radians() / 2.0;
        (rel.abs() <= half).then(|| COMPASS_W / 2.0 + rel / half * COMPASS_W / 2.0)
    };
    for (t, mut n, mut v) in &mut ticks {
        match place((t.0 as f32 * 15.0).to_radians()) {
            Some(x) => {
                n.left = px(x);
                *v = Visibility::Inherited;
            }
            None => *v = Visibility::Hidden,
        }
    }
    let mut items: Vec<(Vec3, char, Color)> = Vec::new();
    if let (Some(c), Some(run)) = (level.0.city.as_ref(), game.0.city.as_ref()) {
        for (i, op) in c.outposts.iter().enumerate() {
            let st = &run.ops[i];
            if st.done || op.camp || st.asleep || game.0.campaign.is_some() {
                continue;
            }
            let p = op.obj_pos.unwrap_or(Vec3::new(op.cx, 0.0, op.cz));
            items.push((p, op.icon, if st.alarm > 0.0 { RED } else { AMBER }));
        }
        if let Some(cp) = &game.0.campaign {
            for p in &cp.marks {
                items.push((*p, '!', GOLD));
            }
        }
        for m in &game.0.director.missions {
            items.push((m.pos, '!', Color::srgb(1.0, 0.42, 0.18)));
        }
        for st in &c.stations {
            if st.entrance.distance(lp.state.pos) < 160.0 {
                items.push((st.entrance, 'M', CYAN));
            }
        }
        items.push((c.safe_pos, '+', MINT));
        for (i, op) in c.outposts.iter().enumerate() {
            if run.ops[i].cleared {
                items.push((Vec3::new(op.cx, 0.0, op.cz), '+', MINT));
            }
        }
    }
    let me = lp.state.pos;
    for (m, mut n, mut v, children) in &mut marks {
        let Some(&(p, icon, col)) = items.get(m.0) else {
            *v = Visibility::Hidden;
            continue;
        };
        // rumbo: 0 al norte (−Z), creciendo hacia el este (+X)
        let bearing = (p.x - me.x).atan2(-(p.z - me.z));
        match place(bearing) {
            Some(x) => {
                n.left = px(x);
                *v = Visibility::Inherited;
                if let Some(&d) = children.first() {
                    if let Ok(mut bg) = mark_bg.get_mut(d) {
                        bg.0 = col;
                    }
                }
                for (mt, mut t) in &mut mark_txt {
                    if mt.0 == m.0 {
                        set(&mut t, icon.to_string());
                    }
                }
            }
            None => *v = Visibility::Hidden,
        }
    }
}

#[allow(clippy::type_complexity)]
fn crosshair(
    lp: Res<LocalPlayer>,
    ui: Res<UiFeed>,
    mut bars: Query<(&mut Node, &mut Visibility, &CrossBar), Without<HitRoot>>,
    mut dot: Query<&mut Visibility, (With<CrossDot>, Without<CrossBar>, Without<HitRoot>)>,
    mut hit: Query<(&mut Visibility, &mut UiTransform), (With<HitRoot>, Without<CrossBar>, Without<CrossDot>)>,
    mut hit_bars: Query<&mut BackgroundColor, With<HitBar>>,
) {
    let s = &lp.state;
    let a = &lp.arsenal;
    let wd = &WEAPONS[a.w];
    let moving = s.horizontal_speed() > 1.0 || !s.on_ground;
    let gap = ((if moving { wd.spread[1] } else { wd.spread[0] }) * 600.0 + a.recoil * 6.0) * (1.0 - s.crouch_t * 0.4);
    let aim_on = s.aim_t > 0.5;
    for (mut n, mut v, b) in &mut bars {
        let (l, t) = match b.0 {
            0 => (-1.0, -15.0 - gap),
            1 => (-1.0, 6.0 + gap),
            2 => (-15.0 - gap, -1.0),
            _ => (6.0 + gap, -1.0),
        };
        n.left = px(l);
        n.top = px(t);
        *v = if aim_on { Visibility::Hidden } else { Visibility::Inherited };
    }
    for mut v in &mut dot {
        *v = if aim_on { Visibility::Hidden } else { Visibility::Inherited };
    }
    let show = ui.mark_t > 0.0;
    let kill = ui.mark == Some(Mark::Kill);
    for (mut v, mut tr) in &mut hit {
        *v = if show { Visibility::Inherited } else { Visibility::Hidden };
        tr.scale = Vec2::splat(if kill { 1.0 + (0.25 - ui.mark_t).max(0.0) * 2.0 } else { 1.0 });
    }
    let mc = match ui.mark {
        Some(Mark::Kill) => RED,
        Some(Mark::Crit) => AMBER,
        _ => INK,
    };
    for mut bg in &mut hit_bars {
        bg.0 = mc;
    }
}

#[allow(clippy::type_complexity)]
fn overlays(lp: Res<LocalPlayer>, time: Res<Time>, mut overlays: Query<(&mut Visibility, &Overlay, Option<&mut ImageNode>), Without<DmgArc>>, mut arc: Query<(&mut Visibility, &mut ImageNode, &mut UiTransform), With<DmgArc>>) {
    let s = &lp.state;
    let rail = if lp.arsenal.w == 2 { ((s.aim_t - 0.6) / 0.3).clamp(0.0, 1.0) } else { 0.0 };
    let low_hp = if lp.hp < 30.0 { 0.35 + (time.elapsed_secs() / 0.18).sin() * 0.1 } else { 0.0 };
    for (mut v, o, img) in &mut overlays {
        let alpha = match o {
            Overlay::Hurt => lp.hurt_flash.max(low_hp),
            // la web oscurecía los bordes al apuntar; se veía como sombras negras alrededor del arma
            Overlay::Aim => 0.0,
            Overlay::Scope => rail,
        };
        *v = if alpha > 0.01 { Visibility::Inherited } else { Visibility::Hidden };
        if let Some(mut img) = img {
            img.color = Color::WHITE.with_alpha(alpha);
        }
    }
    for (mut v, mut img, mut tr) in &mut arc {
        *v = if lp.dmg_t > 0.01 { Visibility::Inherited } else { Visibility::Hidden };
        img.color = Color::WHITE.with_alpha(lp.dmg_t);
        tr.rotation = Rot2::radians(lp.dmg_dir);
    }
}

fn fmt_thousands(n: u32) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, ch) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push('.');
        }
        out.push(ch);
    }
    out
}

/// Números de daño: suben y se desvanecen sobre el enemigo.
fn numbers(ui: Res<UiFeed>, cam: Query<(&Camera, &GlobalTransform), With<MainCamera>>, win: Single<&Window>, target: Option<Res<crate::render_scale::SceneTarget>>, mut q: Query<(&mut Node, &mut Text, &mut TextColor, &mut TextFont, &mut Visibility, &NumText)>) {
    let Ok((camera, ct)) = cam.single() else { return };
    // la cámara dibuja en la textura de escena: pasar de sus píxeles a los de la ventana
    let k = target.map_or(1.0, |t| win.width() / t.size.x.max(1) as f32);
    for (mut n, mut t, mut c, mut f, mut v, nt) in &mut q {
        let Some(&(pos, value, crit, age)) = ui.nums.get(nt.0) else {
            *v = Visibility::Hidden;
            continue;
        };
        let Ok(sp) = camera.world_to_viewport(ct, pos) else {
            *v = Visibility::Hidden;
            continue;
        };
        let sp = sp * k;
        *v = Visibility::Inherited;
        set(&mut t, format!("{}", (value * 10.0).round().max(1.0)));
        n.left = px(sp.x - 12.0);
        n.top = px(sp.y - age * 46.0 - 10.0);
        c.0 = if crit { AMBER } else { INK }.with_alpha(1.0 - age / 0.8);
        f.font_size = (if crit { 24.0 } else { 17.0 }).into();
    }
}

fn pause_screen(cursor: Single<&CursorOptions>, level: Res<LevelRes>, menu: Res<crate::campaign_ui::MenuOpen>, mut q: Query<&mut Visibility, With<Pause>>, mut map: Query<&mut Text, With<PauseMap>>) {
    let paused = cursor.grab_mode == CursorGrabMode::None && !menu.0;
    for mut v in &mut q {
        *v = if paused { Visibility::Inherited } else { Visibility::Hidden };
    }
    let i = &level.0.info;
    for mut t in &mut map {
        set(&mut t, format!("{} · {} — {}   ·   ENTER: MENÚ PRINCIPAL", i.name.to_uppercase(), i.size, i.desc));
    }
}

fn fps(key: Res<ButtonInput<KeyCode>>, mut show: ResMut<ShowFps>, diag: Res<DiagnosticsStore>, mut q: Query<(&mut Text, &T)>) {
    if key.just_pressed(KeyCode::F4) {
        show.0 = !show.0;
        crate::config::set("fps", if show.0 { "1" } else { "0" });
    }
    let v = if show.0 { diag.get(&FrameTimeDiagnosticsPlugin::FPS).and_then(|d| d.smoothed()).map(|f| format!("{f:.0} FPS")).unwrap_or_default() } else { String::new() };
    for (mut t, k) in &mut q {
        if *k == T::Fps {
            set(&mut t, v.clone());
        }
    }
}

fn hook_ring(hk: Res<crate::hook_view::Hookable>, mut q: Query<(&mut Visibility, &mut BorderColor, &mut Node), With<HookRing>>) {
    for (mut v, mut b, mut n) in &mut q {
        *v = if hk.can || hk.hooked { Visibility::Inherited } else { Visibility::Hidden };
        *b = BorderColor::all(CYAN.with_alpha(if hk.hooked { 1.0 } else { 0.6 }));
        n.border = UiRect::all(px(if hk.hooked { 2.0 } else { 1.0 }));
    }
}
