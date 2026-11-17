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
                 open form and the status bar's message. progress is GET progress's answer.",
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
    if let Value::Object(paths) = &mut doc["paths"] {
        paths.extend(more_paths(&answer, &body));
    }
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

/// The export formats `job/export` takes, by id with their names, added to the description
/// (they come from the app's own Export menu).
pub fn add_export_formats(doc: &mut Value, formats: &[(String, String)]) {
    let ids: Vec<&str> = formats.iter().map(|(id, _)| id.as_str()).collect();
    let content =
        &mut doc["paths"]["/job/export"]["post"]["requestBody"]["content"]["application/json"];
    content["schema"]["properties"]["format"]["enum"] = json!(ids);
    let examples: Map<String, Value> = formats
        .iter()
        .map(|(id, name)| {
            (
                id.clone(),
                json!({ "summary": name, "value": { "format": id, "path": "/home/me/sign" } }),
            )
        })
        .collect();
    content["examples"] = Value::Object(examples);
}

/// The calls that show what the job makes and run the slow jobs: pictures of the views, the
/// state, the toolpaths with their times, calculating and previewing, G-code, saving.
// one JSON document, read top to bottom as the page shows it
#[expect(clippy::too_many_lines)]
fn more_paths(
    answer: &dyn Fn(&str) -> Value,
    body: &dyn Fn(Value, Map<String, Value>) -> Value,
) -> Map<String, Value> {
    let get = |summary: &str, description: &str, what: &str| {
        json!({ "get": {
            "summary": summary, "description": description, "responses": answer(what)
        } })
    };
    let examples = |list: &[(&str, &str, Value)]| -> Map<String, Value> {
        list.iter()
            .map(|(k, summary, value)| {
                (
                    (*k).to_owned(),
                    json!({ "summary": summary, "value": value }),
                )
            })
            .collect()
    };
    let which = json!({
        "description": "\"all\", \"visible\", one toolpath's id, or a list of ids",
        "oneOf": [
            { "type": "string", "enum": ["all", "visible"] },
            { "type": "integer" },
            { "type": "array", "items": { "type": "integer" } },
        ],
    });
    let work = |what: &str| {
        json!({
            "type": "object",
            "properties": {
                "toolpaths": which,
                "wait": { "type": "boolean", "description": format!(
                    "Answer once {what} is done (the default); false answers straight away, \
                     and GET progress follows it") },
            },
        })
    };
    let mut picture = answer("The picture");
    picture["200"] = json!({
        "description": "The picture, as a PNG",
        "content": { "image/png": { "schema": { "type": "string", "format": "binary" } } },
    });
    picture["503"] = json!({ "$ref": "#/components/responses/Error" });
    let query = |name: &str, description: &str, schema: Value| {
        json!({ "name": name, "in": "query", "required": false,
                "description": description, "schema": schema })
    };
    let views: Vec<&str> = crate::Look::names().collect();
    let mut paths = Map::new();
    paths.insert(
        "/view.png".into(),
        json!({ "get": {
        "summary": "A picture of the 2D or 3D view",
        "description": "The view as Cut Craft's window draws it, as a PNG. The window shows \
            that view for a moment while the picture is taken, then goes back to how it was. \
            Open this in a web browser to see the job.",
        "parameters": [
            query("view", "2d, or the 3D view from the top, front, right or at an angle (iso)",
                json!({ "type": "string", "enum": views, "default": "iso" })),
            query("preview", "after: the toolpath preview, if there is one; before: the \
                model as it is before cutting",
                json!({ "type": "string", "enum": ["after", "before"], "default": "after" })),
            query("width", "How wide the picture is, pixels; the view's own size if left out",
                json!({ "type": "integer", "minimum": crate::call::WIDTHS.start(),
                        "maximum": crate::call::WIDTHS.end() })),
        ],
        "responses": picture,
    } }),
    );
    paths.insert(
        "/state".into(),
        get(
            "The material, the model and what is picked",
            "The material as Job Setup has it (size, thickness, where Z zero and the datum \
         are), where the 3D model sits in it (Material Setup) with each part of the model \
         and its heights, and what is picked: vectors, model parts and the toolpath picked \
         in the toolpath list. Lengths are in the job's units.",
            "The state",
        ),
    );
    paths.insert(
        "/toolpaths".into(),
        get(
            "Every toolpath with its run time and warnings",
            "Each toolpath in list order: its tool, whether it is shown, its run time as the \
         Toolpath Summary works it out, and what to look at before cutting it (out of \
         date, entries made another way, what the checks before cutting found). The shown \
         ones' times are added up.",
            "The toolpaths",
        ),
    );
    paths.insert(
        "/posts".into(),
        get(
            "The post processors to make G-code with",
            "The active machine's post processors that suit this job, and which is used when \
         POST gcode names none.",
            "The posts",
        ),
    );
    paths.insert(
        "/progress".into(),
        get(
            "How far a slow job has got",
            "Whether Cut Craft is calculating or previewing, what, how far along (0 to 1, when \
         it can tell), for how long, and the time left as the progress window says it \
         (seconds_left once it can judge, time_left in words). Answered straight away, even \
         while busy.",
            "The progress",
        ),
    );
    paths.insert(
        "/cancel".into(),
        json!({ "post": {
        "summary": "Cancel the slow job",
        "description": "Presses Cancel on the progress window: the job stops and nothing \
            changes.",
        "responses": answer("What was cancelled"),
    } }),
    );
    paths.insert(
        "/calculate".into(),
        json!({ "post": {
        "summary": "Calculate toolpaths",
        "description": "Recalculates these toolpaths, as Recalculate does, with its \
            progress window. Other calls wait until it is done.",
        "requestBody": body(work("calculating"), examples(&[
            ("all", "Every toolpath", json!({ "toolpaths": "all" })),
            ("visible", "The shown ones", json!({ "toolpaths": "visible" })),
            ("one", "One toolpath", json!({ "toolpaths": 3 })),
            ("no_wait", "Answer straight away", json!({ "toolpaths": "all", "wait": false })),
        ])),
        "responses": answer("The toolpaths, calculated"),
    } }),
    );
    paths.insert(
        "/preview".into(),
        json!({ "post": {
        "summary": "Preview toolpaths",
        "description": "Cuts these toolpaths, in list order, into fresh material, as \
            Preview does (without playing it). GET view.png then shows the result.",
        "requestBody": body(work("the preview"), examples(&[
            ("all", "Every toolpath", json!({ "toolpaths": "all" })),
            ("visible", "The shown ones", json!({ "toolpaths": "visible" })),
            ("one", "One toolpath", json!({ "toolpaths": 3 })),
        ])),
        "responses": answer("What was previewed"),
    } }),
    );
    paths.insert(
        "/preview/reset".into(),
        json!({ "post": {
        "summary": "Reset the preview",
        "description": "Takes the preview away, as Reset Preview does; Undo Last Preview \
            brings it back.",
        "responses": answer("What happened"),
    } }),
    );
    paths.insert("/gcode".into(), json!({ "post": {
        "summary": "Make the G-code",
        "description": "The G-code Save Toolpaths would write, each file's name and text, \
            after the same checks before cutting. An error stops it (nothing is made); \
            warnings stop it too until \"accept_warnings\": true, as Save Anyway. With \
            \"save_to\", the files are saved there as well. Nothing is sent to the machine: \
            Cut Craft is not a G-code sender.",
        "requestBody": body(json!({
            "type": "object",
            "properties": {
                "toolpath": { "type": "integer", "description": "Only this toolpath" },
                "group": { "type": "string", "description": "Only this toolpath group" },
                "each": { "type": "boolean", "description": "Each shown toolpath in a file of its own" },
                "post": { "type": "string", "description": "A post from GET posts; the machine's default if left out" },
                "save_to": { "type": "string", "description": "Save the files here too" },
                "replace": { "type": "boolean", "description": "Write over a file already there" },
                "accept_warnings": { "type": "boolean", "description": "Go ahead though the checks found warnings" },
            },
        }), examples(&[
            ("shown", "Every shown toolpath, in one file", json!({})),
            ("one", "One toolpath", json!({ "toolpath": 3 })),
            ("each", "Each shown toolpath in a file of its own", json!({ "each": true })),
            ("save", "Saved as well", json!({ "save_to": "/home/me/sign.nc" })),
        ])),
        "responses": answer("The G-code"),
    } }));
    paths.insert(
        "/job/save".into(),
        json!({ "post": {
        "summary": "Save the job",
        "description": "Save, or Save As when a path is given (the job is from then on that \
            file, as Save As makes it).",
        "requestBody": body(json!({
            "type": "object",
            "properties": { "path": { "type": "string" } },
        }), examples(&[
            ("save", "Save where it is", json!({})),
            ("save_as", "Save As", json!({ "path": "/home/me/sign.ccjob" })),
        ])),
        "responses": answer("Where it was saved"),
    } }),
    );
    paths.insert(
        "/notes".into(),
        json!({
            "get": {
                "summary": "The job's notes, with their variables filled in",
                "description": "The notes as typed (Markdown-like: # headings, - lists, \
                    - [ ] check boxes, **bold**, *italic*, [links](https://…) and \
                    {{variables}}; never HTML), whether they open with the job and go at the \
                    top of the G-code, the notes filled in as plain text and as HTML, and \
                    every variable (key, group, name, what it is, value now): the Variable \
                    Explorer's list.",
                "responses": answer("The notes"),
            },
            "post": {
                "summary": "Change the job's notes",
                "description": "Sets the notes (one undo, as Apply on the Notes form). What \
                    is left out stays as it is. Answers as GET notes.",
                "requestBody": body(json!({
                    "type": "object",
                    "properties": {
                        "text": { "type": "string", "description": "The notes, with {{variables}}" },
                        "in_gcode": { "type": "boolean", "description": "Put them at the top of the G-code" },
                        "opens_with_job": { "type": "boolean", "description": "Open them when the job opens" },
                    },
                }), examples(&[
                    ("write", "Notes with a check list and a variable", json!({
                        "text": "# Setup\n- [ ] Clamp the {{material.thickness}} board",
                        "in_gcode": true,
                    })),
                ])),
                "responses": answer("The notes"),
            },
        }),
    );
    paths.insert(
        "/job/new".into(),
        json!({ "post": {
        "summary": "Start a new job",
        "description": "New, with Job Setup's OK pressed: settings left out are what Job \
            Setup starts with, the saved default job's (GET commands/job_setup lists them). \
            Settings Job Setup refuses leave the job open as it was. Unsaved work is kept, and \
            the call refused, unless \"discard\": true.",
        "requestBody": body(json!({
            "type": "object",
            "properties": {
                "settings": { "type": "object" },
                "discard": { "type": "boolean" },
            },
        }), examples(&[
            ("plain", "As Job Setup starts", json!({})),
            ("size", "A 300 x 150 x 12 mm board", json!({ "settings": { "size":
                { "width": 300, "height": 150, "thickness": 12 } } })),
        ])),
        "responses": answer("The new job's state"),
    } }),
    );
    paths.insert(
        "/job/open".into(),
        json!({ "post": {
        "summary": "Open a job",
        "description": "Open, on this job file. Unsaved work is kept, and the call refused, \
            unless \"discard\": true.",
        "requestBody": body(json!({
            "type": "object", "required": ["path"],
            "properties": {
                "path": { "type": "string" },
                "discard": { "type": "boolean" },
            },
        }), examples(&[
            ("open", "Open a job", json!({ "path": "/home/me/sign.ccjob" })),
        ])),
        "responses": answer("The job's state"),
    } }),
    );
    paths.insert(
        "/job/export".into(),
        json!({ "post": {
        "summary": "Export the job",
        "description": "An Export menu item (vectors as DXF, SVG and the like), saved to \
            path.",
        "requestBody": body(json!({
            "type": "object", "required": ["format", "path"],
            "properties": {
                "format": { "type": "string" },
                "path": { "type": "string" },
            },
        }), Map::new()),
        "responses": answer("Where it was saved"),
    } }),
    );
    paths
}
