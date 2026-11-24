//! Array Copy Toolpath: a grid of copies of the toolpaths shown in the toolpath list when
//! Apply is pressed (tick the ones to copy there), above and to the right of them, kept up to
//! date with them: recalculating copies them again.

use cut_craft_core::document::{Document, EntityId};
use cut_craft_core::geometry::{Point, Rect};
use cut_craft_core::toolpath::{Move, MoveKind, Toolpath, made_by};
use cut_craft_core::{Change, Command, CommandError, Form, Meta};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct ArrayCopyToolpath;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Spacing {
    /// X and Y are from each copy's start to the next one's
    #[default]
    Offset,
    /// X and Y are the gaps between the copies' extents
    Gap,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    /// What the toolpath is called in the toolpath list. It copies the toolpaths ticked
    /// (shown) in the toolpath list when Apply is pressed: tick them there first
    /// (it is marked out of date when one of them changes: recalculate it then, and the
    /// hidden ones out of date are recalculated first). Those
    /// toolpaths are then hidden, so saving the shown toolpaths does not cut them twice
    pub name: String,
    /// The toolpaths to copy (none: the ones shown in the toolpath list when Apply is
    /// pressed). Not on the form: tick the toolpaths to copy in the toolpath list
    #[schemars(skip)]
    #[serde(default)]
    pub sources: Vec<EntityId>,
    /// How many copies up the job, counting the first. Copies that would go past the
    /// material's edge are still made, with a warning saying how many rows and columns fit
    #[schemars(range(min = 1))]
    pub rows: u32,
    /// How many copies across the job, counting the first
    #[schemars(range(min = 1))]
    pub columns: u32,
    /// How the X and Y distances are measured
    pub spacing: Spacing,
    /// The distance between copies across the job
    #[schemars(extend("x-unit" = "mm"))]
    pub x: f64,
    /// The distance between copies up the job
    #[schemars(extend("x-unit" = "mm"))]
    pub y: f64,
    /// Set on each array toolpath made: the one source it copies
    #[serde(default)]
    pub only: Option<EntityId>,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            name: "Array Copy".into(),
            sources: vec![],
            rows: 2,
            columns: 2,
            spacing: Spacing::Gap,
            x: 10.0,
            y: 10.0,
            only: None,
        }
    }
}

/// The XY extent of a toolpath's feed moves.
fn extent(t: &Toolpath) -> Option<Rect> {
    t.moves
        .iter()
        .filter(|m| m.kind != MoveKind::Rapid)
        .map(|m| Rect::point(m.to.truncate()))
        .reduce(Rect::union)
}

/// The toolpaths shown in the toolpath list, but none made by `strategy`.
fn visible(doc: &Document, strategy: &str) -> Vec<EntityId> {
    doc.toolpaths
        .iter()
        .filter(|(_, t)| t.visible && t.strategy != strategy)
        .map(|(&id, _)| id)
        .collect()
}

/// What to say when copies of `bounds`, `step` apart, run off the material: how many rows
/// and columns fit. A cut's centre may go `slack` past the edge (a tool radius, as a part
/// cut out right at the edge does).
fn past_edge(
    doc: &Document,
    p: &Params,
    (bounds, step): (Rect, Point),
    slack: f64,
) -> Option<String> {
    if doc.job.rotary.is_some() {
        return None;
    }
    let area = doc.job.area();
    let fit = |count: u32, low: f64, high: f64, step: f64, min: f64, max: f64| {
        (0..count)
            .take_while(|&i| {
                let by = f64::from(i) * step;
                low + by >= min - slack && high + by <= max + slack
            })
            .count()
    };
    let columns = fit(
        p.columns,
        bounds.min.x,
        bounds.max.x,
        step.x,
        area.min.x,
        area.max.x,
    );
    let rows = fit(
        p.rows,
        bounds.min.y,
        bounds.max.y,
        step.y,
        area.min.y,
        area.max.y,
    );
    let wanted = (p.columns as usize, p.rows as usize);
    (columns < wanted.0 || rows < wanted.1).then(|| {
        let plural = |n: usize, what: &str| {
            if n == 1 {
                format!("1 {what}")
            } else {
                format!("{n} {what}s")
            }
        };
        format!(
            "Some copies go past the edge of the material: {} and {} fit. Use fewer, or \
             less space between them",
            plural(columns, "column"),
            plural(rows, "row")
        )
    })
}

