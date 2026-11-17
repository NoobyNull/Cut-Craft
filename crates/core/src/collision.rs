//! Collet and holder collisions (help: Collet and Holder Collisions): only the bit's flutes
//! cut. Above them come the plain shank, then the collet nut, then the spindle (or router)
//! body, and on a deep cut any of them can run into material still standing beside the
//! cut: a wall left outside a pocket, uncut stock round a deep profile. The toolpaths are
//! cut in order on the same height grid as the preview, and after each move the parts above
//! the flutes are checked against the material left.
//!
//! The holder (collet nut and spindle) is the machine's, as Machine Configuration has it.
//! With Machine Configuration's Holder per bit ticked (a tool changer, its bits each in a
//! holder of their own) a bit can carry its own ([`Tool::holder`]). How far the bit sticks
//! out of the nut is the bit's ([`Tool::stickout`]). These checks are Cut Craft's own.

use crate::document::{Job, Units};
use crate::geometry::Point;
use crate::keep_out::KeepOutZone;
use crate::progress;
use crate::simplify::simplify;
use crate::simulation::Material;
use crate::toolpath::{MoveKind, Tool, ToolShape, Toolpath};
use glam::DVec3;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// What holds the bit: the collet nut and the spindle above it, mm.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct Holder {
    /// Across the collet nut (an ER20 nut is about 35 mm; a trim router's about 25 mm).
    pub nut_diameter: f64,
    /// From the nut's face up to the spindle body.
    pub nut_length: f64,
    /// Across the spindle (or router) body above the nut (65 mm or 80 mm for most spindles).
    pub spindle_diameter: f64,
}

impl Default for Holder {
    /// An ER20 collet nut under a 65 mm spindle: what most hobby machines have, and a little
    /// wider than a trim router's, so it errs on the side of saying.
    fn default() -> Self {
        Self {
            nut_diameter: 35.0,
            nut_length: 20.0,
            spindle_diameter: 65.0,
        }
    }
}

/// How the machine holds its bits: the holder, and whether a bit's own holder in the Tool
/// Database counts instead (Machine Configuration's Holder per bit).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Holding {
    pub holder: Holder,
    pub per_bit: bool,
}

/// The shank taken to show above the flutes (and neck) when a bit's stick-out is not given,
/// mm: a bit is usually pushed into the collet till a little of its shank is left showing.
pub const SHANK_SHOWING: f64 = 5.0;

/// The stick-out taken for a bit with neither a stick-out nor a flute length, mm (an inch).
pub const GUESSED_STICKOUT: f64 = 25.0;

/// How far into the material a part must seem to go to count as hitting it, mm: the grid
/// samples cell centres, so a part passing a wall it has just cut beside can seem to touch.
const TOUCHES: f64 = 0.5;

/// How much wider than the cutting part a shank must be to rub where the flutes did not
/// cut, mm.
const WIDER: f64 = 0.05;

/// Hits kept for each toolpath to show in the preview; past that only counted.
const KEPT: usize = 2000;

/// The simulation's cells across the job's longer side: about the smallest tool's radius,
/// within these bounds (as Save Toolpaths' checks size them).
const FEWEST_CELLS: f64 = 100.0;
const MOST_CELLS: f64 = 800.0;

/// Cutting moves cut at once across the cores while nothing above the flutes is low enough
/// to touch the material.
const BATCH: usize = 4096;

/// Which part above the flutes hits.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Part {
    /// The plain shank, where it is wider than the cutting part.
    Shank,
    ColletNut,
    Spindle,
}

impl Part {
    /// What the messages call it.
    pub fn name(self) -> &'static str {
        match self {
            Self::Shank => "shank",
            Self::ColletNut => "collet nut",
            Self::Spindle => "spindle",
        }
    }
}

/// A round part above the tip: from `rise` above the tip upward, `radius` from the axis.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Band {
    pub part: Part,
    pub rise: f64,
    pub radius: f64,
}

/// How far the bit sticks out below the collet nut, mm, and whether that was taken rather
/// than given: its [`Tool::stickout`]; else its flutes (or V) and neck and
/// [`SHANK_SHOWING`]; else [`GUESSED_STICKOUT`].
pub fn stickout(tool: &Tool) -> (f64, bool) {
    if tool.stickout.is_finite() && tool.stickout > 0.0 {
        return (tool.stickout, false);
    }
    let guess = tool.cutting_length().map_or(GUESSED_STICKOUT, |cuts| {
        cuts + tool.neck_length.max(0.0) + SHANK_SHOWING
    });
    (guess, true)
}

