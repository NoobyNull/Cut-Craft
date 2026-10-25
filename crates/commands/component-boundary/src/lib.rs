//! Create Vector Boundary Around Selected Components: closed vectors round the silhouette of
//! the selected components (all visible ones when none is selected), grouped when there are
//! several, for use as toolpath boundaries.

use cut_craft_core::document::{Document, EntityId};
use cut_craft_core::geometry::Rect;
use cut_craft_core::trace::{self, Fit, Grid};
use cut_craft_core::{Change, Command, CommandError, Meta, NoParams};

pub struct ComponentBoundary;

impl Command for ComponentBoundary {
    const ID: &'static str = "component_boundary";
    type Params = NoParams;

    fn meta() -> Meta {
        Meta {
            label: "Create Vector Boundary Around Selected Components",
            group: "Modeling",
            icon: "",
            shortcut: "",
        }
    }

    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss,
        reason = "cell counts are small and non-negative"
    )]
    fn run(doc: &Document, _: &NoParams) -> Result<Change, CommandError> {
        let mut ids: Vec<EntityId> = doc
            .selected_components
            .iter()
            .copied()
            .filter(|id| doc.components.contains_key(id))
            .collect();
        if ids.is_empty() {
            ids = doc
                .components
                .iter()
                .filter(|(_, c)| c.visible)
                .map(|(id, _)| *id)
                .collect();
        }
        let parts: Vec<_> = ids.iter().map(|id| &doc.components[id]).collect();
        let Some(area) = parts.iter().map(|c| c.extent()).reduce(Rect::union) else {
            return Err(CommandError::Failed(
                "select components to draw a boundary round".into(),
            ));
        };
        let cell = parts.iter().map(|c| c.cell).fold(f64::INFINITY, f64::min);
        let grid = Grid {
            columns: (area.size().x / cell).ceil() as usize,
            rows: (area.size().y / cell).ceil() as usize,
            origin: area.min,
            cell,
        };
        let centre = |c: usize, r: usize| {
            area.min + cut_craft_core::geometry::Point::new(c as f64 + 0.5, r as f64 + 0.5) * cell
        };
        let rings = trace::outlines(grid, |c, r| {
            parts
                .iter()
                .any(|part| part.height_at(centre(c, r)).is_some())
        });
        // ponytail: arcs within half a cell, corners over 60° kept [inf]; Aspire's own smoothing
        // is not described
        let paths: Vec<_> = rings
            .iter()
            .map(|ring| trace::fit(ring, true, Fit::Arcs, cell / 2.0, 60_f64.to_radians()))
            .collect();
        if paths.is_empty() {
            return Err(CommandError::Failed("the components have no shape".into()));
        }
        let group = (paths.len() > 1).then(|| doc.next_group());
        let mut c = Change::builder(doc);
        let added: Vec<_> = paths.into_iter().map(|p| c.add(p)).collect();
        for &id in &added {
            c.set_group(id, group);
        }
        c.select(added);
        Ok(c.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::geometry::Point;
    use cut_craft_core::testing::{domed, run_on, two_components};

    cut_craft_core::contract_tests!(ComponentBoundary, domed);

    #[test]
    fn the_dome_gets_a_circle_round_its_silhouette() {
        let mut doc = domed();
        run_on::<ComponentBoundary>(&mut doc, &NoParams {});
        assert_eq!(doc.vectors.len(), 1);
        let path = doc.vectors.values().next().unwrap();
        assert!(path.closed);
        for q in path.flatten(0.01) {
            let r = q.distance(Point::new(20.0, 20.0));
            assert!((r - 8.0).abs() < 0.4, "{q} is {r} from the middle");
        }
    }

    #[test]
    fn separate_components_give_a_group_and_selection_limits_it() {
        let mut doc = two_components();
        // move the second dome clear of the first
        let mut moved = doc.components[&EntityId(2)].clone();
        moved.origin.x += 20.0;
        doc.components.insert(EntityId(2), moved);
        let mut all = doc.clone();
        run_on::<ComponentBoundary>(&mut all, &NoParams {});
        assert_eq!(all.vectors.len(), 2);
        let first = *all.vectors.keys().next().unwrap();
        assert_eq!(all.group_members(first).len(), 2, "grouped");

        doc.selected_components.insert(EntityId(2));
        run_on::<ComponentBoundary>(&mut doc, &NoParams {});
        assert_eq!(doc.vectors.len(), 1, "only the selected one");
        let b = doc.vectors.values().next().unwrap().bounds();
        assert!(b.min.x > 30.0);
    }

    #[test]
    fn needs_a_component() {
        assert!(
            cut_craft_core::testing::rejection::<ComponentBoundary>(&NoParams {})
                .contains("select")
        );
    }
}
