//! The checks Save Toolpaths makes before writing a file: the job is simulated in cutting
//! order to find rapid moves that would crash into the material and cutting that only cuts
//! air, and each toolpath is checked against the machine (bed, Z travel, speeds, tool
//! changer) and for settings that cannot cut (no moves, no feed rate). These
//! are Cut Craft's own. Every message says how to put the problem right.

pub use crate::check_list::{Check, Phase};
use crate::checks::{LEFT, problems_by_check};
use crate::document::Document;
use crate::progress;
use crate::simulation::{Material, each};
use crate::toolpath::{MoveKind, Tool, ToolShape, Toolpath};
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

/// How far from a toolpath's own moves the simulation's may stray, mm: a 3D finish's steps
/// of a fiftieth of a millimetre come to a tenth as many moves, well within [`CUTS`].
const SIMPLIFIED: f64 = 0.01;

/// Cutting moves the simulation measures at once, across the cores: each sees the material
/// as the batches before it left it, so one cutting again what another in its own batch cut
/// counts as cutting, not air.
const BATCH: usize = 4096;

/// The simulation's cells across the job's longer side: about the smallest tool's radius,
/// within these bounds.
const FEWEST_CELLS: f64 = 100.0;
/// How wide a V-bit's finest stroke is taken to be when sizing the cells, mm.
const V_STROKE: f64 = 1.0;
const MOST_CELLS: f64 = 800.0;

/// Whether a finding stops the save.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    /// Saved only once the user has ticked it as read (or agreed to Save Anyway).
    Warning,
    /// Saved only once the user agrees to Save Anyway, even when ticked: the collet or
    /// spindle hitting the material.
    MustAsk,
    /// Not saved until it is put right.
    Error,
}

/// One thing found, as a sentence saying what is wrong and how to fix it, and the check
/// that found it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Finding {
    pub severity: Severity,
    pub message: String,
    pub check: Check,
}

impl Finding {
    pub fn error(check: Check, message: String) -> Self {
        Self {
            severity: Severity::Error,
            message,
            check,
        }
    }

    pub fn must_ask(check: Check, message: String) -> Self {
        Self {
            severity: Severity::MustAsk,
            message,
            check,
        }
    }

    pub fn warning(check: Check, message: String) -> Self {
        Self {
            severity: Severity::Warning,
            message,
            check,
        }
    }
}

/// How far a bit's feed rate, plunge rate or spindle speed may be from its maker's figure
/// before it is called out, as a share of the figure.
const MAKERS_SHARE: f64 = 0.1;

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
    /// The Tool Database's copy of each bit the toolpaths use, by name, with the cutting
    /// data for the active material: a maker's catalogue copy where there is one. A bit
    /// not in it is not in the Tool Database. `None` when the bits are not compared with
    /// the Tool Database at all.
    pub library: Option<std::collections::BTreeMap<String, Tool>>,
    /// How the machine holds its bits, for the collet and holder checks.
    pub holding: crate::collision::Holding,
    /// The machine's rotary (4th axis), when it has one its firmware can drive.
    pub rotary: Option<crate::rotary_axis::RotaryAxis>,
}

/// Everything found checking `toolpaths`, cut in the order given, on `limits`' machine:
/// errors first, then warnings, each in toolpath order.
pub fn check(doc: &Document, toolpaths: &[&Toolpath], limits: &Limits) -> Vec<Finding> {
    let told_apart = told_apart(toolpaths);
    let toolpaths: Vec<&Toolpath> = told_apart
        .iter()
        .zip(toolpaths)
        .map(|(renamed, &toolpath)| renamed.as_ref().unwrap_or(toolpath))
        .collect();
    let toolpaths = toolpaths.as_slice();
    let mut found = vec![];
    // the settings (with the spots each bit can't reach) fill the first half of the
    // progress bar, cutting the toolpaths in order the second
    progress::part(0, 2, || {
        for (i, toolpath) in toolpaths.iter().enumerate() {
            if progress::cancelled() {
                return;
            }
            progress::stage(&Phase::Settings.stage(&toolpath.name));
            progress::part(i, toolpaths.len(), || {
                settings(doc, toolpath, limits, &mut found);
            });
        }
    });
    if !progress::cancelled() {
        progress::stage(&Phase::Machine.stage(""));
        zeroing(doc, &mut found);
        machine(doc, toolpaths, limits, &mut found);
        keep_out(doc, toolpaths, &mut found);
        tool_numbers(toolpaths, &mut found);
        rotary_setup(doc, limits, &mut found);
    }
    if doc.job.rotary.is_none() && !progress::cancelled() {
        progress::stage(&Phase::Cutting.stage(""));
        progress::part(1, 2, || simulate(doc, toolpaths, &mut found));
    }
    if !progress::cancelled() {
        progress::stage(&Phase::Collisions.stage(""));
        let clamps: Vec<_> = doc.named_keep_out().cloned().collect();
        let hits = crate::collision::find(&doc.job, toolpaths, &limits.holding, &clamps);
        // a warning never switched off: saved only once the user agrees
        found.extend(toolpaths.iter().zip(&hits).filter_map(|(t, hits)| {
            crate::collision::sentence(&t.name, &t.tool, hits, doc.job.units)
                .map(|m| Finding::must_ask(Check::Collisions, m))
        }));
    }
    // stable: each severity keeps toolpath order
    found.sort_by_key(|f| std::cmp::Reverse(f.severity));
    found
}

/// Copies of the toolpaths whose name another one shares, named by which of them each is
/// ("V-Carve (2nd of 2)"), so the findings say which one they mean; `None` for
/// a name of its own.
fn told_apart(toolpaths: &[&Toolpath]) -> Vec<Option<Toolpath>> {
    let named = |name: &str| toolpaths.iter().filter(|t| t.name == name).count();
    let ordinal = |n: usize| match (n % 10, n % 100) {
        (1, k) if k != 11 => format!("{n}st"),
        (2, k) if k != 12 => format!("{n}nd"),
        (3, k) if k != 13 => format!("{n}rd"),
        _ => format!("{n}th"),
    };
    toolpaths
        .iter()
        .enumerate()
        .map(|(i, &toolpath)| {
            let all = named(&toolpath.name);
            (all > 1).then(|| {
                let before = toolpaths[..i]
                    .iter()
                    .filter(|t| t.name == toolpath.name)
                    .count();
                let mut renamed = toolpath.clone();
                renamed.name = format!("{} ({} of {all})", toolpath.name, ordinal(before + 1));
                renamed
            })
        })
        .collect()
}

