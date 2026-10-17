//! Campaña en el cliente (nuevo): menú principal, panel del objetivo, radio de Vera, resumen de
//! la operación con rango S/A/B/C y progreso guardado (`campana` y `rangos` en `~/.arena-cero`).
//! La lógica de las operaciones vive en `arena_sim::campaign`; acá solo se muestra y se elige.

use crate::game::{GameRes, UiFeed};
use crate::hud::Fonts;
use crate::perks::PerkState;
use crate::player::{LocalPlayer, Look};
use crate::world_render::LevelRes;
use arena_sim::campaign::{N_OPS, OP_NAMES, Rank};
use arena_sim::game::Game;
use arena_sim::player::EYE;
use arena_sim::weapons::Arsenal;
use arena_sim::{MapId, PlayerState, build_level};
use bevy::prelude::*;
use bevy::ui::RelativeCursorPosition;
use bevy::window::{CursorGrabMode, CursorOptions};

const INK: Color = Color::srgb(0.902, 0.929, 0.910);
const DIM: Color = Color::srgb(0.498, 0.549, 0.529);
const AMBER: Color = Color::srgb(1.0, 0.690, 0.180);
const CYAN: Color = Color::srgb(0.373, 0.847, 1.0);
const GOLD: Color = Color::srgb(1.0, 0.84, 0.3);
const RED: Color = Color::srgb(1.0, 0.239, 0.353);
const MINT: Color = Color::srgb(0.365, 1.0, 0.690);
const ROWS: usize = 10;

pub struct CampaignUiPlugin;

impl Plugin for CampaignUiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MenuOpen>()
            .init_resource::<Menu>()
            .init_resource::<RadioShow>()
            .insert_resource(Progress::load())
            .add_systems(PostStartup, (spawn, first_screen).chain())
            .add_systems(Update, (menu_input, menu_draw, debrief, objective_panel, radio_panel).chain());
    }
}

/// Menú o resumen en pantalla: la partida no avanza y el mouse queda libre.
#[derive(Resource, Default)]
pub struct MenuOpen(pub bool);

/// Cómo arranca el juego (sale de la línea de comandos).
#[derive(Resource, Clone, Copy, PartialEq, Eq, Debug)]
pub enum StartMode {
    Menu,
    Op(usize),
    Free,
    /// Oleadas o captura: directo a jugar.
    Direct,
}

#[derive(Clone, Copy, PartialEq, Eq, Default)]
enum Page {
    #[default]
    Main,
    Ops,
    Waves,
}

#[derive(Resource, Default)]
struct Menu {
    page: Page,
    sel: usize,
    /// Ya hay una partida empezada (se puede volver).
    playing: bool,
    /// Resumen ya guardado para esta generación de partida.
    saved_gen: Option<u32>,
}

#[derive(Resource)]
struct Progress {
    /// Operaciones desbloqueadas (la próxima a jugar es `unlocked`).
    unlocked: usize,
    best: [Option<Rank>; N_OPS],
}

impl Progress {
    fn load() -> Self {
        let unlocked = crate::config::get("campana").and_then(|v| v.parse().ok()).unwrap_or(0usize).min(N_OPS);
        let mut best = [None; N_OPS];
        if let Some(r) = crate::config::get("rangos") {
            for (i, ch) in r.chars().take(N_OPS).enumerate() {
                best[i] = Rank::from_letter(&ch.to_string());
            }
        }
        Self { unlocked, best }
    }
    fn save(&self) {
        crate::config::set("campana", self.unlocked);
        crate::config::set("rangos", self.best.iter().map(|r| r.map_or("-", |r| r.letter())).collect::<String>());
    }
}

