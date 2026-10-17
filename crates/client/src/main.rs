//! Arena Cero, cliente nativo.
//!
//! Uso: `arena-cero [--op 1-8 | --libre | --map ciudad|deposito|fundicion|cantera|azotea] [--at x,z,yaw,pitch] [--weapon 1-4] [--shot archivo.png]`
//! Sin argumentos abre el menú principal (campaña, mundo libre, oleadas).
//! `--shot` saca una captura a los pocos segundos y cierra (sirve para revisar cambios sin jugar).

mod ambience;
mod arena_cycle;
mod audio;
mod campaign_ui;
mod config;
mod daynight;
mod decals;
mod enemies_view;
mod extras_view;
mod feed_ui;
mod fx;
mod game;
mod gun;
mod gunmodels;
mod hook_view;
mod hud;
mod humanoid;
mod markers;
mod materials;
mod people_view;
mod perks;
mod player;
mod quality;
mod render_scale;
mod shop;
mod streets_ui;
mod subway;
mod tacmap;
mod textures;
mod vehicles_view;
mod world_render;

use arena_sim::{MapId, build_level};
use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};
use bevy::window::PresentMode;

#[derive(Resource, Clone, Default)]
struct Args {
    map: Option<MapId>,
    at: Option<[f32; 4]>,
    shot: Option<String>,
    weapon: Option<usize>,
    /// `--op N`: arranca directo en la operación N (1-8).
    op: Option<usize>,
    /// `--libre`: mundo libre sin pasar por el menú.
    free: bool,
}

fn parse_args() -> Args {
    let mut a = Args::default();
    let mut it = std::env::args().skip(1);
    while let Some(k) = it.next() {
        match k.as_str() {
            "--map" => {
                a.map = it.next().and_then(|m| match m.as_str() {
                    "ciudad" => Some(MapId::Ciudad),
                    "deposito" => Some(MapId::Deposito),
                    "fundicion" => Some(MapId::Fundicion),
                    "cantera" => Some(MapId::Cantera),
                    "azotea" => Some(MapId::Azotea),
                    _ => None,
                })
            }
            "--at" => {
                let v: Vec<f32> = it.next().unwrap_or_default().split(',').filter_map(|s| s.parse().ok()).collect();
                if v.len() == 4 {
                    a.at = Some([v[0], v[1], v[2], v[3]]);
                }
            }
            "--shot" => a.shot = it.next(),
            "--op" => a.op = it.next().and_then(|w| w.parse::<usize>().ok()).map(|w| w.clamp(1, arena_sim::campaign::N_OPS) - 1),
            "--libre" => a.free = true,
            "--weapon" => a.weapon = it.next().and_then(|w| w.parse::<usize>().ok()).map(|w| w.clamp(1, 4) - 1),
            _ => eprintln!("opción desconocida: {k}"),
        }
    }
    a
}

/// `ARENA_NO=gun,enemies,ai,env,shadows`: apaga partes para medir cuánto cuesta cada una.
/// Se lee una sola vez (antes se leía la variable de entorno por enemigo en cada cuadro).
pub fn off(k: &str) -> bool {
    static NO: std::sync::OnceLock<Vec<String>> = std::sync::OnceLock::new();
    NO.get_or_init(|| std::env::var("ARENA_NO").map(|v| v.split(',').map(str::to_string).collect()).unwrap_or_default()).iter().any(|x| x == k)
}

fn main() {
    let args = parse_args();
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window { title: "Arena Cero".into(), present_mode: if std::env::var_os("ARENA_BENCH").is_some() { PresentMode::AutoNoVsync } else { PresentMode::AutoVsync }, resolution: (1440, 900).into(), ..default() }),
            ..default()
        }))
        .insert_resource(start_mode(&args))
        .insert_resource(args)
        .add_plugins((player::PlayerPlugin, hud::HudPlugin, quality::QualityPlugin, render_scale::RenderScalePlugin, gun::GunPlugin, fx::FxPlugin, audio::AudioSynthPlugin, enemies_view::EnemiesViewPlugin, markers::MarkersPlugin, perks::PerksPlugin, tacmap::TacMapPlugin, hook_view::HookViewPlugin, vehicles_view::VehiclesViewPlugin))
        .add_plugins((daynight::DayNightPlugin, shop::ShopPlugin, extras_view::ExtrasViewPlugin, people_view::PeopleViewPlugin, subway::SubwayPlugin, campaign_ui::CampaignUiPlugin, decals::DecalsPlugin, feed_ui::FeedUiPlugin, arena_cycle::ArenaCyclePlugin, streets_ui::StreetsUiPlugin, humanoid::HumanoidPlugin, ambience::AmbiencePlugin))
        .add_systems(PreStartup, setup)
        .add_systems(Update, (world_render::follow_sky, world_render::animate_lava, auto_shot, bench))
        .run();
}

