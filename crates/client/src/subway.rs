//! Subte: en cada estación entra un tren por el túnel, frena, abre las puertas y sigue viaje.
//! Con las puertas abiertas, F en el andén abre la lista de estaciones; 1-3 viaja (la partida se
//! pausa mientras elegís) y el viaje es un fundido a negro. Bajo tierra los enemigos no te ven.

use crate::game::{GameRes, UiFeed};
use crate::hud::Fonts;
use crate::player::LocalPlayer;
use crate::world_render::LevelRes;
use arena_sim::player::EYE;
use arena_sim::subway::{CARS, HEIGHT, LEN, WIDTH, doors, next_in, offset};
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions};

const INK: Color = Color::srgb(0.902, 0.929, 0.910);
const DIM: Color = Color::srgb(0.498, 0.549, 0.529);
const CYAN: Color = Color::srgb(0.373, 0.847, 1.0);

pub struct SubwayPlugin;

impl Plugin for SubwayPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SubwayOpen>().init_resource::<Fade>().add_systems(PostStartup, (spawn, spawn_trains)).add_systems(Update, (input, show, trains, fade));
    }
}

#[derive(Resource, Default)]
pub struct SubwayOpen(pub bool);

/// Fundido del viaje (1 → 0).
#[derive(Resource, Default)]
struct Fade(f32);

#[derive(Component)]
struct Root;
#[derive(Component)]
struct Row(usize);
#[derive(Component)]
struct Hint;
#[derive(Component)]
struct FadeNode;
#[derive(Component)]
struct FadeText;
/// Tren de la estación `0`; las puertas guardan su lado y su posición cerrada.
#[derive(Component)]
struct TrainView(usize);
#[derive(Component)]
struct Door {
    x: f32,
    dir: f32,
}

/// Estación en cuyo andén está el jugador.
pub fn platform_near(level: &arena_sim::Level, pos: Vec3) -> Option<usize> {
    level.city.as_ref()?.stations.iter().position(|s| (s.platform.x - pos.x).abs() < LEN / 2.0 && (s.platform.z - pos.z).abs() < 3.5 && (s.platform.y + EYE - pos.y).abs() < 1.5)
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
    commands.spawn((Node { position_type: PositionType::Absolute, bottom: px(196), width: percent(100), justify_content: JustifyContent::Center, ..default() }, t("", 15.0, CYAN, &f.display), TextShadow::default(), Hint));
    commands
        .spawn((Node { position_type: PositionType::Absolute, width: percent(100), height: percent(100), justify_content: JustifyContent::Center, align_items: AlignItems::Center, ..default() }, BackgroundColor(Color::NONE), GlobalZIndex(8), FadeNode))
        .with_children(|p| {
            p.spawn((t("", 30.0, INK, &f.display), FadeText));
        });
}

/// Trenes: tres coches con franja, ventanas iluminadas y puertas que se corren.
fn spawn_trains(mut commands: Commands, level: Res<LevelRes>, mut meshes: ResMut<Assets<Mesh>>, mut mats: ResMut<Assets<StandardMaterial>>) {
    let Some(c) = level.0.city.as_ref() else { return };
    let cube = meshes.add(Cuboid::new(1.0, 1.0, 1.0));
    let body = mats.add(StandardMaterial { base_color: crate::world_render::col(0xb9c2c8), metallic: 0.6, perceptual_roughness: 0.35, ..default() });
    let stripe = mats.add(StandardMaterial { base_color: crate::world_render::col(0x5fd8ff), emissive: LinearRgba::rgb(0.2, 0.7, 1.0), ..default() });
    let glass = mats.add(StandardMaterial { base_color: crate::world_render::col(0xfff1cc), unlit: true, ..default() });
    let door_m = mats.add(StandardMaterial { base_color: crate::world_render::col(0x8a959c), metallic: 0.5, perceptual_roughness: 0.4, ..default() });
    let dark = mats.add(StandardMaterial { base_color: crate::world_render::col(0x23292d), perceptual_roughness: 0.9, ..default() });
    let head = mats.add(StandardMaterial { base_color: crate::world_render::col(0xfff6dc), unlit: true, ..default() });
    for (i, st) in c.stations.iter().enumerate() {
        let side = (st.platform.z - st.track.z).signum();
        let car_len = LEN / CARS as f32 - 0.6;
        let part = |commands: &mut ChildSpawnerCommands, size: Vec3, at: Vec3, m: &Handle<StandardMaterial>| {
            commands.spawn((Mesh3d(cube.clone()), MeshMaterial3d(m.clone()), Transform::from_translation(at).with_scale(size)));
        };
        commands
            .spawn((Transform::from_translation(st.track), Visibility::Hidden, TrainView(i), crate::world_render::LevelEntity))
            .with_children(|p| {
                for k in 0..CARS {
                    let cx = (k as f32 - (CARS as f32 - 1.0) / 2.0) * (LEN / CARS as f32);
                    let y0 = 0.3;
                    part(p, Vec3::new(car_len, HEIGHT - 0.5, WIDTH), Vec3::new(cx, y0 + 0.25 + (HEIGHT - 0.5) / 2.0, 0.0), &body);
                    part(p, Vec3::new(car_len - 0.4, 0.5, WIDTH - 0.3), Vec3::new(cx, y0 + 0.25, 0.0), &dark);
                    part(p, Vec3::new(car_len, 0.18, WIDTH + 0.02), Vec3::new(cx, y0 + 1.2, 0.0), &stripe);
                    // ventanas a los dos lados
                    for s in [-1.0f32, 1.0] {
                        part(p, Vec3::new(car_len - 1.4, 0.9, 0.03), Vec3::new(cx, y0 + 2.35, s * (WIDTH / 2.0 + 0.01)), &glass);
                    }
                    // dos puertas dobles del lado del andén
                    for dx in [-car_len / 4.0, car_len / 4.0] {
                        for dir in [-1.0f32, 1.0] {
                            let x = cx + dx + dir * 0.55;
                            p.spawn((Mesh3d(cube.clone()), MeshMaterial3d(door_m.clone()), Transform::from_translation(Vec3::new(x, y0 + 0.25 + 1.2, side * (WIDTH / 2.0 + 0.03))).with_scale(Vec3::new(1.1, 2.4, 0.06)), Door { x, dir }));
                        }
                    }
                }
                for e in [-1.0f32, 1.0] {
                    part(p, Vec3::new(0.05, 0.35, 0.5), Vec3::new(e * (LEN / 2.0 - 0.28), 1.1, 0.8), &head);
                    part(p, Vec3::new(0.05, 0.35, 0.5), Vec3::new(e * (LEN / 2.0 - 0.28), 1.1, -0.8), &head);
                }
            });
    }
}

