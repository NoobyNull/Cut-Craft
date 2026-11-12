//! The machine's files at a web address (WebDAV), as WiFi controllers serve their card:
//! FluidNC at `http://fluidnc.local/sd/` (its flash at `/flash/`), with no password.
//! Listing is PROPFIND, sending PUT (folders made with MKCOL), renaming MOVE, deleting DELETE.
// ponytail: no user name or password yet (FluidNC asks for none); a controller that wants one
// answers 401, which says so

use crate::send::{Item, Store};
use std::time::Duration;
use ureq::http::{Method, Request};

/// How long to wait for the machine to answer before saying it can't be reached.
const CONNECTING: Duration = Duration::from_secs(5);
/// How long one request may take in all: a big file over a controller's WiFi is slow.
const LONGEST: Duration = Duration::from_secs(300);

/// Whether `place` is a web address rather than a folder.
pub fn is_address(place: &str) -> bool {
    let p = place.trim().to_ascii_lowercase();
    p.starts_with("http://") || p.starts_with("https://")
}

/// The machine's files at a WebDAV address.
pub struct WebDav {
    /// The address, ending in `/`.
    base: String,
    agent: ureq::Agent,
}

impl WebDav {
    pub fn new(address: &str) -> Self {
        let mut base = address.trim().to_owned();
        if !base.ends_with('/') {
            base.push('/');
        }
        let agent = ureq::Agent::config_builder()
            .timeout_connect(Some(CONNECTING))
            .timeout_global(Some(LONGEST))
            .http_status_as_error(false)
            .allow_non_standard_methods(true)
            .build()
            .new_agent();
        Self { base, agent }
    }

    /// The address of folder `at` (ending in `/`), or of `name` in it.
    fn url(&self, at: &[String], name: Option<&str>) -> Result<String, String> {
        let mut url = self.base.clone();
        for part in at {
            plain(part)?;
            url.push_str(&encode(part));
            url.push('/');
        }
        if let Some(name) = name {
            plain(name)?;
            url.push_str(&encode(name));
        }
        Ok(url)
    }

    /// Sends `method` to `url` with `headers` and `body`: its status and what it said.
    fn ask(
        &self,
        method: &str,
        url: &str,
        headers: &[(&str, &str)],
        body: &[u8],
    ) -> Result<(u16, String), String> {
        let method = Method::from_bytes(method.as_bytes()).expect("a method name");
        let mut request = Request::builder().method(method).uri(url);
        for (name, value) in headers {
            request = request.header(*name, *value);
        }
        let request = request
            .body(body.to_vec())
            .map_err(|e| format!("{url} is not an address the machine can have: {e}"))?;
        let mut response = self.agent.run(request).map_err(|e| self.unreachable(&e))?;
        let status = response.status().as_u16();
        let text = response.body_mut().read_to_string().unwrap_or_default();
        Ok((status, text))
    }

    fn unreachable(&self, error: &ureq::Error) -> String {
        format!(
            "Can't reach the machine at {}. Check it is switched on and on the same network as \
             this computer, and that the address is right ({error})",
            self.base
        )
    }
}

/// What `status` from the machine means for `what`, when it is not a success.
fn refused(what: &str, status: u16) -> String {
    match status {
        401 | 403 => format!(
            "{what}: the machine asks for a password, which Cut Craft can't give it yet. Use \
             its card through a shared folder instead"
        ),
        404 => format!("{what}: it is not on the machine any more. Refresh the list"),
        405 | 409 | 412 => {
            format!("{what}: there is already something called that. Pick another name")
        }
        413 | 507 => format!(
            "{what}: the machine's card is full, or couldn't take it. Delete old jobs in Machine \
             ▸ Machine Files, then try again"
        ),
        503 => format!("{what}: the machine is busy running a job. Try again once it has finished"),
        _ => format!("{what}: the machine answered with error {status}"),
    }
}

fn ok(status: u16) -> bool {
    (200..300).contains(&status)
}

/// A name of one thing in a folder, nothing that reaches elsewhere.
fn plain(name: &str) -> Result<(), String> {
    let bad = name.is_empty() || name == "." || name == ".." || name.contains(['/', '\\']);
    if bad {
        Err(format!(
            "\"{name}\" is not a name a file or folder can have"
        ))
    } else {
        Ok(())
    }
}

/// `name` as it goes in an address: letters, digits and `-._~` as they are, the rest `%XX`.
fn encode(name: &str) -> String {
    name.bytes()
        .map(|b| {
            if b.is_ascii_alphanumeric() || b"-._~".contains(&b) {
                char::from(b).to_string()
            } else {
                format!("%{b:02X}")
            }
        })
        .collect()
}

