//! The 2D view: job coordinates (mm, y up) ↔ screen points (pixels, y down), pointer
//! gestures in job coordinates, and painting the document.

use cut_craft_core::Drag;
use cut_craft_core::document::{Document, EntityId, Job, Units};
use cut_craft_core::geometry::{Path, Point, Rect as JobRect};
use cut_craft_core::toolpath::MoveKind;
use eframe::egui::{self, PointerButton, Pos2, Rect, Sense, Shape, Stroke, Vec2, pos2, vec2};
use std::f32::consts::FRAC_PI_2;

/// Ruler labels stay at least this many rows of the theme's interactive height apart.
// ponytail: fixed spacing until a preferences dialog exists
const RULER_LABEL_ROWS: f32 = 3.0;
/// Minor ruler ticks per labelled step.
const RULER_MINOR_TICKS: f64 = 5.0;

#[derive(Clone, Copy, Debug)]
pub struct ViewTransform {
    /// Screen position of the job origin (X0, Y0).
    pub origin: Pos2,
    /// Pixels per millimetre.
    pub scale: f32,
}

impl ViewTransform {
    /// Fits a `width` × `height` job inside `area` with `margin` pixels to spare, centred.
    /// Fits the material, wherever the datum puts it, into the middle of `area`.
    pub fn fit(area: Rect, margin: f32, job: JobRect) -> Self {
        let inner = area.shrink(margin);
        let size = job.size();
        let scale = (inner.width() / to_f32(size.x)).min(inner.height() / to_f32(size.y));
        let (c, middle) = (inner.center(), job.center());
        Self {
            origin: pos2(
                c.x - to_f32(middle.x) * scale,
                c.y + to_f32(middle.y) * scale,
            ),
            scale,
        }
    }

    /// Zoomed by `factor` about the screen point `at`, which stays over the same job point.
    #[must_use]
    pub fn zoomed_about(self, at: Pos2, factor: f32) -> Self {
        Self {
            origin: at + (self.origin - at) * factor,
            scale: self.scale * factor,
        }
    }

    /// Moved on screen by `by`.
    #[must_use]
    pub fn panned(self, by: Vec2) -> Self {
        Self {
            origin: self.origin + by,
            ..self
        }
    }

    pub fn to_screen(self, p: Point) -> Pos2 {
        pos2(
            self.origin.x + to_f32(p.x) * self.scale,
            self.origin.y - to_f32(p.y) * self.scale,
        )
    }

    pub fn to_job(self, p: Pos2) -> Point {
        Point::new(
            f64::from((p.x - self.origin.x) / self.scale),
            f64::from((self.origin.y - p.y) / self.scale),
        )
    }
}

/// The spacing of ruler labels: the smallest 1, 2 or 5 times a power of ten that puts them at
/// least `min_gap` pixels apart at `pixels_per_unit`.
pub fn ruler_step(pixels_per_unit: f64, min_gap: f64) -> f64 {
    let raw = min_gap / pixels_per_unit;
    let power = 10f64.powf(raw.log10().floor());
    [1.0, 2.0, 5.0, 10.0]
        .into_iter()
        .map(|m| m * power)
        .find(|&step| step >= raw)
        .unwrap_or(10.0 * power)
}

/// The multiples of `step` from `lo` to `hi`.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    reason = "tick counts are small"
)]
pub fn ruler_ticks(lo: f64, hi: f64, step: f64) -> Vec<f64> {
    ((lo / step).ceil() as i64..=(hi / step).floor() as i64)
        .map(|k| k as f64 * step)
        .collect()
}

/// What the pointer did on the canvas this frame.
#[derive(Clone, Copy, Debug)]
pub enum Gesture {
    Click { at: Point, shift: bool },
    Dragging(Drag),
    Dropped { drag: Drag, shift: bool },
}

/// Drawn over the document while a gesture is in progress.
pub enum Overlay {
    None,
    /// What the active drawing tool would create.
    Preview(Vec<Path>),
    /// A selection box in job coordinates; crossing boxes are drawn dashed.
    Window {
        start: Point,
        end: Point,
        crossing: bool,
    },
}

