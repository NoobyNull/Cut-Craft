//! The server tested over real sockets, with a stand-in app thread answering its calls.

use super::*;
use std::io::{Read as _, Write as _};
use std::net::TcpStream;

const TOKEN: &str = "test-token";

fn settings(token: Option<&str>) -> Settings {
    Settings {
        allow_network: false,
        port: 0,
        token: token.map(str::to_owned),
    }
}

/// A server on a free port, with a thread playing the app: it answers every call with the
/// call itself, written out, so a test sees what the server made of its request.
fn running(token: Option<&str>) -> (SocketAddr, std::sync::Arc<std::sync::atomic::AtomicBool>) {
    let server = Server::start(&settings(token), || {}).expect("a free port listens");
    let address = server.address();
    let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let stopped = stop.clone();
    std::thread::spawn(move || {
        while !stopped.load(std::sync::atomic::Ordering::Relaxed) {
            for p in server.take() {
                let echo = format!("{:?}", p.call);
                p.answer(Ok(serde_json::json!({ "call": echo })));
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    });
    (address, stop)
}

struct Reply {
    status: u16,
    body: Value,
}

/// Sends a raw HTTP request and reads the reply.
fn send(address: SocketAddr, request: &str) -> Reply {
    let mut stream = TcpStream::connect(address).unwrap();
    stream.write_all(request.as_bytes()).unwrap();
    let mut text = String::new();
    stream.read_to_string(&mut text).unwrap();
    let status = text[9..12].parse().unwrap();
    let body = text.split("\r\n\r\n").nth(1).unwrap_or_default();
    Reply {
        status,
        body: serde_json::from_str(body).unwrap_or(Value::Null),
    }
}

fn get(address: SocketAddr, path: &str, token: Option<&str>) -> Reply {
    let auth = token.map_or_else(String::new, |t| format!("Authorization: Bearer {t}\r\n"));
    send(
        address,
        &format!("GET {path} HTTP/1.1\r\nHost: localhost\r\n{auth}Connection: close\r\n\r\n"),
    )
}

fn post(address: SocketAddr, path: &str, body: &str) -> Reply {
    send(
        address,
        &format!(
            "POST {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nAuthorization: Bearer {TOKEN}\r\n\
             Content-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        ),
    )
}

#[test]
fn a_call_with_the_token_reaches_the_app_and_its_answer_comes_back() {
    let (address, _stop) = running(Some(TOKEN));
    let reply = get(address, "/api/v1/status", Some(TOKEN));
    assert_eq!(reply.status, 200);
    assert_eq!(reply.body["call"], "Status");
}

#[test]
fn a_call_without_the_right_token_is_turned_away_before_the_app_sees_it() {
    let (address, _stop) = running(Some(TOKEN));
    for token in [None, Some("wrong"), Some("test-token-but-longer")] {
        let reply = get(address, "/api/v1/status", token);
        assert_eq!(reply.status, 401, "{token:?}");
        assert!(
            reply.body["error"]
                .as_str()
                .unwrap()
                .contains("Options ▸ Web API")
        );
    }
}

#[test]
fn with_no_token_set_calls_need_none() {
    let (address, _stop) = running(None);
    assert_eq!(get(address, "/api/v1/job", None).status, 200);
}

#[test]
fn on_localhost_a_call_to_another_host_name_is_refused() {
    let (address, _stop) = running(Some(TOKEN));
    let reply = send(
        address,
        &format!(
            "GET /api/v1/status HTTP/1.1\r\nHost: evil.example:7340\r\n\
             Authorization: Bearer {TOKEN}\r\nConnection: close\r\n\r\n"
        ),
    );
    assert_eq!(reply.status, 403);
}

#[test]
fn local_host_names_are_this_computer_and_others_are_not() {
    for host in [
        "localhost",
        "localhost:7340",
        "127.0.0.1:7340",
        "[::1]:7340",
        "LocalHost",
    ] {
        assert!(is_local_host(host), "{host}");
    }
    for host in [
        "",
        "evil.example",
        "localhost.evil.example:7340",
        "10.0.0.2:7340",
    ] {
        assert!(!is_local_host(host), "{host}");
    }
}

#[test]
fn a_post_that_is_not_json_is_refused_so_web_pages_cannot_send_one() {
    let (address, _stop) = running(Some(TOKEN));
    let reply = send(
        address,
        &format!(
            "POST /api/v1/invoke HTTP/1.1\r\nHost: localhost\r\nAuthorization: Bearer {TOKEN}\r\n\
             Content-Type: text/plain\r\nContent-Length: 17\r\nConnection: close\r\n\r\n\
             {{\"action\":\"undo\"}}"
        ),
    );
    assert_eq!(reply.status, 415);
}

#[test]
fn each_call_is_read_from_its_path_and_body() {
    let (address, _stop) = running(Some(TOKEN));
    let call = |r: Reply| {
        assert_eq!(r.status, 200, "{}", r.body);
        r.body["call"].as_str().unwrap().to_owned()
    };
    assert_eq!(
        call(get(address, "/api/v1/commands", Some(TOKEN))),
        "Commands"
    );
    assert_eq!(
        call(get(address, "/api/v1/commands/draw_circle", Some(TOKEN))),
        "Command(\"draw_circle\")"
    );
    assert_eq!(
        call(get(address, "/api/v1/actions/", Some(TOKEN))),
        "Actions"
    );
    assert_eq!(
        call(post(address, "/api/v1/invoke", r#"{"action":"undo"}"#)),
        "Invoke(\"undo\")"
    );
    assert_eq!(
        call(post(
            address,
            "/api/v1/run",
            r#"{"command":"draw_circle","params":{"radius":5}}"#
        )),
        "Run { command: \"draw_circle\", params: Object {\"radius\": Number(5)} }"
    );
    assert_eq!(
        call(post(
            address,
            "/api/v1/select",
            r#"{"by":"layer","layer":"Cut","add":true}"#
        )),
        "Select { pick: Layer { layer: \"Cut\" }, add: true }"
    );
}

#[test]
fn a_wrong_call_says_what_to_send_instead() {
    let (address, _stop) = running(Some(TOKEN));
    let unknown = get(address, "/api/v1/nonsense", Some(TOKEN));
    assert_eq!(unknown.status, 404);
    assert!(
        unknown.body["error"]
            .as_str()
            .unwrap()
            .contains("/api/v1/commands")
    );
    assert_eq!(get(address, "/elsewhere", Some(TOKEN)).status, 404);
    assert_eq!(get(address, "/api/v1/invoke", Some(TOKEN)).status, 405);
    let missing = post(address, "/api/v1/invoke", r#"{"press":"undo"}"#);
    assert_eq!(missing.status, 400);
    assert!(
        missing.body["error"]
            .as_str()
            .unwrap()
            .contains(r#"{"action": "undo"}"#)
    );
    assert_eq!(
        post(address, "/api/v1/select", r#"{"by":"layer"}"#).status,
        400
    );
    assert_eq!(post(address, "/api/v1/run", "not json").status, 400);
}

#[test]
fn a_taken_port_says_so_and_how_to_fix_it() {
    let first = Server::start(&settings(None), || {}).unwrap();
    let taken = Settings {
        port: first.address().port(),
        ..settings(None)
    };
    let why = Server::start(&taken, || {}).err().unwrap();
    assert!(why.contains("Options ▸ Web API"), "{why}");
}

#[test]
fn a_taken_port_is_named_however_the_system_words_it() {
    let windows = std::io::Error::new(
        std::io::ErrorKind::AddrInUse,
        "Only one usage of each socket address (protocol/network address/port) is normally \
         permitted. (os error 10048)",
    );
    let at = SocketAddr::from(([127, 0, 0, 1], 7340));
    assert_eq!(
        could_not_listen(at, &windows),
        "Port 7340 is in use by another program. Pick another port in Options ▸ Web API."
    );
    let other = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied");
    assert!(could_not_listen(at, &other).starts_with("The Web API could not listen"));
}

#[test]
fn it_listens_on_this_computer_unless_other_computers_are_allowed() {
    let mut s = settings(None);
    s.port = DEFAULT_PORT;
    assert_eq!(s.address().to_string(), "127.0.0.1:7340");
    s.allow_network = true;
    assert_eq!(s.address().to_string(), "0.0.0.0:7340");
}

#[test]
fn stopping_frees_the_port_for_the_next_start() {
    let server = Server::start(&settings(None), || {}).unwrap();
    let again = Settings {
        port: server.address().port(),
        ..settings(None)
    };
    assert!(server.listening());
    drop(server);
    let restarted = Server::start(&again, || {});
    assert!(restarted.is_ok(), "{:?}", restarted.err());
}

#[test]
fn tokens_are_long_and_different_each_time() {
    let (a, b) = (new_token(), new_token());
    assert_eq!(a.len(), 48);
    assert_ne!(a, b);
    assert!(a.bytes().all(|c| c.is_ascii_hexdigit()));
}

mod job {
    use crate::Pick;
    use crate::job::{command, pick, run_params, summary};
    use cut_craft_core::document::{EntityId, Layer, LayerId, Units};
    use cut_craft_core::testing::sample_document;
    use serde_json::json;

    #[test]
    fn the_state_tells_the_material_the_model_and_what_is_picked_in_the_jobs_units() {
        use cut_craft_core::document::Units;
        let mut doc = cut_craft_core::testing::domed();
        let s = crate::job::state(&doc, None);
        assert_eq!(s["units"], "mm");
        assert_eq!(s["material"]["width"], 40.0);
        assert_eq!(s["material"]["thickness"], 10.0);
        assert!(s["material"]["z_zero"].is_string(), "{s}");
        assert!(s["model"]["position"]["gap"].is_number(), "{s}");
        let parts = s["model"]["components"].as_array().unwrap();
        assert_eq!(parts.len(), 1);
        assert_eq!(parts[0]["name"], "dome");
        assert!(parts[0]["height"].as_f64().unwrap() > 7.0, "{s}");
        assert_eq!(s["selection"]["vectors"], json!([]));
        assert_eq!(s["selection"]["toolpath"], serde_json::Value::Null);
        let id = *doc.components.keys().next().unwrap();
        doc.selected_components = [id].into();
        doc.job.units = Units::Inch;
        let s = crate::job::state(&doc, Some(EntityId(9)));
        assert_eq!(s["selection"]["components"], json!([id.0]));
        assert_eq!(s["selection"]["toolpath"], 9);
        assert_eq!(s["material"]["width"], 1.575, "inches, to a thousandth");
    }

    #[test]
    fn the_toolpath_list_gives_each_ones_time_and_what_to_look_at() {
        use cut_craft_core::preflight::{Finding, Severity};
        use cut_craft_core::toolpath::Toolpath;
        let mut doc = sample_document();
        let t = |name: &str, stale: bool| Toolpath {
            name: name.into(),
            strategy: "profile_toolpath".into(),
            params: json!({}),
            tool: cut_craft_core::toolpath::Tool::default(),
            vectors: vec![],
            moves: vec![],
            visible: true,
            power: vec![],
            stale,
            group: None,
            drill_bank: None,
            fallbacks: vec![],
        };
        doc.toolpaths.insert(EntityId(50), t("Outline", false));
        doc.toolpaths.insert(EntityId(51), t("Pocket", true));
        let checks = std::collections::BTreeMap::from([(
            EntityId(50),
            vec![Finding {
                severity: Severity::Error,
                message: "Too deep for the bit".into(),
            }],
        )]);
        let list = crate::job::toolpaths(
            &doc,
            &|t| if t.name == "Outline" { 90.5 } else { 2.0 },
            &checks,
        );
        let rows = list["toolpaths"].as_array().unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0]["name"], "Outline");
        assert_eq!(rows[0]["minutes"], 90.5);
        assert_eq!(rows[0]["time"], "1h 30m 30s");
        assert_eq!(rows[0]["errors"], json!(["Too deep for the bit"]));
        assert_eq!(rows[1]["out_of_date"], true);
        let warned = rows[1]["warnings"][0].as_str().unwrap();
        assert!(warned.contains("Recalculate"), "{warned}");
        assert_eq!(
            list["total_minutes"], 92.5,
            "the shown ones' times added up"
        );
        assert_eq!(list["total_time"], "1h 32m 30s");
    }

    #[test]
    fn the_summary_lists_layers_vectors_and_the_selection_in_the_jobs_units() {
        let mut doc = sample_document();
        let s = summary(&doc);
        assert_eq!(s["units"], "mm");
        assert_eq!(s["vectors"].as_array().unwrap().len(), 4);
        assert_eq!(s["vectors"][0]["min"], json!([10.0, 10.0]));
        assert_eq!(s["vectors"][2]["closed"], false);
        assert_eq!(s["selection"].as_array().unwrap().len(), 4);
        doc.job.units = Units::Inch;
        assert_eq!(summary(&doc)["vectors"][0]["min"], json!([0.394, 0.394]));
    }

    #[test]
    fn vectors_are_picked_by_id_layer_area_or_kind_never_from_a_locked_layer() {
        let mut doc = sample_document();
        let ids: Vec<EntityId> = doc.vectors.keys().copied().collect();
        let by = |doc: &cut_craft_core::document::Document, p: Pick| pick(doc, &p).unwrap();
        assert_eq!(by(&doc, Pick::All).len(), 4);
        assert_eq!(by(&doc, Pick::Open), vec![ids[2]]);
        assert_eq!(by(&doc, Pick::Closed).len(), 3);
        assert_eq!(
            by(&doc, Pick::None),
            [] as [cut_craft_core::document::EntityId; 0]
        );
        let inside = Pick::Inside {
            x1: 0.0,
            y1: 0.0,
            x2: 90.0,
            y2: 45.0,
            units: None,
        };
        assert_eq!(by(&doc, inside), vec![ids[0], ids[1]]);
        let inches = Pick::Inside {
            x1: 0.0,
            y1: 0.0,
            x2: 1.5,
            y2: 1.5,
            units: Some("in".into()),
        };
        assert_eq!(by(&doc, inches), vec![ids[0]]);
        // a second layer, locked, with the circle on it
        let cut = LayerId(7);
        doc.layers.push(Layer::new(cut, "Cut"));
        doc.vector_layers.insert(ids[3], cut);
        assert_eq!(
            by(
                &doc,
                Pick::Layer {
                    layer: "cut".into()
                }
            ),
            vec![ids[3]]
        );
        doc.layers.last_mut().unwrap().locked = true;
        assert_eq!(
            by(
                &doc,
                Pick::Layer {
                    layer: "Cut".into()
                }
            ),
            [] as [cut_craft_core::document::EntityId; 0]
        );
        assert_eq!(by(&doc, Pick::All).len(), 3);
        let locked = pick(
            &doc,
            &Pick::Ids {
                ids: vec![ids[3].0],
            },
        )
        .unwrap_err();
        assert_eq!(locked.status, 409);
    }

    #[test]
    fn a_pick_that_names_nothing_says_what_there_is() {
        let doc = sample_document();
        let layer = pick(
            &doc,
            &Pick::Layer {
                layer: "Nope".into(),
            },
        )
        .unwrap_err();
        assert_eq!(layer.status, 404);
        assert!(
            layer.message.contains("The layers are:"),
            "{}",
            layer.message
        );
        assert_eq!(
            pick(&doc, &Pick::Ids { ids: vec![999] })
                .unwrap_err()
                .status,
            404
        );
        assert_eq!(
            pick(&doc, &Pick::Name { name: "Lid".into() })
                .unwrap_err()
                .status,
            404
        );
        let units = Pick::Inside {
            x1: 0.0,
            y1: 0.0,
            x2: 1.0,
            y2: 1.0,
            units: Some("cm".into()),
        };
        assert_eq!(pick(&doc, &units).unwrap_err().status, 400);
    }

    #[test]
    fn named_parts_are_picked_by_their_name() {
        let mut doc = sample_document();
        let ids: Vec<EntityId> = doc.vectors.keys().copied().collect();
        doc.vector_groups.insert(ids[0], 3);
        doc.vector_groups.insert(ids[1], 3);
        doc.part_names.insert(3, "Lid".into());
        assert_eq!(
            pick(&doc, &Pick::Name { name: "lid".into() }).unwrap(),
            vec![ids[0], ids[1]]
        );
    }

    fn registry() -> cut_craft_core::Registry {
        let mut r = cut_craft_core::Registry::default();
        r.add::<Stretch>();
        r
    }

    /// A stand-in tool with nested settings.
    struct Stretch;

    #[derive(Clone, Default, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
    struct StretchParams {
        by: f64,
        keep: Keep,
    }

    #[derive(Clone, Default, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
    struct Keep {
        width: bool,
        height: bool,
    }

    impl cut_craft_core::Command for Stretch {
        const ID: &'static str = "stretch";
        type Params = StretchParams;
        fn meta() -> cut_craft_core::Meta {
            cut_craft_core::Meta {
                label: "Stretch",
                group: "Edit",
                icon: "",
                shortcut: "",
            }
        }
        fn run(
            doc: &cut_craft_core::document::Document,
            _: &StretchParams,
        ) -> Result<cut_craft_core::Change, cut_craft_core::CommandError> {
            Ok(cut_craft_core::Change::builder(doc).finish())
        }
    }

    #[test]
    fn run_lays_the_callers_settings_over_what_the_form_starts_with() {
        let doc = sample_document();
        let r = registry();
        let params =
            run_params(&r, &doc, "stretch", &json!({ "keep": { "height": true } })).unwrap();
        assert_eq!(
            params,
            json!({ "by": 0.0, "keep": { "width": false, "height": true } })
        );
        assert_eq!(
            run_params(&r, &doc, "nope", &json!({})).unwrap_err().status,
            404
        );
    }

    #[test]
    fn a_misspelt_setting_is_refused_with_the_names_there_are_rather_than_dropped() {
        let doc = sample_document();
        let r = registry();
        let top = run_params(&r, &doc, "stretch", &json!({ "bye": 2.0 })).unwrap_err();
        assert_eq!(top.status, 400);
        assert!(
            top.message.contains("\"bye\"") && top.message.contains("by, keep"),
            "{}",
            top.message
        );
        let inner =
            run_params(&r, &doc, "stretch", &json!({ "keep": { "depth": true } })).unwrap_err();
        assert!(
            inner.message.contains("\"keep.depth\"") && inner.message.contains("width, height"),
            "{}",
            inner.message
        );
    }

    #[test]
    fn a_tools_settings_come_with_their_schema_and_starting_values() {
        let doc = sample_document();
        let c = command(&registry(), &doc, "stretch").unwrap();
        assert_eq!(c["name"], "Stretch");
        assert_eq!(c["starts_with"]["by"], 0.0);
        assert!(c["settings"]["properties"]["keep"].is_object());
    }
}

/// The raw reply to a GET of `path`, headers and all.
fn get_raw(address: SocketAddr, path: &str, host: &str) -> Vec<u8> {
    let mut stream = TcpStream::connect(address).unwrap();
    stream
        .write_all(
            format!("GET {path} HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n\r\n").as_bytes(),
        )
        .unwrap();
    let mut bytes = vec![];
    stream.read_to_end(&mut bytes).unwrap();
    bytes
}

#[test]
fn the_docs_page_opens_in_a_browser_without_the_token() {
    let (address, _stop) = running(Some(TOKEN));
    let to_page = String::from_utf8_lossy(&get_raw(address, "/docs", "localhost")).into_owned();
    assert!(to_page.starts_with("HTTP/1.1 308"), "{to_page}");
    assert!(to_page.contains("Location: /docs/"), "{to_page}");
    let page = String::from_utf8_lossy(&get_raw(address, "/docs/", "localhost")).into_owned();
    assert!(page.starts_with("HTTP/1.1 200"), "{page}");
    assert!(page.contains("text/html"), "{page}");
    // the page's own script, and the description it reads
    let script = get_raw(address, "/docs/swagger-initializer.js", "localhost");
    let script = String::from_utf8_lossy(&script);
    assert!(script.starts_with("HTTP/1.1 200"), "{script}");
    assert!(script.contains(docs::DESCRIPTION), "{script}");
    let missing =
        String::from_utf8_lossy(&get_raw(address, "/docs/no-such.js", "localhost")).into_owned();
    assert!(missing.starts_with("HTTP/1.1 404"), "{missing}");
}

#[test]
fn the_description_needs_no_token_but_every_call_still_does() {
    let (address, _stop) = running(Some(TOKEN));
    let reply = get(address, docs::DESCRIPTION, None);
    assert_eq!(reply.status, 200);
    assert_eq!(reply.body["call"], "OpenApi");
    assert_eq!(get(address, "/api/v1/status", None).status, 401);
    assert_eq!(get(address, "/api/v1/commands", None).status, 401);
}

#[test]
fn on_localhost_the_docs_page_is_not_shown_to_another_host_name() {
    let (address, _stop) = running(None);
    for path in ["/docs/", docs::DESCRIPTION] {
        let raw = get_raw(address, path, "evil.example:7340");
        let reply = String::from_utf8_lossy(&raw);
        assert!(reply.starts_with("HTTP/1.1 403"), "{path}: {reply}");
    }
}

#[test]
fn the_description_lists_every_tool_and_menu_item_with_an_example() {
    let tools = [openapi::Tool {
        id: "draw_circle",
        name: "Draw Circle",
        group: "Draw",
        starts_with: serde_json::json!({ "radius": 10 }),
    }];
    let actions = [("undo".to_owned(), "Undo".to_owned())];
    let doc = openapi::document(&tools, &actions, false);
    assert_eq!(doc["openapi"], "3.0.3");
    let run = &doc["paths"]["/run"]["post"]["requestBody"]["content"]["application/json"];
    assert_eq!(
        run["schema"]["properties"]["command"]["enum"],
        serde_json::json!(["draw_circle"])
    );
    assert_eq!(
        run["examples"]["draw_circle"]["value"],
        serde_json::json!({ "command": "draw_circle", "params": { "radius": 10 } })
    );
    let invoke = &doc["paths"]["/invoke"]["post"]["requestBody"]["content"]["application/json"];
    assert_eq!(
        invoke["examples"]["undo"]["value"],
        serde_json::json!({ "action": "undo" })
    );
    for path in [
        "/status",
        "/commands",
        "/commands/{id}",
        "/actions",
        "/job",
        "/select",
    ] {
        assert!(doc["paths"][path].is_object(), "{path}");
    }
    // Authorize is offered only when calls need the token
    assert!(doc.get("security").is_none());
    let open = openapi::document(&tools, &actions, true);
    assert_eq!(open["security"], serde_json::json!([{ "token": [] }]));
    assert_eq!(
        open["components"]["securitySchemes"]["token"]["scheme"],
        "bearer"
    );
}

#[test]
fn the_new_calls_are_read_from_their_path_query_and_body() {
    let (address, _stop) = running(Some(TOKEN));
    let call = |r: Reply| {
        assert_eq!(r.status, 200, "{}", r.body);
        r.body["call"].as_str().unwrap().to_owned()
    };
    let got = |path: &str| call(get(address, path, Some(TOKEN)));
    assert_eq!(
        got("/api/v1/view.png?view=top&preview=before&width=640"),
        "View(Picture { view: Top, preview: Before, width: Some(640) })"
    );
    assert_eq!(
        got("/api/v1/view.png"),
        "View(Picture { view: Iso, preview: After, width: None })"
    );
    assert_eq!(got("/api/v1/state"), "State");
    assert_eq!(got("/api/v1/toolpaths"), "Toolpaths");
    assert_eq!(got("/api/v1/posts"), "Posts");
    assert_eq!(got("/api/v1/progress"), "Progress");
    assert_eq!(call(post(address, "/api/v1/cancel", "{}")), "Cancel");
    assert_eq!(
        call(post(address, "/api/v1/calculate", "{}")),
        "Calculate { which: All, wait: true }"
    );
    assert_eq!(
        call(post(
            address,
            "/api/v1/calculate",
            r#"{"toolpaths":[3,4],"wait":false}"#
        )),
        "Calculate { which: Ids([3, 4]), wait: false }"
    );
    assert_eq!(
        call(post(address, "/api/v1/preview", r#"{"toolpaths":7}"#)),
        "Preview { which: Ids([7]), wait: true }"
    );
    assert_eq!(
        call(post(
            address,
            "/api/v1/preview",
            r#"{"toolpaths":"visible"}"#
        )),
        "Preview { which: Visible, wait: true }"
    );
    assert_eq!(
        call(post(address, "/api/v1/preview/reset", "")),
        "ResetPreview"
    );
    assert_eq!(
        call(post(address, "/api/v1/gcode", "{}")),
        "Gcode(Gcode { toolpath: None, group: None, each: false, post: None, save_to: None, \
         replace: false, accept_warnings: false })"
    );
    assert_eq!(
        call(post(
            address,
            "/api/v1/gcode",
            r#"{"toolpath":2,"post":"GRBL (mm)","save_to":"/tmp/a.nc","accept_warnings":true}"#
        )),
        "Gcode(Gcode { toolpath: Some(2), group: None, each: false, post: Some(\"GRBL (mm)\"), \
         save_to: Some(\"/tmp/a.nc\"), replace: false, accept_warnings: true })"
    );
    assert_eq!(
        call(post(address, "/api/v1/job/save", "{}")),
        "Save { path: None }"
    );
    assert_eq!(
        call(post(
            address,
            "/api/v1/job/export",
            r#"{"format":"export_dxf","path":"/tmp/a.dxf"}"#
        )),
        "Export { format: \"export_dxf\", path: \"/tmp/a.dxf\" }"
    );
}

#[test]
fn a_wrong_new_call_says_what_to_send_instead() {
    let (address, _stop) = running(Some(TOKEN));
    let bad = get(address, "/api/v1/view.png?view=sideways", Some(TOKEN));
    assert_eq!(bad.status, 400);
    let said = bad.body["error"].as_str().unwrap();
    assert!(said.contains("top") && said.contains("iso"), "{said}");
    let width = get(address, "/api/v1/view.png?width=big", Some(TOKEN));
    assert_eq!(width.status, 400);
    let tiny = get(address, "/api/v1/view.png?width=2", Some(TOKEN));
    assert_eq!(tiny.status, 400, "{}", tiny.body);
    let which = post(address, "/api/v1/calculate", r#"{"toolpaths":"some"}"#);
    assert_eq!(which.status, 400);
    assert!(
        which.body["error"].as_str().unwrap().contains("visible"),
        "{}",
        which.body
    );
    assert_eq!(post(address, "/api/v1/job/export", "{}").status, 400);
    assert_eq!(get(address, "/api/v1/calculate", Some(TOKEN)).status, 405);
    assert_eq!(post(address, "/api/v1/toolpaths", "{}").status, 405);
    // both a toolpath and a group: which is meant is not clear
    let both = post(address, "/api/v1/gcode", r#"{"toolpath":1,"group":"Lid"}"#);
    assert_eq!(both.status, 400);
}

#[test]
fn a_picture_comes_back_as_a_png() {
    let server = Server::start(&settings(None), || {}).expect("a free port listens");
    let address = server.address();
    let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let stopped = stop.clone();
    std::thread::spawn(move || {
        while !stopped.load(std::sync::atomic::Ordering::Relaxed) {
            for p in server.take() {
                p.answer_png(vec![0x89, b'P', b'N', b'G']);
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    });
    let raw = get_raw(address, "/api/v1/view.png", "localhost");
    let text = String::from_utf8_lossy(&raw);
    assert!(text.starts_with("HTTP/1.1 200"), "{text}");
    assert!(text.contains("Content-Type: image/png"), "{text}");
    assert!(raw.ends_with(&[0x89, b'P', b'N', b'G']), "{text}");
    stop.store(true, std::sync::atomic::Ordering::Relaxed);
}

#[test]
fn the_description_lists_the_new_calls_and_shows_the_picture_as_one() {
    let doc = openapi::document(&[], &[], false);
    for (path, method) in [
        ("/view.png", "get"),
        ("/state", "get"),
        ("/toolpaths", "get"),
        ("/posts", "get"),
        ("/progress", "get"),
        ("/cancel", "post"),
        ("/calculate", "post"),
        ("/preview", "post"),
        ("/preview/reset", "post"),
        ("/gcode", "post"),
        ("/job/save", "post"),
        ("/job/export", "post"),
    ] {
        assert!(doc["paths"][path][method].is_object(), "{method} {path}");
    }
    let png = &doc["paths"]["/view.png"]["get"]["responses"]["200"]["content"];
    assert!(png["image/png"].is_object(), "{png}");
    let views = &doc["paths"]["/view.png"]["get"]["parameters"][0]["schema"]["enum"];
    assert_eq!(
        *views,
        serde_json::json!(["2d", "top", "front", "right", "iso"])
    );
}

#[test]
fn starting_and_opening_a_job_are_read_from_their_body() {
    let (address, _stop) = running(Some(TOKEN));
    let call = |r: Reply| {
        assert_eq!(r.status, 200, "{}", r.body);
        r.body["call"].as_str().unwrap().to_owned()
    };
    assert_eq!(
        call(post(address, "/api/v1/job/new", "{}")),
        "NewJob { settings: Object {}, discard: false }"
    );
    assert_eq!(
        call(post(
            address,
            "/api/v1/job/new",
            r#"{"settings":{"size":{"width":300}},"discard":true}"#
        )),
        "NewJob { settings: Object {\"size\": Object {\"width\": Number(300)}}, discard: true }"
    );
    assert_eq!(
        call(post(
            address,
            "/api/v1/job/open",
            r#"{"path":"/tmp/a.ccjob"}"#
        )),
        "Open { path: \"/tmp/a.ccjob\", discard: false }"
    );
    assert_eq!(post(address, "/api/v1/job/open", "{}").status, 400);
    assert_eq!(
        post(address, "/api/v1/job/new", r#"{"settings":3}"#).status,
        400
    );
    let doc = openapi::document(&[], &[], false);
    assert!(doc["paths"]["/job/new"]["post"].is_object());
    assert!(doc["paths"]["/job/open"]["post"].is_object());
}
