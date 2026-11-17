//! Keeps Cut Craft's dialog windows (Did You Know?, Options, the Tool Database...) above the
//! main window until they close, and makes the system's file dialogs belong to whichever window
//! opened them.
//!
//! egui opens each dialog as a window of its own beside the main one, as an ordinary top-level
//! window: a click on the main window raised it over the dialog, which then hid behind it. Here
//! each dialog window is made the main window's own once it opens, which the desktop keeps
//! above its owner and nothing else:
//!
//! - Windows: the main window becomes the dialog's owner (an owned window always stays above
//!   its owner, hides when it is minimised, and never covers other programs).
//! - Linux and the BSDs on X11, which includes `XWayland`, how Cut Craft runs on a Wayland
//!   desktop unless Options says otherwise: the dialog is marked `WM_TRANSIENT_FOR` the main
//!   window, which the window manager keeps above it in the same way.
//! - Wayland run directly, and macOS: no way to mark a window another's own after it is open,
//!   so nothing changes there.
//!
//! The app names its main window each frame ([`set_main_window`]); each dialog window reports
//! itself each frame it is shown ([`dialog_shown`]); [`focused_dialog`] says which dialog window a
//! file dialog opened now belongs to (none: the main window).

use std::collections::HashMap;
use std::sync::Mutex;

use raw_window_handle::{HasWindowHandle, RawWindowHandle};

#[cfg(windows)]
mod windows;
#[cfg(all(unix, not(target_os = "macos")))]
pub mod x11;

/// A window as the desktop knows it, by a number that stays valid while it is open (unlike a
/// handle, it can be kept between frames and shared between threads).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Native {
    /// An X11 window id (also `XWayland`'s).
    X11(u32),
    /// A Windows `HWND`.
    Win32(isize),
}

impl Native {
    /// The window behind `handle`, when this crate can own windows of that kind.
    pub fn from_raw(handle: RawWindowHandle) -> Option<Self> {
        // an X11 window id is a C `unsigned long`: 64 bits on Linux, 32 on Windows
        #[allow(clippy::useless_conversion)]
        match handle {
            RawWindowHandle::Xlib(h) => u32::try_from(h.window).ok().map(Native::X11),
            RawWindowHandle::Xcb(h) => Some(Native::X11(h.window.get())),
            RawWindowHandle::Win32(h) => Some(Native::Win32(h.hwnd.get())),
            _ => None,
        }
    }

    /// The window behind a winit (or other) window, as [`from_raw`](Self::from_raw).
    pub fn of(window: &impl HasWindowHandle) -> Option<Self> {
        Self::from_raw(window.window_handle().ok()?.as_raw())
    }
}

/// How many frames a dialog's window is looked for before giving up. On X11 a dialog that
/// opens with the main window (the tip shown at start) waits for the window manager to take
/// the main window on; frames come at least every [`RETRY`] while looking.
pub const TRIES: u32 = 100;

/// How soon to look again for a dialog's window not found yet.
pub const RETRY: std::time::Duration = std::time::Duration::from_millis(100);

/// One dialog window, as last seen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Dialog {
    /// The frame it was last shown on.
    frame: u64,
    /// Its window once found and made the main window's own.
    window: Option<Native>,
    /// Frames spent looking for it.
    tries: u32,
    /// Whether it had the keyboard when last shown.
    focused: bool,
}

/// The windows this crate knows of.
#[derive(Debug, Default)]
pub struct Windows {
    main: Option<Native>,
    /// The frame the main window was last named on.
    frame: u64,
    dialogs: HashMap<u64, Dialog>,
}

/// What a dialog's report asks to be done.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// Look for the dialog's window (named by its title) and make it `main`'s own.
    Own { main: Native },
    /// Nothing: already done, given up, or there is no main window to own it.
    Nothing,
}

impl Windows {
    /// The main window, named on each `frame` (`None` where windows cannot be owned).
    pub fn set_main(&mut self, main: Option<Native>, frame: u64) {
        self.frame = frame;
        if self.main != main {
            // a new main window (or none): the dialogs belong to the old one, so look again
            for dialog in self.dialogs.values_mut() {
                dialog.window = None;
                dialog.tries = 0;
            }
        }
        self.main = main;
    }

    /// Dialog `key` is shown on `frame`, with or without the keyboard: what to do about its
    /// window. A dialog missing from the frame before is a new window (closed and opened
    /// again), so it is looked for afresh.
    pub fn shown(&mut self, key: u64, frame: u64, focused: bool) -> Step {
        let fresh = Dialog {
            frame,
            window: None,
            tries: 0,
            focused,
        };
        let dialog = self.dialogs.entry(key).or_insert(fresh);
        if dialog.frame + 1 < frame {
            *dialog = fresh;
        }
        dialog.frame = frame;
        dialog.focused = focused;
        match self.main {
            Some(main) if dialog.window.is_none() && dialog.tries < TRIES => {
                dialog.tries += 1;
                Step::Own { main }
            }
            _ => Step::Nothing,
        }
    }

