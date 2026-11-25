//! Job notes: a little rich text, written like Markdown and never read as HTML, with live
//! variables (`{{material.thickness}}`) filled in from the job, the machine and the tools.
//!
//! The one list of variables ([`variables`]) is shared by the Notes panel's Variable
//! Explorer, the job sheet, the G-code header and the Web API, so each shows the same values.
//!
//! What the text understands, a line at a time:
//! - `# `, `## `, `### ` a heading
//! - `- ` or `* ` a bullet; `1. ` a numbered line; `- [ ] ` / `- [x] ` a check box
//! - `**bold**`, `*italic*`, `[words](https://…)` a link (only `http`, `https` and `mailto`)
//! - `{{key}}` a variable
//!
//! Anything else, `<b>` included, is shown as typed.

use crate::document::{Datum, Document, GapFrom, Units, ZZero};
use crate::toolpath::{Tool, ToolShape, Toolpath};
use std::fmt::Write as _;

/// The groups the Variable Explorer shows, in its order.
pub const GROUPS: [&str; 8] = [
    "Job",
    "Material",
    "Model",
    "Machine",
    "Bits",
    "Toolpaths",
    "Document Variables",
    "System",
];

/// Notes starting with this open by themselves when the job is opened.
pub const OPEN_MARK: char = '.';

/// One value notes can show.
#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct Variable {
    /// What is typed between `{{` and `}}`.
    pub key: String,
    /// Which of [`GROUPS`] it is listed under.
    pub group: &'static str,
    /// Where it sits under its group: a toolpath's or a tool's values sit under it (its
    /// number and name); empty for one listed straight under the group.
    pub under: String,
    /// Its name in words.
    pub label: String,
    /// What it is, one line.
    pub description: String,
    /// What it is now, as notes show it.
    pub value: String,
}

/// What the notes need that is not in the job itself: the app fills it in.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// Where the job's file is; empty when it has never been saved.
    pub job_file: String,
    /// The machine the job is cut on, its controller and the post its toolpaths are saved
    /// through; empty when none is set up.
    pub machine: String,
    pub controller: String,
    pub post: String,
    /// The machine's bed, mm (0 when not given).
    pub bed: [f64; 2],
    /// The machine's tool changer: the tool in each slot by name; empty for none.
    pub slots: Vec<Option<String>>,
    /// Today's date and the time, as they are shown.
    pub date: String,
    pub time: String,
    /// What run times go by (the rapid rate, the machine's speeds, accelerations and
    /// cornering, and bit change times), and the Toolpath Summary's scale factor.
    pub motion: crate::cut_time::Motion,
    pub scale: f64,
    /// How tall the 3D model is, mm; `None` when there is none.
    pub model_height: Option<f64>,
    /// How the machine holds its bits: collet nut, spindle and Holder per bit.
    pub holding: crate::collision::Holding,
}

impl Default for Context {
    fn default() -> Self {
        Self {
            job_file: String::new(),
            machine: String::new(),
            controller: String::new(),
            post: String::new(),
            bed: [0.0; 2],
            slots: vec![],
            date: String::new(),
            time: String::new(),
            motion: crate::cut_time::Motion::default(),
            scale: 1.0,
            model_height: None,
            holding: crate::collision::Holding::default(),
        }
    }
}

/// A collet nut's size: across and tall.
fn holder_size(holder: &crate::collision::Holder, units: Units) -> String {
    format!(
        "{} across, {} tall",
        length(holder.nut_diameter, units),
        length(holder.nut_length, units)
    )
}

/// `value` with at most three decimals and no trailing zeros.
fn trimmed(value: f64) -> String {
    let text = format!("{value:.3}");
    let text = text.trim_end_matches('0').trim_end_matches('.');
    if text == "-0" {
        "0".into()
    } else {
        text.into()
    }
}

/// A length in millimetres as the job shows it: `18 mm`, `0.75 in`.
fn length(mm: f64, units: Units) -> String {
    format!("{} {}", trimmed(mm / units.per()), units.suffix())
}

/// A feed in mm/min as the job shows it: `1500 mm/min`, `60 in/min`.
fn feed(mm_per_min: f64, units: Units) -> String {
    match units {
        Units::Mm => format!("{} mm/min", trimmed(mm_per_min)),
        Units::Inch => format!("{} in/min", trimmed(mm_per_min / 25.4)),
    }
}

/// Minutes as `1 h 05 min`, `12 min`, `40 s`.
pub fn duration(minutes: f64) -> String {
    let seconds = (minutes * 60.0).round().max(0.0);
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "rounded and not negative; a run time is far below u64::MAX seconds"
    )]
    let seconds = seconds as u64;
    match (seconds / 3600, (seconds % 3600) / 60, seconds % 60) {
        (0, 0, s) => format!("{s} s"),
        (0, m, _) => format!("{m} min"),
        (h, m, _) => format!("{h} h {m:02} min"),
    }
}

fn shape_name(shape: ToolShape) -> &'static str {
    match shape {
        ToolShape::EndMill => "End Mill",
        ToolShape::BallNose => "Ball Nose",
        ToolShape::VBit => "V-Bit",
        ToolShape::RadiusedEndMill => "Radiused End Mill",
        ToolShape::ThreadMill => "Thread Mill",
        ToolShape::FormTool => "Form Bit",
        ToolShape::SurfacingBit => "Surfacing Bit",
        ToolShape::Laser => "Laser",
    }
}

