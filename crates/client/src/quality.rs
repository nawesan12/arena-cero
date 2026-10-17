//! Calidad gráfica (puerto de `QUALITY` en `01-renderer-escena.js`): limita FPS, sombras,
//! distancia de dibujado y antialias, para que las notebooks no levanten temperatura.
//! F1 baja · F2 media · F3 alta. Se guarda en `~/.arena-cero` entre sesiones (ver `config.rs`).

use crate::player::MainCamera;
use bevy::light::{CascadeShadowConfig, CascadeShadowConfigBuilder, DirectionalLightShadowMap};
use bevy::pbr::{DistanceFog, FogFalloff};
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions};
use std::time::{Duration, Instant};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Quality {
    Baja,
    Media,
    Alta,
}

pub struct Preset {
    /// Tope de FPS en juego. Siempre hay tope: no tiene sentido calentar la máquina de más.
    pub fps: u32,
    pub shadow: usize,
    pub far: f32,
    pub msaa: Msaa,
    /// Escala de resolución respecto de los píxeles lógicos (en Retina, 1 = la mitad de la nativa).
    pub scale: f32,
    pub cascades: usize,
    /// Hasta dónde llegan las sombras.
    pub shadow_dist: f32,
}

impl Quality {
    pub fn preset(self) -> Preset {
        match self {
            Quality::Baja => Preset { fps: 40, shadow: 1024, far: 200.0, msaa: Msaa::Off, scale: 0.75, cascades: 2, shadow_dist: 45.0 },
            Quality::Media => Preset { fps: 60, shadow: 2048, far: 280.0, msaa: Msaa::Sample4, scale: 1.0, cascades: 3, shadow_dist: 80.0 },
            Quality::Alta => Preset { fps: 120, shadow: 4096, far: 360.0, msaa: Msaa::Sample4, scale: 1.6, cascades: 4, shadow_dist: 150.0 },
        }
    }
    fn name(self) -> &'static str {
        match self {
            Quality::Baja => "baja",
            Quality::Media => "media",
            Quality::Alta => "alta",
        }
    }
    fn parse(s: &str) -> Option<Self> {
        match s.trim() {
            "baja" => Some(Quality::Baja),
            "media" => Some(Quality::Media),
            "alta" => Some(Quality::Alta),
            _ => None,
        }
    }
}

#[derive(Resource)]
pub struct QualityRes {
    pub q: Quality,
    /// Distancia final de la niebla del mapa (se recorta a la distancia de dibujado).
    pub fog: [f32; 2],
    pub dirty: bool,
}

fn load() -> Quality {
    std::env::var("ARENA_Q").ok().or_else(|| crate::config::get("calidad")).and_then(|v| Quality::parse(&v)).unwrap_or(Quality::Media)
}

fn save(q: Quality) {
    crate::config::set("calidad", q.name());
}

pub struct QualityPlugin;

impl Plugin for QualityPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(QualityRes { q: load(), fog: [30.0, 120.0], dirty: true })
            .insert_resource(BloomOn(crate::config::get("brillo").is_some_and(|v| v == "1")))
            .add_systems(Update, (switch_quality, apply_quality, toggle_bloom, apply_bloom).chain())
            .add_systems(Last, frame_cap);
    }
}

fn switch_quality(key: Res<ButtonInput<KeyCode>>, mut q: ResMut<QualityRes>) {
    let pick = if key.just_pressed(KeyCode::F1) {
        Some(Quality::Baja)
    } else if key.just_pressed(KeyCode::F2) {
        Some(Quality::Media)
    } else if key.just_pressed(KeyCode::F3) {
        Some(Quality::Alta)
    } else {
        None
    };
    if let Some(p) = pick.filter(|p| *p != q.q) {
        q.q = p;
        q.dirty = true;
        save(p);
        info!("calidad {}", p.name());
    }
}

