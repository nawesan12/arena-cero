//! Audio sintetizado, sin archivos: puerto de `07-audio.js` (efectos) y `08-musica-dinamica.js`.
//! La web armaba cada sonido en vivo con WebAudio (osciladores, ruido filtrado y envolventes).
//! Acá se renderiza cada efecto una vez al arrancar con el mismo sintetizador, y la música se
//! pre-renderiza en cuatro loops sincronizados (uno por nivel de tensión) que se funden entre sí.

use bevy::audio::{AddAudioSource, AudioSinkPlayback, ChannelCount, Decodable, SampleRate, Source, Volume};
use bevy::prelude::*;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

const SR: u32 = 44_100;
/// Ganancia general de efectos (`master.gain` en la web).
const MASTER: f32 = 0.45;

/* ---------- fuente PCM para Bevy ---------- */

#[derive(Asset, TypePath, Clone)]
pub struct Pcm {
    samples: Arc<[f32]>,
    looped: bool,
}

pub struct PcmDecoder {
    samples: Arc<[f32]>,
    i: usize,
    looped: bool,
}

impl Iterator for PcmDecoder {
    type Item = f32;
    fn next(&mut self) -> Option<f32> {
        if self.i >= self.samples.len() {
            if !self.looped || self.samples.is_empty() {
                return None;
            }
            self.i = 0;
        }
        let v = self.samples[self.i];
        self.i += 1;
        Some(v)
    }
}

impl Source for PcmDecoder {
    fn current_span_len(&self) -> Option<usize> {
        None
    }
    fn channels(&self) -> ChannelCount {
        ChannelCount::new(1).unwrap()
    }
    fn sample_rate(&self) -> SampleRate {
        SampleRate::new(SR).unwrap()
    }
    fn total_duration(&self) -> Option<Duration> {
        (!self.looped).then(|| Duration::from_secs_f32(self.samples.len() as f32 / SR as f32))
    }
}

impl Decodable for Pcm {
    type Decoder = PcmDecoder;
    fn decoder(&self) -> PcmDecoder {
        PcmDecoder { samples: self.samples.clone(), i: 0, looped: self.looped }
    }
}

/* ---------- sintetizador ---------- */

#[derive(Clone, Copy)]
enum Wave {
    Sine,
    Square,
    Saw,
    Tri,
}

#[derive(Clone, Copy)]
enum Filt {
    Band,
    Low,
    High,
}

struct Buf {
    s: Vec<f32>,
    rng: u32,
    /// Para los loops de música: lo que se pasa del final vuelve al principio.
    wrap: bool,
}

