//! The Depth add-on, seen from the app: whether it is installed, and Estimate Depth, which
//! runs it on the chosen picture in the background and gives the picture its depth (for
//! Relief from Photo). The add-on is a separate download (a program and about 100 MB of
//! model weights) so the app and its updates stay small.

use cut_craft_core::bitmap::Bitmap;
use cut_craft_core::document::EntityId;
use std::path::{Path, PathBuf};
use std::sync::mpsc;

/// The add-on's id: its folder under the add-ons folder.
pub const ADDON_ID: &str = "depth";
/// The model weights' file in the add-on's folder.
pub const WEIGHTS: &str = "depth_anything_v2_vits.safetensors";

/// The add-on's program file name on this system.
pub fn program_name() -> String {
    format!("cutcraft-depth{}", std::env::consts::EXE_SUFFIX)
}

/// Where add-ons are installed, each in a folder named by its id (none in tests).
pub fn addons_dir() -> Option<PathBuf> {
    crate::files::config_dir().map(|d| d.join("addons"))
}

/// The installed add-on's program and weights.
#[derive(Clone, Debug, PartialEq)]
pub struct Installed {
    pub program: PathBuf,
    pub weights: PathBuf,
}

/// The add-on in `addons` (the add-ons folder), if both its program and weights are there.
pub fn find(addons: &Path) -> Option<Installed> {
    let dir = addons.join(ADDON_ID);
    let installed = Installed {
        program: dir.join(program_name()),
        weights: dir.join(WEIGHTS),
    };
    (installed.program.is_file() && installed.weights.is_file()).then_some(installed)
}

/// The picture as a PNG for the add-on: its colours when it has them, else its greys.
pub fn photo_png(bitmap: &Bitmap) -> Result<Vec<u8>, String> {
    let (w, h) = dims(bitmap)?;
    let rgb = if bitmap.rgb.len() == 3 * bitmap.grey.len() {
        bitmap.rgb.clone()
    } else {
        bitmap.grey.iter().flat_map(|&g| [g, g, g]).collect()
    };
    let image = image::RgbImage::from_raw(w, h, rgb).ok_or("the picture's pixels are short")?;
    let mut png = Vec::new();
    image
        .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
        .map_err(|e| e.to_string())?;
    Ok(png)
}

/// The add-on's answer (a grey PNG, white nearest) as depth for `bitmap`: 0 to 1 a pixel,
/// laid out as its greys.
pub fn read_depth(png: &[u8], bitmap: &Bitmap) -> Result<Vec<f32>, String> {
    let (w, h) = dims(bitmap)?;
    let depth = image::load_from_memory(png)
        .map_err(|e| format!("the add-on's answer does not read: {e}"))?
        .into_luma16();
    let depth = if depth.dimensions() == (w, h) {
        depth
    } else {
        image::imageops::resize(&depth, w, h, image::imageops::FilterType::Triangle)
    };
    Ok(depth
        .into_raw()
        .into_iter()
        .map(|v| f32::from(v) / 65_535.0)
        .collect())
}

fn dims(bitmap: &Bitmap) -> Result<(u32, u32), String> {
    let size = |n: usize| u32::try_from(n).map_err(|_| "the picture is too big".to_string());
    Ok((size(bitmap.columns)?, size(bitmap.rows)?))
}

/// `bitmap` with `depth` as its depth.
pub fn with_depth(bitmap: &Bitmap, depth: Vec<f32>) -> Bitmap {
    Bitmap {
        depth,
        ..bitmap.clone()
    }
}

/// An estimate running in the background: for which picture, and its answer to come.
pub struct Estimating {
    pub bitmap: EntityId,
    answer: mpsc::Receiver<Result<Vec<f32>, String>>,
}

impl Estimating {
    /// Starts the add-on on `bitmap` (`id`); `ctx` is woken when it answers.
    pub fn start(
        installed: &Installed,
        id: EntityId,
        bitmap: &Bitmap,
        ctx: &eframe::egui::Context,
    ) -> Result<Self, String> {
        let png = photo_png(bitmap)?;
        let (send, answer) = mpsc::channel();
        let (installed, bitmap, ctx) = (installed.clone(), bitmap.clone(), ctx.clone());
        std::thread::Builder::new()
            .name("depth estimate".into())
            .spawn(move || {
                let _ = send.send(run(&installed, &png, &bitmap));
                ctx.request_repaint();
            })
            .map_err(|e| e.to_string())?;
        Ok(Self { bitmap: id, answer })
    }

