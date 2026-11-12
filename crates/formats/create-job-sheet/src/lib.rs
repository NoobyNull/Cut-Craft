//! Create Job Sheet: an HTML page for the machine operator, in Aspire's five sections: the
//! job layout, the job notes, the material setup, the toolpaths summary and each toolpath.
//! The Job Setup Sheet Editor's [`Style`] sets its title, logo, header and footer colours and
//! which sections it has. A job of several sheets gets a page for each sheet with toolpaths,
//! and a double-sided job's page names its side.

use base64::Engine as _;
use cut_craft_core::document::{Datum, Document, Flip, Units, ZZero};
use cut_craft_core::toolpath::Toolpath;
use cut_craft_core::{Change, CommandError, Exporter, FileFormat};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt::Write as _;

pub struct JobSheet;

/// How a file the sheet names but this computer does not have is printed: red.
const MISSING: &str = "<span class=\"missing\">NO LOCAL FILE</span>";

/// The rapid rate the time estimates assume, mm/min.
// ponytail: Toolpath Summary's default; the sheet does not read the window's own setting
const RAPID: f64 = 5000.0;

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

/// What the sheet says about where the job is cut, beyond the job itself: all empty (and
/// the default rapid rate) leaves those rows out.
#[derive(Clone, Debug, PartialEq)]
pub struct Setup {
    /// Where the job's own file is; empty when it has none on this computer (never saved,
    /// or moved since), which the sheet prints as NO LOCAL FILE in red.
    pub job_file: String,
    /// Where a job without a file of its own had a copy kept (the recovery file), if any.
    pub job_copy: String,
    /// The machine it is cut on.
    pub machine: String,
    /// The post-processor its toolpaths are saved through.
    pub post: String,
    /// The toolpath files saved for it, in the order they are run, each with whether it is
    /// still there; none (or one not there) is printed as NO LOCAL FILE in red.
    pub outputs: Vec<(String, bool)>,
    /// The machine's tool changer: the tool in each slot by name, slot 1 first; empty for
    /// none, when the tools are numbered as the post numbers them.
    pub slots: Vec<Option<String>>,
    /// When the sheet was made, as it is printed.
    pub printed: String,
    /// The rapid rate the run times assume, mm/min.
    pub rapid: f64,
    /// The machine's X, Y, Z accelerations, mm/s², for the run times (0 for none).
    pub acceleration: [f64; 3],
}

