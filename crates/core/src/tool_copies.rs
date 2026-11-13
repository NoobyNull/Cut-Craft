//! One shape per bit. Some makers' tool databases list a bit once per material, each copy
//! in a material's folder (`V-BIT/SOFTWOOD/1/4" SHANK`) with that material's feeds and
//! speeds; imported, the copies get a number after the name (`… 2`, `… 3`) and only one
//! of them may carry the shank and lengths. [`ToolDatabase::join_material_copies`] makes
//! them one tool: the bit's shape once, and each copy's cutting data kept for the material
//! its folder names.

use crate::toolpath::{CuttingData, Tool, ToolDatabase};

/// Two sizes closer than this (mm or degrees) are the same.
const SAME: f64 = 1e-6;

impl ToolDatabase {
    /// Joins each bit's per-material copies into the bit (see the module): the shape is the
    /// bit's, a size it leaves unset taken from a copy; each copy's cutting data is kept
    /// for its folder's material; the copy's old name still finds the bit
    /// ([`Self::joined`]). A copy's size that differs from the bit's is not kept, and is
    /// said in the lines given back (one per difference), for the user to check.
    pub fn join_material_copies(&mut self) -> Vec<String> {
        let mut report = vec![];
        let mut i = 0;
        while i < self.tools.len() {
            let family = self.copies_of(i);
            if !family.is_empty() {
                report.extend(self.join(i, &family));
            }
            i += 1;
        }
        report
    }

    /// The copies of tool `i` (by index, after it or before it): tools named as it with a
    /// number after, of the same type and cutting size, in a sibling folder each. Empty
    /// unless every one sits in a material folder of its own.
    fn copies_of(&self, i: usize) -> Vec<usize> {
        let bit = &self.tools[i];
        let copy_of = |t: &Tool| {
            t.name
                .strip_prefix(bit.name.as_str())
                .and_then(|rest| rest.strip_prefix(' '))
                .is_some_and(|n| n.parse::<u32>().is_ok_and(|n| n >= 2))
        };
        let copies: Vec<usize> = (0..self.tools.len())
            .filter(|&j| j != i)
            .filter(|&j| {
                let t = &self.tools[j];
                copy_of(t) && same_cutter(bit, t) && self.is_catalogue(t) == self.is_catalogue(bit)
            })
            .collect();
        let groups: Vec<&str> = std::iter::once(i)
            .chain(copies.iter().copied())
            .map(|k| self.tools[k].group.as_str())
            .collect();
        match material_folders(&groups) {
            Some(_) => copies,
            None => vec![],
        }
    }

    /// Joins `copies` into tool `bit`; what was not kept.
    fn join(&mut self, bit: usize, copies: &[usize]) -> Vec<String> {
        let groups: Vec<String> = std::iter::once(bit)
            .chain(copies.iter().copied())
            .map(|k| self.tools[k].group.clone())
            .collect();
        let refs: Vec<&str> = groups.iter().map(String::as_str).collect();
        let Some((group, materials)) = material_folders(&refs) else {
            return vec![];
        };
        let mut report = vec![];
        let mut joined = self.tools[bit].clone();
        add_data(&mut joined, &self.tools[bit], &materials[0]);
        for (copy, material) in copies.iter().zip(&materials[1..]) {
            let copy = self.tools[*copy].clone();
            report.extend(take_shape(&mut joined, &copy));
            add_data(&mut joined, &copy, material);
            if joined.number == 0 {
                joined.number = copy.number;
            }
            self.joined.insert(copy.name.clone(), joined.name.clone());
        }
        joined.group = group;
        for m in &materials {
            self.add_material(m);
        }
        self.tools[bit] = joined;
        let mut gone: Vec<usize> = copies.to_vec();
        gone.sort_unstable();
        for k in gone.into_iter().rev() {
            self.tools.remove(k);
        }
        report
    }
}

/// Whether two tools cut the same: type, diameter, angle and tip radius.
fn same_cutter(a: &Tool, b: &Tool) -> bool {
    a.shape == b.shape
        && (a.diameter - b.diameter).abs() < SAME
        && (a.angle - b.angle).abs() < SAME
        && (a.tip_radius - b.tip_radius).abs() < SAME
}