/// Each tool the toolpaths use, in the order first used, with its number: its slot on a
/// machine with a tool changer (0: changed by hand), else its own number, else its place.
pub fn tools_used<'a>(doc: &'a Document, slots: &[Option<String>]) -> Vec<(u32, &'a Tool)> {
    let mut tools: Vec<&Tool> = vec![];
    for t in doc.toolpaths.values() {
        if !tools.iter().any(|known| known.name == t.tool.name) {
            tools.push(&t.tool);
        }
    }
    tools
        .iter()
        .enumerate()
        .map(|(place, tool)| {
            let number = if slots.is_empty() {
                match tool.number {
                    0 => u32::try_from(place + 1).unwrap_or(1),
                    own => own,
                }
            } else {
                slots
                    .iter()
                    .position(|s| s.as_deref() == Some(tool.name.as_str()))
                    .and_then(|i| u32::try_from(i + 1).ok())
                    .unwrap_or(0)
            };
            (number, *tool)
        })
        .collect()
}

/// Each of `toolpaths`' run time in minutes, cut one after another from the job's home,
/// and the whole job's (the trip back home included), as the Toolpath Summary estimates
/// them.
pub fn run_minutes(doc: &Document, toolpaths: &[&Toolpath], ctx: &Context) -> (Vec<f64>, f64) {
    let home = doc.job.home();
    let job = crate::cut_time::job(
        toolpaths,
        glam::DVec3::new(home.x, home.y, home.z),
        &ctx.motion,
    );
    let minutes = |seconds: f64| seconds / 60.0 * ctx.scale;
    (
        job.toolpaths.iter().map(|t| minutes(t.total())).collect(),
        minutes(job.total().total()),
    )
}

/// Every toolpath's run time in minutes, by id: the shown ones cut one after another from
/// the job's home (getting to each and its bit change included), a hidden one on its own;
/// and the whole job's, every shown toolpath and the trip back home.
pub fn job_minutes(
    doc: &Document,
    ctx: &Context,
) -> (
    std::collections::BTreeMap<crate::document::EntityId, f64>,
    f64,
) {
    let shown: Vec<_> = doc.toolpaths.iter().filter(|(_, t)| t.visible).collect();
    let paths: Vec<&Toolpath> = shown.iter().map(|(_, t)| *t).collect();
    let (minutes, total) = run_minutes(doc, &paths, ctx);
    let mut each: std::collections::BTreeMap<_, _> =
        shown.iter().map(|&(&id, _)| id).zip(minutes).collect();
    for (&id, t) in doc.toolpaths.iter().filter(|(_, t)| !t.visible) {
        each.insert(
            id,
            crate::cut_time::toolpath(t, &ctx.motion).total() / 60.0 * ctx.scale,
        );
    }
    (each, total)
}

/// The file's name without its folder or extension; `Untitled` before it is saved.
fn job_name(file: &str) -> String {
    std::path::Path::new(file)
        .file_stem()
        .map_or_else(|| "Untitled".into(), |s| s.to_string_lossy().into_owned())
}

