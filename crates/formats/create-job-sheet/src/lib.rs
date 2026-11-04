//! Create Job Sheet: an HTML page for the machine operator, in Aspire's five sections: the
//! job layout, the job notes, the material setup, the toolpaths summary and each toolpath.
//! The Job Setup Sheet Editor's [`Style`] sets its title, logo, header and footer colours and
//! which sections it has. A job of several sheets gets a page for each sheet with toolpaths,
//! and a double-sided job's page names its side.

use base64::Engine as _;
use cut_craft_core::document::{Datum, Document, Flip, Units, ZZero};
use cut_craft_core::{Change, CommandError, Exporter, FileFormat};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt::Write as _;

pub struct JobSheet;

/// The rapid rate the time estimates assume when none is given, mm/min: Toolpath Summary's
/// starting one.
pub const RAPID: f64 = 5000.0;

/// The Job Setup Sheet Editor's settings: how every job sheet looks.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "the editor's own check boxes"
)]
pub struct Style {
    pub title: String,
    #[schemars(title = "Header color", extend("x-kind" = "colour"))]
    pub header: [u8; 3],
    #[schemars(title = "Footer color", extend("x-kind" = "colour"))]
    pub footer: [u8; 3],
    /// The logo at the header's left, as a `data:` URL; empty for none
    #[serde(default, skip_serializing_if = "String::is_empty")]
    #[schemars(skip)]
    pub logo: String,
    #[schemars(title = "Job Layout")]
    pub layout: bool,
    #[schemars(title = "Job Notes")]
    pub notes: bool,
    #[schemars(title = "Material Setup")]
    pub material: bool,
    #[schemars(title = "Toolpaths Summary")]
    pub summary: bool,
    #[schemars(title = "Individual Toolpath Summaries")]
    pub toolpaths: bool,
}

impl Default for Style {
    fn default() -> Self {
        Self {
            title: "Job Setup Sheet".into(),
            header: [52, 88, 120],
            footer: [52, 88, 120],
            logo: String::new(),
            layout: true,
            notes: true,
            material: true,
            summary: true,
            toolpaths: true,
        }
    }
}

/// `bytes` (a PNG, JPEG or GIF) as a `data:` URL for [`Style::logo`].
pub fn logo(bytes: &[u8], extension: &str) -> Result<String, String> {
    let kind = match extension.to_ascii_lowercase().as_str() {
        "png" => "png",
        "jpg" | "jpeg" => "jpeg",
        "gif" => "gif",
        other => return Err(format!("a logo is a PNG, JPEG or GIF file, not .{other}")),
    };
    let data = base64::engine::general_purpose::STANDARD.encode(bytes);
    Ok(format!("data:image/{kind};base64,{data}"))
}

impl Exporter for JobSheet {
    const ID: &'static str = "create_job_sheet";
    const FORMAT: FileFormat = FileFormat {
        label: "Job sheet",
        extensions: &["html"],
    };

    fn export(doc: &Document) -> Result<Vec<u8>, CommandError> {
        Ok(sheet(doc, &Style::default(), RAPID)?.into_bytes())
    }
}

/// A page for each sheet of `doc` with toolpaths (the active sheet's when it has only one),
/// each with what goes after the file's name: the sheet's name when there are several, and
/// `_Top` or `_Bottom` in a double-sided job. Times assume rapids at `rapid` mm/min.
pub fn sheets(
    doc: &Document,
    style: &Style,
    rapid: f64,
) -> Result<Vec<(String, String)>, CommandError> {
    let side = |d: &Document| {
        d.job
            .double_sided
            .map(|_| if d.bottom_side { "_Bottom" } else { "_Top" })
            .unwrap_or_default()
    };
    if doc.sheets.len() <= 1 {
        return Ok(vec![(side(doc).to_owned(), sheet(doc, style, rapid)?)]);
    }
    let mut out = vec![];
    for (i, s) in doc.sheets.iter().enumerate() {
        let mut on = doc.clone();
        let mut c = Change::builder(&on);
        c.activate_sheet(i);
        on.apply(&c.finish())
            .map_err(|e| CommandError::Failed(e.to_string()))?;
        if on.toolpaths.is_empty() {
            continue;
        }
        let name: String = s
            .name
            .chars()
            .map(|ch| if ch.is_alphanumeric() { ch } else { '_' })
            .collect();
        out.push((format!("_{name}{}", side(&on)), sheet(&on, style, rapid)?));
    }
    Ok(out)
}

