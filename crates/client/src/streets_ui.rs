//! Nombres de calles (nuevo): al acercarte a una esquina aparecen los carteles de las dos calles
//! que se cruzan, como los de las esquinas porteñas. Ayuda a ubicarse y a seguir la radio.

use crate::hud::Fonts;
use crate::player::{LocalPlayer, MainCamera};
use crate::world_render::LevelRes;
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions};

pub struct StreetsUiPlugin;

impl Plugin for StreetsUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostStartup, spawn).add_systems(Update, update);
    }
}

#[derive(Component)]
struct Sign(usize);
#[derive(Component)]
struct SignText(usize);

fn spawn(mut commands: Commands, f: Res<Fonts>) {
    for i in 0..2 {
        commands
            .spawn((Node { position_type: PositionType::Absolute, padding: UiRect::axes(px(8), px(3)), border: UiRect::all(px(1)), margin: UiRect { left: px(-60), ..default() }, ..default() }, BorderColor::all(Color::WHITE.with_alpha(0.7)), BackgroundColor(Color::srgba(0.06, 0.12, 0.2, 0.85)), Visibility::Hidden, Sign(i)))
            .with_children(|p| {
                p.spawn((Text::new(""), TextFont { font: FontSource::Handle(f.display.clone()), font_size: 13.0.into(), ..default() }, TextColor(Color::WHITE), SignText(i)));
            });
    }
}

#[allow(clippy::type_complexity)]
fn update(
    level: Res<LevelRes>,
    lp: Res<LocalPlayer>,
    cursor: Single<&CursorOptions>,
    win: Single<&Window>,
    target: Option<Res<crate::render_scale::SceneTarget>>,
    cam: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    mut signs: Query<(&Sign, &mut Node, &mut Visibility)>,
    mut texts: Query<(&SignText, &mut Text)>,
) {
    let hide = |signs: &mut Query<(&Sign, &mut Node, &mut Visibility)>| {
        for (_, _, mut v) in signs.iter_mut() {
            *v = Visibility::Hidden;
        }
    };
    let Some(c) = level.0.city.as_ref() else { return hide(&mut signs) };
    if cursor.grab_mode == CursorGrabMode::None {
        return hide(&mut signs);
    }
    let me = lp.state.pos;
    // esquina más cercana
    use arena_sim::city::town::{XS, ZS};
    let _ = c;
    let near = |list: &[arena_sim::city::town::St], v: f32| list.iter().min_by(|a, b| (a.c - v).abs().total_cmp(&(b.c - v).abs())).copied();
    let (Some(stx), Some(stz)) = (near(&XS, me.x), near(&ZS, me.z)) else { return hide(&mut signs) };
    let (sx, sz) = (stx.c, stz.c);
    if (me.x - sx).abs() > stx.w / 2.0 + 14.0 || (me.z - sz).abs() > stz.w / 2.0 + 14.0 {
        return hide(&mut signs);
    }
    let Ok((camera, ct)) = cam.single() else { return };
    let k = target.map_or(1.0, |t| win.width() / t.size.x.max(1) as f32);
    // cada cartel en la esquina de la vereda más cercana, a 3 m de altura
    let cx = sx + if me.x > sx { stx.w / 2.0 + 0.5 } else { -stx.w / 2.0 - 0.5 };
    let cz = sz + if me.z > sz { stz.w / 2.0 + 0.5 } else { -stz.w / 2.0 - 0.5 };
    let items = [(Vec3::new(cx, 3.2, cz), stx.name), (Vec3::new(cx, 2.7, cz), stz.name)];
    for (s, mut n, mut v) in &mut signs {
        let (p, _) = items[s.0];
        match camera.world_to_viewport(ct, p) {
            Ok(sp) if (ct.translation() - p).dot(ct.forward().as_vec3()) < 0.0 => {
                let sp = sp * k;
                n.left = px(sp.x);
                n.top = px(sp.y + s.0 as f32 * 4.0);
                *v = Visibility::Inherited;
            }
            _ => *v = Visibility::Hidden,
        }
    }
    for (t, mut tx) in &mut texts {
        let name = items[t.0].1.to_uppercase();
        if tx.0 != name {
            tx.0 = name;
        }
    }
}
