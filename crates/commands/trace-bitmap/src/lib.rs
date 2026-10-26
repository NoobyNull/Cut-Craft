//! Trace Bitmap (Aspire's Fit Vectors to Bitmap): closed vectors round the dark (or light)
//! regions of the chosen bitmap, fitted with arcs, for machining a picture's shapes.

use cut_craft_core::document::Document;
use cut_craft_core::geometry::Point;
use cut_craft_core::trace::{self, Fit, Grid};
use cut_craft_core::{Change, Command, CommandError, Meta};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct TraceBitmap;

/// Which pixels are traced (Aspire's Trace Color, in black and white mode).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TraceColor {
    #[default]
    Black,
    White,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    /// Grey levels darker than this count as black (0 black … 255 white)
    #[schemars(range(min = 1, max = 255))]
    pub threshold: u8,
    #[schemars(title = "Trace color", extend("x-widget" = "row"))]
    pub trace_color: TraceColor,
    /// Loose (1) rounds more corners off; tight (10) keeps more of them sharp
    #[schemars(range(min = 1, max = 10))]
    pub corner_fit: u8,
    /// Regions smaller than this many pixels across are left out
    #[schemars(extend("x-unit" = "px"), range(min = 1, max = 10))]
    pub noise_filter: u8,
    /// Make the traced vectors one group
    pub group_vectors: bool,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            threshold: 128,
            trace_color: TraceColor::Black,
            // ponytail: Aspire's dialog shows 2 on both sliders before a bitmap is chosen [inf]
            corner_fit: 2,
            noise_filter: 2,
            group_vectors: true,
        }
    }
}

