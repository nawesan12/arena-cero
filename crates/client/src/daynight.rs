//! Día y noche en la ciudad (nuevo): el sol cruza el cielo según la hora del director, el
//! atardecer se pone cálido, de noche hay luna azulada, el cielo y la niebla se oscurecen, los
//! faroles cercanos se prenden y la linterna (L) alumbra hacia adelante.

use crate::game::GameRes;
use crate::player::MainCamera;
use crate::world_render::{LevelRes, SkyDome, SunDisc, WorldSun};
use arena_sim::Mat;
use bevy::pbr::DistanceFog;
use bevy::prelude::*;

pub struct DayNightPlugin;

impl Plugin for DayNightPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Flashlight>().add_systems(PostStartup, setup).add_systems(Update, (toggle_light, update));
    }
}

#[derive(Resource, Default)]
pub struct Flashlight(pub bool);

#[derive(Component)]
struct LampLight;
#[derive(Component)]
struct Torch;

#[derive(Resource)]
struct Lamps(Vec<Vec3>);

/// Color sRGB 0xRRGGBB a lineal, para mezclar.
fn lin(h: u32) -> Vec3 {
    let c = crate::world_render::col(h).to_linear();
    Vec3::new(c.red, c.green, c.blue)
}

fn setup(mut commands: Commands, level: Res<LevelRes>, cam: Query<Entity, With<MainCamera>>) {
    // faroles: la pieza luminosa de cada poste (se prenden solo los más cercanos)
    let lamps: Vec<Vec3> = level.0.world.visuals.iter().filter(|v| v.mat == Mat::Lamp).map(|v| v.center - Vec3::Y * 0.3).collect();
    commands.insert_resource(Lamps(lamps));
    for _ in 0..14 {
        commands.spawn((PointLight { intensity: 0.0, range: 18.0, color: Color::srgb(1.0, 0.86, 0.62), shadow_maps_enabled: false, ..default() }, Transform::default(), LampLight));
    }
    if let Ok(c) = cam.single() {
        let torch = commands.spawn((SpotLight { intensity: 0.0, range: 45.0, outer_angle: 0.42, inner_angle: 0.28, color: Color::srgb(1.0, 0.96, 0.88), shadow_maps_enabled: false, ..default() }, Transform::from_xyz(0.25, -0.2, 0.0), Torch)).id();
        commands.entity(c).add_child(torch);
    }
}

fn toggle_light(key: Res<ButtonInput<KeyCode>>, mut fl: ResMut<Flashlight>, mut snd: ResMut<crate::audio::Sounds>) {
    if key.just_pressed(KeyCode::KeyL) {
        fl.0 = !fl.0;
        snd.play(crate::audio::S::Swap);
    }
}

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
fn update(
    game: Res<GameRes>,
    level: Res<LevelRes>,
    fl: Res<Flashlight>,
    lamps: Res<Lamps>,
    lp: Res<crate::player::LocalPlayer>,
    mut ambient: ResMut<GlobalAmbientLight>,
    mut clear: ResMut<ClearColor>,
    mut sun: Query<(&mut DirectionalLight, &mut Transform), (With<WorldSun>, Without<SunDisc>, Without<LampLight>)>,
    mut fog: Query<&mut DistanceFog, With<MainCamera>>,
    sky: Query<&MeshMaterial3d<StandardMaterial>, With<SkyDome>>,
    mut disc: Query<(&mut Visibility, &mut SunDisc), Without<LampLight>>,
    mut mats: ResMut<Assets<StandardMaterial>>,
    mut lamp_lights: Query<(&mut PointLight, &mut Transform), (With<LampLight>, Without<DirectionalLight>)>,
    mut torch: Query<&mut SpotLight, With<Torch>>,
    mut last: Local<f32>,
) {
    let info = &level.0.info;
    for mut t in &mut torch {
        t.intensity = if fl.0 { 2_400_000.0 } else { 0.0 };
    }
    if !info.city {
        return;
    }
    let hour = game.0.director.hour;
    let night = game.0.director.night();
    // no hace falta recalcular todo cada cuadro: cambia despacio
    if (hour - *last).abs() < 0.01 && *last != 0.0 {
        return;
    }
    *last = hour;
    // sol: sale a las 6, cenit a las 12, se pone a las 18
    let ang = (hour - 6.0) / 12.0 * std::f32::consts::PI;
    let elev = ang.sin();
    let sun_dir = Vec3::new(ang.cos() * 0.8, elev.max(-0.3), 0.45).normalize();
    let day_k = (elev * 3.0).clamp(0.0, 1.0);
    let dusk = (1.0 - (elev * 4.0 - 0.2).abs()).clamp(0.0, 1.0) * (elev > -0.1) as u8 as f32;
    let (sun_c, sun_i) = info.sun;
    let base_c = lin(sun_c).lerp(lin(0xff9a52), dusk * 0.8);
    let moon = Vec3::new(-0.5, 0.75, -0.4).normalize();
    for (mut l, mut t) in &mut sun {
        if day_k > 0.02 {
            l.color = Color::linear_rgb(base_c.x, base_c.y, base_c.z);
            l.illuminance = sun_i * 3000.0 * day_k;
            *t = Transform::from_translation(sun_dir * 200.0).looking_at(Vec3::ZERO, Vec3::Y);
        } else {
            // luna
            l.color = Color::srgb(0.55, 0.65, 1.0);
            l.illuminance = 380.0 * night;
            *t = Transform::from_translation(moon * 200.0).looking_at(Vec3::ZERO, Vec3::Y);
        }
    }
    let (sky_c, ground_c, hemi_i) = info.hemi;
    let day_amb = lin(sky_c) * 0.6 + lin(ground_c) * 0.4;
    let night_amb = lin(0x3a4a78);
    let amb = day_amb.lerp(night_amb, night);
    ambient.color = Color::linear_rgb(amb.x, amb.y, amb.z);
    ambient.brightness = hemi_i * 1000.0 * (1.0 - night * 0.82) * (0.75 + 0.25 * (1.0 - dusk));
    // cielo, niebla y fondo
    let bg = lin(info.bg).lerp(lin(0xd88a5a), dusk * 0.5).lerp(lin(0x0a1020), night);
    let bgc = Color::linear_rgb(bg.x, bg.y, bg.z);
    clear.0 = bgc;
    for mut f in &mut fog {
        f.color = bgc;
    }
    let tint = Vec3::ONE.lerp(Vec3::new(1.0, 0.75, 0.6), dusk * 0.6).lerp(Vec3::new(0.08, 0.1, 0.2), night);
    for h in &sky {
        if let Some(mut m) = mats.get_mut(&h.0) {
            m.base_color = Color::linear_rgb(tint.x, tint.y, tint.z);
        }
    }
    for (mut v, mut d) in &mut disc {
        *v = if day_k > 0.02 { Visibility::Inherited } else { Visibility::Hidden };
        d.0 = sun_dir * 320.0;
    }
    // faroles: los 14 más cercanos al jugador se prenden de noche
    let me = lp.state.pos;
    let mut near: Vec<Vec3> = lamps.0.clone();
    near.sort_by(|a, b| a.distance_squared(me).total_cmp(&b.distance_squared(me)));
    let on = ((night - 0.25) / 0.5).clamp(0.0, 1.0);
    for (i, (mut l, mut t)) in lamp_lights.iter_mut().enumerate() {
        match near.get(i) {
            Some(p) if on > 0.0 => {
                t.translation = *p;
                l.intensity = 900_000.0 * on;
            }
            _ => l.intensity = 0.0,
        }
    }
}
