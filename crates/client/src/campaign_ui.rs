//! Campaña en el cliente (nuevo): menú principal, panel del objetivo, radio de Vera, resumen de
//! la operación con rango S/A/B/C y progreso guardado (`campana` y `rangos` en `~/.arena-cero`).
//! La lógica de las operaciones vive en `arena_sim::campaign`; acá solo se muestra y se elige.

use crate::online::{Online, Role};
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
const ROWS: usize = 12;

pub struct CampaignUiPlugin;

impl Plugin for CampaignUiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MenuOpen>()
            .init_resource::<Menu>()
            .init_resource::<RadioShow>()
            .init_resource::<Briefing>()
            .insert_resource(Progress::load())
            .add_systems(PostStartup, (spawn, first_screen).chain())
            .add_systems(Update, (online_glue, menu_input, menu_draw, debrief, briefing, objective_panel, radio_panel).chain());
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
    Options,
    Online,
}

/// Lo que se puede ajustar en la página de opciones.
#[derive(bevy::ecs::system::SystemParam)]
struct Opts<'w> {
    settings: ResMut<'w, crate::player::Settings>,
    music: ResMut<'w, crate::audio::MusicVol>,
    sfx: ResMut<'w, crate::audio::SfxVol>,
    quality: ResMut<'w, crate::quality::QualityRes>,
    bloom: ResMut<'w, crate::quality::BloomOn>,
    fps: ResMut<'w, crate::hud::ShowFps>,
}

/// Valores de las opciones para dibujar las filas.
struct OptVals {
    sens: f32,
    music: f32,
    sfx: f32,
    quality: &'static str,
    bloom: bool,
    fps: bool,
}

impl Opts<'_> {
    fn vals(&self) -> OptVals {
        OptVals { sens: self.settings.sens, music: self.music.0, sfx: self.sfx.0, quality: self.quality.label(), bloom: self.bloom.0, fps: self.fps.0 }
    }
    /// Mueve una opción para un lado (−1 / +1) y la guarda.
    fn change(&mut self, k: u8, dir: f32) {
        use crate::quality::Quality;
        match k {
            0 => {
                self.settings.sens = ((self.settings.sens + dir * 0.1) * 10.0).round() / 10.0;
                self.settings.sens = self.settings.sens.clamp(0.3, 6.0);
                crate::config::set("sens", self.settings.sens);
            }
            1 => {
                self.music.0 = ((self.music.0 + dir * 0.1) * 10.0).round().clamp(0.0, 10.0) / 10.0;
                crate::config::set("musica", self.music.0);
            }
            2 => {
                self.sfx.0 = ((self.sfx.0 + dir * 0.1) * 10.0).round().clamp(0.0, 10.0) / 10.0;
                crate::config::set("efectos", self.sfx.0);
            }
            3 => {
                let all = [Quality::Baja, Quality::Media, Quality::Alta];
                let i = all.iter().position(|q| *q == self.quality.q).unwrap_or(1) as i32;
                let n = (i + dir as i32).rem_euclid(3) as usize;
                self.quality.set(all[n]);
            }
            4 => {
                self.bloom.0 = !self.bloom.0;
                crate::config::set("brillo", if self.bloom.0 { "1" } else { "0" });
            }
            _ => {
                self.fps.0 = !self.fps.0;
                crate::config::set("fps", if self.fps.0 { "1" } else { "0" });
            }
        }
    }
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
/// Retrato de quién habla: 0 Vera, 1 radio enemiga, 2 créditos.
#[derive(Component)]
struct RadioFace(u8);

fn txt(s: &str, size: f32, c: Color, font: &Handle<Font>) -> impl Bundle {
    (Text::new(s), TextFont { font: FontSource::Handle(font.clone()), font_size: size.into(), ..default() }, TextColor(c))
}

