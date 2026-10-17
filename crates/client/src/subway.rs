//! Subte (nuevo): en el andén, F abre la lista de estaciones; 1-3 viaja (la partida se pausa
//! mientras elegís). Bajo tierra los enemigos no te ven: también sirve para escapar.

use crate::game::{GameRes, UiFeed};
use crate::hud::Fonts;
use crate::player::LocalPlayer;
use crate::world_render::LevelRes;
use arena_sim::player::EYE;
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions};

const INK: Color = Color::srgb(0.902, 0.929, 0.910);
const DIM: Color = Color::srgb(0.498, 0.549, 0.529);
const CYAN: Color = Color::srgb(0.373, 0.847, 1.0);

pub struct SubwayPlugin;

impl Plugin for SubwayPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SubwayOpen>().add_systems(PostStartup, spawn).add_systems(Update, (input, show));
    }
}

#[derive(Resource, Default)]
pub struct SubwayOpen(pub bool);

#[derive(Component)]
struct Root;
#[derive(Component)]
struct Row(usize);

/// Estación en cuyo andén está el jugador.
pub fn platform_near(level: &arena_sim::Level, pos: Vec3) -> Option<usize> {
    level.city.as_ref()?.stations.iter().position(|s| (s.platform.x - pos.x).hypot(s.platform.z - pos.z) < 5.0 && (s.platform.y + EYE - pos.y).abs() < 1.5)
}

fn spawn(mut commands: Commands, f: Res<Fonts>, level: Res<LevelRes>) {
    let t = |s: &str, size: f32, c: Color, font: &Handle<Font>| (Text::new(s), TextFont { font: FontSource::Handle(font.clone()), font_size: size.into(), ..default() }, TextColor(c));
    let names: Vec<&str> = level.0.city.as_ref().map(|c| c.stations.iter().map(|s| s.name).collect()).unwrap_or_default();
    commands
        .spawn((Node { position_type: PositionType::Absolute, width: percent(100), height: percent(100), justify_content: JustifyContent::Center, align_items: AlignItems::Center, ..default() }, BackgroundColor(Color::srgba(0.02, 0.03, 0.035, 0.8)), GlobalZIndex(9), Visibility::Hidden, Root))
        .with_children(|p| {
            p.spawn(Node { flex_direction: FlexDirection::Column, row_gap: px(10), width: px(460), ..default() }).with_children(|c| {
                c.spawn(t("SUBTE", 48.0, INK, &f.display));
                c.spawn(t("ELEGÍ LA ESTACIÓN · F CIERRA", 12.0, DIM, &f.display));
                for (i, n) in names.iter().enumerate() {
                    c.spawn((Node { flex_direction: FlexDirection::Row, column_gap: px(14), padding: UiRect::axes(px(12), px(8)), border: UiRect::all(px(1)), ..default() }, BorderColor::all(CYAN.with_alpha(0.4)), BackgroundColor(Color::srgba(0.063, 0.086, 0.098, 0.9)), Row(i))).with_children(|r| {
                        r.spawn(t(&format!("{}", i + 1), 16.0, CYAN, &f.mono));
                        r.spawn(t(&n.to_uppercase(), 20.0, INK, &f.display));
                    });
                }
            });
        });
}

#[allow(clippy::too_many_arguments)]
fn input(key: Res<ButtonInput<KeyCode>>, cursor: Single<&CursorOptions>, level: Res<LevelRes>, game: Res<GameRes>, mut open: ResMut<SubwayOpen>, mut lp: ResMut<LocalPlayer>, mut ui: ResMut<UiFeed>, mut snd: ResMut<crate::audio::Sounds>) {
    if cursor.grab_mode == CursorGrabMode::None || game.0.driving.is_some() {
        return;
    }
    let here = platform_near(&level.0, lp.state.pos);
    if key.just_pressed(KeyCode::KeyF) {
        if open.0 {
            open.0 = false;
            return;
        }
        if here.is_some() {
            open.0 = true;
            return;
        }
    }
    if !open.0 {
        return;
    }
    let Some(c) = level.0.city.as_ref() else { return };
    for (i, k) in [KeyCode::Digit1, KeyCode::Digit2, KeyCode::Digit3, KeyCode::Digit4].into_iter().enumerate() {
        if key.just_pressed(k) {
            if let Some(st) = c.stations.get(i) {
                if Some(i) != here {
                    lp.state.pos = st.platform + Vec3::new(2.0, EYE, 0.0);
                    lp.state.vel = Vec3::ZERO;
                    lp.prev = lp.state.pos;
                    ui.banner(format!("ESTACIÓN {}", st.name.to_uppercase()), "Subí las escaleras para salir");
                    snd.play(crate::audio::S::Dash);
                }
                open.0 = false;
            }
        }
    }
}

fn show(open: Res<SubwayOpen>, mut root: Query<&mut Visibility, With<Root>>) {
    for mut v in &mut root {
        *v = if open.0 { Visibility::Inherited } else { Visibility::Hidden };
    }
}
