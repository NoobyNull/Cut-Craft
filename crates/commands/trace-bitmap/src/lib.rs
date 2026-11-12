//! Trace Bitmap: closed vectors round the dark (or light)
//! regions of the chosen bitmap, fitted with arcs, for machining a picture's shapes. In
//! colour mode a colour picture is cut down to a few colours and the colours asked for are
//! traced, each on a layer of its own named after it.

pub mod centerline;
pub mod colours;

use colours::{Grouping, Swatches, grouped};
use cut_craft_core::bitmap::Bitmap;
use cut_craft_core::document::Document;
use cut_craft_core::geometry::{Path, Point, Rect};
use cut_craft_core::trace::{self, Fit, Grid};
use cut_craft_core::{Change, Command, CommandError, Meta};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct TraceBitmap;

/// Which pixels are traced (in black and white mode).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TraceColor {
    #[default]
    Black,
    White,
}

/// How the picture is read (black and white, or by colour).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    #[default]
    BlackAndWhite,
    /// A colour picture cut down to a few colours
    Colors,
}

/// How a shape is traced: round its outline, or down its middle.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Method {
    /// Closed shapes round each dark area, for cutting out, pocketing or carving
    #[default]
    Outline,
    /// One line down the middle of each stroke, for engraving or V-carving a drawing,
    /// signature or lettering along its lines
    Centerline,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    /// Round each shape's outline, or one line down the middle of each stroke
    #[serde(default)]
    #[schemars(title = "Trace type", extend("x-widget" = "row"))]
    pub method: Method,
    #[serde(default)]
    #[schemars(extend("x-widget" = "row"))]
    pub mode: Mode,
    /// Colours closer than this count as one: 0 keeps every colour apart, higher joins more
    /// of them
    #[serde(default = "twelve")]
    #[schemars(
        title = "Color similarity",
        range(min = 0, max = 60),
        extend("x-show-when" = {"mode": "colors"}, "x-widget" = "slider")
    )]
    pub similarity: u8,
    /// The colours traced, each named by a swatch in it (`#rrggbb`); none listed: every
    /// colour but the background's. Ticked in the trace window's colour list, or clicked on
    /// the picture.
    #[schemars(skip)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trace: Option<Vec<String>>,
    /// Swatches merged by hand, each set traced as one colour.
    #[schemars(skip)]
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub merged: Vec<Vec<String>>,
    /// Grey levels darker than this count as black (0 black … 255 white)
    #[schemars(range(min = 1, max = 255), extend("x-hide-when" = {"mode": "colors"}))]
    pub threshold: u8,
    #[schemars(
        title = "Trace color",
        extend("x-widget" = "row", "x-hide-when" = {"mode": "colors"})
    )]
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

fn twelve() -> u8 {
    12
}

impl Default for Params {
    fn default() -> Self {
        Self {
            method: Method::Outline,
            mode: Mode::BlackAndWhite,
            similarity: twelve(),
            trace: None,
            merged: vec![],
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

/// What kind of picture is traced: each fills in settings that suit it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PictureType {
    TechnicalDrawing,
    LineDrawing,
    LineArt,
    Logo,
    DetailedLogo,
    Clipart,
    RoughPhoto,
    DetailedPhoto,
}

impl PictureType {
    pub const ALL: [Self; 8] = [
        Self::TechnicalDrawing,
        Self::LineDrawing,
        Self::LineArt,
        Self::Logo,
        Self::DetailedLogo,
        Self::Clipart,
        Self::RoughPhoto,
        Self::DetailedPhoto,
    ];

    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::TechnicalDrawing => "Technical drawing (centerline)",
            Self::LineDrawing => "Line drawing (centerline)",
            Self::LineArt => "Line art",
            Self::Logo => "Logo",
            Self::DetailedLogo => "Detailed logo",
            Self::Clipart => "Clipart",
            Self::RoughPhoto => "Rough photo",
            Self::DetailedPhoto => "Detailed photo",
        }
    }

