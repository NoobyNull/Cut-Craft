//! Merged Toolpath: the toolpaths shown in the toolpath list when Apply is pressed (tick the
//! ones to merge there), all using one bit, made into one, with their cuts
//! in whichever of the orders asked for leaves the least travelling between them, each
//! part's cuts together when asked; and Ungroup, which takes a merged toolpath apart again.

use cut_craft_core::document::{Document, EntityId};
use cut_craft_core::geometry::{Point, inside};
use cut_craft_core::glam::DVec3;
use cut_craft_core::toolpath::{Move, MoveKind, Toolpath, made_by};
use cut_craft_core::{Change, Command, CommandError, Form, Meta, existing};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct MergedToolpath;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[expect(clippy::struct_excessive_bools, reason = "the Ordering tick boxes")]
pub struct Params {
    /// What the toolpath is called in the toolpath list. It merges the toolpaths ticked
    /// (shown) in the toolpath list when Apply is pressed: tick them there first
    /// (it is marked out of date when one of them changes: recalculate it then, and the
    /// hidden ones out of date are recalculated first). Those
    /// toolpaths are then hidden, so saving the shown toolpaths does not cut them twice. They
    /// must use one bit, and all is cut with the first one's feeds and speed: a toolpath
    /// whose copy of that bit is set differently is warned of
    pub name: String,
    /// The toolpaths to merge (none: the ones shown in the toolpath list when Apply is
    /// pressed). Not on the form: tick the toolpaths to merge in the toolpath list
    #[schemars(skip)]
    #[serde(default)]
    pub sources: Vec<EntityId>,
    /// Cuts in order across the job; of the orders ticked, the quickest is kept. Whatever
    /// the order, the cuts inside a closed shape always come before the shape itself
    #[serde(default = "yes")]
    #[schemars(title = "Left to Right", extend("x-group" = "Ordering"))]
    pub left_to_right: bool,
    /// Cuts in order up the job
    #[serde(default = "yes")]
    #[schemars(title = "Bottom to Top", extend("x-group" = "Ordering"))]
    pub bottom_to_top: bool,
    /// Cuts in rows up the job, across each row
    #[serde(default = "yes")]
    #[schemars(title = "Grid", extend("x-group" = "Ordering"))]
    pub grid: bool,
    /// How many rows the Grid order cuts the job into, bottom to top. More rows suit many
    /// small parts; fewer suit a few big ones
    #[serde(default = "grid_rows")]
    #[schemars(
        title = "Grid rows",
        range(min = 1, max = 1000),
        extend("x-group" = "Ordering", "x-show-when" = {"grid": "true"})
    )]
    pub grid_rows: u32,
    /// Each cut the nearest to where the last ended
    #[serde(default = "yes")]
    #[schemars(title = "Shortest Path", extend("x-group" = "Ordering"))]
    pub shortest_path: bool,
    /// Each part's cuts together: a part is an outermost closed cut with the cuts inside
    /// it, cut in the toolpath list's order within the part (its outline always last)
    #[serde(default)]
    #[schemars(title = "Merge By Part", extend("x-group" = "Ordering"))]
    pub merge_by_part: bool,
}

fn yes() -> bool {
    true
}

/// The rows the Grid order used before it was a setting.
fn grid_rows() -> u32 {
    8
}

impl Default for Params {
    fn default() -> Self {
        Self {
            name: "Merged".into(),
            sources: vec![],
            left_to_right: true,
            bottom_to_top: true,
            grid: true,
            grid_rows: grid_rows(),
            shortest_path: true,
            merge_by_part: false,
        }
    }
}

/// Refused when one of `sources` holds the bit off the turning axis: that goes move by move
/// with its own moves, and put in another order its places across would no longer match.
fn refuse_held_off(sources: &[&Toolpath]) -> Result<(), CommandError> {
    match sources
        .iter()
        .find(|t| t.across.iter().any(|a| a.abs() > 1e-9))
    {
        Some(held) => Err(CommandError::Failed(format!(
            "{} holds the bit off the turning axis, so it can't be merged: leave it as it is, or hide it before merging",
            held.name
        ))),
        None => Ok(()),
    }
}