fn apply_quality(mut q: ResMut<QualityRes>, mut shadow: ResMut<DirectionalLightShadowMap>, mut cam: Query<(&mut Projection, &mut DistanceFog, &mut Msaa), With<MainCamera>>, mut gun_cam: Query<&mut Msaa, (With<crate::gun::GunCamera>, Without<MainCamera>)>, mut suns: Query<(&DirectionalLight, &mut CascadeShadowConfig)>) {
    if !q.dirty {
        return;
    }
    let Ok((mut proj, mut fog, mut msaa)) = cam.single_mut() else { return };
    let p = q.q.preset();
    shadow.size = p.shadow;
    if let Projection::Perspective(pp) = &mut *proj {
        pp.far = p.far;
    }
    let end = q.fog[1].min(p.far - 10.0);
    fog.falloff = FogFalloff::Linear { start: q.fog[0].min(end * 0.4), end };
    *msaa = p.msaa;
    // sombras: Bevy las recalcula cada cuadro (la web las calculaba una vez), así que se acortan
    for (l, mut c) in &mut suns {
        if l.shadow_maps_enabled {
            *c = CascadeShadowConfigBuilder { num_cascades: p.cascades, first_cascade_far_bound: (p.shadow_dist * 0.12).max(6.0), maximum_distance: p.shadow_dist, ..default() }.build();
        }
    }
    // las dos cámaras dibujan sobre la misma ventana: tienen que usar el mismo antialias
    for mut m in &mut gun_cam {
        *m = p.msaa;
    }
    q.dirty = false;
}

/// Brillo (bloom) sí/no con F5. Apagado de entrada: pide HDR en las dos cámaras y la Radeon
/// lo siente; además cambia el aspecto, así que es opcional.
#[derive(Resource)]
pub struct BloomOn(pub bool);

fn toggle_bloom(key: Res<ButtonInput<KeyCode>>, mut on: ResMut<BloomOn>, mut ui: ResMut<crate::game::UiFeed>) {
    if key.just_pressed(KeyCode::F5) {
        on.0 = !on.0;
        crate::config::set("brillo", if on.0 { "1" } else { "0" });
        ui.banner(if on.0 { "BRILLO ACTIVADO" } else { "BRILLO DESACTIVADO" }, "F5 cambia");
    }
}

/// Las dos cámaras (mundo y arma) dibujan sobre la misma imagen: tienen que ser las dos HDR o ninguna.
fn apply_bloom(mut commands: Commands, on: Res<BloomOn>, main: Query<Entity, With<MainCamera>>, gun: Query<Entity, With<crate::gun::GunCamera>>, mut done: Local<Option<(bool, usize)>>) {
    let n = main.iter().count() + gun.iter().count();
    if *done == Some((on.0, n)) {
        return;
    }
    *done = Some((on.0, n));
    use bevy::camera::Hdr;
    use bevy::post_process::bloom::{Bloom, BloomPrefilter};
    for e in &main {
        if on.0 {
            commands.entity(e).insert((Hdr, Bloom { intensity: 0.12, prefilter: BloomPrefilter { threshold: 0.8, threshold_softness: 0.4 }, ..Bloom::NATURAL }));
        } else {
            commands.entity(e).remove::<(Bloom, Hdr)>();
        }
    }
    for e in &gun {
        if on.0 {
            commands.entity(e).insert(Hdr);
        } else {
            commands.entity(e).remove::<Hdr>();
        }
    }
}

/// FPS cuando no se está jugando (mouse suelto o ventana sin foco), como los menús de la web.
const IDLE_FPS: u32 = 30;

/// Tope de FPS: duerme lo que falta para completar el cuadro.
fn frame_cap(q: Res<QualityRes>, win: Single<(&Window, &CursorOptions)>, mut last: Local<Option<Instant>>) {
    let (w, cursor) = *win;
    let playing = w.focused && cursor.grab_mode != CursorGrabMode::None;
    let fps = if std::env::var_os("ARENA_BENCH").is_some() { 0 } else if playing { q.q.preset().fps } else { IDLE_FPS };
    let now = Instant::now();
    if fps > 0 {
        if let Some(prev) = *last {
            let target = Duration::from_secs_f64(1.0 / f64::from(fps));
            let spent = now - prev;
            if spent < target {
                std::thread::sleep(target - spent);
            }
        }
    }
    *last = Some(Instant::now());
}
