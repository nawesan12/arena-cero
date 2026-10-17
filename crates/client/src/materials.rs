//! Cómo se ve cada `Mat`: color, textura, rugosidad y si la textura va en metros (world UV).
//! Valores de `03-mapas.js` y `03b-ciudad.js`.

use crate::textures;
use arena_sim::Mat;
use arena_sim::maps::FloorTex;
use bevy::math::Affine2;
use bevy::prelude::*;
use std::collections::HashMap;

pub struct Look {
    pub mat: Handle<StandardMaterial>,
    /// Metros que ocupa una repetición de la textura (None: UV 0..1 por cara).
    pub wuv: Option<f32>,
}

#[derive(Resource)]
pub struct Looks {
    map: HashMap<Mat, Look>,
    /// Materiales de grupo en blanco: el color de cada variante va en los vértices.
    groups: HashMap<Mat, Handle<StandardMaterial>>,
    pub foliage: Handle<StandardMaterial>,
    pub lava: Handle<StandardMaterial>,
    tex: Tex,
}

struct Tex {
    floor: Handle<Image>,
    crate_: Handle<Image>,
    concrete: Handle<Image>,
    corr: Handle<Image>,
    lava: Handle<Image>,
    windows: Handle<Image>,
    sand: Handle<Image>,
    rock: Handle<Image>,
    asphalt: Handle<Image>,
    tile: Handle<Image>,
    plaster: Handle<Image>,
    glass: Handle<Image>,
    brick: Handle<Image>,
    grass: Handle<Image>,
    fronts: [Handle<Image>; 4],
    shop: Handle<Image>,
    panel: Handle<Image>,
    billboards: [Handle<Image>; 4],
    pub sky_city: Handle<Image>,
    pub sky_desert: Handle<Image>,
}

fn c(h: u32) -> Color {
    Color::srgb_u8((h >> 16) as u8, (h >> 8) as u8, h as u8)
}

fn lit(color: u32, rough: f32, metal: f32) -> StandardMaterial {
    StandardMaterial { base_color: c(color), perceptual_roughness: rough, metallic: metal, ..default() }
}

fn tex(t: &Handle<Image>, color: u32, rough: f32, metal: f32) -> StandardMaterial {
    StandardMaterial { base_color_texture: Some(t.clone()), ..lit(color, rough, metal) }
}

fn unlit(color: u32) -> StandardMaterial {
    StandardMaterial { base_color: c(color), unlit: true, ..default() }
}

/// Grupo de material: variantes que comparten textura y terminación y solo cambian de color.
pub fn group(m: Mat) -> Mat {
    match m {
        Mat::Facade(i) => Mat::Facade(match i % 8 {
            3 | 6 => 3,
            1 | 5 => 1,
            _ => 0,
        }),
        Mat::Front(i) => Mat::Front((i / 4).min(3) * 4),
        Mat::Shop(_) => Mat::Shop(0),
        Mat::Chapa(_) => Mat::Chapa(0),
        Mat::Container(_) => Mat::Container(0),
        Mat::Tower(_) => Mat::Tower(0),
        Mat::Neon(_) | Mat::Strip | Mat::StripW | Mat::StripR | Mat::Lamp | Mat::Line | Mat::LineY | Mat::Screen | Mat::Red | Mat::Mint => Mat::StripW,
        // lisos con luz: el color va en los vértices (se pierde algo de brillo metálico en lo chico)
        Mat::Wall | Mat::Pillar | Mat::Steel | Mat::Tank | Mat::Yellow | Mat::Tire | Mat::Wood | Mat::Hedge | Mat::Trunk | Mat::Pole | Mat::Statue | Mat::Bollard | Mat::CarGlass | Mat::Car(_) | Mat::Awning(_) => Mat::Pole,
        m => m,
    }
}

/// ¿El material va por grupo (con color en los vértices)?
pub fn grouped(m: Mat) -> bool {
    group(m) != m || matches!(m, Mat::Facade(_) | Mat::Front(_) | Mat::Shop(_) | Mat::Chapa(_) | Mat::Awning(_) | Mat::Car(_) | Mat::Container(_) | Mat::Tower(_) | Mat::StripW | Mat::Pole)
}

const FACADE: [u32; 8] = [0xd8c3a5, 0xb7c2c6, 0xe0d2ad, 0xa99a8e, 0xc9b294, 0xcfd6d1, 0xc78f6f, 0x9fb0a6];
const TOWER: [u32; 4] = [0xc3d2dc, 0xd9cdb5, 0xaab6c2, 0xe0e4e6];
const CONTAINER: [u32; 4] = [0x8a4528, 0x2a5a63, 0x4a5662, 0x6e6a3a];
const CAR: [u32; 6] = [0xb83a30, 0x2f5f8f, 0xd8d4c8, 0x2d2f31, 0x6f7f4a, 0xd9a43a];
const AWNING: [u32; 4] = [0xc0392b, 0x2e86ab, 0xe0a92a, 0x3f8f5a];

