//! Import Into Selected Shape: pick a closed vector, then a drawing or picture, and it comes
//! in sized to fit inside that shape. A small window then offers Scale to fit (proportions
//! kept), Stretch (the shape's proportions) or Morph (bent to follow the outline), and the
//! margin from the edge; each change redoes the one import step, so Undo takes it all away.
//! An open vector picked instead is a line to bend the drawing along, like lettering on an
//! arc: the drawing runs its length, above, on or below it.

use super::CutCraftApp;
use cut_craft_bug_report::hint::HoverHint;
use cut_craft_core::bend::{Bend, Side};
use cut_craft_core::document::{Document, EntityId};
use cut_craft_core::fit::{FitError, FitMode, Morph, fit_into};
use cut_craft_core::geometry::{Path, Point, Rect};
use cut_craft_core::glam::DAffine2;
use cut_craft_core::{Change, Edit};
use cut_craft_i18n::tr;
use eframe::egui;

/// How the drawing takes the shape's size.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum Way {
    /// As big as fits inside the shape's outline, its proportions kept.
    #[default]
    Scale,
    /// The shape's proportions, as big as fits.
    Stretch,
    /// Bent to follow the shape's outline.
    Morph,
}

/// The default gap from the shape's edge: this share of its shorter side.
const MARGIN_SHARE: f64 = 0.05;
/// Curves are read as points no further apart than this share of the drawing's size.
const DETAIL: f64 = 2000.0;

/// An import waiting in its window: the drawing as read, the shape it goes in, the window's
/// settings, and the history's revision once it was put in (so it is put in again only while
/// nothing else changed).
pub(super) struct FitImport {
    raw: Change,
    shape: EntityId,
    pub(super) way: Way,
    /// Which side of an open line the drawing sits on.
    pub(super) side: Side,
    /// Millimetres: from a shape's edge, or from each end of a line.
    pub(super) margin: f64,
    /// The revision once the fitted drawing went in; `None` while it is not in the job.
    placed: Option<u64>,
    /// Why the last try did not fit, shown in the window.
    problem: Option<String>,
    /// What to look out for in the last try (a bend that crumples), shown in the window.
    caution: Option<String>,
    name: String,
}

/// The one vector selected, which a drawing is fitted into (a closed shape) or bent along
/// (an open line), or what to do instead.
pub(super) fn chosen_shape(doc: &Document) -> Result<EntityId, String> {
    let picked: Vec<EntityId> = doc.selection.iter().copied().collect();
    match picked[..] {
        [] => Err(tr!("fit-import-select-first")),
        [id] if doc.vectors.contains_key(&id) => Ok(id),
        [_] => Err(tr!("fit-import-select-a-vector")),
        _ => Err(tr!("fit-import-select-just-one")),
    }
}

/// Whether the one vector selected is an open line, which a drawing is bent along (the
/// right-click item then reads Import Along This Line…).
pub(super) fn chosen_is_line(doc: &Document) -> bool {
    chosen_shape(doc)
        .ok()
        .and_then(|id| doc.vectors.get(&id))
        .is_some_and(|p| !p.closed)
}

/// The margin a shape starts with: a twentieth of its shorter side. Along an open line the
/// drawing runs its whole length: no gap at the ends.
pub(super) fn default_margin(shape: &Path) -> f64 {
    if shape.closed {
        shape.bounds().size().min_element() * MARGIN_SHARE
    } else {
        0.0
    }
}

/// The warning added when a bend crumples the drawing's inside edge.
fn crumples() -> String {
    tr!("fit-import-crumples")
}

/// Every point of what `change` adds, curves flattened finely: for finding its size and
/// shape. Pictures and 3D components count by their corners.
fn points(change: &Change) -> Vec<Point> {
    let corners = |r: Rect| {
        [
            r.min,
            Point::new(r.max.x, r.min.y),
            r.max,
            Point::new(r.min.x, r.max.y),
        ]
    };
    let rough = bounds(change).map_or(1.0, |b| b.size().max_element().max(1e-6));
    change
        .edits
        .iter()
        .flat_map(|e| match e {
            Edit::Add { path, .. } => path.flatten(rough / DETAIL),
            Edit::AddComponent { component, .. } => corners(component.extent()).to_vec(),
            Edit::AddBitmap { bitmap, .. } => corners(bitmap.rect).to_vec(),
            _ => vec![],
        })
        .collect()
}

