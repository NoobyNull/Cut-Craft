//! Import a bitmap picture: kept in grey levels, placed at the job's lower left at 96 pixels
//! per inch, shrunk to fit the job if it is bigger. A 16-bit or floating point picture (a
//! depth map) keeps its tones at full precision too, for reliefs made from it.

use cut_craft_core::bitmap::Bitmap;
use cut_craft_core::document::Document;
use cut_craft_core::geometry::{Point, Rect};
use cut_craft_core::{Change, CommandError, FileFormat, Importer};

const MM_PER_PX: f64 = 25.4 / 96.0;

pub struct ImportBitmap;

impl Importer for ImportBitmap {
    const ID: &'static str = "import_bitmap";
    const FORMAT: FileFormat = FileFormat {
        label: "Bitmap",
        extensions: &["png", "jpg", "jpeg", "gif", "bmp", "tif", "tiff"],
    };

    fn import(doc: &Document, bytes: &[u8]) -> Result<Change, CommandError> {
        let (columns, rows, grey, depth) = match float_tiff(bytes) {
            Some((columns, rows, heights)) => {
                let tones = stretched(heights);
                (columns, rows, grey_of(&tones), tones)
            }
            None => decoded(bytes)?,
        };
        #[expect(clippy::cast_precision_loss, reason = "pixel counts")]
        let natural = Point::new(columns as f64, rows as f64) * MM_PER_PX;
        let area = doc.job.area();
        let room = area.max - area.min;
        let scale = (room.x / natural.x).min(room.y / natural.y).min(1.0);
        let mut c = Change::builder(doc);
        c.add_bitmap(Bitmap {
            name: "Bitmap".into(),
            columns,
            rows,
            grey,
            rect: Rect::new(area.min, area.min + natural * scale),
            depth,
        });
        Ok(c.finish())
    }
}

type Pixels = (usize, usize, Vec<u8>, Vec<f32>);

/// Any picture the image library reads: its grey levels, and for a 16-bit or floating
/// point one its tones at full precision (0 black to 1 white).
fn decoded(bytes: &[u8]) -> Result<Pixels, CommandError> {
    use image::ColorType::{L8, La8, Rgb8, Rgb32F, Rgba8, Rgba32F};
    let picture = image::load_from_memory(bytes)
        .map_err(|e| CommandError::Failed(format!("not a readable picture: {e}")))?;
    let (columns, rows) = (picture.width() as usize, picture.height() as usize);
    match picture.color() {
        L8 | La8 | Rgb8 | Rgba8 => Ok((columns, rows, picture.into_luma8().into_raw(), vec![])),
        colour => {
            let tones = picture.to_luma32f().into_raw();
            // floating point values may be in any units; 16-bit ones are already 0 to 1
            let tones = if matches!(colour, Rgb32F | Rgba32F) {
                stretched(tones)
            } else {
                tones
            };
            Ok((columns, rows, grey_of(&tones), tones))
        }
    }
}

/// A floating point TIFF, read with the TIFF library: the image library reads none with a
/// single channel, which is how depth generators write them. `None` for any other file.
fn float_tiff(bytes: &[u8]) -> Option<(usize, usize, Vec<f32>)> {
    use tiff::decoder::{Decoder, DecodingResult};
    if !(bytes.starts_with(b"II*\0") || bytes.starts_with(b"MM\0*")) {
        return None;
    }
    let mut tiff = Decoder::new(std::io::Cursor::new(bytes)).ok()?;
    let (w, h) = tiff.dimensions().ok()?;
    let (columns, rows) = (usize::try_from(w).ok()?, usize::try_from(h).ok()?);
    #[expect(
        clippy::cast_possible_truncation,
        reason = "heights to f32, as depth maps are kept"
    )]
    let values: Vec<f32> = match tiff.read_image().ok()? {
        DecodingResult::F32(v) => v,
        DecodingResult::F64(v) => v.into_iter().map(|x| x as f32).collect(),
        _ => return None,
    };
    let pixels = columns * rows;
    let channels = values.len().checked_div(pixels).filter(|&c| c > 0)?;
    // the first channel: a depth map's grey, or red of a colour one
    let heights = values.chunks(channels).map(|p| p[0]).take(pixels).collect();
    Some((columns, rows, heights))
}

/// Heights in any units as tones, the lowest 0 and the highest 1; what is not a number is
/// the lowest.
fn stretched(heights: Vec<f32>) -> Vec<f32> {
    let finite = heights.iter().copied().filter(|h| h.is_finite());
    let (lo, hi) = finite.fold((f32::MAX, f32::MIN), |(lo, hi), h| (lo.min(h), hi.max(h)));
    let span = hi - lo;
    heights
        .into_iter()
        .map(|h| {
            if h.is_finite() && span > 0.0 {
                (h - lo) / span
            } else {
                0.0
            }
        })
        .collect()
}

