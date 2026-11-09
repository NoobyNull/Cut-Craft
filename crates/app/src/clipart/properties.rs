//! Right-click ▸ Properties on a Library tile: what the file is, how big it is (on disk
//! and, for a model or picture, in itself), when it last changed, its tags, and where it is,
//! with buttons to copy its path or show it in its folder. A model is measured in the
//! background, so a big one on a network folder doesn't hold the tab up.

use super::Kind;
use cut_craft_bug_report::hint::HoverHint;
use eframe::egui;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::SystemTime;

/// Millimetres in an inch.
const MM_PER_INCH: f64 = 25.4;

/// What a model or picture measures, read from the file itself.
#[derive(Clone, Debug, PartialEq)]
pub enum Measured {
    /// Its width (X), length (Y) and thickness (Z) as saved, in mm; and how many
    /// triangles its surface is made of (a Cut Craft clipart file is a grid, not triangles).
    Model {
        size: [f64; 3],
        triangles: Option<usize>,
    },
    /// Its width and height in pixels.
    Picture { width: u32, height: u32 },
    /// Nothing to measure (a drawing, say).
    Nothing,
}

/// One file's properties, as the window shows them.
pub struct Properties {
    pub file: PathBuf,
    pub kind: Kind,
    /// The picture beside a model that stands in as its preview.
    pub picture: Option<PathBuf>,
    /// The library folder (as added) the file is in.
    pub library_folder: Option<PathBuf>,
    pub bytes: Option<u64>,
    pub modified: Option<SystemTime>,
    pub tags: Vec<String>,
    /// What it measures, once read; or why it couldn't be.
    pub measured: Option<Result<Measured, String>>,
    /// The measuring under way in the background.
    measuring: Option<mpsc::Receiver<Result<Measured, String>>>,
}

impl Properties {
    /// `file`'s properties, the file's size and date read now; what it measures is read in
    /// the background (`ctx` is woken when it is). `folders` are the library's.
    pub fn new(
        file: &Path,
        kind: Kind,
        picture: Option<&Path>,
        tags: Vec<String>,
        folders: &[PathBuf],
        ctx: Option<&egui::Context>,
    ) -> Self {
        let metadata = std::fs::metadata(file).ok();
        let (send, measuring) = mpsc::channel();
        let (owned, ctx) = (file.to_path_buf(), ctx.cloned());
        std::thread::spawn(move || {
            // the window may be closed by now: then nobody is waiting
            let _ = send.send(measure(&owned));
            if let Some(ctx) = ctx {
                ctx.request_repaint();
            }
        });
        Self {
            file: file.to_path_buf(),
            kind,
            picture: picture.map(Path::to_path_buf),
            library_folder: library_folder(file, folders),
            bytes: metadata.as_ref().map(std::fs::Metadata::len),
            modified: metadata.and_then(|m| m.modified().ok()),
            tags,
            measured: None,
            measuring: Some(measuring),
        }
    }

    /// Takes what the file measures, once it has been measured.
    pub fn update(&mut self) {
        if let Some(rx) = &self.measuring {
            match rx.try_recv() {
                Ok(m) => self.measured = Some(m),
                Err(mpsc::TryRecvError::Empty) => return,
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.measured = Some(Err("Measuring it stopped unexpectedly.".into()));
                }
            }
            self.measuring = None;
        }
    }

    /// Waits for the measuring to finish (for tests).
    #[cfg(test)]
    pub fn wait(&mut self) {
        if let Some(rx) = self.measuring.take() {
            self.measured = Some(
                rx.recv()
                    .unwrap_or_else(|_| Err("Measuring it stopped unexpectedly.".into())),
            );
        }
    }
}

/// The folder of `folders` that `file` is in: the outermost, as the tree shows it.
fn library_folder(file: &Path, folders: &[PathBuf]) -> Option<PathBuf> {
    (folders.iter())
        .filter(|f| file.starts_with(f))
        .min_by_key(|f| f.components().count())
        .cloned()
}

/// What `file` measures: a model's size and triangles, a picture's pixels.
pub fn measure(file: &Path) -> Result<Measured, String> {
    let extension = super::preview::extension(file);
    let name = super::name(file);
    if extension == "ccclip" {
        let bytes = std::fs::read(file).map_err(|e| format!("Couldn't read {name}: {e}"))?;
        let k = fmt_export_as_3d_clipart::read_clip(&bytes).map_err(|e| format!("{name}: {e}"))?;
        let size = k.extent().size();
        let (low, high) = k.height_range().unwrap_or_default();
        return Ok(Measured::Model {
            size: [size.x, size.y, high - low],
            triangles: None,
        });
    }
    if super::preview::MODELS.contains(&extension.as_str()) {
        let bytes = std::fs::read(file).map_err(|e| format!("Couldn't read {name}: {e}"))?;
        let triangles = fmt_import_stl::triangles(&bytes).map_err(|e| format!("{name}: {e}"))?;
        let corners = triangles.iter().flatten();
        let low = corners
            .clone()
            .copied()
            .reduce(cut_craft_core::glam::DVec3::min);
        let high = corners.copied().reduce(cut_craft_core::glam::DVec3::max);
        let size = match (low, high) {
            (Some(low), Some(high)) => (high - low).to_array(),
            _ => [0.0; 3],
        };
        return Ok(Measured::Model {
            size,
            triangles: Some(triangles.len()),
        });
    }
    if super::BITMAPS.contains(&extension.as_str()) {
        let (width, height) =
            image::image_dimensions(file).map_err(|e| format!("Couldn't read {name}: {e}"))?;
        return Ok(Measured::Picture { width, height });
    }
    Ok(Measured::Nothing)
}