    #[must_use]
    pub fn hint(self) -> &'static str {
        match self {
            Self::TechnicalDrawing => {
                "Thin, faint black lines (plans, maps, signatures) traced as one line each"
            }
            Self::LineDrawing => "Bold black lines and sketches traced as one line each",
            Self::LineArt => "Black and white drawings and sketches: thin lines kept",
            Self::Logo => "Simple logos with few colours and little detail",
            Self::DetailedLogo => "Logos with fine detail and more colours",
            Self::Clipart => "Ready-made graphics: smooth shapes, a handful of colours",
            Self::RoughPhoto => "Photos traced as a few large areas, small detail left out",
            Self::DetailedPhoto => "Sharp photos traced with more colours and detail",
        }
    }

    /// `p` with this picture type's settings; what is traced and where are kept.
    // ponytail: our own values, picked by eye on test pictures [inf]
    #[must_use]
    pub fn settings(self, p: &Params) -> Params {
        let (mode, similarity, corner_fit, noise_filter) = match self {
            Self::TechnicalDrawing => (Mode::BlackAndWhite, 12, 8, 2),
            Self::LineDrawing => (Mode::BlackAndWhite, 12, 5, 3),
            Self::LineArt => (Mode::BlackAndWhite, 12, 8, 1),
            Self::Logo => (Mode::Colors, 20, 6, 3),
            Self::DetailedLogo => (Mode::Colors, 10, 7, 2),
            Self::Clipart => (Mode::Colors, 16, 4, 3),
            Self::RoughPhoto => (Mode::Colors, 28, 2, 6),
            Self::DetailedPhoto => (Mode::Colors, 14, 3, 3),
        };
        let centerline = matches!(self, Self::TechnicalDrawing | Self::LineDrawing);
        Params {
            method: if centerline {
                Method::Centerline
            } else {
                Method::Outline
            },
            // faint lines count as ink too
            threshold: if self == Self::TechnicalDrawing {
                170
            } else {
                p.threshold
            },
            mode,
            similarity,
            corner_fit,
            noise_filter,
            trace: None,
            merged: vec![],
            ..p.clone()
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
        let traced = traced(b, None, p, 1.0)?;
        let mut c = Change::builder(doc);
        let mut made = vec![];
        if p.mode == Mode::Colors {
            let named: Vec<String> = traced
                .iter()
                .enumerate()
                .map(|(k, t)| format!("Color {} {}", k + 1, colours::hex(t.colour)))
                .collect();
            c.note(format!("Colors: {}", named.join(", ")));
            for (k, t) in traced.into_iter().enumerate() {
                let layer = doc
                    .layers
                    .iter()
                    .find(|l| l.name == named[k])
                    .map_or_else(|| c.add_layer(&named[k]), |l| l.id);
                let group =
                    (p.group_vectors && t.paths.len() > 1).then(|| doc.next_group() + k as u64);
                for path in t.paths {
                    let id = c.add(path);
                    c.move_to_layer(id, layer);
                    c.set_group(id, group);
                    made.push(id);
                }
            }
        } else {
            let paths: Vec<Path> = traced.into_iter().flat_map(|t| t.paths).collect();
            let group = (p.group_vectors && paths.len() > 1).then(|| doc.next_group());
            for path in paths {
                let id = c.add(path);
                c.set_group(id, group);
                made.push(id);
            }
        }
        c.select(made);
        Ok(c.finish())
    }
}

/// One colour's traced shapes.
#[derive(Clone, Debug)]
pub struct Traced {
    /// Black in black and white mode, else the group's colour.
    pub colour: [u8; 3],
    pub paths: Vec<Path>,
    /// Lines down the middle of strokes (Centerline), drawn as lines, not filled.
    pub strokes: bool,
}

/// What a trace makes: each colour's shapes (none left out), traced off `b` (the chosen
/// picture, or a smaller copy of it `scale` times as coarse, see [`smaller`]), with the
/// colours from `swatches` (the full picture's; `None` works them out from `b`).
///
/// # Errors
/// When the picture has no colours in colour mode, or nothing is left to trace.
pub fn traced(
    b: &Bitmap,
    swatches: Option<&Swatches>,
    p: &Params,
    scale: f64,
) -> Result<Vec<Traced>, CommandError> {
    let shapes: Vec<Traced> = if p.mode == Mode::Colors {
        let own;
        let swatches = if let Some(s) = swatches {
            s
        } else {
            own = Swatches::of(&b.rgb);
            &own
        };
        let c = Colours::of(swatches, b, p)?;
        c.chosen(p)
            .into_iter()
            .map(|g| Traced {
                colour: c.grouping.groups[g].colour,
                paths: shapes_of(b, p, scale, |i| c.labels[i] == g),
                strokes: p.method == Method::Centerline,
            })
            .filter(|t| !t.paths.is_empty())
            .collect()
    } else {
        let on = |grey: u8| match p.trace_color {
            TraceColor::Black => grey < p.threshold,
            TraceColor::White => grey >= p.threshold,
        };
        vec![Traced {
            colour: [0, 0, 0],
            paths: shapes_of(b, p, scale, |i| on(b.grey[i])),
            strokes: p.method == Method::Centerline,
        }]
        .into_iter()
        .filter(|t| !t.paths.is_empty())
        .collect()
    };
    if shapes.is_empty() {
        return Err(CommandError::Failed(if p.mode == Mode::Colors {
            "nothing to trace. Tick more colours, or lower the noise filter".into()
        } else {
            "nothing to trace. Set another threshold or trace colour".into()
        }));
    }
    Ok(shapes)
}