impl Default for Setup {
    fn default() -> Self {
        Self {
            job_file: String::new(),
            job_copy: String::new(),
            machine: String::new(),
            post: String::new(),
            outputs: vec![],
            slots: vec![],
            printed: String::new(),
            rapid: RAPID,
            acceleration: [0.0; 3],
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
        Ok(sheet(doc, &Style::default())?.into_bytes())
    }
}

/// A page for each sheet of `doc` with toolpaths (the active sheet's when it has only one),
/// each with what goes after the file's name: the sheet's name when there are several, and
/// `_Top` or `_Bottom` in a double-sided job.
pub fn sheets(
    doc: &Document,
    style: &Style,
    setup: &Setup,
) -> Result<Vec<(String, String)>, CommandError> {
    let side = |d: &Document| {
        d.job
            .double_sided
            .map_or_default(|_| if d.bottom_side { "_Bottom" } else { "_Top" })
    };
    if doc.sheets.len() <= 1 {
        return Ok(vec![(side(doc).to_owned(), sheet_for(doc, style, setup)?)]);
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
        out.push((
            format!("_{name}{}", side(&on)),
            sheet_for(&on, style, setup)?,
        ));
    }
    Ok(out)
}

/// The page for `doc`'s active sheet and side, as `style` has it.
pub fn sheet(doc: &Document, style: &Style) -> Result<String, CommandError> {
    sheet_for(doc, style, &Setup::default())
}

/// The page for `doc`'s active sheet and side, as `style` has it, cut as `setup` says.
#[expect(
    clippy::too_many_lines,
    reason = "the page's sections in the order printed"
)]
pub fn sheet_for(doc: &Document, style: &Style, setup: &Setup) -> Result<String, CommandError> {
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
         header h1{{margin:0}}@media print{{body{{max-width:none}}header,footer{{print-color-adjust:exact;\
         -webkit-print-color-adjust:exact}}h3,table{{break-inside:avoid}}.missing{{print-color-adjust:exact;-webkit-print-color-adjust:exact}}}}\
         .missing{{color:#c00;font-weight:bold}}</style></head><body><header>",
        hex(style.header),
        hex(style.footer),
    );
    if !style.logo.is_empty() {
        let _ = write!(h, "<img src=\"{}\" alt=\"Logo\">", escape(&style.logo));
    }
    let _ = write!(h, "<h1>{title}</h1></header>");
    let visible: Vec<_> = doc.toolpaths.values().filter(|t| t.visible).collect();
    let minutes = |t: &Toolpath| t.machining_minutes_accelerating(setup.rapid, setup.acceleration);
    let total: f64 = visible.iter().map(|t| minutes(t)).sum();
    let tools = cut_craft_post::tools_and_numbers(&visible, &setup.slots);
    let number = |name: &str| tools.iter().find(|(t, _)| t == name).map_or(0, |&(_, n)| n);
    let has_changer = !setup.slots.is_empty();
    let slot_heading = if has_changer { "Slot" } else { "Tool number" };
    // on a machine with a tool changer, its slot or a change by hand; else the post's T
    let slot = |n: u32| match (has_changer, n) {
        (false, n) => format!("T{n}"),
        (true, 0) => "By hand".to_owned(),
        (true, n) => n.to_string(),
    };
    let changes = visible
        .windows(2)
        .filter(|pair| pair[0].tool.name != pair[1].tool.name)
        .count();
    h.push_str("<h2>Job</h2><table>");
    let job_file = match (setup.job_file.as_str(), setup.job_copy.as_str()) {
        ("", "") => format!("{MISSING}<br>The job has not been saved"),
        ("", copy) => format!(
            "{MISSING}<br>The job has not been saved; a copy is kept at {}",
            escape(copy)
        ),
        (file, _) => escape(file),
    };
    let outputs = if setup.outputs.is_empty() {
        format!("{MISSING}<br>The toolpaths have not been saved")
    } else {
        setup
            .outputs
            .iter()
            .map(|(name, found)| {
                let missing = if *found { "" } else { MISSING };
                format!("{} {missing}", escape(name)).trim_end().to_owned()
            })
            .collect::<Vec<_>>()
            .join("<br>")
    };
    for (heading, value) in [
        ("Job file", job_file),
        ("Machine", escape(&setup.machine)),
        ("Post-processor", escape(&setup.post)),
        ("Toolpath files", outputs),
        ("Printed", escape(&setup.printed)),
    ] {
        if !value.is_empty() {
            let _ = write!(h, "<tr><th>{heading}</th><td>{value}</td></tr>");
        }
    }
    let _ = write!(
        h,
        "<tr><th>Toolpaths</th><td>{}</td></tr><tr><th>Tools</th><td>{}</td></tr>\
         <tr><th>Tool changes</th><td>{changes}</td></tr><tr><th>Total run time</th><td>{}</td></tr></table>",
        visible.len(),
        tools.len(),
        hms(total),
    );
    if !tools.is_empty() {
        let _ = write!(
            h,
            "<h2>Tools</h2><table><tr><th>{slot_heading}</th><th>Tool</th><th>Diameter</th>\
             <th>Toolpaths</th><th>Run time</th></tr>"
        );
        for (name, n) in &tools {
            let using: Vec<_> = visible.iter().filter(|t| &t.tool.name == name).collect();
            let names = using
                .iter()
                .map(|t| escape(&t.name))
                .collect::<Vec<_>>()
                .join(", ");
            let time: f64 = using.iter().map(|t| minutes(t)).sum();
            let diameter = using
                .first()
                .map_or(String::new(), |t| len(t.tool.diameter));
            let _ = write!(
                h,
                "<tr><td>{}</td><td>{}</td><td>{diameter}</td><td>{names}</td><td>{}</td></tr>",
                slot(*n),
                escape(name),
                hms(time),
            );
        }
        h.push_str("</table>");
    }
    if style.layout {
        let layout = String::from_utf8(fmt_export_svg::ExportSvg::export(doc)?)
            .unwrap_or_default()
            .split_once("<svg")
            .map_or_default(|(_, svg)| format!("<svg{svg}"));
        let _ = write!(h, "<h2>Job Layout</h2>{layout}");
    }
    if style.notes && !doc.notes.is_empty() {
        let _ = write!(h, "<h2>Job Notes</h2><pre>{}</pre>", escape(&doc.notes));
    }
    if style.material {
        material(&mut h, doc, &len);
    }
    if style.summary {
        let _ = write!(
            h,
            "<h2>Toolpaths Summary</h2><p>{} toolpaths, estimated time {}</p><table><tr><th>#</th>\
             <th>Toolpath</th><th>{slot_heading}</th><th>Tool</th><th>Run time</th></tr>",
            visible.len(),
            hms(total)
        );
        for (i, t) in visible.iter().enumerate() {
            let _ = write!(
                h,
                "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
                i + 1,
                escape(&t.name),
                slot(number(&t.tool.name)),
                escape(&t.tool.name),
                hms(minutes(t)),
            );
        }
        h.push_str("</table>");
    }
    for t in visible.iter().filter(|_| style.toolpaths) {
        let tool = &t.tool;
        let _ = write!(
            h,
            "<h3>{}</h3><table><tr><th>Tool</th><td>{}</td></tr><tr><th>{slot_heading}</th><td>{}</td></tr>\
             <tr><th>Diameter</th><td>{}</td></tr>\
             <tr><th>Feed rate</th><td>{:.0} mm/min</td></tr><tr><th>Plunge rate</th><td>{:.0} mm/min</td></tr>\
             <tr><th>Spindle speed</th><td>{:.0} rpm</td></tr><tr><th>Estimated time</th><td>{}</td></tr></table>",
            escape(&t.name),
            escape(&tool.name),
            slot(number(&tool.name)),
            len(tool.diameter),
            tool.feed_rate,
            tool.plunge_rate,
            tool.spindle_speed,
            hms(minutes(t)),
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
            let setup = Setup {
                rapid,
                ..Setup::default()
            };
            let page = sheet_for(&doc, &Style::default(), &setup).unwrap();
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
        let html = sheet(&doc, &style).unwrap();
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
        let pages = sheets(&doc, &Style::default(), &Setup::default()).unwrap();
        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].0, "_Top");
        assert!(pages[0].1.contains("<th>Side</th><td>Top</td>"));
        // a second sheet without toolpaths gets no page; with the first one, two pages
        let mut two = cut_craft_core::testing::two_sheets();
        two.toolpaths = doc.toolpaths.clone();
        let pages = sheets(&two, &Style::default(), &Setup::default()).unwrap();
        assert_eq!(pages.len(), 1, "only the sheet with toolpaths");
        assert!(pages[0].0.starts_with('_'));
    }

    #[test]
    fn the_printout_names_the_files_machine_run_times_and_each_tool_s_slot() {
        let mut doc = cut_craft_core::testing::with_toolpath();
        let first = doc.toolpaths.values().next().unwrap().clone();
        let mut carve = first.clone();
        carve.name = "Carve".into();
        carve.tool.name = "60° V-Bit".into();
        doc.toolpaths
            .insert(cut_craft_core::document::EntityId(u64::MAX), carve);
        let setup = Setup {
            job_file: "/jobs/Sign.ccraft".into(),
            machine: "Shop CNC".into(),
            post: "FluidNC ATC (mm)".into(),
            outputs: vec![("Sign_T1.nc".into(), true), ("Sign_T2.nc".into(), false)],
            slots: vec![None, Some("60° V-Bit".into())],
            printed: "2026-11-05 09:30".into(),
            ..Setup::default()
        };
        let html = sheet_for(&doc, &Style::default(), &setup).unwrap();
        for part in [
            "<th>Job file</th><td>/jobs/Sign.ccraft</td>",
            "<th>Machine</th><td>Shop CNC</td>",
            "<th>Post-processor</th><td>FluidNC ATC (mm)</td>",
            "<th>Toolpath files</th><td>Sign_T1.nc<br>Sign_T2.nc <span class=\"missing\">NO LOCAL FILE</span></td>",
            "<th>Printed</th><td>2026-11-05 09:30</td>",
            "<th>Total run time</th>",
            "<h2>Tools</h2>",
            "<th>Slot</th>",
            // the V-bit is in slot 2; the first tool is not in the changer: by hand
            "<td>2</td><td>60° V-Bit</td>",
            "<td>By hand</td>",
            "<th>Tool changes</th><td>1</td>",
            "<th>Slot</th><td>2</td>",
            "@media print",
        ] {
            assert!(html.contains(part), "missing {part}");
        }
        // without a tool changer the column is the tool number, in the order used
        let plain = sheet_for(&doc, &Style::default(), &Setup::default()).unwrap();
        assert!(plain.contains("<th>Tool number</th>") && plain.contains("<td>T2</td>"));
        assert!(
            !plain.contains("<th>Machine</th>"),
            "nothing to say: no row"
        );
        // an unsaved job and unsaved toolpaths are flagged in red, with where a copy is kept
        assert!(plain.contains(
            "<th>Job file</th><td><span class=\"missing\">NO LOCAL FILE</span><br>The job has not been saved</td>"
        ));
        assert!(
            plain.contains(
                "<th>Toolpath files</th><td><span class=\"missing\">NO LOCAL FILE</span>"
            )
        );
        let kept = Setup {
            job_copy: "/config/recovery.ccjob".into(),
            ..Setup::default()
        };
        let html = sheet_for(&doc, &Style::default(), &kept).unwrap();
        assert!(html.contains("a copy is kept at /config/recovery.ccjob"));
        assert!(html.contains(".missing{color:#c00"));
    }
}
