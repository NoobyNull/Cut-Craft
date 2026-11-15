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