/// What one toolpath's own settings will not cut with.
fn settings(doc: &Document, toolpath: &Toolpath, limits: &Limits, found: &mut Vec<Finding>) {
    let (name, tool) = (&toolpath.name, &toolpath.tool);
    if toolpath.moves.is_empty() {
        found.push(Finding::error(
            Check::CanCut,
            format!(
                "{name} has no moves: recalculate it, or untick it in the toolpath list so it \
             is not saved"
            ),
        ));
        return;
    }
    if toolpath.stale {
        found.push(Finding::warning(
            Check::UpToDate,
            format!(
                "{name} is out of date: what it was made from has changed since. Recalculate it \
             so it cuts what the design shows now"
            ),
        ));
    }
    let has = |kind| toolpath.moves.iter().any(|m| m.kind == kind);
    if has(MoveKind::Cut)
        && let Some(drill) = drill_sideways(&toolpath.strategy, tool)
    {
        found.push(Finding::warning(
            Check::Drilling,
            format!("{name}: {drill}"),
        ));
    }
    if has(MoveKind::Cut) && tool.feed_rate <= 0.0 {
        found.push(Finding::error(
            Check::CanCut,
            format!(
                "{name}: {} has no feed rate, so the machine would not move while cutting. Set \
             the feed rate in the toolpath's tool settings",
                tool.name
            ),
        ));
    }
    if has(MoveKind::Plunge) && tool.plunge_rate <= 0.0 {
        found.push(Finding::error(
            Check::CanCut,
            format!(
                "{name}: {} has no plunge rate, so the machine would not move down into the \
             material. Set the plunge rate in the toolpath's tool settings",
                tool.name
            ),
        ));
    }
    // a laser has no spindle
    let laser = !toolpath.power.is_empty();
    if !laser && tool.spindle_speed <= 0.0 {
        found.push(Finding::warning(
            Check::CanCut,
            format!(
                "{name}: {} has no spindle speed, so the spindle may not turn on. Set the \
             spindle speed in the toolpath's tool settings",
                tool.name
            ),
        ));
    }
    on_machine(toolpath, laser, limits, doc.job.units, found);
    let deepest = toolpath
        .moves
        .iter()
        .filter(|m| m.kind != MoveKind::Rapid)
        .map(|m| -m.to.z)
        .fold(0.0, f64::max);
    let below = deepest - doc.job.thickness;
    if flattens_spoilboard(toolpath) {
        found.push(Finding::warning(
            Check::Spoilboard,
            format!(
                "{name} flattens the spoilboard, not the material: take everything off the bed, \
             zero X and Y at the spoilboard's front-left corner and Z on top of it"
            ),
        ));
    } else if doc.job.thickness > 0.0 && below > SPOILBOARD {
        found.push(Finding::warning(
            Check::Spoilboard,
            format!(
                "{name} cuts {} below the material, into the spoilboard. Check the material \
             thickness in Job Setup and the toolpath's cut depth",
                doc.job.units.length(below, 1)
            ),
        ));
    }
    found.extend(
        problems_by_check(doc, toolpath, limits.left.unwrap_or(LEFT))
            .into_iter()
            .map(|(check, problem)| Finding::warning(check, problem)),
    );
    library(toolpath, limits, doc.job.units, found);
}

/// Whether `toolpath` is a Surfacing Toolpath flattening the spoilboard: it cuts below the
/// material on purpose, with the material off the bed.
fn flattens_spoilboard(toolpath: &Toolpath) -> bool {
    toolpath.strategy == "surfacing_toolpath" && toolpath.params["area"] == "spoilboard"
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
            found.push(Finding::warning(
                Check::SpindleRange,
                format!(
                    "{name}: {} turns at {:.0} rpm, but the spindle of {} turns no slower than \
                 {slowest:.0} rpm. Raise the spindle speed (and the feed rate with it, to \
                 keep the chip load)",
                    tool.name, tool.spindle_speed, limits.name
                ),
            ));
        }
        if fastest > 0.0 && tool.spindle_speed > fastest {
            found.push(Finding::warning(
                Check::SpindleRange,
                format!(
                    "{name}: {} turns at {:.0} rpm, but the spindle of {} turns no faster than \
                 {fastest:.0} rpm. Lower the spindle speed (and the feed rate with it, to \
                 keep the chip load)",
                    tool.name, tool.spindle_speed, limits.name
                ),
            ));
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
        found.push(Finding::warning(
            Check::MachineSpeeds,
            format!(
                "{name}: the feed rate of {} is {}, faster than {} moves ({}). It will cut slower \
             than planned; lower the feed rate to match",
                tool.name,
                units.feed(tool.feed_rate, 0),
                limits.name,
                units.feed(sideways, 0)
            ),
        ));
    }
    let plunges = toolpath.moves.iter().any(|m| m.kind == MoveKind::Plunge);
    if z > 0.0 && plunges && tool.plunge_rate > z {
        found.push(Finding::warning(
            Check::MachineSpeeds,
            format!(
                "{name}: the plunge rate of {} is {}, faster than {} moves in Z ({}). Lower the \
             plunge rate to match",
                tool.name,
                units.feed(tool.plunge_rate, 0),
                limits.name,
                units.feed(z, 0)
            ),
        ));
    }
    if !limits.slots.is_empty()
        && !limits
            .slots
            .iter()
            .any(|s| s.as_deref() == Some(tool.name.as_str()))
    {
        found.push(Finding::warning(
            Check::ToolChanger,
            format!(
                "{name}: {} is not loaded in the tool changer of {}, so the machine will stop \
             and ask for it. Load it in a slot in Machine Configuration; if it is no longer \
             in the Tool Database, pick a tool again with the toolpath's Select…",
                tool.name, limits.name
            ),
        ));
    }
}

