//! Vectric `.vtdb` tool databases (Aspire / `VCarve` 12.5): a SQLite file of tool geometries,
//! cutting data per machine and material, and a tool tree. Vendors publish these (Amana,
//! IDC Woodcraft, CIC), so Cut Craft reads them into its own tool database.
//!
//! What was learned from three vendor files (nothing in them names the enumerations):
//! `tool_geometry.tool_type` 0 ball nose, 1 end mill, 2 radiused end mill, 3 V-bit,
//! 4 engraving (angle + flat), 5 tapered ball nose, 6 drill, 8 form tool, 9 diamond drag;
//! `units` / `length_units` 0 mm, 1 inches; `rate_units` 1 mm/min, 2 m/min, 4 in/min, taken as
//! the place in Aspire's rate list (mm/sec, mm/min, m/min, in/sec, in/min, ft/min); a `tooth_size`
//! makes a thread mill (none seen); each geometry has a placeholder entry with no material
//! and no feed, and one with data for the file's one material and machine. All [inf].

use cut_craft_core::toolpath::{RateUnits, Tool, ToolDatabase, ToolGroup, ToolShape};
use rusqlite::{Connection, OpenFlags};
use std::collections::HashMap;
use std::path::Path;

/// Every tool in the file that Cut Craft has a shape for, with its cutting data for each
/// material the file has data for, in the file's groups.
pub fn read(path: &Path) -> Result<ToolDatabase, String> {
    let db = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|e| format!("{}: {e}", path.display()))?;
    from_db(&db)
}

/// [`read`], from the open file. Each geometry is one tool: its first material's data is
/// also its default.
// ponytail: cutting data is kept per material only; a second machine's data for the same
// material is dropped (every vendor file seen has one machine)
fn from_db(db: &Connection) -> Result<ToolDatabase, String> {
    let mut q = db
        .prepare(
            "select g.name_format, g.tool_type, g.units, g.diameter, g.included_angle, \
             g.tip_radius, g.num_flutes, c.rate_units, c.length_units, c.feed_rate, \
             c.plunge_rate, c.spindle_speed, c.stepdown, c.stepover, mt.name, g.id, \
             g.notes, g.flute_length, g.tooth_size, g.tooth_offset \
             from tool_geometry g \
             join tool_entity e on e.tool_geometry_id = g.id \
             join tool_cutting_data c on c.id = e.tool_cutting_data_id \
             left join material mt on mt.id = e.material_id \
             order by g.name_format, g.id, mt.name",
        )
        .map_err(|e| e.to_string())?;
    let rows = q
        .query_map([], |r| {
            Ok(Row {
                name_format: r.get(0)?,
                tool_type: r.get(1)?,
                units: r.get(2)?,
                diameter: r.get(3)?,
                included_angle: r.get(4)?,
                tip_radius: r.get(5)?,
                flutes: r.get(6)?,
                rate_units: r.get(7)?,
                length_units: r.get(8)?,
                feed_rate: r.get(9)?,
                plunge_rate: r.get(10)?,
                spindle_speed: r.get(11)?,
                stepdown: r.get(12)?,
                stepover: r.get(13)?,
                material: r.get(14)?,
                geometry_id: r.get(15)?,
                notes: r.get(16)?,
                flute_length: r.get(17)?,
                tooth_size: r.get(18)?,
                tooth_offset: r.get(19)?,
            })
        })
        .map_err(|e| e.to_string())?;
    let (groups, group_of) = tree(db);
    let mut out = ToolDatabase {
        groups,
        ..ToolDatabase::default()
    };
    // rows come geometry by geometry: the first with data makes the tool, the rest add to it
    let mut last_geometry = None;
    for row in rows {
        let row = row.map_err(|e| e.to_string())?;
        let Some(mut tool) = row.tool() else { continue };
        if let Some(material) = &row.material {
            out.add_material(material);
            tool.materials = vec![tool.cutting(material)];
        }
        match out.tools.last_mut() {
            Some(last) if last_geometry.as_ref() == Some(&row.geometry_id) => {
                // a second machine's data for a material already read is dropped
                let data = tool.materials.pop().filter(|d| {
                    !last
                        .materials
                        .iter()
                        .any(|m| m.material.eq_ignore_ascii_case(&d.material))
                });
                last.materials.extend(data);
            }
            _ => {
                tool.group = group_of.get(&row.geometry_id).cloned().unwrap_or_default();
                tool.notes = row.notes.unwrap_or_default();
                out.tools.push(tool);
                last_geometry = Some(row.geometry_id);
            }
        }
    }
    if out.tools.is_empty() {
        return Err("no tools Cut Craft can use were found in the file".into());
    }
    Ok(out)
}

