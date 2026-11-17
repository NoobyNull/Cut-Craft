//! X11 (and `XWayland`): a dialog window is marked `WM_TRANSIENT_FOR` the main window, which the
//! window manager keeps above it (and only it). Talks to the X server over its own connection,
//! in pure Rust; window ids are the same on every connection to one server.

use std::sync::OnceLock;

use x11rb::connection::Connection;
use x11rb::protocol::xproto::{AtomEnum, ConnectionExt, PropMode, Window};
use x11rb::rust_connection::RustConnection;
use x11rb::wrapper::ConnectionExt as _;

/// The connection to the X server named by `DISPLAY`, opened the first time it is needed.
fn connection() -> Option<&'static (RustConnection, usize)> {
    static CONNECTION: OnceLock<Option<(RustConnection, usize)>> = OnceLock::new();
    CONNECTION
        .get_or_init(|| match RustConnection::connect(None) {
            Ok(connection) => Some(connection),
            Err(e) => {
                log::warn!("dialogs cannot be kept above the main window: no X server ({e})");
                None
            }
        })
        .as_ref()
}

/// Finds this program's window titled `title` (not `main`) and marks it `main`'s own.
pub fn own(title: &str, main: u32) -> Option<u32> {
    let (conn, screen) = connection()?;
    own_on(conn, *screen, std::process::id(), title, main)
}

/// [`own`] on connection `conn` for the program `pid`. Not until the window manager has taken
/// on `main` (it gives it `WM_STATE`): a dialog marked as the own of a window it does not know
/// yet is taken as marked wrongly and left an ordinary window (seen with the tip shown at
/// start, which opens before the main window is shown).
pub fn own_on(
    conn: &impl Connection,
    screen: usize,
    pid: u32,
    title: &str,
    main: u32,
) -> Option<u32> {
    let atoms = Atoms::new(conn)?;
    if cardinals(conn, main, atoms.wm_state, atoms.wm_state).is_empty() {
        return None;
    }
    let found = find(conn, screen, pid, title, main, &atoms)?;
    set_transient_for(conn, found, main)?;
    Some(found)
}

/// The window of program `pid` titled `title`, other than `main`: among the windows the window
/// manager lists, then the screen's top-level windows (one not yet shown, or no window manager).
fn find(
    conn: &impl Connection,
    screen: usize,
    pid: u32,
    title: &str,
    main: u32,
    atoms: &Atoms,
) -> Option<Window> {
    let root = conn.setup().roots.get(screen)?.root;
    let listed = cardinals(conn, root, atoms.client_list, AtomEnum::WINDOW.into());
    let top = conn.query_tree(root).ok()?.reply().ok()?.children;
    listed
        .into_iter()
        .chain(top.into_iter().rev())
        .filter(|&w| w != main)
        .find(|&w| {
            cardinals(conn, w, atoms.pid, AtomEnum::CARDINAL.into()).first() == Some(&pid)
                && name(conn, w, atoms).as_deref() == Some(title)
        })
}

/// Marks `window` as a dialog of `owner`.
fn set_transient_for(conn: &impl Connection, window: Window, owner: Window) -> Option<()> {
    conn.change_property32(
        PropMode::REPLACE,
        window,
        AtomEnum::WM_TRANSIENT_FOR,
        AtomEnum::WINDOW,
        &[owner],
    )
    .ok()?;
    conn.flush().ok()
}

struct Atoms {
    wm_state: u32,
    client_list: u32,
    pid: u32,
    net_name: u32,
    utf8: u32,
}

impl Atoms {
    fn new(conn: &impl Connection) -> Option<Self> {
        let atom = |name: &str| -> Option<u32> {
            Some(
                conn.intern_atom(false, name.as_bytes())
                    .ok()?
                    .reply()
                    .ok()?
                    .atom,
            )
        };
        Some(Self {
            wm_state: atom("WM_STATE")?,
            client_list: atom("_NET_CLIENT_LIST")?,
            pid: atom("_NET_WM_PID")?,
            net_name: atom("_NET_WM_NAME")?,
            utf8: atom("UTF8_STRING")?,
        })
    }
}

/// A property of 32-bit numbers (empty when missing).
fn cardinals(conn: &impl Connection, window: Window, property: u32, kind: u32) -> Vec<u32> {
    conn.get_property(false, window, property, kind, 0, 4096)
        .ok()
        .and_then(|cookie| cookie.reply().ok())
        .and_then(|reply| reply.value32().map(Iterator::collect))
        .unwrap_or_default()
}

/// A window's title: `_NET_WM_NAME`, else `WM_NAME`.
fn name(conn: &impl Connection, window: Window, atoms: &Atoms) -> Option<String> {
    let text = |property: u32, kind: u32| -> Option<String> {
        let reply = conn
            .get_property(false, window, property, kind, 0, 1024)
            .ok()?
            .reply()
            .ok()?;
        (reply.format == 8 && !reply.value.is_empty())
            .then(|| String::from_utf8_lossy(&reply.value).into_owned())
    };
    text(atoms.net_name, atoms.utf8)
        .or_else(|| text(AtomEnum::WM_NAME.into(), AtomEnum::STRING.into()))
}
