//! The machine packages in `machines/`, built in, so first-time setup can offer them without
//! a download.

use crate::Library;
use crate::machine::{Machine, Machines};

/// Each maker's package, in the order setup lists the makers.
const PACKAGES: [&str; 5] = [
    include_str!("../../../machines/carbide3d.json"),
    include_str!("../../../machines/foxalien.json"),
    include_str!("../../../machines/genmitsu.json"),
    include_str!("../../../machines/sienci.json"),
    include_str!("../../../machines/twotrees.json"),
];

/// Every machine in the built-in packages, maker by maker.
pub fn catalogue() -> Vec<Machine> {
    PACKAGES
        .iter()
        .flat_map(|text| {
            serde_json::from_str::<Machines>(text)
                .expect("the built-in machine packages parse")
                .machines
        })
        .collect()
}

/// The makers in `machines`, each once, in the order they first appear.
pub fn makers(machines: &[Machine]) -> Vec<String> {
    let mut makers: Vec<String> = vec![];
    for m in machines {
        if !makers.contains(&m.manufacturer) {
            makers.push(m.manufacturer.clone());
        }
    }
    makers
}

/// The controller of a machine that is not in the packages, as setup asks for it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Controller {
    /// grbl and grblHAL, which most hobby routers run
    #[default]
    Grbl,
    FluidNc,
    Mach3,
    /// Not sure: grbl's posts, the likeliest to suit
    Other,
}

impl Controller {
    pub const ALL: [Self; 4] = [Self::Grbl, Self::FluidNc, Self::Mach3, Self::Other];

    pub fn label(self) -> &'static str {
        match self {
            Self::Grbl => "grbl or grblHAL",
            Self::FluidNc => "FluidNC",
            Self::Mach3 => "Mach3",
            Self::Other => "Something else, or not sure",
        }
    }

    /// The firmware it stands for, when it names one: "grbl or grblHAL" and "not sure" are
    /// left to be read (as standard GRBL) until picked in Machine Configuration.
    pub fn firmware(self) -> Option<crate::Firmware> {
        match self {
            Self::FluidNc => Some(crate::Firmware::FluidNc),
            Self::Mach3 => Some(crate::Firmware::Mach),
            Self::Grbl | Self::Other => None,
        }
    }

    /// The word PowerWhittle's own posts for this controller are named with.
    fn post_word(self) -> &'static str {
        match self {
            Self::Grbl | Self::Other => "GRBL",
            Self::FluidNc => "FluidNC",
            Self::Mach3 => "Mach3",
        }
    }

    /// PowerWhittle's flat (not rotary) posts for this controller, those in the machine's units
    /// first, and which one saves toolpaths: the arcs post in its units.
    pub fn posts(self, inches: bool) -> (Vec<String>, String) {
        let prefix = format!("PowerWhittle {} ", self.post_word());
        let units = if inches { "(inch)" } else { "(mm)" };
        let mut posts: Vec<String> = Library::new(None)
            .entries()
            .into_iter()
            .map(|e| e.name)
            .filter(|n| n.starts_with(&prefix) && !n.contains("Rotary A"))
            .collect();
        // stable: each units' posts keep the library's order
        posts.sort_by_key(|n| !n.contains(units));
        let default = posts
            .iter()
            .find(|n| n.contains(units) && !n.contains("Straight"))
            .or_else(|| posts.first())
            .cloned()
            .unwrap_or_default();
        (posts, default)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_built_in_machine_has_a_bed_and_a_post_cut_craft_has() {
        let names: Vec<String> = Library::new(None)
            .entries()
            .into_iter()
            .map(|e| e.name)
            .collect();
        let all = catalogue();
        assert!(all.len() >= 10, "{}", all.len());
        for m in &all {
            assert!(m.width > 0.0 && m.height > 0.0, "{}", m.name);
            let post = m.post().unwrap_or_else(|| panic!("{} has no post", m.name));
            assert!(names.iter().any(|n| n == post), "{}: {post}", m.name);
        }
        assert_eq!(
            makers(&all),
            ["Carbide 3D", "FoxAlien", "Genmitsu", "Sienci", "Two Trees"]
        );
    }

    #[test]
    fn a_controller_not_listed_gets_its_posts_in_the_chosen_units_first() {
        let (posts, default) = Controller::Grbl.posts(true);
        assert_eq!(default, "PowerWhittle GRBL Arcs (inch) (*.nc)");
        assert!(posts[0].contains("(inch)") && posts.iter().any(|p| p.contains("(mm)")));
        assert!(posts.iter().all(|p| !p.contains("Rotary A")), "{posts:?}");
        let (_, mach3) = Controller::Mach3.posts(false);
        assert_eq!(mach3, "PowerWhittle Mach3 Arcs (mm) (*.tap)");
        let (_, fluid) = Controller::FluidNc.posts(false);
        assert!(
            fluid.starts_with("PowerWhittle FluidNC") && fluid.contains("(mm)"),
            "{fluid}"
        );
        assert_eq!(
            Controller::Other.posts(false).1,
            "PowerWhittle GRBL Arcs (mm) (*.nc)"
        );
    }
}
