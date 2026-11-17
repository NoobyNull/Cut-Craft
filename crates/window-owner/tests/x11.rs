//! On a real X server (Xvfb, when installed; skipped otherwise): a dialog window is found by its
//! program and title and marked as the main window's own.
#![cfg(all(unix, not(target_os = "macos")))]

use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

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

/// A private X server and a connection to it, or `None` where Xvfb is not installed.
fn server(test: u32) -> Option<(Xvfb, RustConnection, usize)> {
    // a display number unlikely to be in use, one per test and run
    let display = format!(":{}", 90 + (std::process::id() % 400) * 2 + test);
    let child = Command::new("Xvfb")
        .args([display.as_str(), "-nolisten", "tcp"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let xvfb = Xvfb(child);
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(10) {
        if let Ok((conn, screen)) = RustConnection::connect(Some(&display)) {
            return Some((xvfb, conn, screen));
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    None
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
    let Some((_xvfb, conn, screen)) = server(0) else {
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
    let Some((_xvfb, conn, screen)) = server(1) else {
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
