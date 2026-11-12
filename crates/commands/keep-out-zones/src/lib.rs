//! Keep-Out Zones: areas a tool must not pass over, made from the selected closed vectors
//! grown by a clearance, or cleared. Once made they do not
//! follow the vectors. Each is named ("Clamp 1"), its label in the views saying so. A
//! toolpath that would cut into one is not made, unless Options allow clipping it.

use cut_craft_core::document::Document;
use cut_craft_core::keep_out::{KeepOutZone, names};
use cut_craft_core::polygon::{Corners, offset};
use cut_craft_core::{Change, Command, CommandError, Meta};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct KeepOutZones;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    /// The zones become the selected closed vectors, grown by the clearance, in place of
    /// any there were
    #[default]
    #[schemars(title = "Create from Selection")]
    Create,
    /// The selected closed vectors, grown by the clearance, are added to the zones there are
    #[schemars(title = "Add Selection")]
    Add,
    /// All the keep-out zones are removed
    #[schemars(title = "Clear Zones")]
    Clear,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    /// What happens to the keep-out zones
    #[schemars(extend("x-widget" = "row"))]
    pub action: Action,
    /// What the zones are called on their labels: "Clamp" names one zone "Clamp" and
    /// several "Clamp 1", "Clamp 2" and so on
    #[serde(default = "clamp")]
    pub name: String,
    /// How far beyond the vectors the zones reach
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub clearance: f64,
}

fn clamp() -> String {
    "Clamp".into()
}

impl Default for Params {
    fn default() -> Self {
        Self {
            action: Action::default(),
            name: clamp(),
            clearance: 0.0,
        }
    }
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
            Action::Create | Action::Add => {
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
                let made = if p.clearance > 0.0 {
                    offset(&closed, p.clearance, Corners::Round)
                } else {
                    closed
                };
                let kept = if p.action == Action::Add {
                    doc.keep_out.clone()
                } else {
                    vec![]
                };
                // numbered on from the zones already called the same
                let base = p.name.trim();
                let same = kept
                    .iter()
                    .filter(|z| z.name == base || z.name.starts_with(&format!("{base} ")))
                    .count();
                let named = names(base, made.len(), same + 1);
                let mut zones = kept;
                zones.extend(
                    named
                        .into_iter()
                        .zip(made)
                        .map(|(name, path)| KeepOutZone::new(name, path)),
                );
                zones
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
                ..Params::default()
            },
        );
        assert_ne!(doc.keep_out, [] as [KeepOutZone; 0]);
        let grown = doc
            .keep_out
            .iter()
            .map(|z| z.path.bounds())
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
        assert_ne!(doc.keep_out.len(), 0, "not linked to the vectors");
        run_on::<KeepOutZones>(
            &mut doc,
            &Params {
                action: Action::Clear,
                ..Params::default()
            },
        );
        assert_eq!(doc.keep_out, [] as [KeepOutZone; 0]);
    }

    #[test]
    fn zones_are_named_and_numbered_on_when_added() {
        let mut doc = sample_document();
        let make = |action, name: &str| Params {
            action,
            name: name.into(),
            clearance: 0.0,
        };
        run_on::<KeepOutZones>(&mut doc, &make(Action::Create, "Clamp"));
        let count = doc.keep_out.len();
        assert!(count >= 2, "the sample selects several closed vectors");
        let named: Vec<_> = doc.keep_out.iter().map(|z| z.name.clone()).collect();
        assert_eq!(named[..2], ["Clamp 1", "Clamp 2"]);
        assert_eq!(doc.keep_out[0].label(), "Keep-out: Clamp 1");
        // added on, not in place of them, numbered after the ones already there
        run_on::<KeepOutZones>(&mut doc, &make(Action::Add, "Clamp"));
        assert_eq!(doc.keep_out.len(), 2 * count);
        assert_eq!(doc.keep_out[count].name, format!("Clamp {}", count + 1));
        // created again: in place of them
        run_on::<KeepOutZones>(&mut doc, &make(Action::Create, "Vise"));
        assert_eq!(doc.keep_out.len(), count);
        assert!(doc.keep_out.iter().all(|z| z.name.starts_with("Vise")));
    }
}
