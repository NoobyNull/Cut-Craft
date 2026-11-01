//! Trace Bitmap (Aspire's Fit Vectors to Bitmap): closed vectors round the dark (or light)
//! regions of the chosen bitmap, fitted with arcs, for machining a picture's shapes. In
//! colour mode a colour picture is cut down to a few colours and the colours asked for are
//! traced, each on a layer of its own named after it.

use cut_craft_core::bitmap::Bitmap;
use cut_craft_core::document::Document;
use cut_craft_core::geometry::{Path, Point, Rect};
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

/// How the picture is read (Aspire's Black and White or Colors).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    #[default]
    BlackAndWhite,
    /// A colour picture cut down to a few colours
    Colors,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    #[serde(default)]
    #[schemars(extend("x-widget" = "row"))]
    pub mode: Mode,
    /// How many colours the picture is cut down to
    #[serde(default = "four")]
    #[schemars(
        title = "Number of colors",
        range(min = 2, max = 16),
        extend("x-show-when" = {"mode": "colors"})
    )]
    pub colors: u8,
    /// The colours to trace, by number from the darkest (1); none: all but the lightest
    #[serde(default)]
    #[schemars(
        title = "Colors to trace",
        extend("x-show-when" = {"mode": "colors"})
    )]
    pub trace_colors: Vec<u8>,
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
    /// Only this part of the picture is traced: drag a box over it in the 2D view, click
    /// the picture to trace all of it again
    #[schemars(skip)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub area: Option<Rect>,
}

fn four() -> u8 {
    4
}

