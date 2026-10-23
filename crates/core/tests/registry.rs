use aspire_core::document::Document;
use aspire_core::geometry::{Path, Point};
use aspire_core::{Change, Command, CommandError, Drag, Interactive, Meta, Registry};
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

#[test]
fn ensure_positive_names_the_parameter() {
    assert_eq!(aspire_core::ensure_positive("width", 3.0), Ok(()));
    assert_eq!(
        aspire_core::ensure_positive("width", 0.0)
            .unwrap_err()
            .to_string(),
        "invalid parameters: width must be greater than 0"
    );
}

impl Interactive for AddSquare {
    fn from_drag(drag: Drag, _: &SquareParams) -> SquareParams {
        let d = (drag.end - drag.start).abs();
        SquareParams { size: d.x.max(d.y) }
    }
}

#[test]
fn interactive_commands_turn_a_drag_into_params() {
    let mut registry = Registry::default();
    registry.add_interactive::<AddSquare>();
    let cmd = registry.get("add_square").unwrap();
    assert!(cmd.interactive());
    let drag = Drag {
        start: Point::ZERO,
        end: Point::new(3.0, -4.0),
        from_center: false,
        constrain: false,
    };
    assert_eq!(
        cmd.drag_params(drag, &cmd.default_params()).unwrap(),
        serde_json::json!({ "size": 4.0 })
    );
}

#[test]
fn plain_commands_are_not_interactive() {
    let mut registry = Registry::default();
    registry.add::<AddSquare>();
    let cmd = registry.get("add_square").unwrap();
    assert!(!cmd.interactive());
    let drag = Drag {
        start: Point::ZERO,
        end: Point::ZERO,
        from_center: false,
        constrain: false,
    };
    assert!(cmd.drag_params(drag, &cmd.default_params()).is_err());
}

/// Test command whose form starts from the document: the number of vectors.
struct CountVectors;

#[derive(Clone, Default, Serialize, Deserialize, JsonSchema)]
struct CountParams {
    count: usize,
}

impl Command for CountVectors {
    const ID: &'static str = "count_vectors";
    type Params = CountParams;

    fn meta() -> Meta {
        Meta {
            label: "Count",
            group: "Test",
            icon: "",
            shortcut: "",
        }
    }

    fn initial_params(doc: &Document) -> CountParams {
        CountParams {
            count: doc.vectors.len(),
        }
    }

    fn run(_: &Document, _: &CountParams) -> Result<Change, CommandError> {
        Ok(Change::default())
    }
}

#[test]
fn forms_can_start_from_the_document() {
    let mut registry = Registry::default();
    registry.add::<CountVectors>().add::<AddSquare>();
    let doc = aspire_core::testing::sample_document();
    assert_eq!(
        registry.get("count_vectors").unwrap().initial_params(&doc),
        serde_json::json!({ "count": 4 })
    );
    assert_eq!(
        registry.get("add_square").unwrap().initial_params(&doc),
        serde_json::json!({ "size": 10.0 }),
        "commands that do not say otherwise start from their defaults"
    );
}