/// Línea de radio en pantalla: quién, texto y cuánto lleva.
#[derive(Resource, Default)]
struct RadioShow {
    cur: Option<(&'static str, String, f32)>,
}

#[derive(Component)]
struct MenuRoot;
#[derive(Component)]
struct MenuTitle;
#[derive(Component)]
struct MenuSub;
#[derive(Component)]
struct MenuRow(usize);
#[derive(Component)]
struct MenuRowText(usize);
#[derive(Component)]
struct MenuRowSub(usize);
#[derive(Component)]
struct DebriefRoot;
#[derive(Component)]
struct DebriefRank;
#[derive(Component)]
struct DebriefTitle;
#[derive(Component)]
struct DebriefBody;
#[derive(Component)]
struct DebriefHint;
#[derive(Component)]
struct ObjRoot;
#[derive(Component)]
struct ObjText;
#[derive(Component)]
struct ObjTimer;
#[derive(Component)]
struct ObjBar;
#[derive(Component)]
struct ObjBarFill;
#[derive(Component)]
struct RadioRoot;
#[derive(Component)]
struct RadioWho;
#[derive(Component)]
struct RadioText;

fn txt(s: &str, size: f32, c: Color, font: &Handle<Font>) -> impl Bundle {
    (Text::new(s), TextFont { font: FontSource::Handle(font.clone()), font_size: size.into(), ..default() }, TextColor(c))
}

fn spawn(mut commands: Commands, f: Res<Fonts>) {
    let full = Node { position_type: PositionType::Absolute, width: percent(100), height: percent(100), justify_content: JustifyContent::Center, align_items: AlignItems::Center, ..default() };
    // menú principal
    commands.spawn((full.clone(), BackgroundColor(Color::srgba(0.02, 0.03, 0.035, 0.86)), GlobalZIndex(20), Visibility::Hidden, MenuRoot)).with_children(|p| {
        p.spawn(Node { flex_direction: FlexDirection::Column, row_gap: px(8), width: px(620), ..default() }).with_children(|c| {
            c.spawn(Node { flex_direction: FlexDirection::Row, column_gap: px(14), align_items: AlignItems::Baseline, ..default() }).with_children(|t| {
                t.spawn(txt("ARENA", 84.0, INK, &f.display));
                t.spawn(txt("CERO", 84.0, AMBER, &f.display));
            });
            c.spawn((txt("", 22.0, CYAN, &f.display), MenuTitle));
            c.spawn((txt("", 13.0, DIM, &f.display), MenuSub));
            c.spawn(Node { height: px(8), ..default() });
            for i in 0..ROWS {
                c.spawn((
                    Node { flex_direction: FlexDirection::Row, justify_content: JustifyContent::SpaceBetween, align_items: AlignItems::Center, padding: UiRect::axes(px(14), px(8)), border: UiRect::left(px(3)), ..default() },
                    BorderColor::all(Color::NONE),
                    BackgroundColor(Color::srgba(0.063, 0.086, 0.098, 0.6)),
                    RelativeCursorPosition::default(),
                    MenuRow(i),
                ))
                .with_children(|r| {
                    r.spawn((txt("", 22.0, INK, &f.display), MenuRowText(i)));
                    r.spawn((txt("", 14.0, DIM, &f.mono), MenuRowSub(i)));
                });
            }
            c.spawn(txt("FLECHAS O CLIC · ENTER ELIGE · ESC VUELVE", 12.0, DIM, &f.mono));
        });
    });
    // resumen de la operación
    commands.spawn((full, BackgroundColor(Color::srgba(0.02, 0.03, 0.035, 0.82)), GlobalZIndex(19), Visibility::Hidden, DebriefRoot)).with_children(|p| {
        p.spawn(Node { flex_direction: FlexDirection::Row, column_gap: px(40), align_items: AlignItems::Center, ..default() }).with_children(|r| {
            r.spawn((txt("S", 220.0, GOLD, &f.display), DebriefRank));
            r.spawn(Node { flex_direction: FlexDirection::Column, row_gap: px(10), width: px(460), ..default() }).with_children(|c| {
                c.spawn((txt("", 34.0, INK, &f.display), DebriefTitle));
                c.spawn((txt("", 17.0, INK, &f.mono), DebriefBody));
                c.spawn((txt("", 15.0, AMBER, &f.display), DebriefHint));
            });
        });
    });
    // objetivo actual (arriba a la izquierda, debajo del reloj)
    commands
        .spawn((Node { position_type: PositionType::Absolute, left: px(18), top: px(96), flex_direction: FlexDirection::Column, row_gap: px(4), padding: UiRect::axes(px(12), px(8)), border: UiRect::left(px(3)), max_width: px(420), ..default() }, BorderColor::all(GOLD), BackgroundColor(Color::srgba(0.04, 0.06, 0.07, 0.55)), Visibility::Hidden, ObjRoot))
        .with_children(|p| {
            p.spawn(txt("OBJETIVO", 11.0, GOLD, &f.mono));
            p.spawn((txt("", 19.0, INK, &f.display), TextShadow::default(), ObjText));
            p.spawn((txt("", 14.0, RED, &f.mono), ObjTimer));
            p.spawn((Node { width: px(200), height: px(3), ..default() }, BackgroundColor(Color::WHITE.with_alpha(0.12)), ObjBar)).with_children(|b| {
                b.spawn((Node { width: percent(0), height: percent(100), ..default() }, BackgroundColor(GOLD), ObjBarFill));
            });
        });
    // radio
    commands
        .spawn((Node { position_type: PositionType::Absolute, left: px(18), top: px(196), flex_direction: FlexDirection::Column, row_gap: px(3), padding: UiRect::axes(px(12), px(8)), border: UiRect::left(px(3)), max_width: px(440), ..default() }, BorderColor::all(CYAN), BackgroundColor(Color::srgba(0.03, 0.06, 0.08, 0.7)), Visibility::Hidden, RadioRoot))
        .with_children(|p| {
            p.spawn((txt("", 12.0, CYAN, &f.mono), RadioWho));
            p.spawn((txt("", 16.0, INK, &f.display), RadioText));
        });
}

fn first_screen(mode: Res<StartMode>, mut open: ResMut<MenuOpen>, mut menu: ResMut<Menu>, mut p: Params) {
    match *mode {
        StartMode::Menu => {
            open.0 = true;
            menu.page = Page::Main;
        }
        StartMode::Op(n) => {
            start_op(&mut p, n);
            menu.playing = true;
        }
        StartMode::Free | StartMode::Direct => menu.playing = true,
    }
}

/// Lo que hace falta para (re)empezar una partida.
#[derive(bevy::ecs::system::SystemParam)]
struct Params<'w> {
    game: ResMut<'w, GameRes>,
    level: ResMut<'w, LevelRes>,
    lp: ResMut<'w, LocalPlayer>,
    look: ResMut<'w, Look>,
    ui: ResMut<'w, UiFeed>,
    perks: ResMut<'w, PerkState>,
}

