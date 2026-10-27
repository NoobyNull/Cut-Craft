//! Tool icons, built into the program from `assets/icons` (SVGs redrawn by `re/make_icons.py`,
//! a few credited PNGs, and dialog pictures kept as Aspire's own bitmaps; all in git), or read
//! from the folder `CUT_CRAFT_ICON_DIR` names while icons are being worked on.

use eframe::egui;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// The table `build.rs` writes: every file under `assets/icons` with its bytes.
mod built_in {
    include!(concat!(env!("OUT_DIR"), "/icons.rs"));
}

/// Where an icon's bytes come from.
enum Source {
    BuiltIn(&'static [u8]),
    File(PathBuf),
}

pub struct Icons {
    /// Relative path (`drawing/create_vectors/draw_circle.svg`) → where its bytes are.
    by_path: HashMap<String, Source>,
    /// File stem (`draw_circle`) → that relative path, for SVGs only.
    by_stem: HashMap<String, String>,
    /// Whether every icon's bytes are already in egui (see [`Icons::preload`]).
    preloaded: bool,
    /// Whether the bitmaps are, which always are (for their alpha).
    bitmaps_loaded: bool,
}

impl Icons {
    fn empty() -> Self {
        Self {
            by_path: HashMap::new(),
            by_stem: HashMap::new(),
            preloaded: false,
            bitmaps_loaded: false,
        }
    }

    /// The icons built into the program.
    pub fn built_in() -> Self {
        let mut icons = Self::empty();
        for &(key, bytes) in built_in::BUILT_IN {
            icons.index(key, Source::BuiltIn(bytes));
        }
        icons
    }

    /// Indexes every `.svg`, `.bmp` and `.png` under `dir`; a missing directory gives an empty
    /// index.
    pub fn load(dir: &Path) -> Self {
        let mut icons = Self::empty();
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
                } else if path
                    .extension()
                    .is_some_and(|e| matches!(e.to_str(), Some("svg" | "bmp" | "png")))
                    && let Ok(rel) = path.strip_prefix(dir)
                {
                    // keyed with `/` as commands name them, whatever the platform's separator
                    let key = rel
                        .components()
                        .map(|c| c.as_os_str().to_string_lossy())
                        .collect::<Vec<_>>()
                        .join("/");
                    icons.index(&key, Source::File(path));
                }
            }
        }
        icons
    }

    /// The built-in icons, unless `CUT_CRAFT_ICON_DIR` names a folder to read instead.
    pub fn default_set() -> Self {
        match std::env::var_os("CUT_CRAFT_ICON_DIR") {
            Some(dir) => Self::load(Path::new(&dir)),
            None => Self::built_in(),
        }
    }

    /// Only SVGs stand in for a command by their name.
    fn index(&mut self, key: &str, source: Source) {
        if let Some(stem) = key.strip_suffix(".svg").and_then(|k| k.rsplit('/').next()) {
            self.by_stem
                .entry(stem.to_owned())
                .or_insert_with(|| key.to_owned());
        }
        self.by_path.insert(key.to_owned(), source);
    }

    /// Hands egui the icons' bytes: the built-in ones all at once the first time (they cost
    /// nothing to hand over), and from a folder the bitmaps at once (keeping their alpha, see
    /// [`with_alpha`]) and, with `all`, every other icon, so none shows a loading spinner the
    /// first time it is drawn (egui's own `file://` loader reads on a thread and draws a frame
    /// without it).
    pub fn preload(&mut self, ctx: &egui::Context, all: bool) {
        let first = !std::mem::replace(&mut self.bitmaps_loaded, true);
        let others = all && !std::mem::replace(&mut self.preloaded, true);
        let built_in = first
            && self
                .by_path
                .values()
                .any(|s| matches!(s, Source::BuiltIn(_)));
        for (key, source) in &self.by_path {
            let bitmap = is_bitmap(key);
            if !(built_in || bitmap && first || !bitmap && others) {
                continue;
            }
            let bytes: egui::load::Bytes = match source {
                Source::BuiltIn(bytes) => match with_alpha(bytes).filter(|_| bitmap) {
                    Some(png) => png.into(),
                    None => egui::load::Bytes::Static(bytes),
                },
                Source::File(path) => match fs::read(path) {
                    Ok(bytes) if bitmap => with_alpha(&bytes).unwrap_or(bytes).into(),
                    Ok(bytes) => bytes.into(),
                    Err(e) => {
                        eprintln!("icon {}: {e}", path.display());
                        continue;
                    }
                },
            };
            ctx.include_bytes(uri(key), bytes);
            // decoding runs on a thread of egui's: started now, ready by the first draw
            let _ = ctx.try_load_image(&uri(key), egui::SizeHint::default());
        }
        if built_in {
            self.preloaded = true;
        }
    }

    /// An indexed icon: from the bytes [`Icons::preload`] handed over, else its file, read
    /// when first drawn.
    pub fn image(&self, key: &str) -> egui::Image<'static> {
        let ready = self.preloaded || self.bitmaps_loaded && is_bitmap(key);
        match self.by_path.get(key) {
            Some(Source::File(path)) if !ready => {
                egui::Image::new(format!("file://{}", path.display()))
            }
            _ => egui::Image::new(uri(key)),
        }
    }

    /// The icon a command names explicitly, else one whose file stem is the command id.
    pub fn find(&self, explicit: &str, id: &str) -> Option<&str> {
        self.by_path
            .get_key_value(explicit)
            .map(|(key, _)| key.as_str())
            .or_else(|| self.by_stem.get(id).map(String::as_str))
    }
}

