//! Tool icons: SVGs under the icon directory (redrawn by `re/make_icons.py`, in git), a few
//! credited PNGs, and dialog pictures kept as Aspire's own bitmaps; all in git.

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
    /// Whether the bitmaps are, which always are (for their alpha).
    bitmaps_loaded: bool,
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
            bitmaps_loaded: false,
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

    /// `CUT_CRAFT_ICON_DIR`, else `icons` in the assets folder ([`crate::files::assets_dir`]).
    pub fn default_dir() -> PathBuf {
        std::env::var_os("CUT_CRAFT_ICON_DIR")
            .map_or_else(|| crate::files::assets_dir().join("icons"), PathBuf::from)
    }

    /// Reads the bitmaps into egui once, keeping their alpha (see [`with_alpha`]); with
    /// `all`, every other icon too, so none shows a loading spinner the first time it is drawn
    /// (egui's own `file://` loader reads on a thread and draws a frame without it).
    // ponytail: every icon at startup (~200 files, ~10 MB); load per folder if that grows slow
    pub fn preload(&mut self, ctx: &egui::Context, all: bool) {
        let bitmaps = !std::mem::replace(&mut self.bitmaps_loaded, true);
        let others = all && !std::mem::replace(&mut self.preloaded, true);
        for path in self.by_path.values() {
            let bitmap = is_bitmap(path);
            if !(bitmap && bitmaps || !bitmap && others) {
                continue;
            }
            match fs::read(path) {
                Ok(bytes) => {
                    let bytes = if bitmap {
                        with_alpha(&bytes).unwrap_or(bytes)
                    } else {
                        bytes
                    };
                    ctx.include_bytes(uri(path), bytes);
                    // decoding runs on a thread of egui's: started now, ready by the first draw
                    let _ = ctx.try_load_image(&uri(path), egui::SizeHint::default());
                }
                Err(e) => eprintln!("icon {}: {e}", path.display()),
            }
        }
    }

    /// An indexed icon: from the bytes [`Icons::preload`] read, else read when first drawn.
    pub fn image(&self, path: &Path) -> egui::Image<'static> {
        if self.preloaded || self.bitmaps_loaded && is_bitmap(path) {
            egui::Image::new(uri(path))
        } else {
            egui::Image::new(format!("file://{}", path.display()))
        }
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

fn is_bitmap(path: &Path) -> bool {
    path.extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("bmp"))
}

