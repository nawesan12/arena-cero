//! Texturas procedurales: los mismos dibujos de canvas de la web (`02-texturas.js`,
//! `03-mapas.js`, `03b-ciudad.js`), hechos en CPU con un lienzo mínimo. Todas se repiten
//! sin costura (el dibujo da la vuelta por los bordes) y llevan mipmaps.

use arena_sim::rng::Rng;
use bevy::asset::RenderAssetUsages;
use bevy::image::{Image, ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor};
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

/// Color en espacio sRGB 0..1 con alfa.
#[derive(Clone, Copy)]
pub struct C(pub f32, pub f32, pub f32, pub f32);

pub fn hex(h: u32) -> C {
    C(((h >> 16) & 255) as f32 / 255.0, ((h >> 8) & 255) as f32 / 255.0, (h & 255) as f32 / 255.0, 1.0)
}

fn rgba(r: f32, g: f32, b: f32, a: f32) -> C {
    C(r / 255.0, g / 255.0, b / 255.0, a)
}

impl C {
    fn a(self, a: f32) -> C {
        C(self.0, self.1, self.2, a)
    }
}

pub struct Canvas {
    pub s: usize,
    px: Vec<[f32; 3]>,
}

impl Canvas {
    pub fn new(s: usize, bg: C) -> Self {
        Self { s, px: vec![[bg.0, bg.1, bg.2]; s * s] }
    }

    fn blend(&mut self, x: i32, y: i32, c: C) {
        let s = self.s as i32;
        let (x, y) = (x.rem_euclid(s), y.rem_euclid(s));
        let p = &mut self.px[(y * s + x) as usize];
        let a = c.3.clamp(0.0, 1.0);
        p[0] += (c.0 - p[0]) * a;
        p[1] += (c.1 - p[1]) * a;
        p[2] += (c.2 - p[2]) * a;
    }

    pub fn rect(&mut self, x: f32, y: f32, w: f32, h: f32, c: C) {
        let (x0, y0) = (x.round() as i32, y.round() as i32);
        let (x1, y1) = ((x + w).round() as i32, (y + h).round() as i32);
        for yy in y0..y1.max(y0 + 1) {
            for xx in x0..x1.max(x0 + 1) {
                self.blend(xx, yy, c);
            }
        }
    }

    /// Borde de rectángulo con el trazo centrado en la línea, como `strokeRect`.
    pub fn stroke_rect(&mut self, x: f32, y: f32, w: f32, h: f32, lw: f32, c: C) {
        let hw = lw / 2.0;
        self.rect(x - hw, y - hw, w + lw, lw, c);
        self.rect(x - hw, y + h - hw, w + lw, lw, c);
        self.rect(x - hw, y + hw, lw, h - lw, c);
        self.rect(x + w - hw, y + hw, lw, h - lw, c);
    }

    pub fn line(&mut self, x0: f32, y0: f32, x1: f32, y1: f32, lw: f32, c: C) {
        let len = (x1 - x0).hypot(y1 - y0).max(1.0);
        let n = len.ceil() as i32;
        let r = (lw / 2.0).max(0.5);
        for i in 0..=n {
            let t = i as f32 / n as f32;
            let (x, y) = (x0 + (x1 - x0) * t, y0 + (y1 - y0) * t);
            // un sello por paso con alfa reducido para que las uniones no se vean más oscuras
            self.rect(x - r, y - r, r * 2.0, r * 2.0, c.a(c.3 * 0.6));
        }
    }

    pub fn radial(&mut self, cx: f32, cy: f32, r: f32, inner: C, outer: C) {
        let (x0, x1, y0, y1) = ((cx - r) as i32, (cx + r) as i32 + 1, (cy - r) as i32, (cy + r) as i32 + 1);
        for y in y0..y1 {
            for x in x0..x1 {
                let d = ((x as f32 + 0.5 - cx).hypot(y as f32 + 0.5 - cy) / r).min(1.0);
                let c = C(inner.0 + (outer.0 - inner.0) * d, inner.1 + (outer.1 - inner.1) * d, inner.2 + (outer.2 - inner.2) * d, inner.3 + (outer.3 - inner.3) * d);
                self.blend(x, y, c);
            }
        }
    }

