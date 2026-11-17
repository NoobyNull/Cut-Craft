//! Windows: the main window becomes a dialog window's owner. An owned window always stays above
//! its owner, is minimised with it, and does not cover other programs.

use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    FindWindowExW, GWLP_HWNDPARENT, GetWindowThreadProcessId, SetWindowLongPtrW,
};

/// Finds this program's top-level window titled `title` (not `main`) and makes `main` its
/// owner.
pub fn own(title: &str, main: isize) -> Option<isize> {
    let wide: Vec<u16> = title.encode_utf16().chain(std::iter::once(0)).collect();
    let main_hwnd = main as HWND;
    let mut after: HWND = std::ptr::null_mut();
    loop {
        #[allow(unsafe_code)]
        // SAFETY: `wide` is a NUL-terminated UTF-16 string that outlives the call; a null
        // parent and class search every top-level window by title.
        let found =
            unsafe { FindWindowExW(std::ptr::null_mut(), after, std::ptr::null(), wide.as_ptr()) };
        if found.is_null() {
            return None;
        }
        after = found;
        let mut pid = 0u32;
        #[allow(unsafe_code)]
        // SAFETY: `found` is a window handle just returned; `pid` is a valid place to write.
        unsafe {
            GetWindowThreadProcessId(found, &raw mut pid);
        }
        if found == main_hwnd || pid != std::process::id() {
            continue;
        }
        #[allow(unsafe_code)]
        // SAFETY: both are this program's own open top-level windows; GWLP_HWNDPARENT on a
        // top-level window sets its owner (documented in SetWindowLongPtr's remarks).
        unsafe {
            SetWindowLongPtrW(found, GWLP_HWNDPARENT, main);
        }
        return Some(found as isize);
    }
}
