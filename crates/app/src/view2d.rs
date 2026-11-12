//! The 2D view: job coordinates (mm, y up) ↔ screen points (pixels, y down), pointer
//! gestures in job coordinates, and painting the document.

use cut_craft_core::Drag;
use cut_craft_core::document::{Document, EntityId, Job, Units};
use cut_craft_core::geometry::{Path, Point, Rect as JobRect};
use cut_craft_core::glam::DAffine2;
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
/// How many of their own sizes apart grid points must be on screen to be drawn.
const GRID_GAP: f32 = 4.0;

/// The distance between grid points drawn: the spacing, or the least multiple of it at
/// least `min_gap` screen points apart at `scale` points a millimetre; none for no spacing.
pub fn grid_step(spacing: f64, scale: f64, min_gap: f64) -> Option<f64> {
    if spacing <= 0.0 || scale <= 0.0 {
        return None;
    }
    let every = (min_gap / (spacing * scale)).ceil().max(1.0);
    Some(spacing * every)
}

/// The grid points `step` apart from the job's zero (whole multiples) inside `job`.
#[expect(clippy::cast_possible_truncation, reason = "counts of grid points")]
pub fn grid_points(job: cut_craft_core::geometry::Rect, step: f64) -> Vec<Point> {
    let range = |lo: f64, hi: f64| (lo / step).ceil() as i64..=(hi / step).floor() as i64;
    let mut points = vec![];
    for row in range(job.min.y, job.max.y) {
        for column in range(job.min.x, job.max.x) {
            #[expect(clippy::cast_precision_loss, reason = "small counts")]
            points.push(Point::new(column as f64 * step, row as f64 * step));
        }
    }
    points
}

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

/// A node handle's edge, as a share of the theme's interactive row height.
// ponytail: fixed share until a preferences dialog sizes handles
pub const NODE_HANDLE_SHARE: f32 = 0.35;

/// What the pointer did on the canvas this frame.
#[derive(Clone, Copy, Debug)]
pub enum Gesture {
    Click { at: Point, shift: bool },
    Dragging(Drag),
    Dropped { drag: Drag, shift: bool },
}

/// The left button's click, drag or drop on `response`, in job coordinates by `to_job`
/// (the 2D view's transform, or where the 3D view's pointer meets the model): `None` while
/// nothing happens, or where `to_job` finds no point.
pub fn gesture_of(
    ui: &egui::Ui,
    response: &egui::Response,
    to_job: impl Fn(Pos2) -> Option<Point>,
) -> Option<Gesture> {
    let modifiers = ui.input(|i| i.modifiers);
    if response.clicked() {
        let at = to_job(response.interact_pointer_pos()?)?;
        return Some(Gesture::Click {
            at,
            shift: modifiers.shift,
        });
    }
    // egui forgets where a press started on the frame it is released, so remember it
    let id = response.id;
    if response.drag_started_by(PointerButton::Primary)
        && let Some(origin) = ui.input(|i| i.pointer.press_origin())
    {
        ui.data_mut(|d| d.insert_temp(id, origin));
    }
    let start: Pos2 = ui.data(|d| d.get_temp(id))?;
    let stopped = response.drag_stopped_by(PointerButton::Primary);
    if stopped {
        ui.data_mut(|d| d.remove::<Pos2>(id));
    }
    let end = ui.input(|i| i.pointer.latest_pos())?;
    let drag = Drag {
        start: to_job(start)?,
        end: to_job(end)?,
        from_center: modifiers.alt,
        constrain: modifiers.command,
    };
    if stopped {
        Some(Gesture::Dropped {
            drag,
            shift: modifiers.shift,
        })
    } else if response.dragged_by(PointerButton::Primary) {
        Some(Gesture::Dragging(drag))
    } else {
        None
    }
}

