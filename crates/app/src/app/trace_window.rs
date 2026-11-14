//! Trace Bitmap's trace window: the settings beside a large preview of what the trace makes
//! (before and after, the result alone, or its outlines or middle lines over the picture), traced again as
//! the settings change (Live updates) or when Update preview is clicked, off the window's
//! thread and from a smaller copy of a big picture. In colour mode it lists the picture's
//! colours in the groups they are traced as, to tick, merge and separate.

use super::CutCraftApp;
use crate::form;
use cmd_trace_bitmap::colours::{Swatches, hex};
use cmd_trace_bitmap::{Colours, Mode, Params, PictureType, Traced};
use cut_craft_bug_report::hint::HoverHint;
use cut_craft_core::bitmap::Bitmap;
use cut_craft_core::geometry::{Point, Rect};
use eframe::egui;
use serde_json::Value;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, TryRecvError};
use std::time::{Duration, Instant};

/// The most pixels the preview traces: a bigger picture is previewed from a copy this size
/// (Trace uses the whole of it).
pub(super) const PREVIEW_PIXELS: usize = 1_000_000;

/// How long settings must stay unchanged before a live preview traces again, so a dragged
/// slider traces once it rests, not on every step.
pub(super) const SETTLE: Duration = Duration::from_millis(200);

/// The result picture's longest edge, in pixels.
const DRAWN_EDGE: usize = 1200;

/// The settings column's width, in points.
const SETTINGS_WIDTH: f32 = 300.0;

/// How the preview shows the trace.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum Look {
    /// The picture and the result side by side.
    #[default]
    BeforeAndAfter,
    /// The result alone, as large as it goes.
    Result,
    /// The result's outlines over the picture, faded as the slider says.
    Outlines,
}

impl Look {
    const ALL: [Self; 3] = [Self::BeforeAndAfter, Self::Result, Self::Outlines];

    fn label(self) -> &'static str {
        match self {
            Self::BeforeAndAfter => "Before and after",
            Self::Result => "Result only",
            Self::Outlines => "Outlines over picture",
        }
    }
}

/// How the colour list is ordered.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum Sort {
    /// By how light they are, darkest first, so alike colours sit together.
    #[default]
    Similarity,
    /// Most of the picture first.
    MostUsed,
}

/// The chosen picture as the window shows and traces it.
struct Picture {
    key: u64,
    /// The picture, or (past [`PREVIEW_PIXELS`]) a smaller copy of it, and how many of the
    /// picture's pixels make one of its across.
    traced: Arc<Bitmap>,
    scale: f64,
    /// The whole picture's colours, once worked out.
    swatches: Option<Arc<Swatches>>,
    texture: egui::TextureHandle,
}

/// What a preview trace made.
struct Made {
    params: Value,
    result: Result<Vec<Traced>, String>,
    drawn: Option<egui::ColorImage>,
    colours: Option<Colours>,
    swatches: Option<Arc<Swatches>>,
}

/// The last preview shown.
struct Shown {
    params: Value,
    result: Result<Vec<Traced>, String>,
    colours: Option<Colours>,
    texture: Option<egui::TextureHandle>,
}

/// The trace window's state while Trace Bitmap is open.
pub(super) struct TraceView {
    pub(super) open: bool,
    /// Trace again as the settings change; off, only when Update preview is clicked.
    pub(super) live: bool,
    pub(super) look: Look,
    /// How strongly the picture shows under the outlines, 0 to 1.
    pub(super) fade: f32,
    /// Zoom (1 fits the picture) and pan, in points.
    pub(super) zoom: f32,
    pub(super) pan: egui::Vec2,
    pub(super) sort: Sort,
    /// Swatches picked in the colour list or the preview, as `#rrggbb`.
    pub(super) selected: Vec<String>,
    picture: Option<Picture>,
    /// When the settings were first seen to differ from the preview's.
    changed: Option<Instant>,
    /// Update preview clicked: trace once the running trace is done.
    asked: bool,
    running: Option<(Value, Receiver<Made>)>,
    shown: Option<Shown>,
}

impl Default for TraceView {
    fn default() -> Self {
        Self {
            open: true,
            live: true,
            look: Look::default(),
            fade: 0.5,
            zoom: 1.0,
            pan: egui::Vec2::ZERO,
            sort: Sort::default(),
            selected: vec![],
            picture: None,
            changed: None,
            asked: false,
            running: None,
            shown: None,
        }
    }
}

impl TraceView {
    #[cfg(test)]
    /// How many colours the preview shown traced, once it has traced (an error: 0).
    pub(super) fn traced_colours(&self) -> Option<usize> {
        self.shown
            .as_ref()
            .map(|s| s.result.as_ref().map_or(0, Vec::len))
    }

    #[cfg(test)]
    /// The settings the preview shown was traced with.
    pub(super) fn shown_params(&self) -> Option<&Value> {
        self.shown.as_ref().map(|s| &s.params)
    }
}

