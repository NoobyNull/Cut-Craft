//! Rotary Lathe: on a rotary job, A spins the material steadily while the bit stays straight
//! over the turning axis and steps down, as on a lathe: to face an end, to turn the part round
//! to its outline, or to part it off.
//!
//! - Face: everything between Start and End taken down to a small holding stub on the axis.
//! - Turn: everything between Start and End taken down to the model's outline (as far out
//!   as any of the model reaches at each point along), and to the stub where there is no
//!   model.
//! - Part off: a groove one bit wide at Start, down to the stub, so the part can be snapped
//!   or sawn free.
//!
//! It knows where the material is: it never cuts air, keeps out of where the chuck and
//! tailstock hold the material, and keeps the collet nut and spindle above the material left
//! beside the cut. Where they would not let the bit down as far as it should go it stops
//! short, and says so in red.

use cut_craft_core::component::{Combine, model_shape_at_resolution, placed_model};
use cut_craft_core::document::{Along, Document, Units};
use cut_craft_core::geometry::Point;
use cut_craft_core::glam::DVec3;
use cut_craft_core::progress;
use cut_craft_core::rotary_axis::Surroundings;
use cut_craft_core::toolpath::{Move, Tool, follow, made_by};
use cut_craft_core::{Change, Command, CommandError, Meta, ensure_positive};
use cut_craft_rotary::{Cutter, Head, Stock};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct RotaryLatheToolpath;

/// What the lathe does.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Operation {
    /// Takes everything between Start and End down to the holding stub: to square off an
    /// end of the material, or clear room beside the part
    #[default]
    Face,
    /// Takes everything between Start and End down to the model's outline, round: the
    /// quickest way to rough a round part, before the carving
    Turn,
    /// Cuts a groove one bit wide at Start down to the holding stub, so the finished part
    /// can be snapped or sawn free
    PartOff,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct Params {
    /// What the toolpath is called in the toolpath list
    pub name: String,
    /// The bit: a square end mill leaves a flat face. Its stepover spaces the turns along
    /// the part and its pass depth the steps down
    pub tool: Tool,
    /// What the lathe does
    #[schemars(extend("x-group" = "Lathe"))]
    pub operation: Operation,
    /// Where along the axis it starts, from the job's start (the chuck end); for Part off,
    /// where the groove is
    #[schemars(extend("x-unit" = "mm", "x-group" = "Along the part"), range(min = 0))]
    pub start: f64,
    /// Where along the axis it ends, from the job's start; 0 runs to the job's end
    #[schemars(extend("x-unit" = "mm", "x-group" = "Along the part", "x-hide-when" = {"operation": "part_off"}), range(min = 0))]
    pub end: f64,
    /// Across the stub left on the turning axis, joining the part to the rest of the
    /// material until it is cut free (for Turn, where there is no model)
    #[schemars(extend("x-unit" = "mm", "x-group" = "Lathe"), range(min = 0))]
    pub holding_stub: f64,
    /// Material left on the model's outline for the carving after
    #[schemars(extend("x-unit" = "mm", "x-group" = "Lathe", "x-show-when" = {"operation": "turn"}), range(min = 0))]
    pub allowance: f64,
    /// The holder above the bit, and the chuck and tailstock: filled in from Machine
    /// Configuration as the toolpath is made (the usual ones without)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(skip)]
    pub surroundings: Option<Surroundings>,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            name: "Rotary Lathe".into(),
            tool: Tool::starter(),
            operation: Operation::Face,
            start: 0.0,
            end: 0.0,
            holding_stub: 8.0,
            allowance: 0.5,
            surroundings: None,
        }
    }
}

impl Command for RotaryLatheToolpath {
    const ID: &'static str = "rotary_lathe_toolpath";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Rotary Lathe",
            group: "3D Toolpaths",
            icon: "toolpaths/rotary_lathe_toolpath.png",
            shortcut: "",
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        let (toolpath, notes) = turned(doc, p)?;
        let mut c = Change::builder(doc);
        c.add_toolpath(toolpath);
        for note in notes {
            c.note(note);
        }
        Ok(c.finish())
    }
}