/// A 32-bit bitmap with its alpha kept, as a PNG: Aspire's dialog pictures are drawn on a
/// see-through background, and egui's bitmap reader takes the fourth byte for padding and
/// shows that background black. `None` for any other bitmap, or one whose alpha is all
/// zero (which then is padding).
// ponytail: uncompressed bitmaps with the 40-byte header, which is what Aspire's are
fn with_alpha(bmp: &[u8]) -> Option<Vec<u8>> {
    let u32_at = |at: usize| Some(u32::from_le_bytes(bmp.get(at..at + 4)?.try_into().ok()?));
    let i32_at = |at: usize| Some(i32::from_le_bytes(bmp.get(at..at + 4)?.try_into().ok()?));
    let bpp = u16::from_le_bytes(bmp.get(28..30)?.try_into().ok()?);
    if !bmp.starts_with(b"BM") || bpp != 32 || u32_at(30)? != 0 {
        return None;
    }
    let start = usize::try_from(u32_at(10)?).ok()?;
    let (w, h) = (i32_at(18)?, i32_at(22)?);
    let (width, rows) = (
        usize::try_from(w).ok()?,
        usize::try_from(h.unsigned_abs()).ok()?,
    );
    let pixels = bmp.get(start..start + width * rows * 4)?;
    if pixels.as_chunks::<4>().0.iter().all(|p| p[3] == 0) {
        return None;
    }
    let mut rgba = Vec::with_capacity(pixels.len());
    for row in 0..rows {
        // rows run bottom up unless the height is negative
        let from = if h > 0 { rows - 1 - row } else { row };
        for p in pixels[from * width * 4..(from + 1) * width * 4]
            .as_chunks::<4>()
            .0
        {
            rgba.extend([p[2], p[1], p[0], p[3]]);
        }
    }
    let image = image::RgbaImage::from_raw(w.unsigned_abs(), h.unsigned_abs(), rgba)?;
    let mut png = std::io::Cursor::new(vec![]);
    image.write_to(&mut png, image::ImageFormat::Png).ok()?;
    Some(png.into_inner())
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

    /// The picture at `uri` once egui's decoding thread has it (a few milliseconds).
    fn decoded(ctx: &egui::Context, uri: &str) -> Result<std::sync::Arc<egui::ColorImage>, String> {
        let until = std::time::Instant::now() + std::time::Duration::from_secs(5);
        loop {
            match ctx.try_load_image(uri, egui::SizeHint::default()) {
                Ok(egui::load::ImagePoll::Ready { image }) => return Ok(image),
                Ok(egui::load::ImagePoll::Pending { .. }) if std::time::Instant::now() < until => {
                    std::thread::sleep(std::time::Duration::from_millis(5));
                }
                Ok(egui::load::ImagePoll::Pending { .. }) => return Err("still pending".into()),
                Err(e) => return Err(e.to_string()),
            }
        }
    }

    #[test]
    fn egui_decodes_bitmaps() {
        let bmp = tiny_bmp();
        let ctx = egui::Context::default();
        egui_extras::install_image_loaders(&ctx);
        ctx.include_bytes("bytes://picture.bmp", bmp);
        assert!(decoded(&ctx, "bytes://picture.bmp").is_ok());
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
        let lazy = icons.image(&path).source(&ctx).uri().unwrap().to_owned();
        assert!(lazy.starts_with("file://"), "read when first drawn: {lazy}");
        icons.preload(&ctx, true);
        let uri = icons.image(&path).source(&ctx).uri().unwrap().to_owned();
        assert!(uri.starts_with("bytes://"), "read already: {uri}");
        assert!(decoded(&ctx, &uri).is_ok(), "decoded from the bytes read");
    }

    /// A 2 × 1 32-bit bitmap, as Aspire's dialog pictures are: a clear white pixel, then an
    /// opaque black one (blue, green, red, alpha).
    fn clear_and_black_bmp() -> Vec<u8> {
        let mut bmp = b"BM".to_vec();
        for n in [62_u32, 0, 54, 40, 2, 1] {
            bmp.extend(n.to_le_bytes());
        }
        bmp.extend(1_u16.to_le_bytes());
        bmp.extend(32_u16.to_le_bytes());
        bmp.extend([0_u8; 24]);
        bmp.extend([255, 255, 255, 0, 0, 0, 0, 255]);
        bmp
    }

    #[test]
    fn bitmaps_keep_their_see_through_parts() {
        let dir = icon_dir("alpha");
        fs::create_dir_all(dir.join("drawing_forms")).unwrap();
        let path = dir.join("drawing_forms/corner.bmp");
        fs::write(&path, clear_and_black_bmp()).unwrap();
        let mut icons = Icons::load(&dir);
        let ctx = egui::Context::default();
        egui_extras::install_image_loaders(&ctx);
        // bitmaps are read up front even when other icons are not, for their alpha
        icons.preload(&ctx, false);
        let uri = icons.image(&path).source(&ctx).uri().unwrap().to_owned();
        let image = decoded(&ctx, &uri).expect("the bitmap decodes");
        assert_eq!(image.size, [2, 1]);
        assert_eq!(
            image.pixels[0].a(),
            0,
            "the clear pixel shows what is behind it"
        );
        assert_eq!(image.pixels[1], egui::Color32::BLACK);
        // a 24-bit bitmap has no alpha to keep, and is left as it is
        assert!(with_alpha(&tiny_bmp()).is_none());
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