/// A cheap mark of a picture's size, place and pixels, to notice another one chosen.
pub(super) fn fingerprint(b: &Bitmap) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    (b.columns, b.rows).hash(&mut h);
    for v in [b.rect.min.x, b.rect.min.y, b.rect.max.x, b.rect.max.y] {
        v.to_bits().hash(&mut h);
    }
    for bytes in [&b.grey, &b.rgb] {
        bytes.len().hash(&mut h);
        bytes.iter().step_by(997).for_each(|v| v.hash(&mut h));
    }
    h.finish()
}

/// How a preview pane shows the picture's rectangle: fitted to `pane`, `zoom` times as big,
/// moved by `pan`.
#[derive(Clone, Copy, Debug)]
pub(super) struct PaneMap {
    centre: egui::Pos2,
    /// Points a millimetre.
    scale: f32,
    middle: Point,
}

impl PaneMap {
    #[expect(clippy::cast_possible_truncation, reason = "screen points")]
    pub(super) fn new(pane: egui::Rect, rect: Rect, zoom: f32, pan: egui::Vec2) -> Self {
        let size = rect.size();
        let fit = (f64::from(pane.width()) / size.x).min(f64::from(pane.height()) / size.y);
        Self {
            centre: pane.center() + pan,
            scale: fit as f32 * zoom,
            middle: rect.center(),
        }
    }

    #[expect(clippy::cast_possible_truncation, reason = "screen points")]
    pub(super) fn to_screen(self, p: Point) -> egui::Pos2 {
        let d = p - self.middle;
        self.centre + egui::vec2(d.x as f32, -d.y as f32) * self.scale
    }

    pub(super) fn to_job(self, at: egui::Pos2) -> Point {
        let d = (at - self.centre) / self.scale;
        self.middle + Point::new(f64::from(d.x), -f64::from(d.y))
    }
}

/// Whether to start a preview trace now: the settings differ from those shown and being
/// traced, nothing is running, and either Update preview was clicked or (live) they have
/// rested for [`SETTLE`].
#[expect(
    clippy::fn_params_excessive_bools,
    reason = "each is one thing the window knows"
)]
pub(super) fn trace_now(
    live: bool,
    asked: bool,
    differ: bool,
    running: bool,
    changed: Option<Instant>,
    now: Instant,
) -> bool {
    !running && (asked || (live && differ && changed.is_some_and(|t| now - t >= SETTLE)))
}

/// Traces `params` off `picture` on a thread of its own; `ctx` is woken when it is done.
fn start(picture: &Picture, params: Value, ctx: &egui::Context) -> Option<(Value, Receiver<Made>)> {
    let (tx, rx) = std::sync::mpsc::channel();
    let (traced, scale) = (Arc::clone(&picture.traced), picture.scale);
    let swatches = picture.swatches.clone();
    let (asked, wake) = (params.clone(), ctx.clone());
    // the colours come from the copy traced: it only holds colours the picture has (its
    // pixels are picked, not blended), so its swatches are near the whole picture's
    let spawned = std::thread::Builder::new()
        .name("trace preview".into())
        .spawn(move || {
            let made = preview(&traced, scale, swatches, asked);
            let _ = tx.send(made);
            wake.request_repaint();
        });
    spawned.ok().map(|_| (params, rx))
}

/// The preview of `params` traced off `b`: its shapes, drawn, and (colour mode) its colours.
fn preview(b: &Bitmap, scale: f64, swatches: Option<Arc<Swatches>>, params: Value) -> Made {
    let p: Params = match serde_json::from_value(params.clone()) {
        Ok(p) => p,
        Err(e) => {
            return Made {
                params,
                result: Err(e.to_string()),
                drawn: None,
                colours: None,
                swatches,
            };
        }
    };
    let coloured = p.mode == Mode::Colors && b.rgb.len() == b.columns * b.rows * 3;
    let swatches = swatches.or_else(|| coloured.then(|| Arc::new(Swatches::of(&b.rgb))));
    let colours = swatches
        .as_deref()
        .filter(|_| coloured)
        .and_then(|s| Colours::of(s, b, &p).ok());
    let result =
        cmd_trace_bitmap::traced(b, swatches.as_deref(), &p, scale).map_err(|e| e.to_string());
    let drawn = result.as_ref().ok().map(|shapes| {
        let (w, h) = drawn_size(b.rect);
        let rgba = cmd_trace_bitmap::drawn(shapes, b.rect, w, h);
        egui::ColorImage::from_rgba_unmultiplied([w, h], &rgba)
    });
    Made {
        params,
        result,
        drawn,
        colours,
        swatches,
    }
}

/// The result picture's size for a picture on `rect`: [`DRAWN_EDGE`] along its longer side.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "pixel counts"
)]
fn drawn_size(rect: Rect) -> (usize, usize) {
    let size = rect.size();
    let long = size.x.max(size.y).max(f64::EPSILON);
    let edge = DRAWN_EDGE as f64;
    (
        ((size.x / long * edge).round() as usize).max(1),
        ((size.y / long * edge).round() as usize).max(1),
    )
}

