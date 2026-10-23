use aspire_core::document::Document;
use aspire_core::geometry::{Path, Point};
use aspire_core::{Change, Command, CommandError, Meta, Registry};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Test command: adds a square of the given size at the origin.
struct AddSquare;

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
struct SquareParams {
    /// Side length
    size: f64,
}

impl Default for SquareParams {
    fn default() -> Self {
        Self { size: 10.0 }
    }
}

impl Command for AddSquare {
    const ID: &'static str = "add_square";
    type Params = SquareParams;

    fn meta() -> Meta {
        Meta {
            label: "Add Square",
            group: "Test",
            icon: "",
            shortcut: "",
        }
    }

    fn run(doc: &Document, p: &SquareParams) -> Result<Change, CommandError> {
        if p.size <= 0.0 {
            return Err(CommandError::InvalidParams("size must be positive".into()));
        }
        let s = p.size;
        let mut c = Change::builder(doc);
        c.add(Path::polyline(
            &[
                Point::ZERO,
                Point::new(s, 0.0),
                Point::new(s, s),
                Point::new(0.0, s),
            ],
            true,
        ));
        Ok(c.finish())
    }
}

aspire_core::contract_tests!(AddSquare);

#[test]
fn registry_runs_a_command_from_json_params() {
    let mut registry = Registry::default();
    registry.add::<AddSquare>();
    let cmd = registry.get("add_square").unwrap();
    let change = cmd
        .run_json(&Document::default(), &serde_json::json!({ "size": 2.0 }))
        .unwrap();
    let mut doc = Document::default();
    doc.apply(&change).unwrap();
    assert_eq!(doc.vectors.len(), 1);
}

#[test]
fn bad_json_params_are_reported_not_panicked() {
    let mut registry = Registry::default();
    registry.add::<AddSquare>();
    let err = registry
        .get("add_square")
        .unwrap()
        .run_json(&Document::default(), &serde_json::json!({ "size": "big" }));
    assert!(matches!(err, Err(CommandError::InvalidParams(_))));
}

#[test]
fn command_validation_errors_come_back_through_run_json() {
    let mut registry = Registry::default();
    registry.add::<AddSquare>();
    let err = registry
        .get("add_square")
        .unwrap()
        .run_json(&Document::default(), &serde_json::json!({ "size": -1.0 }));
    assert_eq!(
        err.unwrap_err().to_string(),
        "invalid parameters: size must be positive"
    );
}

#[test]
fn schema_exposes_parameter_docs_for_form_generation() {
    let mut registry = Registry::default();
    registry.add::<AddSquare>();
    let schema = registry.get("add_square").unwrap().schema();
    assert_eq!(schema["properties"]["size"]["description"], "Side length");
}

#[test]
#[should_panic(expected = "registered twice")]
fn registering_the_same_id_twice_is_a_bug() {
    let mut registry = Registry::default();
    registry.add::<AddSquare>();
    registry.add::<AddSquare>();
}
