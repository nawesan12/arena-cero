//! Economía y gadgets (nuevo): créditos, cajas de botín en los edificios, tienda en los puntos
//! de reabastecimiento y tres gadgets: humo (tapa la vista de los enemigos), señuelo (hace ruido
//! y atrae soldados) y dron explorador (marca a todos los enemigos cercanos).

use crate::game::{Game, GameEvent, Snd};
use crate::maps::Level;
use crate::weapons::{Nade, NadeKind};
use glam::Vec3;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Gadget {
    Smoke,
    Decoy,
    Scout,
}

pub const GADGETS: [Gadget; 3] = [Gadget::Smoke, Gadget::Decoy, Gadget::Scout];

impl Gadget {
    pub fn name(self) -> &'static str {
        match self {
            Gadget::Smoke => "Humo",
            Gadget::Decoy => "Señuelo",
            Gadget::Scout => "Dron explorador",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum LootKind {
    Credits(u32),
    Ammo,
    Nades,
    Gadget(Gadget),
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Loot {
    pub id: u32,
    pub pos: Vec3,
    pub kind: LootKind,
    /// Caja rica (la que suelta el dron de carga).
    pub rich: bool,
}

#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Smoke {
    pub pos: Vec3,
    pub r: f32,
    pub life: f32,
}

#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Decoy {
    pub pos: Vec3,
    pub life: f32,
    pub beep: f32,
}

#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Scout {
    pub pos: Vec3,
    pub life: f32,
    pub tick: f32,
}

/// Lo que se puede comprar en la tienda.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ShopItem {
    Medkit,
    Nades,
    Smoke,
    Decoy,
    Scout,
    Damage,
    Mag,
    Armor,
    Launcher,
}

pub const SHOP: [ShopItem; 9] = [ShopItem::Medkit, ShopItem::Nades, ShopItem::Smoke, ShopItem::Decoy, ShopItem::Scout, ShopItem::Damage, ShopItem::Mag, ShopItem::Armor, ShopItem::Launcher];

impl ShopItem {
    pub fn price(self) -> u32 {
        match self {
            ShopItem::Medkit => 100,
            ShopItem::Nades => 150,
            ShopItem::Smoke => 180,
            ShopItem::Decoy => 160,
            ShopItem::Scout => 300,
            ShopItem::Damage => 600,
            ShopItem::Mag => 450,
            ShopItem::Armor => 500,
            ShopItem::Launcher => 800,
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            ShopItem::Medkit => "Botiquín",
            ShopItem::Nades => "Granadas ×2",
            ShopItem::Smoke => "Humo ×2",
            ShopItem::Decoy => "Señuelo ×2",
            ShopItem::Scout => "Dron explorador",
            ShopItem::Damage => "Munición pesada",
            ShopItem::Mag => "Cargadores grandes",
            ShopItem::Armor => "Placas de blindaje",
            ShopItem::Launcher => "Lanzagranadas",
        }
    }
    pub fn desc(self) -> &'static str {
        match self {
            ShopItem::Medkit => "Integridad al máximo.",
            ShopItem::Nades => "Dos granadas (hasta el máximo).",
            ShopItem::Smoke => "Nube que tapa la vista de los enemigos 12 s.",
            ShopItem::Decoy => "Hace ruido 10 s y atrae a los soldados.",
            ShopItem::Scout => "Marca a todos los enemigos en 50 m durante 12 s.",
            ShopItem::Damage => "+15% de daño con todas las armas.",
            ShopItem::Mag => "+25% de capacidad en todas las armas.",
            ShopItem::Armor => "+20 de integridad máxima.",
            ShopItem::Launcher => "Desbloquea el lanzagranadas sin forzar el arsenal.",
        }
    }
}

/// Créditos por baja según lo que era.
pub fn bounty(label: &str) -> u32 {
    match label {
        "COMANDANTE" => 300,
        "TORRETA" => 50,
        "BAJA SILENCIOSA" => 40,
        "BAJA" => 25,
        "VEHÍCULO" => 60,
        _ => 15,
    }
}

impl Game {
    /// Reparte cajas de botín en lugares al azar dentro de los edificios.
    pub(crate) fn place_loot(&mut self, level: &Level, n: usize) {
        let Some(c) = level.city.as_ref() else { return };
        let mut spots = c.loot_spots.clone();
        for _ in 0..n.min(spots.len()) {
            let i = self.rng.idx(spots.len());
            let pos = spots.swap_remove(i);
            let kind = match self.rng.idx(10) {
                0..=3 => LootKind::Credits(50 + self.rng.idx(4) as u32 * 25),
                4 | 5 => LootKind::Ammo,
                6 => LootKind::Nades,
                7 => LootKind::Gadget(Gadget::Smoke),
                8 => LootKind::Gadget(Gadget::Decoy),
                _ => LootKind::Gadget(Gadget::Scout),
            };
            let id = self.next_id();
            self.loot.push(Loot { id, pos, kind, rich: false });
        }
    }

    pub fn loot_near(&self, pos: Vec3, feet: f32) -> Option<usize> {
        self.loot.iter().position(|l| (l.pos.x - pos.x).hypot(l.pos.z - pos.z) < 1.8 && (l.pos.y - feet).abs() < 1.4)
    }

