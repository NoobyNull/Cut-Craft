//! `cut-craft --help-pictures DIR [TOOL…]`: the help pages' step pictures, taken by the app of
//! its own window, so they always show the buttons and forms as they are.
//!
//! Each tool with a [`Tour`] gets a folder `DIR/<tool id>/` holding `step-N.png` (one picture a
//! step, the thing to press or fill ringed in orange and numbered) and `tour.gif` (the steps
//! one after another, looping), and the Markdown for its page is printed. The steps' words go
//! in the tool's help page over each picture (`crates/help/pages/tools/<tool id>.md`), never
//! into the pictures, so search finds them.
//! What to ring is found by name ([`crate::marks`]), so the rings land right in any layout.

use super::help_tours::TOURS;
use super::{CutCraftApp, View};
use cut_craft_core::geometry::Point;
use eframe::egui;
use std::fmt::Write as _;
use std::path::PathBuf;

/// How long a step is left before it is captured, seconds: forms open and settle.
const HOLD_SECONDS: f64 = 0.8;
/// How long a step in the 3D view is left: the preview cuts and the view settles.
const HOLD_3D_SECONDS: f64 = 4.0;
/// The part of the window a step picture shows, points: centred on what is ringed.
const CROP: [f32; 2] = [1120.0, 700.0];
/// A step picture's width as saved, pixels: the crop made smaller.
const STEP_WIDTH: u32 = 800;
/// Colours in a saved step picture: enough for the app's flat colours and its wood.
const STEP_COLOURS: usize = 64;
/// The GIF's width, pixels; its frames are the step pictures made smaller.
const GIF_WIDTH: u32 = 600;
/// How long the GIF shows each step, hundredths of a second.
const GIF_STEP_CS: u16 = 260;
/// The ring's colour: the icons' orange accent.
const RING: egui::Color32 = egui::Color32::from_rgb(242, 120, 32);

/// What a step rings.
#[derive(Clone, Copy, Debug)]
pub enum Mark {
    /// Nothing: the step shows the whole window.
    None,
    /// A place noted by name: `tool:<id>`, `field:<key>`, `button:<words>`.
    At(&'static str),
    /// The shape drawn through this point of the job (mm from its corner).
    Shape(f64, f64),
    /// Every shape in the job.
    Everything,
}

/// One step: its words, how the app is set up for it, and what to ring.
pub struct Step {
    pub caption: &'static str,
    pub setup: fn(&mut CutCraftApp, &egui::Context),
    pub mark: Mark,
}

/// A tool's steps, each set up on top of the one before.
pub struct Tour {
    pub tool: &'static str,
    pub steps: &'static [Step],
}

/// The tours to take: all of them, or those of the tools named.
pub fn tours(only: &[String]) -> Vec<&'static Tour> {
    TOURS
        .iter()
        .filter(|t| only.is_empty() || only.iter().any(|o| o == t.tool))
        .collect()
}

/// Where the run is.
pub struct HelpShots {
    dir: PathBuf,
    tours: Vec<&'static Tour>,
    tour: usize,
    step: usize,
    since: Option<f64>,
    asked: bool,
    shot: cut_craft_bug_report::Shot,
    /// The tour's step pictures so far, for its GIF.
    frames: Vec<image::RgbaImage>,
}

impl HelpShots {
    pub fn new(ctx: &egui::Context, dir: PathBuf, only: &[String]) -> Self {
        crate::marks::switch_on(ctx);
        // dialogs (Help, the Tool Database, Save Toolpaths) drawn inside the main window, so
        // its picture shows them
        ctx.set_embed_viewports(true);
        Self {
            dir,
            tours: tours(only),
            tour: 0,
            step: 0,
            since: None,
            asked: false,
            shot: cut_craft_bug_report::Shot::default(),
            frames: vec![],
        }
    }