/// How far into the material the bit must go for a point to count as cutting, mm.
const GRAZE: f64 = 0.05;

/// The radius the material is to come down to at each point along, on a grid `cell` apart
/// from `from`.
struct Profile {
    from: f64,
    cell: f64,
    radii: Vec<f64>,
}

impl Profile {
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "clamped to the grid first"
    )]
    fn index(&self, along: f64) -> usize {
        (((along - self.from) / self.cell).floor().max(0.0) as usize).min(self.radii.len() - 1)
    }

    fn at(&self, along: f64) -> f64 {
        self.radii[self.index(along)]
    }

    /// The highest within `half` either side of `along`: what a bit that wide must stay
    /// above.
    fn under(&self, along: f64, half: f64) -> f64 {
        let (a, b) = (self.index(along - half), self.index(along + half));
        self.radii[a..=b]
            .iter()
            .copied()
            .fold(f64::NEG_INFINITY, f64::max)
    }

    /// Brought down to `r` within `half` either side of `along`.
    fn lower(&mut self, along: f64, half: f64, r: f64) {
        let (a, b) = (self.index(along - half), self.index(along + half));
        for x in &mut self.radii[a..=b] {
            *x = x.min(r);
        }
    }
}

/// The toolpath, and what it has to say in red.
#[expect(
    clippy::too_many_lines,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "the lathe's passes worked out in one place; counts of cells and turns are small"
)]
fn turned(
    doc: &Document,
    p: &Params,
) -> Result<(cut_craft_core::toolpath::Toolpath, Vec<String>), CommandError> {
    let Some(rotary) = doc.job.rotary else {
        return Err(CommandError::Failed(
            "the lathe spins the material: set the job up as rotary in Job Setup".into(),
        ));
    };
    let tool = &p.tool;
    ensure_positive("tool diameter", tool.diameter)?;
    let stepover = tool.stepover;
    if !(stepover > 0.0 && stepover <= 1.0) {
        return Err(CommandError::InvalidParams(
            "the tool's stepover must be more than 0% and at most 100%: set it in the Tool Database".into(),
        ));
    }
    let along_x = rotary.along == Along::X;
    let (length, circumference) = if along_x {
        (doc.job.width, doc.job.height)
    } else {
        (doc.job.height, doc.job.width)
    };
    let radius = rotary.diameter / 2.0;
    let stock = Stock::of(&doc.job).expect("a rotary job has its material");
    let swing = stock.swing();
    let around = p.surroundings.unwrap_or_default();
    let head = Head::new(tool, &around, length, swing);
    let units = doc.job.units;
    let reach = tool.reach().max(1e-6);
    let mut notes = vec![];

    // where it cuts along the part: kept out of the chuck and tailstock
    let (start, end) = if p.operation == Operation::PartOff {
        let at = p.start.clamp(0.0, length);
        (at - reach, at + reach)
    } else {
        let end = if p.end > 0.0 {
            p.end.min(length)
        } else {
            length
        };
        (p.start.clamp(0.0, length), end)
    };
    if end <= start {
        notes.push(format!(
            "End ({}) must be further along than Start ({}), so there is nothing to cut.",
            units.length(end, 1),
            units.length(start, 1),
        ));
        return Ok((made(p, tool, vec![]), notes));
    }
    let (free_from, free_to) = head.free();
    let (from, to) = (start.max(free_from), end.min(free_to));
    if from > start + 1e-9 || to < end - 1e-9 {
        notes.push(format!(
            "The chuck and tailstock hold the material up to {} and from {} along it \
             (Machine Configuration's rotary settings): the lathe cuts only between.",
            units.length(free_from.max(0.0), 1),
            units.length(free_to.min(length), 1),
        ));
    }
    if to < from + 1e-9 {
        return Ok((made(p, tool, vec![]), notes));
    }

    // where the material is to come down to
    let cell = (tool.diameter / 8.0).clamp(0.1, 0.5);
    let stub = (p.holding_stub / 2.0).max(0.0);
    let span = |r: f64| Profile {
        from: 0.0,
        cell,
        radii: vec![r; ((length / cell).ceil() as usize).max(1) + 1],
    };
    let mut target = span(stub);
    if p.operation == Operation::Turn {
        if doc.components.is_empty() {
            notes.push(
                "There is no model to turn down to, so nothing is cut: place a model in the \
                 job first, or use Face to take the material down to the holding stub."
                    .into(),
            );
            return Ok((made(p, tool, vec![]), notes));
        }
        progress::stage("Placing the model");
        let origin = doc.job.area().min;
        // full 3D models as they really are in space; the rest from their heights, placed
        // in the material where Material Setup puts them
        let real = |k: &cut_craft_core::component::Component| {
            !k.solid.is_empty() && matches!(k.combine, Combine::Add | Combine::Merge)
        };
        let solids: Vec<[DVec3; 3]> = doc
            .components
            .values()
            .filter(|k| k.visible && real(k))
            .flat_map(cut_craft_core::component::Component::solid_triangles)
            .collect();
        let rest = Document {
            components: doc
                .components
                .iter()
                .filter(|(_, k)| !real(k))
                .map(|(id, k)| (*id, k.clone()))
                .collect(),
            ..doc.clone()
        };
        let count = target.radii.len();
        let mut reach = solid_outline(
            &solids,
            if along_x { origin.x } else { origin.y },
            count,
            cell,
        );
        if !rest.components.is_empty() {
            let model = placed_model(&rest, tool)?;
            // where there is any model at all: elsewhere it comes down to the stub
            let shape = model_shape_at_resolution(&rest);
            // as far out as any of the model reaches at each point along
            let heights = outline(
                &model,
                &shape,
                radius,
                along_x,
                (length, circumference),
                cell,
                origin,
            );
            for (r, h) in reach.iter_mut().zip(heights) {
                *r = r.max(h);
            }
        }
        for (t, r) in target.radii.iter_mut().zip(reach) {
            if r.is_finite() {
                *t = (r + p.allowance.max(0.0)).max(stub);
            }
        }
    }
    // the material left, as far out as it reaches at each point along
    let mut left = span(swing);

    // the turns along the part at each step down, the bit's edge kept within where it cuts:
    // its stepover apart
    let half = reach;
    let step = stepover * tool.diameter;
    let (from, to) = (from + reach, to - reach);
    if to < from - 1e-9 {
        notes.push(format!(
            "Between Start and End there is less room than the bit is wide ({}), so nothing \
             is cut: move them further apart, or use a narrower bit.",
            units.length(tool.diameter, 1),
        ));
        return Ok((made(p, tool, vec![]), notes));
    }
    let places: Vec<f64> = if to - from <= 1e-9 {
        vec![f64::midpoint(from, to)]
    } else {
        let n = ((to - from) / step).ceil().max(1.0) as usize;
        (0..=n)
            .map(|k| from + (to - from) * k as f64 / n as f64)
            .collect()
    };
    let depth = if tool.pass_depth > 0.0 {
        tool.pass_depth
    } else {
        tool.diameter / 2.0
    };
    let lowest_target = places
        .iter()
        .map(|&a| target.under(a, half))
        .fold(f64::INFINITY, f64::min);
    let cutter = Cutter::new(tool);
    let round_cell = cell.max(circumference / 720.0);
    let points = ((circumference / round_cell).ceil() as usize).max(8);
    let mut passes: Vec<Vec<DVec3>> = vec![];
    let mut short: Vec<(f64, f64)> = vec![];
    let mut level = swing;
    progress::stage("Turning");
    while level > lowest_target + 1e-9 {
        progress::stop_if_cancelled()?;
        level = (level - depth).max(lowest_target);
        let mut line: Vec<f64> = places.clone();
        if passes.len() % 2 == 1 {
            line.reverse();
        }
        for along in line {
            // down to this level, or the target, whichever is higher; no lower than the
            // holder lets it beside what is left
            let goal = level.max(target.under(along, half));
            if goal >= left.under(along, half) - GRAZE {
                continue;
            }
            let room = head.lowest_over(along, |a| left.at(a), cell);
            let r = goal.max(room);
            if r > goal + 1e-6 {
                short.push((along, r - goal));
            }
            if r >= left.under(along, half) - GRAZE {
                continue;
            }
            // round the part once, cutting only where the material reaches past the bit
            let ring: Vec<(DVec3, bool)> = (0..=points)
                .map(|j| {
                    let round = j as f64 * circumference / points as f64;
                    let turn = round / radius;
                    let cuts = stock
                        .drop(&cutter, along, 0.0, turn)
                        .is_some_and(|top| r < top - GRAZE);
                    let p = if along_x {
                        DVec3::new(along, round, r - radius)
                    } else {
                        DVec3::new(round, along, r - radius)
                    };
                    (p, cuts)
                })
                .collect();
            passes.extend(stretches(&ring, reach));
            left.lower(along, half, r);
        }
    }
    if !short.is_empty() {
        let (a, b, most) = short.iter().fold(
            (f64::INFINITY, f64::NEG_INFINITY, 0.0f64),
            |(a, b, m), &(at, by)| (a.min(at), b.max(at), m.max(by)),
        );
        notes.push(short_note(&around, units, (a, b), most));
    }
    let safe = doc.job.turning_gap();
    // worked out from the material's corner; written where the material is in the job, as
    // the job's datum puts it
    let corner = doc.job.area().min;
    let moves: Vec<Move> = passes
        .iter()
        .flat_map(|pass| follow(pass, safe))
        .map(|mut m| {
            m.to.x += corner.x;
            m.to.y += corner.y;
            m
        })
        .collect();
    Ok((made(p, tool, moves), notes))
}

