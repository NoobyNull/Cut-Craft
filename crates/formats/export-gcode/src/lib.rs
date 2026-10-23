//! Saves toolpaths as generic metric G-code (RS-274): G21/G90, G0 rapids, G1 feeds with modal
//! F, spindle on per toolpath and a program pause (M0) for each manual tool change.

use cut_craft_core::document::Document;
use cut_craft_core::toolpath::MoveKind;
use cut_craft_core::{CommandError, Exporter, FileFormat};

pub struct ExportGcode;

impl Exporter for ExportGcode {
    const ID: &'static str = "export_gcode";
    const FORMAT: FileFormat = FileFormat {
        label: "G-code",
        extensions: &["nc", "gcode", "tap"],
    };

    fn export(doc: &Document) -> Result<Vec<u8>, CommandError> {
        if doc.toolpaths.is_empty() {
            return Err(CommandError::Failed(
                "there are no toolpaths to save".into(),
            ));
        }
        let mut out = Gcode::default();
        out.line("(Cut Craft)");
        out.line("G21 G90 G17 (millimetres, absolute, XY plane)");
        let mut tool: Option<&str> = None;
        for toolpath in doc.toolpaths.values() {
            if tool.is_some_and(|t| t != toolpath.tool.name) {
                out.line("M5");
                out.line(&format!("M0 (change to {})", comment(&toolpath.tool.name)));
            }
            tool = Some(&toolpath.tool.name);
            out.line(&format!(
                "({} - {})",
                comment(&toolpath.name),
                comment(&toolpath.tool.name)
            ));
            out.line(&format!("S{:.0} M3", toolpath.tool.spindle_speed));
            for m in &toolpath.moves {
                let xyz = format!("X{} Y{} Z{}", coord(m.to.x), coord(m.to.y), coord(m.to.z));
                match m.kind {
                    MoveKind::Rapid => out.line(&format!("G0 {xyz}")),
                    MoveKind::Plunge => out.feed(&xyz, toolpath.tool.plunge_rate),
                    MoveKind::Cut => out.feed(&xyz, toolpath.tool.feed_rate),
                }
            }
        }
        out.line(&format!("G0 Z{}", coord(doc.job.clearance)));
        out.line("M5");
        out.line("M30");
        Ok(out.text.into_bytes())
    }
}

/// G-code text being written, remembering the modal feed rate.
#[derive(Default)]
struct Gcode {
    text: String,
    feed: Option<f64>,
}

impl Gcode {
    fn line(&mut self, line: &str) {
        self.text.push_str(line);
        self.text.push('\n');
    }

    /// A G1 move; F is written only when the rate changes.
    fn feed(&mut self, xyz: &str, rate: f64) {
        let line = if self.feed == Some(rate) {
            format!("G1 {xyz}")
        } else {
            format!("G1 {xyz} F{rate:.1}")
        };
        self.feed = Some(rate);
        self.line(&line);
    }
}

/// Three decimals, without a negative sign on values that round to zero.
fn coord(v: f64) -> String {
    let s = format!("{v:.3}");
    if s == "-0.000" { "0.000".into() } else { s }
}

/// Brackets in place of parentheses, which would end the comment early.
fn comment(text: &str) -> String {
    text.replace('(', "[").replace(')', "]")
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::Change;
    use cut_craft_core::glam::DVec3;
    use cut_craft_core::toolpath::{Move, MoveKind, Tool, Toolpath};

    fn mv(kind: MoveKind, x: f64, y: f64, z: f64) -> Move {
        Move {
            kind,
            to: DVec3::new(x, y, z),
        }
    }

    fn toolpath(name: &str, tool: &str) -> Toolpath {
        Toolpath {
            name: name.into(),
            strategy: "profile_toolpath".into(),
            params: serde_json::json!({}),
            tool: Tool {
                name: tool.into(),
                ..Tool::default()
            },
            vectors: vec![],
            moves: vec![
                mv(MoveKind::Rapid, 0.0, 0.0, 5.0),
                mv(MoveKind::Plunge, 0.0, 0.0, -3.0),
                mv(MoveKind::Cut, 10.0, 0.0, -3.0),
                mv(MoveKind::Cut, 10.0, 10.0, -3.0),
                mv(MoveKind::Rapid, 10.0, 10.0, 5.0),
            ],
        }
    }

    fn gcode(toolpaths: Vec<Toolpath>) -> Vec<String> {
        let mut doc = Document::default();
        let mut c = Change::builder(&doc);
        for t in toolpaths {
            c.add_toolpath(t);
        }
        doc.apply(&c.finish()).unwrap();
        String::from_utf8(ExportGcode::export(&doc).unwrap())
            .unwrap()
            .lines()
            .map(String::from)
            .collect()
    }

    #[test]
    fn says_what_made_it() {
        let lines = gcode(vec![toolpath("Profile", "6 mm End Mill")]);
        assert!(lines.contains(&"(Cut Craft)".to_owned()), "{lines:?}");
    }

    #[test]
    fn starts_in_millimetres_and_absolute_coordinates() {
        let lines = gcode(vec![toolpath("Profile", "6 mm End Mill")]);
        assert!(
            lines.iter().take(3).any(|l| l.starts_with("G21 G90")),
            "{lines:#?}"
        );
    }

    #[test]
    fn rapids_are_g0_and_feeds_are_g1_with_the_rate_only_when_it_changes() {
        let lines = gcode(vec![toolpath("Profile", "6 mm End Mill")]);
        let moves: Vec<&String> = lines
            .iter()
            .filter(|l| l.starts_with("G0 X") || l.starts_with("G1 "))
            .collect();
        assert_eq!(
            moves,
            [
                "G0 X0.000 Y0.000 Z5.000",
                "G1 X0.000 Y0.000 Z-3.000 F300.0",
                "G1 X10.000 Y0.000 Z-3.000 F1000.0",
                "G1 X10.000 Y10.000 Z-3.000",
                "G0 X10.000 Y10.000 Z5.000",
            ]
        );
    }

    #[test]
    fn the_spindle_starts_at_the_tool_speed() {
        assert!(
            gcode(vec![toolpath("Profile", "6 mm End Mill")]).contains(&"S18000 M3".to_owned())
        );
    }

    #[test]
    fn ends_with_the_spindle_off_and_program_end() {
        let lines = gcode(vec![toolpath("Profile", "6 mm End Mill")]);
        assert_eq!(&lines[lines.len() - 2..], ["M5", "M30"]);
    }

    #[test]
    fn a_different_tool_pauses_for_a_change_and_the_same_tool_does_not() {
        let pauses = |l: &[String]| l.iter().filter(|l| l.starts_with("M0")).count();
        assert_eq!(
            pauses(&gcode(vec![toolpath("A", "6 mm"), toolpath("B", "3 mm")])),
            1
        );
        assert_eq!(
            pauses(&gcode(vec![toolpath("A", "6 mm"), toolpath("B", "6 mm")])),
            0
        );
    }

    #[test]
    fn names_cannot_break_out_of_comments() {
        let lines = gcode(vec![toolpath("Profile (outside)", "6 mm (2 flute)")]);
        assert!(
            lines
                .iter()
                .any(|l| l == "(Profile [outside] - 6 mm [2 flute])"),
            "{lines:#?}"
        );
    }

    #[test]
    fn nothing_to_save_is_an_error() {
        assert_eq!(
            ExportGcode::export(&Document::default())
                .unwrap_err()
                .to_string(),
            "there are no toolpaths to save"
        );
    }
}