/// Every variable notes can show, grouped as [`GROUPS`], with its value now.
#[expect(
    clippy::too_many_lines,
    reason = "one list of every variable, in its order"
)]
pub fn variables(doc: &Document, ctx: &Context) -> Vec<Variable> {
    let units = doc.job.units;
    let job = &doc.job;
    let mut out = vec![];
    let mut add =
        |group: &'static str, under: &str, key: String, label: &str, what: &str, value: String| {
            out.push(Variable {
                key,
                group,
                under: under.to_owned(),
                label: label.to_owned(),
                description: what.to_owned(),
                value,
            });
        };
    let none = |s: &str, missing: &str| {
        if s.is_empty() {
            missing.to_owned()
        } else {
            s.to_owned()
        }
    };

    // Job
    add(
        "Job",
        "",
        "job.name".into(),
        "Job name",
        "The job's file name, without its folder",
        job_name(&ctx.job_file),
    );
    add(
        "Job",
        "",
        "job.file".into(),
        "Job file",
        "Where the job is saved on this computer",
        none(&ctx.job_file, "Not saved yet"),
    );
    let folder = std::path::Path::new(&ctx.job_file)
        .parent()
        .map_or_default(|p| p.display().to_string());
    add(
        "Job",
        "",
        "job.folder".into(),
        "Job folder",
        "The folder the job is saved in",
        none(&folder, "Not saved yet"),
    );
    add(
        "Job",
        "",
        "job.units".into(),
        "Units",
        "Whether the job is in millimetres or inches",
        match units {
            Units::Mm => "mm".into(),
            Units::Inch => "inches".into(),
        },
    );

    // Material
    add(
        "Material",
        "",
        "material.size".into(),
        "Material size",
        "Width × height × thickness",
        format!(
            "{} × {} × {}",
            trimmed(job.width / units.per()),
            trimmed(job.height / units.per()),
            length(job.thickness, units)
        ),
    );
    add(
        "Material",
        "",
        "material.width".into(),
        "Width",
        "How wide the material is (X)",
        length(job.width, units),
    );
    add(
        "Material",
        "",
        "material.height".into(),
        "Height",
        "How tall the material is (Y)",
        length(job.height, units),
    );
    add(
        "Material",
        "",
        "material.thickness".into(),
        "Thickness",
        "How thick the material is (Z)",
        length(job.thickness, units),
    );
    add(
        "Material",
        "",
        "material.z_zero".into(),
        "Z zero",
        "Where the bit is zeroed in Z",
        match job.z_zero {
            ZZero::MaterialSurface => "Material surface".into(),
            ZZero::MachineBed => "Machine bed".into(),
        },
    );
    add(
        "Material",
        "",
        "material.xy_origin".into(),
        "XY origin",
        "The corner (or middle) X0 Y0 is at",
        match job.datum {
            Datum::BottomLeft => "Bottom left corner",
            Datum::TopLeft => "Top left corner",
            Datum::TopRight => "Top right corner",
            Datum::BottomRight => "Bottom right corner",
            Datum::Centre => "Middle",
        }
        .into(),
    );
    add(
        "Material",
        "",
        "material.clearance".into(),
        "Safe Z",
        "How high above the material the bit moves between cuts",
        length(job.clearance, units),
    );

    // Model
    let tall = ctx.model_height.unwrap_or(0.0);
    let (above, below) = job.model_position.gaps(job.thickness, tall);
    let model = |mm: f64| {
        if ctx.model_height.is_some() {
            length(mm, units)
        } else {
            "No model".into()
        }
    };
    add(
        "Model",
        "",
        "model.height".into(),
        "Model height",
        "How tall the 3D model is",
        model(tall),
    );
    add(
        "Model",
        "",
        "model.gap_above".into(),
        "Gap above",
        "From the model's top up to the material's top",
        model(above),
    );
    add(
        "Model",
        "",
        "model.gap_below".into(),
        "Gap below",
        "From the model's bottom down to the material's bottom",
        model(below),
    );
    add(
        "Model",
        "",
        "model.measured_from".into(),
        "Gap measured from",
        "Which face of the material the model's gap is set from",
        match job.model_position.from {
            GapFrom::Above => "Top".into(),
            GapFrom::Below => "Bottom".into(),
        },
    );

    // Machine
    add(
        "Machine",
        "",
        "machine.name".into(),
        "Machine",
        "The machine the job is cut on",
        none(&ctx.machine, "No machine set up"),
    );
    add(
        "Machine",
        "",
        "machine.controller".into(),
        "Controller",
        "The machine's controller",
        none(&ctx.controller, "Not given"),
    );
    add(
        "Machine",
        "",
        "machine.post".into(),
        "Post processor",
        "What the toolpaths are saved through",
        none(&ctx.post, "None chosen"),
    );
    let bed = if ctx.bed[0] > 0.0 && ctx.bed[1] > 0.0 {
        format!(
            "{} × {}",
            trimmed(ctx.bed[0] / units.per()),
            length(ctx.bed[1], units)
        )
    } else {
        "Not given".into()
    };
    add(
        "Machine",
        "",
        "machine.bed".into(),
        "Bed size",
        "How much the machine can reach in X and Y",
        bed,
    );
    let changer = if ctx.slots.is_empty() {
        "None: bits are changed by hand".into()
    } else {
        format!("{} slots", ctx.slots.len())
    };
    add(
        "Machine",
        "",
        "machine.tool_changer".into(),
        "Tool changer",
        "Whether the machine changes bits itself",
        changer,
    );
    add(
        "Machine",
        "",
        "machine.collet_nut".into(),
        "Collet nut",
        "The collet nut's size, across and tall (Machine Configuration)",
        holder_size(&ctx.holding.holder, units),
    );
    add(
        "Machine",
        "",
        "machine.spindle_diameter".into(),
        "Spindle diameter",
        "How wide the spindle body above the collet nut is",
        length(ctx.holding.holder.spindle_diameter, units),
    );
    add(
        "Machine",
        "",
        "machine.holder_per_bit".into(),
        "Holder per bit",
        "Whether a bit's own holder in the Bit Library counts instead",
        if ctx.holding.per_bit { "Yes" } else { "No" }.into(),
    );

    // Tools
    let tools = tools_used(doc, &ctx.slots);
    let number = |n: u32| {
        if n == 0 {
            "by hand".to_owned()
        } else {
            format!("T{n}")
        }
    };
    add(
        "Bits",
        "",
        "tools.count".into(),
        "Number of bits",
        "How many bits the toolpaths use",
        tools.len().to_string(),
    );
    let list = tools
        .iter()
        .map(|(n, t)| format!("{}: {}", number(*n), t.name))
        .collect::<Vec<_>>()
        .join("\n");
    add(
        "Bits",
        "",
        "tools.list".into(),
        "Bit list",
        "Each bit, one a line, with its number",
        none(&list, "No bits yet"),
    );
    for (place, (n, tool)) in tools.iter().enumerate() {
        let i = place + 1;
        let under = format!("{i}. {}", tool.name);
        let key = |what: &str| format!("tool.{i}.{what}");
        add(
            "Bits",
            &under,
            key("name"),
            "Name",
            "The bit's name",
            tool.name.clone(),
        );
        add(
            "Bits",
            &under,
            key("number"),
            "Bit number",
            "Its slot or number (T)",
            number(*n),
        );
        add(
            "Bits",
            &under,
            key("type"),
            "Type",
            "What kind of bit it is",
            shape_name(tool.shape).into(),
        );
        add(
            "Bits",
            &under,
            key("diameter"),
            "Diameter",
            "How wide it cuts",
            length(tool.diameter, units),
        );
        add(
            "Bits",
            &under,
            key("pass_depth"),
            "Pass depth",
            "How deep each pass goes",
            length(tool.pass_depth, units),
        );
        add(
            "Bits",
            &under,
            key("stepover"),
            "Stepover",
            "How far over each clearing pass steps",
            format!(
                "{} ({}%)",
                length(tool.stepover * tool.diameter, units),
                trimmed(tool.stepover * 100.0)
            ),
        );
        add(
            "Bits",
            &under,
            key("feed"),
            "Feed rate",
            "How fast it cuts sideways",
            feed(tool.feed_rate, units),
        );
        add(
            "Bits",
            &under,
            key("plunge"),
            "Plunge rate",
            "How fast it goes down into the material",
            feed(tool.plunge_rate, units),
        );
        add(
            "Bits",
            &under,
            key("spindle"),
            "Spindle speed",
            "How fast the spindle turns",
            format!("{} rpm", trimmed(tool.spindle_speed)),
        );
        add(
            "Bits",
            &under,
            key("flutes"),
            "Flutes",
            "Its cutting edges",
            tool.flutes.to_string(),
        );
        let (stickout, guessed) = crate::collision::stickout(tool);
        add(
            "Bits",
            &under,
            key("stick_out"),
            "Stick-out",
            "How far the bit sticks out below the collet nut",
            if guessed {
                format!("{} (not set: guessed)", length(stickout, units))
            } else {
                length(stickout, units)
            },
        );
        let holder = crate::collision::holder_of(tool, &ctx.holding);
        add(
            "Bits",
            &under,
            key("holder"),
            "Holder",
            "The collet nut and spindle it sits in",
            format!(
                "{} nut, {} spindle",
                holder_size(&holder, units),
                length(holder.spindle_diameter, units)
            ),
        );
    }

    // Toolpaths
    let mut lines = vec![];
    let tool_number = |name: &str| {
        tools
            .iter()
            .find(|(_, t)| t.name == name)
            .map_or(0, |(n, _)| *n)
    };
    let paths: Vec<&Toolpath> = doc.toolpaths.values().collect();
    let (times, total) = job_minutes(doc, ctx);
    for (place, (id, t)) in doc.toolpaths.iter().enumerate() {
        let i = place + 1;
        let minutes = times.get(id).copied().unwrap_or_default();
        let n = tool_number(&t.tool.name);
        lines.push(format!(
            "{i}. {} ({}, {}) {}",
            t.name,
            number(n),
            t.tool.name,
            duration(minutes)
        ));
        let under = format!("{i}. {}", t.name);
        let key = |what: &str| format!("toolpath.{i}.{what}");
        add(
            "Toolpaths",
            &under,
            key("name"),
            "Name",
            "The toolpath's name",
            t.name.clone(),
        );
        add(
            "Toolpaths",
            &under,
            key("tool"),
            "Bit",
            "The bit it cuts with",
            t.tool.name.clone(),
        );
        add(
            "Toolpaths",
            &under,
            key("tool_number"),
            "Bit number",
            "The bit's slot or number (T)",
            number(n),
        );
        add(
            "Toolpaths",
            &under,
            key("feed"),
            "Feed rate",
            "How fast it cuts sideways",
            feed(t.tool.feed_rate, units),
        );
        add(
            "Toolpaths",
            &under,
            key("plunge"),
            "Plunge rate",
            "How fast it goes down into the material",
            feed(t.tool.plunge_rate, units),
        );
        add(
            "Toolpaths",
            &under,
            key("spindle"),
            "Spindle speed",
            "How fast the spindle turns",
            format!("{} rpm", trimmed(t.tool.spindle_speed)),
        );
        add(
            "Toolpaths",
            &under,
            key("stepover"),
            "Stepover",
            "How far over each clearing pass steps",
            format!(
                "{} ({}%)",
                length(t.tool.stepover * t.tool.diameter, units),
                trimmed(t.tool.stepover * 100.0)
            ),
        );
        add(
            "Toolpaths",
            &under,
            key("pass_depth"),
            "Pass depth",
            "How deep each pass goes",
            length(t.tool.pass_depth, units),
        );
        add(
            "Toolpaths",
            &under,
            key("run_time"),
            "Run time",
            "About how long it takes to cut, getting to it from the last one included",
            duration(minutes),
        );
        add(
            "Toolpaths",
            &under,
            key("notes"),
            "Its notes",
            "The notes written for it in the Toolpath Summary",
            none(t.summary_notes(), "None"),
        );
    }
    add(
        "Toolpaths",
        "",
        "toolpaths.count".into(),
        "Number of toolpaths",
        "How many toolpaths the job has",
        paths.len().to_string(),
    );
    add(
        "Toolpaths",
        "",
        "toolpaths.list".into(),
        "Toolpath list",
        "Each toolpath, one a line, with its bit and run time",
        none(&lines.join("\n"), "No toolpaths yet"),
    );
    add(
        "Toolpaths",
        "",
        "toolpaths.run_time".into(),
        "Total run time",
        "About how long the whole job takes: every move, bit changes and the trip home",
        duration(total),
    );

    // Document Variables
    for (name, value) in &doc.variables {
        add(
            "Document Variables",
            "",
            format!("var.{name}"),
            name,
            "Set in Edit ▸ Document Variables",
            trimmed(*value),
        );
    }

    // System
    add(
        "System",
        "",
        "date".into(),
        "Date",
        "Today's date",
        ctx.date.clone(),
    );
    add(
        "System",
        "",
        "time".into(),
        "Time",
        "The time now",
        ctx.time.clone(),
    );
    add(
        "System",
        "",
        "app.version".into(),
        "CutCraft version",
        "The version of CutCraft in use",
        env!("CARGO_PKG_VERSION").into(),
    );
    out
}