/// The file's tool tree: its groups in order, and the group each tool geometry is in.
/// A file without a tree has none.
fn tree(db: &Connection) -> (Vec<ToolGroup>, HashMap<String, String>) {
    struct Entry {
        id: String,
        parent: Option<String>,
        geometry: Option<String>,
        name: String,
        notes: String,
    }
    let entries: Vec<Entry> = db
        .prepare(
            "select id, parent_group_id, tool_geometry_id, name, notes from tool_tree_entry \
             order by sibling_order",
        )
        .and_then(|mut q| {
            q.query_map([], |r| {
                Ok(Entry {
                    id: r.get(0)?,
                    parent: r.get::<_, Option<String>>(1)?.filter(|p| !p.is_empty()),
                    geometry: r.get(2)?,
                    name: r.get::<_, Option<String>>(3)?.unwrap_or_default(),
                    notes: r.get::<_, Option<String>>(4)?.unwrap_or_default(),
                })
            })?
            .collect()
        })
        .unwrap_or_default();
    let (mut groups, mut group_of) = (vec![], HashMap::new());
    // depth first from the top, children in sibling order, so groups come in tree order
    let mut pending: Vec<(Option<&Entry>, String)> = vec![(None, String::new())];
    while let Some((group, path)) = pending.pop() {
        if let Some(group) = group {
            groups.push(ToolGroup {
                path: path.clone(),
                notes: group.notes.clone(),
            });
        }
        let parent = group.map(|g| g.id.as_str());
        let mut next = vec![];
        for child in entries.iter().filter(|e| e.parent.as_deref() == parent) {
            if let Some(geometry) = &child.geometry {
                group_of.insert(geometry.clone(), path.clone());
            } else if path.is_empty() {
                next.push((Some(child), child.name.clone()));
            } else {
                next.push((Some(child), format!("{path}/{}", child.name)));
            }
        }
        pending.extend(next.into_iter().rev());
    }
    (groups, group_of)
}

struct Row {
    name_format: String,
    tool_type: i64,
    units: i64,
    diameter: Option<f64>,
    included_angle: Option<f64>,
    tip_radius: Option<f64>,
    flutes: Option<i64>,
    rate_units: i64,
    length_units: i64,
    feed_rate: Option<f64>,
    plunge_rate: Option<f64>,
    spindle_speed: Option<f64>,
    stepdown: Option<f64>,
    stepover: Option<f64>,
    material: Option<String>,
    geometry_id: String,
    notes: Option<String>,
    flute_length: Option<f64>,
    tooth_size: Option<f64>,
    tooth_offset: Option<f64>,
}

const INCH: f64 = 25.4;

fn length(units: i64) -> f64 {
    if units == 1 { INCH } else { 1.0 }
}

/// The file's rate units, numbered in the order of Aspire's list.
fn rate_units(units: i64) -> RateUnits {
    usize::try_from(units)
        .ok()
        .and_then(|i| RateUnits::ALL.get(i).copied())
        .unwrap_or_default()
}

