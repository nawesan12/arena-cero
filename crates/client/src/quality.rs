//! Calidad gráfica (puerto de `QUALITY` en `01-renderer-escena.js`): limita FPS, sombras,
//! distancia de dibujado y antialias, para que las notebooks no levanten temperatura.
//! F1 baja · F2 media · F3 alta. Se guarda en `~/.arena-cero` entre sesiones.

use crate::player::MainCamera;
use bevy::light::DirectionalLightShadowMap;
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
}

impl Quality {
    pub fn preset(self) -> Preset {
        match self {
            Quality::Baja => Preset { fps: 40, shadow: 1024, far: 200.0, msaa: Msaa::Off },
            Quality::Media => Preset { fps: 60, shadow: 2048, far: 280.0, msaa: Msaa::Sample4 },
            Quality::Alta => Preset { fps: 120, shadow: 4096, far: 360.0, msaa: Msaa::Sample4 },
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

fn cfg_path() -> Option<std::path::PathBuf> {
    std::env::var_os("HOME").map(|h| std::path::Path::new(&h).join(".arena-cero"))
}

fn load() -> Quality {
    cfg_path()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|s| s.lines().find_map(|l| l.strip_prefix("calidad=").and_then(Quality::parse)))
        .unwrap_or(Quality::Media)
}

fn save(q: Quality) {
    if let Some(p) = cfg_path() {
        let _ = std::fs::write(p, format!("calidad={}\n", q.name()));
    }
}

pub struct QualityPlugin;

impl Plugin for QualityPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(QualityRes { q: load(), fog: [30.0, 120.0], dirty: true })
            .add_systems(Update, (switch_quality, apply_quality).chain())
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

fn apply_quality(mut q: ResMut<QualityRes>, mut shadow: ResMut<DirectionalLightShadowMap>, mut cam: Query<(&mut Projection, &mut DistanceFog, &mut Msaa), With<MainCamera>>) {
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
    q.dirty = false;
}

/// FPS cuando no se está jugando (mouse suelto o ventana sin foco), como los menús de la web.
const IDLE_FPS: u32 = 30;

/// Tope de FPS: duerme lo que falta para completar el cuadro.
fn frame_cap(q: Res<QualityRes>, win: Single<(&Window, &CursorOptions)>, mut last: Local<Option<Instant>>) {
    let (w, cursor) = *win;
    let playing = w.focused && cursor.grab_mode != CursorGrabMode::None;
    let fps = if playing { q.q.preset().fps } else { IDLE_FPS };
    let now = Instant::now();
    {
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
