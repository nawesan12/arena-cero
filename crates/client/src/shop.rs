//! Tienda (nuevo): con B en un punto de reabastecimiento se abre la lista; 1-9 compra y B o Esc
//! cierra. La partida se pausa mientras está abierta.

use crate::game::{GameRes, UiFeed};
use crate::hud::Fonts;
use crate::player::LocalPlayer;
use crate::world_render::LevelRes;
use arena_sim::economy::{SHOP, ShopItem};
use bevy::prelude::*;

const INK: Color = Color::srgb(0.902, 0.929, 0.910);
const DIM: Color = Color::srgb(0.498, 0.549, 0.529);
const AMBER: Color = Color::srgb(1.0, 0.690, 0.180);
const MINT: Color = Color::srgb(0.365, 1.0, 0.690);

pub struct ShopPlugin;

impl Plugin for ShopPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ShopOpen>().add_systems(PostStartup, spawn).add_systems(Update, (input, show));
    }
}

#[derive(Resource, Default)]
pub struct ShopOpen(pub bool);

#[derive(Component)]
struct Root;
#[derive(Component)]
struct Row(usize);
#[derive(Component)]
struct Credits;

fn spawn(mut commands: Commands, f: Res<Fonts>) {
    let t = |s: &str, size: f32, c: Color, font: &Handle<Font>| (Text::new(s), TextFont { font: FontSource::Handle(font.clone()), font_size: size.into(), ..default() }, TextColor(c));
    commands
        .spawn((Node { position_type: PositionType::Absolute, width: percent(100), height: percent(100), justify_content: JustifyContent::Center, align_items: AlignItems::Center, ..default() }, BackgroundColor(Color::srgba(0.02, 0.03, 0.035, 0.8)), GlobalZIndex(9), Visibility::Hidden, Root))
        .with_children(|p| {
            p.spawn(Node { flex_direction: FlexDirection::Column, row_gap: px(10), width: px(620), ..default() }).with_children(|c| {
                c.spawn(Node { flex_direction: FlexDirection::Row, justify_content: JustifyContent::SpaceBetween, align_items: AlignItems::Baseline, ..default() }).with_children(|h| {
                    h.spawn(t("TIENDA", 48.0, INK, &f.display));
                    h.spawn((t("", 22.0, AMBER, &f.mono), Credits));
                });
                c.spawn(t("1-9 COMPRAR · B CERRAR", 12.0, DIM, &f.display));
                for (i, item) in SHOP.iter().enumerate() {
                    c.spawn((Node { flex_direction: FlexDirection::Row, column_gap: px(14), align_items: AlignItems::Center, padding: UiRect::axes(px(12), px(7)), border: UiRect::all(px(1)), ..default() }, BorderColor::all(INK.with_alpha(0.15)), BackgroundColor(Color::srgba(0.063, 0.086, 0.098, 0.9)), Row(i))).with_children(|r| {
                        r.spawn(t(&format!("{}", i + 1), 14.0, AMBER, &f.mono));
                        r.spawn(Node { flex_direction: FlexDirection::Column, flex_grow: 1.0, ..default() }).with_children(|d| {
                            d.spawn(t(&item.name().to_uppercase(), 17.0, INK, &f.display));
                            d.spawn(t(item.desc(), 12.0, DIM, &f.display));
                        });
                        r.spawn(t(&format!("¢ {}", item.price()), 16.0, AMBER, &f.mono));
                    });
                }
            });
        });
}

/// Aplica en el jugador lo que compró (lo de la partida ya lo hizo `Game::buy`).
fn apply(item: ShopItem, lp: &mut LocalPlayer) {
    let a = &mut lp.arsenal;
    match item {
        ShopItem::Medkit => lp.hp = lp.max_hp,
        ShopItem::Nades => a.nades = (a.nades + 2).min(a.max_nades),
        ShopItem::Damage => a.dmg_mul += 0.15,
        ShopItem::Mag => {
            a.mag_mul += 0.25;
            a.ammo = [0, 1, 2, 3].map(|i| a.mag_of(i));
        }
        ShopItem::Armor => {
            lp.max_hp += 20.0;
            lp.hp += 20.0;
        }
        ShopItem::Launcher => a.unlocked[3] = true,
        _ => {}
    }
}

#[allow(clippy::too_many_arguments)]
fn input(key: Res<ButtonInput<KeyCode>>, mut open: ResMut<ShopOpen>, mut game: ResMut<GameRes>, level: Res<LevelRes>, mut lp: ResMut<LocalPlayer>, mut ui: ResMut<UiFeed>, perks: Res<crate::perks::PerkState>) {
    if key.just_pressed(KeyCode::KeyB) && !perks.active() {
        if open.0 {
            open.0 = false;
        } else if game.0.driving.is_none() && game.0.shop_near(&level.0, lp.state.pos) {
            open.0 = true;
        }
    }
    if !open.0 {
        return;
    }
    if key.just_pressed(KeyCode::Escape) {
        open.0 = false;
        return;
    }
    let keys = [KeyCode::Digit1, KeyCode::Digit2, KeyCode::Digit3, KeyCode::Digit4, KeyCode::Digit5, KeyCode::Digit6, KeyCode::Digit7, KeyCode::Digit8, KeyCode::Digit9];
    for (i, k) in keys.into_iter().enumerate() {
        if !key.just_pressed(k) {
            continue;
        }
        let item = SHOP[i];
        if item == ShopItem::Launcher && lp.arsenal.unlocked[3] {
            ui.banner("YA LO TENÉS", "El lanzagranadas ya está desbloqueado");
            continue;
        }
        if game.0.buy(item) {
            apply(item, &mut lp);
            ui.banner(item.name().to_uppercase(), format!("Te quedan ¢ {}", game.0.credits));
        } else {
            ui.banner("NO TE ALCANZA", format!("Cuesta ¢ {}", item.price()));
        }
    }
}

fn show(open: Res<ShopOpen>, game: Res<GameRes>, mut root: Query<&mut Visibility, With<Root>>, mut credits: Query<&mut Text, With<Credits>>, mut rows: Query<(&Row, &mut BorderColor)>) {
    for mut v in &mut root {
        *v = if open.0 { Visibility::Inherited } else { Visibility::Hidden };
    }
    if !open.0 {
        return;
    }
    let c = game.0.credits;
    for mut t in &mut credits {
        let v = format!("¢ {c}");
        if **t != v {
            **t = v;
        }
    }
    for (r, mut b) in &mut rows {
        *b = BorderColor::all(if SHOP[r.0].price() <= c { MINT.with_alpha(0.5) } else { INK.with_alpha(0.12) });
    }
}
