//! The checks Save Toolpaths makes before writing a file: the job is simulated in cutting
//! order to find rapid moves that would crash into the material and cutting that only cuts
//! air, and each toolpath is checked against the machine (bed, Z travel, speeds, tool
//! changer) and for settings that cannot cut (no moves, no feed rate). These
//! are Cut Craft's own. Every message says how to put the problem right.

pub use crate::check_list::{Check, Phase};
use crate::checks::{LEFT, problems_by_check};
use crate::document::{Along, Document};
use crate::progress;
use crate::simulation::Material;
use crate::toolpath::{MoveKind, Tool, ToolShape, Toolpath};
use crate::work::each;
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

/// How far past the depth allowed into the spoilboard a cut may seem to go before it is
/// refused, mm: rounding in the moves, not a deeper cut.
const DEPTH_SLACK: f64 = 0.001;

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
    /// Saved only once the user ticks it and agrees to Save Anyway: the operator decides.
    Error,
    /// Never saved, sent or made through the Web API: a risk of damage to the machine (a
    /// keep-out zone, past the bed or the Z travel, too deep into the spoilboard). Only
    /// switching its check off for the machine in Machine Configuration lets it past.
    Damage,
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
    pub fn damage(check: Check, message: String) -> Self {
        Self {
            severity: Severity::Damage,
            message,
            check,
        }
    }

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

/// Which checks protect the machine from damage, as its Machine Configuration has them:
/// each refuses what it finds (no Save Anyway) while on, and is said to be switched off in
/// the checks list and the G-code while off.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "each is a switch of its own in Machine Configuration"
)]
pub struct Protect {
    /// A cut into a keep-out zone, the job's or the machine's, is refused.
    pub keep_out: bool,
    /// Toolpaths bigger than the bed are refused.
    pub bed: bool,
    /// Toolpaths going further up and down than the Z axis travels are refused.
    pub z_travel: bool,
    /// A cut deeper into the spoilboard than [`Self::spoilboard_depth`] is refused.
    pub spoilboard: bool,
    /// How far below the material a cut may go into the spoilboard, mm.
    pub spoilboard_depth: f64,
    /// A cut more than 1 mm into the spoilboard, within the depth allowed, is a warning to
    /// read and tick: a wrong material thickness shows first this way.
    pub warn_spoilboard: bool,
}

impl Default for Protect {
    fn default() -> Self {
        Self {
            keep_out: true,
            bed: true,
            z_travel: true,
            spoilboard: true,
            spoilboard_depth: SPOILBOARD,
            warn_spoilboard: true,
        }
    }
}