fn reset_player(p: &mut Params, at: Vec3, yaw: f32) {
    let city = p.level.0.info.city;
    let lp = &mut *p.lp;
    let feet = p.level.0.world.ground_at(at.x, at.z, at.y + 1.0);
    lp.state = PlayerState::new(at.x, at.z);
    lp.state.pos = Vec3::new(at.x, feet + EYE, at.z);
    lp.state.yaw = yaw;
    lp.prev = lp.state.pos;
    lp.start = lp.state.pos;
    lp.arsenal = Arsenal::new(city);
    lp.max_hp = 100.0;
    lp.hp = 100.0;
    lp.regen_mul = 1.0;
    lp.leech = 0.0;
    lp.hurt_flash = 0.0;
    lp.dmg_t = 0.0;
    p.look.yaw = yaw;
    p.look.pitch = 0.0;
    p.perks.taken.clear();
    p.perks.choices.clear();
    p.ui.radio.clear();
    p.ui.banner_queue.clear();
    p.ui.banner = None;
}

/// Ciudad nueva (los colisionadores vuelven a estar como al principio) y partida nueva.
fn fresh_city(p: &mut Params, make: impl FnOnce(&mut arena_sim::Level) -> Game) {
    p.game.0.teardown(&mut p.level.0.world);
    p.level.0 = build_level(MapId::Ciudad);
    p.game.0 = make(&mut p.level.0);
    p.game.1 += 1;
}

fn start_op(p: &mut Params, op: usize) {
    fresh_city(p, |l| Game::new_campaign(l, op));
    if let Some((at, yaw)) = p.game.0.campaign_start() {
        reset_player(p, at, yaw);
    }
}

fn start_free(p: &mut Params) {
    fresh_city(p, |l| Game::new(l, 7));
    let s = p.level.0.info.start;
    reset_player(p, Vec3::new(s[0], 0.0, s[1]), 0.0);
}