impl Row {
    fn tool(&self) -> Option<Tool> {
        // a tapered ball nose's diameter is its shank; its tip radius is the ball (none: a
        // tapered flat bottom, cut as an end mill)
        let tip = self.tip_radius.filter(|r| *r > 0.0);
        // no vendor file has a thread mill, so its tool_type is unknown; its tooth says it is
        let tooth = self.tooth_size.filter(|s| *s > 0.0);
        let shape = match self.tool_type {
            _ if tooth.is_some() => ToolShape::ThreadMill,
            0 => ToolShape::BallNose,
            5 if tip.is_some() => ToolShape::BallNose,
            1 | 5 | 6 => ToolShape::EndMill,
            2 => ToolShape::RadiusedEndMill,
            3 | 4 => ToolShape::VBit,
            _ => return None, // form tools, diamond drags and unknown kinds
        };
        let scale = length(self.units);
        let diameter = match (self.tool_type, tip) {
            (5, Some(r)) => 2.0 * r,
            _ => self.diameter.filter(|d| *d > 0.0)?,
        } * scale;
        // an entry without a feed rate is a placeholder (a machine with no cutting data)
        let feed = self.feed_rate.filter(|f| *f > 0.0)?;
        let cut_scale = length(self.length_units);
        let base = Tool::default();
        let name = self.name();
        let per = rate_units(self.rate_units);
        Some(Tool {
            name,
            shape,
            rate_units: per,
            tooth_size: tooth.map_or(base.tooth_size, |s| s * scale),
            tooth_offset: self
                .tooth_offset
                .filter(|_| tooth.is_some())
                .map_or(base.tooth_offset, |o| o * scale),
            tip_radius: match shape {
                ToolShape::RadiusedEndMill => self.tip_radius.unwrap_or(0.0) * scale,
                _ => 0.0,
            },
            angle: match shape {
                ToolShape::VBit | ToolShape::ThreadMill => {
                    self.included_angle.unwrap_or(base.angle)
                }
                _ => 0.0,
            },
            // a tapered tool's included angle is both sides of its taper; it widens to the
            // diameter the file gives
            taper: match self.tool_type {
                5 => self.included_angle.unwrap_or(0.0).max(0.0) / 2.0,
                _ => 0.0,
            },
            shank_diameter: match self.tool_type {
                5 => self.diameter.unwrap_or(0.0) * scale,
                _ => 0.0,
            },
            flute_length: self.flute_length.filter(|l| *l > 0.0).unwrap_or(0.0) * scale,
            diameter,
            pass_depth: self.stepdown.map_or(base.pass_depth, |s| s * cut_scale),
            stepover: self.stepover.map_or(base.stepover, |s| {
                (s * cut_scale / diameter).clamp(0.01, 1.0)
            }),
            feed_rate: feed * per.mm_per_min(),
            plunge_rate: self
                .plunge_rate
                .map_or(base.plunge_rate, |f| f * per.mm_per_min()),
            spindle_speed: self.spindle_speed.unwrap_or(base.spindle_speed),
            flutes: self
                .flutes
                .and_then(|f| u32::try_from(f).ok())
                .filter(|f| *f > 0)
                .unwrap_or(base.flutes),
            ..base
        })
    }

    /// The vendor's name with Aspire's `{...}` fields filled in.
    fn name(&self) -> String {
        let kind = match self.tool_type {
            0 => "Ball Nose",
            1 => "End Mill",
            2 => "Radiused End Mill",
            3 => "V-Bit",
            4 => "Engraving",
            5 => "Tapered Ball Nose",
            6 => "Drill",
            _ => "Tool",
        };
        let unit = if self.units == 1 { "in" } else { "mm" };
        let mut name = String::new();
        let mut rest = self.name_format.as_str();
        while let Some((before, after)) = rest.split_once('{') {
            name.push_str(before);
            let Some((field, tail)) = after.split_once('}') else {
                rest = "";
                break;
            };
            let value = match field.split('|').next().unwrap_or_default() {
                "Tool Type" => kind.to_owned(),
                "Diameter" => self.diameter.map_or(String::new(), trim),
                "Included Angle" => self.included_angle.map_or(String::new(), trim),
                "Units Short" => unit.to_owned(),
                _ => String::new(),
            };
            name.push_str(&value);
            rest = tail;
        }
        name.push_str(rest);
        name.split_whitespace().collect::<Vec<_>>().join(" ")
    }
}