impl Protect {
    /// The checks switched off, by name, as the G-code and the checks list say them.
    #[must_use]
    pub fn switched_off(&self) -> Vec<&'static str> {
        [
            (self.keep_out, "keep-out zones"),
            (self.bed, "bed size"),
            (self.z_travel, "Z travel"),
            (self.spoilboard, "spoilboard depth"),
        ]
        .into_iter()
        .filter(|(on, _)| !on)
        .map(|(_, name)| name)
        .collect()
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
    /// The Bit Library's copy of each bit the toolpaths use, by name, with the cutting
    /// data for the active material: a maker's catalogue copy where there is one. A bit
    /// not in it is not in the Bit Library. `None` when the bits are not compared with
    /// the Bit Library at all.
    pub library: Option<std::collections::BTreeMap<String, Tool>>,
    /// How the machine holds its bits, for the collet and holder checks.
    pub holding: crate::collision::Holding,
    /// The machine's rotary (4th axis), when it has one its firmware can drive.
    pub rotary: Option<crate::rotary_axis::RotaryAxis>,
    /// Which checks refuse as a risk of damage to the machine.
    pub protect: Protect,
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
        keep_out(doc, toolpaths, limits, &mut found);
        tool_numbers(toolpaths, &mut found);
        rotary_setup(doc, toolpaths, limits, &mut found);
        support_last(doc, toolpaths, &mut found);
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
    // nothing written for it (left out of the files), so nothing harmful: said as a warning
    if toolpath.moves.is_empty() {
        found.push(Finding::warning(
            Check::CanCut,
            format!(
                "{name} has no moves, so it is left out of the files: recalculate it, or \
             untick it in the toolpath list"
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
    // what it could not cut, as it said when it was made: easily missed once its form closed
    for note in toolpath.made_notes() {
        found.push(Finding::warning(
            Check::MissedSpots,
            format!("{name}: {note}"),
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
    spoilboard(doc, toolpath, limits, found);
    found.extend(
        problems_by_check(doc, toolpath, limits.left.unwrap_or(LEFT))
            .into_iter()
            .map(|(check, problem)| Finding::warning(check, problem)),
    );
    library(toolpath, limits, doc.job.units, found);
}

/// How far below the material `toolpath` cuts, into the spoilboard: refused deeper than the
/// machine allows (unless switched off), a warning past 1 mm, and a reminder when it
/// flattens the spoilboard itself.
fn spoilboard(doc: &Document, toolpath: &Toolpath, limits: &Limits, found: &mut Vec<Finding>) {
    let name = &toolpath.name;
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
    } else if doc.job.thickness > 0.0 {
        let protect = limits.protect;
        let units = doc.job.units;
        if protect.spoilboard && below > protect.spoilboard_depth + DEPTH_SLACK {
            found.push(Finding::damage(
                Check::Spoilboard,
                format!(
                    "{name} cuts {} below the material, deeper into the spoilboard than the {} \
                     {} allows, and could cut into the bed. Check the material thickness in \
                     Job Setup and the toolpath's cut depth",
                    units.length(below, 1),
                    units.length(protect.spoilboard_depth, 1),
                    machine_name(limits),
                ),
            ));
        } else if protect.warn_spoilboard && below > SPOILBOARD {
            found.push(Finding::warning(
                Check::Spoilboard,
                format!(
                    "{name} cuts {} below the material, into the spoilboard. Check the material \
                 thickness in Job Setup and the toolpath's cut depth",
                    units.length(below, 1)
                ),
            ));
        }
    }
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
             in the Bit Library, pick a tool again with the toolpath's Select…",
                tool.name, limits.name
            ),
        ));
    }
}

/// Whether each bit is from a maker's catalogue, and its feeds and speeds near the maker's
/// figures, as the Bit Library has them ([`Limits::library`]).
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
                "{name}: {} is not in the Bit Library, so its feeds and speeds cannot be \
                 compared with a maker's figures. Pick the bit with the toolpath's Select…, \
                 or add it to the Bit Library",
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
                 maker's catalogue in the Bit Library",
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

/// Whether the toolpaths keep out of the keep-out zones, the job's and the machine's: a
/// risk of damage to the machine unless its keep-out check is switched off. In a tiled job
/// the machine's zones, fixed to the bed, are checked against each tile's moves where its
/// file has them.
fn keep_out(doc: &Document, toolpaths: &[&Toolpath], limits: &Limits, found: &mut Vec<Finding>) {
    if !limits.protect.keep_out {
        return;
    }
    let per_tile = doc
        .tiling
        .map(|tiling| tiles_as_written(doc, toolpaths, tiling));
    for (i, toolpath) in toolpaths.iter().enumerate() {
        let radius = toolpath.tool.diameter / 2.0;
        let mut names: Vec<String> = vec![];
        let mut add = |zones: Vec<&crate::keep_out::KeepOutZone>| {
            for zone in zones {
                let name = format!("\"{}\"", zone.name);
                if !names.contains(&name) {
                    names.push(name);
                }
            }
        };
        match &per_tile {
            None => add(crate::keep_out::entered(
                doc.named_keep_out(),
                &toolpath.moves,
                radius,
            )),
            Some(written) => {
                add(crate::keep_out::entered(
                    doc.keep_out.iter(),
                    &toolpath.moves,
                    radius,
                ));
                for tile in written {
                    if let Some(moves) = tile.get(i) {
                        add(crate::keep_out::entered(
                            doc.machine_keep_out.iter(),
                            moves,
                            radius,
                        ));
                    }
                }
            }
        }
        if names.is_empty() {
            continue;
        }
        found.push(Finding::damage(
            Check::KeepOut,
            format!(
                "{} would cut into the keep-out zone{} {}, where the bit could hit a clamp \
                 or the machine. Move the part or the zone, then recalculate the toolpath",
                toolpath.name,
                if names.len() == 1 { "" } else { "s" },
                names.join(" and ")
            ),
        ));
    }
}

/// Each tile's moves of each of `toolpaths` (in their order), moved as the tile's file
/// writes them.
fn tiles_as_written(
    doc: &Document,
    toolpaths: &[&Toolpath],
    tiling: crate::tiling::Tiling,
) -> Vec<Vec<Vec<crate::toolpath::Move>>> {
    let saved = Document {
        job: doc.job,
        toolpaths: (1..)
            .map(crate::document::EntityId)
            .zip(toolpaths.iter().map(|&t| t.clone()))
            .collect(),
        ..Document::default()
    };
    (tiling.tiles(doc.job.area()).into_iter())
        .map(|tile| {
            let part = tiling.tiled(&saved, tile);
            (1..=toolpaths.len())
                .map(|n| {
                    part.toolpaths
                        .get(&crate::document::EntityId(n as u64))
                        .map_or_default(|t| t.moves.clone())
                })
                .collect()
        })
        .collect()
}

/// The machine's name for the messages, or "the machine" without one.
fn machine_name(limits: &Limits) -> String {
    if limits.name.is_empty() {
        "the machine".to_owned()
    } else {
        limits.name.clone()
    }
}

/// Whether two different bits share a bit number other than 0 (changed by hand).
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
                "Bit number {number} is given to both {} and {last}: a tool changer picks \
                 bits by number, so one of them would cut with the wrong bit. Give each bit \
                 its own number in the Bit Library (0 means the bit is changed by hand, \
                 and any number of bits may share 0)",
                rest.join(", ")
            ),
        ));
    }
}

