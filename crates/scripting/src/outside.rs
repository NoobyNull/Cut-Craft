//! Gadgets that reach outside the job: an add-on that declares the web hosts it uses can
//! fetch from them, save downloads in its own folder and keep a few settings between runs.
//!
//! These gadgets never touch the document, so they are not commands: Cut Craft runs one only
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
    /// The add-on's own folder: downloads go in its `files` folder, settings in `store.json`.
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
    !name.is_empty()
        && name.len() <= 200
        && !name.starts_with('.')
        && !name.chars().any(|c| {
            c.is_control() || matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|')
        })
}

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_connect(Some(Duration::from_secs(15)))
        .timeout_recv_response(Some(Duration::from_secs(60)))
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
    for _ in 0..=5 {
        let uri = outside.check(&url)?;
        let mut request = ureq::http::Request::builder()
            .method(method.as_str())
            .uri(uri);
        for (k, v) in headers {
            request = request.header(k.as_str(), v.as_str());
        }
        let request = request
            .body(body.clone().unwrap_or_default())
            .map_err(|e| format!("the request could not be made: {e}"))?;
        let response = agent
            .run(request)
            .map_err(|e| format!("could not reach {url}: {e}"))?;
        if !response.status().is_redirection() {
            return Ok(response);
        }
        let next = response
            .headers()
            .get("location")
            .and_then(|l| l.to_str().ok())
            .ok_or_else(|| format!("{url} sent a redirect with nowhere to go"))?;
        url = resolve(&url, next);
        if response.status() == 303 {
            (method, body) = ("GET".to_owned(), None);
        }
    }
    Err("the address redirected too many times".to_owned())
}

/// `next` from a `Location` header, made whole against `from`.
fn resolve(from: &str, next: &str) -> String {
    if next.contains("://") {
        return next.to_owned();
    }
    let origin_end = from
        .find("://")
        .and_then(|s| from[s + 3..].find('/').map(|p| s + 3 + p))
        .unwrap_or(from.len());
    format!("{}/{}", &from[..origin_end], next.trim_start_matches('/'))
}

/// The add-on's saved settings.
fn load_store(path: &Path) -> BTreeMap<String, String> {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
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
    let agent = agent();
    let notes = RefCell::new(Vec::<String>::new());
    let store_path = outside.folder.join("store.json");
    let store = RefCell::new(load_store(&store_path));

    let result = lua.scope(|scope| {
        let cutcraft = lua.create_table()?;
        cutcraft.set(
            "message",
            scope.create_function(|_, text: String| {
                notes.borrow_mut().push(text);
                Ok(())
            })?,
        )?;
        cutcraft.set(
            "sleep",
            scope.create_function(|_, seconds: f64| {
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
                let saved =
                    download(outside, &agent, &url, &file_name).map_err(mlua::Error::runtime)?;
                let answer = lua.create_table()?;
                answer.set("path", saved.0.to_string_lossy().into_owned())?;
                answer.set("size", saved.1)?;
                Ok(answer)
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
    let text = response
        .body_mut()
        .with_config()
        .limit(outside.max_body)
        .read_to_string()
        .map_err(|e| {
            mlua::Error::runtime(format!("the answer from {url} was too long or broken: {e}"))
        })?;
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
    let mut s = store.borrow_mut();
    match value {
        Some(v) => s.insert(key, v),
        None => s.remove(&key),
    };
    if s.len() > 100 {
        return Err(mlua::Error::runtime("too many saved settings"));
    }
    save_store(path, &s).map_err(mlua::Error::runtime)
}

/// `print` goes to the person's messages, as in job gadgets.
const OUTSIDE_PRELUDE: &str = r##"
print = function(...)
  local parts = {}
  for i = 1, select("#", ...) do parts[i] = tostring((select(i, ...))) end
  cutcraft.message(table.concat(parts, "\t"))
end
"##;

fn save_store(path: &Path, store: &BTreeMap<String, String>) -> Result<(), String> {
    let text = serde_json::to_string_pretty(store).map_err(|e| e.to_string())?;
    std::fs::write(path, text).map_err(|e| format!("the add-on's settings could not be saved: {e}"))
}

/// Saves `url` as `file_name` in the add-on's `files` folder: written beside it first, and
/// put in place only once it is whole.
fn download(
    outside: &Outside,
    agent: &ureq::Agent,
    url: &str,
    file_name: &str,
) -> Result<(PathBuf, u64), String> {
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
    let part = folder.join(format!(".{file_name}.part"));
    let written = (|| -> Result<u64, String> {
        let mut file = std::fs::File::create(&part).map_err(|e| e.to_string())?;
        let mut reader = response
            .body_mut()
            .as_reader()
            .take(outside.max_download + 1);
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
        Ok(size) => {
            std::fs::rename(&part, &path)
                .map_err(|e| format!("could not save {file_name}: {e}"))?;
            Ok((path, size))
        }
        Err(why) => {
            let _ = std::fs::remove_file(&part);
            Err(why)
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
        std::thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                let mut reader = BufReader::new(stream.try_clone().unwrap());
                let mut first = String::new();
                reader.read_line(&mut first).unwrap();
                let (mut length, mut line) = (0_usize, String::new());
                while reader.read_line(&mut line).is_ok() && line != "\r\n" {
                    if let Some(v) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                        length = v.trim().parse().unwrap_or(0);
                    }
                    line.clear();
                }
                let mut body = vec![0; length];
                reader.read_exact(&mut body).unwrap();
                let path = first.split(' ').nth(1).unwrap_or("/").to_owned();
                let (status, extra, reply): (&str, String, Vec<u8>) = match path.as_str() {
                    "/list" => ("200 OK", String::new(), b"a\tb".to_vec()),
                    "/echo" => ("200 OK", String::new(), body),
                    "/file" => ("200 OK", String::new(), vec![7; 3000]),
                    "/big" => ("200 OK", String::new(), vec![1; 5000]),
                    "/hop" => ("302 Found", "Location: /file\r\n".into(), vec![]),
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
        let mut o = Outside::new(vec!["127.0.0.1".into()], folder(name));
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
        for name in ["../escape.stl", "a/b.stl", "C:evil", ".hidden", ""] {
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
