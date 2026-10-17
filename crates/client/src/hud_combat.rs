//! HUD de combate (resto de `19-hud...js`): oleada u objetivos, puntaje con combo, cartel
//! central, marca de impacto, números de daño, barra del Coloso, alerta e intentos.

use crate::game::{GameRes, UiFeed};
use crate::hud::Fonts;
use crate::player::MainCamera;
use crate::world_render::LevelRes;
use arena_sim::game::{Mark, Mode};
use bevy::prelude::*;

const INK: Color = Color::srgb(0.902, 0.929, 0.910);
const DIM: Color = Color::srgb(0.498, 0.549, 0.529);
const AMBER: Color = Color::srgb(1.0, 0.690, 0.180);
const RED: Color = Color::srgb(1.0, 0.239, 0.353);
const MINT: Color = Color::srgb(0.365, 1.0, 0.690);

pub struct HudCombatPlugin;

impl Plugin for HudCombatPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostStartup, spawn).add_systems(Update, (update, update_nums, tick_feed));
    }
}

#[derive(Component, Clone, Copy, PartialEq, Eq)]
enum T {
    WLabel,
    Wave,
    Left,
    Lvl,
    Lives,
    Score,
    Kills,
    Combo,
    BTitle,
    BSub,
    Alert,
}

#[derive(Component)]
struct HitBar;
#[derive(Component)]
struct HitRoot;
#[derive(Component)]
struct BossRoot;
#[derive(Component)]
struct BossFill;
#[derive(Component)]
struct AlertRoot;
#[derive(Component)]
struct AlertPill;
#[derive(Component)]
struct NumText(usize);

fn label(text: &str, size: f32, color: Color, font: &Handle<Font>) -> impl Bundle {
    (Text::new(text), TextFont { font: FontSource::Handle(font.clone()), font_size: size.into(), ..default() }, TextColor(color))
}

fn spawn(mut commands: Commands, fonts: Res<Fonts>) {
    let f = &*fonts;
    // arriba a la izquierda: oleada u objetivos
    commands.spawn(Node { position_type: PositionType::Absolute, left: px(16), top: px(14), flex_direction: FlexDirection::Column, ..default() }).with_children(|p| {
        p.spawn((label("OLEADA", 12.0, DIM, &f.display), T::WLabel));
        p.spawn((label("00", 40.0, AMBER, &f.mono), T::Wave));
        p.spawn((label("", 14.0, DIM, &f.display), T::Left));
        p.spawn((label("", 12.0, DIM, &f.display), T::Lvl));
        p.spawn((label("", 12.0, DIM, &f.display), T::Lives));
    });
    // arriba a la derecha: puntaje
    commands.spawn(Node { position_type: PositionType::Absolute, right: px(16), top: px(14), flex_direction: FlexDirection::Column, align_items: AlignItems::End, ..default() }).with_children(|p| {
        p.spawn(label("PUNTAJE", 12.0, DIM, &f.display));
        p.spawn((label("0", 34.0, INK, &f.mono), T::Score));
        p.spawn((label("0 bajas", 12.0, DIM, &f.display), T::Kills));
        p.spawn((label("", 22.0, AMBER, &f.display), T::Combo));
    });
    // cartel central
    commands.spawn(Node { position_type: PositionType::Absolute, top: percent(28), width: percent(100), flex_direction: FlexDirection::Column, align_items: AlignItems::Center, ..default() }).with_children(|p| {
        p.spawn((label("", 72.0, INK, &f.display), T::BTitle));
        p.spawn((label("", 14.0, AMBER, &f.display), T::BSub));
    });
    // alerta (ciudad)
    commands.spawn((Node { position_type: PositionType::Absolute, top: px(150), width: percent(100), justify_content: JustifyContent::Center, ..default() }, AlertRoot, Visibility::Hidden)).with_children(|p| {
        p.spawn((Node { padding: UiRect::axes(px(10), px(3)), border: UiRect::all(px(1)), ..default() }, BorderColor::all(MINT), AlertPill)).with_children(|b| {
            b.spawn((label("OCULTO", 12.0, MINT, &f.display), T::Alert));
        });
    });
    // barra del Coloso
    commands.spawn((Node { position_type: PositionType::Absolute, top: px(156), width: percent(100), flex_direction: FlexDirection::Column, align_items: AlignItems::Center, ..default() }, BossRoot, Visibility::Hidden)).with_children(|p| {
        p.spawn(label("COLOSO", 12.0, RED, &f.display));
        p.spawn((Node { width: px(420), height: px(8), margin: UiRect::top(px(4)), ..default() }, BackgroundColor(Color::WHITE.with_alpha(0.08)))).with_children(|b| {
            b.spawn((Node { width: percent(100), height: percent(100), ..default() }, BackgroundColor(RED), BossFill));
        });
    });
    // marca de impacto: una X en la mira
    commands.spawn((Node { position_type: PositionType::Absolute, left: percent(50), top: percent(50), ..default() }, HitRoot, Visibility::Hidden)).with_children(|p| {
        for a in [45.0f32, -45.0] {
            p.spawn((
                Node { position_type: PositionType::Absolute, left: px(-1), top: px(-14), width: px(2), height: px(28), ..default() },
                BackgroundColor(INK),
                UiTransform { rotation: Rot2::degrees(a), ..default() },
                HitBar,
            ));
        }
    });
    // números de daño
    for i in 0..28 {
        commands.spawn((Node { position_type: PositionType::Absolute, ..default() }, label("", 17.0, INK, &f.mono), Visibility::Hidden, NumText(i)));
    }
}