fn spawn(mut commands: Commands, f: Res<Fonts>, level: Res<LevelRes>, mut images: ResMut<Assets<Image>>) {
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
    // briefing: título, lugar, texto de Vera y mapa con los lugares de la operación
    let map = images.add(crate::tacmap::plan(&level.0));
    commands.spawn((Node { position_type: PositionType::Absolute, width: percent(100), height: percent(100), justify_content: JustifyContent::Center, align_items: AlignItems::Center, column_gap: px(36), ..default() }, BackgroundColor(Color::srgba(0.02, 0.03, 0.035, 0.92)), GlobalZIndex(21), Visibility::Hidden, BriefRoot)).with_children(|p| {
        p.spawn(Node { flex_direction: FlexDirection::Column, row_gap: px(12), width: px(440), ..default() }).with_children(|c| {
            c.spawn(txt("BRIEFING", 14.0, CYAN, &f.mono));
            c.spawn((txt("", 46.0, INK, &f.display), BriefTitle));
            c.spawn((txt("", 15.0, AMBER, &f.display), BriefPlace));
            c.spawn(Node { flex_direction: FlexDirection::Row, column_gap: px(12), align_items: AlignItems::Start, ..default() }).with_children(|r| {
                r.spawn((Node { width: px(64), height: px(64), ..default() }, ImageNode::new(images.add(crate::textures::portrait(0)))));
                r.spawn(Node { flex_direction: FlexDirection::Column, row_gap: px(4), width: px(360), ..default() }).with_children(|t| {
                    t.spawn(txt("> VERA", 12.0, CYAN, &f.mono));
                    t.spawn((txt("", 17.0, INK, &f.display), BriefText));
                });
            });
            c.spawn(txt("ENTER · EMPEZAR LA OPERACIÓN", 15.0, AMBER, &f.display));
        });
        p.spawn((Node { width: px(420), height: px(420), ..default() }, ImageNode::new(map), BriefMap)).with_children(|m| {
            for i in 0..12 {
                m.spawn((Node { position_type: PositionType::Absolute, width: px(12), height: px(12), margin: UiRect { left: px(-6), top: px(-6), ..default() }, border_radius: BorderRadius::MAX, ..default() }, BackgroundColor(GOLD), Visibility::Hidden, BriefDot(i)));
            }
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
            p.spawn(Node { flex_direction: FlexDirection::Row, column_gap: px(10), align_items: AlignItems::Start, ..default() }).with_children(|r| {
                r.spawn((Node { width: px(48), height: px(48), ..default() }, ImageNode::new(images.add(crate::textures::portrait(0))), RadioFace(0)));
                r.spawn((Node { width: px(48), height: px(48), display: Display::None, ..default() }, ImageNode::new(images.add(crate::textures::portrait(1))), RadioFace(1)));
                r.spawn((Node { width: px(48), height: px(48), display: Display::None, ..default() }, ImageNode::new(images.add(crate::textures::portrait(2))), RadioFace(2)));
                r.spawn(Node { flex_direction: FlexDirection::Column, row_gap: px(3), ..default() }).with_children(|t| {
                    t.spawn((txt("", 12.0, CYAN, &f.mono), RadioWho));
                    t.spawn((txt("", 16.0, INK, &f.display), RadioText));
                });
            });
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

/// Online, cada cuadro: arrancar con `--host`/`--join`, armar la partida que manda el servidor,
/// y volver al menú si se corta.
#[allow(clippy::too_many_arguments)]
fn online_glue(args: Res<crate::Args>, mut net: ResMut<Online>, mut open: ResMut<MenuOpen>, mut menu: ResMut<Menu>, mut exit: MessageWriter<AppExit>, mut p: Params, mut started: Local<bool>) {
    if !*started {
        *started = true;
        net.name = crate::config::get("nombre").unwrap_or_default();
        if args.host {
            net.host(&p.game.0, &p.level.0);
            p.ui.banner("PARTIDA ABIERTA", net.status.clone());
        } else if let Some(a) = args.join {
            net.join(a);
            open.0 = true;
            menu.page = Page::Online;
        }
    }
    let Role::Client(r) = &mut net.role else { return };
    if r.net.is_disconnected() {
        let was = r.ready;
        net.stop();
        net.status = if was { "Se cortó la conexión con la partida".into() } else { "No se pudo conectar (¿IP y puerto bien? ¿el anfitrión abrió la partida?)".into() };
        p.ui.banner("SIN CONEXIÓN", net.status.clone());
        // la partida que quedó era una copia: vuelve el mundo libre de fondo
        if p.level.0.info.city {
            start_free(&mut p);
        }
        open.0 = true;
        menu.playing = false;
        menu.page = Page::Online;
        menu.sel = 0;
        return;
    }
    let Some(mode) = r.net.new_game.take().or(if r.ready { None } else { r.net.mode }) else { return };
    let map = mode.map();
    if map != p.level.0.info.id {
        // otro mapa: se reabre el juego en ese mapa y se vuelve a conectar solo
        let addr = r.addr.to_string();
        r.net.disconnect();
        relaunch(&["--map", map_arg(map), "--join", &addr], &mut exit);
        return;
    }
    r.ready = true;
    p.game.0.teardown(&mut p.level.0.world);
    p.level.0 = build_level(map);
    p.game.0 = mode.new_game(&mut p.level.0);
    p.game.1 += 1;
    let (at, yaw) = crate::online::join_spot(&p.game.0, &p.level.0);
    reset_player(&mut p, at, yaw);
    p.ui.banner("EN LA PARTIDA", mode.label());
    open.0 = false;
    menu.playing = true;
    menu.page = Page::Main;
    menu.sel = 0;
}

/// Briefing de la operación que está por empezar (la partida espera hasta Enter).
#[derive(Resource, Default)]
struct Briefing(Option<usize>);

#[derive(Component)]
struct BriefRoot;
#[derive(Component)]
struct BriefTitle;
#[derive(Component)]
struct BriefPlace;
#[derive(Component)]
struct BriefText;
#[derive(Component)]
struct BriefMap;
#[derive(Component)]
struct BriefDot(usize);

/// Lo que hace falta para (re)empezar una partida.
#[derive(bevy::ecs::system::SystemParam)]
struct Params<'w> {
    brief: ResMut<'w, Briefing>,
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
    p.brief.0 = Some(op);
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
    OptionsPage,
    /// Opción `k` (se ajusta con ← →; Enter o clic la sube).
    Opt(u8),
    /// Volver a empezar la operación en curso.
    Restart(usize),
    OnlinePage,
    /// Abrir online: 0 la partida actual, 1 campaña, 2 mundo libre, 3 oleadas.
    Host(u8),
    JoinLan(usize),
    /// Campo de texto: 0 nombre, 1 IP (Enter se une).
    Text(u8),
    StopOnline,
    None,
}

fn rows(menu: &Menu, prog: &Progress, o: &OptVals, camp: Option<usize>, net: &Online) -> Vec<(String, String, bool, Act)> {
    let mut v = Vec::new();
    match menu.page {
        Page::Online => online_rows(menu, prog, net, &mut v),
        Page::Main => {
            if menu.playing {
                v.push(("VOLVER A LA PARTIDA".into(), "ESC".into(), true, Act::Resume));
                if let Some(op) = camp {
                    v.push(("REINICIAR LA OPERACIÓN".into(), format!("OP {} · {}", op + 1, OP_NAMES[op].to_uppercase()), true, Act::Restart(op)));
                }
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
            let sub = match &net.role {
                Role::Off => "COOPERATIVO HASTA 4 · LAN O INTERNET".to_string(),
                Role::Host(h) => format!("PARTIDA ABIERTA · {} CONECTADOS", h.player_count()),
                Role::Client(r) => format!("CONECTADO A {}", r.addr),
            };
            v.push(("JUGAR ONLINE".into(), sub, true, Act::OnlinePage));
            v.push(("OPCIONES".into(), "MOUSE · SONIDO · GRÁFICOS".into(), true, Act::OptionsPage));
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
        Page::Options => {
            let pct = |x: f32| format!("< {:.0} % >", x * 100.0);
            let yes = |b: bool| if b { "< SÍ >".to_string() } else { "< NO >".to_string() };
            v.push(("SENSIBILIDAD DEL MOUSE".into(), format!("< {:.1} >", o.sens), true, Act::Opt(0)));
            v.push(("MÚSICA".into(), pct(o.music), true, Act::Opt(1)));
            v.push(("EFECTOS".into(), pct(o.sfx), true, Act::Opt(2)));
            v.push(("CALIDAD".into(), format!("< {} >", o.quality), true, Act::Opt(3)));
            v.push(("BRILLO (BLOOM)".into(), yes(o.bloom), true, Act::Opt(4)));
            v.push(("MOSTRAR FPS".into(), yes(o.fps), true, Act::Opt(5)));
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

fn online_rows(menu: &Menu, prog: &Progress, net: &Online, v: &mut Vec<(String, String, bool, Act)>) {
    let cursor = |on: bool| if on { "_" } else { "" };
    let sel_act = |k: u8| v.len() == menu.sel && k < 2;
    match &net.role {
        Role::Off => {
            let editing = sel_act(0);
            v.push(("TU NOMBRE".into(), format!("{}{}", net.name.to_uppercase(), cursor(editing)), true, Act::Text(0)));
            if menu.playing {
                v.push(("ABRIR ESTA PARTIDA".into(), "LOS DEMÁS SE SUMAN A LO QUE ESTÁS JUGANDO".into(), true, Act::Host(0)));
            }
            let n = prog.unlocked.min(N_OPS - 1);
            v.push(("CREAR · CAMPAÑA".into(), format!("OP {} · {}", n + 1, OP_NAMES[n].to_uppercase()), true, Act::Host(1)));
            v.push(("CREAR · MUNDO LIBRE".into(), "DISTRITO SUR".into(), true, Act::Host(2)));
            v.push(("CREAR · OLEADAS".into(), "DEPÓSITO · REINICIA EL JUEGO".into(), true, Act::Host(3)));
            let found = net.scanner.as_ref().map(|s| s.found.as_slice()).unwrap_or_default();
            if found.is_empty() {
                v.push(("BUSCANDO EN LA RED LOCAL…".into(), String::new(), false, Act::None));
            }
            for (i, f) in found.iter().enumerate().take(3) {
                v.push((format!("UNIRSE · {}", f.name.to_uppercase()), format!("{} · {}/4 · {}", f.mode.to_uppercase(), f.players, f.addr.ip()), true, Act::JoinLan(i)));
            }
            let editing = v.len() == menu.sel;
            v.push(("UNIRSE POR IP".into(), format!("{}{}", if net.ip.is_empty() && !editing { "ESCRIBÍ LA IP Y ENTER".to_string() } else { net.ip.clone() }, cursor(editing)), true, Act::Text(1)));
        }
        Role::Host(h) => {
            v.push(("PARTIDA ABIERTA".into(), net.status.to_uppercase(), false, Act::None));
            for (id, name) in h.names.iter().take(3) {
                v.push((format!("JUGADOR {id} · {}", name.to_uppercase()), "CONECTADO".into(), false, Act::None));
            }
            v.push(("CERRAR LA PARTIDA ONLINE".into(), "LOS DEMÁS QUEDAN AFUERA".into(), true, Act::StopOnline));
        }
        Role::Client(r) => {
            let state = if r.ready { format!("CONECTADO A {} · {:.0} MS", r.addr, r.net.rtt_ms()) } else { net.status.to_uppercase() };
            v.push((if r.ready { "EN LA PARTIDA".into() } else { "CONECTANDO…".into() }, state, false, Act::None));
            for (id, name) in r.net.players.iter().take(4) {
                v.push((format!("JUGADOR {id} · {}", name.to_uppercase()), if Some(*id) == r.net.you { "VOS".into() } else { String::new() }, false, Act::None));
            }
            v.push(("DESCONECTARSE".into(), String::new(), true, Act::StopOnline));
        }
    }
    if !net.status.is_empty() && matches!(net.role, Role::Off) {
        v.push(("AVISO".into(), net.status.to_uppercase(), false, Act::None));
    }
    v.push(("VOLVER".into(), "ESC".into(), true, Act::Back));
}

/// Nombre de un mapa para la línea de comandos.
fn map_arg(m: MapId) -> &'static str {
    match m {
        MapId::Ciudad => "ciudad",
        MapId::Deposito => "deposito",
        MapId::Fundicion => "fundicion",
        MapId::Cantera => "cantera",
        MapId::Azotea => "azotea",
    }
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
    mut opts: Opts,
    (mut net, mut typed): (ResMut<Online>, MessageReader<bevy::input::keyboard::KeyboardInput>),
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
    let camp = p.game.0.campaign.as_ref().map(|c| c.op).filter(|_| menu.playing);
    let list = rows(&menu, &prog, &opts.vals(), camp, &net);
    // campos de texto: lo que se escribe va al nombre o a la IP
    let text = match list.get(menu.sel).map(|r| r.3) {
        Some(Act::Text(k)) => Some(k),
        _ => None,
    };
    if let Some(k) = text {
        use bevy::input::ButtonState;
        use bevy::input::keyboard::Key;
        for ev in typed.read() {
            if ev.state != ButtonState::Pressed {
                continue;
            }
            let field = if k == 0 { &mut net.name } else { &mut net.ip };
            match &ev.logical_key {
                Key::Backspace => {
                    field.pop();
                }
                Key::Character(c) => {
                    for ch in c.chars() {
                        let ok = if k == 0 { ch.is_alphanumeric() || ch == ' ' } else { ch.is_ascii_digit() || ch == '.' || ch == ':' };
                        if ok && field.chars().count() < if k == 0 { 16 } else { 21 } {
                            field.push(ch);
                        }
                    }
                }
                _ => {}
            }
        }
        if k == 0 {
            crate::config::set("nombre", net.name.trim());
        }
    } else {
        typed.clear();
    }
    // ← → ajustan la opción elegida
    if let Some(Act::Opt(k)) = list.get(menu.sel).map(|r| r.3) {
        let dir = if key.just_pressed(KeyCode::ArrowLeft) || key.just_pressed(KeyCode::KeyA) { -1.0 } else if key.just_pressed(KeyCode::ArrowRight) || key.just_pressed(KeyCode::KeyD) { 1.0 } else { 0.0 };
        if dir != 0.0 {
            opts.change(k, dir);
            snd.play(crate::audio::S::Swap);
        }
    }
    let n = list.len();
    if key.just_pressed(KeyCode::ArrowDown) || (text.is_none() && key.just_pressed(KeyCode::KeyS)) {
        menu.sel = (menu.sel + 1) % n;
        snd.play(crate::audio::S::Swap);
    }
    if key.just_pressed(KeyCode::ArrowUp) || (text.is_none() && key.just_pressed(KeyCode::KeyW)) {
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
    if (key.just_pressed(KeyCode::Enter) || (text.is_none() && key.just_pressed(KeyCode::Space))) && list.get(menu.sel).is_some_and(|r| r.2) {
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
        Act::OptionsPage => {
            menu.page = Page::Options;
            menu.sel = 0;
        }
        Act::Opt(k) => opts.change(k, 1.0),
        Act::Restart(op) => {
            start_op(&mut p, op);
            play(&mut open, &mut menu);
        }
        Act::OnlinePage => {
            menu.page = Page::Online;
            menu.sel = 0;
            if net.scanner.is_none() {
                net.scanner = Some(arena_net::lan::Scanner::new());
            }
        }
        Act::Host(k) => {
            net.status.clear();
            match k {
                0 => {}
                1 => {
                    let op = prog.unlocked.min(N_OPS - 1);
                    if !city {
                        relaunch(&["--op", &(op + 1).to_string(), "--host"], &mut exit);
                        return;
                    }
                    start_op(&mut p, op);
                }
                2 => {
                    if !city {
                        relaunch(&["--libre", "--host"], &mut exit);
                        return;
                    }
                    start_free(&mut p);
                }
                _ => {
                    relaunch(&["--map", "deposito", "--host"], &mut exit);
                    return;
                }
            }
            net.host(&p.game.0, &p.level.0);
            if net.active() {
                p.ui.banner("PARTIDA ABIERTA", net.status.clone());
                play(&mut open, &mut menu);
            }
        }
        Act::JoinLan(i) => {
            if let Some(addr) = net.scanner.as_ref().and_then(|s| s.found.get(i)).map(|f| f.addr) {
                net.join(addr);
            }
        }
        Act::Text(1) => {
            let ip = net.ip.trim().to_owned();
            let addr = if ip.contains(':') { ip.parse().ok() } else { format!("{ip}:{}", arena_net::PORT).parse().ok() };
            match addr {
                Some(a) => net.join(a),
                None => net.status = "Esa IP no es válida (ejemplo: 192.168.0.12)".into(),
            }
        }
        Act::Text(_) => return,
        Act::StopOnline => {
            net.stop();
            menu.sel = 0;
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
    opt_res: (Res<crate::player::Settings>, Res<crate::audio::MusicVol>, Res<crate::audio::SfxVol>, Res<crate::quality::QualityRes>, Res<crate::quality::BloomOn>, Res<crate::hud::ShowFps>),
    net: Res<Online>,
) {
    let show = open.0 && !game.0.campaign.as_ref().is_some_and(|c| c.result.is_some());
    for mut v in &mut root {
        *v = if show { Visibility::Inherited } else { Visibility::Hidden };
    }
    if !show {
        return;
    }
    let (settings, music, sfx, quality, bloom, fps) = opt_res;
    let vals = OptVals { sens: settings.sens, music: music.0, sfx: sfx.0, quality: quality.label(), bloom: bloom.0, fps: fps.0 };
    let camp = game.0.campaign.as_ref().map(|c| c.op).filter(|_| menu.playing);
    let list = rows(&menu, &prog, &vals, camp, &net);
    for mut t in &mut title {
        t.0 = match menu.page {
            Page::Main => "OPERACIONES EN EL DISTRITO SUR".into(),
            Page::Ops => "ELEGÍ LA OPERACIÓN".into(),
            Page::Waves => "OLEADAS · ELEGÍ LA ARENA".into(),
            Page::Options => "OPCIONES".into(),
            Page::Online => "JUGAR ONLINE".into(),
        };
    }
    for mut t in &mut sub {
        t.0 = match menu.page {
            Page::Main => "Ocho operaciones encadenadas. Vera te guía por radio. Cada una termina con un rango.".into(),
            Page::Ops => "Las operaciones anteriores cuentan como hechas: la ciudad queda como la dejaste.".into(),
            Page::Waves => "Drones, kamikazes, pesados y el Coloso cada cinco oleadas.".into(),
            Page::Options => "Flechas izquierda y derecha cambian el valor. Se guarda solo.".into(),
            Page::Online => "Cooperativo hasta 4. En la misma red aparecen solas; por internet, el anfitrión abre el puerto UDP 47800.".into(),
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
    net: Res<Online>,
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
    if net.is_client() {
        for mut t in &mut hint {
            t.0 = "EL ANFITRIÓN ELIGE CÓMO SIGUE".into();
        }
        return;
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
    mut faces: Query<(&RadioFace, &mut Node)>,
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
    let face = if *who == arena_sim::campaign::VERA { 0 } else if *who == "CRÉDITOS" { 2 } else { 1 };
    for (rf, mut n) in &mut faces {
        let want = if rf.0 == face { Display::Flex } else { Display::None };
        if n.display != want {
            n.display = want;
        }
    }
    let n_shown = (t * 55.0) as usize;
    let shown: String = text.chars().take(n_shown).collect();
    // "voz" de radio: un pitido cada cuatro letras mientras escribe
    let before = ((t - dt) * 55.0).max(0.0) as usize;
    if n_shown / 4 != before / 4 && n_shown < text.chars().count() {
        snd.play_vol(crate::audio::S::Blip, 0.35);
    }
    for (mut w, mut c) in &mut who_q {
        let s = format!("> {who}");
        if w.0 != s {
            w.0 = s;
        }
        c.0 = if *who == arena_sim::campaign::VERA { CYAN } else if *who == "CRÉDITOS" { GOLD } else { RED };
    }
    for mut tx in &mut text_q {
        if tx.0 != shown {
            tx.0 = shown.clone();
        }
    }
}

/// Pantalla de briefing antes de cada operación: la partida espera hasta Enter.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn briefing(
    key: Res<ButtonInput<KeyCode>>,
    mut brief: ResMut<Briefing>,
    mut open: ResMut<MenuOpen>,
    game: Res<GameRes>,
    level: Res<LevelRes>,
    mut cursor: Single<&mut CursorOptions>,
    mut root: Query<&mut Visibility, With<BriefRoot>>,
    mut title: Query<&mut Text, (With<BriefTitle>, Without<BriefPlace>, Without<BriefText>)>,
    mut place: Query<&mut Text, (With<BriefPlace>, Without<BriefTitle>, Without<BriefText>)>,
    mut text: Query<&mut Text, (With<BriefText>, Without<BriefTitle>, Without<BriefPlace>)>,
    mut dots: Query<(&BriefDot, &mut Node, &mut Visibility, &mut BackgroundColor), Without<BriefRoot>>,
    mut snd: ResMut<crate::audio::Sounds>,
    mut armed: Local<bool>,
) {
    let show = brief.0.is_some() && game.0.campaign.is_some();
    for mut v in &mut root {
        *v = if show { Visibility::Inherited } else { Visibility::Hidden };
    }
    if !show {
        brief.0 = None;
        *armed = false;
        return;
    }
    open.0 = true;
    cursor.visible = true;
    cursor.grab_mode = CursorGrabMode::None;
    let Some(cp) = game.0.campaign.as_ref() else { return };
    for mut t in &mut title {
        let s = format!("{} · {}", cp.op + 1, cp.def.name.to_uppercase());
        if t.0 != s {
            t.0 = s;
        }
    }
    for mut t in &mut place {
        if t.0 != cp.def.place {
            t.0 = cp.def.place.to_uppercase();
        }
    }
    for mut t in &mut text {
        if t.0 != cp.def.brief {
            t.0 = cp.def.brief.to_string();
        }
    }
    // puntos en el mapa: el inicio en cian y los lugares de la operación en dorado
    let a = level.0.info.a;
    let to = |p: Vec3| ((p.x + a) / (2.0 * a) * 420.0, (p.z + a) / (2.0 * a) * 420.0);
    let mut pts = vec![(cp.def.start, CYAN)];
    if let Some(c) = level.0.city.as_ref() {
        pts.extend(cp.def.points(c).into_iter().map(|p| (p, GOLD)));
    }
    for (d, mut n, mut v, mut bg) in &mut dots {
        match pts.get(d.0) {
            Some(&(p, col)) => {
                let (x, y) = to(p);
                n.left = px(x);
                n.top = px(y);
                bg.0 = col;
                *v = Visibility::Inherited;
            }
            None => *v = Visibility::Hidden,
        }
    }
    // el Enter que abrió el briefing (desde el menú o el resumen) no lo cierra
    if !*armed {
        *armed = true;
        return;
    }
    if key.just_pressed(KeyCode::Enter) || key.just_pressed(KeyCode::Space) {
        brief.0 = None;
        *armed = false;
        open.0 = false;
        snd.play(crate::audio::S::Radio);
    }
}