pub struct Canvas {
    pub transform: ViewTransform,
    pub area: Rect,
    response: egui::Response,
    /// The material, in job coordinates.
    job: JobRect,
}

impl Canvas {
    /// The view of `job`: as `camera` has it when the user has zoomed or panned, else fitted.
    pub fn new(ui: &mut egui::Ui, job: &Job, camera: Option<ViewTransform>) -> Self {
        let (area, response) = ui.allocate_exact_size(ui.available_size(), Sense::click_and_drag());
        let transform = camera
            .unwrap_or_else(|| ViewTransform::fit(area, ui.spacing().interact_size.y, job.area()));
        Self {
            transform,
            area,
            response,
            job: job.area(),
        }
    }

    /// Where the pointer is over the view, on screen.
    /// The view's response, for its right-click menu.
    pub fn response(&self) -> &egui::Response {
        &self.response
    }

    pub fn hover(&self) -> Option<Pos2> {
        self.response.hover_pos()
    }

    /// How far a drag with `button` moved this frame.
    pub fn dragged_by(&self, button: PointerButton) -> Option<Vec2> {
        self.response
            .dragged_by(button)
            .then(|| self.response.drag_delta())
    }

    pub fn cursor(&self) -> Option<Point> {
        self.response.hover_pos().map(|p| self.transform.to_job(p))
    }

    /// Pick radius in millimetres: half the theme's interactive height, on screen.
    pub fn pick_tolerance(&self, ui: &egui::Ui) -> f64 {
        f64::from(ui.spacing().interact_size.y / 2.0 / self.transform.scale)
    }

    pub fn gesture(&self, ui: &egui::Ui) -> Option<Gesture> {
        let modifiers = ui.input(|i| i.modifiers);
        let t = self.transform;
        if self.response.clicked() {
            let at = t.to_job(self.response.interact_pointer_pos()?);
            return Some(Gesture::Click {
                at,
                shift: modifiers.shift,
            });
        }
        // egui forgets where a press started on the frame it is released, so remember it
        let id = self.response.id;
        if self.response.drag_started_by(PointerButton::Primary)
            && let Some(origin) = ui.input(|i| i.pointer.press_origin())
        {
            ui.data_mut(|d| d.insert_temp(id, origin));
        }
        let start: Pos2 = ui.data(|d| d.get_temp(id))?;
        let end = ui.input(|i| i.pointer.latest_pos())?;
        let drag = Drag {
            start: t.to_job(start),
            end: t.to_job(end),
            from_center: modifiers.alt,
            constrain: modifiers.command,
        };
        if self.response.drag_stopped_by(PointerButton::Primary) {
            ui.data_mut(|d| d.remove::<Pos2>(id));
            Some(Gesture::Dropped {
                drag,
                shift: modifiers.shift,
            })
        } else if self.response.dragged_by(PointerButton::Primary) {
            Some(Gesture::Dragging(drag))
        } else {
            None
        }
    }