impl Buf {
    fn new(secs: f32) -> Self {
        Self { s: vec![0.0; (secs * SR as f32) as usize], rng: 0x1234_5678, wrap: false }
    }
    fn add(&mut self, i: usize, v: f32) {
        if self.wrap {
            let n = self.s.len();
            self.s[i % n] += v;
        } else {
            if i >= self.s.len() {
                self.s.resize(i + 1, 0.0);
            }
            self.s[i] += v;
        }
    }
    fn white(&mut self) -> f32 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 17;
        self.rng ^= self.rng << 5;
        (self.rng as f32 / u32::MAX as f32) * 2.0 - 1.0
    }

    /// `tone(type, f0, f1, len, vol)`: barrido exponencial de frecuencia y caída exponencial.
    fn tone(&mut self, at: f32, w: Wave, f0: f32, f1: f32, len: f32, vol: f32) {
        let n = ((len + 0.02) * SR as f32) as usize;
        let start = (at * SR as f32) as usize;
        let mut ph = 0.0f32;
        let f1 = f1.max(1.0);
        for k in 0..n {
            let t = k as f32 / SR as f32;
            let f = if t < len { f0 * (f1 / f0).powf(t / len) } else { f1 };
            ph = (ph + f / SR as f32).fract();
            let g = env_exp(vol, t, len);
            self.add(start + k, osc(w, ph) * g);
        }
    }

    /// `noise(freq, q, len, vol, type)`: ruido blanco filtrado con caída exponencial.
    fn noise(&mut self, at: f32, freq: f32, q: f32, len: f32, vol: f32, f: Filt) {
        let n = ((len + 0.02) * SR as f32) as usize;
        let start = (at * SR as f32) as usize;
        let mut bq = Biquad::new(f, freq, q);
        for k in 0..n {
            let t = k as f32 / SR as f32;
            let x = self.white();
            let g = env_exp(vol, t, len);
            let y = bq.run(x);
            self.add(start + k, y * g);
        }
    }

    /// Nota de la música: ataque lineal de 8 ms y caída exponencial, con pasabajos opcional.
    #[allow(clippy::too_many_arguments)]
    fn note(&mut self, at: f32, w: Wave, freq: f32, dur: f32, vol: f32, cutoff: Option<f32>) {
        let n = ((dur + 0.02) * SR as f32) as usize;
        let start = (at * SR as f32) as usize;
        let mut lp = cutoff.map(|c| Biquad::new(Filt::Low, c, 0.707));
        let mut ph = 0.0f32;
        for k in 0..n {
            let t = k as f32 / SR as f32;
            ph = (ph + freq / SR as f32).fract();
            let g = if t < 0.008 { vol * t / 0.008 } else { env_exp(vol, t - 0.008, dur - 0.008) };
            let mut y = osc(w, ph);
            if let Some(f) = lp.as_mut() {
                y = f.run(y);
            }
            self.add(start + k, y * g);
        }
    }

    fn kick(&mut self, at: f32) {
        let n = (0.27 * SR as f32) as usize;
        let start = (at * SR as f32) as usize;
        let mut ph = 0.0f32;
        for k in 0..n {
            let t = k as f32 / SR as f32;
            let f = if t < 0.18 { 140.0 * (42.0f32 / 140.0).powf(t / 0.18) } else { 42.0 };
            ph = (ph + f / SR as f32).fract();
            self.add(start + k, osc(Wave::Sine, ph) * env_exp(0.55, t, 0.25));
        }
    }

    fn into_pcm(mut self, gain: f32, looped: bool) -> Pcm {
        for v in &mut self.s {
            *v = (*v * gain).clamp(-1.0, 1.0);
        }
        Pcm { samples: self.s.into(), looped }
    }
}

/// `setValueAtTime(a)` + `exponentialRampToValueAtTime(0.0001, len)`.
fn env_exp(a: f32, t: f32, len: f32) -> f32 {
    if len <= 0.0 {
        return 0.0;
    }
    let k = (t / len).min(1.0);
    a * (0.0001f32 / a.max(1e-4)).powf(k).min(1.0) * if t > len { 0.0 } else { 1.0 }
}

fn osc(w: Wave, ph: f32) -> f32 {
    match w {
        Wave::Sine => (ph * std::f32::consts::TAU).sin(),
        Wave::Square => if ph < 0.5 { 1.0 } else { -1.0 },
        Wave::Saw => 2.0 * ph - 1.0,
        Wave::Tri => 1.0 - 4.0 * (ph - 0.5).abs(),
    }
}

/// Filtro biquad (recetas de RBJ, las mismas que usa WebAudio).
struct Biquad {
    b: [f32; 3],
    a: [f32; 2],
    x: [f32; 2],
    y: [f32; 2],
}

impl Biquad {
    fn new(f: Filt, freq: f32, q: f32) -> Self {
        let w0 = std::f32::consts::TAU * freq.min(SR as f32 * 0.45) / SR as f32;
        let (cs, sn) = (w0.cos(), w0.sin());
        let alpha = sn / (2.0 * q.max(0.1));
        let (b0, b1, b2) = match f {
            Filt::Band => (alpha, 0.0, -alpha),
            Filt::Low => ((1.0 - cs) / 2.0, 1.0 - cs, (1.0 - cs) / 2.0),
            Filt::High => ((1.0 + cs) / 2.0, -(1.0 + cs), (1.0 + cs) / 2.0),
        };
        let a0 = 1.0 + alpha;
        Self { b: [b0 / a0, b1 / a0, b2 / a0], a: [-2.0 * cs / a0, (1.0 - alpha) / a0], x: [0.0; 2], y: [0.0; 2] }
    }
    fn run(&mut self, x: f32) -> f32 {
        let y = self.b[0] * x + self.b[1] * self.x[0] + self.b[2] * self.x[1] - self.a[0] * self.y[0] - self.a[1] * self.y[1];
        self.x = [x, self.x[0]];
        self.y = [y, self.y[0]];
        y
    }
}

