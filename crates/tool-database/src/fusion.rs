//! Fusion 360 tool libraries (`.tools`): a zip holding `tools.json`, one entry per tool with
//! its geometry and a preset of feeds and speeds per material. Vendors publish these too
//! (`SpeTool`), so Cut Craft reads them into its own tool database.
//!
//! Learned from one vendor file: `unit` is `inches` or `millimeters`, for lengths and
//! per-minute rates; `geometry` `DC` cutting diameter, `NOF` flutes, `RE` corner radius, `TA`
//! half the included angle (a tapered mill's taper), `LCF` flute length, `SFDM` shank
//! diameter; `type` is Fusion's tool kind. All [inf].
//!
//! Some `.tools` files are plain JSON instead: groups (`Bits`) of tools keyed by id, lengths
//! in mm and rates in mm/s whatever `MetricTool` says. Also [inf], from one file (Amana's).

use cut_craft_core::toolpath::{Tool, ToolDatabase, ToolShape};
use serde_json::Value;
use std::io::Read;

const INCH: f64 = 25.4;
/// Zip's end of central directory record, and a central directory file header.
const END_OF_DIRECTORY: u32 = 0x0605_4b50;
const DIRECTORY_ENTRY: u32 = 0x0201_4b50;
const DEFLATE: u16 = 8;

/// The tools in a Fusion 360 library that Cut Craft has a shape for, one per preset (the
/// name says which material), grouped by Fusion's kind of tool.
pub fn read(bytes: &[u8]) -> Result<ToolDatabase, String> {
    if let Ok(plain) = serde_json::from_slice::<Value>(bytes) {
        return read_plain(&plain);
    }
    let json = unzip(bytes, "tools.json")?;
    let library: Value = serde_json::from_slice(&json).map_err(|e| e.to_string())?;
    let entries = library["data"]
        .as_array()
        .ok_or("no tools in the library")?;
    let tools: Vec<Tool> = entries.iter().flat_map(tools).collect();
    if tools.is_empty() {
        return Err("no tools Cut Craft can use were found in the file".into());
    }
    Ok(ToolDatabase {
        tools,
        groups: vec![],
    })
}

/// A plain JSON library: each group's tools, in their `Index` order.
fn read_plain(library: &Value) -> Result<ToolDatabase, String> {
    let groups = library.as_object().ok_or("not a tool library")?;
    let mut tools = vec![];
    for (group, entries) in groups {
        let mut entries: Vec<&Value> = entries
            .as_object()
            .into_iter()
            .flat_map(|e| e.values())
            .collect();
        entries.sort_by_key(|e| e["Index"].as_i64().unwrap_or(i64::MAX));
        tools.extend(entries.into_iter().filter_map(|e| plain_tool(group, e)));
    }
    if tools.is_empty() {
        return Err("no tools Cut Craft can use were found in the file".into());
    }
    Ok(ToolDatabase {
        tools,
        groups: vec![],
    })
}

fn plain_tool(group: &str, e: &Value) -> Option<Tool> {
    let num = |key: &str| e[key].as_f64();
    let text = |key: &str| e[key].as_str().unwrap_or_default().trim().to_owned();
    let shape = match e["Type"].as_str()? {
        "End Mill" => ToolShape::EndMill,
        "Ball Mill" | "Tapered Ball Mill" => ToolShape::BallNose,
        "V-Bit" | "Vee Bit" | "Engraver" => ToolShape::VBit,
        _ => return None,
    };
    let diameter = num("Diameter").filter(|d| *d > 0.0)?;
    let base = Tool::default();
    // mm/s to mm/min
    let rate = |key: &str| num(key).filter(|r| *r > 0.0).map(|r| r * 60.0);
    let name = [text("Vendor"), text("Name"), text("Type")]
        .into_iter()
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    Some(Tool {
        name,
        shape,
        diameter,
        angle: num("IncludedAngle")
            .filter(|a| *a > 0.0)
            .unwrap_or(base.angle),
        pass_depth: num("PassDepth")
            .filter(|d| *d > 0.0)
            .unwrap_or(base.pass_depth),
        stepover: num("StepOver").map_or(base.stepover, |s| (s / diameter).clamp(0.01, 1.0)),
        feed_rate: rate("FeedRate").unwrap_or(base.feed_rate),
        plunge_rate: rate("PlungeRate").unwrap_or(base.plunge_rate),
        spindle_speed: num("SpindleSpeed").unwrap_or(base.spindle_speed),
        flutes: e["FluteCount"]
            .as_u64()
            .and_then(|f| u32::try_from(f).ok())
            .filter(|f| *f > 0)
            .unwrap_or(base.flutes),
        group: group.to_owned(),
        notes: text("Notes"),
        ..base
    })
}