/// The copies' group with the material folders taken out, and each one's material (the
/// folders where their groups differ): `None` unless every group has one of its own.
fn material_folders(groups: &[&str]) -> Option<(String, Vec<String>)> {
    if groups.len() < 2 {
        return None;
    }
    let split: Vec<Vec<&str>> = groups.iter().map(|g| g.split('/').collect()).collect();
    let shortest = split.iter().map(Vec::len).min()?;
    let mut front = 0;
    while front < shortest && split.iter().all(|s| s[front] == split[0][front]) {
        front += 1;
    }
    let last = |s: &[&str], k: usize| s[s.len() - 1 - k].to_owned();
    let mut back = 0;
    while back < shortest - front && split.iter().all(|s| last(s, back) == last(&split[0], back)) {
        back += 1;
    }
    let materials: Vec<String> = split
        .iter()
        .map(|s| material_name(&s[front..s.len() - back].join("/")))
        .collect();
    let distinct = materials
        .iter()
        .enumerate()
        .all(|(i, m)| !m.is_empty() && !materials[..i].iter().any(|o| o.eq_ignore_ascii_case(m)));
    let group = [&split[0][..front], &split[0][split[0].len() - back..]]
        .concat()
        .join("/");
    distinct.then_some((group, materials))
}

/// A folder's name as a material: `SOFTWOOD` reads Softwood; other names as they are.
fn material_name(folder: &str) -> String {
    if folder.chars().any(char::is_lowercase) {
        return folder.trim().to_owned();
    }
    folder
        .split(' ')
        .map(|w| {
            let mut c = w.chars();
            c.next().map_or_else(String::new, |f| {
                f.to_uppercase()
                    .chain(c.flat_map(char::to_lowercase))
                    .collect()
            })
        })
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_owned()
}

/// `from`'s own cutting data (its default and any per material) as `bit`'s for
/// `material`, where `bit` has none for it yet.
fn add_data(bit: &mut Tool, from: &Tool, material: &str) {
    let mut sets = vec![CuttingData {
        material: material.to_owned(),
        ..from.cutting("")
    }];
    sets.extend(from.materials.iter().cloned());
    for set in sets {
        let had = bit
            .materials
            .iter()
            .any(|m| m.material.eq_ignore_ascii_case(&set.material));
        if !set.material.is_empty() && !had {
            bit.materials.push(set);
        }
    }
}

/// One of a tool's sizes, to read or set.
type Size = fn(&mut Tool) -> &mut f64;

/// The sizes `copy` gives that `bit` leaves unset, into `bit`; a line for each one both
/// give differently (the bit's is kept).
fn take_shape(bit: &mut Tool, copy: &Tool) -> Vec<String> {
    let mut report = vec![];
    let sizes: [(&str, Size); 7] = [
        ("shank diameter", |t| &mut t.shank_diameter),
        ("flute length", |t| &mut t.flute_length),
        ("neck length", |t| &mut t.neck_length),
        ("shank length", |t| &mut t.shank_length),
        ("taper", |t| &mut t.taper),
        ("tooth size", |t| &mut t.tooth_size),
        ("tooth offset", |t| &mut t.tooth_offset),
    ];
    let mut copy = copy.clone();
    let name = bit.name.clone();
    for (what, size) in sizes {
        let theirs = *size(&mut copy);
        let ours = size(bit);
        if *ours == 0.0 {
            *ours = theirs;
        } else if theirs != 0.0 && (theirs - *ours).abs() > SAME {
            report.push(format!(
                "{}: its {what} was {} in \"{}\"; the bit's {} is kept",
                name,
                trim(theirs),
                copy.name,
                trim(*ours)
            ));
        }
    }
    if copy.flutes != bit.flutes {
        report.push(format!(
            "{}: \"{}\" had {} flutes; the bit's {} is kept",
            bit.name, copy.name, copy.flutes, bit.flutes
        ));
    }
    report
}