/// Tones as the grey levels the picture is shown in.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "clamped to 0..=255"
)]
fn grey_of(tones: &[f32]) -> Vec<u8> {
    tones
        .iter()
        .map(|t| (t.clamp(0.0, 1.0) * 255.0).round() as u8)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn png(width: u32, height: u32) -> Vec<u8> {
        let picture = image::GrayImage::from_fn(width, height, |x, _| {
            image::Luma([if x == 0 { 0 } else { 255 }])
        });
        let mut bytes = std::io::Cursor::new(vec![]);
        picture
            .write_to(&mut bytes, image::ImageFormat::Png)
            .unwrap();
        bytes.into_inner()
    }

    #[test]
    fn a_picture_is_placed_at_96_pixels_per_inch_in_grey() {
        let mut doc = Document::default();
        let change = ImportBitmap::import(&doc, &png(96, 48)).unwrap();
        doc.apply(&change).unwrap();
        let b = doc.bitmaps.values().next().unwrap();
        assert_eq!((b.columns, b.rows), (96, 48));
        let size = b.rect.max - b.rect.min;
        assert!((size.x - 25.4).abs() < 1e-9 && (size.y - 12.7).abs() < 1e-9);
        assert_eq!(
            b.darkness(b.rect.min + Point::new(0.1, 1.0)),
            Some(1.0),
            "the black first column"
        );
    }

    #[test]
    fn a_picture_bigger_than_the_job_is_shrunk_to_fit() {
        let mut doc = Document::default();
        let change = ImportBitmap::import(&doc, &png(9600, 10)).unwrap();
        doc.apply(&change).unwrap();
        let b = doc.bitmaps.values().next().unwrap();
        assert!((b.rect.max.x - b.rect.min.x - doc.job.width).abs() < 1e-9);
    }

    fn imported(bytes: &[u8]) -> cut_craft_core::bitmap::Bitmap {
        let mut doc = Document::default();
        doc.apply(&ImportBitmap::import(&doc, bytes).unwrap())
            .unwrap();
        doc.bitmaps.values().next().unwrap().clone()
    }

    #[test]
    fn a_16_bit_depth_map_keeps_its_full_precision() {
        // neighbouring 16-bit levels, which 8 bits would make one grey
        let levels = [0_u16, 32_768, 32_769, 65_535];
        let picture = image::ImageBuffer::<image::Luma<u16>, _>::from_fn(4, 1, |x, _| {
            image::Luma([levels[x as usize]])
        });
        let mut bytes = std::io::Cursor::new(vec![]);
        picture
            .write_to(&mut bytes, image::ImageFormat::Png)
            .unwrap();
        let b = imported(bytes.get_ref());
        assert_eq!(b.depth.len(), 4, "kept at full precision");
        assert!(b.depth[2] > b.depth[1], "{:?}", b.depth);
        assert_eq!(b.grey[1], b.grey[2], "one grey level on screen");
        assert_eq!((b.depth[0], b.depth[3]), (0.0, 1.0));
        // an 8-bit picture needs no depth beside its grey
        assert_eq!(imported(&png(4, 1)).depth, Vec::<f32>::new());
    }

    #[test]
    fn a_floating_point_tiff_is_scaled_from_its_lowest_to_its_highest() {
        // a single-channel 32-bit float TIFF, as depth generators write, in its own units
        let heights = [10.0_f32, 60.0, 110.0, 60.000_5];
        let mut bytes = std::io::Cursor::new(vec![]);
        tiff::encoder::TiffEncoder::new(&mut bytes)
            .unwrap()
            .write_image::<tiff::encoder::colortype::Gray32Float>(4, 1, &heights)
            .unwrap();
        let b = imported(bytes.get_ref());
        assert_eq!(b.depth.len(), 4);
        assert!(
            (b.depth[0] - 0.0).abs() < 1e-6 && (b.depth[2] - 1.0).abs() < 1e-6,
            "{:?}",
            b.depth
        );
        assert!((b.depth[1] - 0.5).abs() < 1e-6);
        assert!(b.depth[3] > b.depth[1], "finer than a grey level");
    }

    #[test]
    fn rubbish_is_refused() {
        let why = ImportBitmap::import(&Document::default(), b"not a picture").unwrap_err();
        assert!(why.to_string().contains("not a readable picture"));
    }

    #[test]
    fn undo_removes_it() {
        let doc = Document::default();
        let change = ImportBitmap::import(&doc, &png(4, 4)).unwrap();
        let mut after = doc.clone();
        after.apply(&change).unwrap();
        after.apply(&change.inverse()).unwrap();
        assert_eq!(after, doc);
    }
}
