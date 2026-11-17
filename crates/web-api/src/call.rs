//! The calls the Web API takes, read from a request's path and JSON body.

use crate::ApiError;
use serde::Deserialize;
use serde_json::Value;

/// One thing a caller asks Cut Craft to do or tell.
#[derive(Clone, Debug, PartialEq)]
pub enum Call {
    /// `GET openapi.json`: the description of every call, which the docs page shows.
    OpenApi,
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
    /// `GET view.png?view&preview&width`: a picture of the 2D view or the 3D view from one
    /// side, as the window draws it.
    View(Picture),
    /// `GET state`: the material, where the model sits in it, and what is picked.
    State,
    /// `GET toolpaths`: each toolpath with its run time, and the checks before cutting.
    Toolpaths,
    /// `GET posts`: the post processors G-code can be made through for this job.
    Posts,
    /// `GET progress`: what a slow job (calculating, previewing) is doing and how far along.
    Progress,
    /// `POST cancel`: Cancel pressed on the slow job's progress window.
    Cancel,
    /// `POST calculate {"toolpaths", "wait"}`: Recalculate on these toolpaths.
    Calculate { which: Which, wait: bool },
    /// `POST preview {"toolpaths", "wait"}`: Preview these toolpaths on fresh material.
    Preview { which: Which, wait: bool },
    /// `POST preview/reset`: Reset Preview.
    ResetPreview,
    /// `POST gcode {..}`: the G-code Save Toolpaths would write, shown and (asked) saved.
    Gcode(Gcode),
    /// `POST job/save {"path"}`: Save, or Save As to `path`.
    Save { path: Option<String> },
    /// `POST job/export {"format", "path"}`: an Export menu item, saving to `path`.
    Export { format: String, path: String },
    /// `GET notes`: the job's notes as typed, with their variables filled in (plain text
    /// and HTML), and every variable notes can show with its value now.
    Notes,
    /// `POST notes {"text", "in_gcode", "opens_with_job"}`: change the notes (one undo);
    /// what is left out stays as it is.
    SetNotes(NotesBody),
    /// `POST job/new {"settings", "discard"}`: New, with Job Setup's OK pressed on these
    /// settings over what it starts with; `discard` throws away unsaved work.
    NewJob { settings: Value, discard: bool },
    /// `POST job/open {"path", "discard"}`: Open this job file.
    Open { path: String, discard: bool },
}

/// What `POST notes` changes; each left out stays as it is.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NotesBody {
    /// The notes, written like Markdown with `{{variables}}`.
    #[serde(default)]
    pub text: Option<String>,
    /// Put the notes at the top of the G-code.
    #[serde(default)]
    pub in_gcode: Option<bool>,
    /// Open the notes by themselves when the job is opened.
    #[serde(default)]
    pub opens_with_job: Option<bool>,
}

/// What `GET view.png` shows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Picture {
    pub view: Look,
    pub preview: Shown,
    /// How wide the picture is, pixels; the view's own size when `None`.
    pub width: Option<u32>,
}

/// Which view, and from where.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Look {
    /// The 2D view, as it is zoomed now.
    TwoD,
    /// The 3D view from straight above.
    Top,
    /// The 3D view from the front edge.
    Front,
    /// The 3D view from the right edge.
    Right,
    /// The 3D view at an angle, as it first opens.
    Iso,
}

impl Look {
    const NAMES: [(&'static str, Self); 5] = [
        ("2d", Self::TwoD),
        ("top", Self::Top),
        ("front", Self::Front),
        ("right", Self::Right),
        ("iso", Self::Iso),
    ];

    /// Every name `view` takes, in the docs' order.
    pub fn names() -> impl Iterator<Item = &'static str> {
        Self::NAMES.iter().map(|(n, _)| *n)
    }
}

/// Whether the 3D view shows the toolpath preview or the model before it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shown {
    /// The model (or the uncut material), as before any preview.
    Before,
    /// The toolpath preview if there is one, else the model.
    After,
}

/// Which toolpaths `calculate` and `preview` work on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Which {
    /// Every toolpath, shown or not (Recalculate All, Preview All).
    All,
    /// Only the shown (ticked) ones.
    Visible,
    /// These, by the ids `GET toolpaths` lists, in list order.
    Ids(Vec<u64>),
}

