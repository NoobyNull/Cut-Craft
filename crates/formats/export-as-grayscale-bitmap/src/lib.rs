//! Export as Grayscale Bitmap: the model as a PNG height map, its highest point white and
//! lowest black, one pixel per model cell.

use cut_craft_core::component::model_at_resolution;
use cut_craft_core::document::Document;
use cut_craft_core::{CommandError, Exporter, FileFormat};

pub struct ExportGrayscaleBitmap;

impl Exporter for ExportGrayscaleBitmap {
    const ID: &'static str = "export_as_grayscale_bitmap";
    const FORMAT: FileFormat = FileFormat {
        label: "Grayscale PNG",
        extensions: &["png"],
    };

    fn export(doc: &Document) -> Result<Vec<u8>, CommandError> {
        if doc.components.is_empty() {
            return Err(CommandError::Failed(
                "there is no 3D model to export".into(),
            ));
        }
        let m = model_at_resolution(doc);
        let (lo, hi) = m
            .heights
            .iter()
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), &h| {
                (lo.min(h), hi.max(h))
            });
        let range = (hi - lo).max(f64::EPSILON);
        let too_big = |_| CommandError::Failed("the model is too big for a bitmap".into());
        let (width, height) = (
            u32::try_from(m.columns).map_err(too_big)?,
            u32::try_from(m.rows).map_err(too_big)?,
        );
        // picture rows run from the top; the model's from the bottom
        let picture = image::GrayImage::from_fn(width, height, |x, y| {
            let row = m.rows - 1 - y as usize;
            let h = m.heights[row * m.columns + x as usize];
            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "0 to 255"
            )]
            let grey = ((h - lo) / range * 255.0).round() as u8;
            image::Luma([grey])
        });
        let mut bytes = std::io::Cursor::new(vec![]);
        picture
            .write_to(&mut bytes, image::ImageFormat::Png)
            .map_err(|e| CommandError::Failed(format!("could not write the PNG: {e}")))?;
        Ok(bytes.into_inner())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_dome_is_white_in_the_middle_and_black_at_the_edge() {
        let doc = cut_craft_core::testing::domed();
        let png = ExportGrayscaleBitmap::export(&doc).unwrap();
        let picture = image::load_from_memory(&png).unwrap().into_luma8();
        let (w, h) = picture.dimensions();
        assert_eq!(picture.get_pixel(w / 2, h / 2).0[0], 255);
        assert_eq!(picture.get_pixel(0, 0).0[0], 0);
    }

    #[test]
    fn no_model_is_refused() {
        assert!(ExportGrayscaleBitmap::export(&Document::default()).is_err());
    }
}