/// Colour mode's view of a picture: its swatches in groups, each pixel's group, and the
/// background's group.
#[derive(Clone, Debug)]
pub struct Colours {
    pub grouping: Grouping,
    /// Each pixel's group, row by row from the top.
    pub labels: Vec<usize>,
    /// The group most of the picture's edge pixels are in.
    pub background: usize,
    /// Each group's name ([`colours::Group::key`]).
    pub keys: Vec<String>,
    /// Each group's swatches as `#rrggbb`, its name first.
    pub names: Vec<Vec<String>>,
    /// Whether each group is an edge blend: in-between colours along the edges of others
    /// (anti-aliasing), nearly all of its pixels touching another colour. Unless ticked to
    /// trace, its pixels go to the colours beside them, so the colours meet without slivers.
    pub blends: Vec<bool>,
}

/// A group is an edge blend when at least this share of its pixels touch another colour…
const BLEND_EDGE: f64 = 0.85;
/// …and it covers less of the picture than this.
const BLEND_SHARE: f64 = 0.05;

impl Colours {
    /// `b`'s pixels in the groups `p` makes of `swatches`.
    ///
    /// # Errors
    /// When `b` has no colours.
    pub fn of(swatches: &Swatches, b: &Bitmap, p: &Params) -> Result<Self, CommandError> {
        if b.rgb.len() != b.columns * b.rows * 3 || swatches.list.is_empty() {
            return Err(CommandError::Failed(
                "this picture has no colours. Set the trace to black and white".into(),
            ));
        }
        let grouping = grouped(swatches, f64::from(p.similarity), &p.merged);
        let mut labels: Vec<usize> = b
            .rgb
            .as_chunks::<3>()
            .0
            .iter()
            .map(|&q| grouping.of_swatch[swatches.index(q)])
            .collect();
        let keys: Vec<String> = grouping.groups.iter().map(|g| g.key(swatches)).collect();
        let names: Vec<Vec<String>> = grouping
            .groups
            .iter()
            .map(|g| {
                g.swatches
                    .iter()
                    .map(|&k| colours::hex(swatches.list[k].colour))
                    .collect()
            })
            .collect();
        let (w, h) = (b.columns, b.rows);
        let ticked = |g: usize| {
            p.trace
                .as_ref()
                .is_some_and(|t| names[g].iter().any(|n| t.contains(n)))
        };
        let blends = edge_blends(&labels, w, h, grouping.groups.len(), ticked);
        give_blends_away(&mut labels, w, h, &blends, &grouping);
        let mut edge = vec![0usize; grouping.groups.len()];
        for r in 0..h {
            for c in 0..w {
                if r == 0 || c == 0 || r == h - 1 || c == w - 1 {
                    edge[labels[r * w + c]] += 1;
                }
            }
        }
        let background = (0..edge.len()).max_by_key(|&g| edge[g]).unwrap_or(0);
        Ok(Self {
            grouping,
            labels,
            background,
            keys,
            names,
            blends,
        })
    }

    /// The groups traced: those `p.trace` names a swatch of, or all but the background.
    #[must_use]
    pub fn chosen(&self, p: &Params) -> Vec<usize> {
        (0..self.grouping.groups.len())
            .filter(|&g| match &p.trace {
                None => g != self.background && !self.blends[g],
                Some(named) => self.names[g].iter().any(|h| named.contains(h)),
            })
            .collect()
    }

    /// The trace list with group `g` ticked (`on`) or not, starting from what is traced now.
    #[must_use]
    pub fn ticked(&self, p: &Params, g: usize, on: bool) -> Vec<String> {
        let mut now: Vec<usize> = self.chosen(p);
        now.retain(|&k| k != g);
        if on {
            now.push(g);
        }
        now.sort_unstable();
        now.into_iter().map(|k| self.keys[k].clone()).collect()
    }
}

/// The shapes `p.method` traces round (or down the middle of) the pixels `on` says.
fn shapes_of(b: &Bitmap, p: &Params, scale: f64, on: impl Fn(usize) -> bool) -> Vec<Path> {
    match p.method {
        Method::Outline => traced_paths(b, p, scale, on),
        Method::Centerline => centre_paths(b, p, scale, on),
    }
}

/// Lines down the middle of the strokes of the pixels `on` says, fitted with arcs; lines
/// shorter than the noise filter (in the full picture's pixels) left out.
#[expect(clippy::cast_precision_loss, reason = "pixel counts are small")]
fn centre_paths(b: &Bitmap, p: &Params, scale: f64, on: impl Fn(usize) -> bool) -> Vec<Path> {
    let pixel = b.rect.size() / Point::new(b.columns as f64, b.rows as f64);
    let in_area = |i: usize| {
        let (c, r) = (i % b.columns, i / b.columns);
        p.area.is_none_or(|area| {
            area.contains(Point::new(
                b.rect.min.x + (c as f64 + 0.5) * pixel.x,
                b.rect.max.y - (r as f64 + 0.5) * pixel.y,
            ))
        })
    };
    let shortest = 2.0 * f64::from(p.noise_filter) / scale.max(1.0);
    let corner = (150.0 - 12.0 * f64::from(p.corner_fit.clamp(1, 10) - 1)).to_radians();
    // ponytail: 0.6 px; the middle line steps a pixel at a time, so it is fitted looser
    // than an outline
    let tolerance = 0.6 * pixel.min_element();
    centerline::lines(b.columns, b.rows, |i| in_area(i) && on(i), shortest)
        .into_iter()
        .filter(|l| l.points.len() > 1)
        .map(|l| {
            let placed: Vec<Point> = l
                .points
                .iter()
                .map(|&(x, y)| {
                    Point::new(
                        b.rect.min.x + (x + 0.5) * pixel.x,
                        b.rect.max.y - (y + 0.5) * pixel.y,
                    )
                })
                .collect();
            trace::fit(&placed, l.closed, Fit::Arcs, tolerance, corner)
        })
        .collect()
}

