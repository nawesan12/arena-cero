//! Jugador local: junta el input cada cuadro, corre la simulación a paso fijo (60 Hz) e
//! interpola la cámara entre pasos. El mismo esquema sirve después para predecir contra el servidor.

use crate::audio::{S, Sounds};
use crate::fx::Fx;
use crate::game::{GameRes, UiFeed};
use crate::audio::MusicLevel;
use crate::perks::PerkState;
use crate::gun::GunRig;
use crate::world_render::LevelRes;
use arena_sim::player::{BASE_FOV, EYE};
use arena_sim::rng::Rng;
use arena_sim::enemies::Target;
use arena_sim::game::{Game, GameEvent, Mark, Snd};
use arena_sim::objectives::Actor;
use arena_sim::vehicle::DriveInput;
use arena_sim::economy::LootKind;
use arena_sim::maps::Level;
use arena_sim::weapons::{Arsenal, WEAPONS, WeaponEvent, WeaponInput};
use arena_sim::{PlayerEvent, PlayerInput, PlayerState, TICK};
use bevy::camera::Exposure;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll};
use bevy::pbr::{DistanceFog, FogFalloff};
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions};

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Time::<Fixed>::from_seconds(TICK as f64))
            .init_resource::<Look>()
            .init_resource::<Latch>()
            .init_resource::<Settings>()
            .init_resource::<HitStop>()
            .add_systems(Update, (grab_cursor, mouse_look, latch_input, test_hook).chain())
            .add_systems(FixedUpdate, step_player)
            .add_systems(PostUpdate, place_camera.before(TransformSystems::Propagate));
    }
}

#[derive(Resource)]
pub struct Settings {
    pub sens: f32,
    pub input: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self { sens: crate::config::get_f32("sens").unwrap_or(1.8).clamp(0.3, 6.0), input: true }
    }
}

/// Congelamiento corto al matar (hit-stop): la partida se frena unos pasos para que el golpe pese.
#[derive(Resource, Default)]
pub struct HitStop(pub f32);

/// Vista: se actualiza cada cuadro (no cada paso) para que el mouse responda sin demora.
#[derive(Resource, Default)]
pub struct Look {
    pub yaw: f32,
    pub pitch: f32,
    pub fov: f32,
    /// Inercia del arma al mover el mouse.
    pub sway_x: f32,
    pub sway_y: f32,
    /// Golpe de campo de visión al matar (grados, se desvanece solo).
    pub punch: f32,
}

/// Pulsaciones guardadas hasta el próximo paso fijo (si no, se perderían entre pasos).
#[derive(Resource, Default)]
struct Latch {
    jump: bool,
    dash: bool,
    hook: bool,
    reload: bool,
    nade: bool,
    select: Option<usize>,
    cycle: i32,
    tag: bool,
    gadget: bool,
    gadget_cycle: bool,
    /// El clic que toma el mouse no dispara.
    grabbed_at: f64,
}

#[derive(Resource)]
pub struct LocalPlayer {
    pub state: PlayerState,
    pub prev: Vec3,
    pub events: Vec<PlayerEvent>,
    pub arsenal: Arsenal,
    pub rng: Rng,
    pub hp: f32,
    pub max_hp: f32,
    pub last_hurt: f32,
    pub regen_mul: f32,
    /// Integridad que devuelve cada baja (mejora Absorción).
    pub leech: f32,
    /// Fase del balanceo al caminar.
    pub bob: f32,
    /// 0..1: viñeta roja al recibir daño.
    pub hurt_flash: f32,
    /// Hacia dónde vino el último golpe (radianes en pantalla) y cuánto queda del indicador.
    pub dmg_dir: f32,
    pub dmg_t: f32,
    pub start: Vec3,
    /// Golpes recibidos desde el último paso (para el sonido).
    pub hurt_count: u32,
}

impl LocalPlayer {
    pub fn hurt(&mut self, d: f32, from: Option<Vec3>) {
        if d <= 0.0 {
            return;
        }
        self.hp = (self.hp - d).max(0.0);
        self.last_hurt = 0.0;
        self.hurt_count += 1;
        self.hurt_flash = (self.hurt_flash + 0.55).min(1.0);
        self.state.shake = (self.state.shake + 0.25).min(1.0);
        if let Some(f) = from {
            let (dx, dz) = (f.x - self.state.pos.x, f.z - self.state.pos.z);
            let (sy, cy) = self.state.yaw.sin_cos();
            let fwd = -dx * sy - dz * cy;
            let rgt = dx * cy - dz * sy;
            self.dmg_dir = rgt.atan2(fwd);
            self.dmg_t = 1.0;
        }
    }
}