/// What `POST gcode` makes: as Save Toolpaths' form would.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Gcode {
    /// Only this toolpath; else every shown one.
    #[serde(default)]
    pub toolpath: Option<u64>,
    /// Only this toolpath group's toolpaths, as one file.
    #[serde(default)]
    pub group: Option<String>,
    /// Each shown toolpath in a file of its own, not all in one.
    #[serde(default)]
    pub each: bool,
    /// The post processor, by the name `GET posts` lists; the machine's default if left out.
    #[serde(default)]
    pub post: Option<String>,
    /// Where to save it as well (the file, as Save Toolpaths names it); shown only if left out.
    #[serde(default)]
    pub save_to: Option<String>,
    /// Write over a file already there.
    #[serde(default)]
    pub replace: bool,
    /// Go ahead though the checks before cutting found warnings (as answering Save Anyway).
    #[serde(default)]
    pub accept_warnings: bool,
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
struct WorkBody {
    #[serde(default)]
    toolpaths: Option<Value>,
    #[serde(default)]
    wait: Option<bool>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SaveBody {
    #[serde(default)]
    path: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct NewBody {
    #[serde(default)]
    settings: Option<Value>,
    #[serde(default)]
    discard: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct OpenBody {
    path: String,
    #[serde(default)]
    discard: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ExportBody {
    format: String,
    path: String,
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
pub(crate) fn parse(route: &str, post: bool, body: Value, query: &str) -> Result<Call, ApiError> {
    let wrong_method = |want: &str| ApiError {
        status: 405,
        message: format!("Use {want} for {route}."),
    };
    match (route, post) {
        ("status", false) => Ok(Call::Status),
        ("openapi.json", false) => Ok(Call::OpenApi),
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
        ("view.png", false) => Ok(Call::View(picture(query)?)),
        ("state", false) => Ok(Call::State),
        ("toolpaths", false) => Ok(Call::Toolpaths),
        ("posts", false) => Ok(Call::Posts),
        ("progress", false) => Ok(Call::Progress),
        ("cancel", true) => Ok(Call::Cancel),
        ("calculate", true) => {
            let (which, wait) = work(body, r#"{"toolpaths": "all", "wait": true}"#)?;
            Ok(Call::Calculate { which, wait })
        }
        ("preview", true) => {
            let (which, wait) = work(body, r#"{"toolpaths": [3], "wait": true}"#)?;
            Ok(Call::Preview { which, wait })
        }
        ("preview/reset", true) => Ok(Call::ResetPreview),
        ("gcode", true) => {
            let g: Gcode = from(body, r#"{"toolpath": 3, "post": "GRBL (mm)"}"#)?;
            if g.toolpath.is_some() && g.group.is_some() {
                return Err(ApiError::bad(
                    "Give \"toolpath\" or \"group\", not both: which is meant is not clear.",
                ));
            }
            Ok(Call::Gcode(g))
        }
        ("job/save" | "job/export" | "job/new" | "job/open", true) => job_file(route, body),
        ("notes", false) => Ok(Call::Notes),
        ("notes", true) => Ok(Call::SetNotes(from(
            body,
            r#"{"text": "Clamp the {{material.thickness}} board", "in_gcode": true}"#,
        )?)),
        (
            "status" | "commands" | "actions" | "job" | "view.png" | "state" | "toolpaths"
            | "posts" | "progress",
            true,
        ) => Err(wrong_method("GET")),
        (
            "invoke" | "run" | "select" | "cancel" | "calculate" | "preview" | "preview/reset"
            | "gcode" | "job/save" | "job/export" | "job/new" | "job/open",
            false,
        ) => Err(wrong_method("POST")),
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

/// The smallest and largest picture `width` takes, pixels.
pub const WIDTHS: std::ops::RangeInclusive<u32> = 16..=8192;

/// `view.png`'s settings, from its query (`view=top&preview=before&width=800`).
fn picture(query: &str) -> Result<Picture, ApiError> {
    let mut picture = Picture {
        view: Look::Iso,
        preview: Shown::After,
        width: None,
    };
    for (key, value) in query_pairs(query) {
        match key.as_str() {
            "view" => {
                picture.view = Look::NAMES
                    .iter()
                    .find(|(n, _)| value.eq_ignore_ascii_case(n))
                    .map(|(_, l)| *l)
                    .ok_or_else(|| {
                        ApiError::bad(format!(
                            "\"view\" is \"{value}\": use one of {}.",
                            Look::names().collect::<Vec<_>>().join(", ")
                        ))
                    })?;
            }
            "preview" => {
                picture.preview = match value.to_ascii_lowercase().as_str() {
                    "before" => Shown::Before,
                    "after" => Shown::After,
                    _ => {
                        return Err(ApiError::bad(format!(
                            "\"preview\" is \"{value}\": use before (the model) or after (the \
                             toolpath preview)."
                        )));
                    }
                };
            }
            "width" => {
                let width = value.parse().ok().filter(|w| WIDTHS.contains(w));
                picture.width = Some(width.ok_or_else(|| {
                    ApiError::bad(format!(
                        "\"width\" is \"{value}\": give pixels, {} to {}.",
                        WIDTHS.start(),
                        WIDTHS.end()
                    ))
                })?);
            }
            _ => {
                return Err(ApiError::bad(format!(
                    "view.png takes view, preview and width, not \"{key}\"."
                )));
            }
        }
    }
    Ok(picture)
}

/// A query's `key=value` pairs, `%XX` and `+` read as what they stand for.
fn query_pairs(query: &str) -> Vec<(String, String)> {
    query
        .split('&')
        .filter(|p| !p.is_empty())
        .map(|p| {
            let (k, v) = p.split_once('=').unwrap_or((p, ""));
            (unescape(k), unescape(v))
        })
        .collect()
}

fn unescape(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => out.push(b' '),
            b'%' => match text.get(i + 1..i + 3).map(|h| u8::from_str_radix(h, 16)) {
                Some(Ok(b)) => {
                    out.push(b);
                    i += 2;
                }
                _ => out.push(b'%'),
            },
            b => out.push(b),
        }
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// `calculate`'s and `preview`'s body: which toolpaths (all of them when left out) and
/// whether to answer once done (yes when left out).
fn work(body: Value, example: &str) -> Result<(Which, bool), ApiError> {
    let b: WorkBody = from(body, example)?;
    let which = match b.toolpaths {
        None => Which::All,
        Some(Value::String(s)) if s.eq_ignore_ascii_case("all") => Which::All,
        Some(Value::String(s)) if s.eq_ignore_ascii_case("visible") => Which::Visible,
        Some(Value::Number(n)) if n.as_u64().is_some() => Which::Ids(vec![n.as_u64().unwrap_or(0)]),
        Some(Value::Array(ids)) if ids.iter().all(|v| v.as_u64().is_some()) => {
            Which::Ids(ids.iter().filter_map(Value::as_u64).collect())
        }
        Some(other) => {
            return Err(ApiError::bad(format!(
                "\"toolpaths\" is {other}: give \"all\", \"visible\", one toolpath's id or a \
                 list of ids (GET {}toolpaths lists them).",
                crate::PREFIX
            )));
        }
    };
    Ok((which, b.wait.unwrap_or(true)))
}

/// The calls on the job's file: save, export, new and open.
fn job_file(route: &str, body: Value) -> Result<Call, ApiError> {
    match route {
        "job/save" => {
            let b: SaveBody = from(body, r#"{"path": "/home/me/sign.ccjob"}"#)?;
            Ok(Call::Save { path: b.path })
        }
        "job/export" => {
            let b: ExportBody = from(
                body,
                r#"{"format": "export_dxf", "path": "/home/me/sign.dxf"}"#,
            )?;
            Ok(Call::Export {
                format: b.format,
                path: b.path,
            })
        }
        "job/new" => {
            let b: NewBody = from(body, r#"{"settings": {"size": {"width": 300}}}"#)?;
            let settings = b
                .settings
                .unwrap_or_else(|| Value::Object(serde_json::Map::new()));
            if !settings.is_object() {
                return Err(ApiError::bad(
                    "\"settings\" must be an object of Job Setup's settings.",
                ));
            }
            Ok(Call::NewJob {
                settings,
                discard: b.discard,
            })
        }
        "job/open" => {
            let b: OpenBody = from(body, r#"{"path": "/home/me/sign.ccjob"}"#)?;
            Ok(Call::Open {
                path: b.path,
                discard: b.discard,
            })
        }
        _ => unreachable!("not a job file call: {route}"),
    }
}
