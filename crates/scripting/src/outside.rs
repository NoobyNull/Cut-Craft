//! Gadgets that reach outside the job: an add-on that declares the web hosts it uses can
//! fetch from them, save downloads in its own folder and keep a few settings between runs.
//!
//! These gadgets never touch the document, so they are not commands: PowerWhittle runs one only
//! when the person asks, never for a preview. Everything else about the sandbox stays the same:
//! no files outside the add-on's folder, no programs, no other hosts, no plain `http://`.
//!
//! The Lua it sees, beside `string`, `table`, `math` and `utf8`:
//!
//! - `cutcraft.http{ method = "GET", url = "https://...", headers = {...}, body = "..." }`
//!   → `{ status = 200, body = "..." }`. The body is text and at most [`Outside::max_body`].
//! - `cutcraft.download(url, file_name)` → `{ path = "...", size = n }`, saved in the add-on's
//!   `files` folder under a plain file name, at most [`Outside::max_download`] bytes.
//! - `cutcraft.store.get(key)` and `cutcraft.store.set(key, value)`: short strings kept in the
//!   add-on's folder between runs.
//! - `cutcraft.sleep(seconds)`, at most a minute, and `cutcraft.message(text)` / `print`.
//!
//! Redirects are followed by hand, at most five, and each one must stay on a listed host.
//! `Authorization` and `Cookie` are dropped when a redirect leaves the first origin.
//!
//! The caller keeps [`Outside::folder`] outside the add-on's install folder (so an update,
//! which replaces that folder, keeps settings and downloads) and never runs two outside
//! gadgets of one add-on at the same time.

use crate::{Limits, ScriptError, budget, sandbox, script_error};
use mlua::chunk::ChunkMode;
use mlua::{LuaSerdeExt, Table};
use serde_json::Value;
use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::Duration;

/// What one add-on's gadgets may reach.
#[derive(Clone, Debug)]
pub struct Outside {
    /// Host names from the add-on's `network` list, such as `relay.example.com`.
    pub hosts: Vec<String>,
    /// The add-on's data folder, such as `<config>/addon-data/<id>/`, kept apart from its
    /// install folder: downloads go in its `files` folder, settings in `store.json`.
    pub folder: PathBuf,
    /// Largest text answer `cutcraft.http` returns, in bytes.
    pub max_body: u64,
    /// Largest file `cutcraft.download` saves, in bytes.
    pub max_download: u64,
    /// Lets tests talk to a server on this computer over plain `http://`.
    #[cfg(test)]
    pub(crate) plain_http_for_tests: bool,
}

impl Outside {
    /// The usual limits: 1 MB answers, 2 GB downloads.
    pub fn new(hosts: Vec<String>, folder: PathBuf) -> Self {
        Self {
            hosts,
            folder,
            max_body: 1 << 20,
            max_download: 2 << 30,
            #[cfg(test)]
            plain_http_for_tests: false,
        }
    }

    fn files(&self) -> PathBuf {
        self.folder.join("files")
    }

    /// `url` when it is `https://` to a listed host on the usual port, else why not.
    fn check(&self, url: &str) -> Result<ureq::http::Uri, String> {
        let uri: ureq::http::Uri = url
            .parse()
            .map_err(|_| format!("\"{url}\" is not a web address"))?;
        let plain = {
            #[cfg(test)]
            {
                self.plain_http_for_tests
            }
            #[cfg(not(test))]
            {
                false
            }
        };
        match uri.scheme_str() {
            Some("https") => {}
            Some("http") if plain => {}
            _ => {
                return Err(format!(
                    "only https:// addresses are allowed, not \"{url}\""
                ));
            }
        }
        let authority = uri
            .authority()
            .map_or("", ureq::http::uri::Authority::as_str);
        if authority.contains('@') {
            return Err(format!("\"{url}\" has a user name in it"));
        }
        if uri.port_u16().is_some_and(|p| p != 443) && !plain {
            return Err(format!("\"{url}\" uses an unusual port"));
        }
        let host = uri.host().unwrap_or_default().to_ascii_lowercase();
        if !self.hosts.iter().any(|h| h.eq_ignore_ascii_case(&host)) {
            return Err(format!(
                "this add-on may not reach {host}; it lists only {}",
                self.hosts.join(", ")
            ));
        }
        Ok(uri)
    }
}