/// Whether each bit is from a maker's catalogue, and its feeds and speeds near the maker's
/// figures, as the Tool Database has them ([`Limits::library`]).
fn library(
    toolpath: &Toolpath,
    limits: &Limits,
    units: crate::document::Units,
    found: &mut Vec<Finding>,
) {
    let Some(library) = &limits.library else {
        return;
    };
    let (name, tool) = (&toolpath.name, &toolpath.tool);
    let Some(listed) = library.get(&tool.name) else {
        found.push(Finding::warning(
            Check::GenericTool,
            format!(
                "{name}: {} is not in the Tool Database, so its feeds and speeds cannot be \
                 compared with a maker's figures. Pick the bit with the toolpath's Select…, \
                 or add it to the Tool Database",
                tool.name
            ),
        ));
        return;
    };
    let maker = listed.manufacturer.trim();
    if maker.is_empty() {
        found.push(Finding::warning(
            Check::GenericTool,
            format!(
                "{name}: {} is a generic bit, not one from a maker's catalogue, so its feeds \
                 and speeds are starting values, not figures made for your bit. Watch the \
                 first cut with a hand near the stop button, or pick your own bit from a \
                 maker's catalogue in the Tool Database",
                tool.name
            ),
        ));
        return;
    }
    let mut changed = vec![];
    let mut compare = |what: &str, ours: f64, makers: f64, shown: &dyn Fn(f64) -> String| {
        if makers > 0.0 && (ours - makers).abs() > makers * MAKERS_SHARE {
            let share = 100.0 * (ours - makers).abs() / makers;
            let way = if ours > makers { "above" } else { "below" };
            changed.push(format!(
                "{what} of {} is {share:.0}% {way} the maker's {}",
                shown(ours),
                shown(makers)
            ));
        }
    };
    let rate = |v: f64| units.feed(v, 0);
    compare("feed rate", tool.feed_rate, listed.feed_rate, &rate);
    compare("plunge rate", tool.plunge_rate, listed.plunge_rate, &rate);
    if toolpath.power.is_empty() {
        compare(
            "spindle speed",
            tool.spindle_speed,
            listed.spindle_speed,
            &|v| format!("{v:.0} rpm"),
        );
    }
    if !changed.is_empty() {
        found.push(Finding::warning(
            Check::MakersFigures,
            format!(
                "{name}: {}'s {}. Faster than the maker says can break the bit; slower can \
                 rub and burn the wood. If you did not mean to change them, pick the bit \
                 again with the toolpath's Select… to put {maker}'s figures back",
                tool.name,
                changed.join("; ")
            ),
        ));
    }
}

/// Whether the toolpaths keep out of the keep-out zones, the job's and the machine's.
fn keep_out(doc: &Document, toolpaths: &[&Toolpath], found: &mut Vec<Finding>) {
    for toolpath in toolpaths {
        let radius = toolpath.tool.diameter / 2.0;
        let zones = crate::keep_out::entered(doc.named_keep_out(), &toolpath.moves, radius);
        if zones.is_empty() {
            continue;
        }
        let names: Vec<String> = zones.iter().map(|z| format!("\"{}\"", z.name)).collect();
        found.push(Finding::error(
            Check::KeepOut,
            format!(
                "{} would cut into the keep-out zone{} {}, where the bit could hit a clamp. \
                 Move the part or the zone, then recalculate the toolpath",
                toolpath.name,
                if zones.len() == 1 { "" } else { "s" },
                names.join(" and ")
            ),
        ));
    }
}

/// Whether two different bits share a tool number other than 0 (changed by hand).
fn tool_numbers(toolpaths: &[&Toolpath], found: &mut Vec<Finding>) {
    let mut by_number: std::collections::BTreeMap<u32, Vec<&str>> =
        std::collections::BTreeMap::new();
    for toolpath in toolpaths {
        let tool = &toolpath.tool;
        if tool.number == 0 {
            continue;
        }
        let bits = by_number.entry(tool.number).or_default();
        if !bits.contains(&tool.name.as_str()) {
            bits.push(&tool.name);
        }
    }
    for (number, bits) in by_number {
        if bits.len() < 2 {
            continue;
        }
        let (last, rest) = bits.split_last().unwrap_or((&"", &[]));
        found.push(Finding::warning(
            Check::ToolNumbers,
            format!(
                "Tool number {number} is given to both {} and {last}: a tool changer picks \
                 bits by number, so one of them would cut with the wrong bit. Give each bit \
                 its own number in the Tool Database (0 means the bit is changed by hand, \
                 and any number of bits may share 0)",
                rest.join(", ")
            ),
        ));
    }
}

/// A rotary job on a machine whose rotary is not set up for it: its centre line not found
/// yet, or lying the other way from the job's cylinder.
fn rotary_setup(doc: &Document, limits: &Limits, found: &mut Vec<Finding>) {
    let (Some(job), Some(axis)) = (doc.job.rotary, limits.rotary) else {
        return;
    };
    if !axis.centre_set {
        found.push(Finding::warning(
            Check::RotarySetup,
            "The rotary's centre line is not set in Machine Configuration. If the bit is not \
             exactly over the turning axis, a round carving comes out lumpy: find the centre \
             line (see the help), then tick Centre line found"
                .to_owned(),
        ));
    }
    if job.along != axis.along {
        found.push(Finding::warning(
            Check::RotarySetup,
            format!(
                "This job's cylinder lies along {}, but the machine's rotary lies along {}: \
                 change the job in Job Setup, or the rotary's direction in Machine Configuration",
                job.along.letter(),
                axis.along.letter()
            ),
        ));
    }
}

/// A job whose material was never set up (Material Setup not shown, as when the
/// toolpaths came from a template): where to zero the bit, from Job Defaults, to read.
fn zeroing(doc: &Document, found: &mut Vec<Finding>) {
    if doc.material_checked {
        return;
    }
    let job = &doc.job;
    let z = match (job.z_zero, job.rotary.is_some()) {
        (crate::document::ZZero::MaterialSurface, false) => "on top of the material",
        (crate::document::ZZero::MachineBed, false) => "on the machine bed",
        (crate::document::ZZero::MaterialSurface, true) => "on top of the cylinder",
        (crate::document::ZZero::MachineBed, true) => "on the cylinder's axis",
    };
    let xy = match job.datum {
        crate::document::Datum::BottomLeft => "at the material's bottom left corner",
        crate::document::Datum::TopLeft => "at the material's top left corner",
        crate::document::Datum::TopRight => "at the material's top right corner",
        crate::document::Datum::BottomRight => "at the material's bottom right corner",
        crate::document::Datum::Centre => "in the middle of the material",
    };
    let offset = if job.offset == crate::geometry::Point::ZERO {
        String::new()
    } else {
        format!(
            ", moved {} {u} across and {} {u} up",
            trim(job.offset.x / job.units.per()),
            trim(job.offset.y / job.units.per()),
            u = job.units.suffix()
        )
    };
    found.push(Finding::warning(
        Check::Zeroing,
        format!(
            "This job's zero comes from your Job Defaults and was not looked at: zero the \
             bit's Z {z}, and X0 Y0 {xy}{offset}. To change them for this job, open Material \
             Setup on the Toolpaths page"
        ),
    ));
}