/// Which of `groups` groups (`labels` holding each pixel's, `w` × `h`) are edge blends; a
/// group `ticked` to trace never is.
#[expect(clippy::cast_precision_loss, reason = "pixel counts")]
fn edge_blends(
    labels: &[usize],
    w: usize,
    h: usize,
    groups: usize,
    ticked: impl Fn(usize) -> bool,
) -> Vec<bool> {
    let mut count = vec![0usize; groups];
    let mut edge = vec![0usize; groups];
    for r in 0..h {
        for c in 0..w {
            let l = labels[r * w + c];
            count[l] += 1;
            let differs = (c > 0 && labels[r * w + c - 1] != l)
                || (c + 1 < w && labels[r * w + c + 1] != l)
                || (r > 0 && labels[(r - 1) * w + c] != l)
                || (r + 1 < h && labels[(r + 1) * w + c] != l);
            if differs {
                edge[l] += 1;
            }
        }
    }
    let total = (w * h).max(1) as f64;
    let blends: Vec<bool> = (0..groups)
        .map(|g| {
            count[g] > 0
                && !ticked(g)
                && edge[g] as f64 >= BLEND_EDGE * count[g] as f64
                && (count[g] as f64) < BLEND_SHARE * total
        })
        .collect();
    // a picture of nothing but edges keeps its colours
    if blends.iter().zip(&count).all(|(&b, &n)| b || n == 0) {
        return vec![false; groups];
    }
    blends
}

/// Each pixel of an edge blend given to the colour beside it it looks most like (or, with
/// none beside it, the nearest colour of all).
fn give_blends_away(
    labels: &mut [usize],
    w: usize,
    h: usize,
    blends: &[bool],
    grouping: &Grouping,
) {
    if !blends.contains(&true) {
        return;
    }
    let colour = |g: usize| grouping.groups[g].colour;
    let nearest = |from: usize, among: &mut dyn Iterator<Item = usize>| {
        among.filter(|&g| !blends[g]).min_by(|&a, &b| {
            colours::difference(colour(a), colour(from))
                .total_cmp(&colours::difference(colour(b), colour(from)))
        })
    };
    // ponytail: a few passes reach blends a few pixels wide; what is left goes by colour
    for _ in 0..4 {
        let before = labels.to_vec();
        let mut left = false;
        for r in 0..h {
            for c in 0..w {
                let own = before[r * w + c];
                if !blends[own] {
                    continue;
                }
                let mut beside = (r.saturating_sub(1)..(r + 2).min(h))
                    .flat_map(|y| (c.saturating_sub(1)..(c + 2).min(w)).map(move |x| y * w + x));
                let mut around = std::iter::from_fn(|| beside.next().map(|i| before[i]));
                match nearest(own, &mut around) {
                    Some(g) => labels[r * w + c] = g,
                    None => left = true,
                }
            }
        }
        if !left {
            return;
        }
    }
    for l in labels.iter_mut() {
        if blends[*l]
            && let Some(g) = nearest(*l, &mut (0..blends.len()))
        {
            *l = g;
        }
    }
}

/// Closed vectors round the pixels `on` says (by index, row by row from the top), fitted
/// with arcs, specks smaller than the noise filter (in the full picture's pixels, `scale`
/// of which make one of `b`'s) left out.
#[expect(clippy::cast_precision_loss, reason = "pixel counts are small")]
fn traced_paths(b: &Bitmap, p: &Params, scale: f64, on: impl Fn(usize) -> bool) -> Vec<Path> {
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
    let smallest = (f64::from(p.noise_filter) / scale.max(1.0)).powi(2);
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

/// A copy of `b` at most `most` pixels, every so many picked (not blended, so no colours are
/// made that the picture does not have), on the same rectangle; and how many of `b`'s pixels
/// make one of its across. `None` when `b` is small enough already.
#[must_use]
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "pixel counts"
)]
pub fn smaller(b: &Bitmap, most: usize) -> Option<(Bitmap, f64)> {
    let pixels = b.columns * b.rows;
    if pixels <= most || most == 0 {
        return None;
    }
    let scale = (pixels as f64 / most as f64).sqrt();
    let columns = ((b.columns as f64 / scale).floor() as usize).max(1);
    let rows = ((b.rows as f64 / scale).floor() as usize).max(1);
    let pick = |c: usize, r: usize| {
        let x = ((c as f64 + 0.5) * b.columns as f64 / columns as f64) as usize;
        let y = ((r as f64 + 0.5) * b.rows as f64 / rows as f64) as usize;
        y.min(b.rows - 1) * b.columns + x.min(b.columns - 1)
    };
    let coloured = b.rgb.len() == pixels * 3;
    let mut grey = Vec::with_capacity(columns * rows);
    let mut rgb = Vec::with_capacity(if coloured { columns * rows * 3 } else { 0 });
    for r in 0..rows {
        for c in 0..columns {
            let i = pick(c, r);
            grey.push(b.grey[i]);
            if coloured {
                rgb.extend_from_slice(&b.rgb[i * 3..i * 3 + 3]);
            }
        }
    }
    Some((
        Bitmap {
            name: b.name.clone(),
            columns,
            rows,
            grey,
            rect: b.rect,
            depth: Vec::new(),
            rgb,
        },
        b.columns as f64 / columns as f64,
    ))
}