/// Whether `name` is a plain file name: no folders, no `..`, nothing hidden or odd.
pub fn plain_file_name(name: &str) -> bool {
    // names Windows keeps for devices, even with an extension (`NUL.stl`)
    const DEVICES: [&str; 28] = [
        "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
        "COM9", "COM¹", "COM²", "COM³", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7",
        "LPT8", "LPT9", "LPT¹", "LPT²", "LPT³",
    ];
    let stem = name.split('.').next().unwrap_or("").trim_end();
    !name.is_empty()
        && name.len() <= 200
        && !name.starts_with('.')
        && !name.ends_with(['.', ' '])
        && !DEVICES.iter().any(|d| stem.eq_ignore_ascii_case(d))
        && !name.chars().any(|c| {
            c.is_control() || matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|')
        })
}

/// Text answers must arrive whole within a minute; a download may take up to `body` in all.
fn agent(body: Duration) -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_connect(Some(Duration::from_secs(15)))
        .timeout_recv_response(Some(Duration::from_secs(60)))
        .timeout_recv_body(Some(body))
        .timeout_send_request(Some(Duration::from_secs(60)))
        .timeout_send_body(Some(Duration::from_secs(300)))
        .user_agent(concat!("Cut-Craft/", env!("CARGO_PKG_VERSION")))
        .http_status_as_error(false)
        .max_redirects(0)
        .tls_config(
            ureq::tls::TlsConfig::builder()
                .root_certs(ureq::tls::RootCerts::PlatformVerifier)
                .build(),
        )
        .build()
        .into()
}

/// Sends one request, following up to five redirects that each stay on a listed host.
fn send(
    outside: &Outside,
    agent: &ureq::Agent,
    method: &str,
    url: &str,
    headers: &BTreeMap<String, String>,
    body: Option<&str>,
) -> Result<ureq::http::Response<ureq::Body>, String> {
    let mut url = url.to_owned();
    let (mut method, mut body) = (method.to_ascii_uppercase(), body.map(str::to_owned));
    let first_origin = origin(&url);
    for _ in 0..=5 {
        let uri = outside.check(&url)?;
        let same_origin = first_origin.is_some() && origin(&url) == first_origin;
        let mut request = ureq::http::Request::builder()
            .method(method.as_str())
            .uri(uri);
        for (k, v) in headers {
            let named = |n: &str| k.eq_ignore_ascii_case(n);
            let secret = named("authorization") || named("cookie");
            // the client writes Host for each address itself, and a 303 drops the body
            let framing = named("host")
                || (body.is_none()
                    && [
                        "content-length",
                        "content-type",
                        "content-encoding",
                        "transfer-encoding",
                    ]
                    .into_iter()
                    .any(named));
            if (same_origin || !secret) && !framing {
                request = request.header(k.as_str(), v.as_str());
            }
        }
        let request = request
            .body(body.clone().unwrap_or_default())
            .map_err(|e| format!("the request could not be made: {e}"))?;
        let response = agent
            .run(request)
            .map_err(|e| format!("could not reach {url}: {e}"))?;
        if !matches!(response.status().as_u16(), 301 | 302 | 303 | 307 | 308) {
            return Ok(response);
        }
        let next = response
            .headers()
            .get("location")
            .and_then(|l| l.to_str().ok())
            .ok_or_else(|| format!("{url} sent a redirect with nowhere to go"))?;
        url = resolve(&url, next)?;
        if response.status() == 303 {
            (method, body) = ("GET".to_owned(), None);
        }
    }
    Err("the address redirected too many times".to_owned())
}

/// The origin of `url` (scheme, host and effective port), so `https://a:443` and
/// `https://a` are the same site; `None` when `url` cannot be read.
fn origin(url: &str) -> Option<url::Origin> {
    url::Url::parse(url).ok().map(|u| u.origin())
}