    /// The answer once there is one.
    pub fn answer(&self) -> Option<Result<Vec<f32>, String>> {
        match self.answer.try_recv() {
            Ok(answer) => Some(answer),
            Err(mpsc::TryRecvError::Empty) => None,
            Err(mpsc::TryRecvError::Disconnected) => {
                Some(Err("the depth estimate stopped unexpectedly".into()))
            }
        }
    }
}

/// Runs the add-on on the picture `png`, through two files in the temporary folder.
fn run(installed: &Installed, png: &[u8], bitmap: &Bitmap) -> Result<Vec<f32>, String> {
    let stem = format!(
        "cutcraft-depth-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    )
    .replace(['(', ')'], "");
    let dir = std::env::temp_dir();
    let (photo, out) = (
        dir.join(format!("{stem}-photo.png")),
        dir.join(format!("{stem}-depth.png")),
    );
    std::fs::write(&photo, png).map_err(|e| format!("cannot write {}: {e}", photo.display()))?;
    let mut command = std::process::Command::new(&installed.program);
    command.arg(&installed.weights).arg(&photo).arg(&out);
    #[cfg(windows)]
    {
        // no console window flashing up
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    let finished = command.output();
    let _ = std::fs::remove_file(&photo);
    let finished = finished.map_err(|e| {
        format!(
            "cannot run the Depth add-on ({}): {e}",
            installed.program.display()
        )
    })?;
    let answer = std::fs::read(&out);
    let _ = std::fs::remove_file(&out);
    if !finished.status.success() {
        let said = String::from_utf8_lossy(&finished.stderr);
        return Err(format!("the Depth add-on failed: {}", said.trim()));
    }
    read_depth(
        &answer.map_err(|e| format!("the Depth add-on gave no answer: {e}"))?,
        bitmap,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::geometry::{Point, Rect};

    fn picture(rgb: Vec<u8>) -> Bitmap {
        Bitmap {
            name: "p".into(),
            columns: 2,
            rows: 1,
            grey: vec![10, 200],
            rect: Rect::new(Point::ZERO, Point::new(2.0, 1.0)),
            depth: Vec::new(),
            rgb,
        }
    }

    #[test]
    fn the_add_on_is_found_only_when_program_and_weights_are_both_there() {
        let addons = std::env::temp_dir().join(format!("cutcraft-addons-{}", std::process::id()));
        let dir = addons.join(ADDON_ID);
        std::fs::create_dir_all(&dir).unwrap();
        assert_eq!(find(&addons), None, "empty");
        std::fs::write(dir.join(program_name()), b"").unwrap();
        assert_eq!(find(&addons), None, "no weights yet");
        std::fs::write(dir.join(WEIGHTS), b"").unwrap();
        assert_eq!(find(&addons).unwrap().weights, dir.join(WEIGHTS));
        std::fs::remove_dir_all(addons).unwrap();
    }

    #[test]
    fn the_photo_goes_in_colour_when_it_has_colour() {
        let read = |png: Vec<u8>| {
            image::load_from_memory(&png)
                .unwrap()
                .into_rgb8()
                .into_raw()
        };
        let grey = picture(Vec::new());
        assert_eq!(read(photo_png(&grey).unwrap()), [10, 10, 10, 200, 200, 200]);
        let colour = picture(vec![1, 2, 3, 4, 5, 6]);
        assert_eq!(read(photo_png(&colour).unwrap()), [1, 2, 3, 4, 5, 6]);
    }

    #[test]
    fn the_answer_becomes_depth_at_the_picture_size() {
        let bitmap = picture(Vec::new());
        let encode = |image: image::ImageBuffer<image::Luma<u16>, Vec<u16>>| {
            let mut png = Vec::new();
            image
                .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
                .unwrap();
            png
        };
        let exact = image::ImageBuffer::from_raw(2, 1, vec![0_u16, 65_535]).unwrap();
        assert_eq!(read_depth(&encode(exact), &bitmap).unwrap(), [0.0, 1.0]);
        let bigger = image::ImageBuffer::from_raw(4, 2, vec![0_u16; 8]).unwrap();
        assert_eq!(read_depth(&encode(bigger), &bitmap).unwrap().len(), 2);
        assert!(read_depth(b"not a picture", &bitmap).is_err());
    }
}