/// A variable's value by key; `None` when there is no such variable (now).
pub fn value<'a>(vars: &'a [Variable], key: &str) -> Option<&'a str> {
    vars.iter().find(|v| v.key == key).map(|v| v.value.as_str())
}

/// Where each `{{key}}` is in `text`: its byte range (braces included) and its key.
pub fn variable_spans(text: &str) -> Vec<(std::ops::Range<usize>, &str)> {
    let mut found = vec![];
    let mut from = 0;
    while let Some(open) = text[from..].find("{{") {
        let open = from + open;
        let Some(close) = text[open + 2..].find("}}") else {
            break;
        };
        let close = open + 2 + close;
        let key = &text[open + 2..close];
        if !key.is_empty()
            && key
                .chars()
                .all(|c| c.is_alphanumeric() || "._-".contains(c))
        {
            found.push((open..close + 2, key));
            from = close + 2;
        } else {
            from = open + 2;
        }
    }
    found
}

/// The `{{key}}` `text` starts with: its byte range and key; `None` when it starts with
/// none. Looks no further than the first `}}`.
fn variable_here(text: &str) -> Option<(std::ops::Range<usize>, &str)> {
    let close = text.strip_prefix("{{")?.find("}}")? + 2;
    let key = &text[2..close];
    (!key.is_empty()
        && key
            .chars()
            .all(|c| c.is_alphanumeric() || "._-".contains(c)))
    .then_some((0..close + 2, key))
}

