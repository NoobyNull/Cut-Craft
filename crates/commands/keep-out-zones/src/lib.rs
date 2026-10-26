//! Keep-Out Zones: areas a tool must not pass over, made from the selected closed vectors
//! grown by a clearance, or cleared (Aspire's help: Keep-out Zones). Once made they do not
//! follow the vectors; a toolpath that passes over one is warned about.

use cut_craft_core::document::Document;
use cut_craft_core::polygon::{Corners, offset};
use cut_craft_core::{Change, Command, CommandError, Meta};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct KeepOutZones;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    /// The zones become the selected closed vectors, grown by the clearance
    #[default]
    #[schemars(title = "Create from Selection")]
    Create,
    /// No zones
    #[schemars(title = "Clear Zones")]
    Clear,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    #[schemars(extend("x-widget" = "row"))]
    pub action: Action,
    /// How far beyond the vectors the zones reach
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub clearance: f64,
}

impl Command for KeepOutZones {
    const ID: &'static str = "keep_out_zones";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Keep-Out Zones",
            group: "Toolpaths",
            icon: "",
            shortcut: "",
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        let zones = match p.action {
            Action::Clear => vec![],
            Action::Create => {
                let closed: Vec<_> = doc
                    .selection
                    .iter()
                    .filter_map(|id| doc.vectors.get(id))
                    .filter(|v| v.closed)
                    .cloned()
                    .collect();
                if closed.is_empty() {
                    return Err(CommandError::Failed(
                        "select closed vectors to make the zones from".into(),
                    ));
                }
                if p.clearance > 0.0 {
                    offset(&closed, p.clearance, Corners::Round)
                } else {
                    closed
                }
            }
        };
        let mut c = Change::builder(doc);
        c.set_keep_out(zones);
        Ok(c.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::testing::{run_on, sample_document};

    cut_craft_core::contract_tests!(KeepOutZones);

    #[test]
    fn zones_grow_by_the_clearance_and_stay_when_the_vectors_go() {
        let mut doc = sample_document();
        let selected: Vec<_> = doc.selection.iter().copied().collect();
        let first = doc.vectors[&selected[0]].bounds();
        run_on::<KeepOutZones>(
            &mut doc,
            &Params {
                action: Action::Create,
                clearance: 2.0,
            },
        );
        assert_ne!(doc.keep_out, [] as [cut_craft_core::geometry::Path; 0]);
        let grown = doc
            .keep_out
            .iter()
            .map(cut_craft_core::geometry::Path::bounds)
            .find(|b| b.min.x < first.min.x);
        let grown = grown.expect("a zone round the first vector");
        assert!(
            (first.min.x - grown.min.x - 2.0).abs() < 0.1,
            "{grown:?} vs {first:?}"
        );
        let mut c = Change::builder(&doc);
        for id in selected {
            c.remove(id);
        }
        doc.apply(&c.finish()).unwrap();
        assert!(!doc.keep_out.is_empty(), "not linked to the vectors");
        run_on::<KeepOutZones>(
            &mut doc,
            &Params {
                action: Action::Clear,
                clearance: 0.0,
            },
        );
        assert_eq!(doc.keep_out, [] as [cut_craft_core::geometry::Path; 0]);
    }
}