/// The page for `doc`'s active sheet and side, as `style` has it, its times assuming rapids
/// at `rapid` mm/min.
pub fn sheet(doc: &Document, style: &Style, rapid: f64) -> Result<String, CommandError> {
    let job = &doc.job;
    let len = |mm: f64| match job.units {
        Units::Mm => format!("{mm:.2} mm"),
        Units::Inch => format!("{:.3} in", mm / 25.4),
    };
    let hex = |[r, g, b]: [u8; 3]| format!("#{r:02x}{g:02x}{b:02x}");
    let title = escape(&style.title);
    let mut h = format!(
        "<!doctype html><html><head><meta charset=\"utf-8\"><title>{title}</title>\
         <style>body{{font-family:sans-serif;max-width:60em;margin:auto}}svg{{max-width:100%;height:auto;border:1px solid #888}}\
         table{{border-collapse:collapse}}td,th{{border:1px solid #888;padding:.2em .6em;text-align:left}}\
         header,footer{{color:#fff;padding:.6em 1em;display:flex;align-items:center;gap:1em}}\
         header{{background:{}}}footer{{background:{};margin-top:2em}}header img{{max-width:165px;max-height:52px}}\
         header h1{{margin:0}}</style></head><body><header>",
        hex(style.header),
        hex(style.footer),
    );
    if !style.logo.is_empty() {
        let _ = write!(h, "<img src=\"{}\" alt=\"Logo\">", escape(&style.logo));
    }
    let _ = write!(h, "<h1>{title}</h1></header>");
    if style.layout {
        let layout = String::from_utf8(fmt_export_svg::ExportSvg::export(doc)?)
            .unwrap_or_default()
            .split_once("<svg")
            .map(|(_, svg)| format!("<svg{svg}"))
            .unwrap_or_default();
        let _ = write!(h, "<h2>Job Layout</h2>{layout}");
    }
    if style.notes && !doc.notes.is_empty() {
        let _ = write!(h, "<h2>Job Notes</h2><pre>{}</pre>", escape(&doc.notes));
    }
    if style.material {
        material(&mut h, doc, &len);
    }
    let visible: Vec<_> = doc.toolpaths.values().filter(|t| t.visible).collect();
    if style.summary {
        let total: f64 = visible.iter().map(|t| t.machining_minutes(rapid)).sum();
        let _ = write!(
            h,
            "<h2>Toolpaths Summary</h2><p>{} toolpaths, estimated time {}</p>",
            visible.len(),
            hms(total)
        );
    }
    for t in visible.iter().filter(|_| style.toolpaths) {
        let tool = &t.tool;
        let _ = write!(
            h,
            "<h3>{}</h3><table><tr><th>Tool</th><td>{}</td></tr><tr><th>Diameter</th><td>{}</td></tr>\
             <tr><th>Feed rate</th><td>{:.0} mm/min</td></tr><tr><th>Plunge rate</th><td>{:.0} mm/min</td></tr>\
             <tr><th>Spindle speed</th><td>{:.0} rpm</td></tr><tr><th>Estimated time</th><td>{}</td></tr></table>",
            escape(&t.name),
            escape(&tool.name),
            len(tool.diameter),
            tool.feed_rate,
            tool.plunge_rate,
            tool.spindle_speed,
            hms(t.machining_minutes(rapid)),
        );
    }
    let _ = writeln!(h, "<footer>{title}</footer></body></html>");
    Ok(h)
}

/// The Material Setup section: the block, Z zero, datum, safe Z, home, and in a
/// double-sided job the side and how the material is turned over.
fn material(h: &mut String, doc: &Document, len: &dyn Fn(f64) -> String) {
    let job = &doc.job;
    let z = match job.z_zero {
        ZZero::MaterialSurface => "Material surface",
        ZZero::MachineBed => "Machine bed",
    };
    let datum = match job.datum {
        Datum::BottomLeft => "Bottom left",
        Datum::TopLeft => "Top left",
        Datum::TopRight => "Top right",
        Datum::BottomRight => "Bottom right",
        Datum::Centre => "Centre",
    };
    let _ = write!(
        h,
        "<h2>Material Setup</h2><table><tr><th>Width (X)</th><td>{}</td></tr>\
         <tr><th>Height (Y)</th><td>{}</td></tr><tr><th>Thickness (Z)</th><td>{}</td></tr>\
         <tr><th>Z zero</th><td>{z}</td></tr><tr><th>XY datum</th><td>{datum}</td></tr>\
         <tr><th>Safe Z</th><td>{}</td></tr>",
        len(job.width),
        len(job.height),
        len(job.thickness),
        len(job.clearance),
    );
    if let Some(home) = job.home {
        let _ = write!(
            h,
            "<tr><th>Home</th><td>X {} Y {}, Z {} above the material</td></tr>",
            len(home.x),
            len(home.y),
            len(home.z)
        );
    }
    if let Some(sides) = job.double_sided {
        let flip = match sides.flip {
            Flip::Horizontal => "left to right",
            Flip::Vertical => "top to bottom",
        };
        let side = if doc.bottom_side { "Bottom" } else { "Top" };
        let _ = write!(
            h,
            "<tr><th>Side</th><td>{side}</td></tr><tr><th>Flip</th><td>Turned over {flip}</td></tr>"
        );
    }
    h.push_str("</table>");
}

fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn hms(minutes: f64) -> String {
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "a positive time"
    )]
    let s = (minutes * 60.0).round() as u64;
    format!("{}:{:02}:{:02}", s / 3600, s / 60 % 60, s % 60)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_time_estimates_use_the_rapid_rate_given() {
        let mut doc = cut_craft_core::testing::with_toolpath();
        let t = doc.toolpaths.values_mut().next().unwrap();
        // a long rapid, so the rate shows in the time
        for x in [0.0, 10_000.0] {
            t.moves.push(cut_craft_core::toolpath::Move {
                kind: cut_craft_core::toolpath::MoveKind::Rapid,
                to: cut_craft_core::glam::DVec3::new(x, 0.0, 5.0),
            });
        }
        let time = |rapid| {
            let page = sheet(&doc, &Style::default(), rapid).unwrap();
            page.split("estimated time ").nth(1).unwrap()[..7].to_owned()
        };
        assert_ne!(time(1000.0), time(10_000.0), "a slower rapid takes longer");
        let exported = String::from_utf8(JobSheet::export(&doc).unwrap()).unwrap();
        assert!(exported.contains(&format!("estimated time {}", time(RAPID))));
    }

    #[test]
    fn the_sheet_has_the_layout_setup_and_each_toolpath() {
        let mut doc = cut_craft_core::testing::with_toolpath();
        doc.notes = "Glue <first>".into();
        let html = String::from_utf8(JobSheet::export(&doc).unwrap()).unwrap();
        for part in [
            "<h2>Job Layout</h2><svg",
            "Glue &lt;first&gt;",
            "<h2>Material Setup</h2>",
            "600.00 mm",
            "<h3>Profile</h3>",
            "1 toolpaths",
        ] {
            assert!(html.contains(part), "missing {part}");
        }
    }

    #[test]
    fn the_editor_s_style_sets_title_colours_logo_and_sections() {
        let mut doc = cut_craft_core::testing::with_toolpath();
        doc.notes = "Glue first".into();
        let style = Style {
            title: "Shop Sheet".into(),
            header: [255, 0, 0],
            logo: logo(b"GIF89a", "gif").unwrap(),
            notes: false,
            toolpaths: false,
            ..Style::default()
        };
        let html = sheet(&doc, &style, RAPID).unwrap();
        assert!(
            html.contains("<h1>Shop Sheet</h1>") && html.contains("header{background:#ff0000}")
        );
        assert!(html.contains("<img src=\"data:image/gif;base64,R0lGODlh\""));
        assert!(!html.contains("Glue first") && !html.contains("<h3>Profile</h3>"));
        assert!(html.contains("<h2>Toolpaths Summary</h2>"));
        assert!(logo(b"x", "bmp").is_err());
    }

    #[test]
    fn a_double_sided_job_names_its_side_and_several_sheets_get_a_page_each() {
        let mut doc = cut_craft_core::testing::with_toolpath();
        doc.job.double_sided = Some(cut_craft_core::document::DoubleSided::default());
        let pages = sheets(&doc, &Style::default(), RAPID).unwrap();
        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].0, "_Top");
        assert!(pages[0].1.contains("<th>Side</th><td>Top</td>"));
        // a second sheet without toolpaths gets no page; with the first one, two pages
        let mut two = cut_craft_core::testing::two_sheets();
        two.toolpaths = doc.toolpaths.clone();
        let pages = sheets(&two, &Style::default(), RAPID).unwrap();
        assert_eq!(pages.len(), 1, "only the sheet with toolpaths");
        assert!(pages[0].0.starts_with('_'));
    }
}