    /// Once a frame, after drawing: sets up each step, rings it, waits, captures and saves it;
    /// after a tour's last step, its GIF; after the last tour, quits.
    pub fn step(&mut self, app: &mut CutCraftApp, ctx: &egui::Context) {
        ctx.request_repaint();
        let now = ctx.input(|i| i.time);
        let Some(tour) = self.tours.get(self.tour).copied() else {
            std::process::exit(0);
        };
        let Some(step) = tour.steps.get(self.step) else {
            self.write_gif(tour.tool);
            print_steps(tour);
            (self.tour, self.step) = (self.tour + 1, 0);
            return;
        };
        let Some(since) = self.since else {
            (step.setup)(app, ctx);
            self.since = Some(now);
            return;
        };
        // a calculation or a 3D preview still going: wait for it
        if app.busy.is_some() || (app.view == View::ThreeD && now - since < HOLD_3D_SECONDS) {
            return;
        }
        let place = where_is(app, ctx, step.mark);
        if let Some(rect) = place {
            ring(ctx, rect, self.step + 1);
        }
        if !self.asked {
            if now - since >= HOLD_SECONDS {
                cut_craft_bug_report::take_screenshot(ctx, &self.shot);
                self.asked = true;
            }
            return;
        }
        let Some(image) = cut_craft_bug_report::taken(&self.shot) else {
            return;
        };
        let picture = crop(&image, place, ctx.pixels_per_point());
        let folder = self.dir.join(tour.tool);
        let path = folder.join(format!("step-{}.png", self.step + 1));
        let saved = std::fs::create_dir_all(&folder)
            .and_then(|()| small_png(&picture).map_err(std::io::Error::other))
            .and_then(|png| std::fs::write(&path, png));
        match saved {
            Ok(()) => println!("{}", path.display()),
            Err(e) => eprintln!("{}: {e}", path.display()),
        }
        if place.is_none() && !matches!(step.mark, Mark::None) {
            log::warn!("{} step {}: nothing to ring", tour.tool, self.step + 1);
        }
        self.frames.push(picture);
        (self.step, self.since, self.asked) = (self.step + 1, None, false);
    }

    /// The tour's steps as one looping GIF, `GIF_WIDTH` wide.
    fn write_gif(&mut self, tool: &str) {
        use image::codecs::gif::{GifEncoder, Repeat};
        let path = self.dir.join(tool).join("tour.gif");
        let frames = std::mem::take(&mut self.frames);
        let written = std::fs::File::create(&path)
            .map_err(|e| e.to_string())
            .and_then(|file| {
                let mut gif = GifEncoder::new_with_speed(std::io::BufWriter::new(file), 10);
                gif.set_repeat(Repeat::Infinite)
                    .map_err(|e| e.to_string())?;
                for picture in frames {
                    let height = GIF_WIDTH * picture.height() / picture.width().max(1);
                    let small = image::imageops::resize(
                        &picture,
                        GIF_WIDTH,
                        height,
                        image::imageops::FilterType::Lanczos3,
                    );
                    let delay = image::Delay::from_numer_denom_ms(u32::from(GIF_STEP_CS) * 10, 1);
                    gif.encode_frame(image::Frame::from_parts(small, 0, 0, delay))
                        .map_err(|e| e.to_string())?;
                }
                Ok(())
            });
        match written {
            Ok(()) => println!("{}", path.display()),
            Err(e) => eprintln!("{}: {e}", path.display()),
        }
    }
}

/// The tour as Markdown for its help page, printed to paste into the page.
fn print_steps(tour: &Tour) {
    println!(
        "\n--- crates/help/pages/tools/{}.md ---\n{}",
        tour.tool,
        steps_markdown(tour)
    );
}

/// A tour's steps as Markdown: the looping GIF, then each step numbered, its picture under
/// its words.
pub fn steps_markdown(tour: &Tour) -> String {
    let tool = tour.tool;
    let mut md = format!("![The steps, one after another](pictures/{tool}/tour.gif)\n\n");
    for (n, step) in tour.steps.iter().enumerate() {
        let n = n + 1;
        let _ = write!(
            md,
            "{n}. {}\n\n   ![Step {n}](pictures/{tool}/step-{n}.png)\n\n",
            step.caption
        );
    }
    md
}

