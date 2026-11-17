//! Options ▸ Web API, the calls that show what the job makes and run its slow jobs: pictures
//! of the views, the state, the toolpaths with their times, calculating and previewing,
//! G-code, saving and exporting. Each runs through what the matching button runs; a call
//! that starts a slow job (or a picture) is answered once it is done, and the calls after it
//! wait their turn. Nothing is ever sent to the machine: Cut Craft is not a G-code sender.

use super::{CutCraftApp, Split, View, files, view3d};
use cut_craft_core::document::EntityId;
use cut_craft_core::preflight::{Finding, Severity};
use cut_craft_web_api::{Answer, ApiError, Call, Gcode, Look, Pending, Picture, Shown, Which, job};
use eframe::egui;
use serde_json::{Value, json};
use std::collections::BTreeMap;

/// How long the window is left showing a view before its picture is taken, seconds: past
/// the 3D view drawing finely once still.
const SETTLE_3D: f64 = 1.0;
/// The same for the 2D view, which draws finely at once.
const SETTLE_2D: f64 = 0.2;
/// The fewest frames drawn before the picture is taken, so the view is laid out.
const SETTLE_FRAMES: u32 = 3;
/// How long the picture may take to come once asked for, seconds.
const PICTURE_WITHIN: f64 = 10.0;

/// What the progress window says while the Web API's checks before cutting run.
const CHECKING_FOR_API: &str = "Checking the toolpaths for the Web API";

/// How a call started: answered now, or later.
pub(super) enum Started {
    Now(Answer),
    /// Answered once the slow job it started is done.
    AfterBusy(Box<dyn FnOnce(&mut CutCraftApp) -> Answer>),
    /// Answered with the picture, once taken.
    Picture(Box<Capture>),
}

/// A call waiting on what it started.
pub(super) enum Later {
    AfterBusy(Pending, Box<dyn FnOnce(&mut CutCraftApp) -> Answer>),
    Picture(Pending, Box<Capture>),
}

/// A picture being taken: the window shows the view asked for, then is put back as it was.
pub(super) struct Capture {
    picture: Picture,
    was: Was,
    since: Option<f64>,
    frames: u32,
    asked: Option<(f64, cut_craft_bug_report::Shot)>,
}

/// How the window was before a picture was taken.
struct Was {
    three_d: bool,
    split: Option<Split>,
    orbit: view3d::Orbit,
    framing: view3d::Framing,
    spinning: Option<f64>,
    /// The preview, set aside to show the model before it.
    set_aside: Option<(
        Option<cut_craft_core::simulation::Material>,
        Option<cut_craft_core::playback::Playback>,
    )>,
}

impl CutCraftApp {
    /// Whether a call is answered even while a slow job runs or another call waits on one.
    pub(super) fn web_answered_while_busy(call: &Call) -> bool {
        matches!(call, Call::Status | Call::Progress | Call::Cancel)
    }

    /// Starts a call: those that start a slow job or a picture are answered later.
    pub(super) fn web_start(&mut self, ctx: &egui::Context, call: &Call) -> Started {
        let now = |answer| Started::Now(answer);
        match call {
            Call::View(picture) => match self.start_picture(picture) {
                Ok(capture) => Started::Picture(Box::new(capture)),
                Err(e) => now(Err(e)),
            },
            Call::State => now(self
                .job_needed()
                .map(|()| job::state(&self.doc, self.selected_toolpath))),
            Call::Toolpaths => self.web_toolpaths(),
            Call::Posts => now(self.web_posts()),
            Call::Progress => now(Ok(self.web_progress())),
            Call::Cancel => now(self.web_cancel()),
            Call::Calculate { which, wait } => self.web_calculate(which, *wait),
            Call::Preview { which, wait } => self.web_preview(which, *wait),
            Call::ResetPreview => now(self.web_reset_preview(ctx)),
            Call::Gcode(gcode) => self.web_gcode(gcode),
            Call::Save { path } => now(self.web_save(path.as_deref())),
            Call::Export { format, path } => now(self.web_export(format, path)),
            other => now(self.web_call(ctx, other)),
        }
    }