/// Drawn over the document while a gesture is in progress.
pub enum Overlay {
    None,
    /// What the active drawing tool would create.
    Preview(Vec<Path>),
    /// A drag moving, scaling or turning the selection: the outlines as they will be, and
    /// each model with the transform carrying it.
    Moving {
        paths: Vec<Path>,
        components: Vec<(EntityId, DAffine2)>,
    },
    /// Node editing: handles on the selected vectors' nodes, and a node's drag in progress.
    Nodes {
        at: Vec<Point>,
        /// Nodes picked with a box, drawn filled: a drag of one moves them all.
        picked: Vec<Point>,
        moving: Vec<Path>,
    },
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
        let t = self.transform;
        gesture_of(ui, &self.response, |at| Some(t.to_job(at)))
    }

    /// Draws the job, its datum, the vectors (selection dashed), toolpaths (`highlighted` in the
    /// selection colour) and `overlay`.
    /// Where a guide's line crosses the view, on screen: from edge to edge for one along an
    /// axis, far past the view either way for an angled one.
    pub fn guide_ends(&self, g: &cut_craft_core::document::Guide) -> [Pos2; 2] {
        let t = self.transform;
        if !g.tilted() {
            let at = t.to_screen(Point::new(g.at, g.at));
            return if g.vertical {
                [pos2(at.x, self.area.top()), pos2(at.x, self.area.bottom())]
            } else {
                [pos2(self.area.left(), at.y), pos2(self.area.right(), at.y)]
            };
        }
        let middle = g.nearest(t.to_job(self.area.center()));
        let far = f64::from(self.area.size().max_elem() / t.scale) * 2.0;
        let d = g.direction();
        [t.to_screen(middle - d * far), t.to_screen(middle + d * far)]
    }

    /// Whether `at` is on a ruler: `Some(true)` the left one (it gives vertical guides),
    /// `Some(false)` the top one; the corner where they meet is neither.
    pub fn ruler_at(&self, ui: &egui::Ui, at: Pos2) -> Option<bool> {
        let thick = ui.spacing().interact_size.y;
        let (x, y) = (at.x - self.area.min.x, at.y - self.area.min.y);
        if !self.area.contains(at) || (x < thick && y < thick) {
            return None;
        }
        (x < thick || y < thick).then_some(x < thick)
    }

    /// Whether a drag with the main button started or stopped this frame.
    pub fn drag_started(&self) -> bool {
        self.response.drag_started_by(PointerButton::Primary)
    }

    pub fn drag_stopped(&self) -> bool {
        self.response.drag_stopped_by(PointerButton::Primary)
    }

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

    /// Every visible component as a greyscale depth map; one being dragged (in `moving`)
    /// faint where it is and drawn as the drag will leave it.
    fn paint_components(
        &self,
        ui: &egui::Ui,
        painter: &egui::Painter,
        doc: &Document,
        stamp: Stamp,
        moving: &[(EntityId, DAffine2)],
    ) {
        let t = self.transform;
        let whole = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
        for (&id, component) in doc.components.iter().filter(|(_, c)| c.visible) {
            let texture = component_texture(ui.ctx(), id, component, stamp);
            let e = component.extent();
            let at = Rect::from_two_pos(t.to_screen(e.min), t.to_screen(e.max));
            let Some((_, by)) = moving.iter().find(|(m, _)| *m == id) else {
                painter.image(texture.id(), at, whole, egui::Color32::WHITE);
                continue;
            };
            painter.image(
                texture.id(),
                at,
                whole,
                egui::Color32::from_white_alpha(MOVING_FADE),
            );
            // the picture's corners carried by the drag: turned, stretched or mirrored with it
            let mut mesh = egui::Mesh::with_texture(texture.id());
            let corners = [
                (Point::new(e.min.x, e.max.y), pos2(0.0, 0.0)),
                (e.max, pos2(1.0, 0.0)),
                (Point::new(e.max.x, e.min.y), pos2(1.0, 1.0)),
                (e.min, pos2(0.0, 1.0)),
            ];
            for (corner, uv) in corners {
                mesh.vertices.push(egui::epaint::Vertex {
                    pos: t.to_screen(by.transform_point2(corner)),
                    uv,
                    color: egui::Color32::WHITE,
                });
            }
            mesh.add_triangle(0, 1, 2);
            mesh.add_triangle(0, 2, 3);
            painter.add(Shape::mesh(mesh));
        }
    }

    /// Selected bitmaps and components: their extent, dashed like selected vectors.
    fn paint_outlines(
        &self,
        painter: &egui::Painter,
        doc: &Document,
        selected: Stroke,
        (dash, gap): (f32, f32),
    ) {
        let t = self.transform;
        for bitmap in doc
            .selected_bitmaps
            .iter()
            .filter_map(|id| doc.bitmaps.get(id))
        {
            let JobRect { min: lo, max: hi } = bitmap.rect;
            let corners = [lo, Point::new(hi.x, lo.y), hi, Point::new(lo.x, hi.y), lo]
                .map(|p| t.to_screen(p));
            painter.extend(Shape::dashed_line(&corners, selected, dash, gap));
        }
        for component in doc
            .selected_components
            .iter()
            .filter_map(|id| doc.components.get(id))
        {
            let JobRect { min: lo, max: hi } = component.extent();
            let corners = [lo, Point::new(hi.x, lo.y), hi, Point::new(lo.x, hi.y), lo]
                .map(|p| t.to_screen(p));
            painter.extend(Shape::dashed_line(&corners, selected, dash, gap));
        }
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
        // asked for, rapids dashed, plunges as dots, retracts as rings and joins dotted
        let marker = gap;
        // what lies off the view is left out, and a frame draws so many points at most
        let view = painter.clip_rect().expand(marker + datum.width);
        let mut room = MOST_TOOLPATH_POINTS;
        for (id, toolpath) in doc
            .toolpaths
            .iter()
            .filter(|(_, t)| t.visible && display.previews_2d)
        {
            let colour = if highlighted == Some(*id) {
                selected.color
            } else if display.solid_previews {
                let [r, g, b] = display.solid_colour;
                egui::Color32::from_rgb(r, g, b)
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
            // the cuts as lines thinned to what the screen shows: one for each cut was
            // millions on a fine 3D finish, more than the graphics card takes
            let mut cuts = Runs::new(view.expand(cut.width), room);
            let mut marks = vec![];
            let mut from: Option<cut_craft_core::glam::DVec3> = None;
            for m in &toolpath.moves {
                let to = t.to_screen(m.to.truncate());
                let Some(was) = from.replace(m.to) else {
                    continue;
                };
                let start = t.to_screen(was.truncate());
                let vertical = (was.truncate() - m.to.truncate()).length() < 1e-9;
                // ponytail: a join is a feed move wholly above the material [inf]
                let join = m.kind == MoveKind::Cut && was.z >= 0.0 && m.to.z >= 0.0;
                let (seen, full) = (
                    view.intersects(Rect::from_two_pos(start, to)),
                    marks.len() + cuts.used() >= room,
                );
                match m.kind {
                    MoveKind::Cut | MoveKind::Plunge if !join && !vertical => {
                        cuts.line(start, to);
                    }
                    // straight down: the line goes on from where it is
                    MoveKind::Plunge if vertical => {
                        if display.plunges && seen && !full {
                            marks.push(Shape::circle_filled(to, marker, travel.color));
                        }
                    }
                    MoveKind::Cut if join => {
                        cuts.gap();
                        if display.joins && seen && !full {
                            marks.extend(Shape::dotted_line(
                                &[start, to],
                                colour,
                                gap,
                                datum.width,
                            ));
                        }
                    }
                    MoveKind::Rapid if vertical => {
                        cuts.gap();
                        if m.to.z > was.z && display.retracts && seen && !full {
                            marks.push(Shape::circle_stroke(to, marker, travel));
                        }
                    }
                    MoveKind::Rapid => {
                        cuts.gap();
                        if display.rapids && seen && !full {
                            marks.extend(Shape::dashed_line(&[start, to], travel, dash, gap));
                        }
                    }
                    _ => {}
                }
            }
            room = room.saturating_sub(cuts.used() + marks.len());
            painter.extend(cuts.finish().into_iter().map(|run| Shape::line(run, cut)));
            painter.extend(marks);
        }
    }

    /// Guide lines (dashed, while shown) and keep-out zones (in the warning colour, heavier
    /// than vectors).
    fn paint_guides_and_zones(
        &self,
        painter: &egui::Painter,
        ui: &egui::Ui,
        doc: &Document,
        display: &crate::display::Display,
        (dash, gap): (f32, f32),
    ) {
        let (t, visuals) = (self.transform, ui.visuals());
        let datum = visuals.widgets.noninteractive.fg_stroke;
        let tolerance = f64::from(0.5 / (ui.ctx().pixels_per_point() * t.scale));
        let screen = |path: &Path| -> Vec<Pos2> {
            path.flatten(tolerance)
                .into_iter()
                .map(|p| t.to_screen(p))
                .collect()
        };
        if display.guides {
            let guide = Stroke::new(datum.width, visuals.selection.bg_fill);
            for g in &doc.guides {
                painter.extend(Shape::dashed_line(&self.guide_ends(g), guide, dash, gap));
            }
        }
        // toolpath tiles: their edges, and their overlap faintly
        if let Some(tiling) = doc.tiling {
            let edge = Stroke::new(2.0 * datum.width, visuals.warn_fg_color);
            let overlap = Stroke::new(datum.width, visuals.error_fg_color);
            let active = tiling.active_tile(doc.job.area());
            for tile in tiling.tiles(doc.job.area()) {
                let reach = tiling.reach(tile);
                for (r, stroke) in [(reach, overlap), (tile, edge)] {
                    let screen_rect =
                        egui::Rect::from_two_pos(t.to_screen(r.min), t.to_screen(r.max));
                    painter.rect_stroke(screen_rect, 0, stroke, egui::StrokeKind::Middle);
                }
                // the active tile, shaded
                if active == Some(tile) {
                    let screen_rect =
                        egui::Rect::from_two_pos(t.to_screen(tile.min), t.to_screen(tile.max));
                    painter.rect_filled(
                        screen_rect,
                        0,
                        visuals.warn_fg_color.gamma_multiply(ACTIVE_TILE_SHADE),
                    );
                }
            }
        }
        // a double-sided job's other side, faint and turned over as the material is
        if let Some(sides) = doc.job.double_sided {
            let turn = sides.flip.mirror(doc.job.area());
            let faint = Stroke::new(datum.width, visuals.weak_text_color());
            for v in doc.other_side.vectors.values() {
                painter.add(Shape::line(screen(&v.path.transformed(&turn)), faint));
            }
        }
        // keep-out zones in the warning colour, heavier than vectors, each named at its
        // top-left corner while labels are shown
        let zone = Stroke::new(2.0 * datum.width, visuals.error_fg_color);
        for z in doc.named_keep_out() {
            painter.add(Shape::line(screen(&z.path), zone));
            if display.keep_out_labels {
                let b = z.path.bounds();
                let corner = t.to_screen(Point::new(b.min.x, b.max.y));
                zone_label(painter, ui, corner, &z.label());
            }
        }
    }

    /// The snap grid's points over the job, `spacing` apart, coloured and sized as asked;
    /// when they would crowd closer than a few of their own size, only every so many.
    pub fn paint_grid(&self, ui: &egui::Ui, spacing: f64, colour: [u8; 3], size: f32) {
        let t = self.transform;
        let Some(step) = grid_step(spacing, f64::from(t.scale), f64::from(GRID_GAP * size)) else {
            return;
        };
        let painter = ui.painter_at(self.area);
        let colour = egui::Color32::from_rgb(colour[0], colour[1], colour[2]);
        for p in grid_points(self.job, step) {
            let at = t.to_screen(p);
            if self.area.contains(at) {
                painter.circle_filled(at, size / 2.0, colour);
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
        stamp: Stamp,
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
            let texture = bitmap_texture(ui.ctx(), *id, bitmap);
            let at = Rect::from_two_pos(t.to_screen(bitmap.rect.min), t.to_screen(bitmap.rect.max));
            let whole = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
            painter.image(texture.id(), at, whole, egui::Color32::WHITE);
        }
        let moving = match overlay {
            Overlay::Moving { components, .. } => components.as_slice(),
            _ => &[],
        };
        self.paint_components(ui, &painter, doc, stamp, moving);
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
            // as in Aspire: selected vectors dashed, selected groups solid
            if doc.selection.contains(id) && doc.vector_groups.contains_key(id) {
                painter.add(Shape::line(screen(path), selected));
            } else if doc.selection.contains(id) {
                painter.extend(Shape::dashed_line(&screen(path), selected, dash, gap));
            } else {
                painter.add(Shape::line(screen(path), normal));
            }
        }

        self.paint_guides_and_zones(&painter, ui, doc, display, (dash, gap));
        self.paint_outlines(&painter, doc, selected, (dash, gap));
        self.paint_toolpaths(&painter, ui, doc, highlighted, display);

        match overlay {
            Overlay::None => {}
            Overlay::Preview(paths) | Overlay::Moving { paths, .. } => {
                for path in paths {
                    painter.add(Shape::line(screen(path), selected));
                }
            }
            Overlay::Nodes { at, picked, moving } => {
                let half = ui.spacing().interact_size.y * NODE_HANDLE_SHARE / 2.0;
                for q in at {
                    let r = Rect::from_center_size(t.to_screen(*q), egui::vec2(half, half) * 2.0);
                    let fill = if picked.iter().any(|p| p.distance(*q) < 1e-9) {
                        selected.color
                    } else {
                        ui.visuals().extreme_bg_color
                    };
                    painter.rect_filled(r, 0.0, fill);
                    painter.rect_stroke(r, 0, selected, egui::StrokeKind::Middle);
                }
                for path in moving {
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
                    painter.rect_stroke(r, 0, selected, egui::StrokeKind::Middle);
                }
            }
        }
    }
}