/// `text` with each `{{key}}` its value; a key with no variable stays as typed, so a typo
/// shows where it is.
pub fn resolve(text: &str, vars: &[Variable]) -> String {
    let mut out = String::new();
    let mut at = 0;
    for (range, key) in variable_spans(text) {
        out.push_str(&text[at..range.start]);
        match value(vars, key) {
            Some(v) => out.push_str(v),
            None => out.push_str(&text[range.clone()]),
        }
        at = range.end;
    }
    out.push_str(&text[at..]);
    out
}

/// A run of text and how it is shown.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Span {
    pub text: String,
    pub bold: bool,
    pub italic: bool,
    /// Where it links to (`http`, `https` or `mailto` only).
    pub link: Option<String>,
    /// The variable it stands for: `text` is its value once resolved, else `{{key}}`.
    pub variable: Option<String>,
}

/// What a line is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Kind {
    Text,
    /// `#` 1, `##` 2, `###` 3.
    Heading(u8),
    Bullet,
    /// Its number as typed.
    Numbered(String),
    /// Ticked or not.
    Check(bool),
}

/// One line of notes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Line {
    pub kind: Kind,
    pub spans: Vec<Span>,
}

impl Line {
    /// Its words, as shown, without their looks.
    pub fn text(&self) -> String {
        self.spans.iter().map(|s| s.text.as_str()).collect()
    }
}

/// Whether a link may be opened: web pages and e-mail addresses only, never files or scripts.
pub fn safe_link(url: &str) -> bool {
    let lower = url.trim().to_ascii_lowercase();
    ["http://", "https://", "mailto:"]
        .iter()
        .any(|scheme| lower.starts_with(scheme) && lower.len() > scheme.len())
}

/// What a line's start makes it, and the rest of the line.
fn kind_of(line: &str) -> (Kind, &str) {
    let trimmed = line.trim_start();
    for (mark, level) in [("### ", 3), ("## ", 2), ("# ", 1)] {
        if let Some(rest) = trimmed.strip_prefix(mark) {
            return (Kind::Heading(level), rest);
        }
    }
    for bullet in ["- ", "* "] {
        if let Some(rest) = trimmed.strip_prefix(bullet) {
            for (mark, ticked) in [("[ ] ", false), ("[x] ", true), ("[X] ", true)] {
                if let Some(rest) = rest.strip_prefix(mark) {
                    return (Kind::Check(ticked), rest);
                }
            }
            return (Kind::Bullet, rest);
        }
    }
    for (mark, ticked) in [("[ ] ", false), ("[x] ", true), ("[X] ", true)] {
        if let Some(rest) = trimmed.strip_prefix(mark) {
            return (Kind::Check(ticked), rest);
        }
    }
    let digits = trimmed.chars().take_while(char::is_ascii_digit).count();
    if digits > 0 && trimmed[digits..].starts_with(". ") {
        return (
            Kind::Numbered(trimmed[..digits].into()),
            &trimmed[digits + 2..],
        );
    }
    (Kind::Text, line)
}