impl Looks {
    pub fn new(images: &mut Assets<Image>, mats: &mut Assets<StandardMaterial>) -> Self {
        let tex = Tex {
            floor: images.add(textures::floor()),
            crate_: images.add(textures::crate_()),
            concrete: images.add(textures::concrete()),
            corr: images.add(textures::corrugated()),
            lava: images.add(textures::lava()),
            windows: images.add(textures::windows()),
            sand: images.add(textures::sand()),
            rock: images.add(textures::rock()),
            asphalt: images.add(textures::asphalt()),
            tile: images.add(textures::tile()),
            plaster: images.add(textures::plaster()),
            glass: images.add(textures::glass()),
            brick: images.add(textures::brick()),
            grass: images.add(textures::grass()),
            fronts: [0, 1, 2, 3].map(|k| images.add(textures::front(k))),
            shop: images.add(textures::shopfront()),
            panel: images.add(textures::panel()),
            billboards: [0, 1, 2, 3].map(|k| images.add(textures::billboard(k))),
            sky_city: images.add(textures::sky(true)),
            sky_desert: images.add(textures::sky(false)),
        };
        let foliage = mats.add(lit(0x4d7d3a, 0.9, 0.0));
        let lava = mats.add(StandardMaterial { base_color_texture: Some(tex.lava.clone()), unlit: true, uv_transform: Affine2::from_scale(Vec2::splat(2.0)), ..default() });
        Self { map: HashMap::new(), groups: HashMap::new(), foliage, lava, tex }
    }

    pub fn sky(&self, city: bool) -> Handle<Image> {
        if city { self.tex.sky_city.clone() } else { self.tex.sky_desert.clone() }
    }

    pub fn floor(&self, kind: FloorTex, color: u32, a: f32, mats: &mut Assets<StandardMaterial>) -> Handle<StandardMaterial> {
        let t = match kind {
            FloorTex::Tiles => &self.tex.floor,
            FloorTex::Sand => &self.tex.sand,
            FloorTex::Asphalt => &self.tex.asphalt,
        };
        mats.add(StandardMaterial { uv_transform: Affine2::from_scale(Vec2::splat(a / 2.0)), ..tex(t, color, 0.92, 0.0) })
    }

    /// Material del grupo de `m` (blanco: el color de la variante viene en los vértices). Así las
    /// 8 fachadas, los 16 frentes, los locales, la chapa, los autos, etc. se dibujan de una vez por zona
    /// en vez de una vez por color.
    pub fn get_group(&mut self, m: Mat, mats: &mut Assets<StandardMaterial>) -> Handle<StandardMaterial> {
        let g = group(m);
        if let Some(h) = self.groups.get(&g) {
            return h.clone();
        }
        let (mut sm, _) = self.describe(g);
        sm.base_color = Color::WHITE;
        if g == Mat::Pole {
            // terminación neutra para todo lo liso (madera, metal, plantas, autos)
            sm.metallic = 0.15;
            sm.perceptual_roughness = 0.7;
        }
        let h = mats.add(sm);
        self.groups.insert(g, h.clone());
        h
    }

    /// Color lineal de la variante (para los vértices) y UV en metros.
    pub fn tint(&self, m: Mat) -> ([f32; 4], Option<f32>) {
        let (sm, wuv) = self.describe(m);
        let c = sm.base_color.to_linear();
        ([c.red, c.green, c.blue, c.alpha], wuv)
    }

    pub fn get(&mut self, m: Mat, mats: &mut Assets<StandardMaterial>) -> &Look {
        if !self.map.contains_key(&m) {
            let (sm, wuv) = self.describe(m);
            let look = Look { mat: if m == Mat::Lava { self.lava.clone() } else { mats.add(sm) }, wuv };
            self.map.insert(m, look);
        }
        &self.map[&m]
    }