/// The holder `tool` is in: its own when it has one and the machine takes holders per bit,
/// else the machine's.
pub fn holder_of(tool: &Tool, machine: &Holding) -> Holder {
    match tool.holder {
        Some(own) if machine.per_bit => own,
        _ => machine.holder,
    }
}

/// The parts above `tool`'s flutes that can hit the material, lowest first: its shank when
/// wider than what cuts (and the flutes' length is known), then the holder's collet nut and
/// spindle.
pub fn bands(tool: &Tool, machine: &Holding) -> Vec<Band> {
    let (out, _) = stickout(tool);
    let holder = holder_of(tool, machine);
    let mut bands = vec![];
    // a tapered tool's taper already runs out to its shank; a V-bit's V is its whole width
    let shank = tool.shank_diameter / 2.0;
    if let Some(cuts) = tool.cutting_length() {
        let rise = cuts + tool.neck_length.max(0.0);
        if tool.shape != ToolShape::VBit && shank > tool.reach() + WIDER && rise < out {
            bands.push(Band {
                part: Part::Shank,
                rise,
                radius: shank,
            });
        }
    }
    if holder.nut_diameter > 0.0 {
        bands.push(Band {
            part: Part::ColletNut,
            rise: out,
            radius: holder.nut_diameter / 2.0,
        });
    }
    if holder.spindle_diameter > 0.0 {
        bands.push(Band {
            part: Part::Spindle,
            rise: out + holder.nut_length.max(0.0),
            radius: holder.spindle_diameter / 2.0,
        });
    }
    bands
}

/// One move where a part above the flutes hits the material.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hit {
    pub part: Part,
    /// The tip's move.
    pub from: DVec3,
    pub to: DVec3,
    /// The part's radius, and its bottom's height above the tip.
    pub radius: f64,
    pub rise: f64,
    /// How far into the material it goes, mm, and where (the deepest cell's centre).
    pub into: f64,
    pub at: Point,
}

/// What one toolpath's parts above the flutes hit.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Found {
    /// Moves that hit, in all.
    pub count: usize,
    /// The deepest hit.
    pub worst: Option<Hit>,
    /// The first [`KEPT`] hits, to show in the preview.
    pub hits: Vec<Hit>,
    /// Moves where a part comes down over a clamp (a keep-out zone), in all.
    pub clamp_count: usize,
    /// The first of them.
    pub clamp: Option<ClampHit>,
}

/// A part above the flutes coming down over a clamp: a keep-out zone standing above the
/// material's top.
#[derive(Clone, Debug, PartialEq)]
pub struct ClampHit {
    pub part: Part,
    /// The zone's name ("Clamp 1"), how tall it stands, and whether that was taken.
    pub zone: String,
    pub stands: f64,
    pub guessed: bool,
    /// How far below the clamp's top the part's bottom comes, mm, and where the tip was.
    pub below: f64,
    pub at: Point,
}

impl Found {
    fn add(&mut self, hit: Hit) {
        self.count += 1;
        if self.worst.is_none_or(|w| hit.into > w.into) {
            self.worst = Some(hit);
        }
        if self.hits.len() < KEPT {
            self.hits.push(hit);
        }
    }

    /// Whether anything hits.
    pub fn any(&self) -> bool {
        self.count > 0 || self.clamp_count > 0
    }
}

/// Whether a toolpath cuts nothing away: a laser burns, and a spoilboard surfacing runs
/// with nothing on the bed.
fn cuts_nothing(toolpath: &Toolpath) -> bool {
    !toolpath.power.is_empty()
        || (toolpath.strategy == "surfacing_toolpath" && toolpath.params["area"] == "spoilboard")
}

