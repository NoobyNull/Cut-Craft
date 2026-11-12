//! Try a sample project, on the start page: a small sign (a word and a rounded border) whose
//! toolpaths are made as it opens, so a newcomer can press Preview and see a result in the
//! first minute. Cut Craft's own: Aspire has none (see `re/SPEC.md`, Start page).

use super::{CutCraftApp, Screen, View};
use cut_craft_core::document::Document;
use serde_json::{Value, json};

/// The sign's drawing: the job, the word (as Draw Text made it, so Edit Text can change it)
/// and the border. Made by `make_sample_sign` below; no toolpaths, those are made on opening.
const SIGN: &[u8] = include_bytes!("../../assets/sample-sign.ccjob");
/// The sign's size, mm: small enough for every machine in `machines/`.
#[cfg(test)]
const SIZE: [f64; 3] = [200.0, 100.0, 18.0];
/// The border's distance in from the edge, and its corners' radius, mm.
#[cfg(test)]
const BORDER_INSET: f64 = 6.0;
#[cfg(test)]
const BORDER_RADIUS: f64 = 10.0;
/// The word, its height and font as drawn.
#[cfg(test)]
const WORD: &str = "HELLO";
#[cfg(test)]
const WORD_HEIGHT: f64 = 28.0;
#[cfg(test)]
const WORD_FONT: &str = "DejaVu Sans";
/// The toolpaths' names, in the order they cut: carve first, then cut the sign free.
pub(super) const CARVE: &str = "1 Carve the word (V-Carve)";
pub(super) const CUT_OUT: &str = "2 Cut the sign out (Profile)";
/// Profile tabs holding the sign in the sheet.
const TABS: u32 = 4;

/// The sign's drawing.
pub(super) fn sign() -> Document {
    cut_craft_core::file::load_job(SIGN).expect("the sample sign opens")
}

impl CutCraftApp {
    /// Try a sample project: the sign, its toolpaths made, as a new unsaved job in the units
    /// new jobs use.
    pub(super) fn open_sample(&mut self) {
        if !self.confirm_discard() {
            return;
        }
        self.reset_job();
        let units = self.doc.job.units;
        self.doc = sign();
        self.doc.job.units = units;
        self.screen = Screen::Job;
        self.view = View::TwoD;
        match self.sample_toolpaths() {
            Ok(()) => self.status.set_message(
                "Sample sign opened with its toolpaths made. Press Preview All Toolpaths on the \
                 Toolpaths page to watch it cut",
            ),
            Err(e) => self
                .status
                .set_message(&format!("The sample's toolpaths could not be made: {e}")),
        }
        self.doc.selection = cut_craft_core::selection::Selection::new();
        self.form = None;
        // opened, not edited: closing it asks nothing
        (self.history, self.session) = Default::default();
    }

    /// The V-carve of the word, then the profile cutting the sign out with tabs.
    fn sample_toolpaths(&mut self) -> Result<(), String> {
        let ids: Vec<_> = self.doc.vectors.keys().copied().collect();
        let border = *ids
            .iter()
            .max_by(|a, b| {
                let area = |id| {
                    let size = self.doc.vectors[id].bounds().size();
                    size.x * size.y
                };
                area(*a).total_cmp(&area(*b))
            })
            .ok_or("the sample has no vectors")?;
        let word: Vec<_> = ids.into_iter().filter(|&id| id != border).collect();
        self.doc.selection = word.into_iter().collect();
        let carve = self.sample_params("vcarve_engrave_toolpath", json!({ "name": CARVE }))?;
        self.apply("vcarve_engrave_toolpath", &carve)?;
        self.doc.selection = [border].into_iter().collect();
        let depth = self.doc.job.thickness;
        let profile = self.sample_params(
            "profile_toolpath",
            json!({
                "name": CUT_OUT,
                "depth": depth,
                "side": "outside",
                "tabs": "flat",
                "tab": { "spacing": "constant_number", "number": TABS },
                // a typical 6 mm end mill's flutes, so it reaches right through: a starter
                // bit with none set is taken to reach only half the material
                "tool": { "flute_length": 22.0 },
            }),
        )?;
        self.apply("profile_toolpath", &profile)
    }

