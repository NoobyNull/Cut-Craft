//! Toolpaths ▸ Materials: a library of materials to show the job in. Picking one sets the
//! material's colour in the 3D view (Job Setup's Material Settings colour); Custom keeps
//! the colour Job Setup has.

use cut_craft_core::document::{Document, Job};
use cut_craft_core::{Change, Command, CommandError, Meta};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct Materials;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Material {
    /// Job Setup's own colour
    #[default]
    Custom,
    Pine,
    Maple,
    #[schemars(title = "Birch Plywood")]
    BirchPlywood,
    Oak,
    Cedar,
    Cherry,
    Mahogany,
    Walnut,
    #[schemars(title = "MDF")]
    Mdf,
    /// High-density urethane sign foam
    #[schemars(title = "HDU Foam")]
    HduFoam,
    #[schemars(title = "HDPE (White)")]
    HdpeWhite,
    #[schemars(title = "Acrylic (Black)")]
    AcrylicBlack,
    Aluminium,
    Brass,
}

/// Each material's colour in the 3D view: an even, lit surface of it.
// ponytail: flat colours; grain and other textures need the 3D view to shade by position
const LIBRARY: [(Material, [u8; 3]); 14] = [
    // the colour a new job starts with
    (Material::Pine, [214, 160, 102]),
    (Material::Maple, [238, 212, 168]),
    (Material::BirchPlywood, [226, 198, 152]),
    (Material::Oak, [190, 142, 88]),
    (Material::Cedar, [186, 110, 70]),
    (Material::Cherry, [168, 92, 58]),
    (Material::Mahogany, [126, 62, 44]),
    (Material::Walnut, [98, 68, 46]),
    (Material::Mdf, [172, 136, 96]),
    (Material::HduFoam, [214, 204, 182]),
    (Material::HdpeWhite, [236, 236, 230]),
    (Material::AcrylicBlack, [38, 38, 42]),
    (Material::Aluminium, [192, 196, 202]),
    (Material::Brass, [198, 166, 90]),
];

impl Material {
    /// Its colour; `None` for Custom.
    pub fn colour(self) -> Option<[u8; 3]> {
        LIBRARY.iter().find(|(m, _)| *m == self).map(|&(_, c)| c)
    }

    /// The material shown in `colour`, Custom when none is.
    pub fn of(colour: [u8; 3]) -> Self {
        LIBRARY
            .iter()
            .find(|(_, c)| *c == colour)
            .map_or(Self::Custom, |&(m, _)| m)
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    /// What the job is cut from, as the 3D view shows it
    #[schemars(extend("x-widget" = "drop-down"))]
    pub material: Material,
}

impl Command for Materials {
    const ID: &'static str = "materials";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Materials",
            group: "Toolpaths",
            icon: "",
            shortcut: "",
        }
    }

    fn initial_params(doc: &Document) -> Params {
        Params {
            material: Material::of(doc.job.colour),
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        let colour = p.material.colour().unwrap_or(doc.job.colour);
        let mut c = Change::builder(doc);
        c.set_job(Job { colour, ..doc.job });
        Ok(c.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::testing::{after, run_on, sample_document};

    cut_craft_core::contract_tests!(Materials);

    #[test]
    fn a_material_colours_the_job_and_custom_keeps_its_colour() {
        let doc = after::<Materials>(&Params {
            material: Material::Walnut,
        });
        assert_eq!(doc.job.colour, Material::Walnut.colour().unwrap());
        assert_eq!(Materials::initial_params(&doc).material, Material::Walnut);
        let mut doc = sample_document();
        doc.job.colour = [1, 2, 3];
        run_on::<Materials>(&mut doc, &Params::default());
        assert_eq!(doc.job.colour, [1, 2, 3]);
        assert_eq!(Materials::initial_params(&doc).material, Material::Custom);
    }

    #[test]
    fn a_new_job_is_pine_and_every_material_has_its_own_colour() {
        assert_eq!(Material::of(Job::default().colour), Material::Pine);
        for (m, c) in LIBRARY {
            assert_eq!(Material::of(c), m, "{m:?} shares a colour");
        }
    }
}