    /// Each frame: a call waiting on a slow job answered once it is done, or a picture
    /// taken a step further. Whether one is still waiting.
    pub(super) fn web_later(&mut self, ctx: &egui::Context) -> bool {
        match self.web_api.later.take() {
            None => false,
            Some(Later::AfterBusy(pending, then)) => {
                if self.busy.is_some() {
                    self.web_api.later = Some(Later::AfterBusy(pending, then));
                    return true;
                }
                pending.answer(then(self));
                false
            }
            Some(Later::Picture(pending, mut capture)) => {
                match self.picture_step(ctx, &mut capture) {
                    None => {
                        self.web_api.later = Some(Later::Picture(pending, capture));
                        true
                    }
                    Some(taken) => {
                        self.put_back(capture.was);
                        match taken {
                            Ok(png) => pending.answer_png(png),
                            Err(e) => pending.answer(Err(e)),
                        }
                        false
                    }
                }
            }
        }
    }

    /// Waits on what `started` started, or answers now.
    pub(super) fn web_wait_on(&mut self, ctx: &egui::Context, pending: Pending, started: Started) {
        match started {
            Started::Now(answer) => pending.answer(answer),
            Started::AfterBusy(then) => {
                self.web_api.later = Some(Later::AfterBusy(pending, then));
                self.web_later(ctx);
            }
            Started::Picture(capture) => {
                self.web_api.later = Some(Later::Picture(pending, capture));
                ctx.request_repaint();
            }
        }
    }

    // ----- pictures

    /// Shows the view `picture` asks for, the window's own settings kept to put back.
    fn start_picture(&mut self, picture: &Picture) -> Result<Capture, ApiError> {
        self.job_needed()?;
        let was = Was {
            three_d: self.view == View::ThreeD,
            split: self.split,
            orbit: self.orbit,
            framing: self.framing,
            spinning: self.spinning_3d,
            set_aside: None,
        };
        let mut capture = Capture {
            picture: picture.clone(),
            was,
            since: None,
            frames: 0,
            asked: None,
        };
        self.split = None;
        let orbit = match picture.view {
            Look::TwoD => {
                self.view = View::TwoD;
                return Ok(capture);
            }
            Look::Top => view3d::Orbit::TOP,
            Look::Front => view3d::Orbit::FRONT,
            Look::Right => view3d::Orbit::RIGHT,
            Look::Iso => view3d::Orbit::default(),
        };
        self.view = View::ThreeD;
        self.orbit = orbit;
        self.framing = self.framing.fitted();
        self.spinning_3d = None;
        if picture.preview == Shown::Before
            && (self.simulation.is_some() || self.playback.is_some())
        {
            capture.was.set_aside = Some((self.simulation.take(), self.playback.take()));
            (self.simulation_coarse, self.fine_3d, self.surface_gpu) = (None, None, None);
        }
        Ok(capture)
    }

    /// The window as it was before the picture.
    fn put_back(&mut self, was: Was) {
        self.view = if was.three_d {
            View::ThreeD
        } else {
            View::TwoD
        };
        self.split = was.split;
        self.orbit = was.orbit;
        self.framing = was.framing;
        self.spinning_3d = was.spinning;
        if let Some((simulation, playback)) = was.set_aside {
            self.simulation = simulation;
            self.playback = playback;
            (self.simulation_coarse, self.fine_3d, self.surface_gpu) = (None, None, None);
        }
    }

