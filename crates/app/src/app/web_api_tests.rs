//! Options ▸ Web API driven for real: the app headless, calls over a socket from another
//! thread, answered between frames as clicks would be.

use super::Harness;
use cut_craft_web_api::State;
use serde_json::{Value, json};
use std::fmt::Write as _;
use std::io::{Read as _, Write as _};
use std::net::{SocketAddr, TcpStream};

/// The app with the Web API switched on, on a free port.
fn switched_on() -> (Harness, SocketAddr) {
    let mut h = Harness::new();
    h.app.options.web_api.enabled = true;
    h.app.options.web_api.port = 0;
    h.frame(vec![]);
    let State::Running(at) = h.app.web_api_state().clone() else {
        panic!("it listens: {:?}", h.app.web_api_state());
    };
    (h, at)
}

/// `method path` with `body` sent from another thread while the app runs frames, as the
/// window would; its status and JSON.
fn call(
    h: &mut Harness,
    at: SocketAddr,
    method: &str,
    path: &str,
    body: Option<Value>,
) -> (u16, Value) {
    let token = Some(h.app.options.web_api.token.clone());
    call_with(h, at, token, method, path, body)
}

/// [`call`] sending `token`, or no token at all.
fn call_with(
    h: &mut Harness,
    at: SocketAddr,
    token: Option<String>,
    method: &str,
    path: &str,
    body: Option<Value>,
) -> (u16, Value) {
    let auth = token.map_or_else(String::new, |t| format!("Authorization: Bearer {t}\r\n"));
    let body = body.map(|b| b.to_string());
    let path = format!("{}{path}", cut_craft_web_api::PREFIX);
    let method = method.to_owned();
    let client = std::thread::spawn(move || {
        let mut s = TcpStream::connect(at).unwrap();
        let mut request =
            format!("{method} {path} HTTP/1.1\r\nHost: localhost\r\n{auth}Connection: close\r\n");
        if let Some(body) = &body {
            let _ = write!(
                request,
                "Content-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
                body.len()
            );
        } else {
            request += "\r\n";
        }
        s.write_all(request.as_bytes()).unwrap();
        let mut bytes = vec![];
        s.read_to_end(&mut bytes).unwrap();
        bytes
    });
    for _ in 0..2000 {
        if client.is_finished() {
            break;
        }
        h.frame(vec![]);
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    let bytes = client.join().unwrap();
    let split = bytes
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .unwrap_or(bytes.len() - 4);
    let head = String::from_utf8_lossy(&bytes[..split]);
    let status = head[9..12].parse().unwrap();
    let body = &bytes[split + 4..];
    let body = if head.contains("Transfer-Encoding: chunked") {
        unchunk(body)
    } else {
        body.to_vec()
    };
    (status, serde_json::from_slice(&body).unwrap_or(Value::Null))
}

/// A long answer (the description) comes in chunks, each led by its length in hex. Lengths
/// count bytes, so a chunk can end inside a letter: they are joined as bytes.
fn unchunk(mut rest: &[u8]) -> Vec<u8> {
    let mut joined = vec![];
    while let Some(end) = rest.windows(2).position(|w| w == b"\r\n") {
        let size = std::str::from_utf8(&rest[..end]).unwrap_or_default();
        let size = usize::from_str_radix(size.trim(), 16).unwrap_or(0);
        if size == 0 {
            break;
        }
        joined.extend_from_slice(&rest[end + 2..end + 2 + size]);
        rest = &rest[end + 4 + size..];
    }
    joined
}

#[test]
fn the_web_api_is_off_until_switched_on_and_the_status_bar_says_nothing_of_it() {
    let mut h = Harness::new();
    assert!(!h.app.options.web_api.enabled);
    h.frame(vec![]);
    assert_eq!(h.app.web_api_state(), &State::Off);
    assert!(h.text_at(super::super::web_api::RUNNING).is_none());
    assert!(h.text_at(super::super::web_api::NOT_RUNNING).is_none());
}

#[test]
fn switched_on_it_listens_on_this_computer_with_a_token_and_shows_green() {
    let (mut h, at) = switched_on();
    assert!(at.ip().is_loopback(), "{at}");
    assert_eq!(h.app.options.web_api.token.len(), 48, "a token is made");
    h.frame(vec![]);
    assert!(h.text_at(super::super::web_api::RUNNING).is_some());
    // switched off: it stops listening and the slot goes
    h.app.options.web_api.enabled = false;
    h.frame(vec![]);
    h.frame(vec![]);
    assert_eq!(h.app.web_api_state(), &State::Off);
    assert!(h.text_at(super::super::web_api::RUNNING).is_none());
    // the listener lets go a moment after it is stopped
    let since = std::time::Instant::now();
    while TcpStream::connect(at).is_ok() {
        assert!(
            since.elapsed() < std::time::Duration::from_secs(5),
            "it stops listening"
        );
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
}

#[test]
fn a_port_in_use_shows_red_and_says_how_to_fix_it() {
    let busy = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let mut h = Harness::new();
    h.app.options.web_api.enabled = true;
    h.app.options.web_api.port = busy.local_addr().unwrap().port();
    h.frame(vec![]);
    h.frame(vec![]);
    let State::Failed(why) = h.app.web_api_state().clone() else {
        panic!("{:?}", h.app.web_api_state());
    };
    assert!(why.contains("Options ▸ Web API"), "{why}");
    assert!(h.text_at(super::super::web_api::NOT_RUNNING).is_some());
    // another port, and it runs
    h.app.options.web_api.port = 0;
    h.frame(vec![]);
    assert!(matches!(h.app.web_api_state(), State::Running(_)));
}

#[test]
fn a_script_draws_selects_and_undoes_as_a_person_would() {
    let (mut h, at) = switched_on();
    let (status, s) = call(&mut h, at, "GET", "status", None);
    assert_eq!(status, 200, "{s}");
    assert_eq!(s["job_open"], true);
    // draw a rectangle, giving only some of its settings
    let (status, made) = call(
        &mut h,
        at,
        "POST",
        "run",
        Some(
            json!({ "command": "draw_rectangle", "params": { "x": 10, "y": 10, "width": 40, "height": 20 } }),
        ),
    );
    assert_eq!(status, 200, "{made}");
    let added = made["added_vectors"].as_array().unwrap().clone();
    assert_eq!(added.len(), 1);
    assert_eq!(h.app.doc.vectors.len(), 1);
    // the job lists it, in the job's units
    let (_, job) = call(&mut h, at, "GET", "job", None);
    assert_eq!(job["vectors"][0]["max"], json!([50.0, 30.0]));
    // pick nothing, then everything on its layer
    let (_, none) = call(&mut h, at, "POST", "select", Some(json!({ "by": "none" })));
    assert_eq!(none["selection"], json!([]));
    let layer = job["layers"][0]["name"].clone();
    let (status, picked) = call(
        &mut h,
        at,
        "POST",
        "select",
        Some(json!({ "by": "layer", "layer": layer })),
    );
    assert_eq!(status, 200, "{picked}");
    assert_eq!(picked["selection"], json!(added));
    // Undo, pressed as its menu item: the pick is taken back, then the rectangle
    for _ in 0..3 {
        let (status, _) = call(
            &mut h,
            at,
            "POST",
            "invoke",
            Some(json!({ "action": "undo" })),
        );
        assert_eq!(status, 200);
    }
    assert!(h.app.doc.vectors.is_empty());
}

#[test]
fn a_call_that_cannot_be_done_says_why_and_changes_nothing() {
    let (mut h, at) = switched_on();
    let (status, e) = call(
        &mut h,
        at,
        "POST",
        "invoke",
        Some(json!({ "action": "exit" })),
    );
    assert_eq!(status, 409, "{e}");
    let (status, e) = call(
        &mut h,
        at,
        "POST",
        "invoke",
        Some(json!({ "action": "no_such" })),
    );
    assert_eq!(status, 404, "{e}");
    let (status, e) = call(
        &mut h,
        at,
        "POST",
        "invoke",
        Some(json!({ "action": "redo" })),
    );
    assert_eq!(status, 409, "nothing to redo: {e}");
    let (status, e) = call(
        &mut h,
        at,
        "POST",
        "run",
        Some(json!({ "command": "draw_circle", "params": { "radius": -5 } })),
    );
    assert_eq!(status, 409, "{e}");
    assert!(e["error"].as_str().unwrap().contains("radius"), "{e}");
    assert!(h.app.doc.vectors.is_empty());
}

#[test]
fn every_tool_and_menu_item_is_listed_for_scripts() {
    let (mut h, at) = switched_on();
    let (_, commands) = call(&mut h, at, "GET", "commands", None);
    let ids: Vec<&str> = commands
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["id"].as_str().unwrap())
        .collect();
    assert!(ids.contains(&"profile_toolpath") && ids.contains(&"draw_circle"));
    let (_, actions) = call(&mut h, at, "GET", "actions", None);
    let undo = actions
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["id"] == "undo")
        .expect("Undo is listed");
    assert_eq!(undo["available"], false, "nothing done yet");
    assert!(
        actions
            .as_array()
            .unwrap()
            .iter()
            .all(|a| a["id"] != "exit")
    );
    let (status, circle) = call(&mut h, at, "GET", "commands/draw_circle", None);
    assert_eq!(status, 200);
    assert!(circle["starts_with"]["radius"].is_number(), "{circle}");
}