/// Sin argumentos se abre el menú; con `--op`, `--libre`, `--map` o `--shot` se va directo.
fn start_mode(a: &Args) -> campaign_ui::StartMode {
    use campaign_ui::StartMode;
    if let Some(op) = a.op {
        StartMode::Op(op)
    } else if a.free {
        StartMode::Free
    } else if a.map.is_some_and(|m| m != MapId::Ciudad) || a.shot.is_some() || a.at.is_some() {
        StartMode::Direct
    } else {
        StartMode::Menu
    }
}

fn setup(mut commands: Commands, args: Res<Args>, mut quality: ResMut<quality::QualityRes>, mut settings: ResMut<player::Settings>, mut meshes: ResMut<Assets<Mesh>>, mut mats: ResMut<Assets<StandardMaterial>>, mut images: ResMut<Assets<Image>>) {
    let t0 = std::time::Instant::now();
    let mut level = build_level(args.map.unwrap_or(MapId::Ciudad));
    let t1 = t0.elapsed();
    let mut looks = materials::Looks::new(&mut images, &mut mats);
    world_render::spawn_level(&mut commands, &mut meshes, &mut mats, &mut looks, &level);
    info!(
        "{}: {} colisionadores, {} cajas visuales · generado en {:.0} ms, mallas en {:.0} ms",
        level.info.name,
        level.world.colliders().count(),
        level.world.visuals.len(),
        t1.as_secs_f32() * 1000.0,
        (t0.elapsed() - t1).as_secs_f32() * 1000.0
    );
    quality.fog = level.info.fog;
    quality.dirty = true;
    // en modo captura no se toma el mouse: la ventana puede quedar debajo de otras
    settings.input = args.shot.is_none();
    let mut g = arena_sim::game::Game::new(&mut level, 7);
    // con ARENA_DRIVE=1 arranca manejando el auto más cercano al inicio (para capturas de prueba)
    if std::env::var_os("ARENA_DRIVE").is_some() {
        let s = level.info.start;
        g.driving = (0..g.vehicles.len()).min_by(|&a, &b| {
            let d = |i: usize| (g.vehicles[i].pos.x - s[0]).hypot(g.vehicles[i].pos.z - s[1]);
            d(a).total_cmp(&d(b))
        });
    }
    commands.insert_resource(game::GameRes(g, 0));
    commands.init_resource::<game::UiFeed>();
    let res = world_render::LevelRes(level);
    player::spawn_player(&mut commands, &res, args.at, args.weapon);
    commands.insert_resource(res);
    commands.insert_resource(looks);
}

/// Con `--shot`: captura a los ~3 s (y otra de control un poco después) y sale.
fn auto_shot(mut commands: Commands, args: Res<Args>, time: Res<Time>, mut frames: Local<u32>, mut done: Local<u8>, mut exit: MessageWriter<AppExit>) {
    let Some(path) = &args.shot else { return };
    let t = time.elapsed_secs();
    *frames += 1;
    if *done == 0 && t > 3.0 && *frames > 120 {
        commands.spawn(Screenshot::primary_window()).observe(save_to_disk(path.clone()));
        *done = 1;
    } else if *done == 1 && *frames > 200 {
        if std::env::var("ARENA_SHOT2").is_ok() {
            commands.spawn(Screenshot::primary_window()).observe(save_to_disk(path.replace(".png", "-b.png")));
        }
        *done = 2;
    } else if *done == 2 && *frames > 260 && std::env::var_os("ARENA_BENCH").is_none() {
        exit.write(AppExit::Success);
    }
}

/// Con `ARENA_BENCH=1`: mide el tiempo medio por cuadro entre los 4 y los 10 s y lo imprime.
fn bench(time: Res<Time<Real>>, mut acc: Local<(u32, f64, bool)>, ents: Query<()>, meshes: Query<(), With<Mesh3d>>, nodes: Query<(), With<Node>>, mut exit: MessageWriter<AppExit>) {
    if std::env::var_os("ARENA_BENCH").is_none() {
        return;
    }
    let t = time.elapsed_secs_f64();
    if t > 4.0 && t < 10.0 {
        acc.0 += 1;
        acc.1 += time.delta_secs_f64();
    } else if t >= 10.0 && !acc.2 {
        acc.2 = true;
        let ms = acc.1 / f64::from(acc.0.max(1)) * 1000.0;
        println!("BENCH {:.2} ms/cuadro ({:.0} FPS) · {} entidades ({} mallas, {} nodos de UI)", ms, 1000.0 / ms, ents.iter().count(), meshes.iter().count(), nodes.iter().count());
        exit.write(AppExit::Success);
    }
}