/// The shapes' and nodes' count, for the details line.
fn counts(shapes: &[Traced]) -> (usize, usize) {
    let paths = shapes.iter().flat_map(|t| &t.paths);
    (paths.clone().count(), paths.map(|p| p.segments.len()).sum())
}

impl CutCraftApp {
    /// Trace Bitmap's side form: the settings are in the trace window, which this reopens.
    pub(super) fn trace_form(&mut self, ui: &mut egui::Ui) {
        ui.label("The settings and a preview of the trace are in the trace window.");
        let view = self.trace_view.get_or_insert_with(TraceView::default);
        if !view.open
            && ui
                .button("Show Trace Window")
                .on_hover_hint("Open the window with the trace settings and preview")
                .clicked()
        {
            view.open = true;
        }
        self.trace_window(ui.ctx());
    }

    /// The trace window, while Trace Bitmap's form is open; its preview kept up to date.
    pub(super) fn trace_window(&mut self, ctx: &egui::Context) {
        if !self.tracing() {
            self.trace_view = None;
            return;
        }
        let mut view = self.trace_view.take().unwrap_or_default();
        self.follow_picture(&mut view, ctx);
        self.follow_preview(&mut view, ctx);
        if view.open {
            let mut open = true;
            let mut pressed = None;
            egui::Window::new("Trace Bitmap")
                .id(egui::Id::unique("trace_window"))
                .open(&mut open)
                .collapsible(false)
                .resizable(true)
                .default_size([980.0, 640.0])
                .min_size([640.0, 420.0])
                .show(ctx, |ui| pressed = self.trace_window_body(ui, &mut view));
            view.open = open;
            match pressed {
                Some(Press::Apply) => {
                    if let Some(form) = &self.form {
                        let params = form.params.clone();
                        let result = self.apply_form("trace_bitmap", &params);
                        if let Some(form) = &mut self.form {
                            form.error = result.err();
                        }
                    }
                }
                Some(Press::Close) => {
                    if let Some(form) = self.form.take() {
                        self.pages.done_with(form.page);
                    }
                    return;
                }
                None => {}
            }
        }
        self.trace_view = Some(view);
    }

    /// The chosen picture, and its smaller copy and picture on the graphics card, made again
    /// when another is chosen.
    fn follow_picture(&self, view: &mut TraceView, ctx: &egui::Context) {
        let Some(b) = self.doc.chosen_bitmap() else {
            view.picture = None;
            return;
        };
        let key = fingerprint(b);
        if view.picture.as_ref().is_some_and(|p| p.key == key) {
            return;
        }
        let (traced, scale) =
            cmd_trace_bitmap::smaller(b, PREVIEW_PIXELS).unwrap_or_else(|| (b.clone(), 1.0));
        let size = [traced.columns, traced.rows];
        let image = if traced.rgb.len() == traced.columns * traced.rows * 3 {
            egui::ColorImage::from_rgb(size, &traced.rgb)
        } else {
            egui::ColorImage::from_gray(size, &traced.grey)
        };
        let texture = ctx.load_texture("trace picture", image, egui::TextureOptions::LINEAR);
        view.picture = Some(Picture {
            key,
            traced: Arc::new(traced),
            scale,
            swatches: None,
            texture,
        });
        view.shown = None;
        view.asked = true;
    }

    /// Takes in a finished preview, and starts the next when the settings ask for one.
    fn follow_preview(&self, view: &mut TraceView, ctx: &egui::Context) {
        if let Some((_, rx)) = &view.running {
            match rx.try_recv() {
                Ok(made) => {
                    view.running = None;
                    if let (Some(picture), Some(s)) = (&mut view.picture, &made.swatches) {
                        picture.swatches.get_or_insert_with(|| Arc::clone(s));
                    }
                    let texture = made.drawn.map(|image| {
                        ctx.load_texture("trace result", image, egui::TextureOptions::LINEAR)
                    });
                    view.shown = Some(Shown {
                        params: made.params,
                        result: made.result,
                        colours: made.colours,
                        texture,
                    });
                }
                Err(TryRecvError::Disconnected) => view.running = None,
                Err(TryRecvError::Empty) => {}
            }
        }
        let (Some(form), Some(picture)) = (&self.form, &view.picture) else {
            return;
        };
        let params = &form.params;
        let differ = view.shown.as_ref().is_none_or(|s| s.params != *params)
            && view.running.as_ref().is_none_or(|(p, _)| p != params);
        let now = Instant::now();
        if differ {
            view.changed.get_or_insert(now);
        } else {
            view.changed = None;
        }
        let running = view.running.is_some();
        if trace_now(view.live, view.asked, differ, running, view.changed, now) {
            view.asked = false;
            view.changed = None;
            view.running = start(picture, params.clone(), ctx);
        } else if differ && view.live && !running {
            ctx.request_repaint_after(SETTLE);
        }
    }

