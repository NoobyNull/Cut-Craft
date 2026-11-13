//! The Materials Library: the materials jobs are cut from, each in a class
//! ([`MaterialClass`]) and with how hard it is, so a bit's feeds for a class can be placed
//! for each material by its hardness. Kept like the Tool Database: a starter list, and the
//! user's own materials added to it.

use crate::material_class::MaterialClass;
use crate::toolpath::{CuttingData, Tool};
use serde::{Deserialize, Serialize};

/// One material: what it is, its class, and how hard it is on its class's scale.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Material {
    pub name: String,
    pub class: MaterialClass,
    /// How hard it is, on its class's scale ([`MaterialClass::scale`]); 0 if not known
    #[serde(default)]
    pub hardness: f64,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub notes: String,
}

/// The library: every material, each class's softest first.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct MaterialLibrary {
    pub materials: Vec<Material>,
}

impl MaterialClass {
    /// The hardness scale its materials are listed on.
    pub fn scale(self) -> &'static str {
        match self {
            Self::Composites | Self::Softwoods | Self::Hardwoods => "Janka (lbf)",
            Self::Acrylics => "Rockwell M",
            Self::SoftMetals | Self::HardMetals => "Brinell (HB)",
        }
    }
}

/// The starter materials: (class, name, hardness), each class softest first. Hardnesses
/// are typical published figures, approximate: stock varies.
const STARTER: &[(MaterialClass, &str, f64)] = {
    use MaterialClass::{Acrylics, Composites, HardMetals, Hardwoods, SoftMetals, Softwoods};
    &[
        (Composites, "EPS Foam", 20.0),
        (Composites, "HDU Foam", 150.0),
        (Composites, "HDPE", 400.0),
        (Composites, "PVC", 600.0),
        (Composites, "MDF", 1100.0),
        (Acrylics, "Extruded Acrylic", 80.0),
        (Acrylics, "Cast Acrylic", 95.0),
        (Softwoods, "Balsa", 100.0),
        (Softwoods, "Western Red Cedar", 350.0),
        (Softwoods, "Eastern White Pine", 380.0),
        (Softwoods, "Basswood", 410.0),
        (Softwoods, "Spruce", 510.0),
        (Softwoods, "Birch Plywood", 600.0),
        (Softwoods, "Douglas Fir", 660.0),
        (Softwoods, "Southern Yellow Pine", 690.0),
        (Hardwoods, "Poplar", 540.0),
        (Hardwoods, "Mahogany", 800.0),
        (Hardwoods, "Cherry", 950.0),
        (Hardwoods, "Walnut", 1010.0),
        (Hardwoods, "Red Oak", 1290.0),
        (Hardwoods, "Ash", 1320.0),
        (Hardwoods, "White Oak", 1360.0),
        (Hardwoods, "Hard Maple", 1450.0),
        (Hardwoods, "Hickory", 1820.0),
        (Hardwoods, "Jatoba", 2690.0),
        (Hardwoods, "Ironwood", 3260.0),
        (SoftMetals, "Lead", 5.0),
        (SoftMetals, "Pewter", 10.0),
        (SoftMetals, "Copper", 40.0),
        (SoftMetals, "Brass", 80.0),
        (SoftMetals, "Aluminium 6061", 95.0),
        (SoftMetals, "Aluminium 7075", 150.0),
        (HardMetals, "Mild Steel", 120.0),
        (HardMetals, "Stainless Steel 304", 200.0),
        (HardMetals, "Titanium Grade 5", 334.0),
    ]
};

impl MaterialLibrary {
    /// The starter list.
    pub fn starter() -> Self {
        Self {
            materials: STARTER
                .iter()
                .map(|&(class, name, hardness)| Material {
                    name: name.to_owned(),
                    class,
                    hardness,
                    notes: String::new(),
                })
                .collect(),
        }
    }

    /// The material `name`, whatever the case.
    pub fn get(&self, name: &str) -> Option<&Material> {
        let name = name.trim();
        self.materials
            .iter()
            .find(|m| m.name.eq_ignore_ascii_case(name))
    }

