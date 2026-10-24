//! Import a bitmap picture: kept in grey levels, placed at the job's lower left at 96 pixels
//! per inch, shrunk to fit the job if it is bigger.

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
        let picture = image::load_from_memory(bytes)
            .map_err(|e| CommandError::Failed(format!("not a readable picture: {e}")))?
            .into_luma8();
        let (columns, rows) = (picture.width() as usize, picture.height() as usize);
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
            grey: picture.into_raw(),
            rect: Rect::new(area.min, area.min + natural * scale),
        });
        Ok(c.finish())
    }
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