    /// The window's contents: the preview, and the settings beside it; which button was
    /// pressed.
    fn trace_window_body(&mut self, ui: &mut egui::Ui, view: &mut TraceView) -> Option<Press> {
        let mut pressed = None;
        // the window as it was sized, not the whole screen it could grow to
        let screen = ui.ctx().content_rect().size();
        let full = ui
            .available_size()
            .min(egui::vec2(screen.x * 0.8, screen.y * 0.8));
        ui.horizontal_top(|ui| {
            let size = egui::vec2(
                (full.x - SETTINGS_WIDTH - ui.spacing().item_spacing.x).max(200.0),
                full.y.max(300.0),
            );
            ui.vertical(|ui| {
                ui.set_width(size.x);
                self.preview_bar(ui, view);
                let foot = ui.spacing().interact_size.y + ui.spacing().item_spacing.y;
                let left = ui.available_size_before_wrap();
                let pane = egui::vec2(size.x, (left.y.min(size.y) - foot).max(200.0));
                preview_panes(ui, view, pane);
                details(ui, view);
            });
            ui.separator();
            ui.vertical(|ui| {
                ui.set_width(SETTINGS_WIDTH);
                let foot = 3.0 * ui.spacing().interact_size.y;
                egui::ScrollArea::vertical()
                    .max_height((full.y - foot).max(100.0))
                    .show(ui, |ui| self.trace_settings(ui, view));
                ui.separator();
                if let Some(error) = self.form.as_ref().and_then(|f| f.error.as_ref()) {
                    ui.colored_label(ui.visuals().error_fg_color, error);
                }
                ui.horizontal(|ui| {
                    if ui
                        .button("Trace")
                        .on_hover_hint("Trace the whole picture into the job with these settings")
                        .clicked()
                    {
                        pressed = Some(Press::Apply);
                    }
                    if ui
                        .button("Close")
                        .on_hover_hint("Close Trace Bitmap, keeping what was applied")
                        .clicked()
                    {
                        pressed = Some(Press::Close);
                    }
                });
            });
        });
        pressed
    }

    /// Over the preview: how it shows, zoom, Live updates and Update preview.
    fn preview_bar(&self, ui: &mut egui::Ui, view: &mut TraceView) {
        ui.horizontal_wrapped(|ui| {
            ui.label("Preview:");
            egui::ComboBox::from_id_salt("trace_look")
                .selected_text(view.look.label())
                .show_ui(ui, |ui| {
                    for look in Look::ALL {
                        ui.selectable_value(&mut view.look, look, look.label());
                    }
                });
            if view.look == Look::Outlines {
                ui.add(egui::Slider::new(&mut view.fade, 0.0..=1.0).text("Picture"))
                    .on_hover_hint("How strongly the picture shows under the outlines");
            }
            if ui.button("−").on_hover_hint("Zoom out").clicked() {
                view.zoom = (view.zoom / 1.5).max(0.25);
            }
            if ui.button("+").on_hover_hint("Zoom in").clicked() {
                view.zoom = (view.zoom * 1.5).min(64.0);
            }
            if ui
                .button("Fit")
                .on_hover_hint("Fit the whole picture in the preview")
                .clicked()
            {
                (view.zoom, view.pan) = (1.0, egui::Vec2::ZERO);
            }
            ui.separator();
            ui.checkbox(&mut view.live, "Live updates").on_hover_hint(
                "Trace again as the settings change; untick on a slow computer and use \
                 Update preview",
            );
            let current = view
                .shown
                .as_ref()
                .zip(self.form.as_ref())
                .is_some_and(|(s, f)| s.params == f.params);
            if ui
                .add_enabled(
                    !current && view.running.is_none(),
                    egui::Button::new("Update preview"),
                )
                .on_hover_hint("Trace the preview again with the settings as they are")
                .on_disabled_hover_text("The preview shows these settings")
                .clicked()
            {
                view.asked = true;
            }
            if view.running.is_some() {
                ui.spinner();
            }
        });
    }