    /// Draws the job, its datum, the vectors (selection dashed), toolpaths (`highlighted` in the
    /// selection colour) and `overlay`.
    /// Rulers along the top and left, in the job's units, as Aspire's 2D view has.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "a few decimals"
    )]
    pub fn rulers(&self, ui: &egui::Ui, units: Units) {
        let (area, t) = (self.area, self.transform);
        let visuals = ui.visuals();
        let painter = ui.painter_at(area);
        let thick = ui.spacing().interact_size.y;
        let gap = ui.spacing().item_spacing.x / 2.0;
        let top = Rect::from_min_size(area.min, vec2(area.width(), thick));
        let left = Rect::from_min_size(area.min, vec2(thick, area.height()));
        let (line, colour) = (
            visuals.widgets.noninteractive.fg_stroke,
            visuals.text_color(),
        );
        for strip in [top, left] {
            painter.rect_filled(strip, 0.0, visuals.extreme_bg_color);
        }
        painter.line_segment([top.left_bottom(), top.right_bottom()], line);
        painter.line_segment([left.right_top(), left.right_bottom()], line);
        let unit = match units {
            Units::Mm => 1.0,
            Units::Inch => 25.4,
        };
        let step = ruler_step(
            f64::from(t.scale) * unit,
            f64::from(thick * RULER_LABEL_ROWS),
        );
        let decimals = if step >= 1.0 {
            0
        } else {
            (-step.log10()).ceil() as usize
        };
        let font = egui::TextStyle::Small.resolve(ui.style());
        let label = |v: f64| format!("{v:.decimals$}");
        let minor = thick / 3.0;
        // across the top
        let (x0, x1) = (
            t.to_job(top.left_top()).x / unit,
            t.to_job(top.right_top()).x / unit,
        );
        for v in ruler_ticks(x0, x1, step / RULER_MINOR_TICKS) {
            let x = t.to_screen(Point::new(v * unit, 0.0)).x;
            if x > left.right() {
                painter.line_segment([pos2(x, top.bottom() - minor), pos2(x, top.bottom())], line);
            }
        }
        for v in ruler_ticks(x0, x1, step) {
            let x = t.to_screen(Point::new(v * unit, 0.0)).x;
            if x > left.right() {
                painter.line_segment([pos2(x, top.top()), pos2(x, top.bottom())], line);
                painter.text(
                    pos2(x + gap, top.top()),
                    egui::Align2::LEFT_TOP,
                    label(v),
                    font.clone(),
                    colour,
                );
            }
        }
        // down the left, labels turned to read up the ruler
        let (y0, y1) = (
            t.to_job(left.left_bottom()).y / unit,
            t.to_job(left.left_top()).y / unit,
        );
        for v in ruler_ticks(y0, y1, step / RULER_MINOR_TICKS) {
            let y = t.to_screen(Point::new(0.0, v * unit)).y;
            if y > top.bottom() {
                painter.line_segment([pos2(left.right() - minor, y), pos2(left.right(), y)], line);
            }
        }
        for v in ruler_ticks(y0, y1, step) {
            let y = t.to_screen(Point::new(0.0, v * unit)).y;
            if y > top.bottom() {
                painter.line_segment([pos2(left.left(), y), pos2(left.right(), y)], line);
                let galley = painter.layout_no_wrap(label(v), font.clone(), colour);
                painter.add(
                    egui::epaint::TextShape::new(pos2(left.left(), y - gap), galley, colour)
                        .with_angle(-FRAC_PI_2),
                );
            }
        }
        painter.rect_filled(
            Rect::from_min_size(area.min, vec2(thick, thick)),
            0.0,
            visuals.extreme_bg_color,
        );
    }

    /// Toolpaths seen from above, as the Toolpath Drawing menu asks.
    fn paint_toolpaths(
        &self,
        painter: &egui::Painter,
        ui: &egui::Ui,
        doc: &Document,
        highlighted: Option<EntityId>,
        display: &crate::display::Display,
    ) {
        let visuals = ui.visuals();
        let t = self.transform;
        let (dash, gap) = (
            ui.spacing().item_spacing.x,
            ui.spacing().item_spacing.x / 2.0,
        );
        let datum = visuals.widgets.noninteractive.fg_stroke;
        let selected = visuals.selection.stroke;
        // toolpaths seen from above: cutting moves (as wide as the tool when solid), and if
        // asked for, rapids dashed, plunges as dots and retracts as rings
        let marker = gap;
        for (id, toolpath) in doc
            .toolpaths
            .iter()
            .filter(|(_, t)| t.visible && display.previews_2d)
        {
            let colour = if highlighted == Some(*id) {
                selected.color
            } else {
                visuals.warn_fg_color
            };
            let cut_width = if display.solid_previews {
                to_f32(toolpath.tool.diameter) * t.scale
            } else {
                datum.width
            };
            let cut = Stroke::new(cut_width.max(datum.width), colour);
            let travel = Stroke::new(datum.width, visuals.error_fg_color);
            let mut from: Option<cut_craft_core::glam::DVec3> = None;
            for m in &toolpath.moves {
                let to = t.to_screen(m.to.truncate());
                if let Some(was) = from {
                    let start = t.to_screen(was.truncate());
                    let vertical = (was.truncate() - m.to.truncate()).length() < 1e-9;
                    match m.kind {
                        MoveKind::Cut => {
                            painter.line_segment([start, to], cut);
                        }
                        MoveKind::Plunge if display.plunges => {
                            painter.circle_filled(to, marker, travel.color);
                        }
                        MoveKind::Rapid if vertical && m.to.z > was.z && display.retracts => {
                            painter.circle_stroke(to, marker, travel);
                        }
                        MoveKind::Rapid if !vertical && display.rapids => {
                            painter.extend(Shape::dashed_line(&[start, to], travel, dash, gap));
                        }
                        _ => {}
                    }
                }
                from = Some(m.to);
            }
        }
    }

    pub fn paint(
        &self,
        ui: &egui::Ui,
        doc: &Document,
        overlay: &Overlay,
        highlighted: Option<EntityId>,
        display: &crate::display::Display,
    ) {
        let painter = ui.painter_at(self.area);
        let visuals = ui.visuals();
        let t = self.transform;
        let (dash, gap) = (
            ui.spacing().item_spacing.x,
            ui.spacing().item_spacing.x / 2.0,
        );

        let job = Rect::from_two_pos(t.to_screen(self.job.min), t.to_screen(self.job.max));
        painter.rect_filled(job, 0.0, visuals.widgets.inactive.bg_fill);
        for (id, bitmap) in &doc.bitmaps {
            let key = egui::Id::new(("bitmap", id.0, bitmap.columns, bitmap.rows));
            let ctx = ui.ctx();
            // uploaded once; kept while the bitmap is the same size
            let texture = ctx
                .data_mut(|d| d.get_temp::<egui::TextureHandle>(key))
                .unwrap_or_else(|| {
                    let texture = ctx.load_texture(
                        format!("bitmap {}", id.0),
                        egui::ColorImage::from_gray([bitmap.columns, bitmap.rows], &bitmap.grey),
                        egui::TextureOptions::LINEAR,
                    );
                    ctx.data_mut(|d| d.insert_temp(key, texture.clone()));
                    texture
                });
            let at = Rect::from_two_pos(t.to_screen(bitmap.rect.min), t.to_screen(bitmap.rect.max));
            let whole = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
            painter.image(texture.id(), at, whole, egui::Color32::WHITE);
        }
        let datum = visuals.widgets.noninteractive.fg_stroke;
        let (o, area) = (t.origin, self.area);
        for line in [
            [pos2(area.left(), o.y), pos2(area.right(), o.y)],
            [pos2(o.x, area.top()), pos2(o.x, area.bottom())],
        ] {
            painter.extend(Shape::dashed_line(&line, datum, dash, gap));
        }

        // chords within half a physical pixel of the true curve
        let tolerance = f64::from(0.5 / (ui.ctx().pixels_per_point() * t.scale));
        let screen = |path: &Path| -> Vec<Pos2> {
            path.flatten(tolerance)
                .into_iter()
                .map(|p| t.to_screen(p))
                .collect()
        };
        let normal = Stroke::new(datum.width, visuals.text_color());
        let selected = visuals.selection.stroke;
        for (id, path) in doc.vectors.iter().filter(|(id, _)| doc.shown(**id)) {
            if doc.selection.contains(id) {
                painter.extend(Shape::dashed_line(&screen(path), selected, dash, gap));
            } else {
                painter.add(Shape::line(screen(path), normal));
            }
        }

        self.paint_toolpaths(&painter, ui, doc, highlighted, display);

        match overlay {
            Overlay::None => {}
            Overlay::Preview(paths) => {
                for path in paths {
                    painter.add(Shape::line(screen(path), selected));
                }
            }
            Overlay::Window {
                start,
                end,
                crossing,
            } => {
                let r = Rect::from_two_pos(t.to_screen(*start), t.to_screen(*end));
                if *crossing {
                    let corners = [
                        r.left_top(),
                        r.right_top(),
                        r.right_bottom(),
                        r.left_bottom(),
                        r.left_top(),
                    ];
                    painter.extend(Shape::dashed_line(&corners, selected, dash, gap));
                } else {
                    painter.rect_stroke(r, 0.0, selected);
                }
            }
        }
    }
}