/// `next` from a `Location` header, resolved against `from` as a URI reference (RFC 3986,
/// by the standard resolver): absolute, `//host/...`, `/path`, `?query`, `#fragment`, or a
/// path relative to `from`'s folder, with `.` and `..` segments removed.
fn resolve(from: &str, next: &str) -> Result<String, String> {
    url::Url::parse(from)
        .and_then(|base| base.join(next))
        .map(String::from)
        .map_err(|e| format!("{from} redirected to an address that is not valid ({e})"))
}

/// The add-on's saved settings: none yet when there is no file, an error when the file is
/// there but cannot be read, so a run never saves over settings it could not see.
fn load_store(path: &Path) -> Result<BTreeMap<String, String>, String> {
    match std::fs::read_to_string(path) {
        Ok(text) => serde_json::from_str(&text)
            .map_err(|e| format!("the add-on's saved settings are damaged ({e})")),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(BTreeMap::new()),
        Err(e) => Err(format!(
            "the add-on's saved settings could not be read ({e})"
        )),
    }
}

/// Runs an outside gadget's `run(settings)` with what `outside` allows, and returns the
/// messages it gave for the person.
pub fn run_outside_gadget(
    name: &str,
    source: &str,
    settings: &Value,
    limits: Limits,
    outside: &Outside,
) -> Result<Vec<String>, ScriptError> {
    let lua = sandbox(limits)?;
    let stopped = Rc::new(Cell::new(false));
    budget(&lua, limits.instructions, Rc::clone(&stopped))?;
    let agent = agent(Duration::from_secs(60));
    let download_agent = self::agent(Duration::from_hours(1));
    let notes = RefCell::new(Vec::<String>::new());
    let note_bytes = Cell::new(0_usize);
    let store_path = outside.folder.join("store.json");
    let store = RefCell::new(load_store(&store_path).map_err(ScriptError::Failed)?);

    let result = lua.scope(|scope| {
        let cutcraft = lua.create_table()?;
        cutcraft.set(
            "message",
            scope.create_function(|_, text: String| {
                // at most 200 messages and 64 KB in all; the rest are dropped
                let mut notes = notes.borrow_mut();
                if notes.len() < 200 && note_bytes.get() + text.len() <= 64 * 1024 {
                    note_bytes.set(note_bytes.get() + text.len());
                    notes.push(text);
                }
                Ok(())
            })?,
        )?;
        cutcraft.set(
            "sleep",
            scope.create_function(|_, seconds: f64| {
                if !seconds.is_finite() {
                    return Err(mlua::Error::runtime(
                        "cutcraft.sleep needs a number of seconds",
                    ));
                }
                std::thread::sleep(Duration::from_secs_f64(seconds.clamp(0.0, 60.0)));
                Ok(())
            })?,
        )?;
        cutcraft.set(
            "http",
            scope
                .create_function(|lua, request: Table| http_call(lua, outside, &agent, &request))?,
        )?;
        cutcraft.set(
            "download",
            scope.create_function(|lua, (url, file_name): (String, String)| {
                // the file waits beside its place until the answer is whole, so running out
                // of memory here cannot leave a saved file the gadget was told failed
                let staged = download(outside, &download_agent, &url, &file_name)
                    .map_err(mlua::Error::runtime)?;
                let answer = (|| {
                    let answer = lua.create_table()?;
                    answer.set("path", staged.path.to_string_lossy().into_owned())?;
                    answer.set("size", staged.size)?;
                    Ok(answer)
                })();
                match answer {
                    Ok(answer) => staged
                        .publish()
                        .map(|()| answer)
                        .map_err(mlua::Error::runtime),
                    Err(e) => {
                        staged.discard();
                        Err(e)
                    }
                }
            })?,
        )?;
        let store_table = lua.create_table()?;
        store_table.set(
            "get",
            scope.create_function(|_, key: String| Ok(store.borrow().get(&key).cloned()))?,
        )?;
        store_table.set(
            "set",
            scope.create_function(|_, (key, value): (String, Option<String>)| {
                store_set(&store, &store_path, key, value)
            })?,
        )?;
        cutcraft.set("store", store_table)?;
        lua.globals().set("cutcraft", cutcraft)?;
        lua.load(OUTSIDE_PRELUDE).set_name("=cutcraft").exec()?;

        lua.load(source)
            .set_name(format!("={name}"))
            .set_mode(ChunkMode::Text)
            .exec()?;
        let run: Option<mlua::Function> = lua.globals().get("run")?;
        let Some(run) = run else {
            return Err(mlua::Error::runtime(
                "it has no `function run(settings)`, so there is nothing to do",
            ));
        };
        run.call::<()>(lua.to_value(settings)?)
    });

    match result {
        Ok(()) => Ok(notes.into_inner()),
        Err(_) if stopped.get() => Err(ScriptError::TooLong),
        Err(e) => Err(script_error(&e)),
    }
}