    /// The settings column: picture type, the form's fields, the colour list, the details
    /// line and Reset.
    fn trace_settings(&mut self, ui: &mut egui::Ui, view: &mut TraceView) {
        let units = self.doc.job.units;
        let material = form::material(&self.doc, units);
        let quick = self
            .doc
            .chosen_bitmap()
            .map(cmd_trace_bitmap::quick_settings);
        let Some(open) = self.form.as_mut() else {
            return;
        };
        ui.horizontal(|ui| {
            ui.label("Picture type");
            egui::ComboBox::from_id_salt("trace_picture_type")
                .selected_text("Choose…")
                .show_ui(ui, |ui| {
                    for kind in PictureType::ALL {
                        if ui
                            .selectable_label(false, kind.label())
                            .on_hover_hint(kind.hint())
                            .clicked()
                            && let Ok(p) = serde_json::from_value::<Params>(open.params.clone())
                            && let Ok(v) = serde_json::to_value(kind.settings(&p))
                        {
                            open.params = v;
                            open.dirty = true;
                            view.selected.clear();
                        }
                    }
                });
            if ui
                .add_enabled(quick.is_some(), egui::Button::new("Quick Trace"))
                .on_hover_hint(
                    "Let Trace Bitmap look at the picture and choose the settings: its colours \
                     when it has some, else black and white split where it suits it best",
                )
                .clicked()
                && let Some(q) = &quick
                && let Ok(p) = serde_json::from_value::<Params>(open.params.clone())
                && let Ok(v) = serde_json::to_value(Params {
                    area: p.area,
                    ..q.clone()
                })
            {
                open.params = v;
                open.dirty = true;
                view.selected.clear();
            }
            if ui
                .button("Reset")
                .on_hover_hint("Back to the settings Trace Bitmap starts with")
                .clicked()
                && let Ok(p) = serde_json::from_value::<Params>(open.params.clone())
                && let Ok(v) = serde_json::to_value(Params {
                    area: p.area,
                    ..Params::default()
                })
            {
                open.params = v;
                open.dirty = true;
                view.selected.clear();
            }
        })
        .response
        .on_hover_hint("Fill in settings that suit the kind of picture traced");
        ui.separator();
        let none = |_: &str| None;
        let look = form::Look {
            units,
            picture: &none,
            material,
            tool: None,
            group_end: None,
        };
        open.dirty |= form::show(ui, &open.fields, &mut open.params, &look);
        let Ok(p) = serde_json::from_value::<Params>(open.params.clone()) else {
            return;
        };
        if p.mode == Mode::Colors {
            ui.separator();
            if let Some(params) = colour_list(ui, view, &p) {
                open.params = params;
                open.dirty = true;
            }
        }
    }
}

/// The preview panes, `size` in all: zoomed with the wheel, panned by dragging, a click
/// picking the colour under it.
fn preview_panes(ui: &mut egui::Ui, view: &mut TraceView, size: egui::Vec2) {
    let (whole, _) = ui.allocate_exact_size(size, egui::Sense::hover());
    let Some(picture) = &view.picture else {
        ui.painter_at(whole).text(
            whole.center(),
            egui::Align2::CENTER_CENTER,
            "Import a picture to trace",
            egui::FontId::proportional(16.0),
            ui.visuals().weak_text_color(),
        );
        return;
    };
    let panes = pane_rects(view.look, whole, ui.spacing().item_spacing.x);
    let rect = picture.traced.rect;
    let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
    let mut clicked = None;
    for (k, &(pane, after)) in panes.iter().enumerate() {
        let response = ui.interact(
            pane,
            egui::Id::unique(("trace_pane", k)),
            egui::Sense::click_and_drag(),
        );
        steer(ui, (&mut view.zoom, &mut view.pan), &response, pane);
        let map = PaneMap::new(pane, rect, view.zoom, view.pan);
        if response.clicked()
            && let Some(at) = response.interact_pointer_pos()
        {
            clicked = Some(map.to_job(at));
        }
        let painter = ui.painter_at(pane);
        painter.rect_filled(pane, 0.0, ui.visuals().extreme_bg_color);
        let at = egui::Rect::from_two_pos(map.to_screen(rect.min), map.to_screen(rect.max));
        let shown = view.shown.as_ref();
        match (after, view.look) {
            (false, _) => {
                painter.image(picture.texture.id(), at, uv, egui::Color32::WHITE);
            }
            (true, Look::Outlines) => {
                let shapes = shown.and_then(|s| s.result.as_ref().ok());
                paint_outlines(&painter, picture, shapes, map, view.fade, at);
            }
            (true, _) => {
                if let Some(texture) = shown.and_then(|s| s.texture.as_ref()) {
                    painter.image(texture.id(), at, uv, egui::Color32::WHITE);
                } else {
                    let note = match shown.map(|s| &s.result) {
                        Some(Err(e)) => e.clone(),
                        _ if view.running.is_some() => "Tracing…".into(),
                        _ => "Click Update preview to see the trace".into(),
                    };
                    painter.text(
                        pane.center(),
                        egui::Align2::CENTER_CENTER,
                        note,
                        egui::FontId::proportional(15.0),
                        ui.visuals().weak_text_color(),
                    );
                }
            }
        }
        // a failed trace says why over the last good picture
        if let (true, Some(Err(e))) = (after, shown.map(|s| &s.result)) {
            painter.text(
                pane.center_top() + egui::vec2(0.0, 8.0),
                egui::Align2::CENTER_TOP,
                e,
                egui::FontId::proportional(14.0),
                ui.visuals().error_fg_color,
            );
        }
        if let Some(colours) = shown.and_then(|s| s.colours.as_ref()) {
            mark_selected(&painter, view, colours, map, picture);
        }
    }
    let shift = ui.input(|i| i.modifiers.shift || i.modifiers.command);
    if let (Some(at), Some(colours)) = (
        clicked,
        view.shown.as_ref().and_then(|s| s.colours.as_ref()),
    ) && let Some(g) = cmd_trace_bitmap::group_at(&picture.traced, colours, at)
    {
        let names = colours.names[g].clone();
        pick(&mut view.selected, &names, shift);
    }
}