/// The box round everything `change` adds.
fn bounds(change: &Change) -> Option<Rect> {
    change
        .edits
        .iter()
        .filter_map(|e| match e {
            Edit::Add { path, .. } => Some(path.bounds()),
            Edit::AddComponent { component, .. } => Some(component.extent()),
            Edit::AddBitmap { bitmap, .. } => Some(bitmap.rect),
            _ => None,
        })
        .reduce(Rect::union)
}

/// `change` with everything it adds carried by `t`.
fn carried(mut change: Change, t: &DAffine2) -> Change {
    for edit in &mut change.edits {
        match edit {
            Edit::Add { path, .. } => *path = path.transformed(t),
            Edit::AddComponent { component, .. } => **component = component.transformed(t),
            Edit::AddBitmap { bitmap, .. } => {
                bitmap.rect = Rect::new(
                    t.transform_point2(bitmap.rect.min),
                    t.transform_point2(bitmap.rect.max),
                );
            }
            _ => {}
        }
    }
    change
}

/// A reason, as a sentence.
fn why(e: FitError) -> String {
    let mut s = e.to_string();
    if let Some(first) = s.get_mut(0..1) {
        first.make_ascii_uppercase();
    }
    s
}

/// `raw` bent along the open `line`, `gap` from each end, on its `side`: its width runs the
/// line's length, proportions kept. Pictures and 3D components cannot bend, so they are
/// scaled the same and centred where their middle lands.
fn bent(raw: &Change, line: &Path, gap: f64, side: Side) -> Result<Change, String> {
    let from = bounds(raw).ok_or_else(|| why(FitError::NoDrawing))?;
    let along = line.flatten(line.bounds().size().max_element().max(1e-6) / DETAIL);
    let bend = Bend::new(from, &along, gap, side).map_err(why)?;
    let step = from.size().max_element() / 400.0;
    let flat = from.size().max_element() / DETAIL;
    let solid = |r: Rect| {
        let middle = bend.point(r.center());
        let half = r.size() * bend.scale() / 2.0;
        Rect::new(middle - half, middle + half)
    };
    let mut change = raw.clone();
    for edit in &mut change.edits {
        match edit {
            Edit::Add { path, .. } => {
                let points = bend.polyline(&path.flatten(flat), path.closed, step);
                if points.len() >= 2 {
                    *path = Path::polyline(&points, path.closed);
                }
            }
            Edit::AddComponent { component, .. } => {
                let r = component.extent();
                let to = solid(r);
                let t = DAffine2::from_translation(to.center())
                    * DAffine2::from_scale(cut_craft_core::glam::DVec2::splat(bend.scale()))
                    * DAffine2::from_translation(-r.center());
                **component = component.transformed(&t);
            }
            Edit::AddBitmap { bitmap, .. } => bitmap.rect = solid(bitmap.rect),
            _ => {}
        }
    }
    if bend.folds() {
        change.notes.push(crumples());
    }
    Ok(change)
}