/// `n` with its thousands grouped: 1,234,567.
pub fn grouped(n: u64) -> String {
    let digits = n.to_string();
    let mut out = String::new();
    for (i, d) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(d);
    }
    out
}

/// A file's size as people read it: "43.7 MB (45,823,412 bytes)".
#[expect(clippy::cast_precision_loss, reason = "shown to one decimal place")]
pub fn file_size(bytes: u64) -> String {
    let units = ["KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit = None;
    for u in units {
        if size < 1024.0 {
            break;
        }
        size /= 1024.0;
        unit = Some(u);
    }
    match unit {
        None if bytes == 1 => "1 byte".into(),
        None => format!("{bytes} bytes"),
        Some(u) => format!("{size:.1} {u} ({} bytes)", grouped(bytes)),
    }
}

/// A model's size in mm and inches: "120.0 × 80.0 × 12.5 mm (4.72 × 3.15 × 0.49 in)".
pub fn model_size([x, y, z]: [f64; 3]) -> String {
    let inch = |v: f64| v / MM_PER_INCH;
    format!(
        "{x:.1} × {y:.1} × {z:.1} mm ({:.2} × {:.2} × {:.2} in)",
        inch(x),
        inch(y),
        inch(z)
    )
}

/// What a kind of file is called, one of them.
pub fn kind_name(kind: Kind) -> &'static str {
    match kind {
        Kind::Model => "3D model",
        Kind::Drawing => "2D drawing",
        Kind::Picture => "Picture",
        Kind::DepthMap => "Depth map (heights as shades of grey)",
    }
}

/// What the window asks for.
pub enum Asked {
    /// Show the file in its folder.
    Locate(PathBuf),
}

/// The Properties window, while `open`; what it asked for. Closed by Close, its X or
/// Escape.
pub fn window(ctx: &egui::Context, open: &mut Option<Properties>) -> Option<Asked> {
    let props = open.as_mut()?;
    props.update();
    let mut asked = None;
    let mut shown = true;
    let mut close = ctx.input(|i| i.key_pressed(egui::Key::Escape));
    egui::Window::new("Properties")
        .id(egui::Id::unique("library_properties"))
        .open(&mut shown)
        .collapsible(false)
        .resizable(false)
        .default_width(380.0)
        .show(ctx, |ui| {
            ui.heading(super::name(&props.file));
            ui.add_space(4.0);
            egui::Grid::new("library_properties_grid")
                .num_columns(2)
                .spacing([12.0, 6.0])
                .show(ui, |ui| rows(ui, props));
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui
                    .button("Copy Path")
                    .on_hover_hint("Copies where the file is, to paste somewhere else.")
                    .clicked()
                {
                    ui.ctx().copy_text(props.file.display().to_string());
                }
                if ui
                    .button("Locate")
                    .on_hover_hint("Shows the file in its folder.")
                    .clicked()
                {
                    asked = Some(Asked::Locate(props.file.clone()));
                }
                if ui.button("Close").clicked() {
                    close = true;
                }
            });
        });
    if close || !shown {
        *open = None;
    }
    asked
}