/// A line's words split into runs by `**`, `*`, links and variables. A mark with no partner
/// later in the line is shown as typed.
fn spans(text: &str) -> Vec<Span> {
    let mut out: Vec<Span> = vec![];
    let mut current = String::new();
    let (mut bold, mut italic) = (false, false);
    let flush = |out: &mut Vec<Span>, current: &mut String, bold: bool, italic: bool| {
        if !current.is_empty() {
            out.push(Span {
                text: std::mem::take(current),
                bold,
                italic,
                ..Span::default()
            });
        }
    };
    let mut i = 0;
    while i < text.len() {
        let rest = &text[i..];
        if rest.starts_with("{{")
            && let Some((range, key)) = variable_here(rest)
        {
            {
                flush(&mut out, &mut current, bold, italic);
                out.push(Span {
                    text: rest[range.clone()].into(),
                    bold,
                    italic,
                    variable: Some(key.into()),
                    ..Span::default()
                });
                i += range.end;
                continue;
            }
        }
        if rest.starts_with("**") && (bold || rest[2..].contains("**")) {
            flush(&mut out, &mut current, bold, italic);
            bold = !bold;
            i += 2;
            continue;
        }
        if rest.starts_with('*')
            && !rest.starts_with("**")
            && (italic || rest[1..].replace("**", "").contains('*'))
        {
            flush(&mut out, &mut current, bold, italic);
            italic = !italic;
            i += 1;
            continue;
        }
        if rest.starts_with('[')
            && let Some(close) = rest.find("](")
            && let Some(end) = rest[close + 2..].find(')')
        {
            {
                {
                    let label = &rest[1..close];
                    let url = &rest[close + 2..close + 2 + end];
                    if !label.contains('[') && safe_link(url) {
                        flush(&mut out, &mut current, bold, italic);
                        // the words may hold variables and looks of their own
                        for mut span in spans(label) {
                            span.bold |= bold;
                            span.italic |= italic;
                            span.link = Some(url.trim().into());
                            out.push(span);
                        }
                        i += close + 2 + end + 1;
                        continue;
                    }
                }
            }
        }
        let c = rest.chars().next().unwrap_or_default();
        current.push(c);
        i += c.len_utf8();
    }
    flush(&mut out, &mut current, bold, italic);
    out
}

/// `text` as lines with their looks, the open mark ([`OPEN_MARK`]) left off.
pub fn parse(text: &str) -> Vec<Line> {
    strip_open_mark(text)
        .lines()
        .map(|line| {
            let (kind, rest) = kind_of(line);
            Line {
                kind,
                spans: spans(rest),
            }
        })
        .collect()
}

/// [`parse`] with each variable's value filled in.
pub fn parse_resolved(text: &str, vars: &[Variable]) -> Vec<Line> {
    let mut lines = parse(text);
    for line in &mut lines {
        for span in &mut line.spans {
            if let Some(v) = span.variable.as_deref().and_then(|key| value(vars, key)) {
                span.text = v.to_owned();
            }
        }
    }
    lines
}

/// Notes without the mark that opens them with the job.
pub fn strip_open_mark(text: &str) -> &str {
    text.strip_prefix(OPEN_MARK).unwrap_or(text)
}

/// Whether the notes open by themselves when the job is opened.
pub fn opens_with_job(text: &str) -> bool {
    text.starts_with(OPEN_MARK)
}

