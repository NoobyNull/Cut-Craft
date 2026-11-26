//! The slow jobs run off the window's thread, with their progress over the window
//! ([`crate::busy`]): calculating and editing toolpaths, recalculating them, previewing
//! them. A job over in a moment finishes as if run in place.

use super::CutCraftApp;
use crate::busy::Busy;
use cut_craft_core::document::{Document, EntityId};
use cut_craft_core::progress;
use cut_craft_core::simulation::Material;
use cut_craft_core::{Change, CommandError, Registry};
use eframe::egui;
use serde_json::Value;
use std::sync::Arc;

impl CutCraftApp {
    /// Runs `work` on its own thread, its progress shown as `what` ("Calculating Pocket
    /// Toolpath") once it takes a while; `then` gets what it made. While it runs the
    /// window can only show it; should the job change anyway, what it made is not used.
    /// `kind` names the sort of job ("calculate:pocket-toolpath", "preview") so its time
    /// left starts from how long the last one took ([`cut_craft_core::eta`]).
    pub(super) fn in_background<T: crate::busy::Outcome + Send + 'static>(
        &mut self,
        what: &str,
        kind: &str,
        work: impl FnOnce() -> T + Send + 'static,
        then: impl FnOnce(&mut Self, T) + 'static,
    ) {
        // one at a time: one already running is seen through first
        if let Some(running) = self.busy.take() {
            running.finish(self);
        }
        let revision = self.history.revision();
        let named = what.to_owned();
        let busy = Busy::start(what, kind, None, work, move |app: &mut Self, made| {
            app.keep_job_times();
            match made {
                Err(why) => app.status.set_message(&why),
                Ok(_) if app.history.revision() != revision => app.status.set_message(
                    &cut_craft_i18n::tr!("status-not-used-job-changed", what = named.clone()),
                ),
                Ok(made) => then(app, made),
            }
        });
        if busy.wait(self.wait_before_showing) {
            busy.finish(self);
        } else {
            self.busy = Some(busy);
        }
    }

    /// Finds the spelling words added and how long each kind of job took last run
    /// ([`cut_craft_core::eta`]), so the first guesses at the time left start from them.
    pub(super) fn load_words_and_job_times(&mut self) {
        let dir = super::files::config_dir();
        (self.spelling).words_at(dir.as_ref().map(|d| d.join("words.txt")));
        self.job_times_path = dir.map(|d| d.join("job-times.json"));
        if let Some(path) = &self.job_times_path {
            cut_craft_core::eta::set_history(cut_craft_core::eta::History::load(path));
        }
    }

    /// Writes how long each kind of job took ([`cut_craft_core::eta`]) to the settings
    /// folder, so the next run's first guesses start from them; nothing in tests.
    pub(super) fn keep_job_times(&self) {
        if let Some(path) = &self.job_times_path
            && let Err(e) = cut_craft_core::eta::history().save(path)
        {
            log::warn!("Could not keep the job times in {}: {e}", path.display());
        }
    }

    /// Once a frame: a finished job's answer used, else its progress over the window, and
    /// Cancel, which lets it go with nothing changed.
    pub(super) fn busy_window(&mut self, ctx: &egui::Context) {
        let Some(busy) = self.busy.take() else {
            return;
        };
        if busy.is_done() {
            busy.finish(self);
        } else if busy.window(ctx) {
            let what = busy.what().to_owned();
            busy.cancel();
            self.status.set_message(&cut_craft_i18n::tr!(
                "status-cancelled-nothing-changed",
                what = what
            ));
        } else {
            self.busy = Some(busy);
        }
    }

    /// What the window says a command's job is doing: a toolpath is calculated, anything
    /// else applied.
    fn doing(&self, id: &str) -> String {
        let meta = self.registry.get(id).map(cut_craft_core::DynCommand::meta);
        let label = meta.map_or(id, |m| m.label);
        if meta.is_some_and(|m| m.group.contains("Toolpath")) {
            format!("Calculating {label}")
        } else {
            format!("Applying {label}")
        }
    }

    /// The [`cut_craft_core::eta`] history kind for running command `id`: a toolpath's
    /// calculation shares one with its Edit and Recalculate.
    fn job_kind(&self, id: &str) -> String {
        let meta = self.registry.get(id).map(cut_craft_core::DynCommand::meta);
        if meta.is_some_and(|m| m.group.contains("Toolpath")) {
            cut_craft_core::eta::calculating(id)
        } else {
            format!("apply:{id}")
        }
    }

    /// A form's Apply or Calculate for command `id` off the window's thread. What goes
    /// wrong shows on the form, if it is still open, else in the status bar.
    pub(super) fn apply_in_background(&mut self, id: &str, params: &Value) -> Result<(), String> {
        self.calculate_in_background(id, params, None)
    }

    /// [`Self::apply_in_background`], what it makes put in place of the toolpaths `onto`
    /// made last time (Calculate pressed again on a toolpath form after a change), from the
    /// vectors they were made from when none are picked now. A toolpath form follows what
    /// it made, so its next Calculate updates that.
    pub(super) fn calculate_in_background(
        &mut self,
        id: &str,
        params: &Value,
        onto: Option<super::calculate_again::Made>,
    ) -> Result<(), String> {
        let mut doc = self.doc.clone();
        let mut old = Vec::new();
        if let Some(made) = onto {
            if doc.selection.is_empty()
                && let Some(picked) = made.selection
            {
                doc.selection = picked;
            }
            // cut after what comes before them, not after themselves (rest machining)
            doc.remaking = made.ids.iter().min().copied();
            old = made.ids;
        }
        let selection = doc.selection.clone();
        let picked = std::mem::replace(&mut self.doc.selection, selection.clone());
        let allowed = self.crossings_allowed(id, params);
        self.doc.selection = picked;
        allowed?;
        let what = self.doing(id);
        let kind = self.job_kind(id);
        let registry = Arc::clone(&self.registry);
        let (command, settings) = (id.to_owned(), self.with_surroundings(id, params));
        let (id, params) = (id.to_owned(), params.clone());
        self.in_background(
            &what,
            &kind,
            move || {
                command_change(&registry, &doc, &command, &settings).map(|c| {
                    if old.is_empty() {
                        c
                    } else {
                        c.onto_toolpaths(&old, &doc)
                    }
                })
            },
            move |app, change| {
                let made = change
                    .as_ref()
                    .map_or_else(|_| Vec::new(), Change::toolpaths_made);
                let applied = app.applied(&id, &params, change);
                if applied.is_ok() {
                    app.follow_made(&id, &params, made, Some(selection));
                }
                app.form_failed(&id, applied);
            },
        );
        Ok(())
    }

    /// Edit Toolpath's Calculate: toolpath `toolpath` made again with `params` in its place,
    /// off the window's thread.
    pub(super) fn edit_in_background(&mut self, id: &str, toolpath: EntityId, params: &Value) {
        let what = self
            .doc
            .toolpaths
            .get(&toolpath)
            .map_or_else(|| self.doing(id), |t| format!("Calculating {}", t.name));
        // the machine's holder, chuck and tailstock as they are now, for a rotary toolpath
        let (registry, doc, settings) = (
            Arc::clone(&self.registry),
            self.doc.clone(),
            self.with_surroundings(id, params),
        );
        let kind = self.job_kind(id);
        let settings_kept = params.clone();
        let id = id.to_owned();
        self.in_background(
            &what,
            &kind,
            move || registry.edit(&doc, toolpath, Some(&settings)),
            move |app, change| {
                // as when first made: not into a keep-out zone unless clipping is allowed
                let change = change.and_then(|c| match app.keep_out_refusal(&c) {
                    Some(why) => Err(CommandError::Failed(why)),
                    None => Ok(c),
                });
                let failed = change.as_ref().err().map(ToString::to_string);
                app.record(change, "Toolpath edited");
                if failed.is_none() {
                    app.follow_made(&id, &settings_kept, vec![toolpath], None);
                }
                app.form_failed(&id, failed.map_or(Ok(()), Err));
            },
        );
    }

    /// What went wrong with form `id`'s job, on the form while it is open.
    fn form_failed(&mut self, id: &str, result: Result<(), String>) {
        let Err(why) = result else { return };
        match self.form.as_mut().filter(|f| f.id == id) {
            Some(form) => form.error = Some(why),
            None => self.status.set_message(&why),
        }
    }

    /// Toolpaths `ids` recalculated in turn off the window's thread, `done` said after;
    /// `then` runs once they are in the job.
    pub(super) fn recalculate_in_background(
        &mut self,
        ids: Vec<EntityId>,
        done: &str,
        then: impl FnOnce(&mut Self) + 'static,
    ) {
        let what = match ids[..] {
            [one] => self.doc.toolpaths.get(&one).map_or_else(
                || "Recalculating the toolpath".to_owned(),
                |t| format!("Recalculating {}", t.name),
            ),
            _ => format!("Recalculating {} toolpaths", ids.len()),
        };
        let (registry, mut doc, done) = (
            Arc::clone(&self.registry),
            self.doc.clone(),
            done.to_owned(),
        );
        // rotary toolpaths keep clear of the machine's holder, chuck and tailstock as they
        // are now, not as they were when first made
        for id in &ids {
            if let Some(t) = doc.toolpaths.get_mut(id) {
                t.params = self.with_surroundings(&t.strategy, &t.params);
            }
        }
        self.in_background(
            &what,
            "recalculate",
            move || registry.recalculate_each(&doc, &ids),
            move |app, change| {
                app.record(change, &done);
                then(app);
            },
        );
    }

    /// Toolpaths `ids` cut in turn into `start` off the window's thread (Preview Toolpath,
    /// Preview All): the cut material becomes the preview, then `then` runs.
    pub(super) fn preview_in_background(
        &mut self,
        ids: &[EntityId],
        start: Material,
        then: impl FnOnce(&mut Self) + 'static,
    ) {
        let source = self.preview_source();
        let cuts: Vec<(u32, cut_craft_core::toolpath::Toolpath)> = ids
            .iter()
            .filter_map(|&id| Some((self.preview_mark(id), source.toolpaths.get(&id)?.clone())))
            .collect();
        let checked: Vec<EntityId> = ids
            .iter()
            .copied()
            .filter(|id| source.toolpaths.contains_key(id))
            .collect();
        let (job, holding) = (source.job, self.holding());
        let clamps: Vec<_> = source.named_keep_out().cloned().collect();
        drop(source);
        let what = match &cuts[..] {
            [(_, one)] => format!("Previewing {}", one.name),
            _ => format!("Previewing {} toolpaths", cuts.len()),
        };
        // the same split as the progress title: the one toolpath's name, else how many
        let done = match &cuts[..] {
            [] => None,
            [(_, one)] => Some(format!("Previewed {}", one.name)),
            _ => Some(format!("Previewed {} toolpaths", cuts.len())),
        };
        let tools: Vec<cut_craft_core::toolpath::Tool> =
            cuts.iter().map(|(_, t)| t.tool.clone()).collect();
        // a toolpath of many moves takes longer to cut than one of a few: its share of the bar
        let weights = preview_weights(cuts.iter().map(|(_, t)| t));
        self.in_background(
            &what,
            "preview",
            move || {
                let mut material = start;
                for (i, (mark, toolpath)) in cuts.iter().enumerate() {
                    if progress::cancelled() {
                        break;
                    }
                    progress::stage(&format!("{} ({} of {})", toolpath.name, i + 1, cuts.len()));
                    material.cutting = *mark;
                    progress::part_weighted(i, &weights, || material.cut(toolpath));
                }
                let toolpaths: Vec<&cut_craft_core::toolpath::Toolpath> =
                    cuts.iter().map(|(_, t)| t).collect();
                let collided = collided(&job, &checked, &toolpaths, (&holding, &clamps));
                (material, collided)
            },
            move |app, (material, collided)| {
                let hit = collision_note(&collided);
                app.preview.collisions = collided;
                let fine_bit =
                    crate::display::fine_bit_note(&tools, material.cell, app.display.detail);
                app.preview.material = Some(material);
                app.player.kept = None;
                app.preview_changed();
                if app.display.auto_open_3d {
                    app.view = super::View::ThreeD;
                }
                // one line, so the fine-bit note never hides what was previewed
                let message = match (done, fine_bit) {
                    (Some(done), Some(note)) => Some(format!("{done}. {note}")),
                    (done, note) => done.or(note),
                };
                let message = match (message, hit) {
                    (Some(message), Some(hit)) => Some(format!("{message}. {hit}")),
                    (message, hit) => message.or(hit),
                };
                if let Some(message) = message {
                    app.status.set_message(&message);
                }
                then(app);
            },
        );
    }
}