/// `raw` fitted into `shape` the chosen way, `margin` from its edge. Morph bends the
/// vectors; pictures and 3D components cannot bend, so they are scaled to fit. An open
/// `shape` is a line: `raw` is bent along it, on its `side`, `margin` from each end.
pub(super) fn fitted(
    raw: &Change,
    shape: &Path,
    margin: f64,
    way: Way,
    side: Side,
) -> Result<Change, String> {
    if !shape.closed {
        let mut change = bent(raw, shape, margin, side)?;
        change.notes.retain(|n| !n.contains("Set Size"));
        return Ok(change);
    }
    let ring = shape.flatten(shape.bounds().size().max_element() / DETAIL);
    let drawing = points(raw);
    let mode = match way {
        Way::Stretch => FitMode::Stretch,
        Way::Scale | Way::Morph => FitMode::Fit,
    };
    let mut change = if way == Way::Morph {
        let from = bounds(raw).ok_or_else(|| why(FitError::NoDrawing))?;
        let morph = Morph::new(from, &ring, margin).map_err(why)?;
        let step = from.size().max_element() / 200.0;
        let flat = from.size().max_element() / DETAIL;
        let mut change = raw.clone();
        // what cannot bend is scaled into the box the drawing's box bends to
        let scaled = fit_into(&drawing, &ring, margin, FitMode::Fit).map_err(why)?;
        for edit in &mut change.edits {
            match edit {
                Edit::Add { path, .. } => {
                    let bent = morph.polyline(&path.flatten(flat), path.closed, step);
                    if bent.len() >= 2 {
                        *path = Path::polyline(&bent, path.closed);
                    }
                }
                Edit::AddComponent { component, .. } => {
                    **component = component.transformed(&scaled);
                }
                Edit::AddBitmap { bitmap, .. } => {
                    bitmap.rect = Rect::new(
                        scaled.transform_point2(bitmap.rect.min),
                        scaled.transform_point2(bitmap.rect.max),
                    );
                }
                _ => {}
            }
        }
        change
    } else {
        let t = fit_into(&drawing, &ring, margin, mode).map_err(why)?;
        carried(raw.clone(), &t)
    };
    // a size the importer guessed (a DXF without units) no longer stands: it was resized
    change.notes.retain(|n| !n.contains("Set Size"));
    Ok(change)
}

impl FitImport {
    pub(super) fn new(raw: Change, shape: EntityId, margin: f64, name: String) -> Self {
        Self {
            raw,
            shape,
            way: Way::default(),
            side: Side::default(),
            margin,
            placed: None,
            problem: None,
            caution: None,
            name,
        }
    }
}

impl CutCraftApp {
    /// Import Into Selected Shape: the shape checked, then a drawing or picture picked.
    pub(super) fn import_into_shape(&mut self) {
        let shape = match chosen_shape(&self.doc) {
            Ok(id) => id,
            Err(why) => {
                self.status.set_message(&why);
                return;
            }
        };
        let dialog = super::filtered_dialog(super::Dialog::ImportIntoShape, &self.registry);
        if let Some(path) = dialog.pick_file() {
            self.import_into(&path, shape);
        }
    }

    /// Reads `path` (off the window's thread) and fits it into the vector `shape`.
    pub(super) fn import_into(&mut self, path: &std::path::Path, shape: EntityId) {
        let (registry, doc, reading) = (
            std::sync::Arc::clone(&self.registry),
            self.doc.clone(),
            path.to_owned(),
        );
        let name = super::file_name(path);
        self.in_background(
            &tr!("fit-import-importing", name = name.clone()),
            &super::panels::importing(path),
            move || super::files::import(&registry, &doc, &reading),
            move |app, read| match read {
                Ok(raw) => {
                    let margin = app.doc.vectors.get(&shape).map_or(0.0, default_margin);
                    app.fit_import = Some(FitImport::new(raw, shape, margin, name));
                    app.place_fit_import();
                }
                Err(e) => app.status.set_message(&e),
            },
        );
    }

    /// Puts the waiting import in with the window's settings, taking out the last try first.
    pub(super) fn place_fit_import(&mut self) {
        let Some(mut fit) = self.fit_import.take() else {
            return;
        };
        if let Some(revision) = fit.placed.take()
            && revision == self.history.revision()
        {
            self.history.undo(&mut self.doc);
        }
        let Some(shape) = self.doc.vectors.get(&fit.shape).cloned() else {
            self.status.set_message(&tr!("fit-import-shape-gone"));
            return;
        };
        let done = if shape.closed {
            tr!("fit-import-imported-into", name = fit.name.clone())
        } else {
            tr!("fit-import-imported-along", name = fit.name.clone())
        };
        match fitted(&fit.raw, &shape, fit.margin, fit.way, fit.side) {
            Ok(change) => {
                let crumples = crumples();
                fit.caution = change.notes.iter().find(|n| **n == crumples).cloned();
                self.record(Ok(change), &done);
                fit.placed = Some(self.history.revision());
                fit.problem = None;
            }
            Err(why) => {
                self.status.set_message(&why);
                fit.problem = Some(why);
                fit.caution = None;
            }
        }
        self.fit_import = Some(fit);
    }