/* ---------- efectos ---------- */

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum S {
    Shot(u8),
    Siren,
    Whiz,
    EShot,
    EShotBig,
    AlertBeep,
    Tag,
    Takedown,
    Plant,
    Objective,
    Ricochet,
    Hit,
    Crit,
    Hook,
    HookMiss,
    Launch,
    Dash,
    Perk,
    Boss,
    Kill,
    Boom,
    Enemy,
    Hurt,
    Reload,
    Pickup,
    Empty,
    Aim,
    Swap,
    DJump,
    Slide,
    Toss,
    Beep,
    Crash,
    /// Bocina y rodar del tren del subte.
    Train,
    /// Clic de radio antes de que hable la operadora.
    Radio,
    /// Racha de bajas.
    Streak,
}

fn render(s: S) -> Pcm {
    use Filt::*;
    use Wave::*;
    let mut b = Buf::new(0.1);
    match s {
        S::Shot(0) => {
            b.noise(0.0, 1800.0, 0.8, 0.11, 0.5, Band);
            b.tone(0.0, Tri, 170.0, 45.0, 0.1, 0.35);
        }
        S::Shot(1) => {
            b.noise(0.0, 900.0, 0.5, 0.28, 0.8, Band);
            b.tone(0.0, Tri, 120.0, 30.0, 0.2, 0.5);
        }
        S::Shot(_) => {
            b.tone(0.0, Saw, 2200.0, 180.0, 0.35, 0.25);
            b.noise(0.0, 4000.0, 1.0, 0.2, 0.3, Band);
            b.tone(0.0, Sine, 90.0, 30.0, 0.3, 0.5);
        }
        S::Siren => {
            b.tone(0.0, Saw, 520.0, 880.0, 0.45, 0.12);
            b.tone(0.45, Saw, 880.0, 520.0, 0.45, 0.12);
            b.tone(0.9, Saw, 520.0, 880.0, 0.45, 0.1);
        }
        S::Whiz => b.noise(0.0, 3000.0, 4.0, 0.09, 0.06, Band),
        // los disparos enemigos se renderizan a volumen 1 y se escalan con la distancia al reproducir
        S::EShot => {
            b.noise(0.0, 1500.0, 0.7, 0.09, 1.0, Band);
            b.tone(0.0, Tri, 200.0, 60.0, 0.08, 0.6);
        }
        S::EShotBig => {
            b.noise(0.0, 900.0, 0.7, 0.25, 1.0, Band);
            b.tone(0.0, Tri, 120.0, 60.0, 0.08, 0.6);
        }
        S::AlertBeep => {
            b.tone(0.0, Square, 1200.0, 1200.0, 0.06, 0.08);
            b.tone(0.09, Square, 1500.0, 1500.0, 0.06, 0.08);
        }
        S::Tag => b.tone(0.0, Sine, 1800.0, 2400.0, 0.06, 0.07),
        S::Takedown => {
            b.noise(0.0, 500.0, 0.8, 0.12, 0.3, Low);
            b.tone(0.0, Sine, 180.0, 60.0, 0.15, 0.3);
        }
        S::Plant => {
            b.tone(0.0, Square, 700.0, 700.0, 0.05, 0.08);
            b.tone(0.12, Square, 900.0, 900.0, 0.05, 0.08);
        }
        S::Objective => {
            b.tone(0.0, Sine, 520.0, 520.0, 0.18, 0.2);
            b.tone(0.16, Sine, 660.0, 660.0, 0.18, 0.2);
            b.tone(0.32, Sine, 880.0, 880.0, 0.3, 0.22);
        }
        S::Ricochet => b.tone(0.0, Sine, 2600.0, 1200.0, 0.08, 0.06),
        S::Hit => b.tone(0.0, Square, 950.0, 1500.0, 0.05, 0.07),
        S::Crit => {
            b.tone(0.0, Square, 1500.0, 2400.0, 0.07, 0.1);
            b.tone(0.0, Sine, 3000.0, 3000.0, 0.04, 0.05);
        }
        S::Hook => {
            b.noise(0.0, 3200.0, 0.8, 0.18, 0.25, Band);
            b.tone(0.0, Saw, 900.0, 260.0, 0.12, 0.12);
        }
        S::HookMiss => b.tone(0.0, Square, 500.0, 180.0, 0.1, 0.06),
        S::Launch => {
            b.tone(0.0, Square, 240.0, 80.0, 0.16, 0.2);
            b.noise(0.0, 700.0, 0.6, 0.22, 0.4, Band);
        }
        S::Dash => {
            b.noise(0.0, 1500.0, 0.5, 0.25, 0.3, Band);
            b.tone(0.0, Sine, 200.0, 700.0, 0.18, 0.2);
        }
        S::Perk => {
            b.tone(0.0, Sine, 400.0, 800.0, 0.3, 0.2);
            b.tone(0.12, Sine, 600.0, 1200.0, 0.3, 0.18);
        }
        S::Boss => {
            b.tone(0.0, Saw, 220.0, 60.0, 0.5, 0.3);
            b.noise(0.0, 400.0, 0.6, 0.4, 0.3, Low);
        }
        S::Kill => {
            b.noise(0.0, 500.0, 0.6, 0.5, 0.6, Low);
            b.tone(0.0, Sine, 140.0, 28.0, 0.45, 0.5);
        }
        S::Boom => {
            b.noise(0.0, 260.0, 0.5, 1.0, 1.0, Low);
            b.tone(0.0, Sine, 80.0, 20.0, 0.8, 0.8);
        }
        S::Enemy => b.tone(0.0, Saw, 640.0, 180.0, 0.16, 1.0),
        S::Hurt => {
            b.tone(0.0, Sine, 110.0, 40.0, 0.25, 0.6);
            b.noise(0.0, 300.0, 1.0, 0.15, 0.3, Low);
        }
        S::Reload => {
            b.tone(0.0, Square, 300.0, 280.0, 0.03, 0.1);
            b.tone(0.9, Square, 520.0, 500.0, 0.04, 0.12);
        }
        S::Pickup => b.tone(0.0, Sine, 520.0, 1300.0, 0.22, 0.25),
        S::Empty => b.tone(0.0, Square, 200.0, 190.0, 0.03, 0.08),
        S::Aim => b.noise(0.0, 2400.0, 2.0, 0.05, 0.06, Band),
        S::Swap => b.tone(0.0, Square, 400.0, 600.0, 0.05, 0.08),
        S::DJump => {
            b.tone(0.0, Sine, 300.0, 900.0, 0.14, 0.18);
            b.noise(0.0, 2600.0, 1.0, 0.12, 0.12, Band);
        }
        S::Slide => b.noise(0.0, 700.0, 0.4, 0.45, 0.25, Low),
        S::Toss => b.noise(0.0, 1200.0, 1.0, 0.12, 0.15, Band),
        S::Beep => b.tone(0.0, Sine, 1400.0, 1400.0, 0.04, 0.08),
        S::Crash => {
            b.noise(0.0, 400.0, 0.6, 0.5, 0.9, Low);
            b.noise(0.0, 2500.0, 0.8, 0.25, 0.3, Band);
            b.tone(0.0, Sine, 90.0, 30.0, 0.4, 0.6);
        }
        S::Train => {
            b.tone(0.0, Saw, 311.0, 311.0, 0.7, 0.07);
            b.tone(0.0, Saw, 392.0, 392.0, 0.7, 0.06);
            b.noise(0.0, 220.0, 0.6, 1.6, 0.35, Low);
        }
        S::Streak => {
            b.tone(0.0, Square, 660.0, 660.0, 0.07, 0.07);
            b.tone(0.07, Square, 880.0, 880.0, 0.07, 0.07);
            b.tone(0.14, Square, 1320.0, 1320.0, 0.14, 0.08);
        }
        S::Radio => {
            b.noise(0.0, 2600.0, 1.2, 0.05, 0.25, Band);
            b.tone(0.05, Square, 1650.0, 1650.0, 0.05, 0.05);
            b.tone(0.11, Square, 2100.0, 2100.0, 0.04, 0.05);
        }
    }
    b.into_pcm(MASTER, false)
}

