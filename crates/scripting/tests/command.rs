//! A gadget file becomes a command like any other: in the registry, with a settings form.

use cmd_draw_circle::DrawCircle;
use cut_craft_core::testing::sample_document;
use cut_craft_core::{DynCommand, Registry};
use cut_craft_scripting::load_gadget;
use serde_json::json;
use std::rc::Rc;

fn commands() -> Rc<Registry> {
    let mut r = Registry::default();
    r.add::<DrawCircle>();
    Rc::new(r)
}

const HOLES: &str = r#"
gadget = { name = "Hole Pattern", description = "Holes evenly round a circle." }
settings = {
  { key = "count", label = "Number of holes", type = "integer", default = 6, min = 1 },
  { key = "radius", label = "Pattern radius", type = "length", default = 50, min = 0,
    help = "From the middle of the job to each hole's centre." },
  { key = "shape", label = "Hole shape", type = "choice", choices = { "round", "square" },
    default = "round" },
  { key = "label", label = "Label", type = "text", default = "" },
  { key = "centre_hole", label = "Hole in the middle too", type = "boolean", default = false },
}
function run(s)
  local c = cutcraft.job().centre
  for i = 0, s.count - 1 do
    local a = i * 2 * math.pi / s.count
    cutcraft.run("draw_circle", {
      center = { x = c.x + s.radius * math.cos(a), y = c.y + s.radius * math.sin(a) },
      radius = 3,
    })
  end
end
"#;

#[test]
fn a_gadget_is_named_and_filed_under_gadgets() {
    let g = load_gadget("hole-pattern.lua", HOLES, commands()).unwrap();
    assert_eq!(g.id(), "gadget_hole_pattern");
    assert_eq!(g.meta().label, "Hole Pattern");
    assert_eq!(g.meta().group, "Gadgets");
}

#[test]
fn a_gadget_without_a_name_is_named_after_its_file() {
    let g = load_gadget("my-first-gadget.lua", "function run() end", commands()).unwrap();
    assert_eq!(g.meta().label, "My First Gadget");
    assert_eq!(g.schema()["properties"], json!({}));
}

#[test]
fn its_settings_become_a_form_in_the_order_written() {
    let g = load_gadget("hole-pattern.lua", HOLES, commands()).unwrap();
    let schema = g.schema();
    let keys: Vec<&String> = schema["properties"].as_object().unwrap().keys().collect();
    assert_eq!(keys, ["count", "radius", "shape", "label", "centre_hole"]);
    let p = &schema["properties"];
    assert_eq!(p["count"]["type"], "integer");
    assert_eq!(p["count"]["title"], "Number of holes");
    assert_eq!(p["count"]["minimum"], 1.0);
    assert_eq!(p["radius"]["type"], "number");
    assert_eq!(p["radius"]["x-unit"], "mm");
    assert_eq!(
        p["radius"]["description"],
        "From the middle of the job to each hole's centre."
    );
    assert_eq!(p["shape"]["enum"], json!(["round", "square"]));
    assert_eq!(p["label"]["type"], "string");
    assert_eq!(p["centre_hole"]["type"], "boolean");
    assert_eq!(schema["description"], "Holes evenly round a circle.");
}

#[test]
fn the_form_starts_from_the_gadgets_defaults() {
    let g = load_gadget("hole-pattern.lua", HOLES, commands()).unwrap();
    assert_eq!(
        g.default_params(),
        json!({ "count": 6, "radius": 50, "shape": "round", "label": "", "centre_hole": false })
    );
}

#[test]
fn ok_on_the_form_runs_the_gadget() {
    let g = load_gadget("hole-pattern.lua", HOLES, commands()).unwrap();
    let doc = sample_document();
    let mut params = g.default_params();
    params["count"] = json!(4);
    let change = g.run_json(&doc, &params).unwrap();
    let mut after = doc.clone();
    after.apply(&change).unwrap();
    assert_eq!(after.vectors.len(), doc.vectors.len() + 4);
}

#[test]
fn a_failing_gadget_says_why_on_the_form() {
    let g = load_gadget(
        "broken.lua",
        r#"function run() error("pick a vector first") end"#,
        commands(),
    )
    .unwrap();
    let err = g.run_json(&sample_document(), &json!({})).unwrap_err();
    assert!(err.to_string().contains("pick a vector first"), "{err}");
}

#[test]
fn a_setting_of_an_unknown_kind_is_refused_when_loading() {
    let err = load_gadget(
        "odd.lua",
        r#"settings = { { key = "n", label = "N", type = "colour" } }
           function run() end"#,
        commands(),
    )
    .err()
    .expect("refused");
    let why = err.to_string();
    assert!(why.contains("\"n\""), "{why}");
    assert!(why.contains("colour"), "{why}");
}

#[test]
fn a_gadget_joins_the_registry() {
    let mut r = Registry::default();
    r.add_dyn(Box::new(
        load_gadget("hole-pattern.lua", HOLES, commands()).unwrap(),
    ));
    assert!(r.get("gadget_hole_pattern").is_some());
}

#[test]
fn a_setting_without_a_default_starts_within_its_range() {
    let g = load_gadget(
        "ranged.lua",
        r#"settings = {
             { key = "holes", type = "integer", min = 2.5 },
             { key = "depth", type = "length", min = 3 },
             { key = "angle", type = "number", min = -10, max = -5 },
           }
           function run() end"#,
        commands(),
    )
    .unwrap();
    assert_eq!(
        g.default_params(),
        json!({ "holes": 3, "depth": 3.0, "angle": -5.0 })
    );
}