    /// A frame of taking the picture: once the view has settled it is asked for, and once
    /// it comes the view's part is cut out (and made the width asked) as a PNG. `None`
    /// while it is still being taken.
    fn picture_step(
        &mut self,
        ctx: &egui::Context,
        capture: &mut Capture,
    ) -> Option<Result<Vec<u8>, ApiError>> {
        ctx.request_repaint();
        let now = ctx.input(|i| i.time);
        let since = *capture.since.get_or_insert(now);
        capture.frames += 1;
        let two_d = capture.picture.view == Look::TwoD;
        let Some((asked_at, shot)) = &capture.asked else {
            let settle = if two_d { SETTLE_2D } else { SETTLE_3D };
            if capture.frames >= SETTLE_FRAMES && now - since >= settle {
                let shot = cut_craft_bug_report::Shot::default();
                cut_craft_bug_report::take_screenshot(ctx, &shot);
                capture.asked = Some((now, shot));
            }
            return None;
        };
        let Some(window) = cut_craft_bug_report::taken(shot) else {
            if now - asked_at > PICTURE_WITHIN {
                return Some(Err(ApiError {
                    status: 503,
                    message: "Cut Craft's window did not draw the picture. It must be open on \
                              screen, not minimised: show it, then try again."
                        .into(),
                }));
            }
            return None;
        };
        let (area, what) = if two_d {
            (self.view_area, "the 2D view")
        } else {
            (self.view_3d_area, "the 3D view")
        };
        let Some(area) = area else {
            return Some(Err(ApiError::refused(format!("{what} was not drawn"))));
        };
        Some(picture_png(
            &window,
            area * ctx.pixels_per_point(),
            what,
            capture.picture.width,
        ))
    }

    // ----- what the job holds

    /// `GET toolpaths`: each toolpath with its time, after the checks before cutting run
    /// on each (off the window's thread, as Save Toolpaths' do).
    fn web_toolpaths(&mut self) -> Started {
        if let Err(e) = self.job_needed() {
            return Started::Now(Err(e));
        }
        self.start_web_checks(self.doc.toolpaths.keys().map(|&id| vec![id]).collect());
        Started::AfterBusy(Box::new(|app| {
            let checks = app.web_checks_found()?;
            Ok(job::toolpaths(
                &app.doc,
                &|t| app.estimated_minutes(t),
                &checks,
            ))
        }))
    }

    /// The checks before cutting, each of `plans` checked on its own, off the window's
    /// thread; [`Self::web_checks_found`] has what they found.
    fn start_web_checks(&mut self, plans: Vec<Vec<EntityId>>) {
        self.web_api.checks = None;
        let doc = self.doc.clone();
        let mut limits = self.limits();
        if self.default_changes() != files::ToolChanges::ToolChanger {
            limits.slots.clear();
        }
        self.in_background(
            CHECKING_FOR_API,
            move || {
                let mut found: BTreeMap<EntityId, Vec<Finding>> = BTreeMap::new();
                for plan in plans {
                    let toolpaths: Vec<&cut_craft_core::toolpath::Toolpath> =
                        plan.iter().filter_map(|id| doc.toolpaths.get(id)).collect();
                    let each = cut_craft_core::preflight::check(&doc, &toolpaths, &limits);
                    if let Some(&first) = plan.first() {
                        found.entry(first).or_default().extend(each);
                    }
                }
                found
            },
            |app, found| app.web_api.checks = Some(found),
        );
    }

    /// What the checks before cutting found, once they have run; why not, if they did not.
    fn web_checks_found(&mut self) -> Result<BTreeMap<EntityId, Vec<Finding>>, ApiError> {
        self.web_api.checks.take().ok_or_else(|| {
            ApiError::refused(format!(
                "The checks before cutting did not finish: {}",
                self.status.message
            ))
        })
    }

    /// `GET posts`: the active machine's posts that suit this job, and the one used when a
    /// call names none.
    fn web_posts(&self) -> Answer {
        self.job_needed()?;
        let default = self.output_post().map(|p| p.name);
        Ok(json!({
            "posts": self.post_choices(),
            "default": default.as_ref().ok(),
            "problem": default.err(),
            "machine": self.machines.active().map(|m| m.name.clone()),
        }))
    }

