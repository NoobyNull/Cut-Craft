//! The calls the Web API takes, read from a request's path and JSON body.

use crate::ApiError;
use serde::Deserialize;
use serde_json::Value;

/// One thing a caller asks Cut Craft to do or tell.
#[derive(Clone, Debug, PartialEq)]
pub enum Call {
    /// `GET status`: the version, whether a job is open, its units, the last message.
    Status,
    /// `GET commands`: every tool, with its name, group and settings.
    Commands,
    /// `GET commands/<id>`: one tool's settings: their schema and what its form starts with.
    Command(String),
    /// `GET actions`: every menu item and button, by id and name.
    Actions,
    /// `GET job`: the layers, vectors, toolpaths and selection.
    Job,
    /// `POST invoke {"action"}`: press a button or menu item, as a click does.
    Invoke(String),
    /// `POST run {"command", "params"}`: run a tool with these settings straight away, with
    /// no form; settings left out are what its form would start with.
    Run { command: String, params: Value },
    /// `POST select {"by", .., "add"}`: pick vectors; `add` keeps what was picked already.
    Select { pick: Pick, add: bool },
}

/// Which vectors `POST select` picks. Locked and hidden layers' vectors are never picked.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(tag = "by", rename_all = "snake_case", deny_unknown_fields)]
pub enum Pick {
    /// These vectors, by the ids `GET job` lists.
    Ids { ids: Vec<u64> },
    /// Every vector on the layer with this name (any case).
    Layer { layer: String },
    /// Every vector of the part with this name (Name Part, Import Cut List).
    Name { name: String },
    /// Every vector wholly inside this box. `units` is "mm" or "in"; the job's when left out.
    Inside {
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
        #[serde(default)]
        units: Option<String>,
    },
    /// Every vector.
    All,
    /// Every open vector (lines that do not join up).
    Open,
    /// Every closed vector (shapes).
    Closed,
    /// Nothing: clears the selection.
    None,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct InvokeBody {
    action: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RunBody {
    command: String,
    #[serde(default)]
    params: Option<Value>,
}

/// The call that `route` (the path after the prefix, no trailing slash) asks for.
pub(crate) fn parse(route: &str, post: bool, body: Value) -> Result<Call, ApiError> {
    let wrong_method = |want: &str| ApiError {
        status: 405,
        message: format!("Use {want} for {route}."),
    };
    match (route, post) {
        ("status", false) => Ok(Call::Status),
        ("commands", false) => Ok(Call::Commands),
        ("actions", false) => Ok(Call::Actions),
        ("job", false) => Ok(Call::Job),
        ("invoke", true) => {
            let b: InvokeBody = from(body, r#"{"action": "undo"}"#)?;
            Ok(Call::Invoke(b.action))
        }
        ("run", true) => {
            let b: RunBody = from(
                body,
                r#"{"command": "draw_circle", "params": {"radius": 10}}"#,
            )?;
            let params = b
                .params
                .unwrap_or_else(|| Value::Object(serde_json::Map::new()));
            if !params.is_object() {
                return Err(ApiError::bad("\"params\" must be an object of settings."));
            }
            Ok(Call::Run {
                command: b.command,
                params,
            })
        }
        ("select", true) => {
            let mut body = body;
            let add = match body.as_object_mut().and_then(|o| o.remove("add")) {
                None => false,
                Some(Value::Bool(add)) => add,
                Some(_) => return Err(ApiError::bad("\"add\" must be true or false.")),
            };
            let pick = from(body, r#"{"by": "layer", "layer": "Cut"}"#)?;
            Ok(Call::Select { pick, add })
        }
        ("status" | "commands" | "actions" | "job", true) => Err(wrong_method("GET")),
        ("invoke" | "run" | "select", false) => Err(wrong_method("POST")),
        (route, false) if route.starts_with("commands/") => {
            Ok(Call::Command(route["commands/".len()..].to_owned()))
        }
        _ => Err(ApiError::not_found(format!(
            "There is no call {route}. GET {}commands and {}actions list what there is.",
            crate::PREFIX,
            crate::PREFIX
        ))),
    }
}

fn from<T: serde::de::DeserializeOwned>(body: Value, example: &str) -> Result<T, ApiError> {
    serde_json::from_value(body)
        .map_err(|e| ApiError::bad(format!("{e}. The body should look like {example}.")))
}