/// An address's `%XX` turned back into what they stand for.
fn decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let hex = bytes
            .get(i + 1..i + 3)
            .and_then(|h| std::str::from_utf8(h).ok())
            .and_then(|h| u8::from_str_radix(h, 16).ok());
        match (bytes[i], hex) {
            (b'%', Some(b)) => {
                out.push(b);
                i += 3;
            }
            (b, _) => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// The path part of `href` (an address or a path), decoded, without `/` at either end.
fn path_of(href: &str) -> String {
    let path = match href.split_once("://") {
        Some((_, rest)) => rest.find('/').map_or("", |at| &rest[at..]),
        None => href,
    };
    decode(path).trim_matches('/').to_owned()
}

/// The first element called `tag` in `node`.
fn named<'a, 'i>(node: roxmltree::Node<'a, 'i>, tag: &str) -> Option<roxmltree::Node<'a, 'i>> {
    node.descendants()
        .find(|n| n.is_element() && n.tag_name().name() == tag)
}

/// The things a PROPFIND answer lists, leaving out the folder `asked` (by its path) itself.
pub fn listing(xml: &str, asked: &str) -> Result<Vec<Item>, String> {
    let doc = roxmltree::Document::parse(xml)
        .map_err(|e| format!("The machine's list of files could not be read: {e}"))?;
    let asked = path_of(asked);
    let mut items = vec![];
    for response in doc
        .descendants()
        .filter(|n| n.is_element() && n.tag_name().name() == "response")
    {
        let Some(href) = named(response, "href").and_then(|h| h.text()) else {
            continue;
        };
        let path = path_of(href.trim());
        if path == asked {
            continue;
        }
        let name = path.rsplit('/').next().unwrap_or_default().to_owned();
        if name.is_empty() || name.starts_with('.') || name == "System Volume Information" {
            continue;
        }
        let folder = named(response, "collection").is_some();
        let size = named(response, "getcontentlength")
            .and_then(|s| s.text())
            .and_then(|s| s.trim().parse().ok())
            .unwrap_or(0);
        items.push(Item {
            name,
            folder,
            size: if folder { 0 } else { size },
        });
    }
    items.sort_by(|a, b| (b.folder, a.name.to_lowercase()).cmp(&(a.folder, b.name.to_lowercase())));
    Ok(items)
}

const PROPFIND: &str = "<?xml version=\"1.0\" encoding=\"utf-8\"?>\
<D:propfind xmlns:D=\"DAV:\"><D:prop><D:resourcetype/><D:getcontentlength/></D:prop>\
</D:propfind>";

impl Store for WebDav {
    fn place(&self) -> String {
        self.base.clone()
    }

    fn list(&self, at: &[String]) -> Result<Vec<Item>, String> {
        let url = self.url(at, None)?;
        let headers = [("Depth", "1"), ("Content-Type", "application/xml")];
        let (status, text) = self.ask("PROPFIND", &url, &headers, PROPFIND.as_bytes())?;
        if !ok(status) {
            return Err(refused("Not listed", status));
        }
        listing(&text, &url)
    }

    fn upload(&self, at: &[String], name: &str, bytes: &[u8]) -> Result<(), String> {
        // each folder on the way, made when it is not there (405: it is)
        for n in 1..=at.len() {
            let (status, _) = self.ask("MKCOL", &self.url(&at[..n], None)?, &[], &[])?;
            if !ok(status) && status != 405 {
                return Err(refused(&format!("{name} not sent"), status));
            }
        }
        let url = self.url(at, Some(name))?;
        let (status, _) = self.ask("PUT", &url, &[], bytes)?;
        if ok(status) {
            Ok(())
        } else {
            Err(refused(&format!("{name} not sent"), status))
        }
    }

    fn exists(&self, at: &[String], name: &str) -> Result<bool, String> {
        let (status, _) = self.ask("HEAD", &self.url(at, Some(name))?, &[], &[])?;
        Ok(ok(status))
    }

    fn rename(&self, at: &[String], from: &str, to: &str) -> Result<(), String> {
        let to_url = self.url(at, Some(to))?;
        let headers = [("Destination", to_url.as_str()), ("Overwrite", "F")];
        let (status, _) = self.ask("MOVE", &self.url(at, Some(from))?, &headers, &[])?;
        if ok(status) {
            Ok(())
        } else {
            Err(refused("Not renamed", status))
        }
    }

    fn delete(&self, at: &[String], name: &str) -> Result<(), String> {
        let (status, _) = self.ask("DELETE", &self.url(at, Some(name))?, &[], &[])?;
        if ok(status) {
            Ok(())
        } else {
            Err(refused("Not deleted", status))
        }
    }

    fn make_folder(&self, at: &[String], name: &str) -> Result<(), String> {
        let url = format!("{}/", self.url(at, Some(name))?);
        let (status, _) = self.ask("MKCOL", &url, &[], &[])?;
        if ok(status) {
            Ok(())
        } else {
            Err(refused("Folder not made", status))
        }
    }

    /// FluidNC refuses commands while it moves (503, "Try again when not moving"), so asking
    /// for its status says whether it is running a job; other controllers don't answer.
    fn busy(&self) -> Option<String> {
        let (scheme, rest) = self.base.split_once("://")?;
        let host = rest.split('/').next()?;
        let url = format!("{scheme}://{host}/command?plain=%3F");
        match self.ask("GET", &url, &[], &[]) {
            Ok((503, _)) => Some(
                "The machine is running a job. Send again once it has finished, so the file it \
                 is cutting is not changed under it"
                    .to_owned(),
            ),
            _ => None,
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use std::collections::{BTreeMap, BTreeSet};
    use std::fmt::Write as _;
    use std::io::{BufRead, BufReader, Read, Write};
    use std::net::TcpListener;
    use std::sync::{Arc, Mutex};

    /// A small WebDAV server as FluidNC's answers, its files and folders by path; `moving`
    /// makes `/command` answer 503.
    #[derive(Default)]
    pub(crate) struct Card {
        pub files: BTreeMap<String, Vec<u8>>,
        pub folders: BTreeSet<String>,
        pub moving: bool,
    }

    /// Serves `card` on a free port in the background; its address.
    pub(crate) fn serve(card: Arc<Mutex<Card>>) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        std::thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                let card = Arc::clone(&card);
                std::thread::spawn(move || {
                    let mut reader = BufReader::new(stream.try_clone().unwrap());
                    let mut stream = stream;
                    while let Some((method, path, headers, body)) = read(&mut reader) {
                        let (status, text) =
                            answer(&mut card.lock().unwrap(), &method, &path, &headers, &body);
                        let reply = format!(
                            "HTTP/1.1 {status} X\r\nContent-Length: {}\r\n\r\n{text}",
                            text.len()
                        );
                        if stream.write_all(reply.as_bytes()).is_err() {
                            break;
                        }
                    }
                });
            }
        });
        format!("http://127.0.0.1:{port}/sd")
    }

    type Asked = (String, String, BTreeMap<String, String>, Vec<u8>);

    fn read(reader: &mut BufReader<std::net::TcpStream>) -> Option<Asked> {
        let mut line = String::new();
        if reader.read_line(&mut line).ok()? == 0 {
            return None;
        }
        let mut parts = line.split_whitespace();
        let (method, path) = (parts.next()?.to_owned(), parts.next()?.to_owned());
        let mut headers = BTreeMap::new();
        loop {
            let mut h = String::new();
            reader.read_line(&mut h).ok()?;
            let h = h.trim_end();
            if h.is_empty() {
                break;
            }
            let (k, v) = h.split_once(':')?;
            headers.insert(k.trim().to_ascii_lowercase(), v.trim().to_owned());
        }
        let length = headers
            .get("content-length")
            .and_then(|l| l.parse().ok())
            .unwrap_or(0);
        let mut body = vec![0; length];
        reader.read_exact(&mut body).ok()?;
        Some((method, path, headers, body))
    }

    fn answer(
        card: &mut Card,
        method: &str,
        path: &str,
        headers: &BTreeMap<String, String>,
        body: &[u8],
    ) -> (u16, String) {
        if path.starts_with("/command") {
            return if card.moving {
                (503, "Try again when not moving".into())
            } else {
                (200, "<Idle>".into())
            };
        }
        let key = decode(path).trim_matches('/').to_owned();
        let parent_there = |card: &Card, key: &str| match key.rsplit_once('/') {
            Some((parent, _)) => card.folders.contains(parent),
            None => true,
        };
        match method {
            "PROPFIND" => {
                if !card.folders.contains(&key) {
                    return (404, String::new());
                }
                let mut xml = format!(
                    "<?xml version=\"1.0\"?><D:multistatus xmlns:D=\"DAV:\"><D:response><D:href>{path}</D:href>\
                     <D:propstat><D:prop><D:resourcetype><D:collection/></D:resourcetype></D:prop></D:propstat></D:response>"
                );
                let inside = |p: &str| p.rsplit_once('/').map(|(a, _)| a) == Some(key.as_str());
                for f in card.folders.iter().filter(|f| inside(f)) {
                    let _ = write!(
                        xml,
                        "<D:response><D:href>/{}/</D:href><D:propstat><D:prop><D:resourcetype><D:collection/></D:resourcetype></D:prop></D:propstat></D:response>",
                        f.split('/').map(encode).collect::<Vec<_>>().join("/")
                    );
                }
                for (f, bytes) in card.files.iter().filter(|(f, _)| inside(f)) {
                    let _ = write!(
                        xml,
                        "<D:response><D:href>/{}</D:href><D:propstat><D:prop><D:resourcetype/><D:getcontentlength>{}</D:getcontentlength></D:prop></D:propstat></D:response>",
                        f.split('/').map(encode).collect::<Vec<_>>().join("/"),
                        bytes.len()
                    );
                }
                xml.push_str("</D:multistatus>");
                (207, xml)
            }
            "MKCOL" => {
                if card.folders.contains(&key) || card.files.contains_key(&key) {
                    (405, String::new())
                } else if parent_there(card, &key) {
                    card.folders.insert(key);
                    (201, String::new())
                } else {
                    (409, String::new())
                }
            }
            "PUT" => {
                if !parent_there(card, &key) {
                    return (409, String::new());
                }
                card.files.insert(key, body.to_vec());
                (201, String::new())
            }
            "HEAD" => (
                if card.files.contains_key(&key) || card.folders.contains(&key) {
                    200
                } else {
                    404
                },
                String::new(),
            ),
            "DELETE" => {
                let under = format!("{key}/");
                let had = card.files.remove(&key).is_some() | card.folders.remove(&key);
                card.files.retain(|f, _| !f.starts_with(&under));
                card.folders.retain(|f| !f.starts_with(&under));
                (if had { 204 } else { 404 }, String::new())
            }
            "MOVE" => {
                let to = path_of(&headers["destination"]);
                if card.files.contains_key(&to) || card.folders.contains(&to) {
                    return (412, String::new());
                }
                if let Some(bytes) = card.files.remove(&key) {
                    card.files.insert(to, bytes);
                    (201, String::new())
                } else if card.folders.remove(&key) {
                    card.folders.insert(to);
                    (201, String::new())
                } else {
                    (404, String::new())
                }
            }
            _ => (404, String::new()),
        }
    }

    pub(crate) fn card() -> (Arc<Mutex<Card>>, String) {
        let card = Arc::new(Mutex::new(Card::default()));
        card.lock().unwrap().folders.insert("sd".into());
        let address = serve(Arc::clone(&card));
        (card, address)
    }

    #[test]
    fn addresses_are_told_from_folders() {
        assert!(is_address("http://fluidnc.local/sd/"));
        assert!(is_address(" HTTPS://cnc/sd"));
        assert!(!is_address("/media/sd"));
        assert!(!is_address(r"\\cnc\sd"));
    }

    #[test]
    fn names_go_in_addresses_and_come_back() {
        assert_eq!(encode("Oak sign 2.nc"), "Oak%20sign%202.nc");
        assert_eq!(decode("Oak%20sign%202.nc"), "Oak sign 2.nc");
        assert_eq!(path_of("http://cnc.local/sd/Oak%20sign/"), "sd/Oak sign");
        assert_eq!(path_of("/sd/a.nc"), "sd/a.nc");
    }

    #[test]
    fn files_are_sent_listed_renamed_and_deleted_over_webdav() {
        let (card, address) = card();
        let store = WebDav::new(&address);
        let sign = vec!["Oak sign".to_owned()];
        store.upload(&sign, "b.nc", b"G0 X0").unwrap();
        store.upload(&sign, "A.nc", b"G0").unwrap();
        store.make_folder(&sign, "old").unwrap();
        assert!(store.exists(&sign, "b.nc").unwrap());
        assert!(!store.exists(&sign, "c.nc").unwrap());
        let names = |s: &WebDav| -> Vec<String> {
            s.list(&sign).unwrap().into_iter().map(|i| i.name).collect()
        };
        assert_eq!(names(&store), ["old", "A.nc", "b.nc"]);
        assert_eq!(store.list(&sign).unwrap()[2].size, 5);
        let err = store.rename(&sign, "A.nc", "b.nc").unwrap_err();
        assert!(err.contains("already something called"), "{err}");
        store.rename(&sign, "A.nc", "c.nc").unwrap();
        store.delete(&sign, "old").unwrap();
        store.delete(&sign, "b.nc").unwrap();
        assert_eq!(names(&store), ["c.nc"]);
        assert_eq!(store.list(&[]).unwrap()[0].name, "Oak sign");
        assert_eq!(card.lock().unwrap().files["sd/Oak sign/c.nc"], b"G0");
    }

    #[test]
    fn a_machine_running_a_job_is_left_alone() {
        let (card, address) = card();
        let store = WebDav::new(&address);
        assert_eq!(store.busy(), None);
        card.lock().unwrap().moving = true;
        assert!(store.busy().unwrap().contains("running a job"));
    }

    #[test]
    fn a_machine_that_cant_be_reached_says_what_to_check() {
        // nothing listens on port 9 here
        let store = WebDav::new("http://127.0.0.1:9/sd/");
        let err = store.list(&[]).unwrap_err();
        assert!(
            err.starts_with("Can't reach the machine at http://127.0.0.1:9/sd/"),
            "{err}"
        );
        assert_eq!(store.busy(), None);
    }
}