    /// Dialog `key`'s window was found and is now the main window's own.
    pub fn owned(&mut self, key: u64, window: Native) {
        if let Some(dialog) = self.dialogs.get_mut(&key) {
            dialog.window = Some(window);
        }
    }

    /// The dialog window that has the keyboard, which a file dialog opened now belongs to (its
    /// button opened it); `None` means the main window. A dialog not shown on this frame or
    /// the one before has closed.
    pub fn focused_dialog(&self) -> Option<Native> {
        let frame = self.frame;
        self.dialogs
            .values()
            .filter(|d| d.focused && d.frame + 1 >= frame)
            .find_map(|d| d.window)
    }
}

static WINDOWS: Mutex<Option<Windows>> = Mutex::new(None);

fn with<R>(f: impl FnOnce(&mut Windows) -> R) -> Option<R> {
    let mut windows = WINDOWS.lock().ok()?;
    Some(f(windows.get_or_insert_with(Windows::default)))
}

/// Names the main window at the start of each `frame` (egui's root pass number), a window from
/// winit or anything with a handle.
pub fn set_main_window(window: Option<&impl HasWindowHandle>, frame: u64) {
    let main = window.and_then(Native::of);
    with(|w| w.set_main(main, frame));
}

/// Dialog `key`, titled `title`, was shown on `frame`, with the keyboard or not: the first time,
/// its window is made the main window's own so it stays above it. `true` while its window is
/// still to be found: look again within [`RETRY`].
pub fn dialog_shown(key: u64, title: &str, frame: u64, focused: bool) -> bool {
    let Some(Step::Own { main }) = with(|w| w.shown(key, frame, focused)) else {
        return false;
    };
    match own(title, main) {
        Some(window) => {
            log::debug!("dialog window {title:?} now kept above the main window");
            with(|w| w.owned(key, window));
            false
        }
        None => true,
    }
}

/// The dialog window a file dialog opened now belongs to, or `None` for the main window
/// ([`Windows::focused_dialog`]).
pub fn focused_dialog() -> Option<Parent> {
    with(|w| w.focused_dialog()).flatten().and_then(Parent::new)
}

/// Finds this program's window titled `title` (not `main`) and makes it `main`'s own.
#[allow(unused_variables)]
fn own(title: &str, main: Native) -> Option<Native> {
    match main {
        #[cfg(all(unix, not(target_os = "macos")))]
        Native::X11(main) => x11::own(title, main).map(Native::X11),
        #[cfg(windows)]
        Native::Win32(main) => windows::own(title, main).map(Native::Win32),
        #[allow(unreachable_patterns)]
        _ => None,
    }
}

/// A window a file dialog or message box can be given as its parent (rfd takes one with a
/// handle), kept as its number so it can be held across frames and threads.
#[derive(Debug, Clone, Copy)]
pub struct Parent {
    window: raw_window_handle::RawWindowHandle,
    display: raw_window_handle::RawDisplayHandle,
}

// The handles hold only a window number here (no pointers): see `Parent::new`.
#[allow(unsafe_code)]
// SAFETY: built only from `Native`, a plain number; no pointer is kept.
unsafe impl Send for Parent {}
#[allow(unsafe_code)]
// SAFETY: as Send.
unsafe impl Sync for Parent {}

impl Parent {
    /// `None` where the platform has no such window.
    pub fn new(window: Native) -> Option<Self> {
        use raw_window_handle as rwh;
        // as in `Native::from_raw`
        #[allow(clippy::useless_conversion)]
        match window {
            Native::X11(id) => Some(Self {
                window: rwh::XlibWindowHandle::new(id.into()).into(),
                display: rwh::XlibDisplayHandle::new(None, 0).into(),
            }),
            Native::Win32(hwnd) => Some(Self {
                window: rwh::Win32WindowHandle::new(std::num::NonZeroIsize::new(hwnd)?).into(),
                display: rwh::WindowsDisplayHandle::new().into(),
            }),
        }
    }
}

impl raw_window_handle::HasWindowHandle for Parent {
    fn window_handle(
        &self,
    ) -> Result<raw_window_handle::WindowHandle<'_>, raw_window_handle::HandleError> {
        #[allow(unsafe_code)]
        // SAFETY: a window number the desktop gave this program for a window that was open when
        // it was read this frame; the dialog that takes it uses it at once, while it is open.
        Ok(unsafe { raw_window_handle::WindowHandle::borrow_raw(self.window) })
    }
}

