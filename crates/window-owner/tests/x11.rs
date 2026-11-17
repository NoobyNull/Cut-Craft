//! On a real X server (Xvfb, when installed; skipped otherwise): a dialog window is found by its
//! program and title and marked as the main window's own.
#![cfg(all(unix, not(target_os = "macos")))]

use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};
use std::time::Duration;

use x11rb::connection::Connection;
use x11rb::protocol::xproto::{
    AtomEnum, ConnectionExt, CreateWindowAux, PropMode, Window, WindowClass,
};
use x11rb::rust_connection::RustConnection;
use x11rb::wrapper::ConnectionExt as _;

struct Xvfb(Child);

impl Drop for Xvfb {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// A private X server and a connection to it, or `None` where Xvfb is not installed. An
/// installed Xvfb that does not start fails the test. Xvfb picks a free display itself and
/// writes its number once ready (`-displayfd`), so the test never reaches another server.
fn server() -> Option<(Xvfb, RustConnection, usize)> {
    let child = match Command::new("Xvfb")
        .args(["-displayfd", "1", "-nolisten", "tcp"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
    {
        Ok(child) => child,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return None,
        Err(e) => panic!("Xvfb would not start: {e}"),
    };
    let mut xvfb = Xvfb(child);
    let stdout = xvfb.0.stdout.take().expect("Xvfb's output");
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let mut line = String::new();
        let _ = BufReader::new(stdout).read_line(&mut line);
        let _ = tx.send(line);
    });
    let line = rx
        .recv_timeout(Duration::from_secs(10))
        .expect("Xvfb did not start within 10 s");
    let number = line.trim();
    assert!(!number.is_empty(), "Xvfb stopped before it was ready");
    let (conn, screen) =
        RustConnection::connect(Some(&format!(":{number}"))).expect("connect to Xvfb");
    Some((xvfb, conn, screen))
}

fn window(conn: &RustConnection, screen: usize, pid: u32, title: &str) -> Window {
    let root = conn.setup().roots[screen].root;
    let id = conn.generate_id().unwrap();
    conn.create_window(
        0,
        id,
        root,
        0,
        0,
        100,
        100,
        0,
        WindowClass::INPUT_OUTPUT,
        0,
        &CreateWindowAux::new(),
    )
    .unwrap();
    let atom = |name: &str| {
        conn.intern_atom(false, name.as_bytes())
            .unwrap()
            .reply()
            .unwrap()
            .atom
    };
    conn.change_property32(
        PropMode::REPLACE,
        id,
        atom("_NET_WM_PID"),
        AtomEnum::CARDINAL,
        &[pid],
    )
    .unwrap();
    conn.change_property8(
        PropMode::REPLACE,
        id,
        atom("_NET_WM_NAME"),
        atom("UTF8_STRING"),
        title.as_bytes(),
    )
    .unwrap();
    conn.flush().unwrap();
    id
}

/// The window manager has taken `window` on (as a running one marks each window it manages).
fn managed(conn: &RustConnection, window: Window) {
    let state = conn
        .intern_atom(false, b"WM_STATE")
        .unwrap()
        .reply()
        .unwrap()
        .atom;
    conn.change_property32(PropMode::REPLACE, window, state, state, &[1, 0])
        .unwrap()
        .check()
        .unwrap();
}

fn transient_for(conn: &RustConnection, window: Window) -> Option<u32> {
    let reply = conn
        .get_property(
            false,
            window,
            AtomEnum::WM_TRANSIENT_FOR,
            AtomEnum::WINDOW,
            0,
            1,
        )
        .unwrap()
        .reply()
        .unwrap();
    reply.value32()?.next()
}

#[test]
fn a_dialog_window_is_marked_as_the_main_windows_own() {
    let Some((_xvfb, conn, screen)) = server() else {
        eprintln!("skipped: Xvfb is not installed");
        return;
    };
    let pid = std::process::id();
    let main = window(&conn, screen, pid, "Cut Craft");
    // another program's window of the same title is left alone
    let other = window(&conn, screen, pid + 1, "Did You Know?");
    let tips = window(&conn, screen, pid, "Did You Know?");
    // not before the window manager has the main window
    assert_eq!(
        cut_craft_window_owner::x11::own_on(&conn, screen, pid, "Did You Know?", main),
        None
    );
    assert_eq!(transient_for(&conn, tips), None);
    managed(&conn, main);
    let owned = cut_craft_window_owner::x11::own_on(&conn, screen, pid, "Did You Know?", main);
    assert_eq!(owned, Some(tips));
    assert_eq!(transient_for(&conn, tips), Some(main));
    assert_eq!(transient_for(&conn, other), None);
    assert_eq!(transient_for(&conn, main), None);
}

#[test]
fn a_window_that_is_not_there_is_not_found() {
    let Some((_xvfb, conn, screen)) = server() else {
        eprintln!("skipped: Xvfb is not installed");
        return;
    };
    let pid = std::process::id();
    let main = window(&conn, screen, pid, "Options");
    managed(&conn, main);
    // the main window is never its own owner, even with the title asked for
    assert_eq!(
        cut_craft_window_owner::x11::own_on(&conn, screen, pid, "Options", main),
        None
    );
    assert_eq!(
        cut_craft_window_owner::x11::own_on(&conn, screen, pid, "Snapping Options", main),
        None
    );
}