/// `value` to three places at most, without trailing zeros.
fn trim(value: f64) -> String {
    let text = format!("{value:.3}");
    text.trim_end_matches('0').trim_end_matches('.').to_owned()
}

/// The checks that cannot be made on `toolpaths` with `limits`, each with why: the
/// machine not saying what it can do, no tool changer or keep-out zones, a rotary job.
#[must_use]
pub fn not_checked(
    doc: &Document,
    toolpaths: &[&Toolpath],
    limits: &Limits,
) -> Vec<(Check, String)> {
    let machine = if limits.name.is_empty() {
        "the machine".to_owned()
    } else {
        limits.name.clone()
    };
    let config = |what: &str| format!("{machine} has no {what} in Machine Configuration");
    let rotary = doc.job.rotary.is_some();
    let mut skipped = vec![];
    if rotary {
        skipped.push((
            Check::Bed,
            "A rotary job wraps round the material, not along the bed".to_owned(),
        ));
    } else if limits.bed[0] <= 0.0 || limits.bed[1] <= 0.0 {
        skipped.push((Check::Bed, config("bed size")));
    }
    if limits.z_travel <= 0.0 {
        skipped.push((Check::ZTravel, config("Z travel")));
    }
    if limits.max_rate.iter().all(|&r| r <= 0.0) {
        skipped.push((Check::MachineSpeeds, config("top speeds")));
    }
    if limits.spindle.iter().all(|&r| r <= 0.0) {
        skipped.push((Check::SpindleRange, config("spindle speed range")));
    }
    if limits.slots.is_empty() {
        skipped.push((
            Check::ToolChanger,
            "No tool changer is used: you change the bits".to_owned(),
        ));
    }
    if doc.named_keep_out().next().is_none() {
        skipped.push((
            Check::KeepOut,
            "The job and the machine have no keep-out zones".to_owned(),
        ));
    }
    if rotary {
        for check in [Check::Rapids, Check::AirCutting] {
            skipped.push((
                check,
                "A rotary job is not cut through in order yet: watch its first moves".to_owned(),
            ));
        }
    }
    if !rotary {
        skipped.push((Check::RotarySetup, "Not a rotary job".to_owned()));
    } else if limits.rotary.is_none() {
        skipped.push((Check::RotarySetup, config("4th axis")));
    }
    if limits.library.is_none() {
        for check in [Check::GenericTool, Check::MakersFigures] {
            skipped.push((
                check,
                "The bits were not compared with the Tool Database".to_owned(),
            ));
        }
    }
    if toolpaths.iter().all(|t| t.tool.number == 0) {
        skipped.push((
            Check::ToolNumbers,
            "Every bit is changed by hand (tool number 0)".to_owned(),
        ));
    }
    skipped
}

/// What to say when a toolpath of `strategy` (a command id such as `profile_toolpath`)
/// cuts with `tool` and it is a drill, which cuts only straight down: in red at the foot of
/// the toolpath's form, and in the checks before saving. `None` for any other bit, or a
/// Drilling toolpath.
pub fn drill_sideways(strategy: &str, tool: &crate::toolpath::Tool) -> Option<String> {
    let drilling = matches!(strategy, "drilling_toolpath" | "drill_bank_toolpath");
    (tool.drill_only && !drilling).then(|| {
        format!(
            "{} is a drill, which cuts only straight down. Moved sideways it can snap or \
             tear the work: pick an end mill for this toolpath, or use the drill in a \
             Drilling toolpath",
            tool.name
        )
    })
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
        found.push(Finding::error(
            Check::Bed,
            format!(
                "The toolpaths cover {} × {}, more than the bed of {} ({} × {}). Make the job \
             smaller, split it with Toolpath Tiling (or make its tiles smaller), or pick the \
             right machine",
                units.length(span.x, 0),
                units.length(span.y, 0),
                limits.name,
                units.length(width, 0),
                units.length(height, 0)
            ),
        ));
    }
    if limits.z_travel > 0.0 && span.z > limits.z_travel {
        found.push(Finding::error(
            Check::ZTravel,
            format!(
                "The toolpaths go from {} above the material to {} below it, more than the {} \
             the Z axis of {} travels. Lower the clearance (Z1) in Job Setup or cut less deep",
                units.length(hi.z.max(0.0), 1),
                units.length((-lo.z).max(0.0), 1),
                units.length(limits.z_travel, 0),
                limits.name
            ),
        ));
    }
}