#[derive(Component)]
pub struct MainCamera;

pub fn spawn_player(commands: &mut Commands, level: &LevelRes, start: Option<[f32; 4]>, weapon: Option<usize>) {
    let info = &level.0.info;
    let mut state = PlayerState::new(info.start[0], info.start[1]);
    if let Some([x, z, yaw, pitch]) = start {
        state.pos.x = x;
        state.pos.z = z;
        state.pos.y = level.0.world.ground_at(x, z, 200.0) + EYE;
        state.yaw = yaw;
        state.pitch = pitch;
    }
    let mut arsenal = Arsenal::new(info.city);
    if let Some(w) = weapon {
        arsenal.unlocked[w] = true;
        arsenal.w = w;
    }
    commands.insert_resource(Look { yaw: state.yaw, pitch: state.pitch, fov: BASE_FOV, ..default() });
    commands.insert_resource(LocalPlayer {
        prev: state.pos,
        start: state.pos,
        state,
        events: Vec::new(),
        arsenal,
        rng: Rng::new(7),
        hp: 100.0,
        max_hp: 100.0,
        last_hurt: 10.0,
        regen_mul: 1.0,
        leech: 0.0,
        bob: 0.0,
        hurt_flash: 0.0,
        dmg_dir: 0.0,
        dmg_t: 0.0,
        hurt_count: 0,
    });
    let fog_end = info.fog[1].min(350.0);
    commands.spawn((
        Camera3d::default(),
        MainCamera,
        Projection::from(PerspectiveProjection { fov: BASE_FOV.to_radians(), near: 0.05, far: 360.0, ..default() }),
        Tonemapping::None,
        Exposure::default(),
        // oídos a 0,5 m: sirve para panear los efectos que traen posición
        SpatialListener::new(0.5),
        DistanceFog { color: crate::world_render::col(info.bg), falloff: FogFalloff::Linear { start: info.fog[0].min(fog_end * 0.4), end: fog_end }, ..default() },
        Transform::from_translation(Vec3::new(info.start[0], 1.6, info.start[1])),
        crate::world_render::LevelEntity,
    ));
}

fn grab_cursor(mut cursor: Single<&mut CursorOptions>, mouse: Res<ButtonInput<MouseButton>>, key: Res<ButtonInput<KeyCode>>, settings: Res<Settings>, time: Res<Time<Real>>, mut latch: ResMut<Latch>, menu: Res<crate::campaign_ui::MenuOpen>) {
    // en el menú y en el resumen el mouse queda libre para hacer clic
    if !settings.input || menu.0 {
        return;
    }
    if mouse.just_pressed(MouseButton::Left) && cursor.grab_mode == CursorGrabMode::None {
        cursor.visible = false;
        cursor.grab_mode = CursorGrabMode::Locked;
        latch.grabbed_at = time.elapsed_secs_f64();
    }
    if key.just_pressed(KeyCode::Escape) {
        cursor.visible = true;
        cursor.grab_mode = CursorGrabMode::None;
    }
}

fn mouse_look(motion: Res<AccumulatedMouseMotion>, cursor: Single<&CursorOptions>, settings: Res<Settings>, mut look: ResMut<Look>) {
    if cursor.grab_mode == CursorGrabMode::None {
        return;
    }
    let d = motion.delta;
    // picos al tomar el puntero
    if d.x.abs() > 180.0 || d.y.abs() > 180.0 {
        return;
    }
    let s = 0.0022 * settings.sens * (look.fov / BASE_FOV);
    look.yaw -= d.x * s;
    look.pitch = (look.pitch - d.y * s).clamp(-1.5, 1.5);
    look.sway_x = (look.sway_x + d.x * 0.00012).clamp(-0.06, 0.06);
    look.sway_y = (look.sway_y + d.y * 0.0001).clamp(-0.05, 0.05);
}