#[test]
fn the_web_api_has_a_help_page_with_its_pictures_linked_from_contents() {
    let h = Harness::new();
    assert!(h.app.all_help_topics().contains(&"web_api"));
    assert!(h.app.help_markdown("contents").contains("(help:web_api)"));
    let page = h.app.help_markdown("web_api");
    for call in [
        "GET status",
        "POST select",
        "POST run",
        "POST invoke",
        "GET job",
    ] {
        assert!(page.contains(call), "{call} is explained");
    }
    // every picture any help page shows is one the help carries
    let carried: Vec<&str> = cut_craft_help::PICTURES.iter().map(|(n, _)| *n).collect();
    let mut shown = std::collections::BTreeSet::new();
    for topic in h.app.all_help_topics() {
        let md = h.app.help_markdown(topic);
        for part in md.split("](pictures/").skip(1) {
            let name = &part[..part.find(')').unwrap()];
            assert!(
                carried.contains(&name),
                "{name} is not in crates/help/pictures"
            );
            shown.insert(name.to_owned());
        }
    }
    assert_eq!(shown.len(), carried.len(), "every picture carried is shown");
}

#[test]
fn the_example_call_is_the_status_call_on_the_chosen_port() {
    assert_eq!(
        super::super::web_api::example_call(8123, false),
        "curl http://localhost:8123/api/v1/status"
    );
    assert_eq!(
        super::super::web_api::example_call(8123, true),
        "curl -H \"Authorization: Bearer <token>\" http://localhost:8123/api/v1/status"
    );
}