/// Cuts the toolpaths in order from uncut material, finding rapid moves that hit it and
/// cutting moves that take nothing off.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "a cell count within bounds; a fraction for a progress bar"
)]
fn simulate(doc: &Document, toolpaths: &[&Toolpath], found: &mut Vec<Finding>) {
    let job = &doc.job;
    let units = job.units;
    // a point's X or Y as the status bar shows it
    let position = |mm: f64| match units {
        crate::document::Units::Mm => format!("{mm:.1}"),
        crate::document::Units::Inch => format!("{:.3}", mm / 25.4),
    };
    // a V-bit's cut is as narrow as it is shallow, however wide the bit: its fine strokes
    // need the finest cells
    let smallest = toolpaths
        .iter()
        .filter(|t| t.tool.diameter > 0.0)
        .map(|t| match t.tool.shape {
            ToolShape::VBit => V_STROKE,
            _ => t.tool.diameter,
        })
        .fold(f64::INFINITY, f64::min);
    if !smallest.is_finite() || job.width <= 0.0 || job.height <= 0.0 {
        return;
    }
    let cells = (job.width.max(job.height) / (smallest / 2.0)).clamp(FEWEST_CELLS, MOST_CELLS);
    let mut material = Material::new(job, cells as usize);
    // how far through the moves of all the toolpaths, for the progress bar
    let total = toolpaths
        .iter()
        .map(|t| t.moves.len())
        .sum::<usize>()
        .max(1) as f64;
    let mut moved = 0.0;
    for (n, toolpath) in (1..).zip(toolpaths) {
        // what this toolpath cuts is marked as its own
        material.cutting = n;
        // a laser burns rather than cuts away; the spoilboard is flattened with nothing on it
        if !toolpath.power.is_empty() || flattens_spoilboard(toolpath) {
            continue;
        }
        let tool = &toolpath.tool;
        let share = (moved / total, toolpath.moves.len() as f64 / total);
        let Some(Cut {
            crashes,
            worst,
            cutting,
            air,
        }) = cut_in_order(&mut material, toolpath, n, share)
        else {
            return;
        };
        moved += toolpath.moves.len() as f64;
        let name = &toolpath.name;
        if let Some((into, at)) = worst {
            let more = match crashes {
                1 => String::new(),
                n => format!(" ({n} rapid moves in all)"),
            };
            found.push(Finding::error(
                Check::Rapids,
                format!(
                    "{name} would crash: a rapid move to X{} Y{} goes {} into material that is \
                 still there{more}. Rapid moves run at full speed and are meant to stay clear \
                 of the material. Raise the clearance (Z1) in Job Setup, or recalculate the \
                 toolpath",
                    position(at.x),
                    position(at.y),
                    units.length(into, 1)
                ),
            ));
        }
        if cutting > 0.0 && air > AIR_LENGTH && air / cutting > AIR_SHARE {
            let minutes = if tool.feed_rate > 0.0 {
                format!(", about {:.0} min", (air / tool.feed_rate).ceil())
            } else {
                String::new()
            };
            found.push(Finding::warning(
                Check::AirCutting,
                format!(
                    "{name} cuts air for {:.0}% of its cutting ({:.1} m{minutes}), where the \
                 material is already gone. If an earlier toolpath cleared this area, cut \
                 less with this one: a smaller area, fewer passes or a shallower start depth",
                    100.0 * air / cutting,
                    air / 1000.0
                ),
            ));
        }
    }
}

/// What cutting one toolpath in order found: its rapid moves into the material (how many,
/// and the deepest with where it went), and how far it cut, in all and in air.
struct Cut {
    crashes: usize,
    worst: Option<(f64, DVec3)>,
    cutting: f64,
    air: f64,
}

/// Cuts `toolpath`, the `n`th, into `material`, finding rapid moves that hit it and cutting
/// moves that take nothing off; the progress bar runs over `share` (where it starts and
/// how much of the bar this toolpath takes). `None` once the person cancelled.
#[expect(clippy::cast_precision_loss, reason = "a fraction for a progress bar")]
fn cut_in_order(
    material: &mut Material,
    toolpath: &Toolpath,
    n: u32,
    (start, width): (f64, f64),
) -> Option<Cut> {
    let tool = &toolpath.tool;
    let clear = clear_of(tool, material.cell);
    // the peaks only come down as the material is cut, so they stay safe to search by
    let peaks = material.peaks();
    let mut cut = Cut {
        crashes: 0,
        worst: None,
        cutting: 0.0,
        air: 0.0,
    };
    // a 3D finish's millions of tiny steps come to far fewer moves within a hair of them,
    // which take off the same material on these cells
    let simplified = crate::simplify::simplify(&toolpath.moves, SIMPLIFIED);
    let mut batch: Vec<(DVec3, DVec3, MoveKind)> = Vec::with_capacity(BATCH);
    // cuts and plunges a batch at a time across the cores, each measured against the
    // material as it was before its batch; rapids one at a time against all cut so far
    let flush = |material: &mut Material, batch: &mut Vec<_>, cut: &mut Cut| {
        let takes = each(batch, |&(a, b, kind): &(DVec3, DVec3, MoveKind)| {
            (kind == MoveKind::Cut).then(|| material.overcut_near(tool, a, b, &peaks, 2.0 * clear))
        });
        for (&(a, b, _), takes) in batch.iter().zip(takes) {
            let Some(takes) = takes else { continue };
            let length = a.distance(b);
            cut.cutting += length;
            if takes < -clear || (takes <= CUTS && cut_before(material, b, n)) {
                cut.air += length;
            }
        }
        let pairs: Vec<(DVec3, DVec3)> = batch.iter().map(|&(a, b, _)| (a, b)).collect();
        material.sweep_each(&pairs, tool);
        batch.clear();
    };
    let steps = simplified.len().max(1) as f64;
    for (done, pair) in simplified.windows(2).enumerate() {
        let (a, b) = (pair[0].to, pair[1].to);
        if pair[1].kind != MoveKind::Rapid {
            batch.push((a, b, pair[1].kind));
            if batch.len() == BATCH {
                if progress::cancelled() {
                    return None;
                }
                flush(material, &mut batch, &mut cut);
                progress::report(start + width * done as f64 / steps);
            }
            continue;
        }
        flush(material, &mut batch, &mut cut);
        let into = material.overcut_near(tool, a, b, &peaks, clear);
        if into > CRASH {
            cut.crashes += 1;
            if cut.worst.is_none_or(|(deepest, _)| into > deepest) {
                cut.worst = Some((into, b));
            }
        }
        material.sweep(a, b, tool);
    }
    flush(material, &mut batch, &mut cut);
    Some(cut)
}