/// `cutcraft.http{ method, url, headers, body }` → `{ status, body }`.
fn http_call(
    lua: &mlua::Lua,
    outside: &Outside,
    agent: &ureq::Agent,
    request: &Table,
) -> mlua::Result<Table> {
    let method: Option<String> = request.get("method")?;
    let url: String = request.get("url")?;
    let body: Option<String> = request.get("body")?;
    let headers: Option<BTreeMap<String, String>> = request.get("headers")?;
    let method = method.unwrap_or_else(|| "GET".to_owned());
    let mut response = send(
        outside,
        agent,
        &method,
        &url,
        &headers.unwrap_or_default(),
        body.as_deref(),
    )
    .map_err(mlua::Error::runtime)?;
    let status = response.status().as_u16();
    // counted after any decompression, so a small packed answer cannot unpack past the limit
    let mut bytes = Vec::new();
    response
        .body_mut()
        .as_reader()
        .take(outside.max_body.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|e| mlua::Error::runtime(format!("the answer from {url} was broken: {e}")))?;
    if bytes.len() as u64 > outside.max_body {
        return Err(mlua::Error::runtime(format!(
            "the answer from {url} was too long"
        )));
    }
    let text = String::from_utf8(bytes)
        .map_err(|_| mlua::Error::runtime(format!("the answer from {url} was not text")))?;
    let answer = lua.create_table()?;
    answer.set("status", status)?;
    answer.set("body", text)?;
    Ok(answer)
}

/// `cutcraft.store.set(key, value)`: `nil` removes it. Short keys and values, at most 100.
fn store_set(
    store: &RefCell<BTreeMap<String, String>>,
    path: &Path,
    key: String,
    value: Option<String>,
) -> mlua::Result<()> {
    if key.len() > 100 || value.as_ref().is_some_and(|v| v.len() > 4096) {
        return Err(mlua::Error::runtime("a saved setting is too long"));
    }
    // change a copy, save it, and only then keep it: a failed save changes nothing
    let mut next = store.borrow().clone();
    match value {
        Some(v) => {
            if next.len() >= 100 && !next.contains_key(&key) {
                return Err(mlua::Error::runtime("too many saved settings"));
            }
            next.insert(key, v);
        }
        None => {
            next.remove(&key);
        }
    }
    save_store(path, &next).map_err(mlua::Error::runtime)?;
    *store.borrow_mut() = next;
    Ok(())
}

/// `print` goes to the person's messages, as in job gadgets.
const OUTSIDE_PRELUDE: &str = r##"
print = function(...)
  local parts = {}
  for i = 1, select("#", ...) do parts[i] = tostring((select(i, ...))) end
  cutcraft.message(table.concat(parts, "\t"))
end
"##;

/// Writes the settings beside `path` first and puts them in place only once written.
fn save_store(path: &Path, store: &BTreeMap<String, String>) -> Result<(), String> {
    let failed = |e: std::io::Error| format!("the add-on's settings could not be saved: {e}");
    let text = serde_json::to_string_pretty(store).map_err(|e| e.to_string())?;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(failed)?;
    }
    let part = path.with_extension("json.part");
    std::fs::write(&part, text).map_err(failed)?;
    std::fs::rename(&part, path).map_err(|e| {
        let _ = std::fs::remove_file(&part);
        failed(e)
    })
}