fn trim(x: f64) -> String {
    let s = format!("{x:.4}");
    s.trim_end_matches('0').trim_end_matches('.').to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn end_mill() -> Row {
        Row {
            name_format: "{Tool Type} ({Diameter|F}{Units Short}) DOWN CUT".into(),
            tool_type: 1,
            units: 1,
            diameter: Some(0.25),
            included_angle: None,
            tip_radius: None,
            flutes: Some(2),
            rate_units: 4,
            length_units: 1,
            feed_rate: Some(100.0),
            plunge_rate: Some(30.0),
            spindle_speed: Some(18000.0),
            stepdown: Some(0.125),
            stepover: Some(0.1),
            material: Some("Softwood".into()),
            geometry_id: String::new(),
            notes: None,
            flute_length: Some(0.875),
            tooth_size: None,
            tooth_offset: None,
        }
    }

    #[test]
    fn names_fill_in_aspires_fields() {
        let t = end_mill().tool().unwrap();
        assert_eq!(t.name, "End Mill (0.25in) DOWN CUT");
        assert_eq!(t.shape, ToolShape::EndMill);
        assert!((t.diameter - 6.35).abs() < 1e-9);
        assert!((t.feed_rate - 2540.0).abs() < 1e-9, "in/min to mm/min");
        assert!((t.pass_depth - 3.175).abs() < 1e-9);
        assert!((t.stepover - 0.4).abs() < 1e-9, "0.1 in of a 0.25 in tool");
        assert_eq!(t.flutes, 2);
    }

    #[test]
    fn a_tapered_ball_nose_cuts_with_its_tip_not_its_shank() {
        // Amana #46280: 1/4 in shank, 1/64 in tip radius, 0.01 in stepover
        let tapered = Row {
            tool_type: 5,
            tip_radius: Some(0.015_625),
            included_angle: Some(12.4),
            stepover: Some(0.01),
            ..end_mill()
        };
        let t = tapered.tool().unwrap();
        assert_eq!(t.shape, ToolShape::BallNose);
        assert!((t.diameter - 0.793_75).abs() < 1e-9, "{}", t.diameter);
        assert!((t.taper - 6.2).abs() < 1e-9, "half the included angle");
        assert!(
            (t.shank_diameter - 6.35).abs() < 1e-9,
            "the file's diameter"
        );
        assert!((t.flute_length - 22.225).abs() < 1e-9, "7/8 in");
        assert!((t.stepover - 0.32).abs() < 1e-9, "0.01 in of a 1/32 in tip");
        // a tapered flat bottom (no tip radius) cuts like an end mill its diameter across
        let flat = Row {
            tip_radius: Some(0.0),
            ..tapered
        };
        let t = flat.tool().unwrap();
        assert_eq!(t.shape, ToolShape::EndMill);
        assert!((t.diameter - 6.35).abs() < 1e-9);
    }

    /// A file with one inch end mill: a placeholder entry for no material, then data for
    /// softwood and oak; and a thread mill with data for oak.
    fn file() -> Connection {
        let db = Connection::open_in_memory().unwrap();
        db.execute_batch(
            "create table tool_geometry (id text, name_format text, notes text, tool_type int, \
             units int, diameter real, included_angle real, tip_radius real, num_flutes int, \
             flute_length real, tooth_size real, tooth_offset real);
             create table tool_cutting_data (id text, rate_units int, length_units int, \
             feed_rate real, plunge_rate real, spindle_speed int, stepdown real, stepover real);
             create table tool_entity (id text, material_id text, machine_id text, \
             tool_geometry_id text, tool_cutting_data_id text);
             create table material (id text, name text);
             create table machine (id text, name text);
             insert into material values ('s', 'Softwood'), ('o', 'Oak');
             insert into machine values ('m', 'Desktop');
             insert into tool_geometry values ('g', '{Tool Type} ({Diameter|F}{Units Short})', \
             null, 1, 1, 0.25, null, null, 2, 1.0, null, null);
             insert into tool_geometry values ('t', 'Thread Mill', null, 1, 0, 10.0, 60.0, \
             null, 1, 20.0, 1.2, 2.5);
             insert into tool_cutting_data values ('c0', 4, 1, null, null, null, null, null), \
             ('c1', 4, 1, 100.0, 30.0, 18000, 0.125, 0.1), \
             ('c2', 4, 1, 50.0, 20.0, 16000, 0.0625, 0.1), \
             ('c3', 1, 0, 300.0, 100.0, 12000, 1.0, 0.5);
             insert into tool_entity values ('e0', null, 'm', 'g', 'c0'), \
             ('e1', 's', 'm', 'g', 'c1'), ('e2', 'o', 'm', 'g', 'c2'), \
             ('e3', 'o', 'm', 't', 'c3');",
        )
        .unwrap();
        db
    }

    #[test]
    fn each_tool_keeps_its_cutting_data_per_material() {
        let db = from_db(&file()).unwrap();
        assert_eq!(db.tools.len(), 2, "one tool per geometry");
        assert_eq!(db.materials, ["Oak", "Softwood"]);
        let mill = db
            .get("End Mill (0.25in)")
            .expect("named without its material");
        assert_eq!(mill.rate_units, RateUnits::InchPerMin);
        let oak = mill.cutting("Oak");
        assert!((oak.feed_rate - 1270.0).abs() < 1e-9, "50 in/min");
        assert!((oak.pass_depth - 1.5875).abs() < 1e-9);
        let soft = mill.cutting("Softwood");
        assert!((soft.feed_rate - 2540.0).abs() < 1e-9);
        assert!((soft.spindle_speed - 18000.0).abs() < 1e-9);
        assert!(
            (mill.feed_rate - 1270.0).abs() < 1e-9,
            "the first material's data is the default"
        );
    }

    #[test]
    fn a_geometry_with_a_tooth_is_a_thread_mill() {
        let db = from_db(&file()).unwrap();
        let t = db.get("Thread Mill").unwrap();
        assert_eq!(t.shape, ToolShape::ThreadMill);
        assert!((t.tooth_size - 1.2).abs() < 1e-9 && (t.tooth_offset - 2.5).abs() < 1e-9);
        assert!((t.angle - 60.0).abs() < 1e-9 && (t.diameter - 10.0).abs() < 1e-9);
        assert_eq!(t.rate_units, RateUnits::MmPerMin);
    }

    #[test]
    fn the_tool_tree_gives_groups_in_order_and_each_tools_group() {
        let db = Connection::open_in_memory().unwrap();
        db.execute_batch(
            "create table tool_tree_entry (id text, parent_group_id text, sibling_order int, \
             tool_geometry_id text, name text, notes text);
             insert into tool_tree_entry values ('m', null, 1, null, 'Metric', 'mm tools');
             insert into tool_tree_entry values ('i', null, 0, null, 'Imperial', '');
             insert into tool_tree_entry values ('e', 'i', 0, null, 'End Mills', 'flat');
             insert into tool_tree_entry values ('t', 'e', 0, 'g1', null, null);
             insert into tool_tree_entry values ('u', 'm', 0, 'g2', null, null);",
        )
        .unwrap();
        let (groups, group_of) = tree(&db);
        let paths: Vec<&str> = groups.iter().map(|g| g.path.as_str()).collect();
        assert_eq!(paths, ["Imperial", "Imperial/End Mills", "Metric"]);
        assert_eq!(groups[2].notes, "mm tools");
        assert_eq!(group_of["g1"], "Imperial/End Mills");
        assert_eq!(group_of["g2"], "Metric");
        let bare = Connection::open_in_memory().unwrap();
        assert_eq!(tree(&bare).0, vec![], "no tree, no groups");
    }

    /// Reads the vendor databases in the folder `CUT_CRAFT_VENDOR_FILES` names (they are not
    /// part of the repo).
    #[test]
    fn vendor_databases_read_when_given() {
        let Some(dir) = std::env::var_os("CUT_CRAFT_VENDOR_FILES") else {
            return;
        };
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for e in entries.flatten() {
            let p = e.path();
            if p.extension().is_some_and(|x| x == "vtdb") {
                let db = read(&p).unwrap_or_else(|err| panic!("{}: {err}", p.display()));
                let tools = db.tools;
                assert!(tools.len() > 10, "{}", p.display());
                assert!(!db.groups.is_empty(), "{} has a tool tree", p.display());
                assert!(
                    tools
                        .iter()
                        .all(|t| t.group.is_empty() || db.groups.iter().any(|g| g.path == t.group))
                );
                assert!(tools.iter().all(|t| t.diameter > 0.0 && t.feed_rate > 0.0));
                // every vendor file seen keeps its data under one material
                assert!(
                    !db.materials.is_empty(),
                    "{} names its material",
                    p.display()
                );
                assert!(tools.iter().all(|t| {
                    !t.materials.is_empty()
                        && t.materials
                            .iter()
                            .all(|m| db.materials.contains(&m.material) && m.feed_rate > 0.0)
                }));
            }
        }
        assert!(read(Path::new("/nonexistent.vtdb")).is_err());
    }
}