fn latch_input(key: Res<ButtonInput<KeyCode>>, mouse: Res<ButtonInput<MouseButton>>, scroll: Res<AccumulatedMouseScroll>, cursor: Single<&CursorOptions>, perks: Res<PerkState>, shop: Res<crate::shop::ShopOpen>, subway: Res<crate::subway::SubwayOpen>, mut latch: ResMut<Latch>) {
    if cursor.grab_mode == CursorGrabMode::None || perks.active() || shop.0 || subway.0 {
        return;
    }
    latch.jump |= key.just_pressed(KeyCode::Space);
    latch.dash |= key.just_pressed(KeyCode::KeyQ);
    latch.hook |= key.just_pressed(KeyCode::KeyE) || key.just_pressed(KeyCode::ControlLeft) || key.just_pressed(KeyCode::ControlRight);
    latch.reload |= key.just_pressed(KeyCode::KeyR);
    latch.nade |= key.just_pressed(KeyCode::KeyG);
    latch.tag |= key.just_pressed(KeyCode::KeyT) || mouse.just_pressed(MouseButton::Middle);
    latch.gadget |= key.just_pressed(KeyCode::KeyV);
    latch.gadget_cycle |= key.just_pressed(KeyCode::KeyZ);
    for (i, k) in [KeyCode::Digit1, KeyCode::Digit2, KeyCode::Digit3, KeyCode::Digit4].into_iter().enumerate() {
        if key.just_pressed(k) {
            latch.select = Some(i);
        }
    }
    if scroll.delta.y != 0.0 {
        latch.cycle = if scroll.delta.y < 0.0 { 1 } else { -1 };
    }
}