/// The toolpath made of `moves`.
fn made(p: &Params, tool: &Tool, moves: Vec<Move>) -> cut_craft_core::toolpath::Toolpath {
    made_by::<RotaryLatheToolpath>(p, p.name.clone(), tool.clone(), vec![], moves)
}

/// In red: the holder stopped the bit short of where it should go.
fn short_note(around: &Surroundings, units: Units, (from, to): (f64, f64), most: f64) -> String {
    format!(
        "The collet nut ({} across) and spindle can't get down beside the material left, or \
         the chuck or tailstock beside the cut: between {} and {} along the part the bit stops \
         up to {} short. Face or turn the material beside it down first, keep Start and End \
         further from the chuck and tailstock, or use a bit that sticks out further.",
        units.length(around.holder.nut_diameter, 0),
        units.length(from, 1),
        units.length(to, 1),
        units.length(most, 1),
    )
}

/// The stretches of a `ring` (each point with whether the bit cuts there) that cut, each
/// with a point of air either end; gaps shorter than two bit widths (`reach` each side)
/// are cut through rather than lifted over.
fn stretches(ring: &[(DVec3, bool)], reach: f64) -> Vec<Vec<DVec3>> {
    let mut keep: Vec<bool> = ring.iter().map(|r| r.1).collect();
    for (i, r) in ring.iter().enumerate() {
        if r.1 {
            if i > 0 {
                keep[i - 1] = true;
            }
            if i + 1 < keep.len() {
                keep[i + 1] = true;
            }
        }
    }
    let mut i = 0;
    while i < keep.len() {
        if keep[i] {
            i += 1;
            continue;
        }
        let gap = i;
        while i < keep.len() && !keep[i] {
            i += 1;
        }
        if gap > 0 && i < keep.len() {
            let far: f64 = ring[gap - 1..=i]
                .windows(2)
                .map(|w| w[0].0.distance(w[1].0))
                .sum();
            if far < 4.0 * reach {
                keep[gap..i].fill(true);
            }
        }
    }
    let mut out = vec![];
    let mut run: Vec<DVec3> = vec![];
    for (r, k) in ring.iter().zip(keep) {
        if k {
            run.push(r.0);
        } else if !run.is_empty() {
            out.push(std::mem::take(&mut run));
        }
    }
    if !run.is_empty() {
        out.push(run);
    }
    out
}