/// The traced shapes drawn as a picture `width` × `height` pixels (RGBA, row by row from
/// the top) of `rect`: white, each colour's shapes filled in turn, a shape inside another
/// of the same colour a hole.
#[must_use]
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "pixel positions"
)]
pub fn drawn(shapes: &[Traced], rect: Rect, width: usize, height: usize) -> Vec<u8> {
    let mut out = vec![255u8; width * height * 4];
    if width == 0 || height == 0 {
        return out;
    }
    let size = rect.size();
    let (sx, sy) = (width as f64 / size.x, height as f64 / size.y);
    let tolerance = 0.5 / sx.max(sy);
    for t in shapes {
        let rings: Vec<Vec<Point>> = t
            .paths
            .iter()
            .map(|p| {
                p.flatten(tolerance)
                    .into_iter()
                    .map(|q| Point::new((q.x - rect.min.x) * sx, (rect.max.y - q.y) * sy))
                    .collect()
            })
            .collect();
        if t.strokes {
            for (path, line) in t.paths.iter().zip(&rings) {
                stroke(&mut out, width, height, line, path.closed, t.colour);
            }
            continue;
        }
        let mut crossings = vec![];
        for row in 0..height {
            let y = row as f64 + 0.5;
            crossings.clear();
            for ring in &rings {
                for (i, a) in ring.iter().enumerate() {
                    let b = ring[(i + 1) % ring.len()];
                    if (a.y <= y) != (b.y <= y) {
                        crossings.push(a.x + (y - a.y) / (b.y - a.y) * (b.x - a.x));
                    }
                }
            }
            crossings.sort_by(f64::total_cmp);
            for pair in crossings.as_chunks::<2>().0 {
                let from = (pair[0] - 0.5).ceil().max(0.0) as usize;
                let to = ((pair[1] - 0.5).floor() + 1.0).clamp(0.0, width as f64) as usize;
                for col in from..to {
                    let at = (row * width + col) * 4;
                    out[at..at + 3].copy_from_slice(&t.colour);
                }
            }
        }
    }
    out
}

/// `line` (in pixels) drawn into `out` (RGBA, `width` × `height`) in `colour`, a 400th of
/// the picture wide (two pixels at least), so it reads when the picture is shown smaller.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "pixel positions"
)]
fn stroke(
    out: &mut [u8],
    width: usize,
    height: usize,
    line: &[Point],
    closed: bool,
    colour: [u8; 3],
) {
    let half = (width.max(height) as f64 / 800.0).max(1.0);
    let mut dot = |p: Point| {
        let (x0, x1) = ((p.x - half).floor().max(0.0), (p.x + half).ceil());
        let (y0, y1) = ((p.y - half).floor().max(0.0), (p.y + half).ceil());
        for y in y0 as usize..(y1 as usize).min(height) {
            for x in x0 as usize..(x1 as usize).min(width) {
                let at = (y * width + x) * 4;
                out[at..at + 3].copy_from_slice(&colour);
            }
        }
    };
    let ends = if closed {
        line.len()
    } else {
        line.len().saturating_sub(1)
    };
    for i in 0..ends {
        let (a, b) = (line[i], line[(i + 1) % line.len()]);
        let steps = (a.distance(b) * 2.0).ceil().max(1.0) as usize;
        for k in 0..=steps {
            dot(a + (b - a) * (k as f64 / steps as f64));
        }
    }
    if line.len() == 1 {
        dot(line[0]);
    }
}

