//! The 2D view: job coordinates (mm, y up) ↔ screen points (pixels, y down), pointer
//! gestures in job coordinates, and painting the document.

use cut_craft_core::Drag;
use cut_craft_core::document::{Document, EntityId, Job};
use cut_craft_core::geometry::{Path, Point, Rect as JobRect};
use cut_craft_core::toolpath::MoveKind;
use eframe::egui::{self, Pos2, Rect, Sense, Shape, Stroke, pos2};

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
    pub fn new(ui: &mut egui::Ui, job: &Job) -> Self {
        let (area, response) = ui.allocate_exact_size(ui.available_size(), Sense::click_and_drag());
        let transform = ViewTransform::fit(area, ui.spacing().interact_size.y, job.area());
        Self {
            transform,
            area,
            response,
            job: job.area(),
        }
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
        if self.response.drag_started()
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
        if self.response.drag_stopped() {
            ui.data_mut(|d| d.remove::<Pos2>(id));
            Some(Gesture::Dropped {
                drag,
                shift: modifiers.shift,
            })
        } else if self.response.dragged() {
            Some(Gesture::Dragging(drag))
        } else {
            None
        }
    }

    /// Draws the job, its datum, the vectors (selection dashed), toolpaths (`highlighted` in the
    /// selection colour) and `overlay`.
    pub fn paint(
        &self,
        ui: &egui::Ui,
        doc: &Document,
        overlay: &Overlay,
        highlighted: Option<EntityId>,
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

        // toolpaths: their cutting moves seen from above
        for (id, toolpath) in &doc.toolpaths {
            let colour = if highlighted == Some(*id) {
                selected.color
            } else {
                visuals.warn_fg_color
            };
            let stroke = Stroke::new(datum.width, colour);
            let mut from = None;
            for m in &toolpath.moves {
                let to = t.to_screen(m.to.truncate());
                if let (Some(from), MoveKind::Cut) = (from, m.kind) {
                    painter.line_segment([from, to], stroke);
                }
                from = Some(to);
            }
        }

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
