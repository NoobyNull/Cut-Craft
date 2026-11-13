//! The checks Save Toolpaths makes before writing a file: the job is simulated in cutting
//! order to find rapid moves that would crash into the material and cutting that only cuts
//! air, and each toolpath is checked against the machine (bed, Z travel, speeds, tool
//! changer) and for settings that cannot cut (no moves, no feed rate). These
//! are Cut Craft's own. Every message says how to put the problem right.

use crate::checks::{LEFT, problems_over};
use crate::document::Document;
use crate::simulation::Material;
use crate::toolpath::{MoveKind, Toolpath};
use glam::DVec3;

/// How deep a rapid move may seem to go into the material before it is a crash, mm: the
/// simulation samples cell centres, so a rapid skimming a wall it has just cut can seem to
/// touch it.
const CRASH: f64 = 0.5;

/// Material a cutting move must take off to count as cutting, mm.
const CUTS: f64 = 0.02;

/// How far clear of all the material a cutting move must stay to be cutting air whatever
/// cut there before, mm. Closer than that, it is air only where an earlier toolpath cut:
/// the simulation's cells are coarser than a finishing stepover, so a toolpath's own rows
/// can seem to take nothing off.
const CLEAR: f64 = 0.5;

/// Air cutting worth a mention: this share of a toolpath's cutting moves, and this length.
const AIR_SHARE: f64 = 0.3;
const AIR_LENGTH: f64 = 100.0;

/// How far below the material a cut may go before it is called out, mm: profiles cut a
/// little into the spoilboard on purpose.
const SPOILBOARD: f64 = 1.0;

/// The simulation's cells across the job's longer side: about the smallest tool's radius,
/// within these bounds.
const FEWEST_CELLS: f64 = 100.0;
const MOST_CELLS: f64 = 800.0;

/// Whether a finding stops the save.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    /// Saved only once the user agrees.
    Warning,
    /// Not saved until it is put right.
    Error,
}

/// One thing found, as a sentence saying what is wrong and how to fix it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Finding {
    pub severity: Severity,
    pub message: String,
}

impl Finding {
    fn error(message: String) -> Self {
        Self {
            severity: Severity::Error,
            message,
        }
    }

    fn warning(message: String) -> Self {
        Self {
            severity: Severity::Warning,
            message,
        }
    }
}

/// What the machine can do, as Machine Configuration has it; 0 (or empty) is not given and
/// is not checked.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Limits {
    /// The machine's name, for the messages.
    pub name: String,
    /// Bed width and height, mm.
    pub bed: [f64; 2],
    /// Z travel, mm.
    pub z_travel: f64,
    /// The fastest each axis moves, X, Y, Z, mm/min.
    pub max_rate: [f64; 3],
    /// The spindle's slowest and fastest speed, rpm.
    pub spindle: [f64; 2],
    /// The tool loaded in each tool changer slot; empty without a tool changer.
    pub slots: Vec<Option<String>>,
    /// Material left behind is mentioned only where it is more than this, mm (Options ▸
    /// Confirmation Choices); `None` for the usual [`crate::checks::LEFT`].
    pub left: Option<f64>,
}

/// Everything found checking `toolpaths`, cut in the order given, on `limits`' machine:
/// errors first, then warnings, each in toolpath order.
pub fn check(doc: &Document, toolpaths: &[&Toolpath], limits: &Limits) -> Vec<Finding> {
    let mut found = vec![];
    for toolpath in toolpaths {
        settings(doc, toolpath, limits, &mut found);
    }
    machine(doc, toolpaths, limits, &mut found);
    if doc.job.rotary.is_none() {
        simulate(doc, toolpaths, &mut found);
    }
    // stable: each severity keeps toolpath order
    found.sort_by_key(|f| std::cmp::Reverse(f.severity));
    found
}

