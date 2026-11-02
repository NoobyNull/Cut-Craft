//! Name Part: gives the selected parts a name, which the cut list and the part labels print.
//! A part is a group or a closed shape with what lies inside it (as the cut list finds
//! parts); its outline's group carries the name, the outline alone getting a group of its
//! own when it has none. Selecting several parts names them all (a cabinet's two sides are
//! both "Side"); an empty name takes the names away.

use cut_craft_core::document::{Document, EntityId};
use cut_craft_core::{Change, Command, CommandError, Meta};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct NamePart;

#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    /// What the cut list and the part labels call the selected parts, such as "Left side"
    /// or "Shelf". Leave it empty to take the name away.
    #[schemars(title = "Part name")]
    pub name: String,
}

/// The selected parts, each as its outline (its biggest closed vector) and its vectors.
fn selected(doc: &Document) -> Vec<(EntityId, Vec<EntityId>)> {
    let ids: Vec<EntityId> = doc
        .selection
        .iter()
        .flat_map(|&id| doc.group_members(id))
        .collect();
    fmt_cut_list::clusters(doc, &ids)
        .into_iter()
        .filter_map(|(members, _)| {
            let outline = members
                .iter()
                .copied()
                .filter(|m| doc.vectors[m].closed)
                .max_by(|a, b| {
                    let size = |id: &EntityId| {
                        let s = doc.vectors[id].bounds().size();
                        s.x * s.y
                    };
                    size(a).total_cmp(&size(b))
                })?;
            Some((outline, members))
        })
        .collect()
}

impl Command for NamePart {
    const ID: &'static str = "name_part";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Name Part",
            group: "Offset and Layout",
            icon: "drawing/weld_and_offset/name_part.png",
            shortcut: "",
        }
    }

    /// The first selected part's name, so the form shows it.
    fn initial_params(doc: &Document) -> Params {
        let name = selected(doc)
            .first()
            .and_then(|(outline, _)| doc.vector_groups.get(outline))
            .and_then(|g| doc.part_names.get(g))
            .cloned()
            .unwrap_or_default();
        Params { name }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        if doc.selection.is_empty() {
            return Err(CommandError::NothingSelected);
        }
        let parts = selected(doc);
        if parts.is_empty() {
            return Err(CommandError::Failed(
                "Select a closed shape to name. Open lines are not parts.".into(),
            ));
        }
        let name = p.name.trim();
        let mut c = Change::builder(doc);
        let mut next = doc.next_group();
        for (outline, members) in parts {
            let group = doc.vector_groups.get(&outline).copied().unwrap_or_else(|| {
                c.set_group(outline, Some(next));
                next += 1;
                next - 1
            });
            // only the outline's group names the part
            for g in members.iter().filter_map(|m| doc.vector_groups.get(m)) {
                if *g != group {
                    c.set_part_name(*g, None);
                }
            }
            c.set_part_name(group, (!name.is_empty()).then(|| name.to_owned()));
        }
        Ok(c.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::testing::{document_with, run_on, square};

    fn named(name: &str) -> Params {
        Params { name: name.into() }
    }

    #[test]
    fn selected_parts_are_named_and_the_cut_list_prints_it() {
        let mut doc = document_with(vec![
            square(0.0, 0.0, 100.0),
            square(10.0, 10.0, 20.0),
            square(200.0, 0.0, 100.0),
        ]);
        run_on::<NamePart>(&mut doc, &named("  Side "));
        let found = fmt_cut_list::parts(&doc);
        assert_eq!(found.len(), 2);
        assert!(found.iter().all(|p| p.name.as_deref() == Some("Side")));
        let rows = fmt_cut_list::rows(&found);
        assert_eq!((rows.len(), rows[0].quantity), (1, 2));
        // the form opens on the name, and an empty name takes it away
        assert_eq!(NamePart::initial_params(&doc).name, "Side");
        run_on::<NamePart>(&mut doc, &named(""));
        assert!(fmt_cut_list::parts(&doc).iter().all(|p| p.name.is_none()));
    }

    #[test]
    fn open_lines_alone_are_not_parts_and_nothing_selected_says_so() {
        let p = cut_craft_core::geometry::Point::new;
        let doc = document_with(vec![cut_craft_core::geometry::Path::polyline(
            &[p(0.0, 0.0), p(10.0, 0.0)],
            false,
        )]);
        let e = NamePart::run(&doc, &named("A")).unwrap_err().to_string();
        assert!(e.contains("closed shape"), "{e}");
        let mut none = doc.clone();
        none.selection.clear();
        assert!(matches!(
            NamePart::run(&none, &named("A")),
            Err(CommandError::NothingSelected)
        ));
    }

    cut_craft_core::contract_tests!(NamePart);
}
