//! Create Job Sheet: an HTML page for the machine operator, in Aspire's five sections: the
//! job layout, the job notes, the material setup, the toolpaths summary and each toolpath.

use cut_craft_core::document::{Datum, Document, Units, ZZero};
use cut_craft_core::{CommandError, Exporter, FileFormat};
use std::fmt::Write as _;

pub struct JobSheet;

/// The rapid rate the time estimates assume, mm/min.
// ponytail: Toolpath Summary's default; the sheet does not read the window's own setting
const RAPID: f64 = 5000.0;

impl Exporter for JobSheet {
    const ID: &'static str = "create_job_sheet";
    const FORMAT: FileFormat = FileFormat {
        label: "Job sheet",
        extensions: &["html"],
    };

    fn export(doc: &Document) -> Result<Vec<u8>, CommandError> {
        let job = &doc.job;
        let len = |mm: f64| match job.units {
            Units::Mm => format!("{mm:.2} mm"),
            Units::Inch => format!("{:.3} in", mm / 25.4),
        };
        let layout = String::from_utf8(fmt_export_svg::ExportSvg::export(doc)?)
            .unwrap_or_default()
            .split_once("<svg")
            .map(|(_, svg)| format!("<svg{svg}"))
            .unwrap_or_default();
        let mut h = String::from(
            "<!doctype html><html><head><meta charset=\"utf-8\"><title>Job Setup Sheet</title>\
             <style>body{font-family:sans-serif;max-width:60em;margin:auto}svg{max-width:100%;height:auto;border:1px solid #888}\
             table{border-collapse:collapse}td,th{border:1px solid #888;padding:.2em .6em;text-align:left}</style></head><body>\
             <h1>Job Setup Sheet</h1>",
        );
        let _ = write!(h, "<h2>Job Layout</h2>{layout}");
        if !doc.notes.is_empty() {
            let _ = write!(h, "<h2>Job Notes</h2><pre>{}</pre>", escape(&doc.notes));
        }
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
             <tr><th>Safe Z</th><td>{}</td></tr></table>",
            len(job.width),
            len(job.height),
            len(job.thickness),
            len(job.clearance),
        );
        let visible: Vec<_> = doc.toolpaths.values().filter(|t| t.visible).collect();
        let total: f64 = visible.iter().map(|t| t.machining_minutes(RAPID)).sum();
        let _ = write!(
            h,
            "<h2>Toolpaths Summary</h2><p>{} toolpaths, estimated time {}</p>",
            visible.len(),
            hms(total)
        );
        for t in visible {
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
                hms(t.machining_minutes(RAPID)),
            );
        }
        h.push_str("</body></html>\n");
        Ok(h.into_bytes())
    }
}

fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
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
}