/// What command `id` makes of `doc` with `params`: a toolpath with a Vector Selector from
/// whatever vectors pass it, rest machining when its settings ask for it, and only what its
/// selector keeps. Runs on any thread.
pub(super) fn command_change(
    registry: &Registry,
    doc: &Document,
    id: &str,
    params: &Value,
) -> Result<Change, String> {
    let cmd = registry
        .get(id)
        .expect("only registered commands are applied");
    let label = cmd.meta().label;
    log::info!("{label}: settings {}", settings_line(params));
    let change = command_change_of(cmd, doc, params);
    match &change {
        Ok(change) => log::info!("{label}: {}", summary(change)),
        Err(why) => log::warn!("{label} failed: {why}"),
    }
    change
}

/// The most of a command's settings the log keeps.
const SETTINGS_CHARS: usize = 400;

/// A command's settings as one line for the log, cut short when long.
fn settings_line(params: &Value) -> String {
    let line = params.to_string();
    if line.chars().count() <= SETTINGS_CHARS {
        return line;
    }
    let mut short: String = line.chars().take(SETTINGS_CHARS).collect();
    short.push('…');
    short
}

/// What a command made, for the log: how many of each kind of thing it added, changed or
/// removed, and how many moves its toolpaths have.
pub(super) fn summary(change: &Change) -> String {
    use cut_craft_core::Edit;
    let (mut vectors, mut toolpaths, mut components, mut bitmaps, mut moves) = (0, 0, 0, 0, 0);
    for edit in &change.edits {
        match edit {
            Edit::Add { .. } | Edit::Remove { .. } | Edit::Replace { .. } => vectors += 1,
            Edit::AddToolpath { toolpath, .. }
            | Edit::ReplaceToolpath {
                after: toolpath, ..
            } => {
                toolpaths += 1;
                moves += toolpath.moves.len();
            }
            Edit::RemoveToolpath { .. } => toolpaths += 1,
            Edit::AddComponent { .. }
            | Edit::RemoveComponent { .. }
            | Edit::ReplaceComponent { .. } => components += 1,
            Edit::AddBitmap { .. } | Edit::RemoveBitmap { .. } | Edit::ReplaceBitmap { .. } => {
                bitmaps += 1;
            }
            _ => {}
        }
    }
    let mut said: Vec<String> = [
        (vectors, "vector"),
        (toolpaths, "toolpath"),
        (components, "component"),
        (bitmaps, "bitmap"),
    ]
    .iter()
    .filter(|(n, _)| *n > 0)
    .map(|(n, what)| format!("{n} {what}{}", if *n == 1 { "" } else { "s" }))
    .collect();
    if moves > 0 {
        said.push(format!("{moves} moves"));
    }
    if change.edits.is_empty() {
        "done, nothing changed".to_owned()
    } else if said.is_empty() {
        format!("done, {} other changes", change.edits.len())
    } else {
        format!("done, changed {}", said.join(", "))
    }
}

