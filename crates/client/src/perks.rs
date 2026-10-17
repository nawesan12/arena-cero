//! Elección de mejoras (puerto de `showPerks`/`pickPerk`): la partida se pausa, aparecen tres
//! tarjetas y se elige con 1, 2 o 3.

use crate::hud::Fonts;
use crate::player::LocalPlayer;
use arena_sim::perks::{self, Perk};
use bevy::prelude::*;

const INK: Color = Color::srgb(0.902, 0.929, 0.910);
const DIM: Color = Color::srgb(0.498, 0.549, 0.529);
const AMBER: Color = Color::srgb(1.0, 0.690, 0.180);

pub struct PerksPlugin;

impl Plugin for PerksPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PerkState>().add_systems(PostStartup, spawn).add_systems(Update, (pick, show));
    }
}

#[derive(Resource, Default)]
pub struct PerkState {
    pub choices: Vec<Perk>,
    pub taken: Vec<Perk>,
}

impl PerkState {
    pub fn active(&self) -> bool {
        !self.choices.is_empty()
    }
    pub fn offer(&mut self, rng: &mut arena_sim::rng::Rng) {
        self.choices = perks::offer(&self.taken, rng);
    }
}

/// Aplica una mejora al jugador (los mismos efectos que la web).
pub fn apply(p: Perk, lp: &mut LocalPlayer) {
    let a = &mut lp.arsenal;
    match p {
        Perk::Hp => {
            lp.max_hp += 25.0;
            lp.hp = lp.max_hp;
        }
        Perk::Dmg => a.dmg_mul += 0.25,
        Perk::Reload => a.reload_mul *= 0.7,
        Perk::Mag => {
            a.mag_mul += 0.5;
            a.ammo = [0, 1, 2, 3].map(|i| a.mag_of(i));
        }
        Perk::Speed => lp.state.speed_mul += 0.12,
        Perk::Regen => lp.regen_mul *= 2.0,
        Perk::Leech => lp.leech += 4.0,
        Perk::Nade => {
            a.max_nades += 1;
            a.nades = a.max_nades;
        }
        Perk::Jump => lp.state.max_jumps += 1,
        Perk::Dash => lp.state.dash_cd_mul *= 0.6,
        Perk::Crit => a.crit_mul = 3.0,
    }
}

#[derive(Component)]
struct Root;
#[derive(Component)]
struct Card(usize);
#[derive(Component)]
struct CardText(usize, u8);

fn spawn(mut commands: Commands, f: Res<Fonts>) {
    let t = |s: &str, size: f32, c: Color, font: &Handle<Font>| (Text::new(s), TextFont { font: FontSource::Handle(font.clone()), font_size: size.into(), ..default() }, TextColor(c));
    commands
        .spawn((Node { position_type: PositionType::Absolute, width: percent(100), height: percent(100), flex_direction: FlexDirection::Column, justify_content: JustifyContent::Center, align_items: AlignItems::Center, row_gap: px(16), ..default() }, BackgroundColor(Color::srgba(0.02, 0.03, 0.035, 0.74)), GlobalZIndex(9), Visibility::Hidden, Root))
        .with_children(|p| {
            p.spawn(t("ELEGÍ UNA MEJORA", 54.0, INK, &f.display));
            p.spawn(t("TECLAS 1 · 2 · 3", 12.0, DIM, &f.display));
            p.spawn(Node { flex_direction: FlexDirection::Row, column_gap: px(14), ..default() }).with_children(|r| {
                for i in 0..3 {
                    r.spawn((Node { width: px(230), min_height: px(150), padding: UiRect::all(px(16)), flex_direction: FlexDirection::Column, row_gap: px(8), border: UiRect::all(px(1)), border_radius: BorderRadius::new(px(0), px(14), px(0), px(0)), ..default() }, BorderColor::all(INK.with_alpha(0.18)), BackgroundColor(Color::srgba(0.063, 0.086, 0.098, 0.94)), Card(i))).with_children(|c| {
                        c.spawn((t("", 13.0, AMBER, &f.mono), CardText(i, 0)));
                        c.spawn((t("", 24.0, INK, &f.display), CardText(i, 1)));
                        c.spawn((t("", 14.0, DIM, &f.display), CardText(i, 2)));
                    });
                }
            });
        });
}

fn pick(key: Res<ButtonInput<KeyCode>>, mut st: ResMut<PerkState>, mut lp: ResMut<LocalPlayer>, mut ui: ResMut<crate::game::UiFeed>, mut snd: ResMut<crate::audio::Sounds>) {
    if !st.active() {
        return;
    }
    for (i, k) in [KeyCode::Digit1, KeyCode::Digit2, KeyCode::Digit3].into_iter().enumerate() {
        if key.just_pressed(k) {
            if let Some(&p) = st.choices.get(i) {
                apply(p, &mut lp);
                st.taken.push(p);
                st.choices.clear();
                ui.banner(p.name().to_uppercase(), p.desc());
                snd.play(crate::audio::S::Pickup);
            }
        }
    }
}

fn show(st: Res<PerkState>, mut root: Query<&mut Visibility, With<Root>>, mut texts: Query<(&CardText, &mut Text)>) {
    for mut v in &mut root {
        *v = if st.active() { Visibility::Inherited } else { Visibility::Hidden };
    }
    if !st.is_changed() {
        return;
    }
    for (ct, mut t) in &mut texts {
        let Some(&p) = st.choices.get(ct.0) else { continue };
        let lvl = st.taken.iter().filter(|&&x| x == p).count();
        let v = match ct.1 {
            0 => format!("{}{}", ct.0 + 1, if lvl > 0 { format!(" · NIVEL {}", lvl + 1) } else { String::new() }),
            1 => p.name().to_uppercase(),
            _ => p.desc().to_string(),
        };
        **t = v;
    }
}