impl Command for TraceBitmap {
    const ID: &'static str = "trace_bitmap";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Trace Bitmap",
            group: "Create Vectors",
            icon: "drawing/create_vectors/trace_bitmap.svg",
            shortcut: "",
        }
    }

    #[expect(clippy::cast_precision_loss, reason = "pixel counts are small")]
    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        let Some(b) = doc.chosen_bitmap() else {
            return Err(CommandError::Failed("import a bitmap to trace".into()));
        };
        let traced = |grey: u8| match p.trace_color {
            TraceColor::Black => grey < p.threshold,
            TraceColor::White => grey >= p.threshold,
        };
        let grid = Grid {
            columns: b.columns,
            rows: b.rows,
            origin: Point::ZERO,
            cell: 1.0,
        };
        // the grid counts rows from the bottom, the bitmap from the top
        let rings = trace::outlines(grid, |c, r| {
            traced(b.grey[(b.rows - 1 - r) * b.columns + c])
        });
        let pixel = b.rect.size() / Point::new(b.columns as f64, b.rows as f64);
        let smallest = f64::from(p.noise_filter).powi(2);
        // ponytail: corner angle and tolerance from Corner Fit are our own scale [inf]
        let corner = (150.0 - 12.0 * f64::from(p.corner_fit.clamp(1, 10) - 1)).to_radians();
        // ponytail: 0.3 px; half a pixel cuts round shapes short against the pixel steps
        let tolerance = 0.3 * pixel.min_element();
        let paths: Vec<_> = rings
            .iter()
            .filter(|ring| trace::ring_area(ring).abs() >= smallest)
            .map(|ring| {
                let placed: Vec<Point> = ring.iter().map(|q| b.rect.min + *q * pixel).collect();
                trace::fit(&placed, true, Fit::Arcs, tolerance, corner)
            })
            .collect();
        if paths.is_empty() {
            return Err(CommandError::Failed(
                "nothing to trace: try another threshold or trace color".into(),
            ));
        }
        let group = (p.group_vectors && paths.len() > 1).then(|| doc.next_group());
        let mut c = Change::builder(doc);
        let added: Vec<_> = paths.into_iter().map(|path| c.add(path)).collect();
        for &id in &added {
            c.set_group(id, group);
        }
        c.select(added);
        Ok(c.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::bitmap::Bitmap;
    use cut_craft_core::geometry::Rect;
    use cut_craft_core::polygon;
    use cut_craft_core::testing::{rejection, run_on};

    /// A 40 x 20 pixel picture on a 80 x 40 mm rectangle: a black disc of radius 6 px at
    /// (10, 10), a black 10 x 10 square at (24..34, 5..15), and one stray black pixel.
    fn picture() -> Document {
        let (columns, rows) = (40, 20);
        let grey = (0..rows)
            .flat_map(|row| (0..columns).map(move |col| (col, row)))
            .map(|(col, row)| {
                let (x, y) = (f64::from(col) + 0.5, f64::from(row) + 0.5);
                let disc = (x - 10.0).hypot(y - 10.0) < 6.0;
                let square = (24..34).contains(&col) && (5..15).contains(&row);
                let speck = (col, row) == (38, 1);
                if disc || square || speck { 0 } else { 255 }
            })
            .collect();
        let mut doc = Document::default();
        let mut c = Change::builder(&doc);
        c.add_bitmap(Bitmap {
            name: "shapes".into(),
            columns: 40,
            rows: 20,
            grey,
            rect: Rect::new(Point::new(0.0, 0.0), Point::new(80.0, 40.0)),
        });
        let change = c.finish();
        doc.apply(&change).unwrap();
        doc
    }

    cut_craft_core::contract_tests!(TraceBitmap, picture);

    #[test]
    fn dark_shapes_become_grouped_vectors_and_specks_are_filtered() {
        let mut doc = picture();
        run_on::<TraceBitmap>(&mut doc, &Params::default());
        assert_eq!(doc.vectors.len(), 2, "disc and square; the speck is noise");
        let first = *doc.vectors.keys().next().unwrap();
        assert_eq!(doc.group_members(first).len(), 2);
        let mut areas: Vec<f64> = doc
            .vectors
            .values()
            .map(|p| polygon::area(p).abs())
            .collect();
        areas.sort_by(f64::total_cmp);
        // pixels are 2 mm: the disc is about pi * 12², the square 20 x 20 mm
        assert!((areas[0] - 400.0).abs() < 20.0, "square {}", areas[0]);
        assert!(
            (areas[1] - std::f64::consts::PI * 144.0).abs() < 40.0,
            "disc {}",
            areas[1]
        );
        // the top of the picture is the top of the job: the disc's centre lands at y = 20 mm
        let disc = doc
            .vectors
            .values()
            .find(|p| p.bounds().min.x < 10.0)
            .unwrap();
        assert!((disc.bounds().center() - Point::new(20.0, 20.0)).length() < 1.0);
    }

    #[test]
    fn noise_filter_one_keeps_the_speck_and_white_traces_the_background() {
        let mut doc = picture();
        run_on::<TraceBitmap>(
            &mut doc,
            &Params {
                noise_filter: 1,
                ..Params::default()
            },
        );
        assert_eq!(doc.vectors.len(), 3);
        let mut white = picture();
        run_on::<TraceBitmap>(
            &mut white,
            &Params {
                trace_color: TraceColor::White,
                ..Params::default()
            },
        );
        let biggest = white
            .vectors
            .values()
            .map(|p| p.bounds().size().x)
            .fold(0.0, f64::max);
        // within the trace's tolerance, 0.3 px of 2 mm: its corners are under the corner
        // angle, so they are rounded
        assert!(
            (biggest - 80.0).abs() < 0.6,
            "the background spans the picture: {biggest}"
        );
    }

    #[test]
    fn needs_a_bitmap_and_something_to_trace() {
        assert!(rejection::<TraceBitmap>(&Params::default()).contains("bitmap"));
        let mut blank = picture();
        for b in blank.bitmaps.values_mut() {
            b.grey.fill(255);
        }
        let err = TraceBitmap::run(&blank, &Params::default())
            .unwrap_err()
            .to_string();
        assert!(err.contains("nothing to trace"), "{err}");
    }
}
