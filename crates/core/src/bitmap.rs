//! Bitmaps: imported pictures placed on the job, in grey levels, for strategies that carve
//! or burn an image (Photo V-Carve, Sketch Carving, Laser Picture).

use crate::geometry::{Point, Rect};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Bitmap {
    pub name: String,
    pub columns: usize,
    pub rows: usize,
    /// Grey level of each pixel row by row from the top: 0 black, 255 white.
    // ponytail: stored as a JSON number array; a packed encoding if job files get large
    pub grey: Vec<u8>,
    /// Where it lies on the job, in millimetres.
    pub rect: Rect,
}

impl Bitmap {
    /// How dark the picture is at `p` (0 white to 1 black, blended between pixels), or
    /// `None` outside it.
    pub fn darkness(&self, p: Point) -> Option<f64> {
        if !self.rect.contains(p) || self.columns == 0 || self.rows == 0 {
            return None;
        }
        let size = self.rect.max - self.rect.min;
        #[expect(clippy::cast_precision_loss, reason = "pixel counts")]
        let (cols, rows) = (self.columns as f64, self.rows as f64);
        // pixel centres, from the top row down
        let x = ((p.x - self.rect.min.x) / size.x * cols - 0.5).clamp(0.0, cols - 1.0);
        let y = ((self.rect.max.y - p.y) / size.y * rows - 0.5).clamp(0.0, rows - 1.0);
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "clamped"
        )]
        let (left, up) = (x.floor() as usize, y.floor() as usize);
        let (right, down) = (
            (left + 1).min(self.columns - 1),
            (up + 1).min(self.rows - 1),
        );
        let (across, below) = (x - x.floor(), y - y.floor());
        let grey = |col: usize, row: usize| f64::from(self.grey[row * self.columns + col]);
        let top = grey(left, up) * (1.0 - across) + grey(right, up) * across;
        let bottom = grey(left, down) * (1.0 - across) + grey(right, down) * across;
        Some(1.0 - (top * (1.0 - below) + bottom * below) / 255.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn darkness_reads_the_pixels_top_row_first_and_blends_between() {
        // 2 x 1: black then white, over a 20 x 10 mm rectangle
        let b = Bitmap {
            name: "two".into(),
            columns: 2,
            rows: 1,
            grey: vec![0, 255],
            rect: Rect::new(Point::ZERO, Point::new(20.0, 10.0)),
        };
        assert_eq!(b.darkness(Point::new(1.0, 5.0)), Some(1.0));
        assert_eq!(b.darkness(Point::new(19.0, 5.0)), Some(0.0));
        assert!((b.darkness(Point::new(10.0, 5.0)).unwrap() - 0.5).abs() < 1e-9);
        assert_eq!(b.darkness(Point::new(21.0, 5.0)), None);
    }
}