/// One plunge-to-retract run of feed moves, with each move's laser power.
#[derive(Clone)]
struct Cut {
    moves: Vec<Move>,
    power: Vec<f64>,
}

impl Cut {
    fn start(&self) -> DVec3 {
        self.moves[0].to
    }

    fn end(&self) -> DVec3 {
        self.moves[self.moves.len() - 1].to
    }
}

/// A toolpath's feed moves split at its rapids.
fn cuts(t: &Toolpath) -> Vec<Cut> {
    let mut out = vec![];
    let mut cut = Cut {
        moves: vec![],
        power: vec![],
    };
    for (i, m) in t.moves.iter().enumerate() {
        if m.kind == MoveKind::Rapid {
            if !cut.moves.is_empty() {
                out.push(std::mem::replace(
                    &mut cut,
                    Cut {
                        moves: vec![],
                        power: vec![],
                    },
                ));
            }
        } else {
            cut.moves.push(*m);
            cut.power.push(t.power.get(i).copied().unwrap_or(0.0));
        }
    }
    if !cut.moves.is_empty() {
        out.push(cut);
    }
    out
}

/// Distance travelled between the cuts in this order, from the first.
fn travel(order: &[Cut]) -> f64 {
    order
        .windows(2)
        .map(|w| w[0].end().truncate().distance(w[1].start().truncate()))
        .sum()
}

/// The cuts sorted by `key` of where each starts.
fn sorted_by(cuts: &[Cut], key: impl Fn(Point) -> (f64, f64)) -> Vec<Cut> {
    let mut order = cuts.to_vec();
    order.sort_by(|a, b| {
        let (ka, kb) = (key(a.start().truncate()), key(b.start().truncate()));
        ka.partial_cmp(&kb).unwrap_or(std::cmp::Ordering::Equal)
    });
    order
}

/// The orders asked for, each of `cuts` (the toolpath list's order): the quickest is kept.
fn orders(cuts: &[Cut], p: &Params, height: f64) -> Vec<Vec<Cut>> {
    let mut tried = vec![];
    if p.left_to_right {
        tried.push(sorted_by(cuts, |q| (q.x, q.y)));
    }
    if p.bottom_to_top {
        tried.push(sorted_by(cuts, |q| (q.y, q.x)));
    }
    if p.grid {
        let row = (height / f64::from(p.grid_rows.max(1))).max(1e-9);
        tried.push(sorted_by(cuts, |q| ((q.y / row).floor(), q.x)));
    }
    if p.shortest_path {
        tried.push(nearest_first(cuts.to_vec()));
    }
    tried
}

/// `cuts` as parts: each outermost closed cut with every cut starting inside it, in the
/// order given; a cut inside none is a part of its own.
fn parts(cuts: &[Cut]) -> Vec<Vec<Cut>> {
    let rings: Vec<Option<Vec<Point>>> = cuts.iter().map(ring).collect();
    // the cut that holds each one: the smallest closed cut it starts inside, if any
    let holder = |i: usize| -> Option<usize> {
        let at = cuts[i].start().truncate();
        (0..cuts.len())
            .filter(|&j| j != i)
            .filter(|&j| {
                rings[j]
                    .as_ref()
                    .is_some_and(|r| inside(std::slice::from_ref(r), at))
            })
            .min_by(|&a, &b| {
                let size = |j: usize| rings[j].as_ref().map_or(usize::MAX, Vec::len);
                size(a).cmp(&size(b))
            })
    };
    let mut outer_of: Vec<usize> = (0..cuts.len()).collect();
    for (i, outer) in outer_of.iter_mut().enumerate() {
        let mut at = i;
        while let Some(h) = holder(at) {
            if h == i {
                break;
            }
            at = h;
        }
        *outer = at;
    }
    let mut out: Vec<(usize, Vec<Cut>)> = vec![];
    for (i, cut) in cuts.iter().enumerate() {
        match out.iter_mut().find(|(outer, _)| *outer == outer_of[i]) {
            Some((_, part)) => part.push(cut.clone()),
            None => out.push((outer_of[i], vec![cut.clone()])),
        }
    }
    out.into_iter()
        .map(|(_, part)| insides_first(&part))
        .collect()
}