/// As far out from the axis as any of the `model` reaches at each point along the part
/// (`cell` apart), minus infinity where there is none (where the model's `shape` has
/// nothing): its heights are below the material's top, `radius` out, unrolled round the part,
/// along and round counted from the material's corner, `origin` in the job's X and Y.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "cell counts are small"
)]
fn outline(
    model: &cut_craft_core::simulation::Material,
    shape: &cut_craft_core::simulation::Material,
    radius: f64,
    along_x: bool,
    (length, circumference): (f64, f64),
    cell: f64,
    origin: Point,
) -> Vec<f64> {
    let count = ((length / cell).ceil() as usize).max(1) + 1;
    let round = ((circumference / model.cell.max(1e-3)).ceil() as usize).max(8);
    (0..count)
        .map(|i| {
            let along = (i as f64 + 0.5) * cell;
            (0..round)
                .filter_map(|j| {
                    let w = (j as f64 + 0.5) * circumference / round as f64;
                    let at = if along_x {
                        Point::new(origin.x + along, origin.y + w)
                    } else {
                        Point::new(origin.x + w, origin.y + along)
                    };
                    // only where some of the model is, not its zero plane round it
                    shape.at(at).filter(|h| h.is_finite())?;
                    model.at(at).filter(|h| h.is_finite())
                })
                .map(|h| radius + h)
                .fold(f64::NEG_INFINITY, f64::max)
        })
        .collect()
}