/// Where each preview pane goes in `whole`, and whether it shows the result (else the
/// picture).
fn pane_rects(look: Look, whole: egui::Rect, gap: f32) -> Vec<(egui::Rect, bool)> {
    match look {
        Look::BeforeAndAfter => {
            let half = (whole.width() - gap) / 2.0;
            let size = egui::vec2(half, whole.height());
            vec![
                (egui::Rect::from_min_size(whole.min, size), false),
                (
                    egui::Rect::from_min_size(whole.min + egui::vec2(half + gap, 0.0), size),
                    true,
                ),
            ]
        }
        Look::Result | Look::Outlines => vec![(whole, true)],
    }
}

/// A drag on a pane pans the preview, the wheel over it zooms about the pointer.
fn steer(
    ui: &egui::Ui,
    (zoom, pan): (&mut f32, &mut egui::Vec2),
    response: &egui::Response,
    pane: egui::Rect,
) {
    if response.dragged() {
        *pan += response.drag_delta();
    }
    if !response.hovered() {
        return;
    }
    let scroll = ui.input(|i| i.smooth_scroll_delta.y);
    if scroll == 0.0 {
        return;
    }
    let before = *zoom;
    *zoom = (*zoom * (scroll * 0.002).exp()).clamp(0.25, 64.0);
    // the point under the pointer stays there
    if let Some(at) = response.hover_pos() {
        let from = at - pane.center() - *pan;
        *pan -= from * (*zoom / before - 1.0);
    }
}

/// The picked colours' pixels marked on the preview, so it is plain where they are.
fn mark_selected(
    painter: &egui::Painter,
    view: &TraceView,
    colours: &Colours,
    map: PaneMap,
    picture: &Picture,
) {
    // only as a small dot per marked pixel when zoomed far enough to see them apart
    let b = &picture.traced;
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_precision_loss,
        reason = "points on screen"
    )]
    let pixel = map.scale * (b.rect.size().x / b.columns.max(1) as f64) as f32;
    if view.selected.is_empty() || pixel < 3.0 {
        return;
    }
    let marked: Vec<bool> = colours
        .names
        .iter()
        .map(|names| names.iter().any(|n| view.selected.contains(n)))
        .collect();
    let clip = painter.clip_rect();
    let colour = painter.ctx().global_style().visuals.selection.stroke.color;
    #[expect(clippy::cast_precision_loss, reason = "pixel counts")]
    for r in 0..b.rows {
        for c in 0..b.columns {
            if !marked[colours.labels[r * b.columns + c]] {
                continue;
            }
            let size = b.rect.size();
            let at = map.to_screen(Point::new(
                b.rect.min.x + (c as f64 + 0.5) / b.columns as f64 * size.x,
                b.rect.max.y - (r as f64 + 0.5) / b.rows as f64 * size.y,
            ));
            if clip.contains(at) {
                painter.circle_filled(at, 1.0, colour);
            }
        }
    }
}

/// The result's outlines over the picture, which shows `fade` strongly (0 to 1) at `at`.
fn paint_outlines(
    painter: &egui::Painter,
    picture: &Picture,
    shapes: Option<&Vec<Traced>>,
    map: PaneMap,
    fade: f32,
    at: egui::Rect,
) {
    let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "clamped to 0..=255"
    )]
    let tint = egui::Color32::from_white_alpha((fade.clamp(0.0, 1.0) * 255.0).round() as u8);
    painter.rect_filled(at, 0.0, egui::Color32::WHITE);
    painter.image(picture.texture.id(), at, uv, tint);
    let Some(shapes) = shapes else { return };
    // a colour few pictures have, to stand out over any of them
    let stroke = egui::Stroke::new(1.5, egui::Color32::from_rgb(235, 0, 170));
    let tolerance = 0.5 / f64::from(map.scale.max(0.01));
    for path in shapes.iter().flat_map(|t| &t.paths) {
        let mut points: Vec<egui::Pos2> = path
            .flatten(tolerance)
            .into_iter()
            .map(|p| map.to_screen(p))
            .collect();
        if path.closed
            && let Some(&first) = points.first()
        {
            points.push(first);
        }
        painter.add(egui::Shape::line(points, stroke));
    }
}

/// Which window button was pressed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Press {
    Apply,
    Close,
}

/// `names` (a group's swatches, or one swatch) picked: alone, or with Shift or Ctrl added to
/// (or, all picked already, taken out of) those picked.
pub(super) fn pick(selected: &mut Vec<String>, names: &[String], add: bool) {
    let all_in = names.iter().all(|n| selected.contains(n));
    if !add {
        selected.clear();
    }
    if add && all_in {
        selected.retain(|n| !names.contains(n));
    } else {
        for n in names {
            if !selected.contains(n) {
                selected.push(n.clone());
            }
        }
    }
}

