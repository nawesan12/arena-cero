//! Simulación de Arena Cero: mundo, colisiones y movimiento del jugador.
//!
//! No depende de Bevy ni de nada gráfico: la usan el cliente (para jugar y, más adelante,
//! para predecir) y el servidor headless. Todo lo que genera es determinista a partir de
//! una semilla, así servidor y clientes arman el mismo mapa sin mandarlo por red.

pub mod campaign;
pub mod city;
pub mod director;
pub mod economy;
pub mod enemies;
pub mod game;
pub mod maps;
pub mod mat;
pub mod nav;
pub mod objectives;
pub mod pawn;
pub mod people;
pub mod perks;
pub mod player;
pub mod rng;
pub mod subway;
pub mod vehicle;
pub mod weapons;
pub mod world;

pub use glam::{Vec3, vec3};
pub use maps::{Level, MapId, MapInfo, build_level};
pub use mat::Mat;
pub use player::{PlayerEvent, PlayerInput, PlayerState};
pub use world::World;

/// Paso fijo de la simulación (60 Hz). El cliente interpola entre pasos para dibujar.
pub const TICK: f32 = 1.0 / 60.0;