    pub fn ellipse(&mut self, cx: f32, cy: f32, rx: f32, ry: f32, c: C) {
        for y in (cy - ry) as i32..=(cy + ry) as i32 {
            for x in (cx - rx) as i32..=(cx + rx) as i32 {
                let (dx, dy) = ((x as f32 - cx) / rx, (y as f32 - cy) / ry);
                if dx * dx + dy * dy <= 1.0 {
                    self.blend(x, y, c);
                }
            }
        }
    }

    /// Degradado vertical por paradas (posición 0..1, color).
    pub fn vgrad(&mut self, stops: &[(f32, C)]) {
        for y in 0..self.s {
            let t = (y as f32 + 0.5) / self.s as f32;
            let c = sample_stops(stops, t);
            for x in 0..self.s {
                self.px[y * self.s + x] = [c.0, c.1, c.2];
            }
        }
    }

    pub fn into_image(self) -> Image {
        to_image(self.s, self.px.iter().map(|p| [p[0], p[1], p[2], 1.0]).collect(), true, true)
    }
}

fn sample_stops(stops: &[(f32, C)], t: f32) -> C {
    let mut c = stops[0].1;
    for w in stops.windows(2) {
        let ((a, ca), (b, cb)) = (w[0], w[1]);
        if t >= a && t <= b {
            let k = if b > a { (t - a) / (b - a) } else { 0.0 };
            c = C(ca.0 + (cb.0 - ca.0) * k, ca.1 + (cb.1 - ca.1) * k, ca.2 + (cb.2 - ca.2) * k, 1.0);
        } else if t > b {
            c = cb;
        }
    }
    c
}

fn srgb_to_lin(v: f32) -> f32 {
    if v <= 0.04045 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) }
}
fn lin_to_srgb(v: f32) -> f32 {
    if v <= 0.0031308 { v * 12.92 } else { 1.055 * v.powf(1.0 / 2.4) - 0.055 }
}

/// Arma la imagen con todos sus mipmaps (promediados en espacio lineal) y sampler que repite.
pub fn to_image(s: usize, px: Vec<[f32; 4]>, srgb: bool, repeat: bool) -> Image {
    let mut data = Vec::new();
    let mut level: Vec<[f32; 4]> = if srgb { px.iter().map(|p| [srgb_to_lin(p[0]), srgb_to_lin(p[1]), srgb_to_lin(p[2]), p[3]]).collect() } else { px };
    let mut size = s;
    let mut mips = 0u32;
    loop {
        for p in &level {
            let enc = |v: f32| ((if srgb { lin_to_srgb(v) } else { v }).clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
            data.extend_from_slice(&[enc(p[0]), enc(p[1]), enc(p[2]), (p[3].clamp(0.0, 1.0) * 255.0 + 0.5) as u8]);
        }
        mips += 1;
        if size == 1 || !repeat {
            break;
        }
        let n = size / 2;
        let mut next = vec![[0.0; 4]; n * n];
        for y in 0..n {
            for x in 0..n {
                let mut acc = [0.0; 4];
                for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                    let p = level[(y * 2 + dy) * size + x * 2 + dx];
                    for k in 0..4 {
                        acc[k] += p[k] * 0.25;
                    }
                }
                next[y * n + x] = acc;
            }
        }
        level = next;
        size = n;
    }
    let format = if srgb { TextureFormat::Rgba8UnormSrgb } else { TextureFormat::Rgba8Unorm };
    let mut img = Image::new_uninit(Extent3d { width: s as u32, height: s as u32, depth_or_array_layers: 1 }, TextureDimension::D2, format, RenderAssetUsages::RENDER_WORLD);
    img.texture_descriptor.mip_level_count = mips;
    img.data = Some(data);
    let mode = if repeat { ImageAddressMode::Repeat } else { ImageAddressMode::ClampToEdge };
    img.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: mode,
        address_mode_v: mode,
        mag_filter: ImageFilterMode::Linear,
        min_filter: ImageFilterMode::Linear,
        mipmap_filter: ImageFilterMode::Linear,
        anisotropy_clamp: if repeat { 8 } else { 1 },
        ..Default::default()
    });
    img
}