/// How faint a dragged component is drawn where it was, out of 255.
const MOVING_FADE: u8 = 70;

/// Which job and which change of it the 2D view draws: what it keeps of the job (its
/// models' pictures) is checked again only when this changes.
pub type Stamp = (u64, u64);

/// A component's picture as last drawn: for which [`Stamp`], from which heights (their
/// hash), and the texture itself (one a component: a new picture lets the old one go).
#[derive(Clone)]
struct ComponentPicture {
    stamp: Stamp,
    hash: u64,
    texture: egui::TextureHandle,
}

/// Component `id`'s picture: its heights hashed only when the job has changed since it
/// was last drawn, and the picture made again only when they have.
fn component_texture(
    ctx: &egui::Context,
    id: EntityId,
    component: &cut_craft_core::component::Component,
    stamp: Stamp,
) -> egui::TextureHandle {
    let key = egui::Id::unique(("component picture", id.0));
    let kept = ctx.data(|d| d.get_temp::<ComponentPicture>(key));
    if let Some(kept) = &kept
        && kept.stamp == stamp
    {
        return kept.texture.clone();
    }
    let hash = component
        .shaped_heights()
        .into_iter()
        .fold(0xcbf2_9ce4_8422_2325_u64, |h, z| {
            (h ^ z.to_bits()).wrapping_mul(0x0100_0000_01b3)
        })
        ^ ((component.columns as u64) << 32 | component.rows as u64);
    let texture = match kept {
        Some(kept) if kept.hash == hash => kept.texture,
        _ => {
            let pixels: Vec<egui::Color32> = component
                .shade()
                .into_iter()
                .map(|g| g.map_or(egui::Color32::TRANSPARENT, egui::Color32::from_gray))
                .collect();
            let image = egui::ColorImage::new([component.columns, component.rows], pixels);
            ctx.load_texture(
                format!("component {}", id.0),
                fitting(ctx, image),
                egui::TextureOptions::LINEAR,
            )
        }
    };
    let picture = ComponentPicture {
        stamp,
        hash,
        texture: texture.clone(),
    };
    ctx.data_mut(|d| d.insert_temp(key, picture));
    texture
}