    /// Juntar una caja: créditos y gadgets acá; munición y granadas las aplica el cliente.
    pub(crate) fn take_loot(&mut self, i: usize) {
        let l = self.loot.swap_remove(i);
        self.events.push(GameEvent::Sound(Snd::Pickup));
        self.events.push(GameEvent::Burst { pos: l.pos + Vec3::Y * 0.5, color: 0x5dffb0, n: 12, speed: 3.0, life: 0.5 });
        let extra = if l.rich { 300 } else { 0 };
        if extra > 0 {
            self.add_credits(extra, "CARGAMENTO");
            self.gadgets[0] += 1;
            self.gadgets[2] += 1;
            self.events.push(GameEvent::Loot(LootKind::Nades));
        }
        match l.kind {
            LootKind::Credits(c) => self.add_credits(c, "BOTÍN"),
            LootKind::Gadget(g) => {
                self.gadgets[g as usize] += 1;
                self.banner(format!("{} +1", g.name().to_uppercase()), "Z cambia de gadget · V lo usa");
            }
            k => self.events.push(GameEvent::Loot(k)),
        }
    }

    pub fn add_credits(&mut self, n: u32, label: &'static str) {
        self.credits += n;
        self.events.push(GameEvent::Credits { amount: n, label });
    }

    /// ¿Hay un punto de reabastecimiento (y tienda) cerca?
    pub fn shop_near(&self, level: &Level, pos: Vec3) -> bool {
        self.supplies(level).iter().any(|(p, _)| (p.x - pos.x).hypot(p.z - pos.z) < 4.0 && (p.y - pos.y).abs() < 3.0)
    }

    /// Compra: descuenta créditos y aplica lo que es de la partida. Devuelve si se pudo.
    pub fn buy(&mut self, item: ShopItem) -> bool {
        if self.credits < item.price() {
            return false;
        }
        self.credits -= item.price();
        match item {
            ShopItem::Smoke => self.gadgets[0] += 2,
            ShopItem::Decoy => self.gadgets[1] += 2,
            ShopItem::Scout => self.gadgets[2] += 1,
            _ => {}
        }
        self.events.push(GameEvent::Sound(Snd::Pickup));
        true
    }

    /// Usar el gadget elegido desde los ojos del jugador.
    pub fn use_gadget(&mut self, eye: Vec3, dir: Vec3, vel: Vec3) {
        let g = GADGETS[self.gadget_sel % 3];
        let i = g as usize;
        if self.gadgets[i] == 0 {
            self.banner(format!("SIN {}", g.name().to_uppercase()), "Comprá en la tienda o buscá en las cajas");
            return;
        }
        self.gadgets[i] -= 1;
        match g {
            Gadget::Scout => {
                self.scouts.push(Scout { pos: eye + Vec3::Y * 0.5, life: 12.0, tick: 0.0 });
                self.events.push(GameEvent::Sound(Snd::Tag));
            }
            _ => {
                let kind = if g == Gadget::Smoke { NadeKind::Smoke } else { NadeKind::Decoy };
                self.nades.push(Nade { pos: eye + dir * 0.6, vel: dir * 16.0 + Vec3::Y * 3.0 + vel * 0.6, fuse: if g == Gadget::Smoke { 1.2 } else { 1.6 }, beep: 99.0, impact: false, gravity: 22.0, dmg: 0.0, enemy: false, kind });
                self.events.push(GameEvent::Sound(Snd::Toss));
            }
        }
    }

    /// Humo, señuelos y drones exploradores activos.
    pub(crate) fn step_gadgets(&mut self, dt: f32) {
        for s in &mut self.smokes {
            s.life -= dt;
            s.r = (s.r + dt * 6.0).min(6.5);
        }
        self.smokes.retain(|s| s.life > 0.0);
        let mut noises = Vec::new();
        for d in &mut self.decoys {
            d.life -= dt;
            d.beep -= dt;
            if d.beep <= 0.0 {
                d.beep = 1.5;
                noises.push(d.pos);
            }
        }
        self.decoys.retain(|d| d.life > 0.0);
        for p in noises {
            self.make_noise(p, 38.0);
            self.events.push(GameEvent::Sound(Snd::Decoy));
        }
        let mut scan = Vec::new();
        for s in &mut self.scouts {
            s.life -= dt;
            s.pos.y = (s.pos.y + dt * 6.0).min(26.0);
            s.tick -= dt;
            if s.tick <= 0.0 {
                s.tick = 0.5;
                scan.push(s.pos);
            }
        }
        self.scouts.retain(|s| s.life > 0.0);
        for p in scan {
            for e in &mut self.enemies {
                if e.alive() && (e.pos.x - p.x).hypot(e.pos.z - p.z) < 50.0 && !e.tagged {
                    e.tagged = true;
                }
            }
        }
    }

    /// Esferas de humo activas, para la percepción de la IA.
    pub fn smoke_spheres(&self) -> Vec<(Vec3, f32)> {
        self.smokes.iter().map(|s| (s.pos + Vec3::Y * 1.5, s.r)).collect()
    }
}

/// ¿El segmento a→b atraviesa alguna nube de humo?
pub fn smoke_blocks(smokes: &[(Vec3, f32)], a: Vec3, b: Vec3) -> bool {
    let d = b - a;
    let len2 = d.length_squared().max(1e-6);
    smokes.iter().any(|&(c, r)| {
        let t = ((c - a).dot(d) / len2).clamp(0.0, 1.0);
        (a + d * t).distance_squared(c) < r * r
    })
}