const ALL: [S; 38] = [
    S::Shot(0), S::Shot(1), S::Shot(2), S::Siren, S::Whiz, S::EShot, S::EShotBig, S::AlertBeep, S::Tag, S::Takedown, S::Plant, S::Objective,
    S::Ricochet, S::Hit, S::Crit, S::Hook, S::HookMiss, S::Launch, S::Dash, S::Perk, S::Boss, S::Kill, S::Boom, S::Enemy, S::Hurt, S::Reload,
    S::Pickup, S::Empty, S::Aim, S::Swap, S::DJump, S::Slide, S::Toss, S::Beep, S::Crash, S::Train, S::Radio, S::Streak,
];

/// Motor: loop de 1 s con frecuencias enteras (fase continua al repetir). La velocidad de
/// reproducción sube un poco el tono según lo rápido que va el auto.
fn engine_loop() -> Pcm {
    // ronroneo grave y redondo: senoidales en 40 y 80 Hz con un pulso suave de cilindros,
    // y apenas de ruido muy filtrado (antes tenía sierra y cuadrada: chillaba)
    let n = SR as usize;
    let mut out = vec![0.0f32; n];
    let mut lp = Biquad::new(Filt::Low, 160.0, 0.7);
    let mut b = Buf::new(0.0);
    for (k, v) in out.iter_mut().enumerate() {
        let t = k as f32 / SR as f32;
        let tau = std::f32::consts::TAU;
        let base = (t * 40.0 * tau).sin() * 0.55 + (t * 80.0 * tau).sin() * 0.25;
        let pulse = 0.75 + 0.25 * (t * 20.0 * tau).sin();
        *v = base * pulse * 0.6 + lp.run(b.white()) * 0.12;
    }
    let fade = 400;
    for i in 0..fade {
        let k = i as f32 / fade as f32;
        out[i] = out[i] * k + out[n - fade + i] * (1.0 - k);
    }
    out.truncate(n - fade);
    Pcm { samples: out.into(), looped: true }
}