/// What the parts above the flutes hit cutting `toolpaths` in the order given into `job`'s
/// uncut material, held by `machine`'s holder (or the bit's own), with `clamps` (the
/// keep-out zones) standing on it: one [`Found`] for each. Quick when no move goes deep
/// enough for anything above the flutes to reach the top or a clamp.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "a cell count within bounds"
)]
pub fn find(
    job: &Job,
    toolpaths: &[&Toolpath],
    machine: &Holding,
    clamps: &[KeepOutZone],
) -> Vec<Found> {
    let mut found = vec![Found::default(); toolpaths.len()];
    let bands: Vec<Vec<Band>> = toolpaths.iter().map(|t| bands(&t.tool, machine)).collect();
    let lowest = |bands: &[Band]| bands.iter().map(|b| b.rise).fold(f64::INFINITY, f64::min);
    if job.rotary.is_none() {
        for ((toolpath, bands), found) in toolpaths.iter().zip(&bands).zip(&mut found) {
            if !cuts_nothing(toolpath) {
                over_clamps(toolpath, bands, clamps, found);
            }
        }
    }
    let deep_anywhere = toolpaths.iter().zip(&bands).any(|(t, bands)| {
        let rise = lowest(bands);
        !cuts_nothing(t) && t.moves.iter().any(|m| m.to.z + rise < 0.0)
    });
    if !deep_anywhere || job.rotary.is_some() || job.width <= 0.0 || job.height <= 0.0 {
        return found;
    }
    let smallest = toolpaths
        .iter()
        .filter(|t| t.tool.diameter > 0.0)
        .map(|t| t.tool.diameter)
        .fold(f64::INFINITY, f64::min);
    if !smallest.is_finite() {
        return found;
    }
    let cells = (job.width.max(job.height) / (smallest / 2.0)).clamp(FEWEST_CELLS, MOST_CELLS);
    let mut material = Material::new(job, cells as usize);
    for ((toolpath, bands), found) in toolpaths.iter().zip(&bands).zip(&mut found) {
        if progress::cancelled() {
            break;
        }
        if cuts_nothing(toolpath) {
            continue;
        }
        cut_and_look(&mut material, toolpath, bands, lowest(bands), found);
    }
    found
}

/// Adds to `found` each move of `toolpath` that brings one of `bands` down over one of the
/// `clamps` lower than it stands.
fn over_clamps(toolpath: &Toolpath, bands: &[Band], clamps: &[KeepOutZone], found: &mut Found) {
    let lowest = bands.iter().map(|b| b.rise).fold(f64::INFINITY, f64::min);
    for clamp in clamps {
        let (stands, guessed) = clamp.height();
        // nothing comes low enough near any clamp: no outline to grow
        if !toolpath.moves.iter().any(|m| m.to.z + lowest < stands) {
            continue;
        }
        let grown: Vec<_> = bands.iter().map(|b| clamp.around(b.radius)).collect();
        for pair in toolpath.moves.windows(2) {
            let (a, b) = (pair[0].to, pair[1].to);
            let low = a.z.min(b.z);
            let hit = bands.iter().zip(&grown).find(|(band, around)| {
                low + band.rise < stands && around.crossed(a.truncate(), b.truncate())
            });
            if let Some((band, _)) = hit {
                found.clamp_count += 1;
                if found.clamp.is_none() {
                    found.clamp = Some(ClampHit {
                        part: band.part,
                        zone: clamp.name.clone(),
                        stands,
                        guessed,
                        below: stands - (low + band.rise),
                        at: b.truncate(),
                    });
                }
            }
        }
    }
}

/// Cuts `toolpath` into `material`, adding to `found` each move after which one of `bands`
/// stands in material.
fn cut_and_look(
    material: &mut Material,
    toolpath: &Toolpath,
    bands: &[Band],
    lowest: f64,
    found: &mut Found,
) {
    let tool = &toolpath.tool;
    // the peaks only come down as the material is cut, so they stay safe to search by
    let peaks = material.peaks();
    let moves = simplify(&toolpath.moves, 0.01);
    let mut batch: Vec<(DVec3, DVec3)> = Vec::with_capacity(BATCH);
    for pair in moves.windows(2) {
        let (a, b) = (pair[0].to, pair[1].to);
        let deep = a.z.min(b.z) + lowest < 0.0;
        if !deep {
            // a rapid hitting the material is a crash, which Save Toolpaths finds itself
            if pair[1].kind != MoveKind::Rapid {
                batch.push((a, b));
            }
            if batch.len() == BATCH {
                material.sweep_each(&batch, tool);
                batch.clear();
            }
            continue;
        }
        material.sweep_each(&batch, tool);
        batch.clear();
        // the move's own cut first: the flutes clear what is in their way as they go
        material.sweep(a, b, tool);
        for band in bands {
            if let Some((into, at)) = material.above_part(a, b, band.radius, band.rise, &peaks)
                && into > TOUCHES
            {
                found.add(Hit {
                    part: band.part,
                    from: a,
                    to: b,
                    radius: band.radius,
                    rise: band.rise,
                    into,
                    at,
                });
                // the lowest part that hits says enough for this move
                break;
            }
        }
    }
    material.sweep_each(&batch, tool);
}