/// The notes as plain text, variables filled in and the marks gone: lists keep a `- `,
/// `1. ` or `[ ]` / `[x]` so they still read as lists, a link keeps its address.
pub fn plain(text: &str, vars: &[Variable]) -> String {
    parse_resolved(text, vars)
        .iter()
        .map(|line| {
            // a link's address once, after all its words
            let mut words = String::new();
            let mut label = String::new();
            for (n, s) in line.spans.iter().enumerate() {
                words.push_str(&s.text);
                let Some(url) = &s.link else { continue };
                label.push_str(&s.text);
                if line.spans.get(n + 1).and_then(|next| next.link.as_ref()) != Some(url) {
                    if *url != label {
                        let _ = write!(words, " ({url})");
                    }
                    label.clear();
                }
            }
            match &line.kind {
                Kind::Text | Kind::Heading(_) => words,
                Kind::Bullet => format!("- {words}"),
                Kind::Numbered(n) => format!("{n}. {words}"),
                Kind::Check(ticked) => format!("[{}] {words}", if *ticked { "x" } else { " " }),
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// `text` safe inside HTML: every `<`, `>`, `&` and quote written as an entity.
pub fn escape_html(text: &str) -> String {
    text.chars()
        .map(|c| match c {
            '<' => "&lt;".into(),
            '>' => "&gt;".into(),
            '&' => "&amp;".into(),
            '"' => "&quot;".into(),
            '\'' => "&#39;".into(),
            c => c.to_string(),
        })
        .collect()
}

/// The notes as HTML for the job sheet, variables filled in. Everything typed is escaped:
/// the only tags are the ones made here for the notes' own looks.
pub fn html(text: &str, vars: &[Variable]) -> String {
    let mut out = String::new();
    let mut list: Option<&str> = None;
    for line in parse_resolved(text, vars) {
        let wanted = match line.kind {
            Kind::Bullet | Kind::Check(_) => Some("ul"),
            Kind::Numbered(_) => Some("ol"),
            _ => None,
        };
        if list != wanted {
            if let Some(tag) = list {
                let _ = write!(out, "</{tag}>");
            }
            if let Some(tag) = wanted {
                let _ = write!(out, "<{tag}>");
            }
            list = wanted;
        }
        let mut words = String::new();
        for s in &line.spans {
            let mut piece = escape_html(&s.text).replace('\n', "<br>");
            if s.italic {
                piece = format!("<em>{piece}</em>");
            }
            if s.bold {
                piece = format!("<strong>{piece}</strong>");
            }
            if let Some(url) = &s.link {
                piece = format!("<a href=\"{}\">{piece}</a>", escape_html(url));
            }
            words.push_str(&piece);
        }
        match line.kind {
            Kind::Heading(level) => {
                let _ = write!(out, "<h{0}>{words}</h{0}>", level + 2);
            }
            // each number as typed, so 3 then 7 stay 3 and 7, and 3 then 1 stay 3 and 1
            Kind::Numbered(n) if let Ok(n) = n.parse::<u64>() => {
                let _ = write!(out, "<li value=\"{n}\">{words}</li>");
            }
            Kind::Bullet | Kind::Numbered(_) => {
                let _ = write!(out, "<li>{words}</li>");
            }
            Kind::Check(ticked) => {
                let tick = if ticked { "&#9745;" } else { "&#9744;" };
                let _ = write!(out, "<li class=\"check\">{tick} {words}</li>");
            }
            Kind::Text if words.is_empty() => out.push_str("<br>"),
            Kind::Text => {
                let _ = write!(out, "<p>{words}</p>");
            }
        }
    }
    if let Some(tag) = list {
        let _ = write!(out, "</{tag}>");
    }
    out
}

/// The most characters a G-code comment line of notes holds, so small controllers' line
/// buffers (GRBL's is 80) take it whole.
pub const GCODE_COMMENT_WIDTH: usize = 70;

/// The notes as lines a G-code comment can hold: plain text ([`plain`]), only printable
/// ASCII, no brackets or semicolons (which would end a comment early), no `!`, `~` or `?`
/// (GRBL acts on those even inside a comment: feed hold, resume, status), each line at most
/// [`GCODE_COMMENT_WIDTH`] long, broken between words.
pub fn gcode_lines(text: &str, vars: &[Variable]) -> Vec<String> {
    let mut out = vec![];
    for line in plain(text, vars).lines() {
        let clean: String = line
            .chars()
            .map(|c| match c {
                '(' => '[',
                ')' => ']',
                ';' => ',',
                '×' => 'x',
                '!' | '?' => '.',
                '~' => '-',
                c if c.is_ascii_graphic() || c == ' ' => c,
                _ => '-',
            })
            .collect();
        let clean = clean.trim_end();
        if clean.is_empty() {
            continue;
        }
        let mut current = String::new();
        for word in clean.split(' ') {
            let mut word = word.to_owned();
            while word.len() > GCODE_COMMENT_WIDTH {
                if !current.is_empty() {
                    out.push(std::mem::take(&mut current));
                }
                let rest = word.split_off(GCODE_COMMENT_WIDTH);
                out.push(word);
                word = rest;
            }
            if !current.is_empty() && current.len() + 1 + word.len() > GCODE_COMMENT_WIDTH {
                out.push(std::mem::take(&mut current));
            }
            if !current.is_empty() {
                current.push(' ');
            }
            current.push_str(&word);
        }
        if !current.trim().is_empty() {
            out.push(current);
        }
    }
    out
}

/// Byte offset of char `index` in `text` (its end when past it).
fn byte_at(text: &str, index: usize) -> usize {
    text.char_indices()
        .nth(index)
        .map_or(text.len(), |(b, _)| b)
}

/// The Notes toolbar's buttons: what each does to the text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    Bold,
    Italic,
    Heading,
    Bullet,
    Numbered,
    Check,
    Link,
}

impl Format {
    /// `text` with this applied to the chars `selected` (a cursor when empty), and the chars
    /// to select after: marks round the selection (or an empty pair with the cursor between),
    /// or a line start added to (taken off) each line the selection touches.
    pub fn apply(
        self,
        text: &str,
        selected: std::ops::Range<usize>,
    ) -> (String, std::ops::Range<usize>) {
        match self {
            Self::Bold => wrap(text, selected, "**", "**"),
            Self::Italic => wrap(text, selected, "*", "*"),
            Self::Link => {
                let (start, end) = (byte_at(text, selected.start), byte_at(text, selected.end));
                let words = if start == end {
                    "words"
                } else {
                    &text[start..end]
                };
                let made = format!("[{words}](https://)");
                let out = format!("{}{made}{}", &text[..start], &text[end..]);
                // the address is left selected, to type over
                let address = selected.start + words.chars().count() + 3;
                (out, address..address + "https://".len())
            }
            Self::Heading => line_starts(text, selected, "## "),
            Self::Bullet => line_starts(text, selected, "- "),
            Self::Numbered => numbered_lines(text, selected),
            Self::Check => line_starts(text, selected, "- [ ] "),
        }
    }
}

fn wrap(
    text: &str,
    selected: std::ops::Range<usize>,
    open: &str,
    close: &str,
) -> (String, std::ops::Range<usize>) {
    let (start, end) = (byte_at(text, selected.start), byte_at(text, selected.end));
    let out = format!(
        "{}{open}{}{close}{}",
        &text[..start],
        &text[start..end],
        &text[end..]
    );
    let shift = open.chars().count();
    (out, selected.start + shift..selected.end + shift)
}

/// Each line the chars `selected` touch starting with `mark`, or, when they all do, none.
fn line_starts(
    text: &str,
    selected: std::ops::Range<usize>,
    mark: &str,
) -> (String, std::ops::Range<usize>) {
    marked_lines(text, selected, &|_| mark.to_owned(), &|line| {
        line.starts_with(mark).then_some(mark.len())
    })
}

/// Each line the chars `selected` touch numbered on from the line above (1, 2, 3… when
/// that is no numbered line), or, when they all are numbered, none.
fn numbered_lines(
    text: &str,
    selected: std::ops::Range<usize>,
) -> (String, std::ops::Range<usize>) {
    let numbered = |line: &str| match kind_of(line) {
        (Kind::Numbered(n), rest) if !line.starts_with(char::is_whitespace) => {
            Some((n.parse::<u64>().unwrap_or(0), line.len() - rest.len()))
        }
        _ => None,
    };
    let first = text
        .chars()
        .take(selected.start)
        .filter(|c| *c == '\n')
        .count();
    let from = first
        .checked_sub(1)
        .and_then(|above| text.split('\n').nth(above))
        .and_then(&numbered)
        .map_or(1, |(n, _)| n + 1);
    marked_lines(
        text,
        selected,
        &|i| format!("{}. ", from + i as u64),
        &|line| numbered(line).map(|(_, len)| len),
    )
}

/// Each line the chars `selected` touch starting with `mark(its place in the selection)`,
/// or, when `has` finds a mark (its length in bytes) on them all, none.
fn marked_lines(
    text: &str,
    selected: std::ops::Range<usize>,
    mark: &dyn Fn(usize) -> String,
    has: &dyn Fn(&str) -> Option<usize>,
) -> (String, std::ops::Range<usize>) {
    let mut lines: Vec<String> = text.split('\n').map(str::to_owned).collect();
    let line_of = |char_index: usize| text.chars().take(char_index).filter(|c| *c == '\n').count();
    let (first, last) = (line_of(selected.start), line_of(selected.end));
    let all = lines[first..=last].iter().all(|l| has(l).is_some());
    let mut moved = 0_isize;
    let count = |s: &str| isize::try_from(s.chars().count()).unwrap_or(0);
    for (i, line) in lines[first..=last].iter_mut().enumerate() {
        if all {
            let len = has(line).unwrap_or(0);
            moved -= count(&line[..len]);
            line.drain(..len);
        } else {
            // another kind of line start is swapped for this one
            let (kind, rest) = kind_of(line);
            let had = if kind == Kind::Text {
                0
            } else {
                line.chars().count() - rest.chars().count()
            };
            let rest = rest.to_owned();
            let mark = mark(i);
            *line = format!("{mark}{rest}");
            moved += count(&mark) - isize::try_from(had).unwrap_or(0);
        }
    }
    let end = selected.end.saturating_add_signed(moved);
    (lines.join("\n"), end..end)
}

/// `text` with `insert` put in at char `at`, and the char just after it.
pub fn insert_at(text: &str, at: usize, insert: &str) -> (String, usize) {
    let at = at.min(text.chars().count());
    let b = byte_at(text, at);
    (
        format!("{}{insert}{}", &text[..b], &text[b..]),
        at + insert.chars().count(),
    )
}

/// `text` with the check box on line `line` (from 0, the open mark left off) ticked or
/// unticked; unchanged when that line is not a check box.
pub fn toggle_check(text: &str, line: usize) -> String {
    let (mark, body) = match text.strip_prefix(OPEN_MARK) {
        Some(body) => (".", body),
        None => ("", text),
    };
    let lines: Vec<String> = body
        .split('\n')
        .enumerate()
        .map(|(i, l)| {
            if i != line {
                return l.to_owned();
            }
            for (from, to) in [("[ ] ", "[x] "), ("[x] ", "[ ] "), ("[X] ", "[ ] ")] {
                let trimmed = l.trim_start();
                let indent = &l[..l.len() - trimmed.len()];
                for bullet in ["- ", "* ", ""] {
                    if let Some(rest) = trimmed
                        .strip_prefix(bullet)
                        .and_then(|r| r.strip_prefix(from))
                    {
                        return format!("{indent}{bullet}{to}{rest}");
                    }
                }
            }
            l.to_owned()
        })
        .collect();
    format!("{mark}{}", lines.join("\n"))
}

#[cfg(test)]
mod tests;
