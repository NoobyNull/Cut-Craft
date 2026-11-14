//! Material classes: materials grouped by how hard they are to cut, softest first, so a
//! bit's feeds and speeds set once for a class (Hardwoods) serve every material in it
//! (Maple, Oak, Walnut). A material with cutting data of its own still uses its own.

use serde::{Deserialize, Serialize};

/// How hard a material is to cut, softest first.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MaterialClass {
    /// MDF, foam, plastics, composites
    Composites,
    Acrylics,
    Softwoods,
    Hardwoods,
    /// Aluminium, brass, pewter, lead
    SoftMetals,
    /// Steel, stainless, titanium
    HardMetals,
}

impl MaterialClass {
    /// Softest first.
    pub const ALL: [Self; 6] = [
        Self::Composites,
        Self::Acrylics,
        Self::Softwoods,
        Self::Hardwoods,
        Self::SoftMetals,
        Self::HardMetals,
    ];

    /// The hardest, cut with when nothing says what the material is.
    pub const HARDEST: Self = Self::HardMetals;

    /// Its name, as tools keep their cutting data for it.
    pub fn name(self) -> &'static str {
        match self {
            Self::Composites => "Composites, MDF, Foam, Plastic",
            Self::Acrylics => "Acrylics",
            Self::Softwoods => "Softwoods",
            Self::Hardwoods => "Hardwoods",
            Self::SoftMetals => "Soft Metals",
            Self::HardMetals => "Hard Metals",
        }
    }

    /// Its number in the list, 1 the softest.
    pub fn number(self) -> usize {
        Self::ALL
            .iter()
            .position(|c| *c == self)
            .map_or(0, |i| i + 1)
    }

    /// The class named `name`: a class's own name, or a material known to be in one
    /// (`Maple`, `Pine`, `HDU Foam`, `6061 Aluminium`), whatever the case. `None` for a
    /// material it does not know.
    pub fn of(name: &str) -> Option<Self> {
        let lower = name.trim().to_lowercase();
        if lower.is_empty() {
            return None;
        }
        if let Some(class) = Self::ALL.iter().find(|c| c.name().to_lowercase() == lower) {
            return Some(*class);
        }
        let has = |words: &[&str]| words.iter().any(|w| lower.contains(w));
        // a sheet of wood plies: cut as softwood, whatever wood faces it
        if has(&["plywood", "ply wood"]) {
            return Some(Self::Softwoods);
        }
        // the hardest first, so "steel-faced MDF" is not taken for MDF alone
        [
            (
                Self::HardMetals,
                &["steel", "titanium", "inconel", "iron "][..],
            ),
            (
                Self::SoftMetals,
                &[
                    "alumin", "brass", "pewter", "lead", "copper", "bronze", "zinc", "metal",
                ],
            ),
            (
                Self::Hardwoods,
                &[
                    "hardwood", "maple", "oak", "walnut", "cherry", "mahogany", "ironwood", "ash",
                    "beech", "birch", "hickory", "teak", "jatoba", "padauk", "wenge",
                ],
            ),
            (
                Self::Softwoods,
                &[
                    "softwood", "pine", "bass", "cedar", "spruce", "fir", "balsa", "redwood",
                    "plywood",
                ],
            ),
            (Self::Acrylics, &["acrylic", "perspex", "plexiglas", "pmma"]),
            (
                Self::Composites,
                &[
                    "mdf",
                    "foam",
                    "hdu",
                    "plastic",
                    "hdpe",
                    "pvc",
                    "composite",
                    "laminate",
                    "particle",
                    "polycarbonate",
                    "delrin",
                    "acetal",
                    "corian",
                    "wax",
                ],
            ),
        ]
        .into_iter()
        .find(|(_, words)| has(words))
        .map(|(class, _)| class)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn materials_fall_in_matthews_six_classes_softest_first() {
        let of = |m| MaterialClass::of(m).map(MaterialClass::number);
        assert_eq!(
            [of("MDF"), of("HDU Foam"), of("Plastic"), of("MDF/Laminate")],
            [Some(1); 4]
        );
        assert_eq!(of("Acrylic (Black)"), Some(2));
        assert_eq!(
            [
                of("Pine"),
                of("Basswood"),
                of("SOFTWOOD"),
                of("Birch Plywood")
            ],
            [Some(3); 4]
        );
        assert_eq!(
            [of("Maple"), of("Mahogany"), of("Ironwood"), of("Hardwood")],
            [Some(4); 4]
        );
        assert_eq!(
            [of("Aluminium"), of("6061 aluminum"), of("Pewter")],
            [Some(5); 3]
        );
        assert_eq!([of("Steel"), of("Titanium")], [Some(6); 2]);
        assert_eq!(of("Hardwoods"), Some(4), "a class's own name");
        assert_eq!(of("Unobtainium"), None);
        assert_eq!(of(""), None);
        assert_eq!(MaterialClass::HARDEST.number(), 6);
    }
}