/// `bitmap`'s texture: uploaded once, and kept while the bitmap is the same size.
fn bitmap_texture(
    ctx: &egui::Context,
    id: EntityId,
    bitmap: &cut_craft_core::bitmap::Bitmap,
) -> egui::TextureHandle {
    let key = egui::Id::unique(("bitmap", id.0, bitmap.columns, bitmap.rows));
    ctx.data_mut(|d| d.get_temp::<egui::TextureHandle>(key))
        .unwrap_or_else(|| {
            let image = egui::ColorImage::from_gray([bitmap.columns, bitmap.rows], &bitmap.grey);
            let texture = ctx.load_texture(
                format!("bitmap {}", id.0),
                fitting(ctx, image),
                egui::TextureOptions::LINEAR,
            );
            ctx.data_mut(|d| d.insert_temp(key, texture.clone()));
            texture
        })
}

/// `image` shrunk, if it must be, to at most `side` pixels each way (the graphics card's
/// largest texture), keeping its shape: a texture bigger than that stops the app.
pub fn within(image: egui::ColorImage, most: usize) -> egui::ColorImage {
    let [width, height] = image.size;
    let step = width.max(height).div_ceil(most.max(1));
    if step <= 1 {
        return image;
    }
    let shrunk = [width.div_ceil(step), height.div_ceil(step)];
    let pixels = (0..shrunk[1])
        .flat_map(|y| (0..shrunk[0]).map(move |x| (x, y)))
        .map(|(x, y)| image.pixels[y * step * width + x * step])
        .collect();
    egui::ColorImage::new(shrunk, pixels)
}