    /// `GET progress`: the slow job running, if any, and how far it has got.
    pub(super) fn web_progress(&self) -> Value {
        match &self.busy {
            None => json!({ "busy": false }),
            Some(busy) => json!({
                "busy": true,
                "doing": busy.what(),
                "step": busy.stage(),
                "fraction": busy.fraction(),
                "seconds": (busy.elapsed().as_secs_f64() * 10.0).round() / 10.0,
            }),
        }
    }

    /// `POST cancel`: Cancel on the progress window.
    fn web_cancel(&mut self) -> Answer {
        let Some(busy) = self.busy.take() else {
            return Err(ApiError::refused("Nothing is running to cancel."));
        };
        let what = busy.what().to_owned();
        busy.cancel();
        self.status
            .set_message(&format!("{what} cancelled: nothing changed"));
        Ok(json!({ "cancelled": what, "message": self.status.message }))
    }

    /// The toolpaths `which` names, in list order; an id that is not one says so.
    fn web_which(&self, which: &Which) -> Result<Vec<EntityId>, ApiError> {
        let ids: Vec<EntityId> = match which {
            Which::All => self.doc.toolpaths.keys().copied().collect(),
            Which::Visible => (self.doc.toolpaths.iter())
                .filter(|(_, t)| t.visible)
                .map(|(&id, _)| id)
                .collect(),
            Which::Ids(ids) => {
                let ids: Vec<EntityId> = ids.iter().copied().map(EntityId).collect();
                if let Some(missing) = ids.iter().find(|id| !self.doc.toolpaths.contains_key(id)) {
                    return Err(no_toolpath(missing.0));
                }
                // in list order, as the buttons work through them
                (self.doc.toolpaths.keys())
                    .filter(|id| ids.contains(id))
                    .copied()
                    .collect()
            }
        };
        if ids.is_empty() {
            return Err(ApiError::refused(
                "There are no toolpaths to work on: make one first, or tick the ones to show.",
            ));
        }
        Ok(ids)
    }

    /// The toolpaths as `GET toolpaths` lists them, without the checks before cutting.
    fn web_toolpath_rows(&self) -> Value {
        job::toolpaths(&self.doc, &|t| self.estimated_minutes(t), &BTreeMap::new())
    }

    /// `POST calculate`: Recalculate on these toolpaths, with its progress window.
    fn web_calculate(&mut self, which: &Which, wait: bool) -> Started {
        let ids = match self.job_needed().and_then(|()| self.web_which(which)) {
            Ok(ids) => ids,
            Err(e) => return Started::Now(Err(e)),
        };
        cut_craft_bug_report::logs::step("Web API calculated toolpaths");
        let done = match ids[..] {
            [_] => "Toolpath recalculated",
            _ => "Toolpaths recalculated",
        };
        self.recalculate_in_background(ids, done, |_| {});
        self.after(wait, |app| {
            Ok(json!({ "message": app.status.message, "toolpaths": app.web_toolpath_rows()["toolpaths"] }))
        })
    }

    /// `POST preview`: Preview on these toolpaths into fresh material, cut at once (not
    /// played), as one preview Undo Last Preview takes back.
    fn web_preview(&mut self, which: &Which, wait: bool) -> Started {
        let ids = match self.job_needed().and_then(|()| self.web_which(which)) {
            Ok(ids) => ids,
            Err(e) => return Started::Now(Err(e)),
        };
        cut_craft_bug_report::logs::step("Web API previewed toolpaths");
        let animate = std::mem::replace(&mut self.display.animate_preview, false);
        match which {
            Which::All => self.preview_all(false),
            Which::Visible => self.preview_all(true),
            Which::Ids(_) => self.preview_ids(&ids),
        }
        self.display.animate_preview = animate;
        self.after(wait, |app| {
            if app.simulation.is_none() {
                return Err(ApiError::refused(format!(
                    "The preview was not made: {}",
                    app.status.message
                )));
            }
            Ok(json!({ "message": app.status.message, "previewed": true }))
        })
    }