/// Numbers each download's temporary file.
static NEXT_PART: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// A whole download written beside its place in the `files` folder, not yet put there.
struct Staged {
    part: PathBuf,
    path: PathBuf,
    size: u64,
}

impl Staged {
    /// Puts the file in its place; if that fails, the part file is removed.
    fn publish(self) -> Result<(), String> {
        std::fs::rename(&self.part, &self.path).map_err(|e| {
            let _ = std::fs::remove_file(&self.part);
            format!("could not save {}: {e}", self.path.display())
        })
    }

    /// Drops the part file and leaves anything already in its place as it was.
    fn discard(self) {
        let _ = std::fs::remove_file(&self.part);
    }
}

/// Fetches `url` for `file_name` in the add-on's `files` folder, written beside it: the
/// caller puts it in place with [`Staged::publish`] once it is ready.
fn download(
    outside: &Outside,
    agent: &ureq::Agent,
    url: &str,
    file_name: &str,
) -> Result<Staged, String> {
    if !plain_file_name(file_name) {
        return Err(format!("\"{file_name}\" is not a plain file name"));
    }
    let mut response = send(outside, agent, "GET", url, &BTreeMap::new(), None)?;
    let status = response.status().as_u16();
    if status != 200 {
        return Err(format!("{url} answered {status} instead of the file"));
    }
    let folder = outside.files();
    std::fs::create_dir_all(&folder)
        .map_err(|e| format!("could not make the downloads folder: {e}"))?;
    let path = folder.join(file_name);
    // a name of its own for each download, so two never write into one file
    let n = NEXT_PART.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let part = folder.join(format!(".{file_name}.{}-{n}.part", std::process::id()));
    let written = (|| -> Result<u64, String> {
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&part)
            .map_err(|e| e.to_string())?;
        let mut reader = response
            .body_mut()
            .as_reader()
            .take(outside.max_download.saturating_add(1));
        let mut buf = vec![0_u8; 64 * 1024];
        let mut total = 0_u64;
        loop {
            let n = reader
                .read(&mut buf)
                .map_err(|e| format!("the download stopped: {e}"))?;
            if n == 0 {
                break;
            }
            total += n as u64;
            if total > outside.max_download {
                return Err("the file is larger than add-ons may download".to_owned());
            }
            file.write_all(&buf[..n]).map_err(|e| e.to_string())?;
        }
        file.sync_all().map_err(|e| e.to_string())?;
        Ok(total)
    })();
    match written {
        Ok(size) => Ok(Staged { part, path, size }),
        Err(e) => {
            let _ = std::fs::remove_file(&part);
            Err(e)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::io::{BufRead, BufReader};
    use std::net::TcpListener;

    /// A tiny web server on this computer that answers a few fixed paths.
    fn server() -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = format!("http://{}", listener.local_addr().unwrap());
        let listener_port = listener.local_addr().unwrap().port();
        std::thread::spawn(move || {
            'conn: for stream in listener.incoming().flatten() {
                let mut reader = BufReader::new(stream.try_clone().unwrap());
                let mut first = String::new();
                reader.read_line(&mut first).unwrap();
                let (mut length, mut line, mut auth) = (0_usize, String::new(), false);
                let mut host = String::new();
                loop {
                    match reader.read_line(&mut line) {
                        Ok(0) | Err(_) => continue 'conn,
                        Ok(_) if line == "\r\n" => break,
                        Ok(_) => {}
                    }
                    let lower = line.to_ascii_lowercase();
                    if let Some(v) = lower.strip_prefix("content-length:") {
                        length = v.trim().parse().unwrap_or(0);
                    }
                    auth |= lower.starts_with("authorization:");
                    if let Some(v) = lower.strip_prefix("host:") {
                        v.trim().clone_into(&mut host);
                    }
                    line.clear();
                }
                let mut body = vec![0; length];
                if reader.read_exact(&mut body).is_err() {
                    continue;
                }
                let port = listener_port;
                let path = first.split(' ').nth(1).unwrap_or("/").to_owned();
                let (status, extra, reply): (&str, String, Vec<u8>) = match path.as_str() {
                    "/list" => ("200 OK", String::new(), b"a\tb".to_vec()),
                    "/echo" => ("200 OK", String::new(), body),
                    "/file" => ("200 OK", String::new(), vec![7; 3000]),
                    "/big" => ("200 OK", String::new(), vec![1; 5000]),
                    "/hop" => ("302 Found", "Location: /file\r\n".into(), vec![]),
                    "/dir/hop" => ("302 Found", "Location: here\r\n".into(), vec![]),
                    "/dir/here" => ("200 OK", String::new(), b"relative".to_vec()),
                    "/whoami" => (
                        "200 OK",
                        String::new(),
                        if auth {
                            b"auth".to_vec()
                        } else {
                            b"none".to_vec()
                        },
                    ),
                    "/hop-same" => ("302 Found", "Location: /whoami\r\n".into(), vec![]),
                    "/hop-host" => (
                        "302 Found",
                        format!("Location: http://localhost:{port}/whoami\r\n"),
                        vec![],
                    ),
                    "/same" => ("304 Not Modified", String::new(), vec![]),
                    "/host" => ("200 OK", String::new(), host.into_bytes()),
                    // 5000 letters packed into 41 bytes
                    "/packed" => (
                        "200 OK",
                        "Content-Encoding: gzip\r\n".into(),
                        vec![
                            31, 139, 8, 0, 0, 0, 0, 0, 2, 255, 237, 193, 49, 1, 0, 0, 0, 194, 160,
                            172, 235, 95, 194, 20, 126, 64, 1, 0, 0, 0, 0, 111, 3, 118, 89, 244,
                            76, 136, 19, 0, 0,
                        ],
                    ),
                    "/away" => (
                        "302 Found",
                        "Location: http://elsewhere.example/x\r\n".into(),
                        vec![],
                    ),
                    _ => ("404 Not Found", String::new(), b"no".to_vec()),
                };
                let mut out = stream;
                let head = format!(
                    "HTTP/1.1 {status}\r\n{extra}Content-Length: {}\r\nConnection: close\r\n\r\n",
                    reply.len()
                );
                let _ = out.write_all(head.as_bytes());
                let _ = out.write_all(&reply);
            }
        });
        addr
    }

    fn folder(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("cutcraft-outside-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn outside(name: &str) -> Outside {
        let mut o = Outside::new(vec!["127.0.0.1".into(), "localhost".into()], folder(name));
        o.plain_http_for_tests = true;
        o.max_download = 4000;
        o
    }

    fn run(o: &Outside, source: &str, base: &str) -> Result<Vec<String>, ScriptError> {
        run_outside_gadget(
            "test.lua",
            source,
            &json!({ "base": base }),
            Limits::default(),
            o,
        )
    }

    #[test]
    fn a_gadget_fetches_text_from_a_listed_host() {
        let base = server();
        let notes = run(
            &outside("fetch"),
            r#"function run(s)
                 local r = cutcraft.http{ url = s.base .. "/list" }
                 local e = cutcraft.http{ method = "POST", url = s.base .. "/echo", body = "model_id=m1",
                                          headers = { ["Content-Type"] = "application/x-www-form-urlencoded" } }
                 local n = cutcraft.http{ url = s.base .. "/nothing" }
                 print(r.status, r.body, e.body, n.status)
               end"#,
            &base,
        )
        .unwrap();
        assert_eq!(notes, vec!["200\ta\tb\tmodel_id=m1\t404"]);
    }

    #[test]
    fn a_download_lands_whole_in_the_add_ons_files_folder() {
        let base = server();
        let o = outside("download");
        let notes = run(
            &o,
            r#"function run(s)
                 local d = cutcraft.download(s.base .. "/hop", "oak leaf.stl")
                 print(d.size)
               end"#,
            &base,
        )
        .unwrap();
        assert_eq!(notes, vec!["3000"]);
        assert_eq!(
            std::fs::read(o.folder.join("files/oak leaf.stl")).unwrap(),
            vec![7; 3000]
        );
    }

    #[test]
    fn unlisted_hosts_plain_http_and_redirects_away_are_refused() {
        let base = server();
        let mut strict = Outside::new(vec!["127.0.0.1".into()], folder("strict"));
        strict.max_download = 4000;
        let cases = [
            (
                &strict,
                format!("cutcraft.http{{ url = '{base}/list' }}"),
                "only https://",
            ),
            (
                &strict,
                "cutcraft.http{ url = 'https://elsewhere.example/' }".to_owned(),
                "may not reach elsewhere.example",
            ),
            (
                &strict,
                "cutcraft.http{ url = 'https://me@127.0.0.1/' }".to_owned(),
                "user name",
            ),
            (
                &strict,
                "cutcraft.http{ url = 'https://127.0.0.1:8443/' }".to_owned(),
                "unusual port",
            ),
        ];
        for (o, call, why) in cases {
            let err = run(o, &format!("function run() {call} end"), &base)
                .unwrap_err()
                .to_string();
            assert!(err.contains(why), "{call}: {err}");
        }
        let err = run(
            &outside("away"),
            "function run(s) cutcraft.http{ url = s.base .. '/away' } end",
            &base,
        )
        .unwrap_err()
        .to_string();
        assert!(err.contains("may not reach elsewhere.example"), "{err}");
    }

    #[test]
    fn odd_file_names_and_oversized_files_are_refused_and_leave_nothing() {
        let base = server();
        let o = outside("odd");
        for name in [
            "../escape.stl",
            "a/b.stl",
            "C:evil",
            ".hidden",
            "",
            "NUL.stl",
            "com1",
            "COM¹.stl",
            "lpt²",
            "trailing.",
            "trailing ",
        ] {
            let src = format!("function run(s) cutcraft.download(s.base .. '/file', {name:?}) end");
            let err = run(&o, &src, &base).unwrap_err().to_string();
            assert!(err.contains("plain file name"), "{name}: {err}");
        }
        let err = run(
            &o,
            "function run(s) cutcraft.download(s.base .. '/big', 'big.stl') end",
            &base,
        )
        .unwrap_err()
        .to_string();
        assert!(err.contains("larger than"), "{err}");
        let left: Vec<_> = std::fs::read_dir(o.folder.join("files"))
            .unwrap()
            .flatten()
            .collect();
        assert_eq!(left.len(), 0, "{left:?}");
    }

    #[test]
    fn redirects_resolve_like_a_browser_and_keep_keys_on_their_own_site() {
        let base = server();
        let notes = run(
            &outside("redirects"),
            r#"function run(s)
                 local key = { Authorization = "Bearer k" }
                 print(cutcraft.http{ url = s.base .. "/dir/hop" }.body,
                       cutcraft.http{ url = s.base .. "/hop-same", headers = key }.body,
                       cutcraft.http{ url = s.base .. "/hop-host", headers = key }.body,
                       cutcraft.http{ url = s.base .. "/same" }.status)
               end"#,
            &base,
        )
        .unwrap();
        assert_eq!(notes, vec!["relative\tauth\tnone\t304"]);
        // the default port is the same site; another port, host or scheme is not
        assert_eq!(
            origin("https://A.example:443/x"),
            origin("https://a.example/y")
        );
        assert_ne!(
            origin("https://a.example:8443/"),
            origin("https://a.example/")
        );
        assert_ne!(origin("https://b.example/"), origin("https://a.example/"));
        assert_ne!(origin("http://a.example/"), origin("https://a.example/"));
        for (from, next, want) in [
            ("https://a.example/x/y?q=1", "z", "https://a.example/x/z"),
            ("https://a.example/x/y", "/z", "https://a.example/z"),
            (
                "https://a.example/x/y?q=1",
                "?r=2",
                "https://a.example/x/y?r=2",
            ),
            (
                "https://a.example/x",
                "//b.example/z",
                "https://b.example/z",
            ),
            ("https://a.example", "z", "https://a.example/z"),
            (
                "https://a.example/x/y?q=1",
                "#part",
                "https://a.example/x/y?q=1#part",
            ),
            ("https://a.example/x/y/z", "../w", "https://a.example/x/w"),
            (
                "https://a.example/x/y",
                "/a/./b/../c",
                "https://a.example/a/c",
            ),
            (
                "https://a.example/x",
                "https://b.example/",
                "https://b.example/",
            ),
        ] {
            assert_eq!(resolve(from, next).as_deref(), Ok(want), "{from} + {next}");
        }
    }

    #[test]
    fn a_bad_sleep_or_a_full_store_is_an_error_not_a_crash() {
        let o = outside("limits");
        let err = run(&o, "function run() cutcraft.sleep(0/0) end", "")
            .unwrap_err()
            .to_string();
        assert!(err.contains("number of seconds"), "{err}");
        let notes = run(
            &o,
            r#"function run()
                 for i = 1, 100 do cutcraft.store.set("k" .. i, "v") end
                 local ok = pcall(cutcraft.store.set, "one-more", "v")
                 cutcraft.store.set("k1", "changed")
                 print(ok, cutcraft.store.get("one-more"), cutcraft.store.get("k1"))
               end"#,
            "",
        )
        .unwrap();
        assert_eq!(notes, vec!["false\tnil\tchanged"]);
    }

    #[test]
    fn saved_settings_last_between_runs() {
        let o = outside("store");
        run(
            &o,
            "function run() cutcraft.store.set('LastReleasedAt', '2000') end",
            "",
        )
        .unwrap();
        let notes = run(&o, "function run() print(cutcraft.store.get('LastReleasedAt'), cutcraft.store.get('none')) end", "").unwrap();
        assert_eq!(notes, vec!["2000\tnil"]);
    }

    #[test]
    fn a_download_that_is_not_published_leaves_the_old_file_alone() {
        let base = server();
        let o = outside("staged");
        std::fs::create_dir_all(o.files()).unwrap();
        std::fs::write(o.files().join("leaf.stl"), "old").unwrap();
        let staged = download(
            &o,
            &agent(Duration::from_secs(10)),
            &format!("{base}/file"),
            "leaf.stl",
        )
        .unwrap();
        // the answer for the gadget failed: the new file is dropped
        staged.discard();
        assert_eq!(
            std::fs::read_to_string(o.files().join("leaf.stl")).unwrap(),
            "old"
        );
        let left = std::fs::read_dir(o.files()).unwrap().count();
        assert_eq!(left, 1, "only the old file is left");
    }

    #[test]
    fn plain_names_with_spaces_are_fine() {
        assert!(plain_file_name("oak leaf.stl"));
        assert!(plain_file_name("console.stl"));
    }

    #[test]
    fn damaged_settings_stop_the_run_instead_of_being_replaced() {
        let o = outside("damaged");
        std::fs::write(o.folder.join("store.json"), "{ not json").unwrap();
        let err = run(&o, "function run() cutcraft.store.set('a', 'b') end", "")
            .unwrap_err()
            .to_string();
        assert!(err.contains("damaged"), "{err}");
        let kept = std::fs::read_to_string(o.folder.join("store.json")).unwrap();
        assert_eq!(kept, "{ not json");
    }

    #[test]
    fn the_host_header_follows_the_address_and_packed_answers_count_unpacked() {
        let base = server();
        let mut o = outside("host");
        o.max_body = 4000;
        let notes = run(
            &o,
            r#"function run(s)
                 print(cutcraft.http{ url = s.base .. "/host", headers = { Host = "evil.example" } }.body)
               end"#,
            &base,
        )
        .unwrap();
        assert_eq!(notes, vec![base.trim_start_matches("http://").to_owned()]);
        let err = run(
            &o,
            "function run(s) cutcraft.http{ url = s.base .. '/packed' } end",
            &base,
        )
        .unwrap_err()
        .to_string();
        assert!(err.contains("too long"), "{err}");
    }

    #[test]
    fn the_rest_of_the_sandbox_stays_closed() {
        let notes = run(
            &outside("closed"),
            "function run() print(io, os, require, load, dofile, package) end",
            "",
        )
        .unwrap();
        assert_eq!(notes, vec!["nil\tnil\tnil\tnil\tnil\tnil"]);
    }
}