/// How far out from the axis full 3D models `triangles` (in their own frame: along the
/// job's axis from `from`, then across and up from the axis) reach at each of `count`
/// points `cell` apart along it, the middles of its cells: where the slice there cuts them,
/// and their corners within the cell. Minus infinity where none is.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "cell counts are small"
)]
fn solid_outline(triangles: &[[DVec3; 3]], from: f64, count: usize, cell: f64) -> Vec<f64> {
    let mut reach = vec![f64::NEG_INFINITY; count];
    let out = |v: DVec3| v.y.hypot(v.z);
    let index = |along: f64| ((along - from) / cell).floor();
    for t in triangles {
        // its corners, in their own cells
        for &v in t {
            let i = index(v.x);
            if i >= 0.0 && (i as usize) < count {
                let r = &mut reach[i as usize];
                *r = r.max(out(v));
            }
        }
        // its slices at the cells' middles: as far out as a line across it goes is at
        // one of its ends
        let (lo, hi) = t
            .iter()
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), v| {
                (lo.min(v.x), hi.max(v.x))
            });
        let first = ((lo - from) / cell - 0.5).ceil().max(0.0) as usize;
        let last = ((hi - from) / cell - 0.5).floor();
        if last < 0.0 {
            continue;
        }
        let upto = (last as usize).min(count.saturating_sub(1));
        for (i, r) in reach.iter_mut().enumerate().take(upto + 1).skip(first) {
            let at = from + (i as f64 + 0.5) * cell;
            for (a, b) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
                if (a.x - at) * (b.x - at) > 0.0 || (a.x - b.x).abs() < 1e-12 {
                    continue;
                }
                *r = r.max(out(a.lerp(b, (at - a.x) / (b.x - a.x))));
            }
        }
    }
    reach
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::component::Component;
    use cut_craft_core::document::{Blank, Job};
    use cut_craft_core::toolpath::MoveKind;

    fn round_job(length: f64, diameter: f64) -> Document {
        Document {
            job: Job::rotary(length, diameter, Along::X),
            ..Document::default()
        }
    }

    fn cuts_of(doc: &Document, p: &Params) -> (Vec<DVec3>, Vec<String>) {
        let (toolpath, notes) = turned(doc, p).unwrap();
        let cuts = toolpath
            .moves
            .iter()
            .filter(|m| m.kind == MoveKind::Cut)
            .map(|m| m.to)
            .collect();
        (cuts, notes)
    }

    #[test]
    fn facing_takes_the_end_down_to_the_stub_a_level_at_a_time_round_and_round() {
        let doc = round_job(100.0, 40.0);
        let p = Params {
            start: 80.0,
            ..Params::default()
        };
        let (cuts, notes) = cuts_of(&doc, &p);
        assert!(notes.is_empty(), "{notes:?}");
        // between 80 and 100 only, the bit's edge inside
        assert!(
            cuts.iter()
                .all(|c| c.x >= 83.0 - 1e-6 && c.x <= 97.0 + 1e-6)
        );
        // all the way round (the unrolled 40π) at each level
        let most = cuts.iter().map(|c| c.y).fold(0.0, f64::max);
        assert!(most > 40.0 * std::f64::consts::PI - 1.0, "{most}");
        // down to the 8 mm stub: 4 out from the axis, the top being 20 out
        let lowest = cuts.iter().map(|c| c.z).fold(f64::INFINITY, f64::min);
        assert!((lowest - (4.0 - 20.0)).abs() < 1e-6, "{lowest}");
        // in steps no deeper than the pass depth
        let mut levels: Vec<f64> = cuts.iter().map(|c| c.z).collect();
        levels.sort_by(f64::total_cmp);
        levels.dedup_by(|a, b| (*a - *b).abs() < 1e-6);
        for w in levels.windows(2) {
            assert!(w[1] - w[0] <= 3.0 + 1e-6, "{levels:?}");
        }
    }

    #[test]
    fn it_never_refuses_what_it_cannot_cut_is_said_in_red() {
        let doc = round_job(100.0, 40.0);
        // End before Start
        let (cuts, notes) = cuts_of(
            &doc,
            &Params {
                start: 60.0,
                end: 40.0,
                ..Params::default()
            },
        );
        assert!(cuts.is_empty(), "{cuts:?}");
        assert!(
            notes.iter().any(|n| n.contains("must be further along")),
            "{notes:?}"
        );
        // less room than the bit: nothing cut past Start or End
        let (cuts, notes) = cuts_of(
            &doc,
            &Params {
                start: 50.0,
                end: 52.0,
                ..Params::default()
            },
        );
        assert!(cuts.is_empty(), "{cuts:?}");
        assert!(
            notes.iter().any(|n| n.contains("less room than the bit")),
            "{notes:?}"
        );
        // Turn with no model
        let (cuts, notes) = cuts_of(
            &doc,
            &Params {
                operation: Operation::Turn,
                ..Params::default()
            },
        );
        assert!(cuts.is_empty(), "{cuts:?}");
        assert!(
            notes.iter().any(|n| n.contains("no model to turn down to")),
            "{notes:?}"
        );
    }

    #[test]
    fn on_a_board_it_cuts_only_where_the_board_reaches_out_to_the_bit() {
        // a board 20 thick, 60 wide: at a radius past 10, only towards its edges is there
        // anything to cut
        let mut doc = round_job(100.0, 20.0);
        let mut r = doc.job.rotary.unwrap();
        r.blank = Blank::Board { width: 60.0 };
        doc.job.rotary = Some(r);
        let p = Params {
            start: 80.0,
            ..Params::default()
        };
        let (toolpath, _) = turned(&doc, &p).unwrap();
        let stock = Stock::of(&doc.job).unwrap();
        let cutter = Cutter::new(&p.tool);
        let mut cutting = 0;
        for m in toolpath.moves.iter().filter(|m| m.kind == MoveKind::Cut) {
            let turn = m.to.y / 10.0;
            let top = stock
                .drop(&cutter, m.to.x, 0.0, turn)
                .unwrap_or(f64::NEG_INFINITY);
            // within a short gap of the board, never far off it
            if 10.0 + m.to.z < top - GRAZE {
                cutting += 1;
            }
        }
        let all = toolpath
            .moves
            .iter()
            .filter(|m| m.kind == MoveKind::Cut)
            .count();
        assert!(cutting * 10 > all * 6, "{cutting} of {all}");
    }

    #[test]
    fn parting_off_cuts_one_groove_down_to_the_stub_and_the_holder_needs_room_beside_it() {
        let doc = round_job(100.0, 40.0);
        // a bit sticking out 40 under a 35 mm nut: room to get down to the 4 mm stub beside
        // a bar 20 out
        let long = Tool {
            stickout: 40.0,
            ..Tool::starter()
        };
        let p = Params {
            tool: long,
            operation: Operation::PartOff,
            start: 50.0,
            ..Params::default()
        };
        let (cuts, notes) = cuts_of(&doc, &p);
        assert!(notes.is_empty(), "{notes:?}");
        assert!(cuts.iter().all(|c| (c.x - 47.0).abs() < 1e-6
            || (c.x - 53.0).abs() < 1e-6
            || (c.x - 50.0).abs() < 3.0 + 1e-6));
        let lowest = cuts.iter().map(|c| c.z).fold(f64::INFINITY, f64::min);
        assert!((lowest - (4.0 - 20.0)).abs() < 1e-6, "{lowest}");
        // sticking out only 10: the nut rests on the bar, 10 above the tip, so the bit stops
        // at 10 out from the axis, and says so
        let stubby = Params {
            tool: Tool {
                stickout: 10.0,
                ..Tool::starter()
            },
            ..p
        };
        let (cuts, notes) = cuts_of(&doc, &stubby);
        let lowest = cuts.iter().map(|c| c.z).fold(f64::INFINITY, f64::min);
        assert!(lowest >= (20.0 - 10.0) - 20.0 - 1e-6, "{lowest}");
        assert!(
            notes.iter().any(|n| n.contains("can't get down")),
            "{notes:?}"
        );
    }

    #[test]
    fn it_keeps_out_of_the_chuck_and_says_so() {
        let doc = round_job(100.0, 40.0);
        let p = Params {
            end: 30.0,
            surroundings: Some(Surroundings {
                chuck: 20.0,
                ..Surroundings::default()
            }),
            ..Params::default()
        };
        let (cuts, notes) = cuts_of(&doc, &p);
        assert!(cuts.iter().all(|c| c.x >= 23.0 - 1e-6), "into the chuck");
        assert!(notes.iter().any(|n| n.contains("chuck")), "{notes:?}");
    }

    #[test]
    fn a_job_zeroed_on_its_centre_turns_the_same_part_where_the_material_is() {
        use cut_craft_core::document::Datum;
        // a bead between 20 and 80 along a 100 bar, on a job zeroed at `datum`
        let on = |datum| {
            let mut doc = Document {
                job: Job {
                    datum,
                    ..Job::rotary(100.0, 40.0, Along::X)
                },
                ..Document::default()
            };
            let area = doc.job.area();
            let model = Component::from_fn("Bead", Combine::Add, area, 0.5, |p| {
                Some(if (20.0..80.0).contains(&(p.x - area.min.x)) {
                    12.0
                } else {
                    6.0
                })
            });
            let mut c = Change::builder(&doc);
            c.add_new_component(model);
            doc.apply(&c.finish()).unwrap();
            doc
        };
        let p = Params {
            operation: Operation::Turn,
            ..Params::default()
        };
        let corner = on(Datum::BottomLeft);
        let centre = on(Datum::Centre);
        let shift = centre.job.area().min;
        assert!(shift.x < 0.0, "zeroed on the middle");
        let (at_corner, _) = cuts_of(&corner, &p);
        let (at_centre, _) = cuts_of(&centre, &p);
        assert!(!at_corner.is_empty(), "nothing cut");
        assert_eq!(at_corner.len(), at_centre.len());
        for (a, b) in at_corner.iter().zip(&at_centre) {
            let moved = *a + DVec3::new(shift.x, shift.y, 0.0);
            assert!((moved - *b).length() < 1e-6, "{a:?} vs {b:?}");
        }
    }

    #[test]
    fn turning_takes_the_material_down_to_the_model_outline_and_no_further() {
        // a model 12 out from the axis between 20 and 80 (and right out to the bar's 20 in
        // the middle), 6 out elsewhere, in a 40 bar
        let mut doc = round_job(100.0, 40.0);
        let model = Component::from_fn("Bead", Combine::Add, doc.job.area(), 0.5, |p| {
            Some(if (45.0..55.0).contains(&p.x) {
                20.0
            } else if (20.0..80.0).contains(&p.x) {
                12.0
            } else {
                6.0
            })
        });
        let mut c = Change::builder(&doc);
        c.add_new_component(model);
        doc.apply(&c.finish()).unwrap();
        let p = Params {
            operation: Operation::Turn,
            allowance: 0.5,
            ..Params::default()
        };
        let (cuts, _) = cuts_of(&doc, &p);
        assert!(!cuts.is_empty(), "nothing cut");
        for c in &cuts {
            // the bit's edge 3 either side: over the bead it stays 12.5 out
            let r = 20.0 + c.z;
            if (17.0..83.0).contains(&c.x) {
                assert!(r >= 12.5 - 1e-6, "{c:?}");
            } else {
                assert!(r >= 6.5 - 1e-6, "{c:?}");
            }
        }
        let lowest = cuts
            .iter()
            .map(|c| 20.0 + c.z)
            .fold(f64::INFINITY, f64::min);
        assert!((lowest - 6.5).abs() < 0.3, "{lowest}");
    }

    #[test]
    fn a_full_model_is_turned_down_to_where_it_really_is() {
        // a 16 by 16 bar from 20 to 80 along the axis of a 40 bar, kept as its real shape:
        // its corners reach 11.3 out. Its unrolled heights (9) and Material Setup's flat-job
        // position do not move it
        let mut doc = round_job(100.0, 40.0);
        let (lo, hi) = (DVec3::new(20.0, -8.0, -8.0), DVec3::new(80.0, 8.0, 8.0));
        let at = |i: usize| {
            DVec3::new(
                if i & 1 == 1 { hi.x } else { lo.x },
                if i & 2 == 2 { hi.y } else { lo.y },
                if i & 4 == 4 { hi.z } else { lo.z },
            )
        };
        let triangles: Vec<[DVec3; 3]> = [
            [0, 2, 3, 1],
            [4, 5, 7, 6],
            [0, 1, 5, 4],
            [2, 6, 7, 3],
            [0, 4, 6, 2],
            [1, 3, 7, 5],
        ]
        .iter()
        .flat_map(|&[a, b, c, d]| [[at(a), at(b), at(c)], [at(a), at(c), at(d)]])
        .collect();
        let mut model = Component::from_fn("Bar", Combine::Add, doc.job.area(), 0.5, |p| {
            (20.0..80.0).contains(&p.x).then_some(9.0)
        });
        model.set_solid(&triangles);
        let mut c = Change::builder(&doc);
        c.add_new_component(model);
        doc.apply(&c.finish()).unwrap();
        let reach = 8.0_f64.hypot(8.0);
        for gap in [0.0, 3.0] {
            doc.job.model_position.gap = gap;
            let p = Params {
                operation: Operation::Turn,
                allowance: 0.0,
                ..Params::default()
            };
            let (cuts, _) = cuts_of(&doc, &p);
            let over: Vec<f64> = cuts
                .iter()
                .filter(|c| (25.0..75.0).contains(&c.x))
                .map(|c| 20.0 + c.z)
                .collect();
            assert!(!over.is_empty(), "the bar is turned down to");
            let lowest = over.iter().copied().fold(f64::INFINITY, f64::min);
            assert!(lowest >= reach - 1e-6, "gap {gap}: {lowest} into the bar");
            assert!(
                lowest - reach < 0.3,
                "gap {gap}: {lowest}, short of {reach}"
            );
        }
    }

    #[test]
    fn where_there_is_no_model_turning_comes_down_to_the_stub() {
        // a model 12 out between 30 and 70 only, in a 40 bar
        let mut doc = round_job(100.0, 40.0);
        let model = Component::from_fn("Bead", Combine::Add, doc.job.area(), 0.5, |p| {
            (30.0..70.0).contains(&p.x).then_some(12.0)
        });
        let mut c = Change::builder(&doc);
        c.add_new_component(model);
        doc.apply(&c.finish()).unwrap();
        let p = Params {
            operation: Operation::Turn,
            allowance: 0.0,
            ..Params::default()
        };
        let (cuts, _) = cuts_of(&doc, &p);
        for c in &cuts {
            let r = 20.0 + c.z;
            if (27.0..73.0).contains(&c.x) {
                assert!(r >= 12.0 - 1e-6, "{c:?}");
            }
        }
        // the 8 mm stub, 4 out, away from the model
        let lowest = cuts
            .iter()
            .filter(|c| c.x < 20.0)
            .map(|c| 20.0 + c.z)
            .fold(f64::INFINITY, f64::min);
        assert!((lowest - 4.0).abs() < 1e-6, "{lowest}");
    }
}