/// The colour list: each group traced as one colour, its swatches beside a tick for whether
/// it is traced; Merge and Separate for the swatches picked. The parameters changed, if any.
fn colour_list(ui: &mut egui::Ui, view: &mut TraceView, p: &Params) -> Option<Value> {
    let mut changed: Option<Params> = None;
    ui.horizontal(|ui| {
        ui.strong("Colors");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.selectable_value(&mut view.sort, Sort::MostUsed, "Most used")
                .on_hover_hint("The colours covering most of the picture first");
            ui.selectable_value(&mut view.sort, Sort::Similarity, "Similar")
                .on_hover_hint("Alike colours next to each other, darkest first");
        });
    });
    let (Some(shown), Some(picture)) = (view.shown.as_ref(), view.picture.as_ref()) else {
        ui.weak("The colours show once the preview is traced");
        return None;
    };
    let (Some(colours), Some(swatches)) = (shown.colours.as_ref(), picture.swatches.as_ref())
    else {
        ui.weak("The colours show once the preview is traced");
        return None;
    };
    let total: usize = colours.grouping.groups.iter().map(|g| g.pixels).sum();
    let traced = colours.chosen(p);
    let mut order: Vec<usize> = (0..colours.grouping.groups.len()).collect();
    if view.sort == Sort::MostUsed {
        order.sort_by_key(|&g| std::cmp::Reverse(colours.grouping.groups[g].pixels));
    }
    ui.weak(format!(
        "{} colours in {} groups. Click swatches to pick them (Shift or Ctrl for more), then \
         Merge; tick the colours to trace",
        swatches.list.len(),
        colours.grouping.groups.len()
    ));
    let edge = ui.spacing().interact_size.y * 0.8;
    let shift = ui.input(|i| i.modifiers.shift || i.modifiers.command);
    for g in order {
        let group = &colours.grouping.groups[g];
        ui.horizontal_wrapped(|ui| {
            let mut on = traced.contains(&g);
            if ui
                .checkbox(&mut on, "")
                .on_hover_hint("Trace this colour")
                .changed()
            {
                changed = Some(Params {
                    trace: Some(colours.ticked(p, g, on)),
                    ..p.clone()
                });
            }
            #[expect(clippy::cast_precision_loss, reason = "pixel counts")]
            let share = group.pixels as f64 * 100.0 / total.max(1) as f64;
            let note = if g == colours.background {
                " background"
            } else if colours.blends[g] {
                " edge blend"
            } else {
                ""
            };
            ui.weak(format!("{share:4.1}%{note}"))
                .on_hover_text(match note {
                    " background" => {
                        "The colour round the picture's edges: not traced unless ticked"
                    }
                    " edge blend" => {
                        "In-between colours along the edges of others: traced with the colours \
                     beside them unless ticked"
                    }
                    _ => "How much of the picture this colour covers",
                });
            for &k in &group.swatches {
                let c = swatches.list[k].colour;
                let name = hex(c);
                let picked = view.selected.contains(&name);
                let (rect, response) =
                    ui.allocate_exact_size(egui::vec2(edge, edge), egui::Sense::click());
                ui.painter()
                    .rect_filled(rect, 2.0, egui::Color32::from_rgb(c[0], c[1], c[2]));
                let stroke = if picked {
                    egui::Stroke::new(2.5, ui.visuals().selection.stroke.color)
                } else {
                    egui::Stroke::new(1.0, ui.visuals().widgets.noninteractive.bg_stroke.color)
                };
                ui.painter()
                    .rect_stroke(rect, 2.0, stroke, egui::StrokeKind::Outside);
                #[expect(clippy::cast_precision_loss, reason = "pixel counts")]
                let share = swatches.list[k].pixels as f64 * 100.0 / total.max(1) as f64;
                if response
                    .on_hover_text(format!("{name}: {share:.1}% of the picture"))
                    .clicked()
                {
                    pick(&mut view.selected, &[name], shift);
                }
            }
        });
    }
    if let Some(merged) = merge_buttons(ui, view, p) {
        changed = Some(Params {
            merged,
            ..changed.unwrap_or_else(|| p.clone())
        });
    }
    changed.and_then(|p| serde_json::to_value(p).ok())
}

