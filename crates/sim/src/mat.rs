/// Materiales del mundo. La simulación solo los etiqueta; el cliente decide cómo se ven
/// (color, textura procedural, rugosidad, UV en metros).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Mat {
    Wall,
    Pillar,
    Crate,
    Concrete,
    /// 0..4: óxido, petróleo, gris, oliva.
    Container(u8),
    Strip,
    StripW,
    StripR,
    Lava,
    Steel,
    Tank,
    Rock,
    Yellow,
    Tire,
    Water,
    Wood,
    /// Vidrio de la cabina de los camiones mineros.
    Glass,
    /// 0..3: skyline nocturno de la azotea.
    Windows(u8),
    Sidewalk,
    InFloor,
    Roof,
    /// 0..8: revoques de colores.
    Facade(u8),
    /// 0..4: fachadas vidriadas.
    Tower(u8),
    Sandbag,
    Jersey,
    Hedge,
    Trunk,
    Pole,
    Lamp,
    Line,
    LineY,
    /// 0..6
    Car(u8),
    CarGlass,
    Screen,
    Red,
    Mint,
    /// 0..4: toldos del mercado.
    Awning(u8),
    Statue,
}

pub const FACADES: u8 = 8;
pub const TOWERS: u8 = 4;
pub const CARS: u8 = 6;
pub const CONTAINERS: u8 = 4;
pub const AWNINGS: u8 = 4;