/// [`within`] what `ctx`'s graphics card takes.
pub fn fitting(ctx: &egui::Context, image: egui::ColorImage) -> egui::ColorImage {
    within(image, ctx.input(|i| i.max_texture_side))
}

/// Most points (and marks) a frame draws toolpaths with: about 100 MB of the graphics card's
/// buffer however dense they are, where a line for each of a fine 3D finish's millions of
/// cuts was more than the 2 GB it takes, and crashed the app.
pub const MOST_TOOLPATH_POINTS: usize = 1_000_000;

/// How far a thinned line strays from the cuts it stands for, in points (the screen's).
const THINNED_WITHIN: f32 = 0.25;

/// Cuts as lines on screen, thinned to what shows: a cut too short to see is joined to the
/// next, cuts in a line are drawn as one, and those wholly off the view are left out. Each
/// point kept stays within half a point of the cuts.
#[derive(Debug)]
pub struct Runs {
    view: Rect,
    /// Points it may still keep.
    room: usize,
    used: usize,
    /// The lines finished, and the one being drawn.
    done: Vec<Vec<Pos2>>,
    run: Vec<Pos2>,
    /// Where the cuts have got to, past the last point kept when too near it to keep.
    end: Option<Pos2>,
    /// The line the last point is being moved along: from the point before, this way.
    along: Option<(Pos2, Vec2)>,
}

