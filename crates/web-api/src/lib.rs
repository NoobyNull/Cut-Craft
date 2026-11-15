//! Options ▸ Web API: a small HTTP server that lets a script, a test or Claude drive Cut Craft
//! the way a person does with the mouse.
//!
//! The server never touches the job. Its thread checks each request (the access token, and on
//! localhost the `Host` header), turns it into a [`Call`] and hands it to the app as a
//! [`Pending`]; the app runs it between frames, on its own thread, exactly as a click would
//! (one Undo step each), and answers. The server waits for that answer and sends it back.
//!
//! It is off until switched on in Options, listens on 127.0.0.1 unless "Allow other computers"
//! is ticked (then 0.0.0.0), and never streams G-code to a machine: Cut Craft is not a G-code
//! sender.

use serde_json::Value;
use std::fmt::Write as _;
use std::net::{Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread::JoinHandle;
use std::time::Duration;

mod call;
pub mod job;

pub use call::{Call, Pick};

/// The port the Web API listens on unless Options says another.
pub const DEFAULT_PORT: u16 = 7340;

/// Every path starts with this, so a later version can change the calls without breaking
/// scripts written for this one.
pub const PREFIX: &str = "/api/v1/";

/// The most a request's body may hold: a job's worth of settings, never a file.
const MAX_BODY: usize = 16 * 1024 * 1024;

/// How long the server waits for the app to answer before telling the caller it is busy.
const ANSWER_WITHIN: Duration = Duration::from_secs(300);

/// How the server is set up: where it listens and the token every call must carry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Settings {
    /// Listen on every network the computer is on (0.0.0.0), not only this computer.
    pub allow_network: bool,
    pub port: u16,
    /// The access token; `None` lets calls in without one.
    pub token: Option<String>,
}

impl Settings {
    /// The address the server listens on.
    pub fn address(&self) -> SocketAddr {
        let ip = if self.allow_network {
            Ipv4Addr::UNSPECIFIED
        } else {
            Ipv4Addr::LOCALHOST
        };
        SocketAddr::from((ip, self.port))
    }
}

/// What the status bar shows about the Web API.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum State {
    /// Switched off in Options: nothing shown.
    Off,
    /// Listening at this address: green "WEB API Running".
    Running(SocketAddr),
    /// Switched on but not listening, with why and how to fix it: red "WEB API Not Running".
    Failed(String),
}

/// A call that went wrong, with the HTTP status that says what kind of wrong.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ApiError {
    pub status: u16,
    pub message: String,
}

impl ApiError {
    /// The request itself is wrong (a missing field, a bad value): 400.
    pub fn bad(message: impl Into<String>) -> Self {
        Self {
            status: 400,
            message: message.into(),
        }
    }

    /// Something it names is not there (a tool, an action, a layer): 404.
    pub fn not_found(message: impl Into<String>) -> Self {
        Self {
            status: 404,
            message: message.into(),
        }
    }

    /// The request was fine but Cut Craft could not do it now (the tool said why): 409.
    pub fn refused(message: impl Into<String>) -> Self {
        Self {
            status: 409,
            message: message.into(),
        }
    }
}

/// What the app answers a call with: the JSON to send back, or why it could not.
pub type Answer = Result<Value, ApiError>;

/// A call waiting for the app to run it and answer.
#[derive(Debug)]
pub struct Pending {
    pub call: Call,
    reply: Sender<Answer>,
}

impl Pending {
    /// Sends the app's answer back to the caller. A caller that gave up waiting is ignored.
    pub fn answer(self, answer: Answer) {
        let _ = self.reply.send(answer);
    }
}

/// A running Web API: listening until dropped.
pub struct Server {
    http: Arc<tiny_http::Server>,
    calls: Receiver<Pending>,
    address: SocketAddr,
    thread: Option<JoinHandle<()>>,
}

impl Server {
    /// Starts listening as `settings` say. `wake` is called each time a call arrives, so the
    /// app can run it straight away rather than at its next frame.
    pub fn start(settings: &Settings, wake: impl Fn() + Send + 'static) -> Result<Self, String> {
        let address = settings.address();
        let http = listen(address)?;
        let address = http
            .server_addr()
            .to_ip()
            .ok_or_else(|| "The Web API could not find its own address.".to_owned())?;
        let http = Arc::new(http);
        let (send, calls) = mpsc::channel();
        let gate = Gate {
            token: settings.token.clone(),
            local_only: !settings.allow_network,
        };
        let listening = Arc::clone(&http);
        let thread = std::thread::Builder::new()
            .name("web-api".into())
            .spawn(move || serve(&listening, &gate, &send, &wake))
            .map_err(|e| format!("The Web API could not start: {e}."))?;
        log::info!("Web API listening on {address}");
        Ok(Self {
            http,
            calls,
            address,
            thread: Some(thread),
        })
    }

