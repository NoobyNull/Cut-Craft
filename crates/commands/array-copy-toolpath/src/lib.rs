//! Array Copy Toolpath: a grid of copies of the source toolpaths, above and to the right of
//! them, kept up to date with them: recalculating copies them again.

use cut_craft_core::document::{Document, EntityId};
use cut_craft_core::geometry::{Point, Rect};
use cut_craft_core::toolpath::{Move, MoveKind, Toolpath, made_by};
use cut_craft_core::{Change, Command, CommandError, Meta};
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
    pub name: String,
    /// The toolpaths to copy (none: the visible ones)
    pub sources: Vec<EntityId>,
    #[schemars(range(min = 1))]
    pub rows: u32,
    #[schemars(range(min = 1))]
    pub columns: u32,
    pub spacing: Spacing,
    #[schemars(extend("x-unit" = "mm"))]
    pub x: f64,
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

impl Command for ArrayCopyToolpath {
    const ID: &'static str = "array_copy_toolpath";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Array Copy Toolpath",
            group: "Toolpath Operations",
            icon: "toolpaths/array_copy_toolpath.svg",
            shortcut: "",
        }
    }

    fn initial_params(doc: &Document) -> Params {
        Params {
            sources: doc
                .toolpaths
                .iter()
                .filter(|(_, t)| t.visible && t.strategy != Self::ID)
                .map(|(&id, _)| id)
                .collect(),
            ..Params::default()
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        if p.rows == 0 || p.columns == 0 {
            return Err(CommandError::InvalidParams(
                "it needs at least 1 row and 1 column".into(),
            ));
        }
        let named = if p.sources.is_empty() {
            Self::initial_params(doc).sources
        } else {
            p.sources.clone()
        };
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
                "the toolpaths to copy have no cuts".into(),
            ));
        };
        let size = bounds.max - bounds.min;
        let step = match p.spacing {
            Spacing::Offset => Point::new(p.x, p.y),
            Spacing::Gap => size + Point::new(p.x, p.y),
        };
        let mut c = Change::builder(doc);
        // ponytail: one array toolpath per source, so each tool's copies run together (Aspire's
        // "minimize tool changes"); copy by copy would need a toolpath per copy and source
        for (id, source) in sources
            .iter()
            .filter(|(id, _)| p.only.is_none_or(|o| o == *id))
        {
            let mut moves: Vec<Move> = vec![];
            let mut power = vec![];
            for row in 0..p.rows {
                for column in 0..p.columns {
                    let by =
                        Point::new(f64::from(column) * step.x, f64::from(row) * step.y).extend(0.0);
                    moves.extend(source.moves.iter().map(|m| Move {
                        kind: m.kind,
                        to: m.to + by,
                    }));
                    power.extend_from_slice(&source.power);
                }
            }
            let one = Params {
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
            c.add_toolpath(toolpath);
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
    fn the_form_starts_with_the_visible_toolpaths() {
        assert_eq!(
            ArrayCopyToolpath::initial_params(&with_profile()).sources,
            [EntityId(1)]
        );
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
    fn needs_a_toolpath_to_copy() {
        let why = ArrayCopyToolpath::run(&Document::default(), &Params::default())
            .unwrap_err()
            .to_string();
        assert!(why.contains("toolpaths to copy"), "{why}");
    }
}