/// Where an icon's bytes are kept in egui; the extension tells the loaders what it is.
fn uri(key: &str) -> String {
    format!("bytes://icons/{key}")
}

fn is_bitmap(key: &str) -> bool {
    Path::new(key)
        .extension()
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
    fn the_built_in_icons_are_the_whole_folder_and_draw_without_a_file() {
        let mut icons = Icons::built_in();
        assert!(icons.by_path.len() > 200, "{} icons", icons.by_path.len());
        assert_eq!(
            icons.find("", "draw_circle"),
            Some("drawing/create_vectors/draw_circle.svg")
        );
        assert!(icons.find("tool_database/end_mill.bmp", "").is_some());
        let ctx = egui::Context::default();
        egui_extras::install_image_loaders(&ctx);
        icons.preload(&ctx, false);
        for key in ["drawing/align/left.svg", "tool_database/end_mill.bmp"] {
            let uri = icons.image(key).source(&ctx).uri().unwrap().to_owned();
            assert!(uri.starts_with("bytes://icons/"), "{uri}");
            assert!(decoded(&ctx, &uri).is_ok(), "{key} decodes");
        }
    }

    #[test]
    fn explicit_icon_path_wins() {
        let icons = Icons::load(&icon_dir("explicit"));
        let found = icons.find(
            "drawing/create_vectors/draw_line_polyline.svg",
            "draw_polyline",
        );
        assert_eq!(found, Some("drawing/create_vectors/draw_line_polyline.svg"));
    }

    #[test]
    fn falls_back_to_a_file_named_after_the_command() {
        let icons = Icons::load(&icon_dir("stem"));
        assert_eq!(icons.find("", "save"), Some("file/save.svg"));
        assert_eq!(
            icons.find("missing/thing.svg", "save"),
            Some("file/save.svg")
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
            Some("import/initial_orientation.bmp")
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
        assert!(decoded(&ctx, "bytes://picture.bmp").is_ok());
    }

    #[test]
    fn preloaded_icons_are_ready_the_first_time_they_are_drawn() {
        let dir = icon_dir("preload");
        fs::create_dir_all(dir.join("import")).unwrap();
        fs::write(dir.join("import/picture.bmp"), tiny_bmp()).unwrap();
        let mut icons = Icons::load(&dir);
        let ctx = egui::Context::default();
        egui_extras::install_image_loaders(&ctx);
        let key = "import/picture.bmp";
        let lazy = icons.image(key).source(&ctx).uri().unwrap().to_owned();
        assert!(lazy.starts_with("file://"), "read when first drawn: {lazy}");
        icons.preload(&ctx, true);
        let uri = icons.image(key).source(&ctx).uri().unwrap().to_owned();
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
        fs::write(dir.join("drawing_forms/corner.bmp"), clear_and_black_bmp()).unwrap();
        let mut icons = Icons::load(&dir);
        let ctx = egui::Context::default();
        egui_extras::install_image_loaders(&ctx);
        // bitmaps are read up front even when other icons are not, for their alpha
        icons.preload(&ctx, false);
        let key = "drawing_forms/corner.bmp";
        let uri = icons.image(key).source(&ctx).uri().unwrap().to_owned();
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