/// A closed cut's outline, if it ends where it starts.
fn ring(cut: &Cut) -> Option<Vec<Point>> {
    let ring: Vec<Point> = cut.moves.iter().map(|m| m.to.truncate()).collect();
    let closed = ring.len() > 2 && ring[0].distance(ring[ring.len() - 1]) < 1e-6;
    closed.then_some(ring)
}

/// How far `at` is from the outline `ring`.
fn off_ring(ring: &[Point], at: Point) -> f64 {
    ring.windows(2)
        .map(|w| {
            let (a, b) = (w[0], w[1]);
            let ab = b - a;
            let t = if ab.length_squared() > 0.0 {
                ((at - a).dot(ab) / ab.length_squared()).clamp(0.0, 1.0)
            } else {
                0.0
            };
            at.distance(a + ab * t)
        })
        .fold(f64::INFINITY, f64::min)
}

/// `order` with no closed cut ahead of a cut that starts inside it, so a part's outline is
/// never cut free before the cuts inside the part; otherwise as given.
fn insides_first(order: &[Cut]) -> Vec<Cut> {
    let rings: Vec<Option<Vec<Point>>> = order.iter().map(ring).collect();
    // for each cut, the cuts that must go before it: those starting inside it
    let waits_for: Vec<Vec<usize>> = rings
        .iter()
        .enumerate()
        .map(|(i, r)| {
            let Some(r) = r else { return vec![] };
            (0..order.len())
                .filter(|&j| j != i)
                .filter(|&j| {
                    let at = order[j].start().truncate();
                    off_ring(r, at) > 1e-6 && inside(std::slice::from_ref(r), at)
                })
                .collect()
        })
        .collect();
    let mut placed = vec![false; order.len()];
    let mut out = Vec::with_capacity(order.len());
    while out.len() < order.len() {
        let ready = (0..order.len())
            .find(|&i| !placed[i] && waits_for[i].iter().all(|&j| placed[j]))
            // outlines that overlap each other: the earliest left
            .or_else(|| (0..order.len()).find(|&i| !placed[i]))
            .expect("a cut is left");
        placed[ready] = true;
        out.push(ready);
    }
    out.into_iter().map(|i| order[i].clone()).collect()
}

/// The cuts, each next one the nearest start to where the last ended.
fn nearest_first(mut left: Vec<Cut>) -> Vec<Cut> {
    let mut order: Vec<Cut> = vec![];
    while !left.is_empty() {
        let at = order.last().map_or(left[0].start(), Cut::end).truncate();
        let next = (0..left.len())
            .min_by(|&a, &b| {
                let d = |i: usize| left[i].start().truncate().distance(at);
                d(a).total_cmp(&d(b))
            })
            .expect("left is not empty");
        order.push(left.swap_remove(next));
    }
    order
}

/// What the merged toolpath warns of: each source whose bit, named as `first`'s, has other
/// settings (feeds, speed or the bit's size), which merging replaces with `first`'s.
fn tool_differences(first: &Toolpath, sources: &[&Toolpath]) -> Vec<String> {
    let (a, mut notes) = (&first.tool, vec![]);
    for source in sources {
        let b = &source.tool;
        let differ = |x: f64, y: f64| (x - y).abs() > 1e-9;
        let fields: Vec<&str> = [
            (a.shape != b.shape, "shape"),
            (differ(a.diameter, b.diameter), "diameter"),
            (differ(a.angle, b.angle), "angle"),
            (differ(a.tip_radius, b.tip_radius), "tip radius"),
            (differ(a.feed_rate, b.feed_rate), "feed rate"),
            (differ(a.plunge_rate, b.plunge_rate), "plunge rate"),
            (differ(a.spindle_speed, b.spindle_speed), "spindle speed"),
        ]
        .into_iter()
        .filter_map(|(differs, field)| differs.then_some(field))
        .collect();
        if !fields.is_empty() {
            notes.push(format!(
                "{} uses {} with a different {}: the merged toolpath cuts it all with {}'s \
                 settings. Edit the bit in one of them to match if that is not right",
                source.name,
                b.name,
                fields.join(", "),
                first.name
            ));
        }
    }
    notes
}