    /// Answered once the slow job is done (`wait`), or straight away saying what started.
    fn after(
        &self,
        wait: bool,
        then: impl FnOnce(&mut CutCraftApp) -> Answer + 'static,
    ) -> Started {
        if wait {
            return Started::AfterBusy(Box::new(then));
        }
        Started::Now(Ok(json!({
            "started": self.busy.as_ref().map(|b| b.what().to_owned()),
            "message": self.status.message,
            "progress": format!("GET {}progress", cut_craft_web_api::PREFIX),
        })))
    }

    /// `POST preview/reset`: Reset Preview, greyed out (refused) when there is none.
    fn web_reset_preview(&mut self, ctx: &egui::Context) -> Answer {
        self.job_needed()?;
        if !self.available("reset_preview") {
            return Err(ApiError::refused("There is no preview to reset."));
        }
        self.invoke(ctx, "reset_preview");
        Ok(json!({ "message": self.status.message }))
    }

    // ----- G-code

    /// `POST gcode`: Save Toolpaths' form filled in from the call; its checks before
    /// cutting run off the window's thread, and the G-code is made once they pass.
    fn web_gcode(&mut self, gcode: &Gcode) -> Started {
        match self.web_gcode_plan(gcode) {
            Err(e) => Started::Now(Err(e)),
            Ok(plan) => {
                self.start_web_checks(vec![plan.into_iter().flatten().collect()]);
                let gcode = gcode.clone();
                Started::AfterBusy(Box::new(move |app| app.web_gcode_made(&gcode)))
            }
        }
    }

    /// Save Toolpaths' form for the call, and what it would hold.
    fn web_save_form(&self, gcode: &Gcode) -> super::SaveForm {
        let mut form = self.default_save_form();
        form.job_sheet = false;
        form.post.clone_from(&gcode.post);
        form.mode = if gcode.toolpath.is_some() || gcode.group.is_some() {
            files::SaveMode::Selected
        } else if gcode.each {
            files::SaveMode::VisibleToMultipleFiles
        } else {
            files::SaveMode::VisibleToOneFile
        };
        form
    }

    /// Runs `f` with the call's toolpath or group picked in the toolpath list, as the form's
    /// Selected mode reads it, then picks back what was.
    fn with_picked<R>(&mut self, gcode: &Gcode, f: impl FnOnce(&mut Self) -> R) -> R {
        let was = (self.selected_toolpath, self.selected_group.clone());
        if gcode.toolpath.is_some() || gcode.group.is_some() {
            self.selected_toolpath = gcode.toolpath.map(EntityId);
            self.selected_group.clone_from(&gcode.group);
        }
        let made = f(self);
        (self.selected_toolpath, self.selected_group) = was;
        made
    }

    /// The files' toolpaths for the call, checked: the toolpath, group and post are there.
    fn web_gcode_plan(&mut self, gcode: &Gcode) -> Result<Vec<Vec<EntityId>>, ApiError> {
        self.job_needed()?;
        if let Some(id) = gcode.toolpath
            && !self.doc.toolpaths.contains_key(&EntityId(id))
        {
            return Err(no_toolpath(id));
        }
        if let Some(name) = &gcode.group
            && self.doc.toolpaths_in_group(name).is_empty()
        {
            return Err(ApiError::not_found(format!(
                "There is no toolpath group \"{name}\" with toolpaths in it."
            )));
        }
        if let Some(path) = &gcode.save_to {
            whole_path(path)?;
        }
        let form = self.web_save_form(gcode);
        let post = self
            .form_post(&form)
            .map_err(|e| no_post(&e, &self.post_choices()))?;
        self.with_picked(gcode, |app| app.plan(&form, &post))
            .map_err(|e| ApiError::refused(capital(&e)))
    }