impl Runs {
    /// For cuts in `view`, keeping `room` points at most.
    pub fn new(view: Rect, room: usize) -> Self {
        Self {
            view,
            room,
            used: 0,
            done: vec![],
            run: vec![],
            end: None,
            along: None,
        }
    }

    /// The points kept so far.
    pub fn used(&self) -> usize {
        self.used
    }

    fn keep(&mut self, p: Pos2) -> bool {
        if self.used >= self.room {
            return false;
        }
        self.used += 1;
        self.run.push(p);
        true
    }

    /// A cut from `from` to `to`, on screen.
    pub fn line(&mut self, from: Pos2, to: Pos2) {
        if !self.view.intersects(Rect::from_two_pos(from, to)) {
            self.gap();
            return;
        }
        if self.end.is_none_or(|end| end.distance(from) > 1e-3) {
            self.gap();
            if !self.keep(from) {
                return;
            }
        }
        self.end = Some(to);
        let Some(&last) = self.run.last() else {
            return;
        };
        if last.distance(to) < 2.0 * THINNED_WITHIN {
            return;
        }
        if let Some((start, way)) = self.along {
            let off = (to - start).dot(way.rot90()).abs();
            if off <= THINNED_WITHIN && (to - start).dot(way) >= (last - start).dot(way) {
                *self.run.last_mut().expect("a point kept") = to;
                return;
            }
        }
        if self.keep(to) {
            self.along = Some((last, (to - last).normalized()));
        } else {
            self.end = None;
        }
    }

    /// No cut joins the last to the next.
    pub fn gap(&mut self) {
        if let Some(end) = self.end.take()
            && self.run.last().is_some_and(|&last| last != end)
        {
            self.keep(end);
        }
        let run = std::mem::take(&mut self.run);
        if run.len() >= 2 {
            self.done.push(run);
        }
        self.along = None;
    }