fn command_change_of(
    cmd: &dyn cut_craft_core::DynCommand,
    doc: &Document,
    params: &Value,
) -> Result<Change, String> {
    let change = match cut_craft_core::selector::Selector::of(params) {
        Some(selector) => {
            let mut sources = doc.clone();
            sources.selection = selector.picked(doc).into_iter().collect();
            if sources.selection.is_empty() {
                return Err("no vectors pass the Vector Selector".into());
            }
            cmd.run_json(&sources, params)
        }
        None => cmd.run_json(doc, params),
    }
    .map_err(|e: CommandError| e.to_string())?;
    let change =
        cut_craft_core::rest::with_rest(&doc.job, params, change).map_err(|e| e.to_string())?;
    let change = cut_craft_core::locked_face::with_locked_face(&doc.job, params, change)
        .map_err(|e| e.to_string())?;
    Ok(cut_craft_core::selector::kept(params, change))
}

/// Each toolpath's share of a preview's progress bar: cutting takes about as long as the
/// moves it makes.
pub(super) fn preview_weights<'a>(
    toolpaths: impl Iterator<Item = &'a cut_craft_core::toolpath::Toolpath>,
) -> Vec<f64> {
    #[expect(clippy::cast_precision_loss, reason = "a share of a progress bar")]
    toolpaths.map(|t| t.moves.len() as f64 + 1.0).collect()
}

