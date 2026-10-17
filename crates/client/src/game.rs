//! La partida en el cliente: el `Game` de la simulación como recurso, más lo que la interfaz
//! necesita mostrar de sus eventos (cartel, marca de impacto, números de daño).

use arena_sim::game::{Game, Mark};
use bevy::prelude::*;

/// La partida y su generación (sube cada vez que se reinicia, para que la vista descarte lo viejo).
#[derive(Resource)]
pub struct GameRes(pub Game, pub u32);

/// Lo que la interfaz muestra a partir de los eventos de la partida.
#[derive(Resource, Default)]
pub struct UiFeed {
    pub banner: Option<(String, String)>,
    pub banner_t: f32,
    pub mark: Option<Mark>,
    pub mark_t: f32,
    /// Números de daño: (posición, valor, crítico, edad).
    pub nums: Vec<(Vec3, f32, bool, f32)>,
    /// "+120 BAJA" debajo de la mira.
    pub popup: Option<(u32, &'static str)>,
    pub popup_t: f32,
    /// "+25 ¢" junto a los créditos.
    pub credit: Option<(u32, &'static str)>,
    pub credit_t: f32,
}

impl UiFeed {
    pub fn banner(&mut self, t: impl Into<String>, s: impl Into<String>) {
        self.banner = Some((t.into(), s.into()));
        self.banner_t = 2.2;
    }
}