    /// The class `name` is in: the library's material, else a class or material it can
    /// tell from the name ([`MaterialClass::of`]).
    pub fn class_of(&self, name: &str) -> Option<MaterialClass> {
        self.get(name)
            .map(|m| m.class)
            .or_else(|| MaterialClass::of(name))
    }

    /// `class`'s materials, softest first.
    pub fn in_class(&self, class: MaterialClass) -> Vec<&Material> {
        let mut found: Vec<&Material> =
            self.materials.iter().filter(|m| m.class == class).collect();
        found.sort_by(|a, b| a.hardness.total_cmp(&b.hardness));
        found
    }

    /// Where `name` sits in its class, from 0 (as soft as the softest listed) to 1 (as hard
    /// as the hardest): how far through the class's feed range it cuts. `None` when it is
    /// not listed with a hardness, or its class has no spread of hardnesses.
    pub fn place(&self, name: &str) -> Option<f64> {
        let m = self.get(name).filter(|m| m.hardness > 0.0)?;
        let known: Vec<f64> = self
            .in_class(m.class)
            .iter()
            .map(|m| m.hardness)
            .filter(|h| *h > 0.0)
            .collect();
        let (soft, hard) = (known.first()?, known.last()?);
        (hard > soft).then(|| ((m.hardness - soft) / (hard - soft)).clamp(0.0, 1.0))
    }

    /// The hardest of `names` (for a toolpath on several layers): by class, then by
    /// hardness in it. A name the library cannot place counts as the hardest class.
    pub fn hardest<'a>(&self, names: impl IntoIterator<Item = &'a str>) -> Option<&'a str> {
        names.into_iter().max_by(|a, b| {
            let rank = |n: &str| {
                let class = self.class_of(n).unwrap_or(MaterialClass::HARDEST);
                (class, self.get(n).map_or(f64::INFINITY, |m| m.hardness))
            };
            let (ca, ha) = rank(a);
            let (cb, hb) = rank(b);
            ca.cmp(&cb).then(ha.total_cmp(&hb))
        })
    }

    pub fn to_json(&self) -> Vec<u8> {
        serde_json::to_vec_pretty(self).expect("materials serialize")
    }

    pub fn from_json(bytes: &[u8]) -> Result<Self, serde_json::Error> {
        serde_json::from_slice(bytes)
    }
}

/// A bit's feeds and speeds for a material class: for its softest material and its
/// hardest, a material in between getting data between the two by its hardness.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ClassFeeds {
    pub class: MaterialClass,
    pub softest: CuttingData,
    pub hardest: CuttingData,
}

impl ClassFeeds {
    /// The data `t` of the way from the softest (0) to the hardest (1), for `material`.
    pub fn at(&self, t: f64, material: &str) -> CuttingData {
        let t = t.clamp(0.0, 1.0);
        let (a, b) = (&self.softest, &self.hardest);
        let mix = |x: f64, y: f64| x + (y - x) * t;
        CuttingData {
            material: material.to_owned(),
            pass_depth: mix(a.pass_depth, b.pass_depth),
            stepover: mix(a.stepover, b.stepover),
            finish_stepover: mix(a.finish_stepover, b.finish_stepover),
            feed_rate: mix(a.feed_rate, b.feed_rate),
            plunge_rate: mix(a.plunge_rate, b.plunge_rate),
            spindle_speed: mix(a.spindle_speed, b.spindle_speed),
        }
    }
}

impl Tool {
    /// The tool cutting `material` as the library knows it: the material's own data, else
    /// its class's feeds placed by its hardness (at the hard end when its hardness is not
    /// known), else as [`Tool::for_material`] (data for another material in its class, or
    /// the default).
    #[must_use]
    pub fn for_material_in(&self, material: &str, library: &MaterialLibrary) -> Tool {
        let own = self
            .materials
            .iter()
            .any(|m| !material.is_empty() && m.material.eq_ignore_ascii_case(material));
        let ranged = (!own)
            .then(|| {
                let class = library.class_of(material)?;
                let feeds = self.class_feeds.iter().find(|f| f.class == class)?;
                Some(feeds.at(library.place(material).unwrap_or(1.0), ""))
            })
            .flatten();
        match ranged {
            Some(data) => {
                let mut tool = self.clone();
                tool.set_cutting(data);
                tool.renamed(material);
                tool
            }
            None => self.for_material(material),
        }
    }
}

