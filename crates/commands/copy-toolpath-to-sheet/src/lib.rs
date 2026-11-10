//! Copy Toolpath to Sheet: a toolpath onto another sheet, with copies of the vectors it was
//! made from, so it can be recalculated there.

use cut_craft_core::document::{Document, EntityId, StoredVector};
use cut_craft_core::{Change, Command, CommandError, Meta};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct CopyToolpathToSheet;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    pub toolpath: EntityId,
    /// The sheet's place in the Sheets list, from 0.
    pub sheet: usize,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            toolpath: EntityId(1),
            sheet: 0,
        }
    }
}

impl Command for CopyToolpathToSheet {
    const ID: &'static str = "copy_toolpath_to_sheet";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Copy Toolpath to Sheet",
            group: "Sheets",
            icon: "",
            shortcut: "",
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        let toolpath = cut_craft_core::existing(&doc.toolpaths, p.toolpath, "toolpath")?;
        if p.sheet >= doc.sheets.len() || p.sheet == doc.active_sheet {
            return Err(CommandError::InvalidParams(
                "pick another sheet to copy the toolpath to".into(),
            ));
        }
        let mut sheets = doc.sheets.clone();
        let objects = &mut sheets[p.sheet].objects;
        // ids are unique over every sheet
        let mut next = doc.next_id().0;
        let mut fresh = || {
            next += 1;
            EntityId(next - 1)
        };
        let mut copy = toolpath.clone();
        copy.vectors = toolpath
            .vectors
            .iter()
            .filter_map(|id| {
                let path = doc.vectors.get(id)?.clone();
                let new = fresh();
                objects.vectors.insert(
                    new,
                    StoredVector {
                        path,
                        layer: Some(doc.layer_of(*id)),
                        group: doc.vector_groups.get(id).copied(),
                    },
                );
                Some(new)
            })
            .collect();
        objects.toolpaths.insert(fresh(), copy);
        let mut c = Change::builder(doc);
        c.set_sheets(sheets, doc.active_sheet);
        Ok(c.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::document::Sheet;
    use cut_craft_core::geometry::Point;
    use cut_craft_core::toolpath::{Tool, Toolpath};

    /// Toolpath 1, made from vector 2, on the second sheet; the first sheet empty.
    fn on_two_sheets() -> Document {
        let mut doc = Document::default();
        let mut c = Change::builder(&doc);
        c.add_toolpath(Toolpath {
            name: "Profile".into(),
            strategy: "profile_toolpath".into(),
            params: serde_json::Value::Null,
            tool: Tool::default(),
            vectors: vec![EntityId(2)],
            moves: vec![],
            visible: true,
            power: vec![],
            stale: false,
            group: None,
            drill_bank: None,
        });
        c.add(cut_craft_core::geometry::Path::polyline(
            &[Point::new(0.0, 0.0), Point::new(10.0, 0.0)],
            false,
        ));
        let mut sheets = vec![Sheet::new("Empty", doc.job)];
        sheets.extend(doc.sheets.clone());
        c.set_sheets(sheets, 1);
        doc.apply(&c.finish()).unwrap();
        doc
    }

    cut_craft_core::contract_tests!(CopyToolpathToSheet, on_two_sheets);

    #[test]
    fn the_toolpath_and_its_vectors_arrive_on_the_sheet() {
        let mut doc = on_two_sheets();
        doc.apply(&CopyToolpathToSheet::run(&doc, &Params::default()).unwrap())
            .unwrap();
        assert_eq!(doc.toolpaths.len(), 1, "the original stays");
        let mut c = Change::builder(&doc);
        c.activate_sheet(0);
        doc.apply(&c.finish()).unwrap();
        let (_, t) = doc.toolpaths.iter().next().unwrap();
        assert_eq!(t.name, "Profile");
        assert_eq!(t.vectors.len(), 1);
        assert!(
            doc.vectors.contains_key(&t.vectors[0]),
            "made from the copy there"
        );
        assert_ne!(t.vectors[0], EntityId(2));
        let refused = CopyToolpathToSheet::run(
            &doc,
            &Params {
                sheet: doc.active_sheet,
                ..Params::default()
            },
        );
        assert!(refused.is_err());
    }
}
