//! Oleadas con cambio de arena (nuevo, como la campaña de oleadas de la web): cada 3 oleadas
//! despejadas, la partida sigue en la próxima arena (Depósito → Cantera → Fundición → Azotea)
//! con el mismo puntaje, mejoras, armas y vida. No hace falta salir del juego.

use crate::game::{GameRes, UiFeed};
use crate::materials::Looks;
use crate::player::{LocalPlayer, Look, MainCamera};
use crate::world_render::{Geometry, LevelRes};
use arena_sim::game::{Game, Mode};
use arena_sim::player::EYE;
use arena_sim::{MapId, build_level};
use bevy::pbr::{DistanceFog, FogFalloff};
use bevy::prelude::*;

pub struct ArenaCyclePlugin;

impl Plugin for ArenaCyclePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, cycle);
    }
}

/// Cada cuántas oleadas se cambia de arena.
const EVERY: u32 = 3;

#[allow(clippy::too_many_arguments)]
fn cycle(
    mut commands: Commands,
    mut game: ResMut<GameRes>,
    mut level: ResMut<LevelRes>,
    mut lp: ResMut<LocalPlayer>,
    mut look: ResMut<Look>,
    mut ui: ResMut<UiFeed>,
    (mut meshes, mut mats, mut looks, mut quality): (ResMut<Assets<Mesh>>, ResMut<Assets<StandardMaterial>>, ResMut<Looks>, ResMut<crate::quality::QualityRes>),
    geometry: Query<Entity, With<Geometry>>,
    mut fog: Query<&mut DistanceFog, With<MainCamera>>,
    // (oleada en la que se anunció, momento de la partida en que se cambia)
    mut pending: Local<Option<(u32, f32)>>,
    mut done_wave: Local<u32>,
    net: Res<crate::online::Online>,
) {
    let g = &game.0;
    // online la arena no cambia (los demás tendrían que reconstruir todo a la vez)
    if g.mode != Mode::Waves || net.active() {
        *pending = None;
        return;
    }
    let cleared = g.wave > 0 && g.wave % EVERY == 0 && g.enemies.is_empty() && g.spawn_queue.is_empty() && g.intermission > 0.0;
    let order = MapId::CAMPAIGN;
    let cur = order.iter().position(|m| *m == level.0.info.id).unwrap_or(0);
    let next = order[(cur + 1) % order.len()];
    if pending.is_none() && cleared && *done_wave != g.wave {
        *pending = Some((g.wave, g.time + 2.0));
        let name = build_level(next).info.name.to_uppercase();
        ui.banner("NUEVA ARENA", format!("Próxima parada: {name}"));
    }
    let Some((wave, at)) = *pending else { return };
    if g.time < at {
        return;
    }
    *pending = None;
    *done_wave = wave;
    // mapa nuevo: geometría, partida (con la cuenta) y jugador en el inicio
    for e in &geometry {
        commands.entity(e).despawn();
    }
    let old = std::mem::replace(&mut game.0, Game::new(&mut level.0, 0));
    level.0 = build_level(next);
    game.0 = Game::next_arena(&mut level.0, &old);
    game.1 += 1;
    crate::world_render::spawn_level(&mut commands, &mut meshes, &mut mats, &mut looks, &level.0);
    let info = &level.0.info;
    let s = info.start;
    let feet = level.0.world.ground_at(s[0], s[1], 200.0);
    let p = Vec3::new(s[0], feet + EYE, s[1]);
    lp.state.pos = p;
    lp.state.vel = Vec3::ZERO;
    lp.state.hook = None;
    lp.prev = p;
    lp.start = p;
    lp.hp = lp.max_hp;
    lp.arsenal.refill();
    look.pitch = 0.0;
    quality.fog = info.fog;
    quality.dirty = true;
    let end = info.fog[1].min(350.0);
    for mut f in &mut fog {
        f.color = crate::world_render::col(info.bg);
        f.falloff = FogFalloff::Linear { start: info.fog[0].min(end * 0.4), end };
    }
    ui.banner(info.name.to_uppercase(), format!("Oleada {} · {} puntos", game.0.wave + 1, game.0.score));
}
