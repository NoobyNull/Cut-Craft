use super::*;
use crate::toolpath::{Tool, Toolpath};
use serde_json::json;

fn doc_with(strategy: &str, params: Value) -> Document {
    let mut doc = Document::default();
    let mut c = crate::Change::builder(&doc);
    c.add_toolpath(Toolpath {
        name: "Old".into(),
        strategy: strategy.into(),
        params,
        tool: Tool::default(),
        vectors: vec![],
        moves: vec![],
        visible: true,
        power: vec![],
        stale: false,
        group: None,
        drill_bank: None,
        fallbacks: vec![],
        across: vec![],
    });
    doc.apply(&c.finish()).unwrap();
    doc
}

fn only(doc: &Document) -> &Toolpath {
    doc.toolpaths.values().next().unwrap()
}

#[test]
fn radial_and_off_centre_open_as_a_rotary_finishing() {
    let mut doc = doc_with(
        OLD,
        json!({ "name": "Rings", "strategy": "radial", "start": 5.0, "end": 50.0 }),
    );
    assert_eq!(upgrade(&mut doc), 1);
    let t = only(&doc);
    assert_eq!(t.strategy, "rotary_finishing_toolpath");
    assert_eq!(t.params["held"], "over_the_axis");
    assert_eq!(t.params["way"], "round_the_part");
    assert_eq!(
        (t.params["name"].as_str(), t.params["start"].as_f64()),
        (Some("Rings"), Some(5.0))
    );

    let mut doc = doc_with(OLD, json!({ "strategy": "off_centre", "offset": 4.5 }));
    upgrade(&mut doc);
    let t = only(&doc);
    assert_eq!(t.strategy, "rotary_finishing_toolpath");
    assert_eq!(t.params["held"], "off_centre");
    assert_eq!(t.params["offset"], 4.5);
}

#[test]
fn an_old_toolpath_with_no_strategy_saved_was_radial() {
    let mut doc = doc_with(OLD, json!({}));
    upgrade(&mut doc);
    assert_eq!(only(&doc).strategy, "rotary_finishing_toolpath");
}

#[test]
fn indexed_opens_as_locked_angles_with_as_many_angles_as_it_had_sides() {
    let mut doc = doc_with(
        OLD,
        json!({ "strategy": "indexed", "faces": 6, "registration_mark": true }),
    );
    upgrade(&mut doc);
    let t = only(&doc);
    assert_eq!(t.strategy, "rotary_locked_angles_toolpath");
    assert_eq!(t.params["most_angles"], 6);
    assert!(t.params.get("registration_mark").is_none());
}

#[test]
fn sides_then_details_and_other_toolpaths_stay_as_they_are() {
    for (strategy, params) in [
        (OLD, json!({ "strategy": "sides_then_details" })),
        ("profile_toolpath", json!({ "strategy": "radial" })),
    ] {
        let mut doc = doc_with(strategy, params.clone());
        assert_eq!(upgrade(&mut doc), 0);
        assert_eq!(only(&doc).strategy, strategy);
        assert_eq!(only(&doc).params, params);
    }
}