/* ---------- las texturas del juego ---------- */

pub fn floor() -> Image {
    let s = 512;
    let mut r = Rng::new(1);
    let mut g = Canvas::new(s, hex(0x2b3134));
    for _ in 0..9000 {
        let v = (30.0 + r.f() * 30.0).floor();
        let (x, y) = (r.f() * s as f32, r.f() * s as f32);
        g.rect(x, y, 2.0, 2.0, rgba(v, v + 4.0, v + 6.0, 0.35));
    }
    g.stroke_rect(0.0, 0.0, s as f32, s as f32, 3.0, rgba(0.0, 0.0, 0.0, 0.55));
    let h = s as f32 / 2.0;
    g.rect(h - 0.5, 0.0, 1.0, s as f32, rgba(255.0, 176.0, 46.0, 0.10));
    g.rect(0.0, h - 0.5, s as f32, 1.0, rgba(255.0, 176.0, 46.0, 0.10));
    g.into_image()
}

pub fn crate_() -> Image {
    let s = 256;
    let sf = s as f32;
    let mut r = Rng::new(2);
    let mut g = Canvas::new(s, hex(0x463b2c));
    for _ in 0..2500 {
        let a = r.f() * 0.25;
        let (x, y) = (r.f() * sf, r.f() * sf);
        g.rect(x, y, 3.0, 1.0, rgba(0.0, 0.0, 0.0, a));
    }
    g.stroke_rect(7.0, 7.0, sf - 14.0, sf - 14.0, 14.0, hex(0x2a2219));
    // franja de peligro abajo
    for y in (s - 46)..(s - 14) {
        for x in 14..(s - 14) {
            if (x + y) % 28 < 14 {
                g.blend(x as i32, y as i32, hex(0xd99a2b));
            }
        }
    }
    g.into_image()
}

pub fn concrete() -> Image {
    let s = 256;
    let sf = s as f32;
    let mut r = Rng::new(3);
    let mut g = Canvas::new(s, hex(0x3a4246));
    for _ in 0..4000 {
        let v = (45.0 + r.f() * 30.0).floor();
        let (x, y) = (r.f() * sf, r.f() * sf);
        g.rect(x, y, 2.0, 2.0, rgba(v, v + 5.0, v + 8.0, 0.4));
    }
    g.rect(0.0, sf - 10.0, sf, 10.0, rgba(0.0, 0.0, 0.0, 0.25));
    g.into_image()
}

pub fn corrugated() -> Image {
    let s = 256;
    let sf = s as f32;
    let mut r = Rng::new(4);
    let mut g = Canvas::new(s, hex(0xd8d8d8));
    for x in 0..s {
        let t = (x % 16) as f32 / 16.0;
        let c = sample_stops(&[(0.0, hex(0x9a9a9a)), (0.5, hex(0xf2f2f2)), (1.0, hex(0x8a8a8a))], t);
        g.rect(x as f32, 0.0, 1.0, sf, c);
    }
    for _ in 0..1800 {
        let a = r.f() * 0.25;
        let (x, y, w) = (r.f() * sf, r.f() * sf, 2.0 + r.f() * 6.0);
        g.rect(x, y, w, 2.0, rgba(60.0, 30.0, 10.0, a));
    }
    g.stroke_rect(4.0, 4.0, sf - 8.0, sf - 8.0, 8.0, hex(0x555555));
    g.into_image()
}