/// What one toolpath's own settings will not cut with.
fn settings(doc: &Document, toolpath: &Toolpath, limits: &Limits, found: &mut Vec<Finding>) {
    let (name, tool) = (&toolpath.name, &toolpath.tool);
    if toolpath.moves.is_empty() {
        found.push(Finding::error(format!(
            "{name} has no moves: recalculate it, or untick it in the toolpath list so it \
             is not saved"
        )));
        return;
    }
    if toolpath.stale {
        found.push(Finding::warning(format!(
            "{name} is out of date: what it was made from has changed since. Recalculate it \
             so it cuts what the design shows now"
        )));
    }
    let has = |kind| toolpath.moves.iter().any(|m| m.kind == kind);
    if has(MoveKind::Cut) && tool.feed_rate <= 0.0 {
        found.push(Finding::error(format!(
            "{name}: {} has no feed rate, so the machine would not move while cutting. Set \
             the feed rate in the toolpath's tool settings",
            tool.name
        )));
    }
    if has(MoveKind::Plunge) && tool.plunge_rate <= 0.0 {
        found.push(Finding::error(format!(
            "{name}: {} has no plunge rate, so the machine would not move down into the \
             material. Set the plunge rate in the toolpath's tool settings",
            tool.name
        )));
    }
    // a laser has no spindle
    let laser = !toolpath.power.is_empty();
    if !laser && tool.spindle_speed <= 0.0 {
        found.push(Finding::warning(format!(
            "{name}: {} has no spindle speed, so the spindle may not turn on. Set the \
             spindle speed in the toolpath's tool settings",
            tool.name
        )));
    }
    on_machine(toolpath, laser, limits, doc.job.units, found);
    let deepest = toolpath
        .moves
        .iter()
        .filter(|m| m.kind != MoveKind::Rapid)
        .map(|m| -m.to.z)
        .fold(0.0, f64::max);
    let below = deepest - doc.job.thickness;
    if doc.job.thickness > 0.0 && below > SPOILBOARD {
        found.push(Finding::warning(format!(
            "{name} cuts {} below the material, into the spoilboard. Check the material \
             thickness in Job Setup and the toolpath's cut depth",
            doc.job.units.length(below, 1)
        )));
    }
    found.extend(
        problems_over(doc, toolpath, limits.left.unwrap_or(LEFT))
            .into_iter()
            .map(Finding::warning),
    );
}

/// Whether one toolpath's speeds suit the machine and its tool is in the tool changer.
fn on_machine(
    toolpath: &Toolpath,
    laser: bool,
    limits: &Limits,
    units: crate::document::Units,
    found: &mut Vec<Finding>,
) {
    let (name, tool) = (&toolpath.name, &toolpath.tool);
    let [slowest, fastest] = limits.spindle;
    if !laser && tool.spindle_speed > 0.0 {
        if slowest > 0.0 && tool.spindle_speed < slowest {
            found.push(Finding::warning(format!(
                "{name}: {} turns at {:.0} rpm, but the spindle of {} turns no slower than \
                 {slowest:.0} rpm. Raise the spindle speed (and the feed rate with it, to \
                 keep the chip load)",
                tool.name, tool.spindle_speed, limits.name
            )));
        }
        if fastest > 0.0 && tool.spindle_speed > fastest {
            found.push(Finding::warning(format!(
                "{name}: {} turns at {:.0} rpm, but the spindle of {} turns no faster than \
                 {fastest:.0} rpm. Lower the spindle speed (and the feed rate with it, to \
                 keep the chip load)",
                tool.name, tool.spindle_speed, limits.name
            )));
        }
    }
    let [x, y, z] = limits.max_rate;
    let sideways = match (x > 0.0, y > 0.0) {
        (true, true) => x.min(y),
        (true, false) => x,
        (false, true) => y,
        (false, false) => 0.0,
    };
    if sideways > 0.0 && tool.feed_rate > sideways {
        found.push(Finding::warning(format!(
            "{name}: the feed rate of {} is {}, faster than {} moves ({}). It will cut slower \
             than planned; lower the feed rate to match",
            tool.name,
            units.feed(tool.feed_rate, 0),
            limits.name,
            units.feed(sideways, 0)
        )));
    }
    let plunges = toolpath.moves.iter().any(|m| m.kind == MoveKind::Plunge);
    if z > 0.0 && plunges && tool.plunge_rate > z {
        found.push(Finding::warning(format!(
            "{name}: the plunge rate of {} is {}, faster than {} moves in Z ({}). Lower the \
             plunge rate to match",
            tool.name,
            units.feed(tool.plunge_rate, 0),
            limits.name,
            units.feed(z, 0)
        )));
    }
    if !limits.slots.is_empty()
        && !limits
            .slots
            .iter()
            .any(|s| s.as_deref() == Some(tool.name.as_str()))
    {
        found.push(Finding::warning(format!(
            "{name}: {} is not loaded in the tool changer of {}, so the machine will stop \
             and ask for it. Load it in a slot in Machine Configuration",
            tool.name, limits.name
        )));
    }
}