impl raw_window_handle::HasDisplayHandle for Parent {
    fn display_handle(
        &self,
    ) -> Result<raw_window_handle::DisplayHandle<'_>, raw_window_handle::HandleError> {
        #[allow(unsafe_code)]
        // SAFETY: an X11 display with no pointer (the default display), or Windows' (none).
        Ok(unsafe { raw_window_handle::DisplayHandle::borrow_raw(self.display) })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MAIN: Native = Native::X11(7);

    fn with_main() -> Windows {
        let mut w = Windows::default();
        w.set_main(Some(MAIN), 10);
        w
    }

    #[test]
    fn a_new_dialog_is_made_the_main_windows_own() {
        let mut w = with_main();
        assert_eq!(w.shown(1, 10, true), Step::Own { main: MAIN });
    }

    #[test]
    fn once_owned_it_is_left_alone() {
        let mut w = with_main();
        w.shown(1, 10, true);
        w.owned(1, Native::X11(9));
        assert_eq!(w.shown(1, 11, true), Step::Nothing);
        assert_eq!(w.shown(1, 12, false), Step::Nothing);
    }

    #[test]
    fn without_a_main_window_nothing_is_done() {
        let mut w = Windows::default();
        assert_eq!(w.shown(1, 10, true), Step::Nothing);
        w.set_main(None, 11);
        assert_eq!(w.shown(1, 11, true), Step::Nothing);
    }

    #[test]
    fn a_window_not_found_is_looked_for_a_while_then_left() {
        let mut w = with_main();
        for frame in 0..u64::from(TRIES) {
            assert_eq!(w.shown(1, frame, true), Step::Own { main: MAIN });
        }
        assert_eq!(w.shown(1, u64::from(TRIES), true), Step::Nothing);
    }

    #[test]
    fn a_dialog_closed_and_opened_again_is_a_new_window() {
        let mut w = with_main();
        w.shown(1, 10, true);
        w.owned(1, Native::X11(9));
        // not shown on frames 11 and 12: closed; back on 13 as a new window
        assert_eq!(w.shown(1, 13, true), Step::Own { main: MAIN });
    }

    #[test]
    fn a_new_main_window_has_its_dialogs_owned_again() {
        let mut w = with_main();
        w.shown(1, 10, true);
        w.owned(1, Native::X11(9));
        w.set_main(Some(Native::X11(8)), 11);
        assert_eq!(
            w.shown(1, 11, true),
            Step::Own {
                main: Native::X11(8)
            }
        );
    }

    #[test]
    fn a_file_dialog_belongs_to_the_dialog_that_has_the_keyboard() {
        let mut w = with_main();
        w.shown(1, 10, false);
        w.owned(1, Native::X11(9));
        w.shown(2, 10, true);
        w.owned(2, Native::X11(11));
        assert_eq!(w.focused_dialog(), Some(Native::X11(11)));
        // opened from the app's own pass just after
        w.set_main(Some(MAIN), 11);
        assert_eq!(w.focused_dialog(), Some(Native::X11(11)));
    }

    #[test]
    fn a_file_dialog_belongs_to_the_main_window_otherwise() {
        let mut w = with_main();
        w.shown(1, 10, false);
        w.owned(1, Native::X11(9));
        assert_eq!(w.focused_dialog(), None);
        // the focused dialog has since closed
        w.shown(2, 10, true);
        w.owned(2, Native::X11(11));
        w.set_main(Some(MAIN), 12);
        assert_eq!(w.focused_dialog(), None);
        assert_eq!(Windows::default().focused_dialog(), None);
    }

    #[test]
    fn windows_are_known_from_their_handles() {
        use raw_window_handle as rwh;
        let xlib = rwh::XlibWindowHandle::new(42).into();
        assert_eq!(Native::from_raw(xlib), Some(Native::X11(42)));
        let xcb = rwh::XcbWindowHandle::new(std::num::NonZeroU32::new(43).unwrap()).into();
        assert_eq!(Native::from_raw(xcb), Some(Native::X11(43)));
        let win = rwh::Win32WindowHandle::new(std::num::NonZeroIsize::new(44).unwrap()).into();
        assert_eq!(Native::from_raw(win), Some(Native::Win32(44)));
        let web = rwh::WebWindowHandle::new(1).into();
        assert_eq!(Native::from_raw(web), None);
    }

    #[test]
    fn a_parent_hands_its_window_back() {
        use raw_window_handle::{HasWindowHandle, RawWindowHandle};
        let parent = Parent::new(Native::X11(42)).unwrap();
        let RawWindowHandle::Xlib(h) = parent.window_handle().unwrap().as_raw() else {
            panic!("an X11 window");
        };
        assert_eq!(h.window, 42);
        assert!(Parent::new(Native::Win32(0)).is_none());
    }
}