#[allow(clippy::too_many_arguments)]
fn step_player(
    key: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    cursor: Single<&CursorOptions>,
    time: Res<Time<Real>>,
    mut look: ResMut<Look>,
    mut latch: ResMut<Latch>,
    mut level: ResMut<LevelRes>,
    mut lp: ResMut<LocalPlayer>,
    mut game: ResMut<GameRes>,
    mut ui: ResMut<UiFeed>,
    mut music: ResMut<MusicLevel>,
    mut rig: ResMut<GunRig>,
    mut fx: ResMut<Fx>,
    mut snd: ResMut<Sounds>,
    (mut perks, shop, torch, subway): (ResMut<PerkState>, Res<crate::shop::ShopOpen>, Res<crate::daynight::Flashlight>, Res<crate::subway::SubwayOpen>),
    (menu, settings, mut hitstop, mut decals, mut net): (Res<crate::campaign_ui::MenuOpen>, Res<Settings>, ResMut<HitStop>, ResMut<crate::decals::Decals>, ResMut<crate::online::Online>),
) {
    // online la partida nunca se pausa (los demás siguen jugando): con menús, quieto
    let online = net.active();
    let paused = perks.active() || shop.0 || subway.0 || menu.0 || (settings.input && cursor.grab_mode == CursorGrabMode::None);
    if paused && !online {
        return;
    }
    // operación terminada: queda congelada detrás del resumen
    if game.0.campaign.as_ref().is_some_and(|c| c.result.is_some()) && !online {
        return;
    }
    if hitstop.0 > 0.0 {
        hitstop.0 -= TICK;
        if !online {
            return;
        }
    }
    let playing = !paused && cursor.grab_mode != CursorGrabMode::None;
    let axis = |pos: KeyCode, neg: KeyCode| if playing { f32::from(u8::from(key.pressed(pos))) - f32::from(u8::from(key.pressed(neg))) } else { 0.0 };
    let input = PlayerInput {
        forward: axis(KeyCode::KeyW, KeyCode::KeyS),
        right: axis(KeyCode::KeyD, KeyCode::KeyA),
        yaw: look.yaw,
        pitch: look.pitch,
        sprint: playing && (key.pressed(KeyCode::ShiftLeft) || key.pressed(KeyCode::ShiftRight)),
        crouch: playing && key.pressed(KeyCode::KeyC),
        aim: playing && mouse.pressed(MouseButton::Right),
        jump: std::mem::take(&mut latch.jump),
        dash: std::mem::take(&mut latch.dash),
        hook: std::mem::take(&mut latch.hook),
    };
    let winput = WeaponInput {
        fire: playing && mouse.pressed(MouseButton::Left) && time.elapsed_secs_f64() - latch.grabbed_at > 0.25 && !net.down(),
        reload: std::mem::take(&mut latch.reload),
        select: latch.select.take(),
        cycle: std::mem::take(&mut latch.cycle),
        nade: std::mem::take(&mut latch.nade),
    };
    let lp = &mut *lp;
    let level = &mut level.0;
    let GameRes(game, generation) = &mut *game;
    let crate::online::Online { role, others, .. } = &mut *net;
    let client = matches!(role, crate::online::Role::Client(_));
    match role {
        crate::online::Role::Client(r) => crate::online::client_pre(r, others, game, level, lp, &mut ui, &mut snd),
        crate::online::Role::Host(h) => {
            // los remotos: controles que llegaron, y se mueven y disparan antes que la IA
            h.receive(game, level);
            game.step_pawns(level, TICK);
            for p in &game.pawns {
                for w in &p.wfx {
                    if let WeaponEvent::Fire { weapon, .. } = w {
                        snd.play_at(S::Shot((*weapon).min(2) as u8), 0.3, p.state.eye());
                    }
                }
            }
        }
        crate::online::Role::Off => {}
    }
    lp.prev = lp.state.eye();
    let aim_before = lp.state.aim_t;
    let mut pev = std::mem::take(&mut lp.events);
    pev.clear();
    let was_driving = game.driving;
    if let Some(vi) = was_driving {
        // manejando: W/S acelerador, A/D dirección, Espacio freno; el jugador va con el auto
        let din = DriveInput { throttle: input.forward, steer: input.right, brake: playing && key.pressed(KeyCode::Space) };
        game.drive(level, &din, TICK);
        let v = &game.vehicles[vi];
        lp.state.pos = v.pos + Vec3::Y * 1.6;
        lp.state.vel = v.forward() * v.speed;
        lp.state.on_ground = true;
        lp.state.hook = None;
        lp.state.crouch_t = 0.0;
    } else {
        lp.state.step(&input, &level.world, TICK, &mut pev);
    }
    for e in &pev {
        match *e {
            PlayerEvent::Burn(d) => {
                lp.hurt(f32::from(d), None);
                fx.burst(Vec3::new(lp.state.pos.x, 0.1, lp.state.pos.z), 0xff8030, 2, 3.0, 0.4);
            }
            PlayerEvent::Dash => {
                fx.burst(lp.state.pos - Vec3::Y, 0x5fd8ff, 18, 3.0, 0.35);
                snd.play(S::Dash);
            }
            PlayerEvent::DoubleJump | PlayerEvent::HookBoost => {
                fx.burst(lp.state.pos - Vec3::Y * EYE, 0x5fd8ff, 16, 4.0, 0.4);
                snd.play(S::DJump);
            }
            PlayerEvent::Slide => snd.play(S::Slide),
            PlayerEvent::HookFired => snd.play(S::Hook),
            PlayerEvent::HookMiss => snd.play(S::HookMiss),
            PlayerEvent::Splash => {
                fx.burst(lp.state.pos - Vec3::Y * EYE, 0x9fd8ff, 30, 5.0, 0.6);
                snd.play(S::Toss);
                ui.banner("AL AGUA", "La corriente te devolvió a la costanera");
            }
            _ => {}
        }
    }
    if lp.state.slide > 0.0 && lp.state.on_ground && lp.rng.next() < 0.5 {
        fx.burst(Vec3::new(lp.state.pos.x, 0.05, lp.state.pos.z), 0x3a4246, 1, 2.0, 0.4);
    }
    lp.events = pev;
    if aim_before < 0.5 && lp.state.aim_t >= 0.5 {
        snd.play(S::Aim);
    }

    // armas: las granadas van a la partida, que también resuelve los disparos contra enemigos
    let mut wev = Vec::new();
    let (arsenal, state, rng) = (&mut lp.arsenal, &mut lp.state, &mut lp.rng);
    let winput = if was_driving.is_some() { WeaponInput::default() } else { winput };
    let pulses = crate::online::Pulses { reload: winput.reload, nade: winput.nade, tag: latch.tag, fire: winput.fire, use_held: playing && key.pressed(KeyCode::KeyF), light: torch.0 };
    // en el cliente las granadas y los impactos los resuelve el servidor (acá solo se ven y se oyen)
    let mut own_nades = Vec::new();
    arsenal.step(&winput, state, if client { &mut own_nades } else { &mut game.nades }, rng, TICK, &mut wev);
    let cam_rot = Quat::from_euler(EulerRot::YXZ, lp.state.yaw, lp.state.pitch, 0.0);
    let muzzle = lp.state.eye() + cam_rot * rig.muzzle_cam;
    if !client {
        game.weapon_events(&wev, lp.arsenal.crit_mul, muzzle, lp.state.body(), level);
    }
    for e in wev {
        match e {
            WeaponEvent::Fire { weapon, .. } => {
                if let crate::online::Role::Host(h) = role {
                    h.shot(weapon as u8, muzzle);
                }
                // el destello dura casi todo el cuadro de disparo (en la web, hasta rate − 0,04)
                rig.flash_t = (WEAPONS[weapon].rate - 0.04).min(0.06);
                snd.play(S::Shot(weapon.min(2) as u8));
            }
            WeaponEvent::Launch { .. } => {
                rig.flash_t = 0.06;
                rig.drum_target += std::f32::consts::PI / 3.0;
                snd.play(S::Launch);
            }
            WeaponEvent::Kick { pitch, yaw } => {
                look.pitch = (look.pitch + pitch).min(1.5);
                look.yaw += yaw;
            }
            WeaponEvent::Toss => snd.play(S::Toss),
            WeaponEvent::Empty => snd.play(S::Empty),
            WeaponEvent::Reload => snd.play(S::Reload),
            WeaponEvent::Swap(_) => snd.play(S::Swap),
            WeaponEvent::Locked => ui.banner("BLOQUEADA", "Forzá el arsenal para usar esta arma"),
            _ => {}
        }
    }

    // partida
    let s = &lp.state;
    let target = Target { pos: s.pos, body: s.body(), vel: s.vel, feet: s.feet(), crouch_t: s.crouch_t, evasive: s.hook.is_some() || s.dash_t > 0.0, light: torch.0 };
    if let crate::online::Role::Client(r) = role {
        // cliente: la partida la corre el servidor; se mandan los controles de este paso
        crate::online::client_send(r, &input, &pulses, lp);
        latch.tag = false;
        latch.gadget = false;
        latch.gadget_cycle = false;
    } else {
        if !crate::off("ai") {
            if game.pawns.is_empty() {
                game.step(level, &target, lp.hp >= lp.max_hp, TICK);
            } else {
                // anfitrión: la IA elige entre todos los jugadores
                let ts = game.all_targets(Some(target));
                let full: Vec<bool> = std::iter::once(lp.hp >= lp.max_hp).chain(game.pawns.iter().map(|p| p.hp >= p.max_hp)).collect();
                game.step_multi(level, &ts, &full, TICK);
                game.step_pawn_use(level, TICK);
            }
        }
        let actor = Actor { pos: lp.state.pos, feet: lp.state.feet(), f_held: pulses.use_held };
        game.step_objectives(level, &actor, TICK);
        game.step_campaign(level, &actor, TICK);
        if !game.pawns.is_empty() {
            game.apply_pawn_hits(1);
        }
        if std::mem::take(&mut latch.tag) {
            game.try_tag(level, lp.state.eye(), lp.state.look_dir());
        }
        if std::mem::take(&mut latch.gadget_cycle) {
            game.gadget_sel = (game.gadget_sel + 1) % 3;
            snd.play(S::Swap);
        }
        if std::mem::take(&mut latch.gadget) && game.driving.is_none() {
            game.use_gadget(lp.state.eye(), lp.state.look_dir(), lp.state.vel);
        }
    }
    if let crate::online::Role::Host(h) = role {
        h.collect(game);
    }
    let beeps = game.nades.iter().filter(|n| !n.impact && n.beep > 0.3 - TICK && n.beep <= 0.3).count();
    if beeps > 0 {
        snd.play(S::Beep);
    }
    for e in std::mem::take(&mut game.events) {
        match e {
            // en solitario el jugador local es el 0; en cooperativo, el índice propio
            GameEvent::Hurt { dmg, from, to } if to == 0 => lp.hurt(dmg, from),
            GameEvent::Hurt { .. } => {}
            GameEvent::Sound(x) => match x {
                Snd::EShot { dist, big, at } => snd.play_at(if big { S::EShotBig } else { S::EShot }, ((if big { 0.5 } else { 0.25 }) - dist * 0.005).max(0.02), at),
                Snd::Enemy { dist, at } => snd.play_at(S::Enemy, (0.14 - dist * 0.004).max(0.02), at),
                Snd::Whiz => snd.play(S::Whiz),
                Snd::Boss => snd.play(S::Boss),
                Snd::Siren => snd.play(S::Siren),
                Snd::AlertBeep => snd.play(S::AlertBeep),
                Snd::Kill => snd.play(S::Kill),
                Snd::Boom => snd.play(S::Boom),
                Snd::Hit => snd.play(S::Hit),
                Snd::Crit => snd.play(S::Crit),
                Snd::Ricochet => snd.play(S::Ricochet),
                Snd::Pickup => snd.play(S::Pickup),
                Snd::Plant => snd.play(S::Plant),
                Snd::Takedown => snd.play(S::Takedown),
                Snd::Objective => snd.play(S::Objective),
                Snd::Beep => snd.play(S::Beep),
                Snd::Tag => snd.play(S::Tag),
                Snd::Crash => snd.play(S::Crash),
                Snd::Decoy => snd.play_vol(S::Beep, 0.6),
                Snd::Toss => snd.play(S::Toss),
                Snd::Scream => snd.play_vol(S::Hurt, 0.5),
                Snd::Train => snd.play_vol(S::Train, 0.7),
                Snd::Radio => snd.play(S::Radio),
            },
            GameEvent::Burst { pos, color, n, speed, life } => fx.burst(pos, color, n, speed, life),
            GameEvent::Tracer { a, b, color, life } => fx.tracer(a, b, color, life),
            GameEvent::Boom { pos, big } => fx.boom(pos, big),
            GameEvent::Shake(k) => lp.state.shake = (lp.state.shake + k).min(1.0),
            GameEvent::Banner { title, sub } => ui.banner(title, sub),
            GameEvent::Mark(m) => {
                ui.mark = Some(m);
                ui.mark_t = if m == Mark::Kill { 0.25 } else { 0.12 };
                if m == Mark::Kill {
                    // la baja pesa: un instante congelado, la vista se abre y tiembla un poco
                    hitstop.0 = hitstop.0.max(0.05);
                    look.punch = 3.0;
                    lp.state.shake = (lp.state.shake + 0.12).min(1.0);
                }
            }
            GameEvent::Impact { pos, normal } => decals.add(pos, normal),
            GameEvent::Streak(n) => {
                ui.streak = Some((n, 0.0));
                snd.play(S::Streak);
                hitstop.0 = hitstop.0.max(0.07);
                look.punch = 5.0;
            }
            GameEvent::Num { pos, value, crit } => ui.nums.push((pos, value, crit, 0.0)),
            GameEvent::Heal(h) => lp.hp = (lp.hp + h).min(lp.max_hp),
            GameEvent::Overdrive => lp.arsenal.overdrive = 8.0,
            GameEvent::Refill => {
                lp.hp = lp.max_hp;
                lp.arsenal.refill();
            }
            GameEvent::Unlock => {
                lp.arsenal.unlocked = [true; 4];
                lp.arsenal.refill();
            }
            GameEvent::Victory { bonus } => ui.banner("DISTRITO LIBERADO", format!("+{bonus} de bono · {} puntos en total", game.score)),
            GameEvent::OfferPerk => {
                perks.offer(&mut lp.rng);
                snd.play(S::Perk);
            }
            GameEvent::ExitVehicle => {
                if let Some(v) = was_driving.and_then(|i| game.vehicles.get(i)) {
                    let p = v.exit_spot(&level.world);
                    lp.state.pos = p + Vec3::Y * EYE;
                    lp.state.vel = Vec3::ZERO;
                    lp.prev = lp.state.pos;
                    look.yaw = v.yaw + std::f32::consts::PI;
                }
            }
            GameEvent::Credits { amount, label } => {
                let total = if ui.credit_t > 0.0 { ui.credit.map_or(0, |c| c.0) } else { 0 } + amount;
                ui.credit = Some((total, label));
                ui.credit_t = 1.6;
            }
            GameEvent::Loot(k) => match k {
                LootKind::Ammo => {
                    let a = &mut lp.arsenal;
                    a.ammo = [0, 1, 2, 3].map(|i| a.mag_of(i));
                    ui.banner("MUNICIÓN", "Cargadores llenos");
                }
                LootKind::Nades => {
                    lp.arsenal.nades = lp.arsenal.max_nades;
                    ui.banner("GRANADAS", "Bandolera llena");
                }
                _ => {}
            },
            GameEvent::WaveCleared => lp.arsenal.nades = (lp.arsenal.nades + 1).min(lp.arsenal.max_nades),
            GameEvent::Radio { who, text } => ui.radio.push_back((who, text)),
            GameEvent::Respawn { pos, yaw } => {
                respawn_at(lp, pos + Vec3::Y * EYE);
                lp.arsenal.refill();
                look.yaw = yaw;
                look.pitch = 0.0;
                lp.state.yaw = yaw;
            }
            GameEvent::Score { points, label } => {
                if !label.starts_with("OBJETIVO") && lp.leech > 0.0 {
                    lp.hp = (lp.hp + lp.leech).min(lp.max_hp);
                }
                // si llegan seguidos se suman, como en los shooters
                let total = if ui.popup_t > 0.0 { ui.popup.map_or(0, |p| p.0) } else { 0 } + points;
                ui.popup = Some((total, label));
                ui.popup_t = 1.4;
                ui.feed.push((format!("{label} +{points}"), 0.0));
                if ui.feed.len() > 5 {
                    ui.feed.remove(0);
                }
            }
        }
    }
    for _ in 0..std::mem::take(&mut lp.hurt_count).min(1) {
        snd.play(S::Hurt);
    }
    music.0 = game.music_level(lp.hp / lp.max_hp);

    // vida: regeneración tras 4 s sin daño
    lp.last_hurt += TICK;
    if lp.last_hurt > 4.0 && lp.hp < lp.max_hp {
        lp.hp = (lp.hp + 4.0 * lp.regen_mul * TICK).min(lp.max_hp);
    }
    if let crate::online::Role::Host(h) = role {
        h.send(game, level, Some(crate::online::host_self(lp)));
        *others = crate::online::pawn_snaps(game);
    }
    // en el cliente la caída la decide el servidor
    if lp.hp <= 0.0 && !client {
        if player_down(lp, game, level, &mut ui) {
            *generation += 1;
            perks.taken.clear();
        }
    }
    if lp.state.on_ground && lp.state.slide <= 0.0 {
        lp.bob += TICK * lp.state.horizontal_speed() * 1.6;
    }
    lp.hurt_flash = (lp.hurt_flash - TICK * 1.6).max(0.0);
    lp.dmg_t = (lp.dmg_t - TICK * 1.2).max(0.0);
}