pub fn lava() -> Image {
    let s = 256;
    let sf = s as f32;
    let mut r = Rng::new(5);
    let mut g = Canvas::new(s, hex(0xc2360c));
    for _ in 0..260 {
        let (x, y, rad) = (r.f() * sf, r.f() * sf, 6.0 + r.f() * 26.0);
        let inner = if r.next() < 0.5 { rgba(255.0, 214.0, 90.0, 0.9) } else { rgba(255.0, 140.0, 30.0, 0.8) };
        g.radial(x, y, rad, inner, rgba(160.0, 30.0, 0.0, 0.0));
    }
    for _ in 0..40 {
        let lw = 2.0 + r.f() * 4.0;
        let (mut x, mut y) = (r.f() * sf, r.f() * sf);
        for _ in 0..6 {
            let (nx, ny) = (x + (r.f() - 0.5) * 60.0, y + (r.f() - 0.5) * 60.0);
            g.line(x, y, nx, ny, lw, rgba(40.0, 8.0, 0.0, 0.55));
            (x, y) = (nx, ny);
        }
    }
    g.into_image()
}

pub fn windows() -> Image {
    let s = 128;
    let mut r = Rng::new(6);
    let mut g = Canvas::new(s, hex(0x0a0e18));
    let mut y = 4;
    while y < s {
        let mut x = 4;
        while x < s {
            let v = r.f();
            let c = if v < 0.22 { hex(0xffd27a) } else if v < 0.3 { hex(0x9fc7ff) } else { hex(0x141a28) };
            g.rect(x as f32, y as f32, 5.0, 6.0, c);
            x += 8;
        }
        y += 10;
    }
    g.into_image()
}

pub fn sand() -> Image {
    let s = 256;
    let sf = s as f32;
    let mut r = Rng::new(7);
    let mut g = Canvas::new(s, hex(0xd6bb86));
    for _ in 0..9000 {
        let c = if r.next() < 0.5 { rgba(150.0, 110.0, 60.0, 0.18) } else { rgba(255.0, 240.0, 200.0, 0.22) };
        let (x, y) = (r.f() * sf, r.f() * sf);
        g.rect(x, y, 2.0, 2.0, c);
    }
    let mut y = 8.0;
    while y < sf {
        let mut x = 0.0;
        while x < sf {
            let (y0, y1) = (y + (x * 0.05 + y).sin() * 3.0, y + ((x + 8.0) * 0.05 + y).sin() * 3.0);
            g.line(x, y0, x + 8.0, y1, 2.0, rgba(140.0, 100.0, 55.0, 0.18));
            x += 8.0;
        }
        y += 18.0;
    }
    g.into_image()
}

pub fn rock() -> Image {
    let s = 256;
    let sf = s as f32;
    let mut r = Rng::new(8);
    let bands = [0xb07a4a, 0xc99460, 0x9a6a40, 0xd2a372, 0xa87448, 0xbd8a58];
    let mut g = Canvas::new(s, hex(bands[0]));
    let mut y = 0.0;
    while y < sf {
        let h = 10.0 + r.f() * 28.0;
        let c = hex(bands[r.idx(bands.len())]);
        g.rect(0.0, y, sf, h, c);
        y += h;
    }
    for _ in 0..5000 {
        let a = r.f() * 0.2;
        let (x, y) = (r.f() * sf, r.f() * sf);
        g.rect(x, y, 3.0, 2.0, rgba(60.0, 35.0, 15.0, a));
    }
    for _ in 0..14 {
        let mut x = r.f() * sf;
        let mut yy = 0.0;
        while yy < sf {
            let nx = x + (r.f() - 0.5) * 14.0;
            g.line(x, yy, nx, yy + 20.0, 2.0, rgba(70.0, 40.0, 20.0, 0.35));
            x = nx;
            yy += 20.0;
        }
    }
    g.into_image()
}

pub fn asphalt() -> Image {
    let s = 512;
    let sf = s as f32;
    let mut r = Rng::new(9);
    let mut g = Canvas::new(s, hex(0x3b3f42));
    for _ in 0..16000 {
        let v = (40.0 + r.f() * 40.0).floor();
        let (x, y) = (r.f() * sf, r.f() * sf);
        g.rect(x, y, 2.0, 2.0, rgba(v, v + 2.0, v + 4.0, 0.45));
    }
    for _ in 0..10 {
        let (mut x, mut y) = (r.f() * sf, r.f() * sf);
        for _ in 0..6 {
            let (nx, ny) = (x + (r.f() - 0.5) * 50.0, y + (r.f() - 0.5) * 50.0);
            g.line(x, y, nx, ny, 1.5, rgba(20.0, 22.0, 24.0, 0.5));
            (x, y) = (nx, ny);
        }
    }
    g.into_image()
}