/// What to say about `found` for the toolpath `name` cutting with `tool` held by `machine`'s
/// holder (or the bit's own), as a sentence that says how to put it right; `None` when
/// nothing hits.
pub fn sentence(name: &str, tool: &Tool, found: &Found, units: Units) -> Option<String> {
    let said: Vec<String> = [
        material_sentence(name, tool, found, units),
        clamp_sentence(name, tool, found, units),
    ]
    .into_iter()
    .flatten()
    .collect();
    (!said.is_empty()).then(|| said.join(". "))
}

/// What to say of a part coming down over a clamp; `None` when none does.
fn clamp_sentence(name: &str, tool: &Tool, found: &Found, units: Units) -> Option<String> {
    let hit = found.clamp.as_ref()?;
    let times = match found.clamp_count {
        1 => String::new(),
        n => format!(" ({n} moves in all)"),
    };
    let stands = units.length(hit.stands, 1);
    let taken = if hit.guessed {
        format!(
            ", taken as {stands} tall because its height is not set: set how tall it stands \
             in Keep-Out Zones"
        )
    } else {
        format!(", which stands {stands} tall")
    };
    let part = match hit.part {
        Part::Shank => format!("shank of {}", tool.name),
        part => part.name().to_owned(),
    };
    Some(format!(
        "{name}: the {part} would come down {} below the top of {}{taken}, near X{} Y{}{times}. \
         Move the clamp further from the cut, or let the bit stick out further",
        units.length(hit.below, 1),
        hit.zone,
        position(hit.at.x, units),
        position(hit.at.y, units),
    ))
}

/// What to say of a part hitting the material; `None` when none does.
fn material_sentence(name: &str, tool: &Tool, found: &Found, units: Units) -> Option<String> {
    let worst = found.worst?;
    let where_ = format!(
        "near X{} Y{}",
        position(worst.at.x, units),
        position(worst.at.y, units)
    );
    let times = match found.count {
        1 => String::new(),
        n => format!(" ({n} moves in all)"),
    };
    let into = units.length(worst.into, 1);
    let (out, guessed) = stickout(tool);
    let out = units.length(out, 1);
    let taken = if guessed {
        format!(
            ", taken as {out} because its Stick-out is not set in the Tool Database. Measure \
             it and set it there"
        )
    } else {
        String::new()
    };
    Some(match worst.part {
        Part::Shank => format!(
            "{name}: the shank of {}, wider than its cutting part, would rub {into} into the \
             material {where_}{times}, because the cut goes deeper than the flutes. Cut less \
             deep, or use a bit with longer flutes",
            tool.name
        ),
        part => format!(
            "{name}: the {} would hit the material {where_}, going {into} into it{times}. \
             {} sticks out {out} below the collet nut{taken}. Clear the material round the \
             cut first (a wider pocket), let the bit stick out further, or use a longer bit",
            part.name(),
            tool.name
        ),
    })
}

