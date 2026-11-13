//! Part Labels (the Advanced Machining Module's Print Part Labels, made plain): a label for
//! each part of the job, as an HTML page the browser prints, so every part cut can be told
//! apart on the bench. A label has the part's name, its number ("3 of 12"), its size, its
//! sheet and the job's name, and a QR code holding the same words, which any phone camera
//! reads. Label printers get one label a page at the label's size; plain paper gets the
//! labels in a grid with lines to cut along.

use cut_craft_core::document::Document;
use cut_craft_core::{CommandError, Exporter, FileFormat};
use fmt_cut_list::{Part, number, parts, unit};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt::Write as _;

/// The label sizes offered: common label-printer rolls, and plain paper.
// ponytail: our own pick of sizes; these are the
// common rolls of 4-inch, Dymo / Zebra and Brother printers [inf]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Size {
    /// Labels in a grid on Letter or A4 paper, with dashed lines to cut along
    #[default]
    #[schemars(title = "Plain paper (cut out with scissors)")]
    PlainPaper,
    #[schemars(title = "4 × 6 in (shipping label printer)")]
    In4x6,
    #[schemars(title = "4 × 3 in")]
    In4x3,
    #[schemars(title = "4 × 2 in")]
    In4x2,
    #[schemars(title = "3 × 2 in")]
    In3x2,
    #[schemars(title = "2.25 × 1.25 in (Dymo, Zebra)")]
    In2x1,
    #[schemars(title = "100 × 50 mm")]
    Mm100x50,
    #[schemars(title = "62 × 29 mm (Brother)")]
    Mm62x29,
}

impl Size {
    /// Width and height, mm.
    pub fn mm(self) -> (f64, f64) {
        let inch = |w: f64, h: f64| (w * 25.4, h * 25.4);
        match self {
            Self::PlainPaper => (90.0, 50.0),
            Self::In4x6 => inch(4.0, 6.0),
            Self::In4x3 => inch(4.0, 3.0),
            Self::In4x2 => inch(4.0, 2.0),
            Self::In3x2 => inch(3.0, 2.0),
            Self::In2x1 => inch(2.25, 1.25),
            Self::Mm100x50 => (100.0, 50.0),
            Self::Mm62x29 => (62.0, 29.0),
        }
    }
}

/// How the labels look, kept with the options.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
#[expect(clippy::struct_excessive_bools, reason = "the form's check boxes")]
pub struct Style {
    /// Pick the size of the labels in your printer; plain paper works on any printer
    #[schemars(title = "Label size")]
    pub size: Size,
    /// A square code a phone camera reads: it holds the same words as the label
    #[schemars(title = "QR code")]
    pub qr_code: bool,
    #[schemars(title = "Part size")]
    pub part_size: bool,
    /// Which sheet of material the part is cut from
    #[schemars(title = "Sheet name")]
    pub sheet: bool,
    /// The job's file name on every label
    #[schemars(title = "Job name")]
    pub job: bool,
}

impl Default for Style {
    fn default() -> Self {
        Self {
            size: Size::PlainPaper,
            qr_code: true,
            part_size: true,
            sheet: true,
            job: true,
        }
    }
}

/// What a label says, line by line: the name first.
pub fn lines(doc: &Document, part: &Part, count: usize, style: &Style, job: &str) -> Vec<String> {
    let units = doc.job.units;
    let mut out = vec![part.title(), format!("{} of {count}", part.number)];
    if style.part_size {
        out.push(format!(
            "{} × {} × {} {}",
            number(part.length, units),
            number(part.width, units),
            number(part.thickness, units),
            unit(units)
        ));
    }
    if style.sheet {
        out.push(part.sheet.clone());
    }
    if style.job && !job.is_empty() {
        out.push(job.to_owned());
    }
    out
}

/// The labels of every part of `doc` as an HTML page to print; `job` is the job's name.
pub fn labels(doc: &Document, style: &Style, job: &str) -> Result<String, CommandError> {
    let found = parts(doc);
    if found.is_empty() {
        return Err(CommandError::Failed(
            "The job has no parts to label. Draw or import closed shapes first.".into(),
        ));
    }
    let (w, h) = style.size.mm();
    let paper = style.size == Size::PlainPaper;
    // the text sized to the label: the name biggest, the rest smaller, all in millimetres
    // the code as tall as the label, but no more than two fifths of its width; the words in
    // what is left, the name biggest
    let pad = (h * 0.06).max(1.5);
    let code = if style.qr_code {
        (h - 2.0 * pad).min(w * 0.4)
    } else {
        0.0
    };
    let room = w - code - 3.0 * pad;
    let name_size = (h * 0.2).min(room * 0.14);
    let page = if paper {
        "@page{margin:10mm}body{display:flex;flex-wrap:wrap;gap:0}\
         .label{border:0.3mm dashed #888}"
            .to_owned()
    } else {
        format!(
            "@page{{size:{w}mm {h}mm;margin:0}}.label{{break-after:page}}\
             .label:last-child{{break-after:auto}}"
        )
    };
    let mut html = format!(
        "<!doctype html><html><head><meta charset=\"utf-8\"><title>Part labels{}</title><style>\
         body{{margin:0;font-family:Arial,Helvetica,sans-serif}}\
         .label{{box-sizing:border-box;width:{w}mm;height:{h}mm;padding:{pad}mm;display:flex;\
         gap:{pad}mm;align-items:center;overflow:hidden;page-break-inside:avoid}}\
         .label svg{{width:{code:.2}mm;height:{code:.2}mm;flex:none}}.text{{min-width:0}}\
         .text div{{white-space:nowrap;overflow:hidden;text-overflow:ellipsis;font-size:{small:.2}mm}}\
         .text .name{{font-weight:bold;font-size:{name_size:.2}mm}}\
         {page}\
         @media screen{{body{{background:#eee;padding:4mm}}.label{{background:#fff;margin:2mm}}}}\
         </style></head><body>",
        if job.is_empty() {
            String::new()
        } else {
            format!(" - {}", escape(job))
        },
        small = name_size * 0.65,
    );
    for part in &found {
        let words = lines(doc, part, found.len(), style, job);
        html.push_str("<div class=\"label\">");
        if style.qr_code {
            html.push_str(&qr(&words.join("\n"))?);
        }
        html.push_str("<div class=\"text\">");
        for (i, line) in words.iter().enumerate() {
            let class = if i == 0 { " class=\"name\"" } else { "" };
            let _ = write!(html, "<div{class}>{}</div>", escape(line));
        }
        html.push_str("</div></div>");
    }
    html.push_str("</body></html>\n");
    Ok(html)
}

