/// Materiales del mundo. La simulación solo los etiqueta; el cliente decide cómo se ven
/// (color, textura procedural, rugosidad, UV en metros).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
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
    /// Azulejos blancos del subte.
    Tile,
    /// Balasto de las vías.
    Gravel,
    /// 0..4: carteles publicitarios de las azoteas (se ven sin luz: de noche brillan).
    Billboard(u8),
    /// 0..4: neón del borde de algunas azoteas.
    Neon(u8),
    /// Bolardos y tachos metálicos de la calle.
    Bollard,
    /// Pasto (parques, bosque, cerros).
    Grass,
    /// Agua del río (opaca, a diferencia de `Water`).
    Sea,
    /// Asfalto (diagonales, encima del piso).
    Asphalt,
    /// Fachada con ventanas: estilo × 4 + color (0-3 clásica, 4-7 moderna, 8-11 ladrillo, 12 arcos de dock).
    Front(u8),
    /// Planta baja con vidriera o puerta (0..6).
    Shop(u8),
    /// Chapa acanalada pintada de El Cerro (0..6).
    Chapa(u8),
}

pub const FACADES: u8 = 8;
pub const TOWERS: u8 = 4;
pub const CARS: u8 = 6;
pub const CONTAINERS: u8 = 4;
pub const AWNINGS: u8 = 4;
