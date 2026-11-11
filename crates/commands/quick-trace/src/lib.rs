//! Quick Trace: traces the chosen picture into vectors in one click, with settings picked to
//! suit it. A colour picture is traced by its colours, each on a layer of its own; a black
//! and white or grey one is split into dark and light at the level that best tells them
//! apart. Trace Bitmap's window shows and changes the same settings, with a preview.

use cut_craft_core::document::Document;
use cut_craft_core::{Change, Command, CommandError, Meta, NoParams};

pub struct QuickTrace;

impl Command for QuickTrace {
    const ID: &'static str = "quick_trace";
    type Params = NoParams;

    fn meta() -> Meta {
        Meta {
            label: "Quick Trace",
            group: "Create Vectors",
            icon: "drawing/create_vectors/quick_trace.png",
            shortcut: "",
        }
    }

    fn run(doc: &Document, _: &NoParams) -> Result<Change, CommandError> {
        let Some(b) = doc.chosen_bitmap() else {
            return Err(CommandError::Failed("import a bitmap to trace".into()));
        };
        let p = cmd_trace_bitmap::quick_settings(b);
        cmd_trace_bitmap::TraceBitmap::run(doc, &p)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::bitmap::Bitmap;
    use cut_craft_core::geometry::{Point, Rect};
    use cut_craft_core::testing::{rejection, run_on};

    /// A grey picture: a dark grey disc (90) on light grey (210), no black or white in it.
    fn picture() -> Document {
        let (w, h) = (40usize, 40usize);
        let grey: Vec<u8> = (0..w * h)
            .map(|i| {
                #[expect(clippy::cast_precision_loss, reason = "small")]
                let (x, y) = ((i % w) as f64 + 0.5, (i / w) as f64 + 0.5);
                if (x - 20.0).hypot(y - 20.0) < 10.0 {
                    90
                } else {
                    210
                }
            })
            .collect();
        let mut doc = Document::default();
        let mut c = Change::builder(&doc);
        c.add_bitmap(Bitmap {
            name: "disc".into(),
            columns: w,
            rows: h,
            grey,
            rect: Rect::new(Point::ZERO, Point::new(40.0, 40.0)),
            depth: Vec::new(),
            rgb: Vec::new(),
        });
        doc.apply(&c.finish()).unwrap();
        doc
    }

    cut_craft_core::contract_tests!(QuickTrace, picture);

    #[test]
    fn a_grey_picture_is_split_where_its_greys_part() {
        let mut doc = picture();
        run_on::<QuickTrace>(&mut doc, &NoParams {});
        // the disc, the dark side of the picture's own split; the light grey round it is not
        assert_eq!(doc.vectors.len(), 1);
        let disc = doc.vectors.values().next().unwrap().bounds();
        assert!((disc.size().x - 20.0).abs() < 1.5, "{disc:?}");
    }

    #[test]
    fn needs_a_picture() {
        assert!(rejection::<QuickTrace>(&NoParams {}).contains("bitmap"));
    }
}
