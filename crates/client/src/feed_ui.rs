//! Lo arcade de la interfaz (nuevo): anunciador de rachas en el centro ("DOBLE BAJA", "TRIPLE
//! BAJA"...) que entra grande y se achica, y lista de bajas y puntos a la derecha, debajo del puntaje.

use crate::game::UiFeed;
use crate::hud::Fonts;
use bevy::prelude::*;

const INK: Color = Color::srgb(0.902, 0.929, 0.910);
const AMBER: Color = Color::srgb(1.0, 0.690, 0.180);
const GOLD: Color = Color::srgb(1.0, 0.84, 0.3);
const RED: Color = Color::srgb(1.0, 0.239, 0.353);
const FEED: usize = 5;

pub struct FeedUiPlugin;

impl Plugin for FeedUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostStartup, spawn).add_systems(Update, update);
    }
}

#[derive(Component)]
struct StreakText;
#[derive(Component)]
struct FeedLine(usize);

fn spawn(mut commands: Commands, f: Res<Fonts>) {
    let t = |s: &str, size: f32, c: Color, font: &Handle<Font>| (Text::new(s), TextFont { font: FontSource::Handle(font.clone()), font_size: size.into(), ..default() }, TextColor(c), TextShadow::default());
    commands.spawn((Node { position_type: PositionType::Absolute, top: percent(36), width: percent(100), justify_content: JustifyContent::Center, ..default() }, UiTransform::default())).with_children(|p| {
        p.spawn((t("", 40.0, GOLD, &f.display), StreakText, UiTransform::default()));
    });
    commands.spawn(Node { position_type: PositionType::Absolute, right: px(18), top: px(118), flex_direction: FlexDirection::Column, align_items: AlignItems::End, row_gap: px(2), ..default() }).with_children(|p| {
        for i in 0..FEED {
            p.spawn((t("", 13.0, INK, &f.display), FeedLine(i)));
        }
    });
}

fn streak_name(n: u32) -> &'static str {
    match n {
        2 => "DOBLE BAJA",
        3 => "TRIPLE BAJA",
        4 => "CUÁDRUPLE",
        5 => "RACHA ×5",
        6 => "IMPARABLE",
        7 => "DEVASTADOR",
        _ => "LEYENDA",
    }
}

fn update(time: Res<Time>, mut ui: ResMut<UiFeed>, mut streak: Query<(&mut Text, &mut TextColor, &mut UiTransform), (With<StreakText>, Without<FeedLine>)>, mut lines: Query<(&FeedLine, &mut Text, &mut TextColor), Without<StreakText>>) {
    let dt = time.delta_secs();
    if let Some(s) = ui.streak.as_mut() {
        s.1 += dt;
    }
    if ui.streak.is_some_and(|s| s.1 > 1.6) {
        ui.streak = None;
    }
    for (mut t, mut c, mut tr) in &mut streak {
        match ui.streak {
            Some((n, age)) => {
                let s = streak_name(n);
                if t.0 != s {
                    t.0 = s.into();
                }
                // entra grande, se asienta y al final se desvanece
                let k = 1.0 + (1.0 - (age / 0.18).min(1.0)).powi(2) * 0.8;
                tr.scale = Vec2::splat(k);
                let a = if age > 1.2 { 1.0 - (age - 1.2) / 0.4 } else { 1.0 };
                c.0 = if n >= 4 { RED } else { GOLD }.with_alpha(a.max(0.0));
            }
            None => {
                if !t.0.is_empty() {
                    t.0.clear();
                }
            }
        }
    }
    for f in &mut ui.feed {
        f.1 += dt;
    }
    ui.feed.retain(|f| f.1 < 4.0);
    // la más nueva arriba
    let n = ui.feed.len();
    for (l, mut t, mut c) in &mut lines {
        match n.checked_sub(l.0 + 1).and_then(|i| ui.feed.get(i)) {
            Some((s, age)) => {
                if t.0 != *s {
                    t.0 = s.clone();
                }
                let a = if *age > 3.2 { 1.0 - (age - 3.2) / 0.8 } else { 1.0 };
                c.0 = if l.0 == 0 { AMBER } else { INK }.with_alpha(a.max(0.0) * (1.0 - l.0 as f32 * 0.12));
            }
            None => {
                if !t.0.is_empty() {
                    t.0.clear();
                }
            }
        }
    }
}