    /// The window over an import into a shape: how it fits, the margin, Done and Undo
    /// Import. It closes by itself once something else changes the job.
    pub(super) fn fit_import_window(&mut self, ctx: &egui::Context) {
        let Some(fit) = self.fit_import.as_ref() else {
            return;
        };
        if fit
            .placed
            .is_some_and(|revision| revision != self.history.revision())
        {
            self.fit_import = None;
            return;
        }
        let units = self.doc.job.units;
        let (mut way, mut side, mut margin) = (fit.way, fit.side, fit.margin);
        let (problem, caution) = (fit.problem.clone(), fit.caution.clone());
        let line = self.doc.vectors.get(&fit.shape).is_some_and(|p| !p.closed);
        let (title, mark) = if line {
            (tr!("fit-import-along-line-title"), "Import Along Line")
        } else {
            (tr!("fit-import-into-shape-title"), "Import Into Shape")
        };
        let mut open = true;
        let (mut done, mut undo) = (false, false);
        egui::Window::new(title)
            .id(egui::Id::unique("fit_import_window"))
            .open(&mut open)
            .resizable(false)
            .collapsible(false)
            .show(ctx, |ui| {
                cut_craft_bug_report::mark(ui, mark);
                if line {
                    along_line_settings(ui, &mut side, &mut margin, units);
                } else {
                    into_shape_settings(ui, &mut way, &mut margin, units);
                }
                if let Some(why) = &problem {
                    ui.colored_label(ui.visuals().warn_fg_color, why);
                }
                if let Some(look_out) = &caution {
                    ui.colored_label(ui.visuals().warn_fg_color, look_out);
                }
                ui.separator();
                ui.horizontal(|ui| {
                    done = ui
                        .button(tr!("fit-import-done"))
                        .on_hover_hint(tr!("fit-import-keep-the-drawing-as-it"))
                        .clicked();
                    undo = ui
                        .button(tr!("fit-import-undo-import"))
                        .on_hover_hint(tr!("fit-import-take-the-drawing-back-out"))
                        .clicked();
                });
            });
        if undo {
            if let Some(revision) = self.fit_import.take().and_then(|f| f.placed)
                && revision == self.history.revision()
            {
                self.history.undo(&mut self.doc);
            }
            self.status.set_message(&tr!("fit-import-undone"));
            return;
        }
        if done || !open {
            self.fit_import = None;
            return;
        }
        if let Some(fit) = self.fit_import.as_mut()
            && (fit.way != way || fit.side != side || (fit.margin - margin).abs() > f64::EPSILON)
        {
            fit.way = way;
            fit.side = side;
            fit.margin = margin;
            self.place_fit_import();
        }
    }
}

/// The window's settings for a drawing going into a closed shape: the way it takes the
/// shape's size, and the margin from the edge.
fn into_shape_settings(
    ui: &mut egui::Ui,
    way: &mut Way,
    margin: &mut f64,
    units: cut_craft_core::document::Units,
) {
    ui.label(tr!("fit-import-how-the-drawing-takes-the"));
    let ways = [
        (Way::Scale, "fit-import-way-scale"),
        (Way::Stretch, "fit-import-way-stretch"),
        (Way::Morph, "fit-import-way-morph"),
    ];
    ui.horizontal(|ui| {
        for (w, key) in ways {
            ui.radio_value(way, w, tr!(key))
                .on_hover_hint(tr!(&format!("{key}.tooltip")));
        }
    });
    if let Some((_, key)) = ways.iter().find(|w| w.0 == *way) {
        ui.weak(tr!(&format!("{key}.tooltip")));
    }
    ui.horizontal(|ui| {
        ui.label(tr!("fit-import-margin"))
            .on_hover_hint(tr!("fit-import-the-gap-kept-between-the"));
        crate::form::length_edit(ui, margin, units, 0.0);
    });
}