/// A rotary job on a machine whose rotary is not set up for it: its centre line not found
/// yet, or lying the other way from the job's cylinder.
/// Past this many turns one way, a controller keeping A as a single-precision number places
/// it no finer than about a hundredth of a degree, and it gets coarser from there.
pub const MOST_FREE_TURNS: f64 = 500.0;

/// On a rotary job, a toolpath cutting after one that cuts away the centre stock (Cut Away
/// the Support): by then the part is held at its joined end alone, or not at all.
fn support_last(doc: &Document, toolpaths: &[&Toolpath], found: &mut Vec<Finding>) {
    if doc.job.rotary.is_none() {
        return;
    }
    let away = |t: &Toolpath| t.params["cut_support"].as_bool() == Some(true);
    let Some(first) = toolpaths.iter().position(|t| away(t)) else {
        return;
    };
    let later: Vec<&str> = toolpaths[first + 1..]
        .iter()
        .filter(|t| !away(t))
        .map(|t| t.name.as_str())
        .collect();
    if !later.is_empty() {
        found.push(Finding::warning(
            Check::RotarySetup,
            format!(
                "{} has Cut Away the Support ticked, but {} cut after it, when the part is \
                 held less firmly or not at all: move it to the end of the toolpath list",
                toolpaths[first].name,
                later.join(", ")
            ),
        ));
    }
}

fn rotary_setup(
    doc: &Document,
    toolpaths: &[&Toolpath],
    limits: &Limits,
    found: &mut Vec<Finding>,
) {
    let (Some(job), Some(axis)) = (doc.job.rotary, limits.rotary) else {
        return;
    };
    // turning freely, A carries on round from each pass to the next: about a turn for each
    // turn's worth of cutting round the part
    let round = |p: DVec3| if job.along == Along::X { p.y } else { p.x };
    let circumference = std::f64::consts::PI * job.diameter;
    let turns: f64 = toolpaths
        .iter()
        .flat_map(|t| t.moves.windows(2))
        .filter(|m| m[1].kind != MoveKind::Rapid)
        .map(|m| (round(m[1].to) - round(m[0].to)).abs())
        .sum::<f64>()
        / circumference;
    if axis.turning == crate::rotary_axis::Turning::Free && turns > MOST_FREE_TURNS {
        found.push(Finding::warning(
            Check::RotarySetup,
            format!(
                "A turns freely and these toolpaths wind it about {turns:.0} turns one way. Past \
                 {MOST_FREE_TURNS:.0} turns many controllers place A less finely, and detail can \
                 be lost: in Machine Configuration's Rotary, set A can turn to Up to a few \
                 turns each way"
            ),
        ));
    }
    if let Some(limit) = axis.limit() {
        let limit = limit / 360.0 * circumference;
        for t in toolpaths {
            if let Some(span) = too_far(t, &round, limit, circumference) {
                // a cable, hose or limit switch on the rotary: asked every time, like a crash
                found.push(Finding::must_ask(
                    Check::RotarySetup,
                    format!(
                        "\"{}\" turns A {:.0}° in one go without lifting, further than A can \
                         turn allows ({:.0}° each way): the file turns A past that limit. In \
                         Machine Configuration's Rotary, allow more turns or let A turn freely",
                        t.name,
                        span / circumference * 360.0,
                        limit / circumference * 360.0,
                    ),
                ));
            }
        }
    }
    let marked = toolpaths.iter().any(|t| {
        t.params
            .get("registration_mark")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false)
    });
    if !marked
        && toolpaths
            .iter()
            .any(|t| t.strategy.starts_with("rotary_") && t.strategy != "rotary_lathe_toolpath")
    {
        found.push(Finding::warning(
            Check::RotarySetup,
            "No toolpath cuts a registration mark at A=0. Tick Registration mark in Rotary \
             Finishing, or pencil a line on the material at A=0, so the part can be put back \
             exactly as it was if the job has to be started again"
                .to_owned(),
        ));
    }
    let off_axis = toolpaths
        .iter()
        .find(|t| t.across.iter().any(|a| a.abs() > 1e-9));
    if !axis.centre_set {
        let why = match off_axis {
            Some(t) => format!(
                "\"{}\" holds the bit to one side of the turning axis, measured from the \
                 axis: until it is found, the bit is held in the wrong place and cuts into the \
                 part",
                t.name
            ),
            None => "If the bit is not exactly over the turning axis, a round carving comes out \
                     lumpy"
                .to_owned(),
        };
        found.push(Finding::warning(
            Check::RotarySetup,
            format!(
                "The rotary's centre line is not set in Machine Configuration. {why}: find the \
                 centre line (see the help), then tick Centre line found"
            ),
        ));
    }
    room(&job, doc.job.turning_gap(), &axis, toolpaths, found);
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

