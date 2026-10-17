//! Escala de resolución (el `pixelRatio` de la web): la escena 3D y el arma se dibujan en una
//! textura más chica que la ventana y se estiran a pantalla completa; el HUD se dibuja encima a
//! resolución nativa, así el texto queda nítido. En una pantalla Retina, calidad media dibuja la
//! escena a la mitad de la resolución nativa (4 veces menos píxeles).

use crate::gun::GunCamera;
use crate::player::MainCamera;
use crate::quality::QualityRes;
use bevy::camera::RenderTarget;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureFormat};
use bevy::window::PrimaryWindow;

pub struct RenderScalePlugin;

impl Plugin for RenderScalePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostStartup, setup).add_systems(Last, resize);
    }
}

#[derive(Resource)]
pub struct SceneTarget {
    pub image: Handle<Image>,
    pub size: UVec2,
}

/// Tamaño en píxeles de la textura de escena para la ventana y la calidad actuales.
fn target_size(win: &Window, scale: f32) -> UVec2 {
    let native = win.resolution.scale_factor().max(1.0);
    let k = (scale / native).min(1.0);
    let p = win.resolution.physical_size().as_vec2() * k;
    p.max(Vec2::splat(64.0)).as_uvec2()
}

fn setup(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    q: Res<QualityRes>,
    win: Single<&Window, With<PrimaryWindow>>,
    main_cam: Query<Entity, With<MainCamera>>,
    gun_cam: Query<Entity, With<GunCamera>>,
) {
    let size = target_size(&win, q.q.preset().scale);
    let image = images.add(Image::new_target_texture(size.x, size.y, TextureFormat::Rgba8UnormSrgb, None));
    for e in main_cam.iter().chain(gun_cam.iter()) {
        commands.entity(e).insert(RenderTarget::Image(image.clone().into()));
    }
    // cámara de la interfaz: pinta la escena estirada y el HUD encima, sobre la ventana
    commands.spawn((Camera2d, Camera { order: 2, ..default() }, Msaa::Off, IsDefaultUiCamera));
    commands.spawn((
        Node { position_type: PositionType::Absolute, width: percent(100), height: percent(100), ..default() },
        ImageNode::new(image.clone()).with_mode(NodeImageMode::Stretch),
        GlobalZIndex(-100),
    ));
    commands.insert_resource(SceneTarget { image, size });
}

fn resize(mut images: ResMut<Assets<Image>>, q: Res<QualityRes>, win: Single<&Window, With<PrimaryWindow>>, target: Option<ResMut<SceneTarget>>) {
    let Some(mut t) = target else { return };
    let size = target_size(&win, q.q.preset().scale);
    if size == t.size {
        return;
    }
    t.size = size;
    if let Some(mut img) = images.get_mut(&t.image) {
        img.resize(Extent3d { width: size.x, height: size.y, depth_or_array_layers: 1 });
    }
}