/// Caída del jugador. En la ciudad cuesta un intento y se reaparece en el punto de
/// reabastecimiento más cercano; sin intentos (o en oleadas) la partida vuelve a empezar.
/// Devuelve si la partida volvió a empezar.
/// Vuelve a poner al jugador en `at` (ojos), conservando las mejoras de movimiento.
pub fn respawn_at(lp: &mut LocalPlayer, at: Vec3) {
    let (yaw, sm, mj, dm) = (lp.state.yaw, lp.state.speed_mul, lp.state.max_jumps, lp.state.dash_cd_mul);
    lp.state = PlayerState::new(at.x, at.z);
    lp.state.speed_mul = sm;
    lp.state.max_jumps = mj;
    lp.state.dash_cd_mul = dm;
    lp.state.pos = Vec3::new(at.x, at.y, at.z);
    lp.state.yaw = yaw;
    lp.prev = lp.state.pos;
    lp.hp = lp.max_hp;
    lp.last_hurt = 10.0;
    lp.hurt_flash = 1.0;
}

fn player_down(lp: &mut LocalPlayer, game: &mut Game, level: &mut Level, ui: &mut UiFeed) -> bool {
    // campaña: sin intentos; el paso vuelve a empezar desde el punto de control
    if game.campaign.is_some() {
        lp.hp = lp.max_hp;
        game.campaign_down(level);
        ui.banner("CAÍSTE", "Volvés al último punto de control");
        return false;
    }
    let city_lives = game.city.as_ref().map(|c| c.lives);
    let respawn = respawn_at;
    match city_lives {
        Some(l) if l > 1 => {
            game.city.as_mut().unwrap().lives -= 1;
            game.on_player_down();
            let safe = level.city.as_ref().map_or(lp.start, |c| c.safe_pos + Vec3::new(1.5, EYE - 1.0, 1.5));
            respawn(lp, safe);
            let left = l - 1;
            ui.banner("CAÍSTE", if left > 1 { format!("Te quedan {left} intentos") } else { "Último intento".into() });
            false
        }
        _ => {
            let (score, wave, kills) = (game.score, game.wave, game.kills);
            game.teardown(&mut level.world);
            *game = Game::new(level, 7);
            let start = lp.start;
            respawn(lp, start);
            lp.arsenal = Arsenal::new(level.info.city);
            if level.info.city {
                ui.banner("SIN INTENTOS", format!("{score} puntos · {kills} bajas · el distrito vuelve a empezar"));
            } else {
                ui.banner("TE DESMANTELARON", format!("{score} puntos · oleada {wave} · {kills} bajas"));
            }
            lp.max_hp = 100.0;
            lp.hp = 100.0;
            lp.regen_mul = 1.0;
            lp.leech = 0.0;
            true
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn place_camera(
    time: Res<Time>,
    fixed: Res<Time<Fixed>>,
    mut look: ResMut<Look>,
    lp: Res<LocalPlayer>,
    game: Res<GameRes>,
    level: Res<LevelRes>,
    mut cam: Query<(&mut Transform, &mut Projection), With<MainCamera>>,
    mut chase: Local<Option<Vec3>>,
) {
    let Ok((mut t, mut proj)) = cam.single_mut() else { return };
    let dt = time.delta_secs();
    // manejando: cámara en tercera persona que sigue al auto y se puede girar con el mouse
    if let Some(v) = game.0.driving.and_then(|i| game.0.vehicles.get(i)) {
        let behind = v.yaw + std::f32::consts::PI;
        let mut d = (behind - look.yaw).rem_euclid(std::f32::consts::TAU);
        if d > std::f32::consts::PI {
            d -= std::f32::consts::TAU;
        }
        // vuelve sola detrás del auto (más rápido cuanto más rápido va)
        look.yaw += d * (dt * (1.0 + v.speed.abs() * 0.12)).min(1.0);
        look.pitch = look.pitch.clamp(-0.7, 0.25);
        let pitch = look.pitch - 0.18;
        let dir = arena_sim::player::look_dir(look.yaw, pitch);
        let pivot = v.pos + Vec3::Y * 1.9;
        let free = level.0.world.ray(pivot, -dir, 7.5);
        let want = pivot - dir * (free - 0.4).clamp(1.5, 7.0);
        let cur = chase.get_or_insert(want);
        *cur = cur.lerp(want, (dt * 12.0).min(1.0));
        t.translation = *cur;
        t.rotation = Quat::from_euler(EulerRot::YXZ, look.yaw, pitch, 0.0);
        let fov = 75.0 + v.speed.abs() * 0.35;
        look.fov += (fov - look.fov) * (dt * 4.0).min(1.0);
        if let Projection::Perspective(p) = &mut *proj {
            p.fov = look.fov.to_radians();
        }
        return;
    }
    *chase = None;
    let s = &lp.state;
    let a = fixed.overstep_fraction();
    let eye = lp.prev.lerp(s.eye(), a);
    let target = s.target_fov(lp.arsenal.w == 2);
    look.fov += (target - look.fov) * (dt * 20.0).min(1.0);
    let sh = s.shake * s.shake * 0.035;
    let jitter = |k: f32| (((time.elapsed_secs() * 97.0 + k).sin() * 43758.547).fract() - 0.5) * sh;
    let roll = if s.slide > 0.0 { -0.06 } else { 0.0 } * s.crouch_t;
    t.translation = eye;
    t.rotation = Quat::from_euler(EulerRot::YXZ, look.yaw + jitter(1.3), look.pitch + jitter(7.1), roll);
    look.punch *= (1.0 - dt * 9.0).max(0.0);
    if let Projection::Perspective(p) = &mut *proj {
        p.fov = (look.fov + look.punch).to_radians();
    }
}

/// Con `ARENA_HOOK=segundos`: dispara el gancho en ese momento (para capturas de prueba).
fn test_hook(time: Res<Time<Real>>, mut latch: ResMut<Latch>, mut done: Local<bool>) {
    let at: f32 = std::env::var("ARENA_HOOK").ok().and_then(|v| v.parse().ok()).unwrap_or(f32::MAX);
    if !*done && time.elapsed_secs() > at {
        *done = true;
        latch.hook = true;
    }
}