/// Merge, Separate and Pick None under the colour list; the merged sets changed, if any.
fn merge_buttons(ui: &mut egui::Ui, view: &mut TraceView, p: &Params) -> Option<Vec<Vec<String>>> {
    let merged_picked = p
        .merged
        .iter()
        .any(|set| set.iter().any(|h| view.selected.contains(h)));
    let mut merged = None;
    ui.horizontal(|ui| {
        if ui
            .add_enabled(view.selected.len() > 1, egui::Button::new("Merge"))
            .on_hover_hint("Trace the picked swatches as one colour, their average")
            .on_disabled_hover_text("Pick two swatches or more to merge")
            .clicked()
        {
            merged = Some(cmd_trace_bitmap::colours::merge(&p.merged, &view.selected));
            view.selected.clear();
        }
        if ui
            .add_enabled(merged_picked, egui::Button::new("Separate"))
            .on_hover_hint("Take the picked swatches out of what they were merged with")
            .on_disabled_hover_text("Pick a swatch that was merged by hand")
            .clicked()
        {
            merged = Some(cmd_trace_bitmap::colours::separate(
                &p.merged,
                &view.selected,
            ));
            view.selected.clear();
        }
        if ui
            .add_enabled(!view.selected.is_empty(), egui::Button::new("Pick None"))
            .clicked()
        {
            view.selected.clear();
        }
    });
    merged
}

/// What the preview's trace made: shapes, nodes and colours, and whether it was traced off
/// a smaller copy.
fn details(ui: &mut egui::Ui, view: &TraceView) {
    let Some(shown) = &view.shown else {
        ui.weak("Not traced yet");
        return;
    };
    match &shown.result {
        Ok(shapes) => {
            let (paths, nodes) = counts(shapes);
            let what = if shapes.iter().any(|t| t.strokes) {
                "lines"
            } else {
                "shapes"
            };
            let mut line = format!("{paths} {what}, {nodes} nodes");
            if shown.colours.is_some() {
                use std::fmt::Write as _;
                let _ = write!(line, ", {} colours traced", shapes.len());
            }
            ui.label(line);
        }
        Err(e) => {
            ui.colored_label(ui.visuals().error_fg_color, e);
        }
    }
    if view.picture.as_ref().is_some_and(|p| p.scale > 1.0) {
        ui.weak("Big picture: previewed from a smaller copy. Trace uses the whole picture");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_live_preview_waits_for_the_settings_to_rest_and_one_trace_at_a_time() {
        let now = Instant::now();
        let then = now.checked_sub(SETTLE).unwrap();
        // just changed: not yet
        assert!(!trace_now(true, false, true, false, Some(now), now));
        assert!(trace_now(true, false, true, false, Some(then), now));
        // one already running: after it
        assert!(!trace_now(true, true, true, true, Some(then), now));
        // not live: only when asked
        assert!(!trace_now(false, false, true, false, Some(then), now));
        assert!(trace_now(false, true, true, false, None, now));
    }

    #[test]
    fn panes_fit_the_picture_and_map_back_and_forth() {
        let pane = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(200.0, 100.0));
        let rect = Rect::new(Point::ZERO, Point::new(50.0, 50.0));
        let map = PaneMap::new(pane, rect, 1.0, egui::Vec2::ZERO);
        // 50 mm tall in 100 points: the top left corner at (50, 0)
        let corner = map.to_screen(Point::new(0.0, 50.0));
        assert!(
            (corner - egui::pos2(50.0, 0.0)).length() < 1e-3,
            "{corner:?}"
        );
        let back = map.to_job(egui::pos2(120.0, 30.0));
        assert!((map.to_screen(back) - egui::pos2(120.0, 30.0)).length() < 1e-3);
        let zoomed = PaneMap::new(pane, rect, 2.0, egui::vec2(10.0, 0.0));
        let middle = zoomed.to_screen(Point::new(25.0, 25.0));
        assert!((middle - egui::pos2(110.0, 50.0)).length() < 1e-3);
    }

    #[test]
    fn picking_swatches_alone_or_added_with_shift() {
        let mut picked = vec![];
        pick(&mut picked, &["#000000".into()], false);
        pick(&mut picked, &["#ffffff".into()], true);
        assert_eq!(picked, vec!["#000000".to_string(), "#ffffff".into()]);
        // shift on one picked already lets it go
        pick(&mut picked, &["#000000".into()], true);
        assert_eq!(picked, vec!["#ffffff".to_string()]);
        // a plain click picks it alone
        pick(&mut picked, &["#123456".into()], false);
        assert_eq!(picked, vec!["#123456".to_string()]);
    }

    #[test]
    fn another_picture_has_another_fingerprint() {
        let b = Bitmap {
            name: "a".into(),
            columns: 2,
            rows: 1,
            grey: vec![0, 255],
            rect: Rect::new(Point::ZERO, Point::new(2.0, 1.0)),
            depth: Vec::new(),
            rgb: Vec::new(),
        };
        let mut other = b.clone();
        other.grey[0] = 10;
        assert_eq!(fingerprint(&b), fingerprint(&b.clone()));
        assert_ne!(fingerprint(&b), fingerprint(&other));
    }

    #[test]
    fn the_result_is_drawn_at_the_pictures_shape() {
        let (w, h) = drawn_size(Rect::new(Point::ZERO, Point::new(200.0, 100.0)));
        assert_eq!((w, h), (DRAWN_EDGE, DRAWN_EDGE / 2));
    }
}
