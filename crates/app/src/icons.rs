//! Tool icons: SVGs under the icon directory, dialog pictures kept as Aspire's own bitmaps
//! (both from `re/make_icons.py`), and a few PNGs for links Aspire has no icon for; not in git.

use eframe::egui;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

pub struct Icons {
    /// Relative path (`drawing/create_vectors/draw_circle.svg`) → absolute path.
    by_path: HashMap<String, PathBuf>,
    /// File stem (`draw_circle`) → absolute path.
    by_stem: HashMap<String, PathBuf>,
    /// Whether every icon's bytes are already in egui (see [`Icons::preload`]).
    preloaded: bool,
}

impl Icons {
    /// Indexes every `.svg`, `.bmp` and `.png` under `dir`; a missing directory gives an empty
    /// index.
    /// Only SVGs stand in for a command by their name.
    pub fn load(dir: &Path) -> Self {
        let mut icons = Self {
            by_path: HashMap::new(),
            by_stem: HashMap::new(),
            preloaded: false,
        };
        let mut pending = vec![dir.to_path_buf()];
        while let Some(folder) = pending.pop() {
            let Ok(entries) = fs::read_dir(&folder) else {
                continue;
            };
            let mut paths: Vec<PathBuf> = entries.flatten().map(|e| e.path()).collect();
            paths.sort();
            for path in paths {
                if path.is_dir() {
                    pending.push(path);
                } else if let Some(ext @ ("svg" | "bmp" | "png")) =
                    path.extension().and_then(|e| e.to_str())
                {
                    let (Ok(rel), Some(stem)) = (path.strip_prefix(dir), path.file_stem()) else {
                        continue;
                    };
                    if ext == "svg" {
                        icons
                            .by_stem
                            .entry(stem.to_string_lossy().into_owned())
                            .or_insert_with(|| path.clone());
                    }
                    // keyed with `/` as commands name them, whatever the platform's separator
                    let key = rel
                        .components()
                        .map(|c| c.as_os_str().to_string_lossy())
                        .collect::<Vec<_>>()
                        .join("/");
                    icons.by_path.insert(key, path);
                }
            }
        }
        icons
    }

    /// `CUT_CRAFT_ICON_DIR`, else `assets/icons` under the working directory.
    pub fn default_dir() -> PathBuf {
        std::env::var_os("CUT_CRAFT_ICON_DIR")
            .map_or_else(|| PathBuf::from("assets/icons"), PathBuf::from)
    }

    /// Reads every icon into egui once, so none shows a loading spinner the first time it is
    /// drawn (egui's own `file://` loader reads on a thread and draws a frame without it).
    // ponytail: every icon at startup (~200 files, ~10 MB); load per folder if that grows slow
    pub fn preload(&mut self, ctx: &egui::Context) {
        if std::mem::replace(&mut self.preloaded, true) {
            return;
        }
        for path in self.by_path.values() {
            match fs::read(path) {
                Ok(bytes) => ctx.include_bytes(uri(path), bytes),
                Err(e) => log::warn!("icon {}: {e}", path.display()),
            }
        }
    }

    /// An indexed icon, drawn from the bytes [`Icons::preload`] read.
    pub fn image(path: &Path) -> egui::Image<'static> {
        egui::Image::new(uri(path))
    }

    /// The icon a command names explicitly, else one whose file stem is the command id.
    pub fn find(&self, explicit: &str, id: &str) -> Option<&Path> {
        self.by_path
            .get(explicit)
            .or_else(|| self.by_stem.get(id))
            .map(PathBuf::as_path)
    }
}

/// Where an icon's bytes are kept in egui; the extension tells the loaders what it is.
fn uri(path: &Path) -> String {
    format!("bytes://{}", path.display())
}

