//! Options ▸ Web API, the calls that show what the job makes and run its slow jobs: pictures
//! of the views, the state, the toolpaths with their times, calculating and previewing,
//! G-code, saving and exporting. Each runs through what the matching button runs; a call
//! that starts a slow job (or a picture) is answered once it is done, and the calls after it
//! wait their turn. Nothing is ever sent to the machine: Cut Craft is not a G-code sender.

use super::{CutCraftApp, Split, View, files, view3d};
use cut_craft_core::document::EntityId;
use cut_craft_core::preflight::Finding;
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
            Call::NewJob { settings, discard } => now(self.web_new_job(settings, *discard)),
            Call::Open { path, discard } => self.web_open(path, *discard),
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
            && (self.simulation.is_some() || self.player.playback.is_some())
        {
            capture.was.set_aside = Some((self.simulation.take(), self.player.playback.take()));
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
            self.player.playback = playback;
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
                leave_out_windows(ctx);
                cut_craft_bug_report::take_screenshot(ctx, &shot);
                capture.asked = Some((now, shot));
            }
            return None;
        };
        let Some(window) = cut_craft_bug_report::taken(shot) else {
            // the window may hand over a later frame than the one asked on: each frame is
            // kept clear until the picture comes
            leave_out_windows(ctx);
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
                Some(app.job_minutes()),
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
            "check",
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
            Some(busy) => busy.status(),
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
        job::toolpaths(
            &self.doc,
            &|t| self.estimated_minutes(t),
            Some(self.job_minutes()),
            &BTreeMap::new(),
        )
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
            && self.doc.toolpaths_under_group(name).is_empty()
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
        // the toolpaths picked are there to save
        self.with_picked(gcode, |app| app.plan(&form, &post))
            .map_err(|e| ApiError::refused(capital(&e)))?;
        let mut found: Vec<Finding> = found.into_values().flatten().collect();
        found.extend(self.no_tool_changer_for(form.changes));
        // as Save Toolpaths decides: an error stops it; every warning (the collet or
        // spindle hitting the material too) is asked about, and "accept_warnings" is the
        // Save Anyway answer. Nothing is ticked as read through the Web API.
        let warnings: Vec<String> = found.iter().map(|f| f.message.clone()).collect();
        let accepted = if gcode.accept_warnings {
            warnings.as_slice()
        } else {
            &[]
        };
        match super::output::verdict(&found, &std::collections::BTreeSet::default(), accepted) {
            super::output::Verdict::Refused(refused) => return Err(ApiError::refused(refused)),
            super::output::Verdict::Ask(problems) => {
                return Err(ApiError::refused(format!(
                    "The checks before cutting found: {}. Put them right, or send \
                     \"accept_warnings\": true to make the G-code anyway (as Save Anyway).",
                    problems.join("; ")
                )));
            }
            super::output::Verdict::Go => {}
        }
        let saving = gcode.save_to.as_deref().map(std::path::PathBuf::from);
        let path = match &saving {
            Some(path) => files::with_extension_added(path, &post.extension),
            None => std::path::PathBuf::from(self.output().file_name(&form, &post)),
        };
        let made = chrono::Local::now().naive_local();
        let texts = self
            .with_picked(gcode, |app| app.toolpath_texts(&form, &post, &path, made))
            .map_err(|e| ApiError::refused(capital(&e)))?;
        if saving.is_some() {
            // an earlier save there (the same names, or the tile, numbered and side files
            // an earlier layout left) is written over only when asked, and then as Save
            // Toolpaths does: archived or replaced as Options say (Replace when they ask)
            let mut earlier = crate::files::earlier_save(&path);
            for (there, _) in texts.iter().filter(|(p, _)| p.exists()) {
                if !earlier.contains(there) {
                    earlier.push(there.clone());
                }
            }
            if let Some(there) = earlier.first().filter(|_| !gcode.replace) {
                return Err(ApiError::refused(format!(
                    "{} is there already: send \"replace\": true to write over it (archived \
                     or replaced as Options say), or save somewhere else.",
                    there.display()
                )));
            }
            let written = if earlier.is_empty() {
                super::output::write(&texts)
            } else {
                let archiving = self.options.confirmations.earlier_gcode
                    == crate::options::EarlierGcode::Archive;
                let texts = texts.clone();
                super::output::write_over(|| Ok(texts), &path, &earlier, archiving, made)
                    .map(|over| over.written)
            }
            .map_err(ApiError::refused)?;
            let said = self.output().saved_message(&written, &path);
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

    /// Refused while there is unsaved work, unless the call says to throw it away.
    fn unsaved_kept(&self, discard: bool) -> Result<(), ApiError> {
        if self.screen == super::Screen::Job && !discard && self.session.is_dirty(&self.history) {
            return Err(ApiError::refused(format!(
                "The job open has unsaved changes. Save it first (POST {}job/save), or send \
                 \"discard\": true to throw them away.",
                cut_craft_web_api::PREFIX
            )));
        }
        Ok(())
    }

    /// `POST job/new`: New, then Job Setup's OK on its starting settings (the saved
    /// default job's) with the call's laid over them. Settings it will not take are tried
    /// on a job of their own first, so they leave the job open as it was.
    fn web_new_job(&mut self, settings: &Value, discard: bool) -> Answer {
        self.unsaved_kept(discard)?;
        let mut fresh = cut_craft_core::document::Document::default();
        if let Some(default) = self.options.default_job {
            fresh.job = default;
        }
        let params = job::run_params(&self.registry, &fresh, "job_setup", settings)?;
        let setup = self
            .registry
            .get("job_setup")
            .ok_or_else(|| ApiError::refused("Job Setup is missing."))?;
        if let Err(why) = (setup.run_json(&fresh, &params).map_err(|e| e.to_string()))
            .and_then(|change| fresh.apply(&change).map_err(|e| e.to_string()))
        {
            return Err(ApiError::refused(format!(
                "Job Setup would not take that: {why}"
            )));
        }
        cut_craft_bug_report::logs::step("Web API started a new job");
        // a drawing waiting on a Job Setup the window started is not this job's
        self.import_after_setup = None;
        self.reset_job();
        self.view = View::TwoD;
        self.start_command("job_setup");
        let Some(form) = self.form.as_mut().filter(|f| f.id == "job_setup") else {
            self.screen = super::Screen::Start;
            return Err(ApiError::refused("Job Setup did not open."));
        };
        form.new_job = true;
        form.params = params;
        self.screen = super::Screen::NewJob;
        self.job_setup_ok();
        if self.screen != super::Screen::Job {
            let why = (self.form.as_ref())
                .and_then(|f| f.error.clone())
                .unwrap_or_else(|| self.status.message.clone());
            self.form = None;
            self.job_setup_cancel();
            return Err(ApiError::refused(format!(
                "Job Setup would not take that: {why}"
            )));
        }
        Ok(json!({
            "message": self.status.message,
            "state": job::state(&self.doc, self.selected_toolpath),
        }))
    }

    /// `POST job/open`: Open on `path`, answered once it has been read (a big job is read
    /// with its progress window).
    fn web_open(&mut self, path: &str, discard: bool) -> Started {
        let path = match whole_path(path).and_then(|p| self.unsaved_kept(discard).map(|()| p)) {
            Ok(path) => path,
            Err(e) => return Started::Now(Err(e)),
        };
        if !path.is_file() {
            return Started::Now(Err(ApiError::refused(format!(
                "There is no job file at {}.",
                path.display()
            ))));
        }
        cut_craft_bug_report::logs::step("Web API opened a job");
        // a job read puts itself in place of the one open, which starts a new job epoch;
        // one that will not read leaves it, even when it is the same file
        let before = self.job_epoch;
        self.open_path(&path);
        Started::AfterBusy(Box::new(move |app| {
            if app.job_epoch == before
                || app.screen != super::Screen::Job
                || app.session.path.as_deref() != Some(path.as_path())
            {
                return Err(ApiError::refused(format!(
                    "{} did not open: {}",
                    path.display(),
                    app.status.message
                )));
            }
            Ok(json!({
                "message": app.status.message,
                "state": job::state(&app.doc, app.selected_toolpath),
            }))
        }))
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

/// Takes this frame's windows, menus and pop-ups out of what is drawn, so the picture taken
/// of it shows the view alone (the frame after the picture draws them again): everything
/// above the panels.
fn leave_out_windows(ctx: &egui::Context) {
    let above: Vec<egui::LayerId> = ctx.memory(|m| {
        m.layer_ids()
            .filter(|l| l.order != egui::Order::Background)
            .collect()
    });
    ctx.graphics_mut(|g| {
        for layer in above {
            if let Some(list) = g.get_mut(layer) {
                *list = egui::layers::PaintList::default();
            }
        }
    });
}