    /// Where it is listening (the real port, when started on port 0).
    pub fn address(&self) -> SocketAddr {
        self.address
    }

    /// Whether it is still listening; false if its thread stopped.
    pub fn listening(&self) -> bool {
        self.thread.as_ref().is_some_and(|t| !t.is_finished())
    }

    /// The calls that came in since last asked, oldest first.
    pub fn take(&self) -> Vec<Pending> {
        self.calls.try_iter().collect()
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.http.unblock();
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
        log::info!("Web API stopped");
    }
}

/// How long a start waits for its port to come free: a server just stopped lets its port
/// go a moment later, so a restart (another token) on the same port would otherwise fail.
const PORT_FREED_WITHIN: Duration = Duration::from_secs(1);

/// The HTTP server listening at `address`, once its port is free.
fn listen(address: SocketAddr) -> Result<tiny_http::Server, String> {
    let since = std::time::Instant::now();
    loop {
        match tiny_http::Server::http(address) {
            Ok(http) => return Ok(http),
            Err(e) => {
                if !port_in_use(&*e) || address.port() == 0 || since.elapsed() >= PORT_FREED_WITHIN
                {
                    return Err(could_not_listen(address, &*e));
                }
                std::thread::sleep(Duration::from_millis(50));
            }
        }
    }
}