/// Job coordinates are f64 millimetres; the screen is f32 pixels.
#[expect(
    clippy::cast_possible_truncation,
    reason = "screen precision is all that is needed here"
)]
fn to_f32(v: f64) -> f32 {
    v as f32
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui::{pos2, vec2};

    #[test]
    fn zooming_keeps_the_point_under_the_cursor_still() {
        let t = ViewTransform {
            origin: pos2(100.0, 400.0),
            scale: 2.0,
        };
        let cursor = pos2(250.0, 180.0);
        let zoomed = t.zoomed_about(cursor, 1.5);
        assert!((zoomed.scale - 3.0).abs() < 1e-6);
        assert!(zoomed.to_job(cursor).distance(t.to_job(cursor)) < 1e-4);
    }

    #[test]
    fn panning_moves_everything_with_the_drag() {
        let t = ViewTransform {
            origin: pos2(100.0, 400.0),
            scale: 2.0,
        };
        let p = Point::new(10.0, 20.0);
        let moved = t.panned(vec2(30.0, -5.0));
        assert!((moved.to_screen(p) - t.to_screen(p) - vec2(30.0, -5.0)).length() < 1e-4);
    }

    #[test]
    fn ruler_steps_go_one_two_five_and_never_crowd_their_labels() {
        let step = |pixels_per_unit| ruler_step(pixels_per_unit, 50.0);
        for (ppu, expected) in [
            (0.5, 100.0),
            (3.0, 20.0),
            (12.0, 5.0),
            (60.0, 1.0),
            (600.0, 0.1),
        ] {
            assert!(
                (step(ppu) - expected).abs() < 1e-9,
                "{ppu} px per unit: {}",
                step(ppu)
            );
        }
        assert_eq!(
            ruler_ticks(-35.0, 120.0, 20.0),
            [-20.0, 0.0, 20.0, 40.0, 60.0, 80.0, 100.0, 120.0]
        );
    }

    #[test]
    fn a_job_centred_on_its_datum_is_still_fitted_in_the_middle() {
        let area = Rect::from_min_size(pos2(0.0, 0.0), vec2(500.0, 300.0));
        let job = Job {
            datum: cut_craft_core::document::Datum::Centre,
            ..Job::default()
        };
        let t = ViewTransform::fit(area, 10.0, job.area());
        let middle = t.to_screen(Point::ZERO);
        assert!(
            (middle.x - 250.0).abs() < 1e-3 && (middle.y - 150.0).abs() < 1e-3,
            "{middle:?}"
        );
    }

    #[test]
    fn fit_keeps_the_aspect_ratio_and_centres_the_job() {
        let area = Rect::from_min_size(pos2(0.0, 0.0), vec2(500.0, 300.0));
        let t = ViewTransform::fit(area, 10.0, Job::default().area());
        assert!(
            (t.scale - 0.7).abs() < 1e-6,
            "limited by height: (300-20)/400"
        );
        let (bl, tr) = (
            t.to_screen(Point::new(0.0, 0.0)),
            t.to_screen(Point::new(600.0, 400.0)),
        );
        assert!(
            (bl.y - 290.0).abs() < 1e-3 && (tr.y - 10.0).abs() < 1e-3,
            "{bl:?} {tr:?}"
        );
        assert!(
            (f32::midpoint(bl.x, tr.x) - 250.0).abs() < 1e-3,
            "centred horizontally"
        );
    }

    #[test]
    fn y_points_up_in_the_job_and_down_on_screen() {
        let t = ViewTransform {
            origin: pos2(100.0, 200.0),
            scale: 2.0,
        };
        assert_eq!(t.to_screen(Point::new(10.0, 5.0)), pos2(120.0, 190.0));
    }

    #[test]
    fn screen_to_job_inverts_job_to_screen() {
        let t = ViewTransform {
            origin: pos2(37.0, 410.0),
            scale: 1.7,
        };
        let p = Point::new(123.4, -56.7);
        assert!(t.to_job(t.to_screen(p)).distance(p) < 1e-3);
    }
}
