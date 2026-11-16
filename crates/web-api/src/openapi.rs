//! The `OpenAPI` description of the Web API that the docs page (`/docs`) shows. It is made from
//! the app's own tools and menu items each time it is asked for, so a tool added to Cut Craft
//! (or a gadget installed) is in it straight away, ready to try with its starting settings.

use serde_json::{Map, Value, json};

/// A tool as the description lists it.
pub struct Tool<'a> {
    pub id: &'a str,
    pub name: &'a str,
    pub group: &'a str,
    /// What its form starts with, offered as the settings to try.
    pub starts_with: Value,
}

/// The description: every call, with an example for each tool (`run`) and each button or
/// menu item (`invoke`). `token` is whether calls need the access token (other computers
/// allowed), so the page offers Authorize only then.
// one JSON document, read top to bottom as the page shows it
#[expect(clippy::too_many_lines)]
pub fn document(tools: &[Tool<'_>], actions: &[(String, String)], token: bool) -> Value {
    let error = json!({ "$ref": "#/components/responses/Error" });
    let answer = |what: &str| {
        json!({
            "200": { "description": what, "content": { "application/json": {} } },
            "400": error, "404": error, "409": error,
        })
    };
    let body = |schema: Value, examples: Map<String, Value>| {
        json!({
            "required": true,
            "content": { "application/json": { "schema": schema, "examples": examples } }
        })
    };
    let run_examples: Map<String, Value> = tools
        .iter()
        .map(|t| {
            (
                t.id.to_owned(),
                json!({
                    "summary": format!("{} ({})", t.name, t.group),
                    "value": { "command": t.id, "params": t.starts_with },
                }),
            )
        })
        .collect();
    let invoke_examples: Map<String, Value> = actions
        .iter()
        .map(|(id, name)| {
            (
                id.clone(),
                json!({ "summary": name, "value": { "action": id } }),
            )
        })
        .collect();
    let select_examples: Map<String, Value> = [
        (
            "layer",
            "Every vector on a layer",
            json!({ "by": "layer", "layer": "Layer 1" }),
        ),
        (
            "ids",
            "These vectors, by id (GET job lists them)",
            json!({ "by": "ids", "ids": [1, 2] }),
        ),
        (
            "name",
            "A named part",
            json!({ "by": "name", "name": "Lid" }),
        ),
        (
            "inside",
            "Every vector wholly inside a box",
            json!({ "by": "inside", "x1": 0, "y1": 0, "x2": 100, "y2": 50 }),
        ),
        ("all", "Every vector", json!({ "by": "all" })),
        ("open", "Every open vector", json!({ "by": "open" })),
        ("closed", "Every closed vector", json!({ "by": "closed" })),
        (
            "none",
            "Nothing (clears the selection)",
            json!({ "by": "none" }),
        ),
        (
            "add",
            "Add a layer to what is picked",
            json!({ "by": "layer", "layer": "Layer 1", "add": true }),
        ),
    ]
    .into_iter()
    .map(|(k, summary, value)| (k.to_owned(), json!({ "summary": summary, "value": value })))
    .collect();
    let ids: Vec<&str> = tools.iter().map(|t| t.id).collect();
    let action_ids: Vec<&str> = actions.iter().map(|(id, _)| id.as_str()).collect();
    let get = |summary: &str, description: &str, what: &str| {
        json!({ "get": {
            "summary": summary, "description": description, "responses": answer(what)
        } })
    };
    let mut doc = json!({
        "openapi": "3.0.3",
        "info": {
            "title": "Cut Craft Web API",
            "version": env!("CARGO_PKG_VERSION"),
            "description": "Drive Cut Craft the way you do with the mouse. Every call runs in \
                the open window, as a click would, and is one Undo step. Lengths are in the \
                job's units. Help ▸ Contents ▸ Web API explains more.",
        },
        "servers": [{ "url": "/api/v1" }],
        "paths": {
            "/status": get(
                "What Cut Craft is doing",
                "The version, whether a job is open, whether it is busy, the job's units, the \
                 open form and the status bar's message.",
                "Cut Craft's state",
            ),
            "/commands": get(
                "Every tool",
                "Every tool by id, with its name and group. POST run runs one.",
                "The tools",
            ),
            "/commands/{id}": {
                "get": {
                    "summary": "One tool's settings",
                    "description": "What each setting is (a JSON schema) and what the tool's \
                        form starts with on this job.",
                    "parameters": [{
                        "name": "id", "in": "path", "required": true,
                        "schema": { "type": "string", "enum": ids },
                    }],
                    "responses": answer("The tool's settings"),
                }
            },
            "/actions": get(
                "Every button and menu item",
                "Every menu item and tool button by id, with whether it can be used now.",
                "The buttons and menu items",
            ),
            "/job": get(
                "The job",
                "The layers, every vector (id, layer, part name, size, open or closed, \
                 selected), the toolpaths and the selection.",
                "The job",
            ),
            "/select": { "post": {
                "summary": "Pick vectors",
                "description": "Picks vectors as clicking them does. Vectors on hidden or \
                    locked layers are never picked. \"add\": true keeps what was picked.",
                "requestBody": body(json!({
                    "type": "object", "required": ["by"],
                    "properties": {
                        "by": { "type": "string", "enum":
                            ["ids", "layer", "name", "inside", "all", "open", "closed", "none"] },
                        "add": { "type": "boolean" },
                    },
                }), select_examples),
                "responses": answer("What is picked now"),
            } },
            "/run": { "post": {
                "summary": "Run a tool",
                "description": "Runs a tool straight away with these settings, with no \
                    form. Settings left out are what its form starts with; a misspelt one is \
                    refused with the names there are. Pick a tool under Examples to start \
                    from its settings.",
                "requestBody": body(json!({
                    "type": "object", "required": ["command"],
                    "properties": {
                        "command": { "type": "string", "enum": ids },
                        "params": { "type": "object" },
                    },
                }), run_examples),
                "responses": answer("What it made"),
            } },
            "/invoke": { "post": {
                "summary": "Press a button or menu item",
                "description": "Presses it as a click does. A greyed-out one is refused; Exit \
                    cannot be pressed from here.",
                "requestBody": body(json!({
                    "type": "object", "required": ["action"],
                    "properties": { "action": { "type": "string", "enum": action_ids } },
                }), invoke_examples),
                "responses": answer("What happened"),
            } },
        },
        "components": {
            "responses": { "Error": {
                "description": "It could not be done, and nothing was changed. The error \
                    says why and what to do.",
                "content": { "application/json": { "schema": {
                    "type": "object", "properties": { "error": { "type": "string" } }
                } } },
            } },
        },
    });
    if token {
        doc["components"]["securitySchemes"] = json!({
            "token": {
                "type": "http", "scheme": "bearer",
                "description": "The access token from Options ▸ Web API.",
            }
        });
        doc["security"] = json!([{ "token": [] }]);
    }
    doc
}
