//! Cut Craft's own icon on its windows: the title bar, the taskbar and Alt+Tab. Without it
//! the windows show the window library's stand-in. The same `.ico` is built into
//! `cut-craft.exe` on Windows (`build.rs`), so Explorer and shortcuts show it too.

/// The app icon at each size Windows asks for, each a PNG, made from `assets/cut-craft.svg`.
pub(super) const ICO: &[u8] = include_bytes!("../../../../assets/windows/cut-craft.ico");

/// The pictures in an `.ico`: each one's width in pixels and its bytes, as stored.
fn pictures(ico: &[u8]) -> Vec<(u32, &[u8])> {
    let word = |at: usize| u16::from_le_bytes([ico[at], ico[at + 1]]);
    let long = |at: usize| u32::from_le_bytes([ico[at], ico[at + 1], ico[at + 2], ico[at + 3]]);
    (0..usize::from(word(4)))
        .map(|i| {
            let entry = 6 + 16 * i;
            // 0 stands for 256
            let width = match ico[entry] {
                0 => 256,
                w => u32::from(w),
            };
            let (len, at) = (long(entry + 8) as usize, long(entry + 12) as usize);
            (width, &ico[at..at + len])
        })
        .collect()
}

/// The largest picture in the icon, for the windows; the system shrinks it where it needs
/// less.
pub(super) fn window_icon() -> std::sync::Arc<egui::IconData> {
    let (_, png) = pictures(ICO)
        .into_iter()
        .max_by_key(|(width, _)| *width)
        .expect("the icon has pictures");
    std::sync::Arc::new(eframe::icon_data::from_png_bytes(png).expect("the icon is a PNG"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_icon_has_every_size_windows_asks_for() {
        let widths: Vec<u32> = pictures(ICO).iter().map(|(w, _)| *w).collect();
        for size in [16, 24, 32, 48, 64, 256] {
            assert!(widths.contains(&size), "no {size} px picture in {widths:?}");
        }
    }

    #[test]
    fn each_picture_is_the_size_it_says() {
        for (width, png) in pictures(ICO) {
            let picture = eframe::icon_data::from_png_bytes(png).expect("a PNG");
            assert_eq!((picture.width, picture.height), (width, width));
        }
    }

    #[test]
    fn the_windows_get_the_largest_picture() {
        let icon = window_icon();
        assert_eq!((icon.width, icon.height), (256, 256));
        // not see-through all over: a real picture
        assert!(icon.rgba.chunks(4).any(|p| p[3] == 255));
    }

    #[test]
    fn the_main_window_shows_cut_crafts_icon() {
        let builder = super::super::main_viewport(None, "Cut Craft", true);
        assert_eq!(builder.icon, Some(window_icon()));
    }
}
