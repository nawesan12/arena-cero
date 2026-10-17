//! Sonido de la ciudad (nuevo): capas de ambiente en loop que se mezclan según dónde estás
//! (tránsito siempre; gente si hay peatones cerca; pájaros en plazas, parques y Barrio Jardín;
//! agua en la costanera; viento en las azoteas; zumbido en el subte; grillos de noche) y sonidos
//! sueltos con posición: bocinas de los autos que pasan, perros en los barrios y gaviotas en el río.

use crate::audio::{AMBS, Amb, Pcm, S, SfxVol, Sounds, ambience};
use crate::game::GameRes;
use crate::player::LocalPlayer;
use crate::world_render::LevelRes;
use arena_sim::city::town::{Use, XS, ZS, block_rect, block_use};
use bevy::audio::{AudioSinkPlayback, Volume};
use bevy::prelude::*;

pub struct AmbiencePlugin;

impl Plugin for AmbiencePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostStartup, setup).add_systems(Update, (mix, one_shots));
    }
}

#[derive(Component)]
struct Layer(Amb);

fn setup(mut commands: Commands, mut pcm: ResMut<Assets<Pcm>>) {
    for a in AMBS {
        let h = pcm.add(ambience(a));
        commands.spawn((AudioPlayer(h), PlaybackSettings::LOOP.with_volume(Volume::Linear(0.0)), Layer(a)));
    }
}

/// Qué uso tiene la manzana donde está (x, z), si está adentro de una.
fn block_at(x: f32, z: f32) -> Option<Use> {
    let i = (0..XS.len() - 1).find(|&i| x >= XS[i].c && x < XS[i + 1].c)?;
    let j = (0..ZS.len() - 1).find(|&j| z >= ZS[j].c && z < ZS[j + 1].c)?;
    let (x0, z0, x1, z1) = block_rect(i, j);
    (x > x0 - 4.0 && x < x1 + 4.0 && z > z0 - 4.0 && z < z1 + 4.0).then(|| block_use(i, j))
}

/// Volumen objetivo de cada capa según el lugar.
fn targets(game: &GameRes, level: &LevelRes, lp: &LocalPlayer) -> [f32; 7] {
    let mut v = [0.0f32; 7];
    let p = lp.state.pos;
    let feet = lp.state.feet();
    if level.0.city.is_none() {
        // arenas: solo un poco de viento
        v[4] = 0.25;
        return v;
    }
    let g = &game.0;
    let night = g.director.night();
    let under = feet < -1.5;
    if under {
        v[5] = 0.55;
        v[0] = 0.08;
        return v;
    }
    let high = ((feet - 8.0) / 25.0).clamp(0.0, 1.0);
    // tránsito: fuerte en la calle, más bajo arriba y de noche
    v[0] = 0.32 * (1.0 - high * 0.6) * (1.0 - night * 0.4);
    // gente: según cuántos peatones vivos hay cerca
    let near = g.people.iter().filter(|q| q.alive && q.pos.distance(p) < 25.0).count() as f32;
    v[1] = (near / 8.0).min(1.0) * 0.35 * (1.0 - high);
    // pájaros de día en plazas, parques y Jardín; grillos de noche
    let green = match block_at(p.x, p.z) {
        Some(Use::Park | Use::Plaza(_) | Use::Refugio) => 1.0,
        _ if p.z > 150.0 => 0.6,
        _ => 0.15,
    };
    v[2] = green * 0.4 * (1.0 - night);
    v[6] = (green + 0.2).min(1.0) * 0.3 * night;
    // agua: cerca de la costanera (norte y este)
    let shore = (p.z + 284.5).abs().min((p.x - 284.5).abs());
    v[3] = ((40.0 - shore) / 40.0).clamp(0.0, 1.0) * 0.45;
    // viento en las azoteas
    v[4] = high * 0.5 + 0.03;
    v
}

fn mix(time: Res<Time>, game: Res<GameRes>, level: Res<LevelRes>, lp: Res<LocalPlayer>, sfx: Res<SfxVol>, menu: Res<crate::campaign_ui::MenuOpen>, mut layers: Query<(&Layer, &mut AudioSink)>, mut cur: Local<[f32; 7]>) {
    let t = targets(&game, &level, &lp);
    let k = (time.delta_secs() * 1.2).min(1.0);
    let mute = if menu.0 { 0.35 } else { 1.0 };
    for (l, mut sink) in &mut layers {
        let i = AMBS.iter().position(|a| *a == l.0).unwrap_or(0);
        cur[i] += (t[i] - cur[i]) * k;
        sink.set_volume(Volume::Linear(cur[i] * sfx.0 * mute));
    }
}

/// Bocinas, perros y gaviotas, cada tanto y con posición.
fn one_shots(time: Res<Time>, game: Res<GameRes>, level: Res<LevelRes>, lp: Res<LocalPlayer>, mut snd: ResMut<Sounds>, mut next: Local<[f32; 3]>, mut rng: Local<u32>) {
    if level.0.city.is_none() {
        return;
    }
    let dt = time.delta_secs();
    let p = lp.state.pos;
    if lp.state.feet() < -1.5 {
        return;
    }
    let mut rand = || {
        *rng = rng.wrapping_mul(1664525).wrapping_add(1013904223);
        (*rng >> 8) as f32 / 16_777_216.0
    };
    for n in next.iter_mut() {
        *n -= dt;
    }
    let g = &game.0;
    // bocina de un auto del tránsito que esté cerca
    if next[0] <= 0.0 {
        next[0] = 5.0 + rand() * 12.0;
        if let Some(v) = g.vehicles.iter().filter(|v| v.ai.as_ref().is_some_and(|a| a.civil) && !v.wrecked && v.pos.distance(p) < 60.0).min_by(|a, b| a.pos.distance(p).total_cmp(&b.pos.distance(p))) {
            let d = v.pos.distance(p);
            snd.play_at(S::Horn, (0.5 - d * 0.006).max(0.08), v.pos);
        }
    }
    // perros en los barrios (más de noche)
    if next[1] <= 0.0 {
        next[1] = 9.0 + rand() * 20.0;
        let barrio = p.z > 140.0 || p.x < -150.0;
        if barrio && rand() < 0.6 + g.director.night() * 0.3 {
            let a = rand() * std::f32::consts::TAU;
            let at = p + Vec3::new(a.cos(), 0.0, a.sin()) * (20.0 + rand() * 30.0);
            snd.play_at(S::Bark, 0.25, at);
        }
    }
    // gaviotas sobre el río
    if next[2] <= 0.0 {
        next[2] = 6.0 + rand() * 10.0;
        let shore = (p.z + 284.5).abs().min((p.x - 284.5).abs());
        if shore < 60.0 && g.director.night() < 0.5 {
            let at = Vec3::new(p.x + 20.0, 15.0, -300.0).lerp(Vec3::new(300.0, 15.0, p.z), if (p.x - 284.5).abs() < (p.z + 284.5).abs() { 1.0 } else { 0.0 });
            snd.play_at(S::Gull, 0.3, at);
        }
    }
}