/// The cells of `material` (row by row) standing in the way of the parts in `hits`: under a
/// part and higher than its bottom there, to show in red in the preview. A wall the part hit
/// that a later toolpath cut away is no longer shown.
pub fn struck_cells(material: &Material, hits: &[Hit]) -> Vec<usize> {
    let mut struck = vec![false; material.heights.len()];
    // the cells from `lo` to `hi` mm along a side `count` cells long that starts at `origin`
    let span = |lo: f64, hi: f64, origin: f64, count: usize| {
        let first = ((lo - origin) / material.cell).floor().max(0.0);
        let last = ((hi - origin) / material.cell).ceil().max(0.0);
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "non-negative cell indices"
        )]
        let cells = (first as usize)..(last as usize).min(count);
        cells
    };
    for hit in hits {
        let (from, to) = (hit.from.truncate(), hit.to.truncate());
        let bottom = hit.from.z.min(hit.to.z) + hit.rise;
        let (lo, hi) = (from.min(to) - hit.radius, from.max(to) + hit.radius);
        let columns = span(lo.x, hi.x, material.origin.x, material.columns);
        for row in span(lo.y, hi.y, material.origin.y, material.rows) {
            for column in columns.clone() {
                let cell = row * material.columns + column;
                if !struck[cell]
                    && material.heights[cell] > bottom
                    && distance_to_line(material.centre(column, row), from, to) <= hit.radius
                {
                    struck[cell] = true;
                }
            }
        }
    }
    struck
        .iter()
        .enumerate()
        .filter_map(|(cell, &yes)| yes.then_some(cell))
        .collect()
}

/// How far `p` is from the line from `a` to `b`.
fn distance_to_line(p: Point, a: Point, b: Point) -> f64 {
    let along = b - a;
    let t = if along == Point::ZERO {
        0.0
    } else {
        ((p - a).dot(along) / along.length_squared()).clamp(0.0, 1.0)
    };
    p.distance(a + along * t)
}