    /// Once the checks have run: refused for an error, or (unless accepted) a warning;
    /// else the G-code made, and saved where asked.
    fn web_gcode_made(&mut self, gcode: &Gcode) -> Answer {
        let found = self.web_checks_found()?;
        let form = self.web_save_form(gcode);
        let post = self
            .form_post(&form)
            .map_err(|e| no_post(&e, &self.post_choices()))?;
        let plan = self
            .with_picked(gcode, |app| app.plan(&form, &post))
            .map_err(|e| ApiError::refused(capital(&e)))?;
        let mut found: Vec<Finding> = found.into_values().flatten().collect();
        if form.changes == files::ToolChanges::ToolChanger && !self.has_tool_changer() {
            found.push(Self::no_tool_changer());
        }
        if let Some(refused) = Self::refused(&found) {
            return Err(ApiError::refused(refused));
        }
        let mut warnings: Vec<String> = (found.iter())
            .filter(|f| f.severity == Severity::Warning)
            .map(|f| f.message.clone())
            .collect();
        let stale: Vec<&str> = (plan.iter().flatten())
            .filter_map(|id| self.doc.toolpaths.get(id))
            .filter(|t| t.stale)
            .map(|t| t.name.as_str())
            .collect();
        if !stale.is_empty() {
            warnings.insert(
                0,
                format!(
                    "Out of date: {} changed since they were made. Recalculate before \
                     cutting (POST {}calculate)",
                    stale.join(", "),
                    cut_craft_web_api::PREFIX
                ),
            );
        }
        // as Save Toolpaths: every warning is asked about
        if !warnings.is_empty() && !gcode.accept_warnings {
            return Err(ApiError::refused(format!(
                "The checks before cutting found: {}. Put them right, or send \
                 \"accept_warnings\": true to make the G-code anyway (as Save Anyway).",
                warnings.join("; ")
            )));
        }
        let saving = gcode.save_to.as_deref().map(std::path::PathBuf::from);
        let path = match &saving {
            Some(path) => files::with_extension_added(path, &post.extension),
            None => std::path::PathBuf::from(format!(
                "{}.{}",
                cut_craft_machines::send::safe_name(&self.output_name(&form), false),
                post.extension
            )),
        };
        let made = chrono::Local::now().naive_local();
        let texts = self
            .with_picked(gcode, |app| app.toolpath_texts(&form, &post, &path, made))
            .map_err(|e| ApiError::refused(capital(&e)))?;
        if saving.is_some() {
            if !gcode.replace
                && let Some((there, _)) = texts.iter().find(|(p, _)| p.exists())
            {
                return Err(ApiError::refused(format!(
                    "{} is there already: send \"replace\": true to write over it, or save \
                     somewhere else.",
                    there.display()
                )));
            }
            for (path, text) in &texts {
                files::write_text(path, text).map_err(ApiError::refused)?;
            }
            let written: Vec<std::path::PathBuf> = texts.iter().map(|(p, _)| p.clone()).collect();
            let said = Self::saved_message(&written, &path, self.doc.tiling.is_some());
            self.status.set_message(&format!("{said} (Web API)"));
        }
        cut_craft_bug_report::logs::step("Web API made G-code");
        let files: Vec<Value> = texts
            .iter()
            .map(|(p, text)| {
                json!({
                    "name": p.file_name().map(|n| n.to_string_lossy().into_owned()),
                    "path": saving.as_ref().map(|_| p.display().to_string()),
                    "lines": text.lines().count(),
                    "gcode": text,
                })
            })
            .collect();
        Ok(json!({
            "post": post.name,
            "files": files,
            "warnings": warnings,
            "saved": saving.is_some(),
        }))
    }

    // ----- saving

