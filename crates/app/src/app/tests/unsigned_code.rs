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

/// Puts the unsigned gadget `source`, from the file `file`, in My Gadgets as Cut Craft
/// finds it on starting: unread, by its file's name. Its command id.
fn unsigned(h: &mut Harness, file: &str, source: &str) -> &'static str {
    let commands = std::sync::Arc::new(crate::commands::registry());
    let (gadget, waiting) = cut_craft_scripting::waiting_gadget(file, source, commands);
    let id = cut_craft_core::DynCommand::id(&gadget);
    std::sync::Arc::get_mut(&mut h.app.registry)
        .expect("nothing else holds the commands between frames")
        .add_dyn(Box::new(gadget));
    h.app.my_gadgets.push((file.to_owned(), id));
    let code = crate::trust::Code::gadget(id, file, source.as_bytes());
    let waiting = Some(waiting);
    h.app
        .unsigned_gadgets
        .insert(id.to_owned(), crate::gadgets::Unsigned { code, waiting });
    id
}

fn my_gadgets(h: &Harness) -> Vec<String> {
    h.app
        .installed_items()
        .into_iter()
        .map(|(n, _)| n)
        .collect()
}

#[test]
fn an_unsigned_gadget_waits_by_its_file_name_until_the_user_says_yes() {
    let mut h = Harness::new();
    let id = unsigned(&mut h, "star-burst.lua", STAR);
    assert_eq!(my_gadgets(&h), ["star-burst.lua \u{26a0} (not official)"]);
    // Don't Run: nothing opens, and it is asked about again next time
    h.app.trust.ask = |code| {
        assert_eq!(code.name, "star-burst.lua");
        Answer::DontRun
    };
    h.app.invoke(&h.ctx, id);
    assert!(h.app.form.is_none(), "not run: no form");
    assert_eq!(my_gadgets(&h), ["star-burst.lua \u{26a0} (not official)"]);
    // Run This Time: read now, so its form opens and it takes its own name
    h.app.trust.ask = |_| Answer::RunThisTime;
    h.app.invoke(&h.ctx, id);
    assert_eq!(h.app.form.as_ref().map(|f| f.id), Some(id));
    assert_eq!(my_gadgets(&h), ["Star Burst \u{26a0} (not official)"]);
    // not asked again while Cut Craft runs
    h.app.form = None;
    h.app.trust.ask = |_| panic!("asked twice");
    h.app.invoke(&h.ctx, id);
    assert!(h.app.form.is_some());
}

#[test]
fn a_broken_unsigned_gadget_says_why_once_said_yes_to() {
    let mut h = Harness::new();
    let id = unsigned(&mut h, "typo.lua", "function run(\n");
    h.app.trust.ask = |_| Answer::RunThisTime;
    h.app.invoke(&h.ctx, id);
    assert!(h.app.form.is_none());
    assert!(
        h.app
            .status
            .message
            .starts_with("typo.lua could not be used"),
        "{}",
        h.app.status.message
    );
}

#[test]
fn built_in_gadgets_need_no_question() {
    let mut h = Harness::new();
    h.app.trust.ask = |_| panic!("a built-in gadget is signed with the app");
    h.app.invoke(&h.ctx, "gadget_drill_grid");
    assert!(h.app.form.is_some());
}

#[test]
fn scripts_cannot_run_an_unsigned_gadget_until_the_user_says_yes_in_the_window() {
    let (mut h, at) = switched_on();
    let id = unsigned(&mut h, "star-burst.lua", STAR);
    // a script never brings up the question: it is refused, saying how to allow it
    h.app.trust.ask = |_| panic!("a script made Cut Craft ask");
    let run = json!({ "command": id, "params": { "radius": 4 } });
    let (status, e) = call(&mut h, at, "POST", "run", Some(run.clone()));
    assert_eq!(status, 409, "{e}");
    let why = e["error"].as_str().unwrap();
    assert!(why.contains("Run and Remember"), "{why}");
    let press = json!({ "action": id });
    let (status, e) = call(&mut h, at, "POST", "invoke", Some(press));
    assert_eq!(status, 409, "{e}");
    assert!(h.app.doc.vectors.is_empty() && h.app.form.is_none());
    // the user runs it in the window and says yes: scripts may use it from then on
    h.app.trust.ask = |_| Answer::RunThisTime;
    h.app.invoke(&h.ctx, id);
    h.app.form = None;
    let (status, made) = call(&mut h, at, "POST", "run", Some(run));
    assert_eq!(status, 200, "{made}");
    assert_eq!(h.app.doc.vectors.len(), 1);
}

#[test]
fn an_unsigned_add_on_program_is_asked_about_before_it_starts() {
    let dir = std::env::temp_dir().join(format!("cut-craft-addon-ask-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("depth")).unwrap();
    let program = dir.join("depth").join("program");
    std::fs::write(&program, "one").unwrap();
    let mut h = Harness::new();
    h.app.addons = crate::addons::Addons::new(Some(dir.clone()));
    // put in place by hand: no install record, so unsigned
    h.app.trust.ask = |code| {
        assert!(crate::trust::warning(code).contains("program that runs on this computer"));
        Answer::DontRun
    };
    assert!(!h.app.may_run_add_on("depth", "The Depth add-on", &program));
    h.app.trust.ask = |_| Answer::RunAndRemember;
    assert!(h.app.may_run_add_on("depth", "The Depth add-on", &program));
    h.app.trust.ask = |_| panic!("asked twice about the same program");
    assert!(h.app.may_run_add_on("depth", "The Depth add-on", &program));
    // a new program in its place: asked again
    std::fs::write(&program, "two").unwrap();
    h.app.trust.ask = |_| Answer::DontRun;
    assert!(!h.app.may_run_add_on("depth", "The Depth add-on", &program));
    let _ = std::fs::remove_dir_all(dir);
}