impl crate::document::Document {
    /// The active sheet's material, as Job Setup names it; empty when not named.
    pub fn job_material(&self) -> &str {
        self.sheets
            .get(self.active_sheet)
            .map_or("", |s| s.material.as_str())
    }

    /// The material a toolpath over the vectors `ids` cuts with: the hardest of their
    /// layers' materials, a layer naming none being the job's material. Where neither is
    /// named it counts as the hardest class, the safe side. Empty when nothing is named
    /// anywhere, so jobs made before materials keep cutting with the active material.
    pub fn cutting_material(
        &self,
        ids: impl IntoIterator<Item = crate::document::EntityId>,
        library: &MaterialLibrary,
    ) -> String {
        let job = self.job_material();
        let on: Vec<&str> = ids
            .into_iter()
            .map(|id| {
                let layer = self
                    .layer(self.layer_of(id))
                    .map_or("", |l| l.material.as_str());
                if layer.is_empty() { job } else { layer }
            })
            .collect();
        if on.iter().all(|n| n.is_empty()) {
            return job.to_owned();
        }
        let unnamed = MaterialClass::HARDEST.name();
        let names = on
            .into_iter()
            .map(|n| if n.is_empty() { unnamed } else { n });
        library.hardest(names).unwrap_or_default().to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_bit_s_class_feeds_are_placed_by_each_material_s_hardness() {
        let lib = MaterialLibrary::starter();
        let data = |material: &str, feed_rate, spindle_speed| CuttingData {
            material: material.into(),
            pass_depth: 3.0,
            stepover: 0.4,
            finish_stepover: 0.1,
            feed_rate,
            plunge_rate: 500.0,
            spindle_speed,
        };
        let tool = Tool {
            feed_rate: 999.0,
            class_feeds: vec![ClassFeeds {
                class: MaterialClass::Hardwoods,
                softest: data("", 3000.0, 18000.0),
                hardest: data("", 1000.0, 16000.0),
            }],
            materials: vec![data("Walnut", 2222.0, 17000.0)],
            ..Tool::default()
        };
        let feed = |m: &str| tool.for_material_in(m, &lib).feed_rate;
        assert_eq!(feed("Poplar"), 3000.0, "the softest hardwood");
        assert_eq!(feed("Ironwood"), 1000.0, "the hardest");
        let t = (1450.0 - 540.0) / (3260.0 - 540.0);
        assert!((feed("Hard Maple") - (3000.0 - 2000.0 * t)).abs() < 1e-9);
        assert_eq!(feed("Walnut"), 2222.0, "its own data wins");
        assert_eq!(
            feed("Some Hardwood"),
            1000.0,
            "unknown hardness: the hard end"
        );
        assert_eq!(feed("Pine"), 999.0, "no feeds for softwoods: the default");
    }

    #[test]
    fn the_starter_list_has_every_class_each_softest_first_by_hardness() {
        let lib = MaterialLibrary::starter();
        for class in MaterialClass::ALL {
            let names: Vec<&str> = lib
                .in_class(class)
                .iter()
                .map(|m| m.name.as_str())
                .collect();
            assert!(names.len() >= 2, "{class:?}");
            assert_eq!(lib.class_of(names[0]), Some(class));
        }
        let hardwoods: Vec<&str> = lib
            .in_class(MaterialClass::Hardwoods)
            .iter()
            .map(|m| m.name.as_str())
            .collect();
        assert_eq!(
            hardwoods.first(),
            Some(&"Poplar"),
            "not alphabetical: Janka"
        );
        assert_eq!(hardwoods.last(), Some(&"Ironwood"));
    }

    #[test]
    fn a_material_s_place_in_its_class_is_by_hardness() {
        let lib = MaterialLibrary::starter();
        assert_eq!(lib.place("Poplar"), Some(0.0));
        assert_eq!(lib.place("ironwood"), Some(1.0));
        let maple = lib.place("Hard Maple").unwrap();
        assert!((maple - (1450.0 - 540.0) / (3260.0 - 540.0)).abs() < 1e-9);
        assert_eq!(lib.place("Unobtainium"), None);
    }

    #[test]
    fn the_hardest_of_several_is_by_class_then_hardness_and_unknown_is_hardest() {
        let lib = MaterialLibrary::starter();
        assert_eq!(lib.hardest(["Pine", "Walnut", "MDF"]), Some("Walnut"));
        assert_eq!(lib.hardest(["Walnut", "Hickory"]), Some("Hickory"));
        assert_eq!(
            lib.hardest(["Walnut", "mystery board"]),
            Some("mystery board")
        );
        assert_eq!(lib.hardest([]), None);
    }

    #[test]
    fn the_library_saves_and_loads() {
        let lib = MaterialLibrary::starter();
        assert_eq!(MaterialLibrary::from_json(&lib.to_json()).unwrap(), lib);
    }

    #[test]
    fn a_toolpath_cuts_with_the_hardest_material_its_vectors_are_on() {
        let lib = MaterialLibrary::starter();
        let mut doc = crate::testing::two_layers();
        let ids: Vec<_> = doc.vectors.keys().copied().collect();
        // nothing named: as before materials, the active material
        assert_eq!(doc.cutting_material(ids.clone(), &lib), "");
        // the job's material
        doc.sheets[0].material = "Pine".into();
        assert_eq!(doc.cutting_material(ids.clone(), &lib), "Pine");
        // one vector on a layer of maple: the harder wins
        let second = doc.layers[1].id;
        doc.vector_layers.insert(ids[0], second);
        doc.layers[1].material = "Hard Maple".into();
        assert_eq!(doc.cutting_material(ids.clone(), &lib), "Hard Maple");
        // only the pine ones
        assert_eq!(doc.cutting_material(ids[1..].to_vec(), &lib), "Pine");
        // the job's material not named: the rest count as the hardest class
        doc.sheets[0].material.clear();
        assert_eq!(
            doc.cutting_material(ids.clone(), &lib),
            MaterialClass::HARDEST.name()
        );
        assert_eq!(doc.cutting_material([ids[0]], &lib), "Hard Maple");
    }

    #[test]
    fn a_bit_is_picked_for_the_material_named_else_the_active_one() {
        let lib = MaterialLibrary::starter();
        let mut db = crate::toolpath::ToolDatabase::default();
        let mut bit = Tool {
            name: "Bit".into(),
            feed_rate: 999.0,
            ..Tool::default()
        };
        let base = bit.cutting("");
        let data = |material: &str, feed_rate| CuttingData {
            material: material.into(),
            feed_rate,
            ..base.clone()
        };
        bit.class_feeds.push(ClassFeeds {
            class: MaterialClass::Hardwoods,
            softest: data("", 3000.0),
            hardest: data("", 1000.0),
        });
        bit.set_cutting(data("MDF", 2500.0));
        db.tools.push(bit);
        let feed = |db: &crate::toolpath::ToolDatabase, m: &str| {
            db.pick_in("Bit", m, &lib).unwrap().feed_rate
        };
        assert_eq!(feed(&db, "MDF"), 2500.0);
        let ironwood = feed(&db, "Ironwood");
        assert!(
            (ironwood - 1000.0).abs() < 1e-9,
            "the hardest of the class: {ironwood}"
        );
        assert_eq!(feed(&db, ""), 999.0, "no material anywhere: the bit's own");
        db.material = "MDF".into();
        assert_eq!(feed(&db, ""), 2500.0, "else the active material");
    }
}