    /// `POST job/save`: Save, or Save As to `path`.
    fn web_save(&mut self, path: Option<&str>) -> Answer {
        self.job_needed()?;
        let path = match path {
            Some(path) => {
                files::named_for_saving(&whole_path(path)?, &[cut_craft_core::file::JOB_EXTENSION])
            }
            None => self.session.path.clone().ok_or_else(|| {
                ApiError::refused(
                    "The job has never been saved: give \"path\", where to save it (as Save As).",
                )
            })?,
        };
        cut_craft_bug_report::logs::step("Web API saved the job");
        self.save_to(&path);
        if self.session.path.as_deref() != Some(path.as_path())
            || self.session.is_dirty(&self.history)
        {
            return Err(ApiError::refused(self.status.message.clone()));
        }
        Ok(json!({ "path": path, "message": self.status.message }))
    }

    /// `POST job/export`: an Export menu item, to `path`.
    fn web_export(&mut self, format: &str, path: &str) -> Answer {
        self.job_needed()?;
        let Some(entry) = self.registry.exporter(format) else {
            let ids: Vec<&str> = self.registry.exporters().map(|e| e.id).collect();
            return Err(ApiError::not_found(format!(
                "There is no export format \"{format}\". The formats are: {}.",
                ids.join(", ")
            )));
        };
        let path = files::named_for_saving(&whole_path(path)?, entry.format.extensions);
        cut_craft_bug_report::logs::step(&format!("Web API exported {format}"));
        files::export(&self.registry, format, &self.doc, &path).map_err(ApiError::refused)?;
        self.status
            .set_message(&format!("Exported {} (Web API)", path.display()));
        Ok(json!({ "path": path, "message": self.status.message }))
    }
}

/// The view's `area` (pixels) of the `window`'s picture as a PNG, made `width` wide if asked.
fn picture_png(
    window: &egui::ColorImage,
    area: egui::Rect,
    what: &str,
    width: Option<u32>,
) -> Result<Vec<u8>, ApiError> {
    let mut part = super::panels::part_of(window, area, what).map_err(ApiError::refused)?;
    if let Some(width) = width.filter(|&w| w != part.width()) {
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "a picture's height, in pixels"
        )]
        let height = (f64::from(part.height()) * f64::from(width) / f64::from(part.width().max(1)))
            .round()
            .max(1.0) as u32;
        part = image::imageops::resize(&part, width, height, image::imageops::FilterType::Triangle);
    }
    let mut png = std::io::Cursor::new(vec![]);
    part.write_to(&mut png, image::ImageFormat::Png)
        .map_err(|e| ApiError::refused(format!("The picture could not be made: {e}")))?;
    Ok(png.into_inner())
}

/// A path a call gives, which must be whole: a bare name would land wherever Cut Craft was
/// started from.
fn whole_path(path: &str) -> Result<std::path::PathBuf, ApiError> {
    let path = std::path::PathBuf::from(path.trim());
    if path.is_absolute() {
        Ok(path)
    } else {
        Err(ApiError::bad(format!(
            "\"{}\" is not a whole path: give the folder too, for example {}",
            path.display(),
            if cfg!(windows) {
                r"C:\Users\me\Documents\sign.nc"
            } else {
                "/home/me/sign.nc"
            }
        )))
    }
}

fn no_toolpath(id: u64) -> ApiError {
    ApiError::not_found(format!(
        "There is no toolpath {id}. GET {}toolpaths lists them.",
        cut_craft_web_api::PREFIX
    ))
}

/// A post that would not load, with the ones there are.
fn no_post(why: &str, posts: &[String]) -> ApiError {
    let there = if posts.is_empty() {
        String::new()
    } else {
        format!(" The posts for this job are: {}.", posts.join(", "))
    };
    ApiError::refused(format!("{}.{there}", capital(why).trim_end_matches('.')))
}

/// `text` with a capital first letter, as a sentence on its own.
fn capital(text: &str) -> String {
    let mut chars = text.chars();
    chars
        .next()
        .map_or_else(String::new, |c| c.to_uppercase().chain(chars).collect())
}