/// The window's settings for a drawing bent along an open line: which side of it the
/// drawing sits on, and the gap left at each end.
fn along_line_settings(
    ui: &mut egui::Ui,
    side: &mut Side,
    gap: &mut f64,
    units: cut_craft_core::document::Units,
) {
    ui.label(tr!("fit-import-the-drawing-is-bent-to"));
    ui.label(tr!("fit-import-where-it-sits"));
    let sides = [
        (Side::Above, "fit-import-side-above"),
        (Side::On, "fit-import-side-on"),
        (Side::Below, "fit-import-side-below"),
    ];
    ui.horizontal(|ui| {
        for (s, key) in sides {
            ui.radio_value(side, s, tr!(key))
                .on_hover_hint(tr!(&format!("{key}.tooltip")));
        }
    });
    if let Some((_, key)) = sides.iter().find(|s| s.0 == *side) {
        ui.weak(tr!(&format!("{key}.tooltip")));
    }
    ui.horizontal(|ui| {
        ui.label(tr!("fit-import-gap-at-each-end"))
            .on_hover_hint(tr!("fit-import-how-far-in-from-each"));
        crate::form::length_edit(ui, gap, units, 0.0);
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::geometry::Path;

    fn doc_with(paths: &[Path]) -> (Document, Vec<EntityId>) {
        let mut doc = Document::default();
        let mut c = Change::builder(&doc);
        let ids: Vec<EntityId> = paths.iter().map(|p| c.add(p.clone())).collect();
        doc.apply(&c.finish()).unwrap();
        (doc, ids)
    }

    fn import_of(paths: &[Path]) -> Change {
        let doc = Document::default();
        let mut c = Change::builder(&doc);
        let ids: Vec<EntityId> = paths.iter().map(|p| c.add(p.clone())).collect();
        c.select(ids);
        c.finish()
    }

    fn square(at: Point, side: f64) -> Path {
        Path::polyline(
            &[
                at,
                at + Point::new(side, 0.0),
                at + Point::new(side, side),
                at + Point::new(0.0, side),
            ],
            true,
        )
    }

    #[test]
    fn the_shape_must_be_one_closed_vector_and_says_what_to_pick() {
        let open = Path::polyline(&[Point::ZERO, Point::new(10.0, 0.0)], false);
        let (mut doc, ids) = doc_with(&[square(Point::ZERO, 10.0), open, square(Point::ZERO, 5.0)]);
        assert!(
            chosen_shape(&doc)
                .unwrap_err()
                .starts_with("Select the shape")
        );
        doc.selection = [ids[0]].into();
        assert_eq!(chosen_shape(&doc), Ok(ids[0]));
        assert!(!chosen_is_line(&doc));
        doc.selection = [ids[1]].into();
        assert_eq!(
            chosen_shape(&doc),
            Ok(ids[1]),
            "an open line, to bend along"
        );
        assert!(chosen_is_line(&doc));
        doc.selection = [ids[0], ids[2]].into();
        assert!(chosen_shape(&doc).unwrap_err().contains("just one"));
    }

    #[test]
    fn the_margin_starts_at_a_twentieth_of_the_shorter_side() {
        let plaque = Path::polyline(
            &[
                Point::ZERO,
                Point::new(200.0, 0.0),
                Point::new(200.0, 100.0),
                Point::new(0.0, 100.0),
            ],
            true,
        );
        assert!((default_margin(&plaque) - 5.0).abs() < 1e-9);
    }

    #[test]
    fn a_drawing_comes_in_scaled_into_a_circle_and_stays_selected() {
        let raw = import_of(&[square(Point::new(500.0, 500.0), 4.0)]);
        let shape = Path::circle(Point::new(50.0, 50.0), 50.0);
        let change = fitted(&raw, &shape, 0.0, Way::Scale, Side::Above).unwrap();
        let b = bounds(&change).unwrap();
        assert!((b.size().x - 100.0 / 2f64.sqrt()).abs() < 0.1, "{b:?}");
        assert!(b.center().distance(Point::new(50.0, 50.0)) < 1e-6);
        assert!(
            change
                .edits
                .iter()
                .any(|e| matches!(e, Edit::Select { .. })),
            "the import stays selected, to nudge"
        );
    }

    #[test]
    fn morph_bends_the_drawings_edges_onto_the_circle() {
        let raw = import_of(&[square(Point::ZERO, 10.0)]);
        let shape = Path::circle(Point::new(0.0, 0.0), 30.0);
        let change = fitted(&raw, &shape, 0.0, Way::Morph, Side::Above).unwrap();
        let Some(Edit::Add { path, .. }) =
            change.edits.iter().find(|e| matches!(e, Edit::Add { .. }))
        else {
            panic!("a vector");
        };
        let ring = path.flatten(0.01);
        assert!(ring.len() > 40, "straight sides became curves");
        for p in ring {
            assert!((p.length() - 30.0).abs() < 0.2, "{p} is on the circle");
        }
    }

    #[test]
    fn along_an_open_arch_lettering_stands_on_it_and_runs_its_length() {
        // a 20 by 2 strip on a half circle of radius 50 drawn right to left
        let raw = import_of(&[Path::polyline(
            &[
                Point::ZERO,
                Point::new(20.0, 0.0),
                Point::new(20.0, 2.0),
                Point::new(0.0, 2.0),
            ],
            true,
        )]);
        let arch: Vec<Point> = (0..=90)
            .map(|i| {
                let a = std::f64::consts::PI * f64::from(i) / 90.0;
                Point::new(a.cos(), a.sin()) * 50.0
            })
            .collect();
        let line = Path::polyline(&arch, false);
        assert!(
            default_margin(&line).abs() < 1e-12,
            "it runs the whole line"
        );
        let change = fitted(&raw, &line, 0.0, Way::Scale, Side::Above).unwrap();
        let Some(Edit::Add { path, .. }) =
            change.edits.iter().find(|e| matches!(e, Edit::Add { .. }))
        else {
            panic!("a vector");
        };
        let points = path.flatten(0.01);
        // the strip grows 157 / 20 times: its height is about 15.7, outside the arch
        let scale = std::f64::consts::PI * 50.0 / 20.0;
        for p in &points {
            let r = p.length();
            assert!(
                r > 49.9 && r < 50.0 + 2.0 * scale + 0.1,
                "{p} sits on the arch"
            );
        }
        assert!(points.iter().any(|p| (p.length() - 50.0).abs() < 0.05));
        assert!(
            points
                .iter()
                .any(|p| p.distance(Point::new(-50.0, 0.0)) < 0.05)
        );
        assert!(
            points
                .iter()
                .any(|p| p.distance(Point::new(50.0, 0.0)) < 0.05)
        );
        assert!(change.notes.is_empty(), "no crumpling outside an arch");
        // a square inside the arch is far taller (157) than the bend (50) allows: a warning
        let square = import_of(&[square(Point::ZERO, 10.0)]);
        let below = fitted(&square, &line, 0.0, Way::Scale, Side::Below).unwrap();
        assert!(below.notes.iter().any(|n| n.contains("folds over itself")));
    }

    #[test]
    fn a_unit_guess_note_goes_once_the_drawing_is_resized() {
        let mut raw = import_of(&[square(Point::ZERO, 10.0)]);
        raw.notes = vec![
            "Read as inches: it is 6 by 6 in. If that is the wrong size, it is still selected: use Set Size".into(),
            "Left out: 2 texts".into(),
        ];
        let shape = Path::circle(Point::ZERO, 30.0);
        let change = fitted(&raw, &shape, 0.0, Way::Scale, Side::Above).unwrap();
        assert_eq!(change.notes, vec!["Left out: 2 texts".to_owned()]);
    }
}