    fn describe(&self, m: Mat) -> (StandardMaterial, Option<f32>) {
        let t = &self.tex;
        match m {
            Mat::Wall => (lit(0x20272b, 0.85, 0.0), None),
            Mat::Pillar => (lit(0x2a3236, 0.7, 0.2), None),
            Mat::Crate => (tex(&t.crate_, 0xffffff, 0.8, 0.0), None),
            Mat::Concrete => (tex(&t.concrete, 0xffffff, 0.9, 0.0), None),
            Mat::Container(i) => (tex(&t.corr, CONTAINER[i as usize % 4], 0.6, 0.35), None),
            Mat::Strip => (unlit(0xffb02e), None),
            Mat::StripW => (unlit(0xdfe8e4), None),
            Mat::StripR => (unlit(0xff3d5a), None),
            Mat::Lava => (StandardMaterial::default(), None),
            Mat::Steel => (lit(0x3b4248, 0.4, 0.7), None),
            Mat::Tank => (lit(0x55606a, 0.5, 0.5), None),
            Mat::Rock => (tex(&t.rock, 0xffffff, 0.95, 0.0), None),
            Mat::Yellow => (lit(0xe0a92a, 0.55, 0.3), None),
            Mat::Tire => (lit(0x1a1a1a, 0.9, 0.0), None),
            Mat::Water => (StandardMaterial { alpha_mode: AlphaMode::Blend, base_color: c(0x3f8fae).with_alpha(0.85), ..lit(0x3f8fae, 0.1, 0.3) }, None),
            Mat::Wood => (lit(0x8a6440, 0.85, 0.0), None),
            Mat::Glass => (StandardMaterial { alpha_mode: AlphaMode::Blend, base_color: c(0x9fe9ff).with_alpha(0.16), ..lit(0x9fe9ff, 0.0, 1.0) }, None),
            Mat::Windows(r) => {
                let r = f32::from(r) + 1.0;
                (StandardMaterial { base_color_texture: Some(t.windows.clone()), unlit: true, uv_transform: Affine2::from_scale(Vec2::new(r, r * 2.0)), ..default() }, None)
            }
            Mat::Sidewalk => (tex(&t.tile, 0xffffff, 0.92, 0.0), Some(2.4)),
            Mat::InFloor => (tex(&t.tile, 0xa8a092, 0.8, 0.0), Some(1.6)),
            Mat::Roof => (tex(&t.concrete, 0xb9b9b9, 0.95, 0.0), Some(4.0)),
            // fachadas variadas (antes las 8 eran el mismo revoque de otro color): dos de ladrillo,
            // dos de paneles de hormigón y el resto revoque
            Mat::Facade(i) => match i % 8 {
                3 => (tex(&t.brick, 0xf4ece2, 0.9, 0.0), Some(3.0)),
                6 => (tex(&t.brick, 0xe2cfc0, 0.9, 0.0), Some(3.0)),
                1 | 5 => (tex(&t.panel, FACADE[i as usize % 8], 0.85, 0.0), Some(3.0)),
                _ => (tex(&t.plaster, FACADE[i as usize % 8], 0.88, 0.0), Some(3.0)),
            },
            Mat::Tower(i) => (tex(&t.glass, TOWER[i as usize % 4], 0.3, 0.45), Some(3.4)),
            Mat::Sandbag => (tex(&t.plaster, 0x9a8660, 1.0, 0.0), Some(0.9)),
            Mat::Jersey => (tex(&t.concrete, 0xd6d2c8, 0.9, 0.0), Some(2.0)),
            Mat::Hedge => (lit(0x3f6a32, 1.0, 0.0), None),
            Mat::Trunk => (lit(0x5a4130, 1.0, 0.0), None),
            Mat::Pole => (lit(0x2c3236, 0.5, 0.6), None),
            Mat::Lamp => (unlit(0xfff2d0), None),
            Mat::Line => (unlit(0xe8e4d4), None),
            Mat::LineY => (unlit(0xe0b23a), None),
            Mat::Car(i) => (lit(CAR[i as usize % 6], 0.4, 0.5), None),
            Mat::CarGlass => (lit(0x1d2a33, 0.15, 0.8), None),
            Mat::Screen => (unlit(0x5fd8ff), None),
            Mat::Red => (unlit(0xff3d5a), None),
            Mat::Mint => (unlit(0x5dffb0), None),
            Mat::Awning(i) => (lit(AWNING[i as usize % 4], 0.8, 0.0), None),
            Mat::Statue => (lit(0x7a8a80, 0.45, 0.6), None),
            Mat::Tile => (tex(&t.tile, 0xe8eef0, 0.35, 0.0), Some(0.8)),
            Mat::Gravel => (tex(&t.rock, 0x6a645c, 1.0, 0.0), Some(1.5)),
            Mat::Billboard(k) => (StandardMaterial { base_color_texture: Some(t.billboards[k as usize % 4].clone()), unlit: true, ..default() }, None),
            Mat::Neon(k) => (unlit([0xff3df0, 0x3dfcff, 0xffd23d, 0x7dff5a][k as usize % 4]), None),
            Mat::Bollard => (lit(0x30363a, 0.45, 0.7), None),
            Mat::Grass => (tex(&t.grass, 0xffffff, 0.95, 0.0), Some(2.0)),
            Mat::Sea => (lit(0x2c6a80, 0.12, 0.05), None),
            Mat::Asphalt => (tex(&t.asphalt, 0xffffff, 0.92, 0.0), Some(4.0)),
            Mat::Front(i) => {
                let style = (i / 4).min(3);
                let tint = [[0xffffff, 0xf2e6d2, 0xe9dcd8, 0xe6e2da], [0xffffff, 0xe0e6ea, 0xece6dc, 0xd8dcd6], [0xffffff, 0xf4ece6, 0xe8dcd4, 0xfff6ee], [0xffffff; 4]][style as usize][i as usize % 4];
                (tex(&t.fronts[style as usize], tint, 0.85, 0.0), Some(3.4))
            }
            Mat::Shop(i) => (tex(&t.shop, [0xd94a3a, 0x3a8fd9, 0xe8c23a, 0x4ab06a, 0xe07a2a, 0xf0f0f0][i as usize % 6], 0.6, 0.0), Some(3.4)),
            Mat::Chapa(i) => (tex(&t.corr, [0xd94a3a, 0x3a8fd9, 0xe8c23a, 0x4ab06a, 0xe07a2a, 0x9a5ad0][i as usize % 6], 0.7, 0.2), Some(2.4)),
        }
    }
}
