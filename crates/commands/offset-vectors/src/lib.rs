//! Offset Vectors.

use cut_craft_core::document::Document;
use cut_craft_core::polygon::{self, Corners};
use cut_craft_core::{Change, Command, CommandError, Meta};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct OffsetVectors;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    /// Outwards when positive, inwards when negative
    #[schemars(extend("x-unit" = "mm"))]
    pub distance: f64,
    pub corners: Corners,
    /// Keep the original vectors as well as the offset ones
    pub keep_original: bool,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            distance: 5.0,
            corners: Corners::Round,
            keep_original: true,
        }
    }
}

impl Command for OffsetVectors {
    const ID: &'static str = "offset_vectors";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Offset Vectors",
            group: "Offset and Layout",
            icon: "drawing/weld_and_offset/offset_vectors.png",
            shortcut: "",
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        if p.distance == 0.0 {
            return Err(CommandError::InvalidParams("distance must not be 0".into()));
        }
        let (ids, paths) = doc.selected_paths(|p| p.closed);
        if ids.is_empty() {
            return Err(CommandError::Failed(
                "select closed vectors to offset".into(),
            ));
        }
        let result = polygon::offset(&paths, p.distance, p.corners);
        if result.is_empty() {
            return Err(CommandError::Failed(
                "the inward offset left nothing: try a smaller distance".into(),
            ));
        }
        Ok(Change::swap(
            doc,
            if p.keep_original { vec![] } else { ids },
            result,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use cut_craft_core::testing::{document_with, square};

    fn offset(p: &Params) -> Result<Document, String> {
        let mut doc = document_with(vec![square(0.0, 0.0, 10.0)]);
        let change = OffsetVectors::run(&doc, p).map_err(|e| e.to_string())?;
        doc.apply(&change).unwrap();
        Ok(doc)
    }

    #[test]
    fn outwards_adds_a_larger_outline_and_keeps_the_original() {
        let doc = offset(&Params {
            distance: 1.0,
            corners: Corners::Sharp,
            keep_original: true,
        })
        .unwrap();
        assert_eq!(doc.vectors.len(), 2);
        let selected = &doc.vectors[doc.selection.iter().next().unwrap()];
        assert!((cut_craft_core::polygon::area(selected) - 144.0).abs() < 1e-3);
    }

    #[test]
    fn can_replace_the_original() {
        let doc = offset(&Params {
            keep_original: false,
            ..Params::default()
        })
        .unwrap();
        assert_eq!(doc.vectors.len(), 1);
    }

    #[test]
    fn too_far_inwards_is_reported() {
        let err = offset(&Params {
            distance: -6.0,
            ..Params::default()
        })
        .unwrap_err();
        assert_eq!(
            err,
            "the inward offset left nothing: try a smaller distance"
        );
    }

    #[test]
    fn zero_distance_is_rejected() {
        assert_eq!(
            offset(&Params {
                distance: 0.0,
                ..Params::default()
            })
            .unwrap_err(),
            "invalid parameters: distance must not be 0"
        );
    }

    cut_craft_core::contract_tests!(OffsetVectors);
}