/// How far round the first pass of `t` (its cuts between lifts) goes that no whole turn
/// keeps within `limit` either way of A=0, all flat round the material: `None` when every
/// pass fits.
fn too_far(
    t: &Toolpath,
    round: &impl Fn(DVec3) -> f64,
    limit: f64,
    circumference: f64,
) -> Option<f64> {
    let fits = |low: f64, high: f64| {
        // the fewest whole turns that bring its low end within the limit
        let turns = ((-limit - low) / circumference - 1e-9).ceil();
        high + turns * circumference <= limit + 1e-6
    };
    let mut pass: Option<(f64, f64)> = None;
    for m in t.moves.windows(2) {
        if m[1].kind == MoveKind::Rapid {
            if let Some((low, high)) = pass.take().filter(|&(l, h)| !fits(l, h)) {
                return Some(high - low);
            }
            continue;
        }
        let (a, b) = (round(m[0].to), round(m[1].to));
        let (low, high) = pass.unwrap_or((a, a));
        pass = Some((low.min(a).min(b), high.max(a).max(b)));
    }
    pass.filter(|&(l, h)| !fits(l, h)).map(|(l, h)| h - l)
}

/// Whether the material has room to turn on the rotary: below it down to the spoilboard
/// (or the bed with the spoilboard out), above it up to the bit lifted all the way, and
/// whether the bit has room to lift clear of its corners before A turns.
fn room(
    job: &crate::document::Rotary,
    turning_gap: f64,
    axis: &crate::rotary_axis::RotaryAxis,
    toolpaths: &[&Toolpath],
    found: &mut Vec<Finding>,
) {
    use crate::document::Blank;
    let swing = job.swing();
    let radius = job.diameter / 2.0;
    let shape = match job.blank {
        Blank::Round => "round material's sides",
        Blank::Square => "square material's corners",
        Blank::Board { .. } => "board's edges",
    };
    match axis.room_below() {
        Some(below) if swing > below + 1e-9 => {
            let fix = if !axis.spoilboard_off && swing <= below + axis.spoilboard.max(0.0) {
                "take the spoilboard out from under the rotary (and tick Spoilboard taken \
                 out in Machine Configuration's Rotary)"
            } else {
                "use smaller material, or raise the rotary"
            };
            found.push(Finding::warning(
                Check::RotarySetup,
                format!(
                    "The {shape} swing {swing:.1} mm out from the turning axis, but the axis is \
                     only {below:.1} mm above what is under it: the material hits it as it \
                     turns. {}",
                    capitalised(fix)
                ),
            ));
        }
        None if job.blank != Blank::Round => found.push(Finding::warning(
            Check::RotarySetup,
            format!(
                "The {shape} swing {swing:.1} mm out from the turning axis as A turns. Measure \
                 the axis height in Machine Configuration's Rotary so it can be checked that \
                 they clear the spoilboard"
            ),
        )),
        _ => {}
    }
    if let Some(above) = axis.room_above() {
        if swing + 1e-9 >= above {
            found.push(Finding::warning(
                Check::RotarySetup,
                format!(
                    "The {shape} swing {swing:.1} mm out from the turning axis, but with Z all \
                     the way up the bit is only {above:.1} mm above it: A can't turn the \
                     material under the bit. Use smaller material, or a shorter bit"
                ),
            ));
        }
        let highest = toolpaths
            .iter()
            .flat_map(|t| t.moves.iter().map(move |m| (t, radius + m.to.z)))
            .max_by(|a, b| a.1.total_cmp(&b.1));
        if let Some((t, high)) = highest
            && high > above + 1e-9
        {
            found.push(Finding::warning(
                Check::RotarySetup,
                format!(
                    "\"{}\" lifts the bit {high:.1} mm above the turning axis, higher than Z \
                     can go ({above:.1} mm): lower the clearance (Z1) in Job Setup",
                    t.name
                ),
            ));
        }
    }
    // A turning with the bit lower than the material swings: the file lifts it clear first,
    // which needs that much room under Z all the way up
    let round = |p: DVec3| if job.along == Along::X { p.y } else { p.x };
    let lift = radius + turning_gap;
    let Some(above) = axis.room_above() else {
        return;
    };
    if lift <= above + 1e-9 {
        return;
    }
    let low = turned_low(toolpaths, radius, lift, round);
    for (t, low) in toolpaths.iter().zip(low) {
        if low {
            found.push(Finding::warning(
                Check::RotarySetup,
                format!(
                    "\"{}\" turns A, and the file lifts the bit {lift:.1} mm above the turning \
                     axis first so the {shape} swing under it, higher than Z can go \
                     ({above:.1} mm). Use smaller material, or lower the clearance (Z1) in Job \
                     Setup",
                    t.name,
                ),
            ));
        }
    }
}