/// The lowest and highest corners of what `moves` reach; `None` without any.
fn bounds<'a>(moves: impl Iterator<Item = &'a crate::toolpath::Move>) -> Option<(DVec3, DVec3)> {
    moves.fold(None, |reached, m| {
        Some(reached.map_or((m.to, m.to), |(lo, hi): (DVec3, DVec3)| {
            (lo.min(m.to), hi.max(m.to))
        }))
    })
}

/// How far across and along the bed the toolpaths go in one file: in a tiled job, each tile
/// is saved on its own, so the largest tile's.
fn bed_span(doc: &Document, toolpaths: &[&Toolpath]) -> Option<DVec3> {
    let Some(tiling) = doc.tiling else {
        let (lo, hi) = bounds(toolpaths.iter().flat_map(|t| &t.moves))?;
        return Some(hi - lo);
    };
    let saved = Document {
        job: doc.job,
        toolpaths: (1..)
            .map(crate::document::EntityId)
            .zip(toolpaths.iter().map(|&t| t.clone()))
            .collect(),
        ..Document::default()
    };
    tiling
        .tiles(doc.job.area())
        .into_iter()
        .filter_map(|tile| {
            let part = tiling.tiled(&saved, tile);
            let (lo, hi) = bounds(part.toolpaths.values().flat_map(|t| &t.moves))?;
            Some(hi - lo)
        })
        .reduce(DVec3::max)
}

/// Whether the toolpaths fit the machine's bed and Z travel.
fn machine(doc: &Document, toolpaths: &[&Toolpath], limits: &Limits, found: &mut Vec<Finding>) {
    let Some((lo, hi)) = bounds(toolpaths.iter().flat_map(|t| &t.moves)) else {
        return;
    };
    let span = bed_span(doc, toolpaths).unwrap_or(hi - lo);
    let [width, height] = limits.bed;
    // a rotary job's moves go round the material, not along the bed
    let wraps = doc.job.rotary.is_some();
    let units = doc.job.units;
    if !wraps && width > 0.0 && height > 0.0 && (span.x > width || span.y > height) {
        found.push(Finding::error(format!(
            "The toolpaths cover {} × {}, more than the bed of {} ({} × {}). Make the job \
             smaller, split it with Toolpath Tiling (or make its tiles smaller), or pick the \
             right machine",
            units.length(span.x, 0),
            units.length(span.y, 0),
            limits.name,
            units.length(width, 0),
            units.length(height, 0)
        )));
    }
    if limits.z_travel > 0.0 && span.z > limits.z_travel {
        found.push(Finding::error(format!(
            "The toolpaths go from {} above the material to {} below it, more than the {} \
             the Z axis of {} travels. Lower the clearance (Z1) in Job Setup or cut less deep",
            units.length(hi.z.max(0.0), 1),
            units.length((-lo.z).max(0.0), 1),
            units.length(limits.z_travel, 0),
            limits.name
        )));
    }
}

