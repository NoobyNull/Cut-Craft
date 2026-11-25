//! Unsigned code: a gadget not signed with Cut Craft's release key runs only once the user
//! has said yes to it, from the menus, its form or a script alike (`crate::trust`).

use super::web_api::{call, switched_on};
use super::*;
use crate::trust::Answer;
use serde_json::json;

/// A gadget with a name and one setting, that draws a circle.
const STAR: &str = r#"
gadget = { name = "Star Burst" }
settings = { { key = "radius", type = "length", default = 5 } }
function run(s)
  cutcraft.run("draw_circle", { center = { x = 20, y = 20 }, radius = s.radius })
end
"#;

#[test]
fn an_unsigned_gadget_waits_by_its_file_name_until_the_user_says_yes() {
    let mut h = Harness::new();
    let id = h.given_unsigned_gadget("star-burst.lua", STAR);
    assert_eq!(h.my_gadgets(), ["star-burst.lua \u{26a0} (not official)"]);
    // Don't Run: nothing opens, and it is asked about again next time
    h.answer_unsigned_with(|code| {
        assert_eq!(code.name, "star-burst.lua");
        Answer::DontRun
    });
    h.invoke(id);
    assert!(h.form_id().is_none(), "not run: no form");
    assert_eq!(h.my_gadgets(), ["star-burst.lua \u{26a0} (not official)"]);
    // Run This Time: read now, so its form opens and it takes its own name
    h.answer_unsigned_with(|_| Answer::RunThisTime);
    h.invoke(id);
    assert_eq!(h.form_id(), Some(id));
    assert_eq!(h.my_gadgets(), ["Star Burst \u{26a0} (not official)"]);
    // not asked again while Cut Craft runs
    h.close_form();
    h.answer_unsigned_with(|_| panic!("asked twice"));
    h.invoke(id);
    assert!(h.form_id().is_some());
}

#[test]
fn a_broken_unsigned_gadget_says_why_once_said_yes_to() {
    let mut h = Harness::new();
    let id = h.given_unsigned_gadget("typo.lua", "function run(\n");
    h.answer_unsigned_with(|_| Answer::RunThisTime);
    h.invoke(id);
    assert!(h.form_id().is_none());
    assert!(
        h.status_message().starts_with("typo.lua could not be used"),
        "{}",
        h.status_message()
    );
}

#[test]
fn built_in_gadgets_need_no_question() {
    let mut h = Harness::new();
    h.answer_unsigned_with(|_| panic!("a built-in gadget is signed with the app"));
    h.invoke("gadget_drill_grid");
    assert!(h.form_id().is_some());
}

#[test]
fn scripts_cannot_run_an_unsigned_gadget_until_the_user_says_yes_in_the_window() {
    let (mut h, at) = switched_on();
    let id = h.given_unsigned_gadget("star-burst.lua", STAR);
    // a script never brings up the question: it is refused, saying how to allow it
    h.answer_unsigned_with(|_| panic!("a script made Cut Craft ask"));
    let run = json!({ "command": id, "params": { "radius": 4 } });
    let (status, e) = call(&mut h, at, "POST", "run", Some(run.clone()));
    assert_eq!(status, 409, "{e}");
    let why = e["error"].as_str().unwrap();
    assert!(why.contains("Run and Remember"), "{why}");
    let press = json!({ "action": id });
    let (status, e) = call(&mut h, at, "POST", "invoke", Some(press));
    assert_eq!(status, 409, "{e}");
    assert!(h.doc().vectors.is_empty() && h.form_id().is_none());
    // the user runs it in the window and says yes: scripts may use it from then on
    h.answer_unsigned_with(|_| Answer::RunThisTime);
    h.invoke(id);
    h.close_form();
    let (status, made) = call(&mut h, at, "POST", "run", Some(run));
    assert_eq!(status, 200, "{made}");
    assert_eq!(h.doc().vectors.len(), 1);
}

#[test]
fn an_unsigned_add_on_program_is_asked_about_before_it_starts() {
    let dir = std::env::temp_dir().join(format!("cut-craft-addon-ask-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("depth")).unwrap();
    let program = dir.join("depth").join("program");
    std::fs::write(&program, "one").unwrap();
    let mut h = Harness::new();
    h.given_add_ons_folder(dir.clone());
    // put in place by hand: no install record, so unsigned
    h.answer_unsigned_with(|code| {
        assert!(crate::trust::warning(code).contains("program that runs on this computer"));
        Answer::DontRun
    });
    assert!(!h.may_run_add_on("depth", "The Depth add-on", &program));
    h.answer_unsigned_with(|_| Answer::RunAndRemember);
    assert!(h.may_run_add_on("depth", "The Depth add-on", &program));
    h.answer_unsigned_with(|_| panic!("asked twice about the same program"));
    assert!(h.may_run_add_on("depth", "The Depth add-on", &program));
    // a new program in its place: asked again
    std::fs::write(&program, "two").unwrap();
    h.answer_unsigned_with(|_| Answer::DontRun);
    assert!(!h.may_run_add_on("depth", "The Depth add-on", &program));
    let _ = std::fs::remove_dir_all(dir);
}