/* ---------- música ---------- */

fn mtof(m: f32) -> f32 {
    440.0 * 2f32.powf((m - 69.0) / 12.0)
}

/// La m · Fa · Sol · Mi
const PROG: [[f32; 3]; 4] = [[45.0, 48.0, 52.0], [41.0, 45.0, 48.0], [43.0, 47.0, 50.0], [40.0, 44.0, 47.0]];
const SIXTEENTH: f32 = 60.0 / 118.0 / 4.0;

/// Un loop de 64 semicorcheas (4 compases) para un nivel de tensión (0 calma … 3 jefe).
fn music_loop(level: u32) -> Pcm {
    use Wave::*;
    let mut b = Buf::new(64.0 * SIXTEENTH);
    b.wrap = true;
    for st in 0..64u32 {
        let t = st as f32 * SIXTEENTH;
        let ch = PROG[(st / 16) as usize % 4];
        let s16 = st % 16;
        let l = level as f32;
        if s16 == 0 {
            for n in ch {
                b.note(t, Tri, mtof(n + 12.0), if level > 0 { 1.8 } else { 2.4 }, if level > 0 { 0.03 } else { 0.05 }, None);
            }
        }
        if level == 0 {
            continue;
        }
        if s16 % 4 == 0 {
            b.kick(t);
        }
        if s16 % 2 == 0 {
            let oct = if s16 % 8 == 6 { 12.0 } else { 0.0 };
            b.note(t, Saw, mtof(ch[0] - 12.0 + oct), 0.14, 0.15, Some(480.0 + l * 200.0));
        }
        if level >= 2 {
            if s16 % 2 == 1 {
                b.noise(t, 7000.0, 0.7, 0.03, 0.07, Filt::High);
            }
            if s16 % 8 == 4 {
                b.noise(t, 1800.0, 0.7, 0.12, 0.2, Filt::Band);
            }
        }
        if level >= 3 && s16 % 2 == 0 {
            b.note(t, Square, mtof(ch[((s16 / 2) % 3) as usize] + 24.0), 0.1, 0.04, Some(2600.0));
        }
    }
    b.into_pcm(0.55, true)
}

/* ---------- plugin ---------- */

#[derive(Resource)]
pub struct Sounds {
    map: HashMap<S, Handle<Pcm>>,
    /// Efecto, volumen y, si tiene, de dónde viene (para el paneo).
    queue: Vec<(S, f32, Option<Vec3>)>,
    engine: Option<Handle<Pcm>>,
}

impl Sounds {
    pub fn engine(&self) -> Option<Handle<Pcm>> {
        self.engine.clone()
    }
    /// Reproduce un efecto (volumen relativo 1 = como en la web).
    pub fn play(&mut self, s: S) {
        self.queue.push((s, 1.0, None));
    }
    pub fn play_vol(&mut self, s: S, v: f32) {
        self.queue.push((s, v, None));
    }
    /// Con paneo: suena más del lado de donde viene. El volumen lo decide quien llama.
    pub fn play_at(&mut self, s: S, v: f32, at: Vec3) {
        self.queue.push((s, v, Some(at)));
    }
}