/// Vuelve a abrir el juego con otros argumentos (para cambiar de mapa).
fn relaunch(args: &[&str], exit: &mut MessageWriter<AppExit>) {
    if let Ok(exe) = std::env::current_exe() {
        let _ = std::process::Command::new(exe).args(args).spawn();
    }
    exit.write(AppExit::Success);
}

#[derive(Clone, Copy, PartialEq)]
enum Act {
    Continue,
    OpsPage,
    Free,
    WavesPage,
    Resume,
    Quit,
    Op(usize),
    Waves(&'static str),
    Back,
    None,
}

fn rows(menu: &Menu, prog: &Progress) -> Vec<(String, String, bool, Act)> {
    let mut v = Vec::new();
    match menu.page {
        Page::Main => {
            if menu.playing {
                v.push(("VOLVER A LA PARTIDA".into(), "ESC".into(), true, Act::Resume));
            }
            if prog.unlocked < N_OPS {
                let n = prog.unlocked;
                v.push((if n == 0 { "EMPEZAR LA CAMPAÑA".into() } else { "CONTINUAR LA CAMPAÑA".into() }, format!("OP {} · {}", n + 1, OP_NAMES[n].to_uppercase()), true, Act::Continue));
            } else {
                v.push(("CAMPAÑA COMPLETA".into(), "ELEGÍ UNA OPERACIÓN".into(), true, Act::OpsPage));
            }
            v.push(("ELEGIR OPERACIÓN".into(), format!("{}/{} DESBLOQUEADAS", (prog.unlocked + 1).min(N_OPS), N_OPS), true, Act::OpsPage));
            v.push(("MUNDO LIBRE".into(), "DISTRITO SUR SIN GUION".into(), true, Act::Free));
            v.push(("OLEADAS".into(), "ARENAS CERRADAS".into(), true, Act::WavesPage));
            v.push(("SALIR".into(), String::new(), true, Act::Quit));
        }
        Page::Ops => {
            for i in 0..N_OPS {
                let open = i <= prog.unlocked;
                let sub = match (open, prog.best[i]) {
                    (false, _) => "BLOQUEADA".to_string(),
                    (true, Some(r)) => format!("MEJOR RANGO {}", r.letter()),
                    (true, None) => "SIN JUGAR".to_string(),
                };
                v.push((format!("{} · {}", i + 1, OP_NAMES[i].to_uppercase()), sub, open, Act::Op(i)));
            }
            v.push(("VOLVER".into(), "ESC".into(), true, Act::Back));
        }
        Page::Waves => {
            for (id, name) in [("deposito", "DEPÓSITO"), ("fundicion", "FUNDICIÓN"), ("cantera", "CANTERA"), ("azotea", "AZOTEA")] {
                v.push((name.into(), "OLEADAS · REINICIA EL JUEGO".into(), true, Act::Waves(id)));
            }
            v.push(("VOLVER".into(), "ESC".into(), true, Act::Back));
        }
    }
    v
}

#[allow(clippy::too_many_arguments)]
fn menu_input(
    key: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut open: ResMut<MenuOpen>,
    mut menu: ResMut<Menu>,
    prog: Res<Progress>,
    mut cursor: Single<&mut CursorOptions>,
    hover: Query<(&MenuRow, &RelativeCursorPosition)>,
    mut exit: MessageWriter<AppExit>,
    mut snd: ResMut<crate::audio::Sounds>,
    mut p: Params,
) {
    let debrief = p.game.0.campaign.as_ref().is_some_and(|c| c.result.is_some());
    // en pausa, Enter abre el menú
    if !open.0 {
        if cursor.grab_mode == CursorGrabMode::None && !debrief && key.just_pressed(KeyCode::Enter) {
            open.0 = true;
            menu.page = Page::Main;
            menu.sel = 0;
        }
        return;
    }
    if debrief {
        return;
    }
    cursor.visible = true;
    cursor.grab_mode = CursorGrabMode::None;
    let list = rows(&menu, &prog);
    let n = list.len();
    if key.just_pressed(KeyCode::ArrowDown) || key.just_pressed(KeyCode::KeyS) {
        menu.sel = (menu.sel + 1) % n;
        snd.play(crate::audio::S::Swap);
    }
    if key.just_pressed(KeyCode::ArrowUp) || key.just_pressed(KeyCode::KeyW) {
        menu.sel = (menu.sel + n - 1) % n;
        snd.play(crate::audio::S::Swap);
    }
    let mut act = Act::None;
    for (r, rc) in &hover {
        if rc.cursor_over() && r.0 < n {
            menu.sel = r.0;
            if mouse.just_pressed(MouseButton::Left) && list[r.0].2 {
                act = list[r.0].3;
            }
        }
    }
    if (key.just_pressed(KeyCode::Enter) || key.just_pressed(KeyCode::Space)) && list.get(menu.sel).is_some_and(|r| r.2) {
        act = list[menu.sel].3;
    }
    if key.just_pressed(KeyCode::Escape) {
        act = if menu.page == Page::Main { if menu.playing { Act::Resume } else { Act::None } } else { Act::Back };
    }
    let city = p.level.0.info.city;
    let play = |open: &mut MenuOpen, menu: &mut Menu| {
        open.0 = false;
        menu.playing = true;
        menu.page = Page::Main;
        menu.sel = 0;
    };
    match act {
        Act::None => return,
        Act::Continue | Act::Op(_) => {
            let op = if let Act::Op(i) = act { i } else { prog.unlocked.min(N_OPS - 1) };
            if !city {
                relaunch(&["--op", &(op + 1).to_string()], &mut exit);
                return;
            }
            start_op(&mut p, op);
            play(&mut open, &mut menu);
        }
        Act::Free => {
            if !city {
                relaunch(&["--libre"], &mut exit);
                return;
            }
            start_free(&mut p);
            play(&mut open, &mut menu);
        }
        Act::Resume => play(&mut open, &mut menu),
        Act::OpsPage => {
            menu.page = Page::Ops;
            menu.sel = prog.unlocked.min(N_OPS - 1);
        }
        Act::WavesPage => {
            menu.page = Page::Waves;
            menu.sel = 0;
        }
        Act::Back => {
            menu.page = Page::Main;
            menu.sel = 0;
        }
        Act::Waves(m) => relaunch(&["--map", m], &mut exit),
        Act::Quit => {
            exit.write(AppExit::Success);
        }
    }
    snd.play(crate::audio::S::Pickup);
}

#[allow(clippy::type_complexity)]
fn menu_draw(
    open: Res<MenuOpen>,
    menu: Res<Menu>,
    prog: Res<Progress>,
    game: Res<GameRes>,
    mut root: Query<&mut Visibility, With<MenuRoot>>,
    mut rows_q: Query<(&MenuRow, &mut Visibility, &mut BorderColor, &mut BackgroundColor), Without<MenuRoot>>,
    mut texts: Query<(&MenuRowText, &mut Text, &mut TextColor), (Without<MenuRowSub>, Without<MenuTitle>, Without<MenuSub>)>,
    mut subs: Query<(&MenuRowSub, &mut Text), (Without<MenuRowText>, Without<MenuTitle>, Without<MenuSub>)>,
    mut title: Query<&mut Text, (With<MenuTitle>, Without<MenuRowText>, Without<MenuRowSub>, Without<MenuSub>)>,
    mut sub: Query<&mut Text, (With<MenuSub>, Without<MenuRowText>, Without<MenuRowSub>, Without<MenuTitle>)>,
) {
    let show = open.0 && !game.0.campaign.as_ref().is_some_and(|c| c.result.is_some());
    for mut v in &mut root {
        *v = if show { Visibility::Inherited } else { Visibility::Hidden };
    }
    if !show {
        return;
    }
    let list = rows(&menu, &prog);
    for mut t in &mut title {
        t.0 = match menu.page {
            Page::Main => "OPERACIONES EN EL DISTRITO SUR".into(),
            Page::Ops => "ELEGÍ LA OPERACIÓN".into(),
            Page::Waves => "OLEADAS · ELEGÍ LA ARENA".into(),
        };
    }
    for mut t in &mut sub {
        t.0 = match menu.page {
            Page::Main => "Ocho operaciones encadenadas. Vera te guía por radio. Cada una termina con un rango.".into(),
            Page::Ops => "Las operaciones anteriores cuentan como hechas: la ciudad queda como la dejaste.".into(),
            Page::Waves => "Drones, kamikazes, pesados y el Coloso cada cinco oleadas.".into(),
        };
    }
    for (r, mut v, mut b, mut bg) in &mut rows_q {
        match list.get(r.0) {
            Some(_) => {
                *v = Visibility::Inherited;
                let sel = r.0 == menu.sel;
                *b = BorderColor::all(if sel { AMBER } else { Color::NONE });
                bg.0 = if sel { Color::srgba(0.12, 0.14, 0.15, 0.85) } else { Color::srgba(0.063, 0.086, 0.098, 0.6) };
            }
            None => *v = Visibility::Hidden,
        }
    }
    for (r, mut t, mut c) in &mut texts {
        if let Some((label, _, on, _)) = list.get(r.0) {
            if t.0 != *label {
                t.0 = label.clone();
            }
            c.0 = if !on { DIM.with_alpha(0.5) } else if r.0 == menu.sel { AMBER } else { INK };
        }
    }
    for (r, mut t) in &mut subs {
        if let Some((_, s, _, _)) = list.get(r.0) {
            if t.0 != *s {
                t.0 = s.clone();
            }
        }
    }
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn debrief(
    key: Res<ButtonInput<KeyCode>>,
    mut open: ResMut<MenuOpen>,
    mut menu: ResMut<Menu>,
    mut prog: ResMut<Progress>,
    mut cursor: Single<&mut CursorOptions>,
    mut root: Query<&mut Visibility, With<DebriefRoot>>,
    mut rank_q: Query<(&mut Text, &mut TextColor), (With<DebriefRank>, Without<DebriefTitle>, Without<DebriefBody>, Without<DebriefHint>)>,
    mut title: Query<&mut Text, (With<DebriefTitle>, Without<DebriefRank>, Without<DebriefBody>, Without<DebriefHint>)>,
    mut body: Query<&mut Text, (With<DebriefBody>, Without<DebriefRank>, Without<DebriefTitle>, Without<DebriefHint>)>,
    mut hint: Query<&mut Text, (With<DebriefHint>, Without<DebriefRank>, Without<DebriefTitle>, Without<DebriefBody>)>,
    mut snd: ResMut<crate::audio::Sounds>,
    mut p: Params,
) {
    let res = p.game.0.campaign.as_ref().and_then(|c| c.result.clone());
    for mut v in &mut root {
        *v = if res.is_some() { Visibility::Inherited } else { Visibility::Hidden };
    }
    let Some(r) = res else { return };
    open.0 = true;
    cursor.visible = true;
    cursor.grab_mode = CursorGrabMode::None;
    let gen_ = p.game.1;
    if menu.saved_gen != Some(gen_) {
        menu.saved_gen = Some(gen_);
        prog.unlocked = prog.unlocked.max((r.op + 1).min(N_OPS));
        if prog.best[r.op].is_none_or(|b| r.rank < b) {
            prog.best[r.op] = Some(r.rank);
        }
        prog.save();
        snd.play(crate::audio::S::Perk);
    }
    let last = r.op + 1 >= N_OPS;
    for (mut t, mut c) in &mut rank_q {
        t.0 = r.rank.letter().into();
        c.0 = match r.rank {
            Rank::S => GOLD,
            Rank::A => MINT,
            Rank::B => CYAN,
            Rank::C => DIM,
        };
    }
    for mut t in &mut title {
        t.0 = format!("OPERACIÓN {} · {} CUMPLIDA", r.op + 1, OP_NAMES[r.op].to_uppercase());
    }
    let mmss = |s: f32| format!("{}:{:02}", (s / 60.0) as u32, (s % 60.0) as u32);
    for mut t in &mut body {
        t.0 = format!(
            "TIEMPO      {}  (REFERENCIA {})\nCAÍDAS      {}\nBAJAS       {}\nDERRIBOS    {}\nCRÍTICOS    {}\nSILENCIOSAS {}\n\nBONO RANGO  +{}\nPUNTAJE     {}",
            mmss(r.time),
            mmss(r.par),
            r.deaths,
            r.kills,
            r.takedowns,
            r.crits,
            r.stealth,
            r.bonus,
            r.score
        );
    }
    for mut t in &mut hint {
        t.0 = if last { "¡CAMPAÑA COMPLETA!   ENTER · MENÚ   R · REPETIR".into() } else { format!("ENTER · OPERACIÓN {}: {}   R · REPETIR   ESC · MENÚ", r.op + 2, OP_NAMES[r.op + 1].to_uppercase()) };
    }
    let go = |op: usize, p: &mut Params, open: &mut MenuOpen| {
        start_op(p, op);
        open.0 = false;
    };
    if key.just_pressed(KeyCode::KeyR) {
        go(r.op, &mut p, &mut open);
    } else if key.just_pressed(KeyCode::Enter) && !last {
        go(r.op + 1, &mut p, &mut open);
    } else if key.just_pressed(KeyCode::Escape) || (key.just_pressed(KeyCode::Enter) && last) {
        // al menú, con una ciudad libre de fondo
        start_free(&mut p);
        menu.page = Page::Main;
        menu.sel = 0;
        menu.playing = false;
    }
}

#[allow(clippy::type_complexity)]
fn objective_panel(
    game: Res<GameRes>,
    open: Res<MenuOpen>,
    mut root: Query<&mut Visibility, With<ObjRoot>>,
    mut text: Query<&mut Text, (With<ObjText>, Without<ObjTimer>)>,
    mut timer: Query<&mut Text, (With<ObjTimer>, Without<ObjText>)>,
    mut bar: Query<&mut Visibility, (With<ObjBar>, Without<ObjRoot>)>,
    mut fill: Query<&mut Node, With<ObjBarFill>>,
) {
    let cp = game.0.campaign.as_ref().filter(|c| c.result.is_none() && !open.0);
    for mut v in &mut root {
        *v = if cp.is_some() { Visibility::Inherited } else { Visibility::Hidden };
    }
    let Some(cp) = cp else { return };
    for mut t in &mut text {
        let s = cp.objective.to_uppercase();
        if t.0 != s {
            t.0 = s;
        }
    }
    for mut t in &mut timer {
        let s = cp.limit_left.map_or(String::new(), |l| format!("TIEMPO {:.1} S", l.max(0.0)));
        if t.0 != s {
            t.0 = s;
        }
    }
    for mut v in &mut bar {
        *v = if cp.progress.is_some() { Visibility::Inherited } else { Visibility::Hidden };
    }
    for mut n in &mut fill {
        n.width = percent(cp.progress.unwrap_or(0.0) * 100.0);
    }
}

/// La radio escribe letra por letra; cada línea queda en pantalla según su largo.
#[allow(clippy::type_complexity)]
fn radio_panel(
    time: Res<Time>,
    open: Res<MenuOpen>,
    mut ui: ResMut<UiFeed>,
    mut show: ResMut<RadioShow>,
    mut snd: ResMut<crate::audio::Sounds>,
    mut root: Query<&mut Visibility, With<RadioRoot>>,
    mut who_q: Query<(&mut Text, &mut TextColor), (With<RadioWho>, Without<RadioText>)>,
    mut text_q: Query<&mut Text, (With<RadioText>, Without<RadioWho>)>,
) {
    if open.0 {
        for mut v in &mut root {
            *v = Visibility::Hidden;
        }
        return;
    }
    let dt = time.delta_secs();
    if let Some(cur) = show.cur.as_mut() {
        cur.2 += dt;
        let dur = 2.2 + cur.1.chars().count() as f32 * 0.045;
        if cur.2 > dur {
            show.cur = None;
        }
    }
    if show.cur.is_none() {
        if let Some((who, text)) = ui.radio.pop_front() {
            show.cur = Some((who, text, 0.0));
            snd.play(crate::audio::S::Radio);
        }
    }
    for mut v in &mut root {
        *v = if show.cur.is_some() { Visibility::Inherited } else { Visibility::Hidden };
    }
    let Some((who, text, t)) = show.cur.as_ref() else { return };
    let shown: String = text.chars().take((t * 55.0) as usize).collect();
    for (mut w, mut c) in &mut who_q {
        let s = format!("> {who}");
        if w.0 != s {
            w.0 = s;
        }
        c.0 = if *who == arena_sim::campaign::VERA { CYAN } else { RED };
    }
    for mut tx in &mut text_q {
        if tx.0 != shown {
            tx.0 = shown.clone();
        }
    }
}
