//! Vectric `.vtdb` tool databases (Aspire / `VCarve` 12.5): a SQLite file of tool geometries,
//! cutting data per machine and material, and a tool tree. Vendors publish these (Amana,
//! IDC Woodcraft, CIC), so Cut Craft reads them into its own tool database.
//!
//! What was learned from three vendor files (nothing in them names the enumerations):
//! `tool_geometry.tool_type` 0 ball nose, 1 end mill, 2 radiused end mill, 3 V-bit,
//! 4 engraving (angle + flat), 5 tapered ball nose, 6 drill, 8 form tool, 9 diamond drag;
//! `units` / `length_units` 0 mm, 1 inches; `rate_units` 1 mm/min, 4 in/min (Aspire's rate
//! list is mm/sec, mm/min, m/min, in/sec, in/min, ft/min). All [inf].

use cut_craft_core::toolpath::{Tool, ToolShape};
use rusqlite::{Connection, OpenFlags};
use std::path::Path;

/// Every tool in the file that Cut Craft has a shape for, each paired with its cutting data
/// (one tool per material and machine it has data for; the name says which).
pub fn read(path: &Path) -> Result<Vec<Tool>, String> {
    let db = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|e| format!("{}: {e}", path.display()))?;
    let mut q = db
        .prepare(
            "select g.name_format, g.tool_type, g.units, g.diameter, g.included_angle, \
             g.tip_radius, g.num_flutes, c.rate_units, c.length_units, c.feed_rate, \
             c.plunge_rate, c.spindle_speed, c.stepdown, c.stepover, m.name, mt.name \
             from tool_geometry g \
             join tool_entity e on e.tool_geometry_id = g.id \
             join tool_cutting_data c on c.id = e.tool_cutting_data_id \
             left join material mt on mt.id = e.material_id \
             left join machine m on m.id = e.machine_id \
             order by g.name_format, mt.name",
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
                machine: r.get(14)?,
                material: r.get(15)?,
            })
        })
        .map_err(|e| e.to_string())?;
    let mut tools = Vec::new();
    for row in rows {
        if let Some(tool) = row.map_err(|e| e.to_string())?.tool() {
            tools.push(tool);
        }
    }
    if tools.is_empty() {
        return Err("no tools Cut Craft can use were found in the file".into());
    }
    Ok(tools)
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
    machine: Option<String>,
    material: Option<String>,
}

const INCH: f64 = 25.4;

fn length(units: i64) -> f64 {
    if units == 1 { INCH } else { 1.0 }
}

/// A rate in mm/min.
fn rate(units: i64) -> f64 {
    match units {
        0 => 60.0,
        2 => 1000.0,
        3 => 60.0 * INCH,
        4 => INCH,
        5 => 12.0 * INCH,
        _ => 1.0,
    }
}

impl Row {
    fn tool(&self) -> Option<Tool> {
        let shape = match self.tool_type {
            0 | 5 => ToolShape::BallNose,
            1 | 6 => ToolShape::EndMill,
            2 => ToolShape::RadiusedEndMill,
            3 | 4 => ToolShape::VBit,
            _ => return None, // form tools, diamond drags and unknown kinds
        };
        let scale = length(self.units);
        let diameter = self.diameter.filter(|d| *d > 0.0)? * scale;
        // an entry without a feed rate is a placeholder (a machine with no cutting data)
        let feed = self.feed_rate.filter(|f| *f > 0.0)?;
        let cut_scale = length(self.length_units);
        let base = Tool::default();
        let name = self.name();
        // ponytail: a tapered ball nose keeps its tip diameter and loses the taper [inf]
        Some(Tool {
            name,
            shape,
            tip_radius: match shape {
                ToolShape::RadiusedEndMill => self.tip_radius.unwrap_or(0.0) * scale,
                _ => 0.0,
            },
            angle: match shape {
                ToolShape::VBit => self.included_angle.unwrap_or(base.angle),
                _ => 0.0,
            },
            diameter,
            pass_depth: self.stepdown.map_or(base.pass_depth, |s| s * cut_scale),
            stepover: self.stepover.map_or(base.stepover, |s| {
                (s * cut_scale / diameter).clamp(0.01, 1.0)
            }),
            feed_rate: feed * rate(self.rate_units),
            plunge_rate: self
                .plunge_rate
                .map_or(base.plunge_rate, |f| f * rate(self.rate_units)),
            spindle_speed: self.spindle_speed.unwrap_or(base.spindle_speed),
            flutes: self
                .flutes
                .and_then(|f| u32::try_from(f).ok())
                .filter(|f| *f > 0)
                .unwrap_or(base.flutes),
        })
    }

    /// The vendor's name with Aspire's `{...}` fields filled in, plus the material it is for.
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
        let name = name.split_whitespace().collect::<Vec<_>>().join(" ");
        match self.material.as_ref().or(self.machine.as_ref()) {
            Some(m) => format!("{name} [{m}]"),
            None => name,
        }
    }
}

fn trim(x: f64) -> String {
    let s = format!("{x:.4}");
    s.trim_end_matches('0').trim_end_matches('.').to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_fill_in_aspires_fields() {
        let row = Row {
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
            machine: None,
            material: Some("Softwood".into()),
        };
        let t = row.tool().unwrap();
        assert_eq!(t.name, "End Mill (0.25in) DOWN CUT [Softwood]");
        assert_eq!(t.shape, ToolShape::EndMill);
        assert!((t.diameter - 6.35).abs() < 1e-9);
        assert!((t.feed_rate - 2540.0).abs() < 1e-9, "in/min to mm/min");
        assert!((t.pass_depth - 3.175).abs() < 1e-9);
        assert!((t.stepover - 0.4).abs() < 1e-9, "0.1 in of a 0.25 in tool");
        assert_eq!(t.flutes, 2);
    }

    /// Reads whichever vendor databases are in `~/Downloads` (not part of the repo).
    #[test]
    fn vendor_databases_read_when_present() {
        let Some(home) = std::env::var_os("HOME") else {
            return;
        };
        let dir = Path::new(&home).join("Downloads");
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for e in entries.flatten() {
            let p = e.path();
            if p.extension().is_some_and(|x| x == "vtdb") {
                let tools = read(&p).unwrap_or_else(|err| panic!("{}: {err}", p.display()));
                assert!(tools.len() > 10, "{}", p.display());
                assert!(tools.iter().all(|t| t.diameter > 0.0 && t.feed_rate > 0.0));
            }
        }
        assert!(read(Path::new("/nonexistent.vtdb")).is_err());
    }
}