/// Cuts the toolpaths in order from uncut material, finding rapid moves that hit it and
/// cutting moves that take nothing off.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "a cell count within bounds"
)]
fn simulate(doc: &Document, toolpaths: &[&Toolpath], found: &mut Vec<Finding>) {
    let job = &doc.job;
    let units = job.units;
    // a point's X or Y as the status bar shows it
    let position = |mm: f64| match units {
        crate::document::Units::Mm => format!("{mm:.1}"),
        crate::document::Units::Inch => format!("{:.3}", mm / 25.4),
    };
    let smallest = toolpaths
        .iter()
        .map(|t| t.tool.diameter)
        .filter(|&d| d > 0.0)
        .fold(f64::INFINITY, f64::min);
    if !smallest.is_finite() || job.width <= 0.0 || job.height <= 0.0 {
        return;
    }
    let cells = (job.width.max(job.height) / (smallest / 2.0)).clamp(FEWEST_CELLS, MOST_CELLS);
    let mut material = Material::new(job, cells as usize);
    for (n, toolpath) in (1..).zip(toolpaths) {
        // what this toolpath cuts is marked as its own
        material.cutting = n;
        // a laser burns rather than cuts away
        if !toolpath.power.is_empty() {
            continue;
        }
        let tool = &toolpath.tool;
        // the peaks only come down as the material is cut, so they stay safe to search by
        let peaks = material.peaks();
        let (mut crashes, mut worst): (usize, Option<(f64, DVec3)>) = (0, None);
        let (mut cutting, mut air) = (0.0, 0.0);
        for pair in toolpath.moves.windows(2) {
            let (a, b) = (pair[0].to, pair[1].to);
            match pair[1].kind {
                MoveKind::Rapid => {
                    let into = material.overcut_on(tool, a, b, &peaks);
                    if into > CRASH {
                        crashes += 1;
                        if worst.is_none_or(|(deepest, _)| into > deepest) {
                            worst = Some((into, b));
                        }
                    }
                }
                MoveKind::Cut => {
                    let length = a.distance(b);
                    cutting += length;
                    let takes = material.overcut_on(tool, a, b, &peaks);
                    if takes < -CLEAR || (takes <= CUTS && cut_before(&material, b, n)) {
                        air += length;
                    }
                }
                MoveKind::Plunge => {}
            }
            material.sweep(a, b, tool);
        }
        let name = &toolpath.name;
        if let Some((into, at)) = worst {
            let more = match crashes {
                1 => String::new(),
                n => format!(" ({n} rapid moves in all)"),
            };
            found.push(Finding::error(format!(
                "{name} would crash: a rapid move to X{} Y{} goes {} into material that is \
                 still there{more}. Rapid moves run at full speed and are meant to stay clear \
                 of the material. Raise the clearance (Z1) in Job Setup, or recalculate the \
                 toolpath",
                position(at.x),
                position(at.y),
                units.length(into, 1)
            )));
        }
        if cutting > 0.0 && air > AIR_LENGTH && air / cutting > AIR_SHARE {
            let minutes = if tool.feed_rate > 0.0 {
                format!(", about {:.0} min", (air / tool.feed_rate).ceil())
            } else {
                String::new()
            };
            found.push(Finding::warning(format!(
                "{name} cuts air for {:.0}% of its cutting ({:.1} m{minutes}), where the \
                 material is already gone. If an earlier toolpath cleared this area, cut \
                 less with this one: a smaller area, fewer passes or a shallower start depth",
                100.0 * air / cutting,
                air / 1000.0
            )));
        }
    }
}