/// Why the server could not listen at `address`, worded for the status bar.
fn could_not_listen(address: SocketAddr, why: &(dyn std::error::Error + 'static)) -> String {
    let text = why.to_string();
    if port_in_use(why) {
        format!(
            "Port {} is in use by another program. Pick another port in Options ▸ Web API.",
            address.port()
        )
    } else {
        format!(
            "The Web API could not listen on {address}: {text}. Pick another port in \
             Options ▸ Web API."
        )
    }
}

/// Whether `why` says another program has the port. Each system words it its own way
/// (Windows: "Only one usage of each socket address..."), so the kind of error is read.
fn port_in_use(why: &(dyn std::error::Error + 'static)) -> bool {
    why.downcast_ref::<std::io::Error>()
        .is_some_and(|e| e.kind() == std::io::ErrorKind::AddrInUse)
        || why.to_string().to_lowercase().contains("in use")
}

/// A new random access token: 48 hex characters.
pub fn new_token() -> String {
    let mut bytes = [0u8; 24];
    getrandom::fill(&mut bytes).expect("the operating system gives random bytes");
    bytes.iter().fold(String::new(), |mut hex, b| {
        let _ = write!(hex, "{b:02x}");
        hex
    })
}

/// What each request must pass before the app sees it.
struct Gate {
    token: Option<String>,
    local_only: bool,
}

impl Gate {
    fn check(&self, request: &tiny_http::Request) -> Result<(), ApiError> {
        if self.local_only {
            let host = header(request, "Host").unwrap_or_default();
            if !is_local_host(&host) {
                return Err(ApiError {
                    status: 403,
                    message: "The Web API only answers calls to localhost or 127.0.0.1.".into(),
                });
            }
        }
        if let Some(token) = &self.token {
            let given = header(request, "Authorization")
                .and_then(|h| h.strip_prefix("Bearer ").map(|t| t.trim().to_owned()));
            if !given.is_some_and(|g| same(g.as_bytes(), token.as_bytes())) {
                return Err(ApiError {
                    status: 401,
                    message: "Send the access token from Options ▸ Web API as \
                              \"Authorization: Bearer <token>\"."
                        .into(),
                });
            }
        }
        Ok(())
    }
}

/// Compares in the same time whatever the bytes, so the token cannot be guessed by timing.
fn same(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && a.iter().zip(b).fold(0u8, |d, (x, y)| d | (x ^ y)) == 0
}

/// Whether a `Host` header names this computer: blocks a web page that points its own name
/// at 127.0.0.1 to reach the API ("DNS rebinding").
fn is_local_host(host: &str) -> bool {
    let name = match host.strip_prefix('[') {
        Some(v6) => v6.split(']').next().unwrap_or_default(),
        None => host.rsplit_once(':').map_or(host, |(name, _)| name),
    };
    matches!(
        name.to_ascii_lowercase().as_str(),
        "localhost" | "127.0.0.1" | "::1"
    )
}

fn header(request: &tiny_http::Request, name: &'static str) -> Option<String> {
    request
        .headers()
        .iter()
        .find(|h| h.field.equiv(name))
        .map(|h| h.value.as_str().to_owned())
}

/// The server thread: each request checked, handed to the app and answered, one at a time,
/// so calls run in the order they were sent.
fn serve(
    http: &tiny_http::Server,
    gate: &Gate,
    calls: &Sender<Pending>,
    wake: &(impl Fn() + Send),
) {
    for mut request in http.incoming_requests() {
        let answer = gate
            .check(&request)
            .and_then(|()| read_call(&mut request))
            .and_then(|call| ask_app(call, calls, wake));
        let method = request.method().to_string();
        let url = request.url().to_owned();
        match &answer {
            Ok(_) => log::info!("Web API: {method} {url}"),
            Err(e) => log::info!(
                "Web API: {method} {url} refused ({}): {}",
                e.status,
                e.message
            ),
        }
        let _ = request.respond(response(answer));
    }
}

fn read_call(request: &mut tiny_http::Request) -> Result<Call, ApiError> {
    let method = request.method().clone();
    let url = request.url().to_owned();
    let path = url.split('?').next().unwrap_or_default();
    let Some(route) = path.strip_prefix(PREFIX) else {
        return Err(ApiError::not_found(format!(
            "Web API paths start with {PREFIX}, for example {PREFIX}status."
        )));
    };
    let body = match method {
        tiny_http::Method::Get => Value::Null,
        tiny_http::Method::Post => {
            // a page in a web browser cannot send JSON to another site without asking first,
            // and the API never says yes, so this also keeps web pages out
            let json = header(request, "Content-Type")
                .is_some_and(|t| t.trim().starts_with("application/json"));
            if !json {
                return Err(ApiError {
                    status: 415,
                    message: "Send the body as JSON, with \"Content-Type: application/json\"."
                        .into(),
                });
            }
            read_json(request)?
        }
        _ => {
            return Err(ApiError {
                status: 405,
                message: "The Web API takes GET and POST only.".into(),
            });
        }
    };
    let post = method == tiny_http::Method::Post;
    call::parse(route.trim_end_matches('/'), post, body)
}

fn read_json(request: &mut tiny_http::Request) -> Result<Value, ApiError> {
    use std::io::Read as _;
    let mut text = String::new();
    request
        .as_reader()
        .take(MAX_BODY as u64 + 1)
        .read_to_string(&mut text)
        .map_err(|e| ApiError::bad(format!("The body could not be read: {e}.")))?;
    if text.len() > MAX_BODY {
        return Err(ApiError {
            status: 413,
            message: "The body is too big: a call carries settings, not files.".into(),
        });
    }
    if text.trim().is_empty() {
        return Ok(Value::Object(serde_json::Map::new()));
    }
    serde_json::from_str(&text).map_err(|e| ApiError::bad(format!("The body is not JSON: {e}.")))
}

fn ask_app(call: Call, calls: &Sender<Pending>, wake: &impl Fn()) -> Answer {
    let (reply, answer) = mpsc::channel();
    calls.send(Pending { call, reply }).map_err(|_| closing())?;
    wake();
    match answer.recv_timeout(ANSWER_WITHIN) {
        Ok(answer) => answer,
        Err(RecvTimeoutError::Timeout) => Err(ApiError {
            status: 503,
            message: "Cut Craft did not answer in time: it may be busy, or waiting on a \
                      question in its window. Look at the window, then try again."
                .into(),
        }),
        Err(RecvTimeoutError::Disconnected) => Err(closing()),
    }
}

fn closing() -> ApiError {
    ApiError {
        status: 503,
        message: "Cut Craft is closing or the Web API was switched off.".into(),
    }
}

fn response(answer: Answer) -> tiny_http::Response<std::io::Cursor<Vec<u8>>> {
    let (status, body) = match answer {
        Ok(body) => (200, body),
        Err(e) => (e.status, serde_json::json!({ "error": e.message })),
    };
    let json = tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..])
        .expect("a fixed header is valid");
    tiny_http::Response::from_data(serde_json::to_vec_pretty(&body).unwrap_or_default())
        .with_status_code(status)
        .with_header(json)
}

#[cfg(test)]
mod tests;