pub fn tile() -> Image {
    let s = 128;
    let sf = s as f32;
    let mut r = Rng::new(10);
    let mut g = Canvas::new(s, hex(0xbdb6aa));
    for _ in 0..1600 {
        let v = (150.0 + r.f() * 60.0).floor();
        let (x, y) = (r.f() * sf, r.f() * sf);
        g.rect(x, y, 2.0, 2.0, rgba(v, v - 4.0, v - 12.0, 0.4));
    }
    let h = sf / 2.0;
    for (x, y) in [(1.0, 1.0), (h + 1.0, 1.0), (1.0, h + 1.0), (h + 1.0, h + 1.0)] {
        g.stroke_rect(x, y, h - 2.0, h - 2.0, 2.0, rgba(90.0, 84.0, 74.0, 0.55));
    }
    g.into_image()
}

pub fn plaster() -> Image {
    let s = 128;
    let sf = s as f32;
    let mut r = Rng::new(11);
    let mut g = Canvas::new(s, hex(0xe8e4dc));
    for _ in 0..2600 {
        let v = (200.0 + r.f() * 55.0).floor();
        let (x, y) = (r.f() * sf, r.f() * sf);
        g.rect(x, y, 2.0, 2.0, rgba(v, v, v - 6.0, 0.5));
    }
    for _ in 0..6 {
        let a = 0.05 + r.f() * 0.08;
        let (y, h) = (r.f() * sf, 2.0 + r.f() * 6.0);
        g.rect(0.0, y, sf, h, rgba(120.0, 110.0, 95.0, a));
    }
    g.into_image()
}

pub fn glass() -> Image {
    let s = 128;
    let sf = s as f32;
    let mut g = Canvas::new(s, hex(0xd7dadc));
    for y in 10..(s - 16) {
        for x in 8..(s - 8) {
            let t = (x + y) as f32 / (2.0 * sf);
            let c = sample_stops(&[(0.0, hex(0x5b7c96)), (0.5, hex(0x8fb2c8)), (1.0, hex(0x46637a))], t);
            g.blend(x as i32, y as i32, c);
        }
    }
    g.rect(8.0, 10.0, (sf - 16.0) / 2.0 - 2.0, sf - 26.0, rgba(255.0, 255.0, 255.0, 0.18));
    g.rect(sf / 2.0 - 2.0, 10.0, 4.0, sf - 26.0, hex(0xc8cccf));
    g.into_image()
}

/// Cielo de día: degradado con nubes. `city` usa la paleta del Distrito Sur.
pub fn sky(city: bool) -> Image {
    let s = 256;
    let sf = s as f32;
    let mut r = Rng::new(if city { 12 } else { 13 });
    let mut g = Canvas::new(s, hex(0));
    if city {
        g.vgrad(&[(0.0, hex(0x4a86c4)), (0.4, hex(0x9cc4e2)), (0.5, hex(0xe9dcc4)), (0.56, hex(0xb7aa98)), (1.0, hex(0x6d675f))]);
    } else {
        g.vgrad(&[(0.0, hex(0x3d7fc4)), (0.42, hex(0x8cc0e6)), (0.5, hex(0xdcecf2)), (0.56, hex(0xead8b4)), (1.0, hex(0xc9ac7c))]);
    }
    for _ in 0..(if city { 16 } else { 18 }) {
        let (x, y, w) = (r.f() * sf, sf * (0.16 + r.f() * 0.2), 30.0 + r.f() * 60.0);
        // la nube que se sale por un costado vuelve por el otro (el lienzo da la vuelta)
        g.ellipse(x, y, w, w * if city { 0.16 } else { 0.18 }, rgba(255.0, 255.0, 255.0, if city { 0.4 } else { 0.35 }));
    }
    g.into_image()
}