/// Nivel de tensión que pide el juego (0..3).
#[derive(Resource, Default)]
pub struct MusicLevel(pub u8);

#[derive(Resource)]
pub struct MusicVol(pub f32);

#[derive(Component)]
struct MusicLayer(u8);

pub struct AudioSynthPlugin;

impl Plugin for AudioSynthPlugin {
    fn build(&self, app: &mut App) {
        app.add_audio_source::<Pcm>()
            .init_resource::<MusicLevel>()
            .insert_resource(MusicVol(crate::config::get_f32("musica").unwrap_or(0.6)))
            .add_systems(Startup, setup_audio)
            .add_systems(Update, (play_queue, mix_music, mute_key));
    }
}

fn setup_audio(mut commands: Commands, mut pcm: ResMut<Assets<Pcm>>) {
    let t0 = std::time::Instant::now();
    let map = ALL.iter().map(|&s| (s, pcm.add(render(s)))).collect();
    commands.insert_resource(Sounds { map, queue: Vec::new(), engine: Some(pcm.add(engine_loop())) });
    for l in 0..4u8 {
        let h = pcm.add(music_loop(u32::from(l)));
        commands.spawn((AudioPlayer(h), PlaybackSettings::LOOP.with_volume(Volume::Linear(0.0)), MusicLayer(l)));
    }
    info!("audio sintetizado en {:.0} ms", t0.elapsed().as_secs_f32() * 1000.0);
}

/// Los efectos con posición se ponen a 0,4 m del oyente en la dirección de la fuente: así el
/// audio espacial solo panea (un oído entre 0,5 y 1) y no atenúa; la distancia ya está en el volumen.
fn play_queue(mut commands: Commands, mut snd: ResMut<Sounds>, cam: Query<&GlobalTransform, With<SpatialListener>>) {
    let q = std::mem::take(&mut snd.queue);
    let ear = cam.single().ok().map(|t| t.translation());
    for (s, v, at) in q {
        let Some(h) = snd.map.get(&s) else { continue };
        let settings = PlaybackSettings::DESPAWN.with_volume(Volume::Linear(v));
        match (at, ear) {
            (Some(p), Some(e)) if p.distance_squared(e) > 0.25 => {
                let pos = e + (p - e).normalize() * 0.4;
                commands.spawn((AudioPlayer(h.clone()), settings.with_spatial(true), Transform::from_translation(pos)));
            }
            _ => {
                commands.spawn((AudioPlayer(h.clone()), settings));
            }
        }
    }
}

/// Funde las capas: suena solo la del nivel pedido, con transición suave.
fn mix_music(time: Res<Time>, level: Res<MusicLevel>, vol: Res<MusicVol>, mut layers: Query<(&MusicLayer, &mut AudioSink)>, mut cur: Local<[f32; 4]>) {
    let k = (time.delta_secs() * 1.5).min(1.0);
    for (l, mut sink) in &mut layers {
        let i = l.0 as usize;
        let target = if l.0 == level.0 { 1.0 } else { 0.0 };
        cur[i] += (target - cur[i]) * k;
        sink.set_volume(Volume::Linear(cur[i] * vol.0));
    }
}

/// M: música sí / no (como en la web).
fn mute_key(key: Res<ButtonInput<KeyCode>>, mut vol: ResMut<MusicVol>) {
    if key.just_pressed(KeyCode::KeyM) {
        vol.0 = if vol.0 > 0.0 { 0.0 } else { 0.6 };
        crate::config::set("musica", vol.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn todos_los_efectos_suenan_y_no_saturan() {
        for s in ALL {
            let p = render(s);
            let peak = p.samples.iter().fold(0.0f32, |m, v| m.max(v.abs()));
            assert!(peak > 0.005, "{s:?} está mudo");
            assert!(p.samples.iter().all(|v| v.is_finite()), "{s:?}");
        }
    }

    #[test]
    fn loops_de_musica_del_mismo_largo() {
        let n: Vec<usize> = (0..4).map(|l| music_loop(l).samples.len()).collect();
        assert!(n.iter().all(|&x| x == n[0]));
    }
}