/// The window's rows: a name on the left, what it is on the right.
fn rows(ui: &mut egui::Ui, props: &Properties) {
    let extension = super::preview::extension(&props.file).to_uppercase();
    let row = |ui: &mut egui::Ui, name: &str, value: String, hint: Option<&str>| {
        let label = ui.strong(name);
        if let Some(hint) = hint {
            label.on_hover_hint(hint);
        }
        ui.add(egui::Label::new(value).wrap());
        ui.end_row();
    };
    let kind = if extension.is_empty() {
        kind_name(props.kind).to_owned()
    } else {
        format!("{} ({extension} file)", kind_name(props.kind))
    };
    row(ui, "Kind", kind, None);
    match &props.measured {
        None => row(ui, "Size", "Measuring...".into(), None),
        Some(Err(e)) => row(ui, "Size", format!("Couldn't measure it. {e}"), None),
        Some(Ok(Measured::Model { size, triangles })) => {
            row(
                ui,
                "Size",
                model_size(*size),
                Some(
                    "Width (X) × length (Y) × thickness (Z), as saved in the file. You can \
                     change its size when you add it to a project.",
                ),
            );
            if let Some(n) = triangles {
                row(
                    ui,
                    "Triangles",
                    grouped(*n as u64),
                    Some(
                        "A model's surface is made of flat triangles. More of them means \
                         finer detail, and a slower file to open.",
                    ),
                );
            }
        }
        Some(Ok(Measured::Picture { width, height })) => {
            row(ui, "Size", format!("{width} × {height} pixels"), None);
        }
        Some(Ok(Measured::Nothing)) => {}
    }
    if let Some(picture) = &props.picture {
        row(
            ui,
            "Preview",
            super::name(picture),
            Some("The picture beside it with the same name, shown as its tile."),
        );
    }
    let on_disk = props.bytes.map_or_else(|| "Unknown".into(), file_size);
    row(ui, "File size", on_disk, None);
    let modified = props.modified.map_or_else(
        || "Unknown".into(),
        |m| {
            chrono::DateTime::<chrono::Local>::from(m)
                .format("%Y-%m-%d %H:%M")
                .to_string()
        },
    );
    row(ui, "Modified", modified, None);
    let tags = if props.tags.is_empty() {
        "None yet. Right-click it and pick Add Tag... to add one.".into()
    } else {
        props.tags.join(", ")
    };
    row(
        ui,
        "Tags",
        tags,
        Some("Words to find it by: the search box looks through them."),
    );
    if let Some(folder) = &props.library_folder {
        row(
            ui,
            "Library folder",
            folder.display().to_string(),
            Some("The folder you added to the Library that this file is in."),
        );
    }
    row(ui, "Location", props.file.display().to_string(), None);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fmt::Write as _;

    fn folder(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "cut-craft-properties-{name}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// An ASCII STL of `triangles`, each three corners.
    fn stl(triangles: &[[[f64; 3]; 3]]) -> String {
        let mut s = String::from("solid t\n");
        for t in triangles {
            s.push_str(" facet normal 0 0 1\n  outer loop\n");
            for [x, y, z] in t {
                writeln!(s, "   vertex {x} {y} {z}").unwrap();
            }
            s.push_str("  endloop\n endfacet\n");
        }
        s.push_str("endsolid t\n");
        s
    }

    #[test]
    fn a_model_measures_its_box_and_triangles() {
        let dir = folder("model");
        let file = dir.join("wedge.stl");
        let wedge = [
            [[0.0, 0.0, 0.0], [100.0, 0.0, 0.0], [100.0, 50.0, 0.0]],
            [[0.0, 0.0, 0.0], [100.0, 50.0, 12.5], [0.0, 50.0, 0.0]],
        ];
        std::fs::write(&file, stl(&wedge)).unwrap();
        assert_eq!(
            measure(&file).unwrap(),
            Measured::Model {
                size: [100.0, 50.0, 12.5],
                triangles: Some(2)
            }
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_picture_measures_its_pixels() {
        let dir = folder("picture");
        let file = dir.join("bird.png");
        image::RgbaImage::new(30, 20).save(&file).unwrap();
        assert_eq!(
            measure(&file).unwrap(),
            Measured::Picture {
                width: 30,
                height: 20
            }
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_drawing_has_nothing_to_measure_and_a_broken_model_says_so() {
        let dir = folder("other");
        let (svg, broken) = (dir.join("leaf.svg"), dir.join("broken.stl"));
        std::fs::write(&svg, "<svg/>").unwrap();
        std::fs::write(&broken, "not a model").unwrap();
        assert_eq!(measure(&svg).unwrap(), Measured::Nothing);
        let e = measure(&broken).unwrap_err();
        assert!(e.contains("broken.stl"), "{e}");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn sizes_read_as_people_say_them() {
        assert_eq!(grouped(0), "0");
        assert_eq!(grouped(999), "999");
        assert_eq!(grouped(1_000), "1,000");
        assert_eq!(grouped(45_823_412), "45,823,412");
        assert_eq!(file_size(1), "1 byte");
        assert_eq!(file_size(900), "900 bytes");
        assert_eq!(file_size(2048), "2.0 KB (2,048 bytes)");
        assert_eq!(file_size(45_823_412), "43.7 MB (45,823,412 bytes)");
        assert_eq!(
            model_size([127.0, 25.4, 12.7]),
            "127.0 × 25.4 × 12.7 mm (5.00 × 1.00 × 0.50 in)"
        );
    }

    /// The file's size, date and tags, and the library folder it's in (the outermost added,
    /// as the tree shows it), are read as the window opens; its measuring follows.
    #[test]
    fn properties_of_a_file() {
        let dir = folder("props");
        let inner = dir.join("animals");
        std::fs::create_dir_all(&inner).unwrap();
        let file = inner.join("bird.png");
        image::RgbaImage::new(4, 2).save(&file).unwrap();
        let folders = [inner.clone(), dir.clone(), folder("elsewhere")];
        let mut props = Properties::new(
            &file,
            Kind::Picture,
            None,
            vec!["birds".into()],
            &folders,
            None,
        );
        assert_eq!(props.library_folder.as_deref(), Some(dir.as_path()));
        assert_eq!(props.bytes, Some(std::fs::metadata(&file).unwrap().len()));
        assert!(props.modified.is_some());
        assert_eq!(props.tags, ["birds"]);
        props.wait();
        assert_eq!(
            props.measured,
            Some(Ok(Measured::Picture {
                width: 4,
                height: 2
            }))
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
