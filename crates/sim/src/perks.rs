//! Mejoras (puerto de `PERKS` en `09-estado.js`): se elige una de tres después de cada objetivo
//! de la ciudad y de cada oleada despejada.

use crate::rng::Rng;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Perk {
    Hp,
    Dmg,
    Reload,
    Mag,
    Speed,
    Regen,
    Leech,
    Nade,
    Jump,
    Dash,
    Crit,
}

pub const ALL: [Perk; 11] = [Perk::Hp, Perk::Dmg, Perk::Reload, Perk::Mag, Perk::Speed, Perk::Regen, Perk::Leech, Perk::Nade, Perk::Jump, Perk::Dash, Perk::Crit];

impl Perk {
    pub fn name(self) -> &'static str {
        match self {
            Perk::Hp => "Blindaje",
            Perk::Dmg => "Munición perforante",
            Perk::Reload => "Manos rápidas",
            Perk::Mag => "Cargador extendido",
            Perk::Speed => "Servos",
            Perk::Regen => "Nanorreparación",
            Perk::Leech => "Absorción",
            Perk::Nade => "Bandolera",
            Perk::Jump => "Propulsores",
            Perk::Dash => "Impulso",
            Perk::Crit => "Puntería fina",
        }
    }
    pub fn desc(self) -> &'static str {
        match self {
            Perk::Hp => "+25 de integridad máxima y reparación completa.",
            Perk::Dmg => "+25% de daño con todas las armas.",
            Perk::Reload => "Recargás un 30% más rápido.",
            Perk::Mag => "+50% de capacidad en todas las armas.",
            Perk::Speed => "+12% de velocidad de movimiento.",
            Perk::Regen => "Regenerás integridad el doble de rápido.",
            Perk::Leech => "Cada baja repara 4 de integridad.",
            Perk::Nade => "+1 granada máxima y recarga completa.",
            Perk::Jump => "Un salto extra en el aire.",
            Perk::Dash => "El dash recarga un 40% más rápido.",
            Perk::Crit => "Los disparos al núcleo hacen ×3 en vez de ×2.",
        }
    }
    /// Solo se puede tomar una vez.
    pub fn once(self) -> bool {
        self == Perk::Crit
    }
}

/// Tres opciones al azar, sin repetir y sin las únicas ya tomadas.
pub fn offer(taken: &[Perk], rng: &mut Rng) -> Vec<Perk> {
    let mut pool: Vec<Perk> = ALL.iter().copied().filter(|p| !(p.once() && taken.contains(p))).collect();
    let mut out = Vec::new();
    while out.len() < 3 && !pool.is_empty() {
        out.push(pool.remove(rng.idx(pool.len())));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tres_distintas_y_sin_repetir_unicas() {
        let mut r = Rng::new(3);
        for _ in 0..50 {
            let o = offer(&[Perk::Crit], &mut r);
            assert_eq!(o.len(), 3);
            assert!(!o.contains(&Perk::Crit));
            assert!(o[0] != o[1] && o[1] != o[2] && o[0] != o[2]);
        }
    }
}