/// The grey level halfway between a picture's dark and light (Otsu's method: the level
/// that best splits its grey levels in two), for tracing a picture without settings.
#[must_use]
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    reason = "grey levels and pixel counts"
)]
pub fn auto_threshold(grey: &[u8]) -> u8 {
    let mut counts = [0usize; 256];
    for &g in grey {
        counts[usize::from(g)] += 1;
    }
    let total = grey.len() as f64;
    let sum: f64 = (0..256).map(|g| g as f64 * counts[g] as f64).sum();
    let (mut below, mut below_sum, mut best, mut at) = (0.0, 0.0, -1.0, 128usize);
    for (g, &n) in counts.iter().enumerate() {
        below += n as f64;
        below_sum += g as f64 * n as f64;
        let above = total - below;
        if below == 0.0 || above == 0.0 {
            continue;
        }
        let (m1, m2) = (below_sum / below, (sum - below_sum) / above);
        let between = below * above * (m1 - m2).powi(2);
        if between > best {
            (best, at) = (between, g);
        }
    }
    // levels darker than the threshold are black: the split's dark side up to `at`
    (at + 1).clamp(1, 255) as u8
}

/// Settings that suit `b` with no one choosing them (Quick Trace): its colours when it has
/// some, else black and white split at [`auto_threshold`].
#[must_use]
pub fn quick_settings(b: &Bitmap) -> Params {
    let coloured = b.rgb.len() == b.columns * b.rows * 3
        && b.rgb.as_chunks::<3>().0.iter().any(|&[r, g, bl]| {
            let (lo, hi) = (r.min(g).min(bl), r.max(g).max(bl));
            hi - lo > 24
        });
    if coloured {
        PictureType::Clipart.settings(&Params::default())
    } else {
        Params {
            threshold: auto_threshold(&b.grey),
            ..Params::default()
        }
    }
}