/// The bytes of the file `name` in the zip archive `zip`.
// ponytail: stored and deflated entries without zip64; that is what Fusion writes
fn unzip(zip: &[u8], name: &str) -> Result<Vec<u8>, String> {
    let broken = || "not a Fusion 360 tool library (a zip)".to_owned();
    let u16_at = |at: usize| {
        zip.get(at..at + 2)
            .map(|b| u16::from_le_bytes([b[0], b[1]]))
    };
    let u32_at = |at: usize| {
        zip.get(at..at + 4)
            .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    };
    let end = (0..zip.len().saturating_sub(21))
        .rev()
        .find(|&at| u32_at(at) == Some(END_OF_DIRECTORY))
        .ok_or_else(broken)?;
    let count = u16_at(end + 10).ok_or_else(broken)?;
    let mut at = u32_at(end + 16).ok_or_else(broken)? as usize;
    for _ in 0..count {
        if u32_at(at) != Some(DIRECTORY_ENTRY) {
            return Err(broken());
        }
        let field = |offset: usize| u16_at(at + offset).map(usize::from).ok_or_else(broken);
        let method = u16_at(at + 10).ok_or_else(broken)?;
        let size = u32_at(at + 20).ok_or_else(broken)? as usize;
        let (name_len, extra_len, comment_len) = (field(28)?, field(30)?, field(32)?);
        let local = u32_at(at + 42).ok_or_else(broken)? as usize;
        if zip.get(at + 46..at + 46 + name_len) == Some(name.as_bytes()) {
            let start = local
                + 30
                + usize::from(u16_at(local + 26).ok_or_else(broken)?)
                + usize::from(u16_at(local + 28).ok_or_else(broken)?);
            let data = zip.get(start..start + size).ok_or_else(broken)?;
            return match method {
                0 => Ok(data.to_vec()),
                DEFLATE => {
                    let mut out = vec![];
                    flate2::read::DeflateDecoder::new(data)
                        .read_to_end(&mut out)
                        .map_err(|e| e.to_string())?;
                    Ok(out)
                }
                _ => Err(format!("{name} is packed in a way Cut Craft cannot read")),
            };
        }
        at += 46 + name_len + extra_len + comment_len;
    }
    Err(format!("no {name} in the library"))
}

