//! The 2D view: job coordinates (mm, y up) ↔ screen points (pixels, y down).

use aspire_core::document::Document;
use aspire_core::geometry::Point;
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
    pub fn fit(area: Rect, margin: f32, width: f64, height: f64) -> Self {
        let inner = area.shrink(margin);
        let (w, h) = (to_f32(width), to_f32(height));
        let scale = (inner.width() / w).min(inner.height() / h);
        let c = inner.center();
        Self {
            origin: pos2(c.x - w * scale / 2.0, c.y + h * scale / 2.0),
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

/// Draws the job and its vectors; returns the pointer position in job coordinates.
pub fn paint(ui: &mut egui::Ui, doc: &Document) -> Option<Point> {
    let (area, response) = ui.allocate_exact_size(ui.available_size(), Sense::hover());
    let painter = ui.painter_at(area);
    let visuals = ui.visuals();
    let spacing = ui.spacing();
    let t = ViewTransform::fit(area, spacing.interact_size.y, doc.job.width, doc.job.height);

    let job = Rect::from_two_pos(
        t.to_screen(Point::ZERO),
        t.to_screen(Point::new(doc.job.width, doc.job.height)),
    );
    painter.rect_filled(job, 0.0, visuals.widgets.inactive.bg_fill);

    let (dash, gap) = (spacing.item_spacing.x, spacing.item_spacing.x / 2.0);
    let datum = visuals.widgets.noninteractive.fg_stroke;
    let o = t.origin;
    for line in [
        [pos2(area.left(), o.y), pos2(area.right(), o.y)],
        [pos2(o.x, area.top()), pos2(o.x, area.bottom())],
    ] {
        painter.extend(Shape::dashed_line(&line, datum, dash, gap));
    }

    // chords within half a physical pixel of the true curve
    let tolerance = f64::from(0.5 / (ui.ctx().pixels_per_point() * t.scale));
    let normal = Stroke::new(datum.width, visuals.text_color());
    let selected = visuals.selection.stroke;
    for (id, path) in &doc.vectors {
        let points: Vec<Pos2> = path
            .flatten(tolerance)
            .into_iter()
            .map(|p| t.to_screen(p))
            .collect();
        if doc.selection.contains(id) {
            painter.extend(Shape::dashed_line(&points, selected, dash, gap));
        } else {
            painter.add(Shape::line(points, normal));
        }
    }
    response.hover_pos().map(|p| t.to_job(p))
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
    fn fit_keeps_the_aspect_ratio_and_centres_the_job() {
        let area = Rect::from_min_size(pos2(0.0, 0.0), vec2(500.0, 300.0));
        let t = ViewTransform::fit(area, 10.0, 600.0, 400.0);
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