fn tick_feed(time: Res<Time>, mut ui: ResMut<UiFeed>) {
    let dt = time.delta_secs();
    ui.banner_t -= dt;
    ui.mark_t -= dt;
    for n in &mut ui.nums {
        n.3 += dt;
    }
    ui.nums.retain(|n| n.3 < 0.8);
    if ui.nums.len() > 28 {
        let extra = ui.nums.len() - 28;
        ui.nums.drain(..extra);
    }
}

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
fn update(
    game: Res<GameRes>,
    level: Res<LevelRes>,
    ui: Res<UiFeed>,
    mut texts: Query<(&mut Text, &mut TextColor, &T)>,
    mut boss: Query<&mut Visibility, (With<BossRoot>, Without<AlertRoot>, Without<HitRoot>)>,
    mut boss_fill: Query<&mut Node, With<BossFill>>,
    mut alert: Query<&mut Visibility, (With<AlertRoot>, Without<BossRoot>, Without<HitRoot>)>,
    mut alert_border: Query<&mut BorderColor, With<AlertPill>>,
    mut hit: Query<&mut Visibility, (With<HitRoot>, Without<BossRoot>, Without<AlertRoot>)>,
    mut hit_bars: Query<&mut BackgroundColor, With<HitBar>>,
) {
    let g = &game.0;
    let city = g.mode == Mode::City;
    let al = g.city_alert();
    let (alert_txt, alert_col) = [("OCULTO", MINT), ("BUSCANDO", AMBER), ("DETECTADO", RED)][al as usize];
    let banner_a = if ui.banner_t > 0.0 { 1.0 } else { (1.0 + ui.banner_t / 0.4).max(0.0) };
    for (mut t, mut c, k) in &mut texts {
        let v: String = match *k {
            T::WLabel => (if city { "OBJETIVOS" } else { "OLEADA" }).into(),
            T::Wave => match &g.city {
                Some(run) => format!("{}/{}", run.done.len(), level.0.city.as_ref().map_or(7, |c| c.outposts.len())),
                None => format!("{:02}", g.wave),
            },
            T::Left => match &g.city {
                Some(run) => format!("{} · {}:{:02}", level.0.info.name, (run.time / 60.0) as u32, (run.time % 60.0) as u32),
                None => {
                    let left = g.enemies.len() + g.spawn_queue.len();
                    if g.wave > 0 { format!("{left} hostil{}", if left == 1 { "" } else { "es" }) } else { "Preparate".into() }
                }
            },
            T::Lvl => match &g.city {
                Some(run) => [(!run.antenna).then_some("sin refuerzos"), (!run.powered).then_some("torretas apagadas"), run.intel.then_some("enemigos marcados")].into_iter().flatten().collect::<Vec<_>>().join(" · "),
                None => level.0.info.name.to_string(),
            },
            T::Lives => match &g.city {
                Some(run) => format!("INTENTOS {}{}", "◆".repeat(run.lives as usize), "◇".repeat(3usize.saturating_sub(run.lives as usize))),
                None => String::new(),
            },
            T::Score => fmt_thousands(g.score),
            T::Kills => format!("{} baja{}", g.kills, if g.kills == 1 { "" } else { "s" }),
            T::Combo => if g.combo > 1 && !city { format!("COMBO ×{}", g.combo) } else { String::new() },
            T::BTitle => {
                c.0 = INK.with_alpha(banner_a);
                ui.banner.as_ref().map(|b| b.0.clone()).unwrap_or_default()
            }
            T::BSub => {
                c.0 = AMBER.with_alpha(banner_a);
                ui.banner.as_ref().map(|b| b.1.to_uppercase()).unwrap_or_default()
            }
            T::Alert => {
                c.0 = alert_col;
                alert_txt.into()
            }
        };
        if **t != v {
            **t = v;
        }
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
    for mut v in &mut alert {
        *v = if city { Visibility::Inherited } else { Visibility::Hidden };
    }
    for mut b in &mut alert_border {
        *b = BorderColor::all(alert_col);
    }
    let show = ui.mark_t > 0.0;
    for mut v in &mut hit {
        *v = if show { Visibility::Inherited } else { Visibility::Hidden };
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
fn update_nums(ui: Res<UiFeed>, cam: Query<(&Camera, &GlobalTransform), With<MainCamera>>, mut q: Query<(&mut Node, &mut Text, &mut TextColor, &mut TextFont, &mut Visibility, &NumText)>) {
    let Ok((camera, ct)) = cam.single() else { return };
    for (mut n, mut t, mut c, mut f, mut v, nt) in &mut q {
        let Some(&(pos, value, crit, age)) = ui.nums.get(nt.0) else {
            *v = Visibility::Hidden;
            continue;
        };
        let Ok(sp) = camera.world_to_viewport(ct, pos) else {
            *v = Visibility::Hidden;
            continue;
        };
        *v = Visibility::Inherited;
        let txt = format!("{}", (value * 10.0).round().max(1.0));
        if **t != txt {
            **t = txt;
        }
        n.left = px(sp.x - 12.0);
        n.top = px(sp.y - age * 46.0 - 10.0);
        c.0 = if crit { AMBER } else { INK }.with_alpha(1.0 - age / 0.8);
        f.font_size = (if crit { 24.0 } else { 17.0 }).into();
    }
}