/// Whether the cell at `at` was last cut by a toolpath before the `n`th.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "checked to be on the grid first"
)]
fn cut_before(material: &Material, at: DVec3, n: u32) -> bool {
    let (column, row) = (
        (at.x - material.origin.x) / material.cell,
        (at.y - material.origin.y) / material.cell,
    );
    if column < 0.0 || row < 0.0 {
        return false;
    }
    let (column, row) = (column as usize, row as usize);
    column < material.columns
        && row < material.rows
        && (1..n).contains(&material.cut_by[row * material.columns + column])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::Job;
    use crate::toolpath::{Move, Tool};

    fn job() -> Document {
        Document {
            job: Job {
                width: 200.0,
                height: 200.0,
                thickness: 18.0,
                ..Job::default()
            },
            ..Document::default()
        }
    }

    fn tool() -> Tool {
        Tool {
            name: "6 mm End Mill".into(),
            diameter: 6.0,
            feed_rate: 1500.0,
            plunge_rate: 500.0,
            spindle_speed: 18000.0,
            flute_length: 25.0,
            ..Tool::default()
        }
    }

    fn mv(kind: MoveKind, x: f64, y: f64, z: f64) -> Move {
        Move {
            kind,
            to: DVec3::new(x, y, z),
        }
    }

    /// Down at (x0, y), along to x1 at `depth`, and back up.
    fn slot(name: &str, y: f64, x0: f64, x1: f64, depth: f64) -> Toolpath {
        Toolpath {
            name: name.into(),
            strategy: "profile_toolpath".into(),
            params: serde_json::Value::Null,
            tool: tool(),
            vectors: vec![],
            moves: vec![
                mv(MoveKind::Rapid, x0, y, 5.0),
                mv(MoveKind::Plunge, x0, y, -depth),
                mv(MoveKind::Cut, x1, y, -depth),
                mv(MoveKind::Rapid, x1, y, 5.0),
            ],
            visible: true,
            power: vec![],
            stale: false,
            group: None,
            drill_bank: None,
            fallbacks: vec![],
        }
    }

    fn messages(found: &[Finding], severity: Severity) -> Vec<&str> {
        found
            .iter()
            .filter(|f| f.severity == severity)
            .map(|f| f.message.as_str())
            .collect()
    }

    #[test]
    fn a_good_toolpath_has_nothing_to_say() {
        let found = check(
            &job(),
            &[&slot("Slot", 50.0, 20.0, 180.0, 3.0)],
            &Limits::default(),
        );
        assert_eq!(found, vec![]);
    }

    #[test]
    fn a_rapid_through_the_material_is_a_crash() {
        let mut path = slot("Slot", 50.0, 20.0, 180.0, 3.0);
        // straight across uncut material at full speed, 3 mm down
        path.moves.insert(1, mv(MoveKind::Rapid, 20.0, 100.0, -3.0));
        path.moves.insert(2, mv(MoveKind::Rapid, 20.0, 50.0, -3.0));
        let found = check(&job(), &[&path], &Limits::default());
        let errors = messages(&found, Severity::Error);
        assert_eq!(errors.len(), 1, "{found:?}");
        assert!(
            errors[0].contains("Slot would crash") && errors[0].contains("clearance (Z1)"),
            "{errors:?}"
        );
    }

    #[test]
    fn a_rapid_inside_what_was_cut_is_no_crash() {
        let mut path = slot("Slot", 50.0, 20.0, 180.0, 3.0);
        // back along the slot just cut, at its depth
        path.moves.insert(3, mv(MoveKind::Rapid, 20.0, 50.0, -3.0));
        let found = check(&job(), &[&path], &Limits::default());
        assert_eq!(messages(&found, Severity::Error), Vec::<&str>::new());
    }

    #[test]
    fn cutting_again_where_an_earlier_toolpath_cut_is_air_cutting() {
        let first = slot("Roughing", 50.0, 20.0, 180.0, 3.0);
        let again = slot("Again", 50.0, 20.0, 180.0, 3.0);
        let deeper = slot("Deeper", 50.0, 20.0, 180.0, 6.0);
        let found = check(&job(), &[&first, &again, &deeper], &Limits::default());
        let warnings = messages(&found, Severity::Warning);
        assert_eq!(warnings.len(), 1, "{found:?}");
        assert!(
            warnings[0].starts_with("Again cuts air for 100%"),
            "{warnings:?}"
        );
    }

    #[test]
    fn cutting_above_the_material_is_air_cutting() {
        let above = slot("Above", 50.0, 20.0, 180.0, -1.0);
        let found = check(&job(), &[&above], &Limits::default());
        assert!(
            messages(&found, Severity::Warning)
                .iter()
                .any(|m| m.contains("cuts air")),
            "{found:?}"
        );
    }

    #[test]
    fn toolpaths_that_cannot_cut_are_errors() {
        let mut empty = slot("Empty", 50.0, 20.0, 180.0, 3.0);
        empty.moves.clear();
        let mut still = slot("Still", 80.0, 20.0, 180.0, 3.0);
        still.tool.feed_rate = 0.0;
        still.tool.plunge_rate = 0.0;
        let found = check(&job(), &[&empty, &still], &Limits::default());
        let errors = messages(&found, Severity::Error);
        assert_eq!(errors.len(), 3, "{found:?}");
        assert!(errors[0].contains("Empty has no moves"), "{errors:?}");
        assert!(errors[1].contains("no feed rate"), "{errors:?}");
        assert!(errors[2].contains("no plunge rate"), "{errors:?}");
    }

    #[test]
    fn errors_come_before_warnings() {
        let mut stale = slot("Stale", 50.0, 20.0, 180.0, 3.0);
        stale.stale = true;
        let mut empty = slot("Empty", 80.0, 20.0, 180.0, 3.0);
        empty.moves.clear();
        let found = check(&job(), &[&stale, &empty], &Limits::default());
        assert_eq!(
            found.iter().map(|f| f.severity).collect::<Vec<_>>(),
            vec![Severity::Error, Severity::Warning],
            "{found:?}"
        );
        assert!(found[1].message.contains("out of date"));
    }

    #[test]
    fn the_machine_limits_are_checked() {
        let limits = Limits {
            name: "Shapeoko".into(),
            bed: [100.0, 100.0],
            z_travel: 6.0,
            max_rate: [1000.0, 1200.0, 300.0],
            spindle: [10000.0, 16000.0],
            slots: vec![Some("3 mm Ball Nose".into()), None],
            left: None,
        };
        let found = check(&job(), &[&slot("Slot", 50.0, 20.0, 180.0, 3.0)], &limits);
        let errors = messages(&found, Severity::Error);
        assert_eq!(errors.len(), 2, "{found:?}");
        assert!(
            errors
                .iter()
                .any(|m| m.contains("more than the bed of Shapeoko"))
        );
        assert!(errors.iter().any(|m| m.contains("the Z axis of Shapeoko")));
        let warnings = messages(&found, Severity::Warning);
        assert_eq!(warnings.len(), 4, "{found:?}");
        assert!(
            warnings
                .iter()
                .any(|m| m.contains("no faster than 16000 rpm"))
        );
        assert!(
            warnings
                .iter()
                .any(|m| m.contains("faster than Shapeoko moves (1000"))
        );
        assert!(warnings.iter().any(|m| m.contains("moves in Z (300")));
        assert!(
            warnings
                .iter()
                .any(|m| m.contains("not loaded in the tool changer"))
        );
    }

    #[test]
    fn an_inch_job_hears_about_its_machine_in_inches() {
        let limits = Limits {
            name: "Shapeoko".into(),
            bed: [101.6, 101.6],
            z_travel: 6.0,
            max_rate: [508.0, 635.0, 127.0],
            ..Limits::default()
        };
        let mut doc = job();
        doc.job.units = crate::document::Units::Inch;
        let found = check(&doc, &[&slot("Slot", 50.0, 20.0, 180.0, 3.0)], &limits);
        let all: Vec<&str> = found.iter().map(|f| f.message.as_str()).collect();
        let says = |text: &str| all.iter().any(|m| m.contains(text));
        assert!(says("(4.00 in × 4.00 in)"), "{all:?}");
        assert!(says("faster than Shapeoko moves (20.0 in/min)"), "{all:?}");
        assert!(says("moves in Z (5.0 in/min)"), "{all:?}");
        assert!(says("the 0.24 in the Z axis"), "{all:?}");
        // only the bit's own name says mm
        assert!(
            !all.iter()
                .any(|m| m.replace("6 mm End Mill", "").contains(" mm")),
            "{all:?}"
        );
        let deep = slot("Cut out", 50.0, 20.0, 180.0, 21.0);
        let found = check(&doc, &[&deep], &Limits::default());
        assert!(
            found
                .iter()
                .any(|f| f.message.contains("0.118 in below the material")),
            "{found:?}"
        );
    }

    #[test]
    fn a_tiled_job_fits_the_bed_when_each_tile_does() {
        let limits = Limits {
            name: "Desktop".into(),
            bed: [100.0, 100.0],
            ..Limits::default()
        };
        let wide = slot("Slot", 50.0, 20.0, 180.0, 3.0);
        let mut doc = job();
        assert_eq!(check(&doc, &[&wide], &limits).len(), 1, "too wide untiled");
        doc.tiling = Some(crate::tiling::Tiling {
            width: 80.0,
            height: 80.0,
            overlap: 0.0,
            ..crate::tiling::Tiling::default()
        });
        assert_eq!(check(&doc, &[&wide], &limits), vec![]);
        // tiles bigger than the bed still do not fit
        doc.tiling = Some(crate::tiling::Tiling {
            width: 150.0,
            height: 150.0,
            overlap: 0.0,
            ..crate::tiling::Tiling::default()
        });
        assert_eq!(check(&doc, &[&wide], &limits).len(), 1);
    }

    #[test]
    fn a_cut_well_into_the_spoilboard_is_flagged() {
        let through = slot("Cut out", 50.0, 20.0, 180.0, 18.5);
        let found = check(&job(), &[&through], &Limits::default());
        assert_eq!(found, vec![], "half a millimetre through is on purpose");
        let deep = slot("Cut out", 50.0, 20.0, 180.0, 21.0);
        let found = check(&job(), &[&deep], &Limits::default());
        assert!(
            messages(&found, Severity::Warning)
                .iter()
                .any(|m| m.contains("3.0 mm below the material")),
            "{found:?}"
        );
    }

    #[test]
    fn a_fine_stepover_is_not_air_cutting() {
        // 0.3 mm between rows is finer than the simulation's cells
        let mut raster = slot("Finishing", 20.0, 20.0, 180.0, 1.0);
        raster.moves.truncate(2);
        for row in 0..100 {
            let (y, x) = (
                20.0 + f64::from(row) * 0.3,
                if row % 2 == 0 { 180.0 } else { 20.0 },
            );
            raster.moves.push(mv(
                MoveKind::Cut,
                raster.moves.last().unwrap().to.x,
                y,
                -1.0,
            ));
            raster.moves.push(mv(MoveKind::Cut, x, y, -1.0));
        }
        let found = check(&job(), &[&raster], &Limits::default());
        assert_eq!(found, vec![]);
    }
}