/// `picture` made [`STEP_WIDTH`] wide and saved as a PNG of [`STEP_COLOURS`] colours: about a
/// quarter the size of a full-colour one, and as sharp to read.
fn small_png(picture: &image::RgbaImage) -> Result<Vec<u8>, String> {
    let height = (STEP_WIDTH * picture.height() / picture.width().max(1)).max(1);
    let small = image::imageops::resize(
        picture,
        STEP_WIDTH,
        height,
        image::imageops::FilterType::Lanczos3,
    );
    let quant = color_quant::NeuQuant::new(10, STEP_COLOURS, small.as_raw());
    let indices: Vec<u8> = small
        .pixels()
        .map(|p| u8::try_from(quant.index_of(&p.0)).unwrap_or(0))
        .collect();
    let palette: Vec<u8> = quant
        .color_map_rgba()
        .chunks(4)
        .flat_map(|c| [c[0], c[1], c[2]])
        .collect();
    let mut out = vec![];
    let mut encoder = png::Encoder::new(&mut out, small.width(), small.height());
    encoder.set_color(png::ColorType::Indexed);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.set_palette(palette);
    encoder.set_compression(png::Compression::High);
    let mut writer = encoder.write_header().map_err(|e| e.to_string())?;
    writer
        .write_image_data(&indices)
        .map_err(|e| e.to_string())?;
    writer.finish().map_err(|e| e.to_string())?;
    Ok(out)
}

/// Where `mark` is on screen, if it can be found.
fn where_is(app: &CutCraftApp, ctx: &egui::Context, mark: Mark) -> Option<egui::Rect> {
    match mark {
        Mark::None => None,
        Mark::At(name) => crate::marks::get(ctx, name),
        Mark::Everything => {
            let view = app.view_transform?;
            let bounds = app
                .doc
                .vectors
                .values()
                .map(cut_craft_core::geometry::Path::bounds)
                .reduce(cut_craft_core::geometry::Rect::union)?;
            Some(egui::Rect::from_two_pos(
                view.to_screen(bounds.min),
                view.to_screen(bounds.max),
            ))
        }
        Mark::Shape(x, y) => {
            let at = app.doc.job.area().min + Point::new(x, y);
            let id = app.doc.pick(at, 2.0)?;
            let bounds = app.doc.vectors.get(&id)?.bounds();
            let view = app.view_transform?;
            Some(egui::Rect::from_two_pos(
                view.to_screen(bounds.min),
                view.to_screen(bounds.max),
            ))
        }
    }
}

/// An orange ring round `rect` with the step's number beside it, over everything else.
fn ring(ctx: &egui::Context, rect: egui::Rect, number: usize) {
    let layer = egui::LayerId::new(egui::Order::Tooltip, egui::Id::unique("help-picture-ring"));
    let painter = ctx.layer_painter(layer);
    let ring = rect.expand(6.0);
    painter.rect_stroke(
        ring,
        8.0,
        egui::Stroke::new(4.0, RING),
        egui::StrokeKind::Outside,
    );
    // the number at the ring's corner, kept inside the window
    let screen = ctx.content_rect().shrink(18.0);
    let badge = (ring.left_top() + egui::vec2(-4.0, -4.0)).clamp(screen.min, screen.max);
    painter.circle_filled(badge, 15.0, RING);
    painter.circle_stroke(badge, 15.0, egui::Stroke::new(2.0, egui::Color32::WHITE));
    painter.text(
        badge,
        egui::Align2::CENTER_CENTER,
        number.to_string(),
        egui::FontId::proportional(18.0),
        egui::Color32::WHITE,
    );
}

/// The [`CROP`]-sized part of the window's `image` centred on `place` (kept inside the
/// window), or the whole window when there is nothing to centre on.
fn crop(
    image: &egui::ColorImage,
    place: Option<egui::Rect>,
    pixels_per_point: f32,
) -> image::RgbaImage {
    let [w, h] = image.size;
    let rgba: Vec<u8> = image
        .pixels
        .iter()
        .flat_map(egui::Color32::to_array)
        .collect();
    let (w32, h32) = (
        u32::try_from(w).unwrap_or(u32::MAX),
        u32::try_from(h).unwrap_or(u32::MAX),
    );
    let whole = image::RgbaImage::from_raw(w32, h32, rgba).unwrap_or_default();
    let Some(place) = place else {
        // the whole window, made as wide as a crop
        let (cw, _) = crop_size(w32, h32, pixels_per_point);
        let height = cw * h32 / w32.max(1);
        return image::imageops::resize(&whole, cw, height, image::imageops::FilterType::Lanczos3);
    };
    let (cw, ch) = crop_size(w32, h32, pixels_per_point);
    let centre = place.center();
    let (x, y) = crop_origin(
        (centre.x * pixels_per_point, centre.y * pixels_per_point),
        (cw, ch),
        (w32, h32),
    );
    image::imageops::crop_imm(&whole, x, y, cw, ch).to_image()
}