/// How far clear of the material `tool`'s cut must seem, on cells `cell` wide, to be
/// cutting air: [`CLEAR`], or more for a V-bit, whose sloping side can pass a cell's centre
/// by half a cell's diagonal while its tip cuts a groove between the centres.
fn clear_of(tool: &Tool, cell: f64) -> f64 {
    let half = (tool.angle / 2.0).to_radians();
    if tool.shape != ToolShape::VBit || half <= 0.0 || half >= std::f64::consts::FRAC_PI_2 {
        return CLEAR;
    }
    CLEAR.max(cell * std::f64::consts::FRAC_1_SQRT_2 / half.tan())
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
            // its zero looked at in Material Setup: no Zeroing warning in every test
            material_checked: true,
            ..Document::default()
        }
    }

    #[test]
    fn a_zero_never_looked_at_is_said_where_to_set() {
        let mut doc = job();
        let path = slot("Profile", 150.0, 20.0, 180.0, 3.0);
        let zeroing = |doc: &Document| {
            check(doc, &[&path], &Limits::default())
                .into_iter()
                .filter(|f| f.check == Check::Zeroing)
                .collect::<Vec<_>>()
        };
        assert!(zeroing(&doc).is_empty(), "looked at in Material Setup");
        doc.material_checked = false;
        let found = zeroing(&doc);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].severity, Severity::Warning, "to read and tick");
        assert!(
            found[0]
                .message
                .contains("zero the bit's Z on top of the material, and X0 Y0 at the material's bottom left corner. "),
            "{}",
            found[0].message
        );
        doc.job.datum = crate::document::Datum::Centre;
        doc.job.offset = crate::geometry::Point::new(10.0, 5.0);
        assert!(
            zeroing(&doc)[0]
                .message
                .contains("in the middle of the material, moved 10 mm across and 5 mm up."),
        );
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

    fn checks_of(found: &[Finding]) -> Vec<Check> {
        found.iter().map(|f| f.check).collect()
    }

    #[test]
    fn a_rotary_job_asks_for_the_rotarys_centre_line_and_its_direction() {
        use crate::document::Along;
        use crate::rotary_axis::RotaryAxis;
        let doc = Document {
            job: Job::rotary(200.0, 50.0, Along::Y),
            ..Document::default()
        };
        let cut = slot("Wrap", 20.0, 10.0, 60.0, 2.0);
        let rotary_findings = |limits: &Limits, doc: &Document| -> Vec<String> {
            check(doc, &[&cut], limits)
                .into_iter()
                .filter(|f| f.check == Check::RotarySetup)
                .map(|f| f.message)
                .collect()
        };
        let mut limits = Limits {
            rotary: Some(RotaryAxis::default()),
            ..Limits::default()
        };
        let said = rotary_findings(&limits, &doc);
        assert_eq!(said.len(), 2, "{said:?}");
        assert!(said[0].contains("centre line is not set"), "{said:?}");
        assert!(
            said[1].contains("lies along Y") && said[1].contains("along X"),
            "{said:?}"
        );
        limits.rotary = Some(RotaryAxis {
            centre_set: true,
            along: Along::Y,
            ..RotaryAxis::default()
        });
        assert_eq!(rotary_findings(&limits, &doc), Vec::<String>::new());
        assert!(
            !not_checked(&doc, &[&cut], &limits)
                .iter()
                .any(|(c, _)| *c == Check::RotarySetup)
        );
        let flat = job();
        assert_eq!(rotary_findings(&limits, &flat), Vec::<String>::new());
        assert!(
            not_checked(&flat, &[&cut], &limits)
                .iter()
                .any(|(c, why)| *c == Check::RotarySetup && why == "Not a rotary job")
        );
        limits.rotary = None;
        assert!(
            not_checked(&doc, &[&cut], &limits)
                .iter()
                .any(|(c, why)| *c == Check::RotarySetup && why.contains("4th axis"))
        );
    }

    #[test]
    fn each_finding_names_the_check_that_found_it() {
        let limits = Limits {
            name: "Shapeoko".into(),
            bed: [100.0, 100.0],
            slots: vec![Some("3 mm Ball Nose".into())],
            ..Limits::default()
        };
        let crash = Toolpath {
            moves: vec![
                mv(MoveKind::Rapid, 10.0, 10.0, 5.0),
                mv(MoveKind::Rapid, 10.0, 10.0, -5.0),
            ],
            ..slot("Crash", 10.0, 10.0, 10.0, 0.0)
        };
        let found = check(
            &job(),
            &[&slot("Slot", 50.0, 20.0, 180.0, 3.0), &crash],
            &limits,
        );
        let checks = checks_of(&found);
        assert!(checks.contains(&Check::Bed), "{found:?}");
        assert!(checks.contains(&Check::ToolChanger), "{found:?}");
        assert!(checks.contains(&Check::Rapids), "{found:?}");
    }

    #[test]
    fn cutting_into_a_keep_out_zone_must_be_fixed() {
        let mut doc = job();
        let clamp = crate::testing::square(90.0, 40.0, 20.0);
        doc.keep_out = vec![crate::keep_out::KeepOutZone::new("Clamp 1", clamp)];
        let found = check(
            &doc,
            &[&slot("Slot", 50.0, 20.0, 180.0, 3.0)],
            &Limits::default(),
        );
        let zone: Vec<&Finding> = found.iter().filter(|f| f.check == Check::KeepOut).collect();
        assert_eq!(zone.len(), 1, "{found:?}");
        assert_eq!(zone[0].severity, Severity::Error);
        assert!(
            zone[0].message.contains("\"Clamp 1\""),
            "{}",
            zone[0].message
        );
        // clear of the zone: nothing to say
        let found = check(
            &doc,
            &[&slot("Slot", 150.0, 20.0, 180.0, 3.0)],
            &Limits::default(),
        );
        assert!(!checks_of(&found).contains(&Check::KeepOut), "{found:?}");
    }

    #[test]
    fn two_bits_with_one_tool_number_are_called_out_but_hand_changed_ones_are_not() {
        let mut first = slot("Pocket", 50.0, 20.0, 180.0, 3.0);
        let mut second = slot("V-Carve", 100.0, 20.0, 180.0, 1.0);
        second.tool.name = "60° V-Bit".into();
        first.tool.number = 3;
        second.tool.number = 3;
        let found = check(&job(), &[&first, &second], &Limits::default());
        let numbers: Vec<&Finding> = found
            .iter()
            .filter(|f| f.check == Check::ToolNumbers)
            .collect();
        assert_eq!(numbers.len(), 1, "{found:?}");
        assert!(
            numbers[0]
                .message
                .starts_with("Tool number 3 is given to both 6 mm End Mill and 60° V-Bit"),
            "{}",
            numbers[0].message
        );
        // the same bit twice, or bits changed by hand (0), share a number safely
        let mut again = slot("Profile", 150.0, 20.0, 180.0, 3.0);
        again.tool.number = 3;
        second.tool.number = 0;
        first.tool.number = 3;
        let mut hand = second.clone();
        hand.tool.name = "1/4\" Ball Nose".into();
        let found = check(
            &job(),
            &[&first, &again, &second, &hand],
            &Limits::default(),
        );
        assert!(
            !checks_of(&found).contains(&Check::ToolNumbers),
            "{found:?}"
        );
    }

    fn with_library(tools: Vec<Tool>) -> Limits {
        Limits {
            library: Some(tools.into_iter().map(|t| (t.name.clone(), t)).collect()),
            ..Limits::default()
        }
    }

    #[test]
    fn a_generic_bit_or_one_not_in_the_tool_database_is_called_out() {
        let slot = slot("Slot", 50.0, 20.0, 180.0, 3.0);
        let found = check(&job(), &[&slot], &with_library(vec![tool()]));
        let generic: Vec<&str> = found
            .iter()
            .filter(|f| f.check == Check::GenericTool)
            .map(|f| f.message.as_str())
            .collect();
        assert_eq!(generic.len(), 1, "{found:?}");
        assert!(generic[0].contains("is a generic bit"), "{}", generic[0]);
        let found = check(&job(), &[&slot], &with_library(vec![]));
        assert!(
            found
                .iter()
                .any(|f| f.check == Check::GenericTool
                    && f.message.contains("not in the Tool Database")),
            "{found:?}"
        );
        // not compared at all: nothing said, and the list says why
        let found = check(&job(), &[&slot], &Limits::default());
        assert!(
            !checks_of(&found).contains(&Check::GenericTool),
            "{found:?}"
        );
        let skipped = not_checked(&job(), &[&slot], &Limits::default());
        assert!(skipped.iter().any(|(c, _)| *c == Check::GenericTool));
    }

    #[test]
    fn feeds_and_speeds_changed_from_the_makers_figures_are_called_out() {
        let makers = Tool {
            manufacturer: "Amana".into(),
            feed_rate: 1000.0,
            plunge_rate: 480.0,
            spindle_speed: 18000.0,
            ..tool()
        };
        // feed 1500 is 50% above 1000; plunge 500 within a tenth of 480; spindle the same
        let found = check(
            &job(),
            &[&slot("Slot", 50.0, 20.0, 180.0, 3.0)],
            &with_library(vec![makers.clone()]),
        );
        let changed: Vec<&str> = found
            .iter()
            .filter(|f| f.check == Check::MakersFigures)
            .map(|f| f.message.as_str())
            .collect();
        assert_eq!(changed.len(), 1, "{found:?}");
        assert!(
            changed[0].contains("feed rate of 1500 mm/min is 50% above the maker's 1000 mm/min"),
            "{}",
            changed[0]
        );
        assert!(!changed[0].contains("plunge"), "{}", changed[0]);
        assert!(changed[0].contains("Amana's figures"), "{}", changed[0]);
        assert!(
            !checks_of(&found).contains(&Check::GenericTool),
            "{found:?}"
        );
        // as the maker gives: nothing to say
        let same = Tool {
            feed_rate: 1500.0,
            ..makers
        };
        let found = check(
            &job(),
            &[&slot("Slot", 50.0, 20.0, 180.0, 3.0)],
            &with_library(vec![same]),
        );
        assert!(
            !checks_of(&found).contains(&Check::MakersFigures),
            "{found:?}"
        );
    }

    #[test]
    fn checks_that_cannot_be_made_say_why() {
        let slot = slot("Slot", 50.0, 20.0, 180.0, 3.0);
        let skipped = not_checked(&job(), &[&slot], &Limits::default());
        let checks: Vec<Check> = skipped.iter().map(|(c, _)| *c).collect();
        for check in [
            Check::Bed,
            Check::ZTravel,
            Check::MachineSpeeds,
            Check::SpindleRange,
            Check::ToolChanger,
            Check::KeepOut,
            Check::ToolNumbers,
        ] {
            assert!(checks.contains(&check), "{check:?} in {skipped:?}");
        }
        assert!(
            skipped.iter().any(|(c, why)| *c == Check::Bed
                && why == "the machine has no bed size in Machine Configuration"),
            "{skipped:?}"
        );
        let limits = Limits {
            name: "Shapeoko".into(),
            bed: [800.0, 800.0],
            z_travel: 100.0,
            max_rate: [5000.0, 5000.0, 1000.0],
            spindle: [10000.0, 24000.0],
            slots: vec![Some("6 mm End Mill".into())],
            left: None,
            library: Some(std::collections::BTreeMap::new()),
            holding: crate::collision::Holding::default(),
            rotary: None,
        };
        let skipped = not_checked(&job(), &[&slot], &limits);
        assert!(
            skipped.iter().all(|(c, _)| matches!(
                c,
                Check::KeepOut | Check::ToolNumbers | Check::RotarySetup
            )),
            "{skipped:?}"
        );
    }

    #[test]
    fn the_collet_hitting_the_material_asks_first() {
        let mut deep = slot("Deep slot", 100.0, 50.0, 150.0, 30.0);
        deep.tool.flute_length = 32.0;
        deep.tool.stickout = 25.0;
        let doc = Document {
            job: Job {
                thickness: 40.0,
                ..job().job
            },
            ..Document::default()
        };
        let found = check(&doc, &[&deep], &Limits::default());
        let warned = messages(&found, Severity::MustAsk);
        assert!(
            warned
                .iter()
                .any(|m| m.starts_with("Deep slot: the collet nut would hit the material")),
            "{warned:?}"
        );
        assert!(
            found
                .iter()
                .filter(|f| f.severity == Severity::MustAsk)
                .all(|f| f.check == Check::Collisions),
            "{found:?}"
        );
    }

    fn messages(found: &[Finding], severity: Severity) -> Vec<&str> {
        found
            .iter()
            .filter(|f| f.severity == severity)
            .map(|f| f.message.as_str())
            .collect()
    }

    /// A 3D finish's raster as a 1/64" tapered ball nose cuts it: rows `stepover` apart
    /// in steps of a fiftieth of a millimetre over a gently waving surface.
    #[expect(clippy::cast_precision_loss, reason = "a row number")]
    fn fine_finish(rows: usize, stepover: f64) -> Toolpath {
        let mut finish = slot("3D Finishing", 0.0, 0.0, 0.0, 0.0);
        finish.strategy = "3d_finishing_toolpath".into();
        finish.tool = Tool {
            name: "TBN 1/64\"".into(),
            shape: ToolShape::BallNose,
            diameter: 0.396_875,
            taper: 6.3,
            shank_diameter: 6.35,
            flute_length: 25.0,
            ..tool()
        };
        let mut moves = vec![mv(MoveKind::Rapid, 0.0, 0.0, 5.0)];
        for row in 0..rows {
            let y = 10.0 + stepover * row as f64;
            moves.push(mv(MoveKind::Rapid, 0.0, y, 5.0));
            moves.push(mv(MoveKind::Plunge, 0.0, y, -1.0));
            for step in 1..=10_000 {
                let x = 0.02 * f64::from(step);
                moves.push(mv(MoveKind::Cut, x, y, -1.0 - 0.5 * (x / 5.0).sin()));
            }
            moves.push(mv(MoveKind::Rapid, 200.0, y, 5.0));
        }
        finish.moves = moves;
        finish
    }

    #[test]
    fn a_3d_finish_of_millions_of_tiny_steps_is_checked_in_moments() {
        // Matthew's lock-up (v0.6.10): Analyse on a 3D finish of 12.7 million moves ran the
        // simulation a move at a time on one core, about a minute per million moves. A
        // million here took over a minute before; simplified to the simulation's cells and
        // measured a batch at a time across the cores, a few seconds at most.
        let finish = fine_finish(100, 0.16);
        assert!(finish.moves.len() > 1_000_000);
        let started = std::time::Instant::now();
        let found = check(&job(), &[&finish], &Limits::default());
        let took = started.elapsed();
        assert!(took < std::time::Duration::from_secs(20), "took {took:?}");
        // the first rows cut uncut material: no crash and no air
        assert!(messages(&found, Severity::Error).is_empty(), "{found:?}");
        assert!(
            !messages(&found, Severity::Warning)
                .iter()
                .any(|m| m.contains("cuts air")),
            "{found:?}"
        );
    }

    #[test]
    fn a_fine_finish_run_twice_still_cuts_air_the_second_time() {
        // simplified and batched, the second pass over what the first cut is still air
        let first = fine_finish(20, 0.16);
        let mut again = first.clone();
        again.name = "Again".into();
        let found = check(&job(), &[&first, &again], &Limits::default());
        let warnings = messages(&found, Severity::Warning);
        assert!(
            warnings.iter().any(|m| m.starts_with("Again cuts air")),
            "{warnings:?}"
        );
        assert!(
            !warnings
                .iter()
                .any(|m| m.starts_with("3D Finishing cuts air")),
            "{warnings:?}"
        );
    }

    #[test]
    fn a_drill_cutting_sideways_is_warned_about_and_drilling_with_it_is_not() {
        let mut sideways = slot("Profile", 50.0, 20.0, 80.0, 3.0);
        sideways.tool.drill_only = true;
        sideways.tool.name = "Drill 6mm".into();
        let mut drilling = sideways.clone();
        drilling.strategy = "drilling_toolpath".into();
        drilling.name = "Holes".into();
        let warned = |t: &Toolpath| {
            check(&job(), &[t], &Limits::default())
                .iter()
                .any(|f| f.severity == Severity::Warning && f.message.contains("is a drill"))
        };
        assert!(warned(&sideways));
        assert!(!warned(&drilling));
        assert!(!warned(&slot("Profile", 50.0, 20.0, 80.0, 3.0)));
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
    fn fine_v_carving_on_a_big_job_is_not_air() {
        // a 3/4 in V-bit carving grooves 1 mm deep, 2 mm wide, into fresh stock on a
        // 1.2 m job: cells sized by the bit's width (9.5 mm) missed the thin grooves and
        // called 97% of a fresh V-Carve air
        let mut doc = job();
        doc.job.width = 1200.0;
        doc.job.height = 1200.0;
        let mut carve = slot("V-Carve", 50.0, 20.0, 180.0, 1.0);
        carve.tool = Tool {
            name: "3/4 in V-Bit".into(),
            shape: crate::toolpath::ToolShape::VBit,
            diameter: 19.05,
            angle: 90.0,
            ..tool()
        };
        carve.moves.clear();
        for k in 0..20 {
            let y = 50.0 + 3.1 * f64::from(k);
            carve.moves.extend([
                mv(MoveKind::Rapid, 20.0, y, 5.0),
                mv(MoveKind::Plunge, 20.0, y, -1.0),
                mv(MoveKind::Cut, 180.0, y, -1.0),
                mv(MoveKind::Rapid, 180.0, y, 5.0),
            ]);
        }
        let found = check(&doc, &[&carve], &Limits::default());
        assert!(
            !messages(&found, Severity::Warning)
                .iter()
                .any(|m| m.contains("cuts air")),
            "{found:?}"
        );
    }

    #[test]
    fn toolpaths_with_the_same_name_are_told_apart() {
        // calculating twice leaves two toolpaths of one name: which one cuts air?
        let first = slot("V-Carve", 50.0, 20.0, 180.0, 3.0);
        let again = slot("V-Carve", 50.0, 20.0, 180.0, 3.0);
        let found = check(&job(), &[&first, &again], &Limits::default());
        let warnings = messages(&found, Severity::Warning);
        assert_eq!(warnings.len(), 1, "{found:?}");
        assert!(
            warnings[0].starts_with("V-Carve (2nd of 2) cuts air for 100%"),
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
            library: None,
            holding: crate::collision::Holding::default(),
            rotary: None,
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
        // a tool deleted from the Tool Database cannot be loaded: the advice says what to do
        assert!(
            warnings
                .iter()
                .any(|m| m.contains("pick a tool again with the toolpath's Select"))
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
    fn flattening_the_spoilboard_is_a_reminder_not_a_cut_too_deep() {
        // zeroed on the bed, a skim off the spoilboard is under the 20 mm material
        let mut skim = slot("Surfacing", 50.0, 20.0, 180.0, 20.25);
        skim.strategy = "surfacing_toolpath".into();
        skim.params = serde_json::json!({"area": "spoilboard"});
        let found = check(&job(), &[&skim], &Limits::default());
        let warnings = messages(&found, Severity::Warning);
        assert!(
            !warnings.iter().any(|m| m.contains("below the material")),
            "{found:?}"
        );
        assert!(
            warnings
                .iter()
                .any(|m| m.contains("Surfacing flattens the spoilboard")),
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