/// The toolpaths shown in the toolpath list, but none made by `strategy`.
fn visible(doc: &Document, strategy: &str) -> Vec<EntityId> {
    doc.toolpaths
        .iter()
        .filter(|(_, t)| t.visible && t.strategy != strategy)
        .map(|(&id, _)| id)
        .collect()
}

impl Command for MergedToolpath {
    const ID: &'static str = "merged_toolpath";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Merged Toolpath",
            group: "Toolpath Operations",
            icon: "toolpaths/merge_toolpaths.png",
            shortcut: "",
            form: Form::TOOLPATH,
        }
    }

    fn initial_params(_: &Document) -> Params {
        // no sources: the toolpaths shown when Apply is pressed, not when the form opened
        Params::default()
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        let mut named = if p.sources.is_empty() {
            visible(doc, Self::ID)
        } else {
            p.sources.clone()
        };
        // a toolpath named twice is used once, not cut twice
        let mut seen = std::collections::BTreeSet::new();
        named.retain(|id| seen.insert(*id));
        let sources: Vec<&Toolpath> = named
            .iter()
            .filter_map(|id| doc.toolpaths.get(id))
            .collect();
        let Some(first) = sources.first() else {
            return Err(CommandError::Failed(
                "show the toolpaths to merge in the toolpath list".into(),
            ));
        };
        // a drill bank's plunges are its own: merged, they would be drilled by one drill
        if let Some(bank) = sources.iter().find(|t| t.drill_bank.is_some()) {
            return Err(CommandError::Failed(format!(
                "{} is a drill bank toolpath, which cannot be merged: hide it before merging",
                bank.name
            )));
        }
        if let Some(other) = sources.iter().find(|t| t.tool.name != first.tool.name) {
            return Err(CommandError::Failed(format!(
                "only toolpaths with the same bit merge: {} uses {} and {} uses {}. Show only toolpaths that use one bit",
                first.name, first.tool.name, other.name, other.tool.name
            )));
        }
        refuse_held_off(&sources)?;
        let notes = tool_differences(first, &sources);
        let in_order: Vec<Cut> = sources.iter().flat_map(|t| cuts(t)).collect();
        let height = doc.job.height;
        let order = if p.merge_by_part {
            // the parts in the quickest of the orders asked for, each part's cuts as listed
            let parts = parts(&in_order);
            let firsts: Vec<Cut> = parts.iter().map(|part| part[0].clone()).collect();
            let quickest = orders(&firsts, p, height)
                .into_iter()
                .min_by(|a, b| travel(a).total_cmp(&travel(b)))
                .unwrap_or(firsts);
            quickest
                .iter()
                .flat_map(|first| {
                    parts
                        .iter()
                        .find(|part| {
                            part[0].start() == first.start() && part[0].end() == first.end()
                        })
                        .cloned()
                        .unwrap_or_default()
                })
                .collect()
        } else {
            orders(&in_order, p, height)
                .into_iter()
                .map(|o| insides_first(&o))
                .min_by(|a, b| travel(a).total_cmp(&travel(b)))
                .unwrap_or_else(|| insides_first(&in_order))
        };
        let safe = doc.job.clearance;
        let laser = sources.iter().any(|t| !t.power.is_empty());
        let (mut moves, mut power) = (vec![], vec![]);
        for cut in &order {
            moves.push(Move {
                kind: MoveKind::Rapid,
                to: cut.start().truncate().extend(safe),
            });
            moves.extend(&cut.moves);
            moves.push(Move {
                kind: MoveKind::Rapid,
                to: cut.end().truncate().extend(safe),
            });
            power.push(0.0);
            power.extend(&cut.power);
            power.push(0.0);
        }
        // the sources as resolved, so Ungroup and a recalculation know them
        let settings = Params {
            sources: named.clone(),
            ..p.clone()
        };
        let mut toolpath =
            made_by::<Self>(&settings, p.name.clone(), first.tool.clone(), vec![], moves);
        if laser {
            toolpath.power = power;
        }
        let mut c = Change::builder(doc);
        c.add_toolpath(toolpath);
        for note in notes {
            c.note(note);
        }
        // the sources are under the new toolpath: hidden, so the default save (every shown
        // toolpath) does not cut them twice. Recalculating leaves them as they are
        if p.sources.is_empty() {
            c.hide_toolpaths(&named);
        }
        Ok(c.finish())
    }
}