/// [`CROP`] in pixels, never more than the window.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn crop_size(w: u32, h: u32, pixels_per_point: f32) -> (u32, u32) {
    let cw = ((CROP[0] * pixels_per_point) as u32).min(w);
    let ch = ((CROP[1] * pixels_per_point) as u32).min(h);
    (cw, ch)
}

/// The top-left corner of a `size` crop centred on `centre`, slid to stay inside `window`.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)]
fn crop_origin(centre: (f32, f32), size: (u32, u32), window: (u32, u32)) -> (u32, u32) {
    let along = |c: f32, s: u32, w: u32| {
        let start = (c - s as f32 / 2.0).max(0.0) as u32;
        start.min(w.saturating_sub(s))
    };
    (
        along(centre.0, size.0, window.0),
        along(centre.1, size.1, window.1),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_crop_centres_on_the_ring_and_stays_in_the_window() {
        assert_eq!(
            crop_origin((800.0, 500.0), (1000, 625), (1600, 1000)),
            (300, 187)
        );
        assert_eq!(crop_origin((10.0, 10.0), (1000, 625), (1600, 1000)), (0, 0));
        assert_eq!(
            crop_origin((1590.0, 990.0), (1000, 625), (1600, 1000)),
            (600, 375)
        );
        assert_eq!(
            crop_size(800, 500, 1.0),
            (800, 500),
            "never past the window"
        );
    }

    #[test]
    fn every_tour_is_a_tool_with_a_help_page_and_words_on_every_step() {
        for tour in TOURS {
            assert!(
                cut_craft_help::page(tour.tool).is_some(),
                "{}: no help page",
                tour.tool
            );
            for (n, step) in tour.steps.iter().enumerate() {
                let words = step.caption.split_whitespace().count();
                assert!(words >= 5, "{} step {}: say more", tour.tool, n + 1);
            }
        }
    }

    #[test]
    fn a_tours_markdown_numbers_each_step_over_its_picture() {
        let md = steps_markdown(&TOURS[0]);
        let tool = TOURS[0].tool;
        assert!(md.starts_with(&format!(
            "![The steps, one after another](pictures/{tool}/tour.gif)"
        )));
        assert!(
            md.contains(&format!("1. {}", TOURS[0].steps[0].caption)),
            "{md}"
        );
        assert!(
            md.contains(&format!("   ![Step 2](pictures/{tool}/step-2.png)")),
            "{md}"
        );
    }

    #[test]
    fn a_saved_step_is_small_and_reads_back_at_its_width() {
        // a window-like picture: flat panels, a gradient and a ring
        let picture = image::RgbaImage::from_fn(1120, 700, |x, y| {
            let shade = u8::try_from((x + y) % 256).unwrap_or(0);
            if (x / 40 + y / 40) % 7 == 0 {
                image::Rgba([242, 120, 32, 255])
            } else if x > 800 {
                image::Rgba([248, 248, 248, 255])
            } else {
                image::Rgba([shade, shade, 200, 255])
            }
        });
        let png = small_png(&picture).expect("saved");
        let back = image::load_from_memory(&png).expect("a PNG");
        assert_eq!((back.width(), back.height()), (STEP_WIDTH, 500));
        let mut full = std::io::Cursor::new(vec![]);
        picture
            .write_to(&mut full, image::ImageFormat::Png)
            .expect("full colour");
        assert!(
            png.len() * 2 < full.get_ref().len(),
            "{} vs {}",
            png.len(),
            full.get_ref().len()
        );
    }

    #[test]
    fn only_the_tours_asked_for_are_taken() {
        assert_eq!(tours(&[]).len(), TOURS.len());
        let one = tours(&["pocket_toolpath".to_owned()]);
        assert_eq!(one.len(), 1);
        assert_eq!(one[0].tool, "pocket_toolpath");
    }
}