/// The group (by number in the colours' grouping) colour mode puts the chosen picture's
/// pixel at `at` in, for picking colours by clicking them.
#[must_use]
pub fn group_at(b: &Bitmap, colours: &Colours, at: Point) -> Option<usize> {
    if !b.rect.contains(at) || b.columns == 0 || b.rows == 0 {
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
    colours.labels.get(r * b.columns + c).copied()
}

/// The trace list with the colour under `at` on the chosen picture ticked if it was not
/// traced, or let go if it was; `None` off the picture or out of colour mode.
#[must_use]
pub fn toggled_at(doc: &Document, p: &Params, at: Point) -> Option<Vec<String>> {
    let b = doc.chosen_bitmap()?;
    if p.mode != Mode::Colors {
        return None;
    }
    let c = Colours::of(&Swatches::of(&b.rgb), b, p).ok()?;
    let g = group_at(b, &c, at)?;
    let on = !c.chosen(p).contains(&g);
    Some(c.ticked(p, g, on))
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
        // only the blue asked for
        let one = TraceBitmap::run(
            &doc,
            &Params {
                trace: Some(vec!["#1414c8".into()]),
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
        assert!(err.contains("Nothing to trace"), "{err}");
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
            ..Params::default()
        };
        // red and blue are traced to start with, the white round the edge is not
        let red_off = toggled_at(&doc, &p, Point::new(1.0, 2.0)).unwrap();
        assert_eq!(red_off, vec!["#1414c8".to_string()]);
        let p = Params {
            trace: Some(red_off),
            ..p
        };
        let red_on = toggled_at(&doc, &p, Point::new(1.0, 2.0)).unwrap();
        assert_eq!(red_on.len(), 2);
        let white_on = toggled_at(&doc, &p, Point::new(1.0, 3.5)).unwrap();
        assert!(white_on.contains(&"#fafafa".to_string()));
        assert_eq!(
            toggled_at(&doc, &p, Point::new(9.0, 2.0)),
            None,
            "off the picture"
        );
    }

    /// 60 x 40 pixels on 60 x 40 mm: light grey round the edge, a white disc that is not
    /// the background, a near-black square and a dark grey one just apart from it.
    fn coloured() -> Bitmap {
        let (columns, rows) = (60u32, 40u32);
        let mut rgb = vec![];
        for row in 0..rows {
            for col in 0..columns {
                let (x, y) = (f64::from(col) + 0.5, f64::from(row) + 0.5);
                rgb.extend(if (x - 15.0).hypot(y - 20.0) < 8.0 {
                    [255, 255, 255]
                } else if (30..40).contains(&col) && (15..25).contains(&row) {
                    [20, 20, 20]
                } else if (44..54).contains(&col) && (15..25).contains(&row) {
                    [26, 24, 22]
                } else {
                    [190, 190, 190]
                });
            }
        }
        Bitmap {
            name: "coloured".into(),
            columns: columns as usize,
            rows: rows as usize,
            grey: rgb.chunks(3).map(|p| p[0]).collect(),
            rect: Rect::new(Point::ZERO, Point::new(60.0, 40.0)),
            depth: Vec::new(),
            rgb,
        }
    }

    #[test]
    fn the_background_is_the_colour_round_the_edges_not_the_lightest() {
        let b = coloured();
        let p = Params {
            mode: Mode::Colors,
            similarity: 0,
            ..Params::default()
        };
        let c = Colours::of(&Swatches::of(&b.rgb), &b, &p).unwrap();
        assert_eq!(c.grouping.groups.len(), 4);
        assert_eq!(c.keys[c.background], "#bebebe");
        let shapes = traced(&b, None, &p, 1.0).unwrap();
        assert_eq!(
            shapes.len(),
            3,
            "the white disc is traced, the grey round it not"
        );
        // the two dark squares look alike: one colour at the default similarity
        let alike = traced(
            &b,
            None,
            &Params {
                similarity: 12,
                ..p.clone()
            },
            1.0,
        )
        .unwrap();
        assert_eq!(alike.len(), 2);
        assert_eq!(alike[0].paths.len(), 2, "both squares in the dark colour");
        // merged by hand at similarity 0: the same
        let merged = traced(
            &b,
            None,
            &Params {
                merged: vec![vec!["#141414".into(), "#1a1816".into()]],
                ..p
            },
            1.0,
        )
        .unwrap();
        assert_eq!(merged.len(), 2);
    }

    #[test]
    fn edge_blends_go_to_the_colours_beside_them_unless_ticked() {
        // white, a red square with a ring of pink one pixel wide round it (anti-aliasing)
        let n = 60usize;
        let mut rgb = vec![];
        for r in 0..n {
            for c in 0..n {
                let inside = |lo: usize, hi: usize| (lo..hi).contains(&r) && (lo..hi).contains(&c);
                rgb.extend(if inside(20, 40) {
                    [200, 20, 20]
                } else if inside(19, 41) {
                    [228, 140, 140]
                } else {
                    [250, 250, 250]
                });
            }
        }
        let b = Bitmap {
            name: "ring".into(),
            columns: n,
            rows: n,
            grey: vec![128; n * n],
            rect: Rect::new(Point::ZERO, Point::new(60.0, 60.0)),
            depth: Vec::new(),
            rgb,
        };
        let p = Params {
            mode: Mode::Colors,
            ..Params::default()
        };
        let c = Colours::of(&Swatches::of(&b.rgb), &b, &p).unwrap();
        assert_eq!(c.grouping.groups.len(), 3);
        assert_eq!(c.blends.iter().filter(|&&x| x).count(), 1, "the pink");
        let pink = c.keys.iter().position(|k| k == "#e48c8c").unwrap();
        assert!(c.blends[pink]);
        assert!(!c.labels.contains(&pink), "its pixels went to red or white");
        let shapes = traced(&b, None, &p, 1.0).unwrap();
        assert_eq!(shapes.len(), 1, "only the red: no pink sliver");
        assert_eq!(shapes[0].paths.len(), 1);
        // ticked, the pink is a colour of its own
        let kept = Params {
            trace: Some(vec!["#e48c8c".into()]),
            ..p
        };
        let c = Colours::of(&Swatches::of(&b.rgb), &b, &kept).unwrap();
        assert!(!c.blends.contains(&true));
        assert_eq!(traced(&b, None, &kept, 1.0).unwrap().len(), 1);
    }

    #[test]
    fn a_big_picture_is_previewed_from_a_smaller_copy_on_the_same_rectangle() {
        let b = coloured();
        assert!(smaller(&b, 10_000).is_none(), "small enough already");
        let (small, scale) = smaller(&b, 600).unwrap();
        assert_eq!((small.columns, small.rows), (30, 20));
        assert!((scale - 2.0).abs() < 1e-9);
        assert_eq!(small.rect, b.rect);
        assert_eq!(small.rgb.len(), 30 * 20 * 3);
        // only colours the picture has
        let full = Swatches::of(&b.rgb);
        let p = Params {
            mode: Mode::Colors,
            similarity: 0,
            ..Params::default()
        };
        let quick = traced(&small, Some(&full), &p, scale).unwrap();
        let whole = traced(&b, Some(&full), &p, 1.0).unwrap();
        assert_eq!(quick.len(), whole.len());
        let span = |t: &[Traced]| t[0].paths[0].bounds().size().x;
        assert!(
            (span(&quick) - span(&whole)).abs() < 2.5,
            "the same size in the job"
        );
    }

    #[test]
    fn the_result_is_drawn_filled_with_holes_left_white() {
        // a 10 mm square with a 4 mm square hole
        let outside = Path::polyline(
            &[
                Point::new(0.0, 0.0),
                Point::new(10.0, 0.0),
                Point::new(10.0, 10.0),
                Point::new(0.0, 10.0),
            ],
            true,
        );
        let hole = Path::polyline(
            &[
                Point::new(3.0, 3.0),
                Point::new(3.0, 7.0),
                Point::new(7.0, 7.0),
                Point::new(7.0, 3.0),
            ],
            true,
        );
        let shapes = [Traced {
            colour: [200, 20, 20],
            paths: vec![outside, hole],
            strokes: false,
        }];
        let picture = drawn(
            &shapes,
            Rect::new(Point::ZERO, Point::new(10.0, 10.0)),
            10,
            10,
        );
        let at = |c: usize, r: usize| &picture[(r * 10 + c) * 4..(r * 10 + c) * 4 + 3];
        assert_eq!(at(1, 1), [200, 20, 20]);
        assert_eq!(at(5, 5), [255, 255, 255], "the hole");
        assert_eq!(at(9, 0), [200, 20, 20]);
    }

    #[test]
    #[expect(clippy::cast_precision_loss, reason = "small test picture")]
    fn centerline_traces_one_open_line_down_each_stroke() {
        // 80 x 40 pixels on 80 x 40 mm: a bar 5 px thick, and a ring 4 px thick
        let (w, h) = (80usize, 40usize);
        let grey: Vec<u8> = (0..w * h)
            .map(|i| {
                let (x, y) = ((i % w) as f64 + 0.5, (i / w) as f64 + 0.5);
                let bar = (5.0..35.0).contains(&x) && (18.0..23.0).contains(&y);
                let ring = (10.0..14.0).contains(&(x - 58.0).hypot(y - 20.0));
                if bar || ring { 0 } else { 255 }
            })
            .collect();
        let mut doc = Document::default();
        let mut c = Change::builder(&doc);
        c.add_bitmap(Bitmap {
            name: "strokes".into(),
            columns: w,
            rows: h,
            grey,
            rect: Rect::new(Point::ZERO, Point::new(80.0, 40.0)),
            depth: Vec::new(),
            rgb: Vec::new(),
        });
        doc.apply(&c.finish()).unwrap();
        run_on::<TraceBitmap>(
            &mut doc,
            &Params {
                method: Method::Centerline,
                ..Params::default()
            },
        );
        assert_eq!(doc.vectors.len(), 2, "one line per stroke");
        let bar = doc
            .vectors
            .values()
            .find(|p| !p.closed)
            .expect("the bar is open");
        let b = bar.bounds();
        // down the bar's middle (y = 40 - 20.5) along most of its length
        assert!((b.center().y - 19.5).abs() < 1.0, "{b:?}");
        assert!(b.size().x > 20.0, "{b:?}");
        let ring = doc
            .vectors
            .values()
            .find(|p| p.closed)
            .expect("the ring is closed");
        assert!(
            (ring.bounds().size().x - 24.0).abs() < 2.0,
            "{:?}",
            ring.bounds()
        );
    }

    #[test]
    fn lines_are_drawn_as_lines_not_filled() {
        let line = Path::polyline(&[Point::new(1.0, 5.0), Point::new(9.0, 5.0)], false);
        let shapes = [Traced {
            colour: [0, 0, 0],
            paths: vec![line],
            strokes: true,
        }];
        let picture = drawn(
            &shapes,
            Rect::new(Point::ZERO, Point::new(10.0, 10.0)),
            10,
            10,
        );
        let at = |c: usize, r: usize| picture[(r * 10 + c) * 4];
        assert_eq!(at(5, 5), 0, "on the line");
        assert_eq!(at(5, 1), 255, "off it");
    }

    #[test]
    fn quick_settings_split_a_grey_picture_and_keep_colours() {
        // a dark half (40) and a light half (200)
        let grey: Vec<u8> = (0..100).map(|i| if i < 50 { 40 } else { 200 }).collect();
        let t = auto_threshold(&grey);
        assert!((41..=200).contains(&t), "{t}");
        let grey_picture = Bitmap {
            name: "g".into(),
            columns: 10,
            rows: 10,
            grey: grey.clone(),
            rect: Rect::new(Point::ZERO, Point::new(10.0, 10.0)),
            depth: Vec::new(),
            // grey stored as colour: still grey
            rgb: grey.iter().flat_map(|&g| [g, g, g]).collect(),
        };
        let p = quick_settings(&grey_picture);
        assert_eq!((p.mode, p.threshold), (Mode::BlackAndWhite, t));
        let mut coloured = grey_picture;
        coloured.rgb[0..3].copy_from_slice(&[200, 20, 20]);
        assert_eq!(quick_settings(&coloured).mode, Mode::Colors);
    }

    #[test]
    fn picture_types_fill_in_settings_and_keep_the_area() {
        let area = Some(Rect::new(Point::ZERO, Point::new(5.0, 5.0)));
        let p = Params {
            area,
            trace: Some(vec!["#000000".into()]),
            ..Params::default()
        };
        let drawing = PictureType::LineDrawing.settings(&p);
        assert_eq!(drawing.method, Method::Centerline);
        let line_art = PictureType::LineArt.settings(&p);
        assert_eq!(line_art.method, Method::Outline);
        assert_eq!(line_art.mode, Mode::BlackAndWhite);
        assert_eq!(line_art.area, area);
        assert_eq!(line_art.trace, None, "the colours are picked again");
        let photo = PictureType::RoughPhoto.settings(&p);
        assert_eq!(photo.mode, Mode::Colors);
        assert!(photo.similarity > PictureType::DetailedPhoto.settings(&p).similarity);
        assert!(PictureType::ALL.iter().all(|t| !t.hint().is_empty()));
    }
}
