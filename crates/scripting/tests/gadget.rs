//! A gadget drives the same commands the UI does, inside a sandbox, as one undoable change.

use cmd_draw_circle::DrawCircle;
use cut_craft_core::Registry;
use cut_craft_core::document::Document;
use cut_craft_core::testing::sample_document;
use cut_craft_scripting::{Limits, ScriptError, run_gadget};
use serde_json::{Value, json};

fn registry() -> Registry {
    let mut r = Registry::default();
    r.add::<DrawCircle>();
    r
}

fn run(source: &str) -> Result<cut_craft_core::Change, ScriptError> {
    run_with(source, &json!({}))
}

fn run_with(source: &str, settings: &Value) -> Result<cut_craft_core::Change, ScriptError> {
    run_gadget(
        "test.lua",
        source,
        &sample_document(),
        &registry(),
        settings,
        Limits::default(),
    )
}

fn applied(change: &cut_craft_core::Change) -> Document {
    let mut doc = sample_document();
    doc.apply(change).expect("a gadget's change applies");
    doc
}

#[test]
fn a_gadget_runs_a_command_by_id() {
    let change = run(r#"
        function run()
          cutcraft.run("draw_circle", { center = { x = 10, y = 20 }, radius = 5 })
        end
    "#)
    .unwrap();
    let doc = applied(&change);
    assert_eq!(doc.vectors.len(), sample_document().vectors.len() + 1);
}

#[test]
fn every_command_a_gadget_runs_is_one_change() {
    let change = run(r#"
        function run()
          for i = 1, 3 do
            cutcraft.run("draw_circle", { center = { x = i * 20, y = 20 }, radius = 5 })
          end
        end
    "#)
    .unwrap();
    let doc = applied(&change);
    assert_eq!(doc.vectors.len(), sample_document().vectors.len() + 3);
    // and undoing it is one step back to where the job was
    let mut undone = doc.clone();
    undone.apply(&change.inverse()).unwrap();
    assert_eq!(undone.vectors, sample_document().vectors);
}

#[test]
fn the_settings_the_user_chose_reach_the_gadget() {
    let change = run_with(
        r#"
        function run(s)
          for i = 1, s.count do
            cutcraft.run("draw_circle", { center = { x = i * 20, y = 20 }, radius = s.radius })
          end
        end
    "#,
        &json!({ "count": 4, "radius": 3.0 }),
    )
    .unwrap();
    assert_eq!(
        applied(&change).vectors.len(),
        sample_document().vectors.len() + 4
    );
}

#[test]
fn a_failed_command_stops_the_gadget_and_changes_nothing() {
    let err = run(r#"
        function run()
          cutcraft.run("draw_circle", { center = { x = 0, y = 0 }, radius = 5 })
          cutcraft.run("draw_circle", { center = { x = 0, y = 0 }, radius = -1 })
        end
    "#)
    .unwrap_err();
    let ScriptError::Failed(why) = err else {
        panic!("expected a failure, got {err:?}")
    };
    assert!(why.contains("draw_circle"), "{why}");
    assert!(why.contains("radius must be greater than 0"), "{why}");
    assert!(why.contains("test.lua:4"), "names the gadget's line: {why}");
}

#[test]
fn an_unknown_command_says_so() {
    let err = run(r#"function run() cutcraft.run("draw_unicorn", {}) end"#).unwrap_err();
    assert!(
        err.to_string().contains("no tool called \"draw_unicorn\""),
        "{err}"
    );
}

#[test]
fn a_syntax_error_names_the_line() {
    let err = run("function run()\n  if then\nend").unwrap_err();
    assert!(matches!(err, ScriptError::Syntax(_)), "{err:?}");
    assert!(err.to_string().contains("test.lua:2"), "{err}");
}

#[test]
fn a_gadget_without_run_says_what_is_missing() {
    let err = run("x = 1").unwrap_err();
    assert!(err.to_string().contains("function run"), "{err}");
}

#[test]
fn a_gadget_cannot_reach_files_programs_or_other_code() {
    let change = run(r#"
        function run()
          local reachable = {}
          for _, name in ipairs({ "io", "os", "package", "require", "debug", "dofile",
                                  "loadfile", "load", "loadstring", "collectgarbage" }) do
            if _G[name] ~= nil then table.insert(reachable, name) end
          end
          if string.dump ~= nil then table.insert(reachable, "string.dump") end
          if #reachable > 0 then error("reachable: " .. table.concat(reachable, ", ")) end
        end
    "#);
    assert!(change.is_ok(), "{change:?}");
}

#[test]
fn compiled_lua_is_refused() {
    let err = run("\x1bLua\x54\x00 not really bytecode").unwrap_err();
    assert!(matches!(err, ScriptError::Syntax(_)), "{err:?}");
}

#[test]
fn a_gadget_that_never_finishes_is_stopped() {
    let err = run_gadget(
        "test.lua",
        "function run() while true do end end",
        &sample_document(),
        &registry(),
        &json!({}),
        Limits {
            instructions: 1_000_000,
            ..Limits::default()
        },
    )
    .unwrap_err();
    assert_eq!(err, ScriptError::TooLong);
}

#[test]
fn a_gadget_that_eats_memory_is_stopped() {
    let err = run_gadget(
        "test.lua",
        r#"function run() local t = {} for i = 1, 1e9 do t[i] = string.rep("x", 100) .. i end end"#,
        &sample_document(),
        &registry(),
        &json!({}),
        Limits {
            memory: 4 * 1024 * 1024,
            ..Limits::default()
        },
    )
    .unwrap_err();
    assert_eq!(err, ScriptError::OutOfMemory);
}

#[test]
fn a_gadget_can_read_the_job_and_selection() {
    let change = run(r#"
        function run()
          local job = cutcraft.job()
          if job.width <= 0 then error("no job width") end
          if type(cutcraft.selection()) ~= "table" then error("no selection list") end
          cutcraft.message(string.format("job is %g wide", job.width))
        end
    "#)
    .unwrap();
    let width = sample_document().job.width;
    assert_eq!(change.notes, vec![format!("job is {width} wide")]);
}

#[test]
fn later_commands_see_what_earlier_ones_made() {
    let change = run(r#"
        function run()
          local before = cutcraft.vector_count()
          cutcraft.run("draw_circle", { center = { x = 0, y = 0 }, radius = 5 })
          if cutcraft.vector_count() ~= before + 1 then error("the circle is not there yet") end
        end
    "#);
    assert!(change.is_ok(), "{change:?}");
}

#[test]
fn a_gadget_that_runs_too_many_commands_is_stopped() {
    let err = run_gadget(
        "test.lua",
        r#"function run()
             for i = 1, 100 do
               cutcraft.run("draw_circle", { center = { x = i, y = 0 }, radius = 1 })
             end
           end"#,
        &sample_document(),
        &registry(),
        &json!({}),
        Limits {
            commands: 10,
            ..Limits::default()
        },
    )
    .unwrap_err();
    assert_eq!(err, ScriptError::TooManyCommands(10));
}