/// What the collet nut, spindle or shank hit cutting `toolpaths` (`ids`, the same order) in
/// turn into `job`'s uncut material, held as `holding` says, with `clamps` (the keep-out
/// zones) standing on it: each toolpath that hits, with what to say of it.
pub(super) fn collided(
    job: &cut_craft_core::document::Job,
    ids: &[EntityId],
    toolpaths: &[&cut_craft_core::toolpath::Toolpath],
    (holding, clamps): (
        &cut_craft_core::collision::Holding,
        &[cut_craft_core::keep_out::KeepOutZone],
    ),
) -> std::collections::BTreeMap<EntityId, super::Collided> {
    let found = cut_craft_core::collision::find(job, toolpaths, holding, clamps);
    ids.iter()
        .zip(toolpaths)
        .zip(found)
        .filter_map(|((&id, toolpath), found)| {
            let sentence = cut_craft_core::collision::sentence(
                &toolpath.name,
                &toolpath.tool,
                &found,
                job.units,
            )?;
            Some((
                id,
                super::Collided {
                    sentence,
                    hits: found.hits,
                    moves: fingerprint(toolpath),
                },
            ))
        })
        .collect()
}

/// A quick stamp of `toolpath`'s moves and bit, which changes when it is recalculated or its
/// bit is edited: its length and up to 64 moves spread through it.
pub(super) fn fingerprint(toolpath: &cut_craft_core::toolpath::Toolpath) -> u64 {
    use std::hash::{Hash as _, Hasher as _};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    let moves = &toolpath.moves;
    moves.len().hash(&mut hasher);
    let step = (moves.len() / 64).max(1);
    for m in moves.iter().step_by(step).chain(moves.last()) {
        for v in [m.to.x, m.to.y, m.to.z] {
            v.to_bits().hash(&mut hasher);
        }
    }
    let tool = &toolpath.tool;
    for v in [tool.diameter, tool.stickout, tool.flute_length] {
        v.to_bits().hash(&mut hasher);
    }
    hasher.finish()
}

/// The status bar's word on `collided`: which toolpaths hit, and where to look; `None` when
/// none do.
pub(super) fn collision_note(
    collided: &std::collections::BTreeMap<EntityId, super::Collided>,
) -> Option<String> {
    match collided.len() {
        0 => None,
        1 => Some(
            "⚠ The collet or spindle would hit the material in 1 toolpath: it is red in the \
             Toolpaths list and the material in its way is red in the 3D view"
                .into(),
        ),
        n => Some(format!(
            "⚠ The collet or spindle would hit the material in {n} toolpaths: they are red \
             in the Toolpaths list and the material in their way is red in the 3D view"
        )),
    }
}