/// `text` as a QR code drawn in SVG, with the quiet border round it scanners need.
fn qr(text: &str) -> Result<String, CommandError> {
    let code = qrcode::QrCode::new(text.as_bytes())
        .map_err(|e| CommandError::Failed(format!("the label's QR code: {e}")))?;
    let side = code.width();
    let quiet = 2;
    let mut path = String::new();
    for (i, colour) in code.to_colors().into_iter().enumerate() {
        if colour == qrcode::Color::Dark {
            let _ = write!(path, "M{} {}h1v1h-1z", i % side + quiet, i / side + quiet);
        }
    }
    let all = side + 2 * quiet;
    Ok(format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {all} {all}\" \
         shape-rendering=\"crispEdges\"><rect width=\"{all}\" height=\"{all}\" fill=\"#fff\"/>\
         <path d=\"{path}\" fill=\"#000\"/></svg>"
    ))
}

fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// The labels with the default style and no job name: for the exporter list.
pub struct PartLabels;

impl Exporter for PartLabels {
    const ID: &'static str = "part_labels";
    const FORMAT: FileFormat = FileFormat {
        label: "Part labels (open in a web browser to print)",
        extensions: &["html"],
    };

    fn export(doc: &Document) -> Result<Vec<u8>, CommandError> {
        Ok(labels(doc, &Style::default(), "")?.into_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::Importer;

    fn kitchen() -> Document {
        let mut doc = Document::default();
        let change = fmt_cut_list::ImportCutList::import(
            &doc,
            b"Name,Length,Width,Qty\nSide <left>,720,560,2\nShelf,564,540,1\n",
        )
        .unwrap();
        doc.apply(&change).unwrap();
        doc
    }

    #[test]
    fn a_label_for_each_part_with_its_name_number_size_sheet_and_job() {
        let html = labels(&kitchen(), &Style::default(), "Kitchen.ccj").unwrap();
        assert_eq!(html.matches("<div class=\"label\">").count(), 3);
        for words in [
            "Side &lt;left&gt;",
            "1 of 3",
            "3 of 3",
            "720 × 560 × 20 mm",
            "Sheet 1",
            "Kitchen.ccj",
            "<svg",
        ] {
            assert!(html.contains(words), "missing {words}");
        }
    }

    #[test]
    fn the_style_leaves_out_what_is_not_wanted_and_sizes_the_page() {
        let style = Style {
            size: Size::Mm62x29,
            qr_code: false,
            part_size: false,
            sheet: false,
            job: true,
        };
        let html = labels(&kitchen(), &style, "").unwrap();
        assert!(!html.contains("<svg") && !html.contains(" mm</div>"));
        assert!(!html.contains("Sheet 1"));
        assert!(html.contains("@page{size:62mm 29mm;margin:0}"));
        let paper = labels(&kitchen(), &Style::default(), "").unwrap();
        assert!(paper.contains("dashed"), "plain paper has cut lines");
    }

    #[test]
    fn the_qr_code_holds_the_label_s_words() {
        let doc = kitchen();
        let found = parts(&doc);
        let part = found.iter().find(|p| p.title() == "Side <left>").unwrap();
        let words = lines(&doc, part, 3, &Style::default(), "Job");
        assert_eq!(words[0], "Side <left>");
        let svg = qr(&words.join("\n")).unwrap();
        assert!(svg.starts_with("<svg") && svg.contains("h1v1h-1z"));
        // the same words make the same code
        assert_eq!(svg, qr(&words.join("\n")).unwrap());
    }

    #[test]
    fn an_empty_job_says_what_to_do() {
        let e = PartLabels::export(&Document::default())
            .unwrap_err()
            .to_string();
        assert!(e.contains("Draw or import"), "{e}");
    }

    #[test]
    fn the_default_style_round_trips_and_older_options_fill_in() {
        let style: Style = serde_json_value(r#"{"size":"in4x6"}"#);
        assert_eq!(style.size, Size::In4x6);
        assert!(style.qr_code);
    }

    fn serde_json_value(s: &str) -> Style {
        serde_json::from_str(s).unwrap()
    }
}