/// Ungroup: a merged toolpath taken away, its sources shown again.
pub struct UngroupMergedToolpath;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct UngroupParams {
    /// The merged toolpath to take apart
    pub toolpath: EntityId,
}

impl Default for UngroupParams {
    fn default() -> Self {
        Self {
            toolpath: EntityId(1),
        }
    }
}

impl Command for UngroupMergedToolpath {
    const ID: &'static str = "ungroup_merged_toolpath";
    type Params = UngroupParams;

    fn meta() -> Meta {
        Meta {
            label: "Ungroup Merged Toolpath",
            group: "Toolpaths",
            icon: "",
            shortcut: "",
            form: Form::Apply,
        }
    }

    fn run(doc: &Document, p: &UngroupParams) -> Result<Change, CommandError> {
        let merged = existing(&doc.toolpaths, p.toolpath, "toolpath")?;
        if merged.strategy != MergedToolpath::ID {
            return Err(CommandError::Failed(format!(
                "{} is not a merged toolpath. Pick a merged toolpath to ungroup",
                merged.name
            )));
        }
        let settings: Params = serde_json::from_value(merged.params.clone())
            .map_err(|e| CommandError::Failed(e.to_string()))?;
        let mut c = Change::builder(doc);
        c.remove_toolpath(p.toolpath);
        for id in settings.sources {
            if let Some(source) = doc.toolpaths.get(&id).filter(|t| !t.visible) {
                c.replace_toolpath(
                    id,
                    Toolpath {
                        visible: true,
                        ..source.clone()
                    },
                );
            }
        }
        Ok(c.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::testing::with_toolpath;
    use cut_craft_core::toolpath::Tool;

    fn mv(kind: MoveKind, x: f64, z: f64) -> Move {
        Move {
            kind,
            to: DVec3::new(x, 0.0, z),
        }
    }

    /// A plunge-and-cut at each x, in that order, as one toolpath.
    fn cutting_at(xs: &[f64]) -> Vec<Move> {
        xs.iter()
            .flat_map(|&x| {
                [
                    mv(MoveKind::Rapid, x, 5.0),
                    mv(MoveKind::Plunge, x, -1.0),
                    mv(MoveKind::Cut, x + 1.0, -1.0),
                    mv(MoveKind::Rapid, x + 1.0, 5.0),
                ]
            })
            .collect()
    }

    /// Two toolpaths, one tool: cuts at 0 and 100, then at 50.
    fn two() -> Document {
        let mut doc = with_toolpath();
        let mut first = doc.toolpaths[&EntityId(1)].clone();
        first.moves = cutting_at(&[0.0, 100.0]);
        let mut second = first.clone();
        second.name = "Second".into();
        second.moves = cutting_at(&[50.0]);
        let mut c = Change::builder(&doc);
        c.replace_toolpath(EntityId(1), first);
        c.add_toolpath(second);
        let change = c.finish();
        doc.apply(&change).unwrap();
        doc
    }

    cut_craft_core::contract_tests!(MergedToolpath, two);

    fn merged(doc: &Document) -> Toolpath {
        let change = MergedToolpath::run(doc, &Params::default()).unwrap();
        let mut after = doc.clone();
        after.apply(&change).unwrap();
        after
            .toolpaths
            .into_values()
            .find(|t| t.strategy == MergedToolpath::ID)
            .unwrap()
    }

    #[test]
    fn the_form_hides_the_sources_and_merges_the_toolpaths_visible_at_apply() {
        let doc = two();
        let schema = serde_json::to_value(schemars::schema_for!(Params)).unwrap();
        assert!(schema["properties"].get("sources").is_none(), "no id list");
        let name = schema["properties"]["name"]["description"].as_str();
        assert!(
            name.is_some_and(|h| h.contains("ticked")),
            "the form says which toolpaths it merges: {name:?}"
        );
        assert_eq!(MergedToolpath::initial_params(&doc).sources, []);
        let once = merged(&doc).moves.len();
        let doubled = Params {
            sources: vec![EntityId(1), EntityId(1), EntityId(2)],
            ..Params::default()
        };
        let change = MergedToolpath::run(&doc, &doubled).unwrap();
        let mut after = doc.clone();
        after.apply(&change).unwrap();
        let made = after
            .toolpaths
            .values()
            .find(|t| t.strategy == MergedToolpath::ID)
            .unwrap();
        assert_eq!(made.moves.len(), once, "a repeated id is merged once");
    }

    #[test]
    fn the_toolpaths_merged_are_hidden_so_a_save_does_not_cut_them_twice() {
        let doc = two();
        let change = MergedToolpath::run(&doc, &Params::default()).unwrap();
        let mut after = doc.clone();
        after.apply(&change).unwrap();
        assert!(!after.toolpaths[&EntityId(1)].visible);
        assert!(!after.toolpaths[&EntityId(2)].visible);
        assert!(merged(&doc).visible, "the merged toolpath is shown");
    }

    #[test]
    fn grid_rows_is_a_form_field_that_sets_how_tall_the_grid_rows_are() {
        let old: Params = serde_json::from_value(serde_json::json!({
            "name": "Merged", "sources": []
        }))
        .unwrap();
        assert_eq!(old.grid_rows, 8, "the fixed value it replaces");
        let at = |x: f64, y: f64| Cut {
            moves: vec![Move {
                kind: MoveKind::Rapid,
                to: DVec3::new(x, y, 5.0),
            }],
            power: vec![0.0],
        };
        let cuts = [at(0.0, 9.0), at(50.0, 1.0)];
        let grid_only = |rows| Params {
            left_to_right: false,
            bottom_to_top: false,
            shortest_path: false,
            grid_rows: rows,
            ..Params::default()
        };
        let first_x = |rows| orders(&cuts, &grid_only(rows), 80.0)[0][0].start().x;
        assert!(first_x(8) < 1.0, "8 rows of 10 mm: one row, left first");
        assert!(first_x(16) > 49.0, "16 rows of 5 mm: the lower row first");
    }

    #[test]
    fn merges_every_cut_in_the_order_with_least_travel() {
        let t = merged(&two());
        let starts: Vec<f64> = t
            .moves
            .iter()
            .filter(|m| m.kind == MoveKind::Plunge)
            .map(|m| m.to.x)
            .collect();
        assert_eq!(starts, [0.0, 50.0, 100.0]);
    }

    #[test]
    fn drill_bank_toolpaths_do_not_merge() {
        let mut doc = two();
        let id = *doc.toolpaths.keys().last().unwrap();
        let mut bank = doc.toolpaths[&id].clone();
        bank.drill_bank = Some(cut_craft_core::drill_bank::BankToolpath::default());
        let mut c = Change::builder(&doc);
        c.replace_toolpath(id, bank);
        doc.apply(&c.finish()).unwrap();
        let err = MergedToolpath::run(&doc, &Params::default()).unwrap_err();
        assert!(err.to_string().contains("drill bank"), "{err}");
    }

    #[test]
    fn toolpaths_holding_the_bit_off_the_axis_do_not_merge() {
        let mut doc = two();
        let id = *doc.toolpaths.keys().last().unwrap();
        let mut held = doc.toolpaths[&id].clone();
        held.across = vec![3.0; held.moves.len()];
        let mut c = Change::builder(&doc);
        c.replace_toolpath(id, held);
        doc.apply(&c.finish()).unwrap();
        let err = MergedToolpath::run(&doc, &Params::default()).unwrap_err();
        assert!(err.to_string().contains("off the turning axis"), "{err}");
    }

    #[test]
    fn a_same_named_tool_with_other_settings_merges_with_a_warning_naming_them() {
        let mut doc = two();
        let mut t = doc.toolpaths[&EntityId(2)].clone();
        t.tool.feed_rate += 500.0;
        t.tool.spindle_speed += 1000.0;
        let mut c = Change::builder(&doc);
        c.replace_toolpath(EntityId(2), t);
        let change = c.finish();
        doc.apply(&change).unwrap();
        let change = MergedToolpath::run(&doc, &Params::default()).unwrap();
        let note = change.notes.join(" ");
        assert!(
            note.contains("Second") && note.contains("feed rate") && note.contains("spindle speed"),
            "{note}"
        );
        assert!(!note.contains("plunge rate"), "only what differs: {note}");
        assert!(
            MergedToolpath::run(&two(), &Params::default())
                .unwrap()
                .notes
                .is_empty(),
            "the same tool: nothing to say"
        );
    }

    #[test]
    fn toolpaths_with_different_tools_do_not_merge() {
        let mut doc = two();
        let mut t = doc.toolpaths[&EntityId(2)].clone();
        t.tool = Tool {
            name: "Other".into(),
            ..Tool::default()
        };
        let mut c = Change::builder(&doc);
        c.replace_toolpath(EntityId(2), t);
        let change = c.finish();
        doc.apply(&change).unwrap();
        let why = MergedToolpath::run(&doc, &Params::default())
            .unwrap_err()
            .to_string();
        assert!(why.contains("same bit"), "{why}");
    }

    fn starts(t: &Toolpath) -> Vec<(f64, f64)> {
        t.moves
            .iter()
            .filter(|m| m.kind == MoveKind::Plunge)
            .map(|m| (m.to.x, m.to.y))
            .collect()
    }

    /// Plunge-and-cut at each point, in that order.
    fn cutting_at_points(points: &[(f64, f64)]) -> Vec<Move> {
        points
            .iter()
            .flat_map(|&(x, y)| {
                let at = |z| DVec3::new(x, y, z);
                [
                    Move {
                        kind: MoveKind::Rapid,
                        to: at(5.0),
                    },
                    Move {
                        kind: MoveKind::Plunge,
                        to: at(-1.0),
                    },
                    Move {
                        kind: MoveKind::Cut,
                        to: DVec3::new(x + 1.0, y, -1.0),
                    },
                    Move {
                        kind: MoveKind::Rapid,
                        to: DVec3::new(x + 1.0, y, 5.0),
                    },
                ]
            })
            .collect()
    }

    #[test]
    fn only_the_orders_ticked_are_tried() {
        let mut doc = with_toolpath();
        let mut t = doc.toolpaths[&EntityId(1)].clone();
        t.moves = cutting_at_points(&[(50.0, 0.0), (0.0, 50.0), (100.0, 100.0)]);
        let mut c = Change::builder(&doc);
        c.replace_toolpath(EntityId(1), t);
        let change = c.finish();
        doc.apply(&change).unwrap();
        let order = |p: &Params| {
            let change = MergedToolpath::run(&doc, p).unwrap();
            let mut after = doc.clone();
            after.apply(&change).unwrap();
            let t = after
                .toolpaths
                .into_values()
                .find(|t| t.strategy == MergedToolpath::ID)
                .unwrap();
            starts(&t)
        };
        let only = |left, bottom| Params {
            left_to_right: left,
            bottom_to_top: bottom,
            grid: false,
            shortest_path: false,
            ..Params::default()
        };
        assert_eq!(
            order(&only(true, false)),
            [(0.0, 50.0), (50.0, 0.0), (100.0, 100.0)]
        );
        assert_eq!(
            order(&only(false, true)),
            [(50.0, 0.0), (0.0, 50.0), (100.0, 100.0)]
        );
        assert_eq!(
            order(&only(false, false)),
            [(50.0, 0.0), (0.0, 50.0), (100.0, 100.0)],
            "none ticked: the list's order"
        );
    }

    #[test]
    fn merging_by_part_cuts_each_outer_shape_with_what_is_inside_it() {
        // two squares, each with a small cut inside: the insides first in the list
        let square = |x0: f64| {
            let p = |x, y| DVec3::new(x, y, -1.0);
            let corners = [
                p(x0, 0.0),
                p(x0 + 20.0, 0.0),
                p(x0 + 20.0, 20.0),
                p(x0, 20.0),
                p(x0, 0.0),
            ];
            let mut moves = vec![
                Move {
                    kind: MoveKind::Rapid,
                    to: DVec3::new(x0, 0.0, 5.0),
                },
                Move {
                    kind: MoveKind::Plunge,
                    to: corners[0],
                },
            ];
            moves.extend(corners[1..].iter().map(|&to| Move {
                kind: MoveKind::Cut,
                to,
            }));
            moves.push(Move {
                kind: MoveKind::Rapid,
                to: DVec3::new(x0, 0.0, 5.0),
            });
            moves
        };
        let mut doc = with_toolpath();
        let mut insides = doc.toolpaths[&EntityId(1)].clone();
        insides.moves = cutting_at_points(&[(5.0, 10.0), (105.0, 10.0)]);
        let mut squares = insides.clone();
        squares.name = "Squares".into();
        squares.moves = [square(0.0), square(100.0)].concat();
        let mut c = Change::builder(&doc);
        c.replace_toolpath(EntityId(1), insides);
        c.add_toolpath(squares);
        let change = c.finish();
        doc.apply(&change).unwrap();
        let merged = |by_part| {
            let p = Params {
                merge_by_part: by_part,
                ..Params::default()
            };
            let change = MergedToolpath::run(&doc, &p).unwrap();
            let mut after = doc.clone();
            after.apply(&change).unwrap();
            let t = after
                .toolpaths
                .into_values()
                .find(|t| t.strategy == MergedToolpath::ID)
                .unwrap();
            starts(&t).into_iter().map(|(x, _)| x).collect::<Vec<_>>()
        };
        assert_eq!(
            merged(true),
            [5.0, 0.0, 105.0, 100.0],
            "each part: its inside, then its square"
        );
        assert_eq!(
            merged(false),
            [5.0, 0.0, 105.0, 100.0],
            "else the quickest order of all the cuts, but never a shape before what is inside it"
        );
        // the squares first in the list: each still waits for what is inside it
        let sources = vec![EntityId(2), EntityId(1)];
        for by_part in [true, false] {
            let p = Params {
                merge_by_part: by_part,
                sources: sources.clone(),
                ..Params::default()
            };
            let change = MergedToolpath::run(&doc, &p).unwrap();
            let mut after = doc.clone();
            after.apply(&change).unwrap();
            let t = after
                .toolpaths
                .into_values()
                .find(|t| t.strategy == MergedToolpath::ID)
                .unwrap();
            let xs: Vec<f64> = starts(&t).into_iter().map(|(x, _)| x).collect();
            assert_eq!(xs, [5.0, 0.0, 105.0, 100.0], "by part: {by_part}");
        }
    }

    #[test]
    fn ungroup_takes_the_merged_toolpath_away_and_shows_its_sources() {
        let mut doc = two();
        let change = MergedToolpath::run(&doc, &Params::default()).unwrap();
        doc.apply(&change).unwrap();
        let merged = *doc
            .toolpaths
            .iter()
            .find(|(_, t)| t.strategy == MergedToolpath::ID)
            .map(|(id, _)| id)
            .unwrap();
        doc.toolpaths.get_mut(&EntityId(1)).unwrap().visible = false;
        let p = UngroupParams { toolpath: merged };
        cut_craft_core::testing::run_on::<UngroupMergedToolpath>(&mut doc, &p);
        assert!(!doc.toolpaths.contains_key(&merged));
        assert!(
            doc.toolpaths[&EntityId(1)].visible,
            "a hidden source is shown again"
        );
        assert_eq!(doc.toolpaths.len(), 2);
        let not_merged = UngroupParams {
            toolpath: EntityId(1),
        };
        let why = UngroupMergedToolpath::run(&doc, &not_merged)
            .unwrap_err()
            .to_string();
        assert!(why.contains("not a merged toolpath"), "{why}");
    }
}