/// A 1×1 24-bit BMP: file header, info header, one pixel padded to 4 bytes.
#[cfg(test)]
pub fn tiny_bmp() -> Vec<u8> {
    let mut bmp = b"BM".to_vec();
    for n in [58_u32, 0, 54, 40, 1, 1] {
        bmp.extend(n.to_le_bytes());
    }
    bmp.extend(1_u16.to_le_bytes());
    bmp.extend(24_u16.to_le_bytes());
    bmp.extend([0_u8; 24]);
    bmp.extend([255, 0, 0, 0]);
    bmp
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn icon_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("cut-craft-icons-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("drawing/create_vectors")).unwrap();
        fs::create_dir_all(dir.join("file")).unwrap();
        fs::write(
            dir.join("drawing/create_vectors/draw_line_polyline.svg"),
            "<svg/>",
        )
        .unwrap();
        fs::write(dir.join("file/save.svg"), "<svg/>").unwrap();
        dir
    }

    #[test]
    fn explicit_icon_path_wins() {
        let dir = icon_dir("explicit");
        let icons = Icons::load(&dir);
        let found = icons.find(
            "drawing/create_vectors/draw_line_polyline.svg",
            "draw_polyline",
        );
        assert_eq!(
            found,
            Some(
                dir.join("drawing/create_vectors/draw_line_polyline.svg")
                    .as_path()
            )
        );
    }

    #[test]
    fn falls_back_to_a_file_named_after_the_command() {
        let dir = icon_dir("stem");
        let icons = Icons::load(&dir);
        assert_eq!(
            icons.find("", "save"),
            Some(dir.join("file/save.svg").as_path())
        );
        assert_eq!(
            icons.find("missing/thing.svg", "save"),
            Some(dir.join("file/save.svg").as_path())
        );
    }

    #[test]
    fn dialog_pictures_are_bitmaps_found_by_their_path() {
        let dir = icon_dir("bitmap");
        fs::create_dir_all(dir.join("import")).unwrap();
        fs::write(dir.join("import/initial_orientation.bmp"), "BM").unwrap();
        fs::write(dir.join("import/notes.txt"), "").unwrap();
        fs::write(dir.join("import/link.png"), "").unwrap();
        let icons = Icons::load(&dir);
        assert_eq!(
            icons.find("import/initial_orientation.bmp", ""),
            Some(dir.join("import/initial_orientation.bmp").as_path())
        );
        assert_eq!(icons.find("import/notes.txt", ""), None);
        assert!(icons.find("import/link.png", "").is_some(), "PNG icons too");
    }

    #[test]
    fn egui_decodes_bitmaps() {
        let bmp = tiny_bmp();
        let ctx = egui::Context::default();
        egui_extras::install_image_loaders(&ctx);
        ctx.include_bytes("bytes://picture.bmp", bmp);
        let loaded = ctx.try_load_image("bytes://picture.bmp", egui::SizeHint::default());
        let error = loaded.as_ref().err().map(ToString::to_string);
        assert!(
            matches!(loaded, Ok(egui::load::ImagePoll::Ready { .. })),
            "{error:?}"
        );
    }

    #[test]
    fn preloaded_icons_are_ready_the_first_time_they_are_drawn() {
        let dir = icon_dir("preload");
        fs::create_dir_all(dir.join("import")).unwrap();
        let path = dir.join("import/picture.bmp");
        fs::write(&path, tiny_bmp()).unwrap();
        let mut icons = Icons::load(&dir);
        let ctx = egui::Context::default();
        egui_extras::install_image_loaders(&ctx);
        icons.preload(&ctx);
        let uri = Icons::image(&path).source().uri().unwrap().to_owned();
        let loaded = ctx.try_load_image(&uri, egui::SizeHint::default());
        assert!(
            matches!(loaded, Ok(egui::load::ImagePoll::Ready { .. })),
            "no loading spinner: {:?}",
            loaded.err().map(|e| e.to_string())
        );
    }

    #[test]
    fn unknown_icons_and_missing_directories_give_none() {
        let dir = icon_dir("none");
        assert_eq!(Icons::load(&dir).find("", "weld_vectors"), None);
        assert_eq!(
            Icons::load(&dir.join("does-not-exist")).find("", "save"),
            None
        );
    }
}