fn trains(game: Res<GameRes>, mut q: Query<(&TrainView, &mut Transform, &mut Visibility, &Children)>, mut door_q: Query<(&Door, &mut Transform), Without<TrainView>>, level: Res<LevelRes>) {
    let Some(c) = level.0.city.as_ref() else { return };
    for (tv, mut t, mut v, kids) in &mut q {
        let Some(tr) = game.0.trains.get(tv.0) else { continue };
        let Some(st) = c.stations.get(tv.0) else { continue };
        match offset(tr.t) {
            Some(dx) => {
                *v = Visibility::Inherited;
                t.translation = st.track + Vec3::X * dx;
                let open = doors(tr.t);
                for k in kids.iter() {
                    if let Ok((d, mut dt)) = door_q.get_mut(k) {
                        dt.translation.x = d.x + d.dir * open * 1.0;
                    }
                }
            }
            None => *v = Visibility::Hidden,
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn input(
    key: Res<ButtonInput<KeyCode>>,
    cursor: Single<&CursorOptions>,
    level: Res<LevelRes>,
    mut game: ResMut<GameRes>,
    mut open: ResMut<SubwayOpen>,
    mut lp: ResMut<LocalPlayer>,
    mut ui: ResMut<UiFeed>,
    mut snd: ResMut<crate::audio::Sounds>,
    mut fade: ResMut<Fade>,
    mut hint: Query<&mut Text, With<Hint>>,
    mut fade_txt: Query<&mut Text, (With<FadeText>, Without<Hint>)>,
) {
    let mut msg = String::new();
    if cursor.grab_mode != CursorGrabMode::None && game.0.driving.is_none() {
        let feet = Vec3::new(lp.state.pos.x, lp.state.feet(), lp.state.pos.z);
        let here = platform_near(&level.0, lp.state.pos);
        let board = game.0.train_boardable(&level.0, feet);
        if !open.0 {
            if let Some(i) = here {
                msg = match board {
                    Some(_) => "F · SUBIR AL TREN".into(),
                    None => game.0.trains.get(i).map_or(String::new(), |t| format!("PRÓXIMO TREN EN {:.0} S", next_in(t.t).ceil())),
                };
            }
        }
        if key.just_pressed(KeyCode::KeyF) {
            if open.0 {
                open.0 = false;
            } else if board.is_some() {
                open.0 = true;
            }
        } else if open.0 {
            let Some(from) = board.or(here) else {
                open.0 = false;
                return;
            };
            let n = level.0.city.as_ref().map_or(0, |c| c.stations.len());
            for (i, k) in [KeyCode::Digit1, KeyCode::Digit2, KeyCode::Digit3, KeyCode::Digit4].into_iter().enumerate().take(n) {
                if key.just_pressed(k) {
                    open.0 = false;
                    if i == from {
                        continue;
                    }
                    if let Some(p) = game.0.ride_train(&level.0, from, i) {
                        lp.state.pos = p + Vec3::Y * EYE;
                        lp.state.vel = Vec3::ZERO;
                        lp.prev = lp.state.pos;
                        game.0.campaign_rode();
                        // bajo tierra nadie te sigue: se calman las alarmas
                        game.0.on_player_down();
                        let name = level.0.city.as_ref().map_or("", |c| c.stations[i].name);
                        ui.banner(format!("ESTACIÓN {}", name.to_uppercase()), "Subí las escaleras para salir");
                        for mut t in &mut fade_txt {
                            t.0 = format!("VIAJANDO A {}", name.to_uppercase());
                        }
                        fade.0 = 1.0;
                        snd.play(crate::audio::S::Train);
                    }
                }
            }
        }
    }
    for mut t in &mut hint {
        if t.0 != msg {
            t.0 = msg.clone();
        }
    }
}

fn fade(time: Res<Time>, mut fade: ResMut<Fade>, mut node: Query<&mut BackgroundColor, With<FadeNode>>, mut txt: Query<&mut TextColor, With<FadeText>>) {
    if fade.0 <= 0.0 {
        return;
    }
    fade.0 = (fade.0 - time.delta_secs() / 1.8).max(0.0);
    // negro un rato y después se aclara
    let a = (fade.0 * 1.8).min(1.0);
    for mut b in &mut node {
        b.0 = Color::srgba(0.0, 0.0, 0.0, a);
    }
    for mut c in &mut txt {
        c.0 = INK.with_alpha(a);
    }
}

fn show(open: Res<SubwayOpen>, mut root: Query<&mut Visibility, With<Root>>) {
    for mut v in &mut root {
        *v = if open.0 { Visibility::Inherited } else { Visibility::Hidden };
    }
}
