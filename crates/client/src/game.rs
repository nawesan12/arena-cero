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
    /// Radio de la campaña: líneas en espera (quién, texto).
    pub radio: std::collections::VecDeque<(&'static str, String)>,
    /// Racha en pantalla: cantidad y edad.
    pub streak: Option<(u32, f32)>,
    /// Lista de bajas y puntos (texto, edad), la más nueva al final.
    pub feed: Vec<(String, f32)>,
    /// Carteles que llegaron mientras se mostraba otro.
    pub banner_queue: std::collections::VecDeque<(String, String)>,
}

impl UiFeed {
    /// Cartel grande. Si hay otro en pantalla hace cola (antes se pisaban); los repetidos se descartan.
    pub fn banner(&mut self, t: impl Into<String>, s: impl Into<String>) {
        let b = (t.into(), s.into());
        if self.banner_t > 0.6 && self.banner.as_ref() != Some(&b) {
            if !self.banner_queue.contains(&b) && self.banner_queue.len() < 4 {
                self.banner_queue.push_back(b);
            }
            return;
        }
        self.banner = Some(b);
        self.banner_t = 2.2;
    }

    /// Pasa al próximo cartel en espera (lo llama el HUD cada cuadro).
    pub fn next_banner(&mut self) {
        if self.banner_t <= 0.6 {
            if let Some(b) = self.banner_queue.pop_front() {
                self.banner = Some(b);
                self.banner_t = 2.2;
            }
        }
    }
}
