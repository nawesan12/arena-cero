//! Mapas de arena del modo oleadas (puerto de `03-mapas.js`) y la ficha de cada mapa.
//! Donde la web usaba `Math.random` para decorado ahora hay un `Rng` con semilla fija,
//! para que todos los clientes de una partida vean lo mismo.

use crate::city::{self, CityData};
use crate::mat::Mat;
use crate::rng::Rng;
use crate::world::{CylAxis, Cylinder, RotBox, STEP, World};
use glam::Vec3;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum MapId {
    Deposito,
    Fundicion,
    Cantera,
    Azotea,
    Ciudad,
}

impl MapId {
    pub const ARENAS: [MapId; 4] = [MapId::Deposito, MapId::Fundicion, MapId::Cantera, MapId::Azotea];
    /// Recorrido de la campaña de oleadas.
    pub const CAMPAIGN: [MapId; 4] = [MapId::Deposito, MapId::Cantera, MapId::Fundicion, MapId::Azotea];
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FloorTex {
    Tiles,
    Sand,
    Asphalt,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Sky {
    /// Fondo liso del color de la niebla.
    Flat,
    /// Cielo de día con sol (cantera).
    Desert,
    /// Cielo de día con sol (ciudad).
    City,
}

#[derive(Clone, Debug)]
pub struct MapInfo {
    pub id: MapId,
    pub name: &'static str,
    pub size: &'static str,
    pub desc: &'static str,
    /// Medio lado.
    pub a: f32,
    pub start: [f32; 2],
    pub bg: u32,
    pub fog: [f32; 2],
    /// Cielo, suelo e intensidad de la luz hemisférica de la web.
    pub hemi: (u32, u32, f32),
    pub sun: (u32, f32),
    pub floor: u32,
    pub floor_tex: FloorTex,
    pub sky: Sky,
    pub city: bool,
}

pub fn info(id: MapId) -> MapInfo {
    match id {
        MapId::Deposito => MapInfo {
            id,
            name: "Depósito",
            size: "120 m",
            desc: "Complejo industrial con torres, contenedores y plaza elevada.",
            a: 60.0,
            start: [0.0, 24.0],
            bg: 0x0b1013,
            fog: [30.0, 120.0],
            hemi: (0x9fbccc, 0x2a2018, 0.75),
            sun: (0xffe2b8, 1.05),
            floor: 0xffffff,
            floor_tex: FloorTex::Tiles,
            sky: Sky::Flat,
            city: false,
        },
        MapId::Fundicion => MapInfo {
            id,
            name: "Fundición",
            size: "68 m",
            desc: "Arena cerrada y rápida. Las piletas de metal fundido queman.",
            a: 34.0,
            start: [0.0, 26.0],
            bg: 0x140b07,
            fog: [14.0, 66.0],
            hemi: (0xffb488, 0x2a1208, 0.62),
            sun: (0xffc89a, 0.8),
            floor: 0xc9a690,
            floor_tex: FloorTex::Tiles,
            sky: Sky::Flat,
            city: false,
        },
        MapId::Cantera => MapInfo {
            id,
            name: "Cantera",
            size: "100 m",
            desc: "Pleno sol en una cantera: meseta, agujas de roca y camiones mineros.",
            a: 50.0,
            start: [0.0, 40.0],
            bg: 0x9cc8e2,
            fog: [70.0, 200.0],
            hemi: (0xdcefff, 0xc8a878, 1.05),
            sun: (0xfff1d6, 1.7),
            floor: 0xffffff,
            floor_tex: FloorTex::Sand,
            sky: Sky::Desert,
            city: false,
        },
        MapId::Azotea => MapInfo {
            id,
            name: "Azotea",
            size: "92 m",
            desc: "Techo de un rascacielos de noche. Abierto y con poca cobertura.",
            a: 46.0,
            start: [0.0, 34.0],
            bg: 0x070b16,
            fog: [40.0, 190.0],
            hemi: (0x7f98d8, 0x101018, 0.62),
            sun: (0xa9c1ff, 0.75),
            floor: 0x8e9ab8,
            floor_tex: FloorTex::Tiles,
            sky: Sky::Flat,
            city: false,
        },
        MapId::Ciudad => MapInfo {
            id,
            name: "Distrito Sur",
            size: "590 m",
            desc: "Siete puestos en el centro y cuatro barrios alrededor: puerto, centro financiero, barrio jardín y el cerro. Autopistas, torres con pasarelas, campamentos y autos para manejar.",
            a: 296.0,
            start: [-132.0, 154.0],
            bg: 0xa9c6dc,
            fog: [120.0, 330.0],
            hemi: (0xe4f0ff, 0x8a7a66, 0.95),
            sun: (0xffe6c4, 1.55),
            floor: 0xffffff,
            floor_tex: FloorTex::Asphalt,
            sky: Sky::City,
            city: true,
        },
    }
}

/// Un mapa armado: ficha, geometría y (en la ciudad) los puestos.
pub struct Level {
    pub info: MapInfo,
    pub world: World,
    pub city: Option<CityData>,
}

impl Level {
    /// Posición del sol de la web (dirección de la luz principal).
    pub fn sun_pos(&self) -> Vec3 {
        let a = self.info.a;
        Vec3::new(a * 0.5, a * 0.85, a * 0.3)
    }
}

pub fn build_level(id: MapId) -> Level {
    let info = info(id);
    let mut w = World::new(info.a);
    let mut city = None;
    match id {
        MapId::Deposito => deposito(&mut w),
        MapId::Fundicion => fundicion(&mut w),
        MapId::Cantera => cantera(&mut w),
        MapId::Azotea => azotea(&mut w),
        MapId::Ciudad => city = Some(city::build(&mut w)),
    }
    Level { info, world: w, city }
}

/* Depósito: complejo industrial abierto */
fn deposito(w: &mut World) {
    w.add_perimeter(8.0, Some(6.4));
    let ph = STEP * 3.0;
    w.add_box(0.0, 0.0, 12.0, ph, 12.0, Mat::Concrete);
    w.add_stairs(0.0, -6.0, 0.0, -1.0, 5.0, ph, Mat::Concrete);
    w.add_stairs(0.0, 6.0, 0.0, 1.0, 5.0, ph, Mat::Concrete);
    w.add_stairs(-6.0, 0.0, -1.0, 0.0, 5.0, ph, Mat::Concrete);
    w.add_stairs(6.0, 0.0, 1.0, 0.0, 5.0, ph, Mat::Concrete);
    w.add_box(-4.2, -4.2, 2.0, ph + 1.1, 2.0, Mat::Crate);
    w.add_box(4.2, 4.2, 2.0, ph + 1.1, 2.0, Mat::Crate);
    w.add_strip(0.0, ph + 0.01, -5.95, 12.0, 0.02, 0.1, Mat::Strip);
    w.add_strip(0.0, ph + 0.01, 5.95, 12.0, 0.02, 0.1, Mat::Strip);
    for i in 0..8 {
        let a = i as f32 / 8.0 * std::f32::consts::TAU + std::f32::consts::PI / 8.0;
        let (x, z) = (a.cos() * 19.0, a.sin() * 19.0);
        w.add_box(x, z, 1.8, 10.0, 1.8, Mat::Pillar);
        w.add_strip(x, 3.4, z, 1.86, 0.12, 1.86, Mat::Strip);
    }
    let th = STEP * 10.0;
    for (sx, sz) in [(1.0, 1.0), (1.0, -1.0), (-1.0, 1.0), (-1.0, -1.0)] {
        let (cx, cz) = (sx * 38.0, sz * 38.0);
        w.add_box(cx, cz, 8.0, th, 8.0, Mat::Concrete);
        w.add_box(cx + sx * 3.8, cz, 0.4, th + 1.0, 8.0, Mat::Concrete);
        w.add_box(cx, cz + sz * 3.8, 8.0, th + 1.0, 0.4, Mat::Concrete);
        w.add_stairs(cx - sx * 4.0, cz - sz * 1.5, -sx, 0.0, 3.0, th, Mat::Concrete);
        w.add_strip(cx - sx * 3.95, th + 0.01, cz, 0.1, 0.02, 8.0, Mat::Strip);
        w.add_strip(cx, th + 0.01, cz - sz * 3.95, 8.0, 0.02, 0.1, Mat::Strip);
    }
    #[rustfmt::skip]
    let containers: [(f32, f32, bool, u32, usize); 20] = [
        (44.0, -14.0, false, 1, 0), (47.5, -14.0, false, 2, 1), (44.0, -4.0, false, 1, 2), (51.0, -4.0, false, 1, 3),
        (44.0, 8.0, false, 2, 1), (47.5, 16.0, false, 1, 0), (51.0, 8.0, false, 1, 2),
        (-44.0, 14.0, false, 1, 3), (-47.5, 14.0, false, 2, 0), (-44.0, 4.0, false, 1, 1), (-51.0, 4.0, false, 1, 2),
        (-44.0, -8.0, false, 2, 3), (-47.5, -16.0, false, 1, 1), (-51.0, -8.0, false, 1, 0),
        (14.0, 46.0, true, 1, 2), (-6.0, 50.0, true, 2, 1), (4.0, 44.0, true, 1, 3),
        (-14.0, -46.0, true, 1, 0), (6.0, -50.0, true, 2, 2), (-4.0, -44.0, true, 1, 1),
    ];
    for (x, z, ax, st, c) in containers {
        w.add_container(x, z, ax, st, c);
    }
    #[rustfmt::skip]
    let crates: [[f32; 5]; 28] = [
        [-10.,-26.,2.,2.,2.],[10.,26.,2.,2.,2.],[-26.,10.,2.,2.,2.],[26.,-10.,2.,2.,2.],
        [-28.,-28.,3.,1.2,3.],[28.,28.,3.,1.2,3.],[-28.,28.,2.,2.,2.],[28.,-28.,2.,2.,2.],
        [-16.,32.,4.,1.2,2.],[16.,-32.,4.,1.2,2.],[-30.,-6.,2.,2.,2.],[30.,6.,2.,2.,2.],
        [-12.,-12.,1.,1.1,4.],[12.,12.,1.,1.1,4.],[-12.,12.,4.,1.1,1.],[12.,-12.,4.,1.1,1.],
        [0.,-28.,6.,1.1,0.8],[0.,28.,6.,1.1,0.8],[-28.,0.,0.8,1.1,6.],[28.,0.,0.8,1.1,6.],
        [-22.,-40.,2.,2.,2.],[22.,40.,2.,2.,2.],[-40.,-24.,3.,1.2,3.],[40.,24.,3.,1.2,3.],
        [-34.,0.,2.,1.1,4.],[34.,0.,2.,1.1,4.],[0.,-38.,4.,1.1,2.],[0.,38.,4.,1.1,2.],
    ];
    for [x, z, ww, h, d] in crates {
        w.add_box(x, z, ww, h, d, Mat::Crate);
    }
}

/* Fundición: arena compacta con piletas de metal fundido */
fn fundicion(w: &mut World) {
    w.add_perimeter(9.0, Some(7.0));
    w.add_box(0.0, 0.0, 6.0, 7.0, 6.0, Mat::Pillar);
    w.add_visual(0.0, 9.0, 0.0, 3.0, 4.0, 3.0, Mat::Pillar, true);
    for y in [2.0, 4.6] {
        w.add_strip(0.0, y, 0.0, 6.06, 0.14, 6.06, Mat::Strip);
    }
    w.add_visual(0.0, 7.02, 0.0, 3.2, 0.04, 3.2, Mat::Lava, false);
    w.add_lava(-15.0, 0.0, 6.0, 10.0, 22.0);
    w.add_lava(15.0, 0.0, 6.0, 10.0, 22.0);
    w.add_lava(0.0, -16.0, 10.0, 5.0, 22.0);
    w.add_lava(0.0, 16.0, 10.0, 5.0, 22.0);
    for (x, z) in [(-9.0, -9.0), (9.0, -9.0), (-9.0, 9.0), (9.0, 9.0)] {
        w.add_box(x, z, 1.6, 9.0, 1.6, Mat::Steel);
        w.add_strip(x, 3.0, z, 1.66, 0.12, 1.66, Mat::Strip);
    }
    let ph = STEP * 8.0;
    w.add_box(-26.0, -26.0, 10.0, ph, 10.0, Mat::Steel);
    w.add_stairs(-21.0, -28.0, 1.0, 0.0, 3.0, ph, Mat::Steel);
    w.add_box(-26.0, -21.2, 10.0, ph + 1.0, 0.4, Mat::Steel);
    w.add_box(26.0, 26.0, 10.0, ph, 10.0, Mat::Steel);
    w.add_stairs(21.0, 28.0, -1.0, 0.0, 3.0, ph, Mat::Steel);
    w.add_box(26.0, 21.2, 10.0, ph + 1.0, 0.4, Mat::Steel);
    w.add_strip(-21.05, ph + 0.01, -26.0, 0.1, 0.02, 10.0, Mat::Strip);
    w.add_strip(21.05, ph + 0.01, 26.0, 0.1, 0.02, 10.0, Mat::Strip);
    for (x, z) in [(-24.0, -12.0), (24.0, -12.0), (-24.0, 12.0), (24.0, 12.0)] {
        w.add_box(x, z, 8.0, 1.1, 1.6, Mat::Steel);
        w.add_strip(x, 1.12, z, 8.0, 0.02, 0.3, Mat::Strip);
    }
    #[rustfmt::skip]
    let crates: [[f32; 5]; 12] = [
        [-6.,22.,2.,2.,2.],[6.,-22.,2.,2.,2.],[22.,-6.,2.,2.,2.],[-22.,6.,2.,2.,2.],[16.,24.,2.,2.,2.],[-16.,-24.,2.,2.,2.],
        [0.,22.,5.,1.1,0.8],[0.,-22.,5.,1.1,0.8],[26.,-26.,2.4,2.4,2.4],[-26.,26.,2.4,2.4,2.4],[23.5,-27.,2.,1.2,2.],[-23.5,27.,2.,1.2,2.],
    ];
    for [x, z, ww, h, d] in crates {
        w.add_box(x, z, ww, h, d, Mat::Crate);
    }
}

/* Azotea: techo de un rascacielos de noche, abierto */
fn azotea(w: &mut World) {
    let a = w.a;
    w.add_box(0.0, -a, a * 2.0 + 2.0, 1.3, 1.6, Mat::Concrete);
    w.add_box(0.0, a, a * 2.0 + 2.0, 1.3, 1.6, Mat::Concrete);
    w.add_box(-a, 0.0, 1.6, 1.3, a * 2.0 + 2.0, Mat::Concrete);
    w.add_box(a, 0.0, 1.6, 1.3, a * 2.0 + 2.0, Mat::Concrete);
    w.add_strip(0.0, 1.32, -a + 0.6, a * 2.0, 0.03, 0.2, Mat::StripR);
    w.add_strip(0.0, 1.32, a - 0.6, a * 2.0, 0.03, 0.2, Mat::StripR);
    w.add_strip(-a + 0.6, 1.32, 0.0, 0.2, 0.03, a * 2.0, Mat::StripR);
    w.add_strip(a - 0.6, 1.32, 0.0, 0.2, 0.03, a * 2.0, Mat::StripR);
    let ph = STEP * 3.0;
    w.add_box(0.0, 0.0, 14.0, ph, 14.0, Mat::Concrete);
    w.add_stairs(0.0, -7.0, 0.0, -1.0, 5.0, ph, Mat::Concrete);
    w.add_stairs(0.0, 7.0, 0.0, 1.0, 5.0, ph, Mat::Concrete);
    // helipuerto: una H blanca
    w.add_strip(-2.2, ph + 0.01, 0.0, 0.5, 0.02, 5.0, Mat::StripW);
    w.add_strip(2.2, ph + 0.01, 0.0, 0.5, 0.02, 5.0, Mat::StripW);
    w.add_strip(0.0, ph + 0.01, 0.0, 4.4, 0.02, 0.5, Mat::StripW);
    for [x, z, ww, d] in [[0.0, -6.8, 14.0, 0.2], [0.0, 6.8, 14.0, 0.2], [-6.8, 0.0, 0.2, 14.0], [6.8, 0.0, 0.2, 14.0]] {
        w.add_strip(x, ph + 0.01, z, ww, 0.02, d, Mat::Strip);
    }
    w.add_box(-24.0, -20.0, 6.0, 4.2, 5.0, Mat::Concrete);
    w.add_strip(-24.0, 1.2, -17.45, 1.6, 2.4, 0.1, Mat::Strip);
    w.add_box(24.0, 20.0, 5.0, 5.0, 5.0, Mat::Concrete);
    w.add_strip(24.0, 1.2, 17.45, 1.6, 2.4, 0.1, Mat::Strip);
    for (x, z) in [(-32.0, 28.0), (-26.0, 32.0), (32.0, -28.0)] {
        w.cylinders.push(Cylinder { center: Vec3::new(x, 2.6, z), radius: 1.6, length: 4.0, axis: CylAxis::Y, mat: Mat::Tank });
        w.add_visual(x, 0.3, z, 3.0, 0.6, 3.0, Mat::Steel, true);
        w.add_collider(x, z, 3.0, 4.6, 3.0, 0.0);
    }
    w.add_box(34.0, 34.0, 0.6, 16.0, 0.6, Mat::Steel);
    w.add_strip(34.0, 16.2, 34.0, 0.7, 0.4, 0.7, Mat::StripR);
    for x in [-20.0, -14.0, 14.0, 20.0] {
        for z in [-30.0, 30.0] {
            w.add_box(x, z, 3.0, 1.6, 2.0, Mat::Pillar);
        }
    }
    let mut z = -12.0;
    while z <= 12.0 {
        w.add_box(-34.0, z, 5.0, 0.9, 2.0, Mat::Steel);
        w.add_box(34.0, z, 5.0, 0.9, 2.0, Mat::Steel);
        z += 6.0;
    }
    for [x, z, ww, h, d] in [[-12.0, 12.0, 2.0, 2.0, 2.0], [12.0, -12.0, 2.0, 2.0, 2.0], [-14.0, -10.0, 4.0, 1.1, 1.0], [14.0, 10.0, 4.0, 1.1, 1.0], [0.0, -24.0, 6.0, 1.1, 0.8], [0.0, 24.0, 6.0, 1.1, 0.8]] {
        w.add_box(x, z, ww, h, d, Mat::Crate);
    }
    // skyline de fondo, sin colisión
    let mut r = Rng::new(46);
    for i in 0..48 {
        let ang = i as f32 / 48.0 * std::f32::consts::TAU + r.f() * 0.08;
        let rad = a + 22.0 + r.f() * 70.0;
        let ww = 10.0 + r.f() * 18.0;
        let h = 30.0 + r.f() * 80.0;
        let top = -24.0 + r.f() * 46.0;
        w.rot_boxes.push(RotBox {
            center: Vec3::new(ang.cos() * rad, top - h / 2.0, ang.sin() * rad),
            size: Vec3::new(ww, h, ww),
            yaw: -ang,
            mat: Mat::Windows((i % 3) as u8),
        });
    }
}

fn add_truck(w: &mut World, x: f32, z: f32, along_x: bool) {
    let (ww, d) = if along_x { (7.0, 3.2) } else { (3.2, 7.0) };
    w.add_box(x, z, ww, 3.0, d, Mat::Yellow);
    let cab = if along_x { [x + 2.6, z, 1.8, 1.6, 3.0] } else { [x, z + 2.6, 3.0, 1.6, 1.8] };
    w.add_visual(cab[0], 3.8, cab[1], cab[2], cab[3], cab[4], Mat::Yellow, true);
    w.add_visual(cab[0], 3.9, cab[1], cab[2] * 0.9, 0.7, cab[4] * 0.9, Mat::Glass, false);
    for a in [-1.0, 1.0] {
        for b in [-1.0, 1.0] {
            // rueda: el eje va a lo ancho del camión (en la web quedaban giradas 90°)
            let (cx, cz, axis) = if along_x { (x + a * 2.4, z + b * 1.75, CylAxis::Z) } else { (x + b * 1.75, z + a * 2.4, CylAxis::X) };
            w.cylinders.push(Cylinder { center: Vec3::new(cx, 0.75, cz), radius: 0.75, length: 0.5, axis, mat: Mat::Tire });
        }
    }
}

fn cantera(w: &mut World) {
    let a = w.a;
    let mut r = Rng::new(50);
    // paredes de roca con escalones de piedra
    for [x, z, ww, d] in [[0.0, -a, a * 2.0 + 2.0, 2.0], [0.0, a, a * 2.0 + 2.0, 2.0], [-a, 0.0, 2.0, a * 2.0 + 2.0], [a, 0.0, 2.0, a * 2.0 + 2.0]] {
        w.add_box(x, z, ww, 14.0, d, Mat::Rock);
    }
    for i in 0..26 {
        let side = i % 4;
        let t = r.signed() * (a - 6.0);
        let ww = 3.0 + r.f() * 5.0;
        let h = 3.0 + r.f() * 7.0;
        let d = 2.0 + r.f() * 3.0;
        let (x, z) = match side {
            0 => (t, -a + 1.0 + d / 2.0),
            1 => (t, a - 1.0 - d / 2.0),
            2 => (-a + 1.0 + d / 2.0, t),
            _ => (a - 1.0 - d / 2.0, t),
        };
        if (x * x + (z - 40.0) * (z - 40.0)).sqrt() < 8.0 {
            continue;
        }
        if side < 2 { w.add_box(x, z, ww, h, d, Mat::Rock) } else { w.add_box(x, z, d, h, ww, Mat::Rock) };
    }
    // meseta norte con dos escaleras
    let mh = STEP * 8.0;
    w.add_box(0.0, -36.0, 36.0, mh, 14.0, Mat::Rock);
    w.add_stairs(-10.0, -29.0, 0.0, 1.0, 4.0, mh, Mat::Rock);
    w.add_stairs(10.0, -29.0, 0.0, 1.0, 4.0, mh, Mat::Rock);
    w.add_box(-14.0, -32.0, 4.0, mh + 1.1, 1.0, Mat::Wood);
    w.add_box(14.0, -32.0, 4.0, mh + 1.1, 1.0, Mat::Wood);
    w.add_box(0.0, -31.0, 6.0, mh + 1.1, 1.0, Mat::Wood);
    // terraza sudeste
    let th = STEP * 5.0;
    w.add_box(34.0, 30.0, 16.0, th, 16.0, Mat::Rock);
    w.add_stairs(26.0, 30.0, -1.0, 0.0, 4.0, th, Mat::Rock);
    w.add_box(37.0, 27.0, 2.0, th + 2.0, 2.0, Mat::Crate);
    // agujas de roca
    for [x, z, ww, h] in [[-22.0, -8.0, 3.0, 8.0], [22.0, -12.0, 3.5, 10.0], [-8.0, 14.0, 2.6, 7.0], [14.0, 10.0, 3.0, 9.0], [-30.0, -24.0, 4.0, 6.0], [30.0, 2.0, 2.6, 8.0], [-18.0, 30.0, 3.0, 7.0], [4.0, -14.0, 2.4, 6.0]] {
        w.add_box(x, z, ww, h, ww, Mat::Rock);
        w.add_box(x + ww * 0.4, z - ww * 0.3, ww * 0.7, h * 0.55, ww * 0.8, Mat::Rock);
    }
    // camiones mineros
    add_truck(w, -14.0, 2.0, true);
    add_truck(w, 18.0, 22.0, false);
    add_truck(w, -34.0, 4.0, false);
    // laguna (decorativa)
    w.add_visual(-28.0, 0.04, 24.0, 12.0, 0.004, 9.0, Mat::Water, false);
    for [x, z, ww, d] in [[-28.0, 19.0, 13.0, 0.6], [-28.0, 29.0, 13.0, 0.6], [-34.8, 24.0, 0.6, 10.6]] {
        w.add_box(x, z, ww, 0.5, d, Mat::Rock);
    }
    // cajas y vigas de madera
    for [x, z, ww, h, d] in [[0.0, 26.0, 6.0, 1.1, 0.8], [-6.0, 32.0, 2.0, 2.0, 2.0], [8.0, 34.0, 2.0, 1.2, 2.0], [24.0, -24.0, 4.0, 1.1, 1.0], [-24.0, -16.0, 2.0, 2.0, 2.0], [36.0, -12.0, 2.0, 2.0, 2.0], [-40.0, -36.0, 3.0, 1.2, 3.0], [6.0, 2.0, 4.0, 1.1, 1.0], [-4.0, -2.0, 1.0, 1.1, 4.0]] {
        w.add_box(x, z, ww, h, d, Mat::Crate);
    }
    for (x, z) in [(-4.0, 40.0), (4.0, 40.0)] {
        w.add_strip(x, 0.03, z, 0.3, 0.02, 8.0, Mat::Wood);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn todos_los_mapas_se_arman() {
        for id in MapId::ARENAS.into_iter().chain([MapId::Ciudad]) {
            let l = build_level(id);
            assert!(l.world.colliders().count() > 20, "{:?}", id);
            assert!(!l.world.visuals.is_empty());
            assert_eq!(l.city.is_some(), l.info.city);
        }
    }

    #[test]
    fn determinista() {
        let a = build_level(MapId::Ciudad);
        let b = build_level(MapId::Ciudad);
        assert_eq!(a.world.colliders().count(), b.world.colliders().count());
        let fa: Vec<_> = a.world.colliders().take(500).map(|c| c.min.x.to_bits()).collect();
        let fb: Vec<_> = b.world.colliders().take(500).map(|c| c.min.x.to_bits()).collect();
        assert_eq!(fa, fb);
    }
}