/// A size for a message: up to 3 decimals, no trailing zeros.
fn trim(v: f64) -> String {
    let s = format!("{v:.3}");
    s.trim_end_matches('0').trim_end_matches('.').to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::toolpath::ToolShape;

    const BIT: &str = "SpeTool W06004 V Bit 90 Deg 3/4\" CD x1/4\" SD";
    const TOP: &str = "SpeTool/V-BIT";
    const SHANK: &str = "1/4\" SHANK";

    /// The bit as one maker's file lists it: once per material folder, the feeds the
    /// folder's, the shank given on the first only.
    fn copies() -> ToolDatabase {
        let make = |name: String, folder: &str, feed: f64, shank: f64| Tool {
            name,
            group: format!("{TOP}/{folder}/{SHANK}"),
            shape: ToolShape::VBit,
            diameter: 19.05,
            angle: 90.0,
            tip_radius: 0.0,
            shank_diameter: shank,
            feed_rate: feed,
            ..Tool::default()
        };
        let mut db = ToolDatabase {
            tools: vec![
                make(BIT.into(), "MDF/Laminate", 1000.0, 6.35),
                make(format!("{BIT} 2"), "SOFTWOOD", 2000.0, 0.0),
                make(format!("{BIT} 3"), "HARDWOOD", 1500.0, 0.0),
                make(format!("{BIT} 4"), "PLASTIC", 1200.0, 0.0),
                Tool {
                    name: "6 mm End Mill".into(),
                    ..Tool::default()
                },
            ],
            ..ToolDatabase::default()
        };
        db.tools[2].number = 3;
        db
    }

    #[test]
    fn a_bit_listed_per_material_becomes_one_bit_with_data_per_material() {
        let mut db = copies();
        let report = db.join_material_copies();
        assert_eq!(report, Vec::<String>::new(), "nothing differed");
        assert_eq!(db.tools.len(), 2, "one bit, and the end mill");
        let bit = &db.tools[0];
        assert_eq!(bit.name, BIT);
        assert_eq!(bit.group, format!("{TOP}/{SHANK}"));
        assert_eq!(bit.shank_diameter, 6.35, "the shank is the bit's");
        assert_eq!(bit.number, 3, "a copy's tool number is kept");
        let feed = |m: &str| bit.for_material(m).feed_rate;
        assert_eq!(
            [
                feed("MDF/Laminate"),
                feed("Softwood"),
                feed("Hardwood"),
                feed("Plastic")
            ],
            [1000.0, 2000.0, 1500.0, 1200.0]
        );
        assert_eq!(feed(""), 1000.0, "the default stays the bit's own");
        assert_eq!(
            db.materials,
            ["MDF/Laminate", "Softwood", "Hardwood", "Plastic"]
        );
        // a toolpath or holder naming a copy finds the bit
        assert_eq!(db.index_of(&format!("{BIT} 3")), Some(0));
        assert_eq!(db.get(&format!("{BIT} 3")).unwrap().shank_diameter, 6.35);
        assert_eq!(db.join_material_copies(), Vec::<String>::new());
        assert_eq!(db.tools.len(), 2, "joined once");
    }

    #[test]
    fn a_copy_s_size_fills_one_the_bit_leaves_unset_and_a_differing_one_is_said() {
        let mut db = copies();
        db.tools[0].shank_diameter = 0.0;
        db.tools[1].shank_diameter = 6.35;
        db.tools[2].shank_diameter = 12.7;
        db.tools[3].flute_length = 9.5;
        let report = db.join_material_copies();
        let bit = &db.tools[0];
        assert_eq!((bit.shank_diameter, bit.flute_length), (6.35, 9.5));
        assert_eq!(report.len(), 1, "{report:?}");
        assert!(
            report[0].contains("shank diameter was 12.7") && report[0].contains("6.35"),
            "{}",
            report[0]
        );
    }

    #[test]
    fn tools_that_only_share_a_name_are_not_joined() {
        // a different size, the same folder, or no folder to name a material
        for change in [
            |db: &mut ToolDatabase| db.tools[1].diameter = 12.7,
            |db: &mut ToolDatabase| db.tools[1].group = db.tools[0].group.clone(),
            |db: &mut ToolDatabase| {
                for t in &mut db.tools {
                    t.group.clear();
                }
            },
        ] {
            let mut db = copies();
            change(&mut db);
            let before = db.tools.len();
            db.join_material_copies();
            assert!(db.tools.len() > before - 3, "the odd one stays apart");
        }
        // New Tool and New Tool 2, made in one group, stay two tools
        let mut db = ToolDatabase {
            tools: vec![
                Tool {
                    name: "New Tool".into(),
                    ..Tool::default()
                },
                Tool {
                    name: "New Tool 2".into(),
                    ..Tool::default()
                },
            ],
            ..ToolDatabase::default()
        };
        db.join_material_copies();
        assert_eq!(db.tools.len(), 2);
    }

    #[test]
    fn folder_names_read_as_materials() {
        assert_eq!(material_name("SOFTWOOD"), "Softwood");
        assert_eq!(material_name("SOLID SURFACE"), "Solid Surface");
        assert_eq!(material_name("MDF/Laminate"), "MDF/Laminate");
    }
}
