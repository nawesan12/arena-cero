//! La partida en el cliente: el `Game` de la simulación como recurso, más lo que la interfaz
//! necesita mostrar de sus eventos (cartel, marca de impacto, números de daño).

use arena_sim::game::{Game, Mark};
use bevy::prelude::*;

#[derive(Resource)]
pub struct GameRes(pub Game);

/// Lo que la interfaz muestra a partir de los eventos de la partida.
#[derive(Resource, Default)]
pub struct UiFeed {
    pub banner: Option<(String, String)>,
    pub banner_t: f32,
    pub mark: Option<Mark>,
    pub mark_t: f32,
    /// Números de daño: (posición, valor, crítico, edad).
    pub nums: Vec<(Vec3, f32, bool, f32)>,
}

impl UiFeed {
    pub fn banner(&mut self, t: impl Into<String>, s: impl Into<String>) {
        self.banner = Some((t.into(), s.into()));
        self.banner_t = 2.2;
    }
}