    /// Command `id`'s params as its form would start, with `changes` laid over them (one
    /// level of objects merged).
    fn sample_params(&self, id: &str, changes: Value) -> Result<Value, String> {
        let command = self.registry.get(id).ok_or(format!("no {id}"))?;
        let mut params = command.initial_params(&self.doc);
        if let (Some(params), Value::Object(changes)) = (params.as_object_mut(), changes) {
            for (key, value) in changes {
                match (params.get_mut(&key), value) {
                    (Some(Value::Object(old)), Value::Object(new)) => old.extend(new),
                    (_, value) => {
                        params.insert(key, value);
                    }
                }
            }
        }
        Ok(params)
    }
}

/// Writes `assets/sample-sign.ccjob` again, from Draw Text in `WORD_FONT` (installed where it
/// runs) and Draw Rectangle: `cargo test -p cut-craft make_sample_sign -- --ignored`.
#[test]
#[ignore = "rewrites the sample's drawing; needs the DejaVu Sans font"]
fn make_sample_sign() {
    use cut_craft_core::geometry::{Path, Rect};
    let mut app = CutCraftApp::new();
    let [width, height, thickness] = SIZE;
    app.doc.job = cut_craft_core::document::Job {
        width,
        height,
        thickness,
        ..cut_craft_core::document::Job::default()
    };
    let rectangle = json!({
        "x": BORDER_INSET, "y": BORDER_INSET,
        "width": width - 2.0 * BORDER_INSET, "height": height - 2.0 * BORDER_INSET,
        "corners": "rounded", "radius": BORDER_RADIUS,
    });
    app.apply("draw_rectangle", &rectangle).unwrap();
    let text = |at: [f64; 2]| {
        let mut params = app.registry.get("draw_text").unwrap().default_params();
        params["text"] = WORD.into();
        params["font"] = WORD_FONT.into();
        params["bold"] = true.into();
        params["height"] = WORD_HEIGHT.into();
        params["anchor"] = json!({ "x": at[0], "y": at[1] });
        params
    };
    // drawn once to measure, then again centred
    let mut trial = CutCraftApp::new();
    trial.apply("draw_text", &text([0.0, 0.0])).unwrap();
    let drawn = trial.doc.vectors.values().map(Path::bounds);
    let b = drawn.reduce(Rect::union).expect("the word was drawn");
    let at = [
        (width - b.size().x) / 2.0 - b.min.x,
        (height - b.size().y) / 2.0 - b.min.y,
    ];
    app.apply("draw_text", &text(at)).unwrap();
    app.doc.selection = cut_craft_core::selection::Selection::new();
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/sample-sign.ccjob");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, cut_craft_core::file::save_job(&app.doc)).unwrap();
}

#[test]
fn the_word_sits_inside_the_border_clear_of_its_cut() {
    let doc = sign();
    let bounds: Vec<_> = doc
        .vectors
        .values()
        .map(cut_craft_core::geometry::Path::bounds)
        .collect();
    let area = |b: &cut_craft_core::geometry::Rect| b.size().x * b.size().y;
    let border = *bounds
        .iter()
        .max_by(|a, b| area(a).total_cmp(&area(b)))
        .unwrap();
    // the border's rounded corners and the profile's tool both need room
    let room = BORDER_RADIUS / 2.0;
    for b in bounds.iter().filter(|b| **b != border) {
        assert!(
            b.min.x >= border.min.x + room
                && b.min.y >= border.min.y + room
                && b.max.x <= border.max.x - room
                && b.max.y <= border.max.y - room,
            "{b:?} in {border:?}"
        );
    }
}
