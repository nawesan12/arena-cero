//! Arena Cero, cliente nativo.
//!
//! Uso: `arena-cero [--map ciudad|deposito|fundicion|cantera|azotea] [--at x,z,yaw,pitch] [--shot archivo.png]`
//! `--shot` saca una captura a los pocos segundos y cierra (sirve para revisar cambios sin jugar).

mod hud;
mod materials;
mod player;
mod quality;
mod textures;
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
            _ => eprintln!("opción desconocida: {k}"),
        }
    }
    a
}

fn main() {
    let args = parse_args();
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window { title: "Arena Cero".into(), present_mode: PresentMode::AutoVsync, resolution: (1440, 900).into(), ..default() }),
            ..default()
        }))
        .insert_resource(args)
        .add_plugins((player::PlayerPlugin, hud::HudPlugin, quality::QualityPlugin))
        .add_systems(PreStartup, setup)
        .add_systems(Update, (world_render::follow_sky, world_render::animate_lava, auto_shot))
        .run();
}

fn setup(mut commands: Commands, args: Res<Args>, mut quality: ResMut<quality::QualityRes>, mut settings: ResMut<player::Settings>, mut meshes: ResMut<Assets<Mesh>>, mut mats: ResMut<Assets<StandardMaterial>>, mut images: ResMut<Assets<Image>>) {
    let t0 = std::time::Instant::now();
    let level = build_level(args.map.unwrap_or(MapId::Ciudad));
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
    let res = world_render::LevelRes(level);
    player::spawn_player(&mut commands, &res, args.at);
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
    } else if *done == 2 && *frames > 260 {
        exit.write(AppExit::Success);
    }
}