#[test]
fn this_computer_needs_no_token_but_open_to_the_network_every_call_does() {
    let (mut h, at) = switched_on();
    let (status, _) = call_with(&mut h, at, None, "GET", "status", None);
    assert_eq!(status, 200, "a program on this computer needs no token");
    // other computers allowed: it listens on every network, and wants the token
    h.app.options.web_api.allow_network = true;
    h.app.options.web_api.port = at.port();
    h.frame(vec![]);
    let State::Running(open) = h.app.web_api_state().clone() else {
        panic!("{:?}", h.app.web_api_state());
    };
    assert!(open.ip().is_unspecified(), "{open}");
    let here = SocketAddr::from(([127, 0, 0, 1], open.port()));
    let (status, e) = call_with(&mut h, here, None, "GET", "status", None);
    assert_eq!(status, 401, "{e}");
    let (status, _) = call_with(&mut h, here, Some("wrong".into()), "GET", "status", None);
    assert_eq!(status, 401);
    let (status, _) = call(&mut h, here, "GET", "status", None);
    assert_eq!(status, 200);
}

#[test]
fn the_docs_page_offers_every_tool_and_menu_item_to_try() {
    let (mut h, at) = switched_on();
    let (status, doc) = call_with(&mut h, at, None, "GET", "openapi.json", None);
    assert_eq!(status, 200, "{doc}");
    let run = &doc["paths"]["/run"]["post"]["requestBody"]["content"]["application/json"];
    let circle = &run["examples"]["draw_circle"]["value"];
    assert_eq!(circle["command"], "draw_circle");
    assert!(circle["params"]["radius"].is_number(), "{circle}");
    let commands = run["schema"]["properties"]["command"]["enum"]
        .as_array()
        .unwrap();
    assert!(commands.iter().any(|c| c == "profile_toolpath"));
    let invoke = &doc["paths"]["/invoke"]["post"]["requestBody"]["content"]["application/json"];
    assert_eq!(invoke["examples"]["undo"]["value"]["action"], "undo");
    assert!(
        invoke["examples"].get("exit").is_none(),
        "Exit is not offered"
    );
    // on this computer the page offers no Authorize; open to the network it does
    assert!(doc.get("security").is_none());
    h.app.options.web_api.allow_network = true;
    h.frame(vec![]);
    let State::Running(open) = h.app.web_api_state().clone() else {
        panic!("{:?}", h.app.web_api_state());
    };
    let here = SocketAddr::from(([127, 0, 0, 1], open.port()));
    let (status, doc) = call_with(&mut h, here, None, "GET", "openapi.json", None);
    assert_eq!(status, 200, "the page reads it without the token: {doc}");
    assert_eq!(doc["security"], json!([{ "token": [] }]));
    let (status, _) = call_with(&mut h, here, None, "GET", "job", None);
    assert_eq!(status, 401, "calls tried from it still need the token");
}

#[test]
fn options_points_to_the_docs_page_on_the_chosen_port() {
    assert_eq!(
        super::super::web_api::docs_address(8123),
        "http://localhost:8123/docs/"
    );
    let page = Harness::new().app.help_markdown("web_api");
    assert!(
        page.contains("http://localhost:7340/docs/"),
        "help points to /docs"
    );
}