impl Command for ArrayCopyToolpath {
    const ID: &'static str = "array_copy_toolpath";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Array Copy Toolpath",
            group: "Toolpath Operations",
            icon: "toolpaths/array_copy_toolpath.png",
            shortcut: "",
            form: Form::Toolpath,
        }
    }

    fn initial_params(_: &Document) -> Params {
        // no sources: the toolpaths shown when Apply is pressed, not when the form opened
        Params::default()
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        if p.rows == 0 || p.columns == 0 {
            return Err(CommandError::InvalidParams(
                "set rows and columns to 1 or more".into(),
            ));
        }
        let mut named = if p.sources.is_empty() {
            visible(doc, Self::ID)
        } else {
            p.sources.clone()
        };
        // a toolpath named twice is used once, not cut twice
        let mut seen = std::collections::BTreeSet::new();
        named.retain(|id| seen.insert(*id));
        let sources: Vec<(EntityId, &Toolpath)> = named
            .iter()
            .filter_map(|id| doc.toolpaths.get(id).map(|t| (*id, t)))
            .collect();
        if sources.is_empty() {
            return Err(CommandError::Failed(
                "show the toolpaths to copy in the toolpath list".into(),
            ));
        }
        let Some(bounds) = sources
            .iter()
            .filter_map(|(_, t)| extent(t))
            .reduce(Rect::union)
        else {
            return Err(CommandError::Failed(
                "the toolpaths to copy have no cuts. Edit them and press Calculate first".into(),
            ));
        };
        let size = bounds.max - bounds.min;
        let step = match p.spacing {
            Spacing::Offset => Point::new(p.x, p.y),
            Spacing::Gap => size + Point::new(p.x, p.y),
        };
        let mut c = Change::builder(doc);
        let slack = sources
            .iter()
            .map(|(_, t)| cut_craft_core::checks::edge_allowance(t))
            .fold(0.0, f64::max);
        if let Some(note) = past_edge(doc, p, (bounds, step), slack) {
            c.note(note);
        }
        // ponytail: one array toolpath per source, so each tool's copies run together (the
        // "minimize tool changes"); copy by copy would need a toolpath per copy and source
        for (id, source) in sources
            .iter()
            .filter(|(id, _)| p.only.is_none_or(|o| o == *id))
        {
            let mut moves: Vec<Move> = vec![];
            let mut power = vec![];
            // where each copy is, for a drill bank's plunges
            let mut placed = vec![];
            for row in 0..p.rows {
                for column in 0..p.columns {
                    let by =
                        Point::new(f64::from(column) * step.x, f64::from(row) * step.y).extend(0.0);
                    placed.push((by.truncate(), moves.len()));
                    moves.extend(source.moves.iter().map(|m| Move {
                        kind: m.kind,
                        to: m.to + by,
                    }));
                    power.extend_from_slice(&source.power);
                }
            }
            // saved with the toolpaths it copied, so hiding them (the grid counts the
            // original as its first copy) does not change what recalculating copies
            let one = Params {
                sources: named.clone(),
                only: Some(*id),
                ..p.clone()
            };
            let mut toolpath = made_by::<Self>(
                &one,
                format!("{} - {}", p.name, source.name),
                source.tool.clone(),
                vec![],
                moves,
            );
            toolpath.power = power;
            toolpath.drill_bank = source.drill_bank.as_ref().map(|b| b.copied(&placed));
            c.add_toolpath(toolpath);
        }
        // the sources are under the new toolpath: hidden, so the default save (every shown
        // toolpath) does not cut them twice. Recalculating leaves them as they are
        if p.sources.is_empty() {
            c.hide_toolpaths(&named);
        }
        Ok(c.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::testing::with_toolpath;
    use cut_craft_core::toolpath::Tool;

    /// A document with a profile cut round a 10 mm square at the origin.
    fn with_profile() -> Document {
        let mut doc = with_toolpath();
        let at = |kind, x: f64, y: f64, z: f64| Move {
            kind,
            to: cut_craft_core::glam::DVec3::new(x, y, z),
        };
        let mut t = doc.toolpaths[&EntityId(1)].clone();
        t.moves = vec![
            at(MoveKind::Rapid, 0.0, 0.0, 5.0),
            at(MoveKind::Plunge, 0.0, 0.0, -1.0),
            at(MoveKind::Cut, 10.0, 0.0, -1.0),
            at(MoveKind::Cut, 10.0, 10.0, -1.0),
            at(MoveKind::Rapid, 10.0, 10.0, 5.0),
        ];
        let mut c = Change::builder(&doc);
        c.replace_toolpath(EntityId(1), t);
        let change = c.finish();
        doc.apply(&change).unwrap();
        doc
    }

    fn array(doc: &Document, p: &Params) -> Vec<Toolpath> {
        let change = ArrayCopyToolpath::run(doc, p).unwrap();
        let mut after = doc.clone();
        after.apply(&change).unwrap();
        after
            .toolpaths
            .into_values()
            .filter(|t| t.strategy == ArrayCopyToolpath::ID)
            .collect()
    }

    fn with_sources() -> Document {
        with_profile()
    }

    cut_craft_core::contract_tests!(ArrayCopyToolpath, with_sources);

    #[test]
    fn a_drill_banks_plunges_are_copied_with_its_moves() {
        use cut_craft_core::drill_bank::{BankHit, BankToolpath, DrillBank};
        let mut doc = with_profile();
        let mut t = doc.toolpaths[&EntityId(1)].clone();
        t.drill_bank = Some(BankToolpath {
            bank: DrillBank::new(1, 0, 32.0, 5.0),
            hits: vec![BankHit {
                start: 0,
                end: t.moves.len(),
                head: Point::ZERO,
                drills: vec![],
            }],
        });
        let mut c = Change::builder(&doc);
        c.replace_toolpath(EntityId(1), t);
        doc.apply(&c.finish()).unwrap();
        let p = Params {
            sources: vec![EntityId(1)],
            ..Params::default()
        };
        let made = &array(&doc, &p)[0];
        let hits = &made.drill_bank.as_ref().expect("still a drill bank").hits;
        assert_eq!(hits.len(), usize::try_from(p.rows * p.columns).unwrap());
        for hit in hits {
            // each copy's head is where its first move is
            assert_eq!(hit.head, made.moves[hit.start].to.truncate());
        }
        assert_eq!(hits.last().unwrap().end, made.moves.len());
    }

    #[test]
    fn the_form_hides_the_sources_and_copies_the_toolpaths_visible_at_apply() {
        let doc = with_profile();
        let schema = schemars::schema_for!(Params);
        assert!(
            schema.as_value()["properties"].get("sources").is_none(),
            "no id list"
        );
        let name = schema.as_value()["properties"]["name"]["description"].as_str();
        assert!(
            name.is_some_and(|h| h.contains("ticked")),
            "the form says which toolpaths it copies: {name:?}"
        );
        assert_eq!(ArrayCopyToolpath::initial_params(&doc).sources, []);
        let once = array(&doc, &Params::default())[0].moves.len();
        let doubled = Params {
            sources: vec![EntityId(1), EntityId(1)],
            ..Params::default()
        };
        let made = array(&doc, &doubled);
        assert_eq!(made.len(), 1, "a repeated id is copied once");
        assert_eq!(made[0].moves.len(), once);
    }

    #[test]
    fn copies_go_up_and_right_by_the_gap_past_the_extent() {
        let doc = with_profile();
        let p = Params {
            sources: vec![EntityId(1)],
            ..Params::default()
        };
        let made = array(&doc, &p);
        assert_eq!(made.len(), 1);
        let plunges: Vec<Point> = made[0]
            .moves
            .iter()
            .filter(|m| m.kind == MoveKind::Plunge)
            .map(|m| m.to.truncate())
            .collect();
        assert_eq!(
            plunges,
            [
                Point::new(0.0, 0.0),
                Point::new(20.0, 0.0),
                Point::new(0.0, 20.0),
                Point::new(20.0, 20.0)
            ]
        );
        let offset = array(
            &doc,
            &Params {
                spacing: Spacing::Offset,
                x: 15.0,
                y: 30.0,
                ..p
            },
        );
        let plunges: Vec<Point> = offset[0]
            .moves
            .iter()
            .filter(|m| m.kind == MoveKind::Plunge)
            .map(|m| m.to.truncate())
            .collect();
        assert_eq!(plunges[3], Point::new(15.0, 30.0));
    }

    #[test]
    fn each_source_gets_its_own_array_with_its_tool() {
        let mut doc = with_profile();
        let mut second = doc.toolpaths[&EntityId(1)].clone();
        second.name = "Second".into();
        second.tool = Tool {
            name: "Other".into(),
            ..Tool::default()
        };
        let mut c = Change::builder(&doc);
        let id = c.add_toolpath(second);
        let change = c.finish();
        doc.apply(&change).unwrap();
        let made = array(
            &doc,
            &Params {
                sources: vec![EntityId(1), id],
                ..Params::default()
            },
        );
        assert_eq!(made.len(), 2);
        assert!(made.iter().any(|t| t.tool.name == "Other"));
        assert!(made.iter().all(|t| t.params["only"].is_number()));
    }

    #[test]
    fn a_saved_array_still_copies_its_source_after_the_source_is_hidden() {
        let mut doc = with_profile();
        let made = array(&doc, &Params::default());
        let saved: Params = serde_json::from_value(made[0].params.clone()).unwrap();
        // the grid counts the original as its first copy, so its source is usually hidden
        let mut hidden = doc.toolpaths[&EntityId(1)].clone();
        hidden.visible = false;
        let mut c = Change::builder(&doc);
        c.replace_toolpath(EntityId(1), hidden);
        doc.apply(&c.finish()).unwrap();
        let again = array(&doc, &saved);
        assert_eq!(again.len(), 1, "recalculating still makes the array");
        assert_eq!(again[0].moves, made[0].moves);
    }

    #[test]
    fn the_toolpaths_copied_are_hidden_so_a_save_does_not_cut_them_twice() {
        let doc = with_profile();
        let change = ArrayCopyToolpath::run(&doc, &Params::default()).unwrap();
        let mut after = doc.clone();
        after.apply(&change).unwrap();
        assert!(
            !after.toolpaths[&EntityId(1)].visible,
            "the first copy sits on the source"
        );
        // recalculating (sources saved) leaves a source shown again as it is
        let saved: Params = serde_json::from_value(
            after
                .toolpaths
                .values()
                .find(|t| t.strategy == ArrayCopyToolpath::ID)
                .unwrap()
                .params
                .clone(),
        )
        .unwrap();
        let again = ArrayCopyToolpath::run(&doc, &saved).unwrap();
        let mut shown = doc.clone();
        shown.apply(&again).unwrap();
        assert!(shown.toolpaths[&EntityId(1)].visible);
    }

    #[test]
    fn copies_past_the_material_edge_are_made_and_warned_of_with_what_fits() {
        // the material is 600 x 400: 10 mm parts 20 mm apart fit 30 across, 20 up
        let doc = with_profile();
        let p = Params {
            rows: 25,
            columns: 2,
            ..Params::default()
        };
        let change = ArrayCopyToolpath::run(&doc, &p).unwrap();
        let note = change.notes.join(" ");
        assert!(
            note.contains("edge of the material") && note.contains("20 rows"),
            "{note}"
        );
        let mut after = doc.clone();
        after.apply(&change).unwrap();
        let made = after
            .toolpaths
            .values()
            .find(|t| t.strategy == ArrayCopyToolpath::ID)
            .unwrap();
        let why = cut_craft_core::checks::problems(&after, made).join(" ");
        assert!(
            why.contains("edge of the material"),
            "checked on save: {why}"
        );
        let fits = ArrayCopyToolpath::run(&doc, &Params::default()).unwrap();
        assert!(fits.notes.is_empty(), "{:?}", fits.notes);
    }

    #[test]
    fn needs_a_toolpath_to_copy() {
        let why = ArrayCopyToolpath::run(&Document::default(), &Params::default())
            .unwrap_err()
            .to_string();
        assert!(why.contains("toolpaths to copy"), "{why}");
    }
}