/// A point's X or Y as the status bar shows it.
fn position(mm: f64, units: Units) -> String {
    match units {
        Units::Mm => format!("{mm:.1}"),
        Units::Inch => format!("{:.3}", mm / 25.4),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::toolpath::Move;

    fn job() -> Job {
        Job {
            width: 200.0,
            height: 200.0,
            thickness: 50.0,
            ..Job::default()
        }
    }

    fn bit() -> Tool {
        Tool {
            name: "6 mm End Mill".into(),
            diameter: 6.0,
            flute_length: 20.0,
            stickout: 25.0,
            ..Tool::default()
        }
    }

    /// A slot along y at x = 100 from y 50 to 150, `depth` deep in one go.
    fn slot(tool: Tool, depth: f64) -> Toolpath {
        let at = |x: f64, y: f64, z: f64, kind| Move {
            to: DVec3::new(x, y, z),
            kind,
        };
        Toolpath {
            name: "Slot".into(),
            strategy: "pocket_toolpath".into(),
            params: serde_json::Value::Null,
            tool,
            vectors: vec![],
            visible: true,
            power: vec![],
            stale: false,
            group: None,
            drill_bank: None,
            fallbacks: vec![],
            moves: vec![
                at(100.0, 50.0, 5.0, MoveKind::Rapid),
                at(100.0, 50.0, -depth, MoveKind::Plunge),
                at(100.0, 150.0, -depth, MoveKind::Cut),
                at(100.0, 150.0, 5.0, MoveKind::Rapid),
            ],
        }
    }

    #[test]
    fn a_shallow_cut_hits_nothing() {
        let found = find(&job(), &[&slot(bit(), 10.0)], &Holding::default(), &[]);
        assert!(!found[0].any());
    }

    #[test]
    fn a_cut_deeper_than_the_stickout_hits_the_collet_nut() {
        let found = find(&job(), &[&slot(bit(), 30.0)], &Holding::default(), &[]);
        let worst = found[0].worst.expect("the nut hits");
        assert_eq!(worst.part, Part::ColletNut);
        // the nut's face is 25 above the tip at -30: the uncut top stands 5 above it
        assert!((worst.into - 5.0).abs() < 0.6, "{}", worst.into);
    }

    #[test]
    fn material_cleared_first_is_not_hit() {
        // a wide pocket cleared to 30 deep round the slot: nothing left for the nut
        let mut clear = slot(bit(), 30.0);
        clear.name = "Clear".into();
        clear.tool = Tool {
            diameter: 60.0,
            flute_length: 40.0,
            stickout: 45.0,
            ..bit()
        };
        let found = find(
            &job(),
            &[&clear, &slot(bit(), 30.0)],
            &Holding::default(),
            &[],
        );
        assert!(!found[1].any(), "{:?}", found[1].worst);
    }

    #[test]
    fn a_wide_shank_rubs_below_the_nut() {
        // a 1/8" bit with 6 mm flutes on a 1/4" shank, 10 deep
        let tool = Tool {
            diameter: 3.175,
            flute_length: 6.0,
            shank_diameter: 6.35,
            stickout: 30.0,
            ..bit()
        };
        let found = find(&job(), &[&slot(tool, 10.0)], &Holding::default(), &[]);
        assert_eq!(found[0].worst.map(|w| w.part), Some(Part::Shank));
    }

    #[test]
    fn a_bit_carries_its_own_holder() {
        let mut tool = bit();
        tool.holder = Some(Holder {
            nut_diameter: 0.0,
            nut_length: 0.0,
            spindle_diameter: 0.0,
        });
        // only once the machine takes holders per bit
        let found = find(
            &job(),
            &[&slot(tool.clone(), 30.0)],
            &Holding::default(),
            &[],
        );
        assert!(found[0].any());
        let per_bit = Holding {
            per_bit: true,
            ..Holding::default()
        };
        let found = find(&job(), &[&slot(tool, 30.0)], &per_bit, &[]);
        assert!(!found[0].any());
    }

    #[test]
    fn the_stock_hit_is_marked_for_the_preview() {
        let toolpath = slot(bit(), 30.0);
        let found = find(&job(), &[&toolpath], &Holding::default(), &[]);
        let mut material = Material::new(&job(), 200);
        material.cut(&toolpath);
        let struck = struck_cells(&material, &found[0].hits);
        assert_ne!(struck, Vec::<usize>::new());
        // round the slot, inside the nut's reach, never in the slot itself
        let (a, b) = (Point::new(100.0, 50.0), Point::new(100.0, 150.0));
        for &i in &struck {
            let at = material.centre(i % material.columns, i / material.columns);
            let off = distance_to_line(at, a, b);
            assert!(off > 3.0 && off <= 17.5 + 1.0, "{at:?}");
        }
    }

    /// A clamp 10 mm square beside the slot, its near edge `gap` from the slot's line.
    fn clamp(gap: f64, stands: f64) -> KeepOutZone {
        let (x, y) = (100.0 + gap, 95.0);
        let corners = [
            Point::new(x, y),
            Point::new(x + 10.0, y),
            Point::new(x + 10.0, y + 10.0),
            Point::new(x, y + 10.0),
        ];
        KeepOutZone::new("Clamp 1", crate::geometry::Path::polyline(&corners, true))
            .standing(stands)
    }

    #[test]
    fn the_collet_nut_coming_down_over_a_clamp_is_found() {
        // 10 deep, the nut's face is 15 above the top: a 20 mm clamp 10 mm away is under it
        let found = find(
            &job(),
            &[&slot(bit(), 10.0)],
            &Holding::default(),
            &[clamp(10.0, 0.0)],
        );
        let hit = found[0]
            .clamp
            .as_ref()
            .expect("the nut comes over the clamp");
        assert_eq!(hit.part, Part::ColletNut);
        assert!(hit.guessed && (hit.below - 5.0).abs() < 1e-9);
        let said = sentence("Slot", &bit(), &found[0], Units::Mm).unwrap();
        assert!(
            said.contains("below the top of Clamp 1, taken as 20.0 mm tall"),
            "{said}"
        );
        // a lower clamp, or one further off, is clear
        for clear in [clamp(10.0, 12.0), clamp(30.0, 0.0)] {
            let found = find(&job(), &[&slot(bit(), 10.0)], &Holding::default(), &[clear]);
            assert!(!found[0].any(), "{:?}", found[0].clamp);
        }
    }

    #[test]
    fn stickout_is_guessed_from_the_flutes() {
        let mut tool = bit();
        tool.stickout = 0.0;
        assert_eq!(stickout(&tool), (20.0 + SHANK_SHOWING, true));
        tool.flute_length = 0.0;
        assert_eq!(stickout(&tool), (GUESSED_STICKOUT, true));
    }

    #[test]
    fn the_sentence_says_what_hits_and_how_to_fix_it() {
        let tool = bit();
        let found = find(
            &job(),
            &[&slot(tool.clone(), 30.0)],
            &Holding::default(),
            &[],
        );
        let said = sentence("Slot", &tool, &found[0], Units::Mm).unwrap();
        assert!(said.starts_with("Slot: the collet nut would hit the material near X"));
        assert!(said.contains("sticks out 25.0 mm below the collet nut."));
        assert!(said.contains("Clear the material round the cut first"));
    }
}