/// Fusion's kind of tool as a group name: `flat end mill` → `Flat End Mill`.
fn title(kind: &str) -> String {
    kind.split_whitespace()
        .map(|w| {
            let mut c = w.chars();
            c.next()
                .map_or_else(String::new, |f| f.to_uppercase().chain(c).collect())
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// One tool per preset (one with Cut Craft's feeds when there are none); none for a kind Cut
/// Craft has no shape for.
fn tools(entry: &Value) -> Vec<Tool> {
    let g = &entry["geometry"];
    let num = |v: &Value| v.as_f64();
    let corner = num(&g["RE"]).unwrap_or(0.0);
    let kind = entry["type"].as_str().unwrap_or_default();
    let shape = match kind {
        "tapered mill" if corner > 0.0 => ToolShape::BallNose,
        "flat end mill" | "face mill" | "tapered mill" => ToolShape::EndMill,
        "ball end mill" => ToolShape::BallNose,
        "bull nose end mill" => ToolShape::RadiusedEndMill,
        "chamfer mill" => ToolShape::VBit,
        _ => return vec![], // round-over and other form tools, drills, lathe tools
    };
    let scale = if entry["unit"] == "inches" { INCH } else { 1.0 };
    let Some(diameter) = num(&g["DC"]).filter(|d| *d > 0.0).map(|d| d * scale) else {
        return vec![];
    };
    let base = Tool::default();
    let unit = if scale == INCH { "in" } else { "mm" };
    let named = |s: &str| entry[s].as_str().unwrap_or_default().trim().to_owned();
    let name = [
        named("vendor"),
        named("product-id"),
        title(kind),
        format!("{}{unit}", trim(diameter / scale)),
    ]
    .into_iter()
    .filter(|s| !s.is_empty())
    .collect::<Vec<_>>()
    .join(" ");
    let tool = Tool {
        name: name.clone(),
        shape,
        diameter,
        tip_radius: if shape == ToolShape::RadiusedEndMill {
            corner * scale
        } else {
            0.0
        },
        angle: num(&g["TA"]).map_or(base.angle, |half| half * 2.0),
        // a tapered mill's `TA` is its taper, widening from `DC` at the tip to the shank
        taper: match kind {
            "tapered mill" => num(&g["TA"]).unwrap_or(0.0),
            _ => 0.0,
        },
        shank_diameter: num(&g["SFDM"]).unwrap_or(0.0) * scale,
        flute_length: num(&g["LCF"]).unwrap_or(0.0) * scale,
        flutes: g["NOF"]
            .as_u64()
            .and_then(|f| u32::try_from(f).ok())
            .filter(|f| *f > 0)
            .unwrap_or(base.flutes),
        group: title(kind),
        notes: named("description"),
        ..base
    };
    let presets = entry["start-values"]["presets"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    if presets.is_empty() {
        return vec![tool];
    }
    presets
        .iter()
        .map(|p| {
            let length = |key: &str| num(&p[key]).map(|v| v * scale);
            let material = p["name"].as_str().unwrap_or_default();
            Tool {
                name: if material.is_empty() {
                    name.clone()
                } else {
                    format!("{name} [{material}]")
                },
                pass_depth: length("stepdown").unwrap_or(tool.pass_depth),
                stepover: length("stepover")
                    .map_or(tool.stepover, |s| (s / diameter).clamp(0.01, 1.0)),
                feed_rate: length("v_f").unwrap_or(tool.feed_rate),
                plunge_rate: length("v_f_plunge").unwrap_or(tool.plunge_rate),
                spindle_speed: num(&p["n"]).unwrap_or(tool.spindle_speed),
                ..tool.clone()
            }
        })
        .collect()
}

fn trim(x: f64) -> String {
    let s = format!("{x:.4}");
    s.trim_end_matches('0').trim_end_matches('.').to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    /// A zip with one deflated file, as Fusion writes them.
    fn zip(name: &str, body: &[u8]) -> Vec<u8> {
        let mut packed = flate2::write::DeflateEncoder::new(vec![], flate2::Compression::default());
        packed.write_all(body).unwrap();
        let packed = packed.finish().unwrap();
        let size = u32::try_from(packed.len()).unwrap();
        let name_len = u16::try_from(name.len()).unwrap();
        let mut out = vec![];
        out.extend(0x0403_4b50_u32.to_le_bytes());
        out.extend([0; 4]);
        out.extend(DEFLATE.to_le_bytes());
        out.extend([0; 8]);
        out.extend(size.to_le_bytes());
        out.extend([0; 4]);
        out.extend(name_len.to_le_bytes());
        out.extend([0; 2]);
        out.extend(name.as_bytes());
        out.extend(&packed);
        let directory = u32::try_from(out.len()).unwrap();
        out.extend(DIRECTORY_ENTRY.to_le_bytes());
        out.extend([0; 6]);
        out.extend(DEFLATE.to_le_bytes());
        out.extend([0; 8]);
        out.extend(size.to_le_bytes());
        out.extend([0; 4]);
        out.extend(name_len.to_le_bytes());
        out.extend([0; 12]);
        out.extend(0_u32.to_le_bytes());
        out.extend(name.as_bytes());
        let directory_size = u32::try_from(out.len()).unwrap() - directory;
        out.extend(END_OF_DIRECTORY.to_le_bytes());
        out.extend([0; 6]);
        out.extend(1_u16.to_le_bytes());
        out.extend(directory_size.to_le_bytes());
        out.extend(directory.to_le_bytes());
        out.extend([0; 2]);
        out
    }

    #[test]
    fn a_tapered_mill_keeps_its_taper_shank_and_flute_length() {
        let library = serde_json::json!({ "data": [
            { "type": "tapered mill", "unit": "inches",
              "geometry": { "DC": 0.03125, "RE": 0.015_625, "TA": 6.2, "SFDM": 0.25, "LCF": 1 } }
        ]});
        let db = read(&zip("tools.json", library.to_string().as_bytes())).unwrap();
        let t = &db.tools[0];
        assert_eq!(t.shape, ToolShape::BallNose);
        assert!((t.diameter - 0.793_75).abs() < 1e-9);
        assert!((t.taper - 6.2).abs() < 1e-9);
        assert!((t.shank_diameter - 6.35).abs() < 1e-9);
        assert!((t.flute_length - 25.4).abs() < 1e-9);
    }

    #[test]
    fn presets_become_tools_in_millimetres() {
        let library = serde_json::json!({ "data": [
            { "type": "flat end mill", "unit": "inches", "vendor": "Acme", "product-id": "E1",
              "description": "Up-cut spiral",
              "geometry": { "DC": 0.25, "NOF": 2 },
              "start-values": { "presets": [
                  { "name": "Softwood", "n": 18000, "v_f": 100, "v_f_plunge": 30,
                    "stepdown": 0.125, "stepover": 0.1 },
                  { "name": "MDF", "n": 16000, "v_f": 120, "v_f_plunge": 40 } ] } },
            { "type": "chamfer mill", "unit": "millimeters", "geometry": { "DC": 12, "TA": 45 } },
            { "type": "radius mill", "unit": "inches", "geometry": { "DC": 0.5, "RE": 0.25 } }
        ]});
        let db = read(&zip("tools.json", library.to_string().as_bytes())).unwrap();
        assert_eq!(db.tools.len(), 3, "two presets and a V-bit; no round-over");
        let t = &db.tools[0];
        assert_eq!(t.name, "Acme E1 Flat End Mill 0.25in [Softwood]");
        assert!((t.diameter - 6.35).abs() < 1e-9 && (t.feed_rate - 2540.0).abs() < 1e-9);
        assert!((t.pass_depth - 3.175).abs() < 1e-9 && (t.stepover - 0.4).abs() < 1e-9);
        assert_eq!(
            (t.group.as_str(), t.notes.as_str()),
            ("Flat End Mill", "Up-cut spiral")
        );
        assert_eq!(db.tools[1].spindle_speed, 16000.0);
        let v = &db.tools[2];
        assert_eq!(
            (v.shape, v.angle, v.diameter),
            (ToolShape::VBit, 90.0, 12.0)
        );
        assert!(read(b"not a zip").is_err());
    }

    #[test]
    fn plain_libraries_read_in_index_order_with_rates_per_second() {
        let library = serde_json::json!({ "Bits": {
            "{b}": { "Index": 2, "Type": "Ball Mill", "Name": "B2", "Diameter": 3.0 },
            "{a}": { "Index": 1, "Type": "End Mill", "Vendor": "Acme", "Name": "A1",
                     "Diameter": 6.35, "FeedRate": 46.5, "StepOver": 2.54, "Notes": "upcut" },
            "{c}": { "Index": 3, "Type": "Drill", "Diameter": 3.0 } } });
        let db = read(library.to_string().as_bytes()).unwrap();
        assert_eq!(db.tools.len(), 2, "no drill");
        let t = &db.tools[0];
        assert_eq!(
            (t.name.as_str(), t.group.as_str()),
            ("Acme A1 End Mill", "Bits")
        );
        assert!((t.feed_rate - 2790.0).abs() < 1e-9 && (t.stepover - 0.4).abs() < 1e-9);
        assert_eq!(db.tools[1].shape, ToolShape::BallNose);
    }

    /// Reads the Fusion libraries in the folder `CUT_CRAFT_VENDOR_FILES` names (they are not
    /// part of the repo).
    #[test]
    fn vendor_libraries_read_when_given() {
        let Some(dir) = std::env::var_os("CUT_CRAFT_VENDOR_FILES") else {
            return;
        };
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for p in entries.flatten().map(|e| e.path()) {
            if p.extension().is_some_and(|x| x == "tools") {
                let db = read(&std::fs::read(&p).unwrap())
                    .unwrap_or_else(|e| panic!("{}: {e}", p.display()));
                assert!(!db.tools.is_empty(), "{}", p.display());
                assert!(
                    db.tools
                        .iter()
                        .all(|t| t.diameter > 0.0 && t.feed_rate > 0.0)
                );
            }
        }
    }
}