impl Default for Params {
    fn default() -> Self {
        Self {
            mode: Mode::BlackAndWhite,
            colors: four(),
            trace_colors: vec![],
            threshold: 128,
            trace_color: TraceColor::Black,
            // ponytail: Aspire's dialog shows 2 on both sliders before a bitmap is chosen [inf]
            corner_fit: 2,
            noise_filter: 2,
            group_vectors: true,
            area: None,
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
            icon: "drawing/create_vectors/trace_bitmap.png",
            shortcut: "",
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        let Some(b) = doc.chosen_bitmap() else {
            return Err(CommandError::Failed("import a bitmap to trace".into()));
        };
        if p.mode == Mode::Colors {
            return by_colour(doc, b, p);
        }
        let traced = |grey: u8| match p.trace_color {
            TraceColor::Black => grey < p.threshold,
            TraceColor::White => grey >= p.threshold,
        };
        let paths = traced_paths(b, p, |i| traced(b.grey[i]));
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

/// Closed vectors round the pixels `on` says (by index, row by row from the top), fitted
/// with arcs, specks smaller than the noise filter left out.
#[expect(clippy::cast_precision_loss, reason = "pixel counts are small")]
fn traced_paths(b: &Bitmap, p: &Params, on: impl Fn(usize) -> bool) -> Vec<Path> {
    let grid = Grid {
        columns: b.columns,
        rows: b.rows,
        origin: Point::ZERO,
        cell: 1.0,
    };
    let pixel = b.rect.size() / Point::new(b.columns as f64, b.rows as f64);
    // the grid counts rows from the bottom, the bitmap from the top
    let in_area = |c: usize, r: usize| {
        p.area.is_none_or(|area| {
            area.contains(b.rect.min + Point::new(c as f64 + 0.5, r as f64 + 0.5) * pixel)
        })
    };
    let rings = trace::outlines(grid, |c, r| {
        in_area(c, r) && on((b.rows - 1 - r) * b.columns + c)
    });
    let smallest = f64::from(p.noise_filter).powi(2);
    // ponytail: corner angle and tolerance from Corner Fit are our own scale [inf]
    let corner = (150.0 - 12.0 * f64::from(p.corner_fit.clamp(1, 10) - 1)).to_radians();
    // ponytail: 0.3 px; half a pixel cuts round shapes short against the pixel steps
    let tolerance = 0.3 * pixel.min_element();
    rings
        .iter()
        .filter(|ring| trace::ring_area(ring).abs() >= smallest)
        .map(|ring| {
            let placed: Vec<Point> = ring.iter().map(|q| b.rect.min + *q * pixel).collect();
            trace::fit(&placed, true, Fit::Arcs, tolerance, corner)
        })
        .collect()
}

/// Colour mode: the picture cut down to `p.colors` colours, numbered from the darkest, and
/// each colour asked for traced onto a layer named after it.
fn by_colour(doc: &Document, b: &Bitmap, p: &Params) -> Result<Change, CommandError> {
    if b.rgb.len() != b.columns * b.rows * 3 {
        return Err(CommandError::Failed(
            "this picture has no colours: trace it in black and white".into(),
        ));
    }
    let (palette, labels) = palette(&b.rgb, usize::from(p.colors.clamp(2, 16)));
    let wanted: Vec<usize> = if p.trace_colors.is_empty() {
        (0..palette.len() - 1).collect()
    } else {
        p.trace_colors
            .iter()
            .filter_map(|&n| usize::from(n).checked_sub(1))
            .filter(|&k| k < palette.len())
            .collect()
    };
    let mut c = Change::builder(doc);
    let named: Vec<String> = palette
        .iter()
        .enumerate()
        .map(|(k, [r, g, b])| format!("Color {} #{r:02x}{g:02x}{b:02x}", k + 1))
        .collect();
    c.note(format!("Colors: {}", named.join(", ")));
    let mut made = vec![];
    for k in wanted {
        let paths = traced_paths(b, p, |i| usize::from(labels[i]) == k);
        if paths.is_empty() {
            continue;
        }
        let layer = doc
            .layers
            .iter()
            .find(|l| l.name == named[k])
            .map_or_else(|| c.add_layer(&named[k]), |l| l.id);
        let group = (p.group_vectors && paths.len() > 1).then(|| doc.next_group() + k as u64);
        for path in paths {
            let id = c.add(path);
            c.move_to_layer(id, layer);
            c.set_group(id, group);
            made.push(id);
        }
    }
    if made.is_empty() {
        return Err(CommandError::Failed(
            "nothing to trace: try other colours or more of them".into(),
        ));
    }
    c.select(made);
    Ok(c.finish())
}

/// The number (from the darkest, 1) of the colour colour mode cuts the chosen picture's
/// pixel at `at` down to, for picking the colours to trace by clicking them.
#[must_use]
pub fn colour_at(doc: &Document, p: &Params, at: Point) -> Option<u8> {
    let b = doc.chosen_bitmap()?;
    if b.rgb.len() != b.columns * b.rows * 3 || !b.rect.contains(at) {
        return None;
    }
    let size = b.rect.size();
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss,
        reason = "pixel counts"
    )]
    let (c, r) = (
        (((at.x - b.rect.min.x) / size.x * b.columns as f64) as usize).min(b.columns - 1),
        (((b.rect.max.y - at.y) / size.y * b.rows as f64) as usize).min(b.rows - 1),
    );
    let (_, labels) = palette(&b.rgb, usize::from(p.colors.clamp(2, 16)));
    labels.get(r * b.columns + c).map(|k| k + 1)
}

/// The colours to trace with colour `n` (from 1) clicked: taken out if it was there, else
/// added (none listed means all but the lightest, so a click starts a list of its own).
#[must_use]
pub fn toggled(colours: &[u8], n: u8) -> Vec<u8> {
    if colours.contains(&n) {
        colours.iter().copied().filter(|&k| k != n).collect()
    } else {
        let mut out = colours.to_vec();
        out.push(n);
        out.sort_unstable();
        out
    }
}

/// `rgb` (three bytes a pixel) cut down to `count` colours by k-means, numbered from the
/// darkest: the colours, and each pixel's number.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "colour channels and small counts"
)]
fn palette(rgb: &[u8], count: usize) -> (Vec<[u8; 3]>, Vec<u8>) {
    // ponytail: plain k-means, 12 rounds, on at most 20 000 pixels; median cut if it is slow
    const ROUNDS: usize = 12;
    const SAMPLES: usize = 20_000;
    let pixels: Vec<[f64; 3]> = rgb
        .chunks(3)
        .map(|p| [f64::from(p[0]), f64::from(p[1]), f64::from(p[2])])
        .collect();
    let step = (pixels.len() / SAMPLES).max(1);
    let sample: Vec<[f64; 3]> = pixels.iter().step_by(step).copied().collect();
    let light = |c: &[f64; 3]| 0.299 * c[0] + 0.587 * c[1] + 0.114 * c[2];
    // start from the darkest pixel, then each time the pixel farthest from those chosen
    let gap = |a: &[f64; 3], b: &[f64; 3]| (0..3).map(|i| (a[i] - b[i]).powi(2)).sum::<f64>();
    let darkest = sample
        .iter()
        .min_by(|a, b| light(a).total_cmp(&light(b)))
        .copied()
        .unwrap_or_default();
    let mut centres = vec![darkest];
    while centres.len() < count {
        let far = sample.iter().max_by(|a, b| {
            let d = |p: &[f64; 3]| {
                centres
                    .iter()
                    .map(|c| gap(c, p))
                    .fold(f64::INFINITY, f64::min)
            };
            d(a).total_cmp(&d(b))
        });
        match far {
            Some(p) if centres.iter().all(|c| gap(c, p) > 0.0) => centres.push(*p),
            _ => break,
        }
    }
    let count = centres.len();
    let nearest = |centres: &[[f64; 3]], p: &[f64; 3]| {
        let d = |c: &[f64; 3]| (0..3).map(|i| (c[i] - p[i]).powi(2)).sum::<f64>();
        (0..centres.len())
            .min_by(|&a, &b| d(&centres[a]).total_cmp(&d(&centres[b])))
            .unwrap_or(0)
    };
    for _ in 0..ROUNDS {
        let mut sums = vec![([0.0; 3], 0usize); count];
        for p in &sample {
            let k = nearest(&centres, p);
            for (sum, v) in sums[k].0.iter_mut().zip(p) {
                *sum += v;
            }
            sums[k].1 += 1;
        }
        for (centre, (sum, n)) in centres.iter_mut().zip(&sums) {
            if *n > 0 {
                *centre = sum.map(|v| v / *n as f64);
            }
        }
    }
    centres.sort_by(|a, b| light(a).total_cmp(&light(b)));
    centres.dedup_by(|a, b| (0..3).all(|i| (a[i] - b[i]).abs() < 0.5));
    let labels = pixels.iter().map(|p| nearest(&centres, p) as u8).collect();
    let palette = centres.iter().map(|c| c.map(|v| v.round() as u8)).collect();
    (palette, labels)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::bitmap::Bitmap;
    use cut_craft_core::polygon;
    use cut_craft_core::testing::{rejection, run_on};

    #[test]
    fn colours_are_traced_each_onto_a_layer_of_its_own() {
        // white, with a red square on the left and a blue square on the right
        let (columns, rows) = (40, 20);
        let mut rgb = vec![];
        for row in 0..rows {
            for col in 0..columns {
                let inside = (5..15).contains(&row);
                rgb.extend(match col {
                    5..15 if inside => [200, 20, 20],
                    25..35 if inside => [20, 20, 200],
                    _ => [250, 250, 250],
                });
            }
        }
        let grey = rgb
            .chunks(3)
            .map(|p| {
                u8::try_from((u16::from(p[0]) + u16::from(p[1]) + u16::from(p[2])) / 3).unwrap()
            })
            .collect();
        let mut doc = Document::default();
        let mut c = Change::builder(&doc);
        c.add_bitmap(Bitmap {
            name: "colours".into(),
            columns,
            rows,
            grey,
            rect: Rect::new(Point::ZERO, Point::new(40.0, 20.0)),
            depth: Vec::new(),
            rgb,
        });
        doc.apply(&c.finish()).unwrap();
        let p = Params {
            mode: Mode::Colors,
            colors: 3,
            ..Params::default()
        };
        let change = TraceBitmap::run(&doc, &p).unwrap();
        assert!(
            change.notes[0].starts_with("Colors: Color 1 #"),
            "{:?}",
            change.notes
        );
        doc.apply(&change).unwrap();
        // the two darker colours, not the white: two new layers, one square on each
        let colour_layers: Vec<_> = doc
            .layers
            .iter()
            .filter(|l| l.name.starts_with("Color "))
            .collect();
        assert_eq!(
            colour_layers.len(),
            2,
            "{:?}",
            doc.layers.iter().map(|l| &l.name).collect::<Vec<_>>()
        );
        for l in colour_layers {
            assert_eq!(doc.vectors_on(l.id).count(), 1, "{}", l.name);
        }
        // only the second colour asked for
        let one = TraceBitmap::run(
            &doc,
            &Params {
                trace_colors: vec![2],
                ..p.clone()
            },
        )
        .unwrap();
        let added = one
            .edits
            .iter()
            .filter(|e| matches!(e, cut_craft_core::Edit::Add { .. }))
            .count();
        assert_eq!(added, 1);
        // a grey picture has no colours to trace
        let mut grey_doc = doc.clone();
        grey_doc.bitmaps.values_mut().for_each(|b| b.rgb.clear());
        assert!(TraceBitmap::run(&grey_doc, &p).is_err());
    }

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
            depth: Vec::new(),
            rgb: Vec::new(),
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

    #[test]
    fn a_box_dragged_over_the_picture_traces_only_that_part() {
        let mut doc = picture();
        let right = Params {
            area: Some(Rect::new(Point::new(40.0, 0.0), Point::new(80.0, 40.0))),
            ..Params::default()
        };
        run_on::<TraceBitmap>(&mut doc, &right);
        assert_eq!(doc.vectors.len(), 1, "the square, not the disc");
        let square = doc.vectors.values().next().unwrap().bounds();
        assert!(square.min.x > 40.0, "{square:?}");
    }

    #[test]
    fn clicking_a_colour_picks_it_and_clicking_again_lets_it_go() {
        // white, a red half-height band on the left, a blue one on the right
        let (columns, rows) = (8, 4);
        let mut rgb = vec![];
        for row in 0..rows {
            for col in 0..columns {
                rgb.extend(match (row, col) {
                    (1..3, 0..4) => [200, 20, 20],
                    (1..3, _) => [20, 20, 200],
                    _ => [250, 250, 250],
                });
            }
        }
        let mut doc = Document::default();
        let mut c = Change::builder(&doc);
        c.add_bitmap(Bitmap {
            name: "colours".into(),
            columns,
            rows,
            grey: vec![128; columns * rows],
            rect: Rect::new(Point::ZERO, Point::new(8.0, 4.0)),
            depth: Vec::new(),
            rgb,
        });
        doc.apply(&c.finish()).unwrap();
        let p = Params {
            mode: Mode::Colors,
            colors: 3,
            ..Params::default()
        };
        let (red, blue) = (
            colour_at(&doc, &p, Point::new(1.0, 2.0)).unwrap(),
            colour_at(&doc, &p, Point::new(7.0, 2.0)).unwrap(),
        );
        let white = colour_at(&doc, &p, Point::new(1.0, 3.5)).unwrap();
        assert_ne!(red, blue);
        assert_eq!(white, 3, "the lightest is the last");
        assert_eq!(
            colour_at(&doc, &p, Point::new(9.0, 2.0)),
            None,
            "off the picture"
        );
        assert_eq!(toggled(&[], red), vec![red]);
        assert_eq!(toggled(&[blue, red], red), vec![blue]);
    }
}