    /// The lines to draw.
    pub fn finish(mut self) -> Vec<Vec<Pos2>> {
        self.gap();
        self.done
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

/// How strongly the active toolpath tile is shaded in the 2D view.
const ACTIVE_TILE_SHADE: f32 = 0.15;

/// A keep-out zone's label ("Keep-out: Clamp 1"), small, in the warning colour on the
/// panel's background so it reads over whatever is under it, its top-left corner just in
/// from `corner`.
pub fn zone_label(painter: &egui::Painter, ui: &egui::Ui, corner: Pos2, text: &str) {
    let visuals = ui.visuals();
    let font = egui::TextStyle::Small.resolve(ui.style());
    let galley = painter.layout_no_wrap(text.to_owned(), font, visuals.error_fg_color);
    let pad = ui.spacing().item_spacing.x / 2.0;
    let at = corner + egui::vec2(2.0 * pad, 2.0 * pad);
    let back = egui::Rect::from_min_size(at, galley.size()).expand(pad);
    painter.rect_filled(back, pad, visuals.panel_fill.gamma_multiply(0.85));
    painter.galley(at, galley, visuals.error_fg_color);
}

#[cfg(test)]
mod tests {

    #[test]
    fn the_grid_thins_out_when_its_points_would_crowd() {
        // 10 mm apart at 2 points a mm is 20 points: drawn as it is
        assert_eq!(grid_step(10.0, 2.0, 8.0), Some(10.0));
        // at 0.3 points a mm only every third is 8 or more points apart
        assert_eq!(grid_step(10.0, 0.3, 8.0), Some(30.0));
        assert_eq!(grid_step(0.0, 1.0, 8.0), None);
        let job =
            cut_craft_core::geometry::Rect::new(Point::new(-5.0, 0.0), Point::new(20.0, 10.0));
        let points = grid_points(job, 10.0);
        assert_eq!(points.len(), 3 * 2, "x 0, 10, 20 by y 0, 10");
        assert!(points.contains(&Point::new(20.0, 10.0)));
    }

    use super::*;
    use eframe::egui::{pos2, vec2};

    /// Points of `runs` all told.
    fn points(runs: &[Vec<Pos2>]) -> usize {
        runs.iter().map(Vec::len).sum()
    }

    /// The distance from `p` to the nearest of `runs`' lines.
    fn off(runs: &[Vec<Pos2>], p: Pos2) -> f32 {
        runs.iter()
            .flat_map(|run| run.windows(2))
            .map(|w| {
                let (a, b) = (w[0], w[1]);
                let along = (b - a).length_sq();
                let t = if along > 0.0 {
                    ((p - a).dot(b - a) / along).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                p.distance(a + (b - a) * t)
            })
            .fold(f32::INFINITY, f32::min)
    }

    /// A 3D finishing raster of millions of tiny cuts (Matthew's: 6.8 million) drew a line
    /// for each, and the 2 GB of lines crashed the graphics card's buffer: rows that are
    /// straight on screen are drawn as a line each.
    #[test]
    fn a_dense_raster_is_drawn_with_a_line_a_row() {
        let view = Rect::from_min_size(Pos2::ZERO, vec2(1000.0, 800.0));
        let mut runs = Runs::new(view, usize::MAX);
        let (rows, steps) = (200, 10_000);
        let mut at = pos2(10.0, 10.0);
        for row in 0..rows {
            let dir = if row % 2 == 0 { 1.0 } else { -1.0 };
            for _ in 0..steps {
                let to = at + vec2(dir * 0.09, 0.0);
                runs.line(at, to);
                at = to;
            }
            // the stepover, a cut too
            let to = at + vec2(0.0, 3.5);
            runs.line(at, to);
            at = to;
        }
        let drawn = runs.finish();
        assert_eq!(drawn.len(), 1, "one run: nothing broke it");
        assert!(
            points(&drawn) <= 2 * rows + 2,
            "{} points for {rows} rows",
            points(&drawn)
        );
    }

    #[test]
    #[expect(clippy::cast_precision_loss, reason = "small counts")]
    fn a_curve_drawn_thinned_stays_within_half_a_pixel() {
        let view = Rect::from_min_size(Pos2::ZERO, vec2(1000.0, 800.0));
        let mut runs = Runs::new(view, usize::MAX);
        let centre = pos2(400.0, 400.0);
        let on = |i: usize| {
            let a = i as f32 * 0.0005;
            centre + vec2(a.cos(), a.sin()) * (300.0 + 40.0 * (7.0 * a).sin())
        };
        let count = 12_000;
        for i in 0..count {
            runs.line(on(i), on(i + 1));
        }
        let drawn = runs.finish();
        assert!(points(&drawn) < count / 4, "{} points", points(&drawn));
        for i in (0..=count).step_by(37) {
            let gap = off(&drawn, on(i));
            assert!(gap <= 0.5, "point {i} is {gap} px off");
        }
        let last = *drawn.last().unwrap().last().unwrap();
        assert!(
            last.distance(on(count)) < 1e-3,
            "it ends where the cuts end"
        );
    }

    #[test]
    fn cuts_out_of_view_are_left_out_and_break_the_line() {
        let view = Rect::from_min_size(Pos2::ZERO, vec2(100.0, 100.0));
        let mut runs = Runs::new(view, usize::MAX);
        runs.line(pos2(10.0, 10.0), pos2(50.0, 10.0));
        runs.line(pos2(50.0, 10.0), pos2(50.0, 500.0));
        runs.line(pos2(50.0, 500.0), pos2(90.0, 500.0));
        runs.line(pos2(90.0, 500.0), pos2(90.0, 50.0));
        runs.line(pos2(90.0, 50.0), pos2(60.0, 50.0));
        let drawn = runs.finish();
        assert_eq!(
            drawn,
            vec![
                vec![pos2(10.0, 10.0), pos2(50.0, 10.0), pos2(50.0, 500.0)],
                vec![pos2(90.0, 500.0), pos2(90.0, 50.0), pos2(60.0, 50.0)],
            ]
        );
    }

    #[test]
    fn a_gap_between_cuts_starts_a_new_line() {
        let view = Rect::from_min_size(Pos2::ZERO, vec2(100.0, 100.0));
        let mut runs = Runs::new(view, usize::MAX);
        runs.line(pos2(10.0, 10.0), pos2(20.0, 10.0));
        runs.gap();
        runs.line(pos2(20.0, 10.0), pos2(30.0, 10.0));
        // not starting where the last ended: a jump, not a line across
        runs.line(pos2(30.0, 40.0), pos2(40.0, 40.0));
        assert_eq!(
            runs.finish(),
            vec![
                vec![pos2(10.0, 10.0), pos2(20.0, 10.0)],
                vec![pos2(20.0, 10.0), pos2(30.0, 10.0)],
                vec![pos2(30.0, 40.0), pos2(40.0, 40.0)],
            ]
        );
    }

    /// However the cuts fall, a frame draws no more than its share of points.
    #[test]
    #[expect(clippy::cast_precision_loss, reason = "small counts")]
    fn toolpaths_draw_no_more_than_the_points_they_are_given() {
        let view = Rect::from_min_size(Pos2::ZERO, vec2(1000.0, 800.0));
        let mut runs = Runs::new(view, 1000);
        // a zigzag every point of which turns
        for i in 0..100_000 {
            let x = (i % 900) as f32 + 10.0;
            let y = if i % 2 == 0 { 10.0 } else { 700.0 };
            let (tx, ty) = ((i + 1) % 900, if i % 2 == 0 { 700.0 } else { 10.0 });
            runs.line(pos2(x, y), pos2(tx as f32 + 10.0, ty));
        }
        assert!(runs.used() <= 1000, "{}", runs.used());
        assert!(points(&runs.finish()) <= 1000);
    }

    /// A picture wider or taller than the graphics card takes is shrunk to fit, not
    /// refused: a 16 604 pixel scan crashed the app (wgpu's limit was 8192).
    #[test]
    fn pictures_fit_the_graphics_card() {
        let wide = egui::ColorImage::new([16_604, 6], vec![egui::Color32::RED; 16_604 * 6]);
        let fitted = within(wide, 8192);
        assert!(
            fitted.size[0] <= 8192 && fitted.size[0] > 4000,
            "{:?}",
            fitted.size
        );
        assert_eq!(fitted.size[1], 2, "both sides shrink alike");
        assert_eq!(fitted.pixels.len(), fitted.size[0] * fitted.size[1]);
        assert_eq!(fitted.pixels[0], egui::Color32::RED);
        let small = egui::ColorImage::new([10, 10], vec![egui::Color32::BLUE; 100]);
        assert_eq!(
            within(small.clone(), 8192),
            small,
            "one that fits is left alone"
        );
    }

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