/// For each of `toolpaths` in order, whether a rapid of it turns A with the bit lower than
/// `lift` above the axis, as the post writes it: A (as `round`) and Z once a bit held off the
/// axis is turned to where its tip cuts, carried from one toolpath to the next, as the post's
/// first move of each starts where the last one ended.
fn turned_low(
    toolpaths: &[&Toolpath],
    radius: f64,
    lift: f64,
    round: impl Fn(DVec3) -> f64,
) -> Vec<bool> {
    let mut was: Option<(f64, f64)> = None;
    toolpaths
        .iter()
        .map(|t| {
            let mut low = false;
            for (i, m) in t.moves.iter().enumerate() {
                let now = match t.across.get(i) {
                    Some(&across) => {
                        crate::rotary_axis::held_at(radius, round(m.to), m.to.z, across)
                    }
                    None => (round(m.to), m.to.z),
                };
                if let Some(before) = was
                    && m.kind == MoveKind::Rapid
                    && (now.0 - before.0).abs() > 1e-6
                    && radius + before.1.min(now.1) < lift - 1e-6
                {
                    low = true;
                }
                was = Some(now);
            }
            low
        })
        .collect()
}

fn capitalised(text: &str) -> String {
    let mut chars = text.chars();
    chars.next().map_or_else(String::new, |first| {
        first.to_uppercase().chain(chars).collect()
    })
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
    let machine = machine_name(limits);
    let config = |what: &str| format!("{machine} has no {what} in Machine Configuration");
    let off = format!("Switched off for {machine} in Machine Configuration");
    let protect = limits.protect;
    let rotary = doc.job.rotary.is_some();
    let mut skipped = vec![];
    if !protect.bed {
        skipped.push((Check::Bed, off.clone()));
    } else if rotary {
        skipped.push((
            Check::Bed,
            "A rotary job wraps round the material, not along the bed".to_owned(),
        ));
    } else if limits.bed[0] <= 0.0 || limits.bed[1] <= 0.0 {
        skipped.push((Check::Bed, config("bed size")));
    }
    if !protect.z_travel {
        skipped.push((Check::ZTravel, off.clone()));
    } else if limits.z_travel <= 0.0 {
        skipped.push((Check::ZTravel, config("Z travel")));
    }
    if !protect.spoilboard {
        skipped.push((Check::Spoilboard, off.clone()));
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
    if !protect.keep_out {
        skipped.push((Check::KeepOut, off));
    } else if doc.named_keep_out().next().is_none() {
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
                "The bits were not compared with the Bit Library".to_owned(),
            ));
        }
    }
    if toolpaths.iter().all(|t| t.tool.number == 0) {
        skipped.push((
            Check::ToolNumbers,
            "Every bit is changed by hand (bit number 0)".to_owned(),
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
    (tiles_as_written(doc, toolpaths, tiling).into_iter())
        .filter_map(|tile| {
            let (lo, hi) = bounds(tile.iter().flatten())?;
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
    let protect = limits.protect;
    if protect.bed && !wraps && width > 0.0 && height > 0.0 && (span.x > width || span.y > height) {
        found.push(Finding::damage(
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
    if protect.z_travel && limits.z_travel > 0.0 && span.z > limits.z_travel {
        found.push(Finding::damage(
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
mod tests;
