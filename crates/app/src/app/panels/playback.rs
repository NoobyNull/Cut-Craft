//! Previews and their playback: the preview grid, the detail slider, playing the cut and its bar.

use super::{
    CLOSE_UP, Cow, CutCraftApp, EntityId, HoverHint, Material, PLAYBACK_SPEEDS, View, egui,
    playback_note, times, tr, view3d,
};

impl CutCraftApp {
    /// Preview All (or, `visible_only`, Preview Visible): start again from uncut material,
    /// then preview each toolpath (each shown one) in list order.
    pub(in super::super) fn preview_all(&mut self, visible_only: bool) {
        let visible: Vec<EntityId> = self
            .doc
            .toolpaths
            .iter()
            .filter(|(_, t)| t.visible || !visible_only)
            .map(|(&id, _)| id)
            .collect();
        if self.display.animate_preview {
            self.finish_playback();
            self.before_preview = Some(super::super::BeforePreview(
                self.simulation.clone(),
                self.preview_side,
            ));
            self.play(&visible, true, move |app| {
                app.preview_other_side(visible_only);
            });
        } else {
            self.preview_fresh(&visible, move |app| app.preview_other_side(visible_only));
        }
    }

    /// Previews `ids` in order on fresh material, as one preview to undo.
    pub(in super::super) fn preview_ids(&mut self, ids: &[EntityId]) {
        if !self.display.animate_preview {
            self.preview_fresh(ids, |_| {});
            return;
        }
        self.finish_playback();
        self.before_preview = Some(super::super::BeforePreview(
            self.simulation.clone(),
            self.preview_side,
        ));
        self.play(ids, true, |_| {});
    }

    /// Cuts `ids` in order into fresh material, as one preview to undo, then `then`.
    pub(super) fn preview_fresh(
        &mut self,
        ids: &[EntityId],
        then: impl FnOnce(&mut Self) + 'static,
    ) {
        self.finish_playback();
        // the preview before stays shown until this one is cut
        self.before_preview = Some(super::super::BeforePreview(
            self.simulation.clone(),
            self.preview_side,
        ));
        let fresh = self.fresh_material();
        self.preview_in_background(ids, fresh, then);
    }

    /// The grid a preview is worked out on now: where the job's toolpaths cut (all of them,
    /// so previewing one after another stays on the same grid), at the Preview tab's
    /// detail. A double-sided or rotary job's covers the whole block, as turning it over or
    /// wrapping it needs.
    pub(in super::super) fn preview_grid(&self) -> crate::preview_detail::Grid {
        // asked for every frame (the slider's hover text): worked out again only once the
        // toolpaths, the job or the detail change, as a big job has millions of moves
        let moves = (self.doc.toolpaths.values()).fold(0, |n, t| n + t.moves.len() + 1);
        let key = (
            (self.job_epoch, self.history.revision(), moves),
            self.display.detail,
            self.doc.job,
        );
        if let Some((kept, grid)) = self.preview_grid_kept.get()
            && kept == key
        {
            return grid;
        }
        let grid = self.work_out_preview_grid();
        self.preview_grid_kept.set(Some((key, grid)));
        grid
    }

    /// [`Self::preview_grid`], worked out.
    pub(super) fn work_out_preview_grid(&self) -> crate::preview_detail::Grid {
        let job = self.doc.job;
        let whole = job.double_sided.is_some() || job.rotary.is_some();
        let source = self.preview_source();
        let cut = (!whole)
            .then(|| crate::preview_detail::cut_area(&job, source.toolpaths.values()))
            .flatten();
        // a flat job's preview goes to the card in tiles; a rotary or double-sided one whole
        let card = self.gpu_3d.as_ref().map(if whole {
            view3d::gpu::Gpu::most_points_whole
        } else {
            view3d::gpu::Gpu::most_points
        });
        crate::preview_detail::grid(&job, cut, self.display.detail, card)
    }

    /// Uncut material on [`Self::preview_grid`].
    pub(in super::super) fn fresh_material(&self) -> Material {
        let grid = self.preview_grid();
        Material::over(&self.doc.job, grid.area, grid.cell)
    }

    /// The preview showing worked out again at the detail now asked for (the slider let
    /// go, or a quick pick), in the background: the toolpaths that left their mark on it,
    /// in the job's order, and the other side too when it was previewed.
    pub(in super::super) fn rework_preview(&mut self) {
        self.finish_playback();
        let Some(material) = &self.simulation else {
            return;
        };
        let marks: std::collections::BTreeSet<u32> = material
            .cut_by
            .iter()
            .copied()
            .filter(|&m| m != 0)
            .collect();
        let other = material.other_side.is_some();
        let ids: Vec<EntityId> = (self.preview_source().toolpaths.keys())
            .copied()
            .filter(|&id| marks.contains(&self.preview_mark(id)))
            .collect();
        if ids.is_empty() {
            return;
        }
        self.preview_fresh(&ids, move |app| {
            if other {
                app.preview_other_side(false);
            }
        });
    }

    /// The Preview tab's Performance ◀ ▶ Detail slider: the preview's detail now (Options
    /// ▸ 3D View keeps the default). Moving it changes nothing until it is let go; then a
    /// preview showing is worked out again at it. A red note under it says when the
    /// preview is coarser than it asks, and why.
    pub(super) fn detail_slider(&mut self, ui: &mut egui::Ui) {
        let grid = self.preview_grid();
        let units = self.doc.job.units;
        let mut percent = self.held_detail.unwrap_or(self.display.detail);
        let measure = |ui: &egui::Ui, text: &str| {
            egui::WidgetText::from(text)
                .into_galley(
                    ui,
                    Some(egui::TextWrapMode::Extend),
                    f32::INFINITY,
                    egui::TextStyle::Body,
                )
                .size()
                .x
        };
        let response = ui
            .horizontal(|ui| {
                let gap = ui.spacing().item_spacing.x;
                // the labels' places stay put whatever the percent: the widest it shows
                let right =
                    measure(ui, &tr!("panels-more-detail")) + measure(ui, "100%") + 2.0 * gap;
                ui.label(tr!("panels-faster"));
                ui.spacing_mut().slider_width = (ui.available_width() - right - gap).max(40.0);
                let slider = ui
                    .add(egui::Slider::new(&mut percent, 0..=100).show_value(false))
                    .on_hover_text(crate::preview_detail::about(&grid, units));
                ui.label(tr!("panels-more-detail"));
                ui.add_sized(
                    [measure(ui, "100%"), ui.spacing().interact_size.y],
                    egui::Label::new(format!("{percent}%")),
                );
                slider
            })
            .inner;
        crate::marks::note(ui, "preview_detail_slider", response.rect);
        if response.dragged() {
            self.held_detail = Some(percent);
        } else if response.drag_stopped() || response.changed() {
            self.held_detail = None;
            if percent != self.display.detail {
                self.display.detail = percent;
                self.rework_preview();
            }
        }
        if let Some(note) = crate::preview_detail::limit_note(&grid, units) {
            ui.colored_label(ui.visuals().error_fg_color, note);
        }
    }

    /// A double-sided job's other side previewed too (its shown toolpaths, or all), cut
    /// into the material's other side, which the 3D view shows as its underside.
    pub(super) fn preview_other_side(&mut self, visible_only: bool) {
        if self.doc.job.double_sided.is_none() || self.doc.other_side.toolpaths.is_empty() {
            return;
        }
        let (job, grid) = (self.doc.job, self.preview_grid());
        let cuts: Vec<cut_craft_core::toolpath::Toolpath> =
            (self.doc.other_side.toolpaths.values())
                .filter(|t| t.visible || !visible_only)
                .cloned()
                .collect();
        // cut off the window's thread, its progress shown, as the first side was
        let weights = super::super::background::preview_weights(cuts.iter());
        self.in_background(
            "Previewing the other side",
            "preview",
            move || {
                let mut other = Material::over(&job, grid.area, grid.cell);
                for (i, t) in cuts.iter().enumerate() {
                    if cut_craft_core::progress::cancelled() {
                        break;
                    }
                    cut_craft_core::progress::stage(&format!(
                        "{} ({} of {})",
                        t.name,
                        i + 1,
                        cuts.len()
                    ));
                    cut_craft_core::progress::part_weighted(i, &weights, || other.cut(t));
                }
                other
            },
            move |app, other| {
                let other = Some(std::sync::Arc::new(other));
                if let Some(playback) = app.player.playback.as_mut() {
                    playback.set_other_side(other);
                } else {
                    let material = app
                        .simulation
                        .get_or_insert_with(|| Material::over(&job, grid.area, grid.cell));
                    material.other_side = other;
                }
                (app.simulation_coarse, app.fine_3d, app.surface_gpu) = (None, None, None);
                if app.display.auto_open_3d {
                    app.view = View::ThreeD;
                }
            },
        );
    }

    /// What a preview cuts: the job's toolpaths, or (Toolpath Tiling with a tile active)
    /// only the active tile's moves, at the machine's corner or in their original place.
    pub(in super::super) fn preview_source(&self) -> Cow<'_, cut_craft_core::document::Document> {
        match self.doc.tiling.and_then(|t| t.preview(&self.doc)) {
            Some(tiled) => Cow::Owned(tiled),
            None => Cow::Borrowed(&self.doc),
        }
    }

    /// Starts playing `toolpaths` into the previewed material (uncut if there is none yet,
    /// or `fresh`: Preview All and a group start again from uncut material), then `then`.
    /// Their moves are laid out off the window's thread, with Cancel should
    /// that take a while (millions of moves); the preview before stays shown till then.
    pub(in super::super) fn play(
        &mut self,
        toolpaths: &[EntityId],
        fresh: bool,
        then: impl FnOnce(&mut Self) + 'static,
    ) {
        self.finish_playback();
        let source = self.preview_source();
        let (list, marks): (Vec<cut_craft_core::toolpath::Toolpath>, Vec<u32>) = toolpaths
            .iter()
            .filter_map(|&id| Some((source.toolpaths.get(&id)?.clone(), self.preview_mark(id))))
            .unzip();
        let checked: Vec<EntityId> = toolpaths
            .iter()
            .copied()
            .filter(|id| source.toolpaths.contains_key(id))
            .collect();
        // the toolpaths cut before these clear material out of the parts' way, so they are
        // checked too (in cutting order), though only these are reported
        let earlier: Vec<(EntityId, cut_craft_core::toolpath::Toolpath)> = (source.toolpaths)
            .iter()
            .take_while(|(id, _)| !checked.contains(id))
            .map(|(&id, t)| (id, t.clone()))
            .collect();
        let (job, holding) = (source.job, self.holding());
        let clamps: Vec<_> = source.named_keep_out().cloned().collect();
        drop(source);
        let grid = self.preview_grid();
        let start = (self.simulation.clone())
            .filter(|m| !fresh && m.lies_on(&self.doc.job))
            .map_or_else(|| self.fresh_material(), |m| m.including(grid.area));
        // on a rotary job, the file's own moves turning the material as well, carrying on
        // from what the toolpaths before these cut when the preview does
        let carries_on = (self.simulation.as_ref()).is_some_and(|m| !fresh && m.lies_on(&job));
        let making = self.turning_making(
            if carries_on {
                earlier.iter().map(|(_, t)| t.clone()).collect()
            } else {
                vec![]
            },
            list.clone(),
        );
        // each move as long as the Toolpath Summary times it
        let motion = self.motion();
        let revision = self.history.revision();
        let what = match &list[..] {
            [one] => format!("Getting {} ready to preview", one.name),
            _ => format!("Getting {} toolpaths ready to preview", list.len()),
        };
        self.in_background(
            &what,
            "preview:ready",
            move || {
                let list: Vec<&cut_craft_core::toolpath::Toolpath> = list.iter().collect();
                let collided = {
                    let ids: Vec<EntityId> =
                        earlier.iter().map(|(id, _)| *id).chain(checked).collect();
                    let all: Vec<&cut_craft_core::toolpath::Toolpath> = earlier
                        .iter()
                        .map(|(_, t)| t)
                        .chain(list.iter().copied())
                        .collect();
                    let mut collided =
                        super::super::background::collided(&job, &ids, &all, (&holding, &clamps));
                    collided.retain(|id, _| !earlier.iter().any(|(e, _)| e == id));
                    collided
                };
                let playback = cut_craft_core::playback::Playback::planned(start, &list, &motion)
                    .marked(&marks);
                let turning = making.map(super::super::turning::Making::make);
                (playback, collided, turning)
            },
            move |app, (playback, collided, turning)| {
                if let Some(hit) = super::super::background::collision_note(&collided) {
                    app.status.set_message(&hit);
                }
                app.collisions = collided;
                app.player.start(playback, revision);
                app.turning = turning;
                app.simulation = None;
                (app.simulation_coarse, app.fine_3d, app.surface_gpu) = (None, None, None);
                if app.display.auto_open_3d {
                    app.view = View::ThreeD;
                }
                then(app);
            },
        );
    }

    /// Once a frame: a playing preview's tool moved on at its speed, or toward where it was
    /// sent, cutting for no longer than the frame's budget; the rest waits for the next
    /// frame, so the window answers (Stop, Esc) however long the toolpath. At the end, the
    /// cut material becomes the preview.
    pub(in super::super) fn advance_playback(&mut self, ctx: &egui::Context) {
        self.drop_stale_playback();
        self.drop_preview_of_old_block();
        let dt = f64::from(ctx.input(|i| i.stable_dt));
        let frame = self.player.advance(dt, std::time::Instant::now());
        if frame.repaint {
            ctx.request_repaint();
        }
        if let Some(after) = frame.repaint_after {
            ctx.request_repaint_after(after);
        }
        if frame.finished {
            self.finish_playback();
        }
        self.follow_turning(ctx);
    }

    /// Once the job has changed (an edit, a recalculation, Undo), a preview's moves are old:
    /// a playing one stops, keeping what it has cut, and its controls close, so Run or a
    /// drag never plays the old moves over the job again.
    pub(super) fn drop_stale_playback(&mut self) {
        if !self.player.stale(self.history.revision()) {
            return;
        }
        self.finish_playback();
        self.player.kept = None;
        self.status
            .set_message(&cut_craft_i18n::tr!("status-preview-closed-job-changed"));
    }

    /// Once Job Setup changes the block's size, thickness or place, the preview of the old
    /// block goes (Undo Last Preview's too), so the 3D view shows the new block and the
    /// next Preview cuts into that.
    pub(super) fn drop_preview_of_old_block(&mut self) {
        let job = self.doc.job;
        let old = |m: &Material| !m.lies_on(&job);
        if self.simulation.as_ref().is_some_and(old)
            || (self.before_preview.as_ref()).is_some_and(|b| b.0.as_ref().is_some_and(old))
        {
            (self.simulation, self.before_preview) = (None, None);
            (self.simulation_coarse, self.fine_3d, self.surface_gpu) = (None, None, None);
            self.status
                .set_message(&cut_craft_i18n::tr!("status-preview-new-material"));
        }
    }

    /// Ends a playing preview, keeping what it has cut (Stop, or the end of the path), and
    /// the playback, so its controls stay to scrub it again.
    pub(in super::super) fn finish_playback(&mut self) {
        if let Some(cut) = self.player.finish() {
            self.simulation = Some(cut);
            (self.simulation_coarse, self.fine_3d, self.surface_gpu) = (None, None, None);
        }
    }

    pub(in super::super) fn preview_toolpath(&mut self, id: EntityId) {
        if self.display.animate_preview {
            self.finish_playback();
            self.before_preview = Some(super::super::BeforePreview(
                self.simulation.clone(),
                self.preview_side,
            ));
            self.play(&[id], false, |_| {});
            return;
        }
        self.finish_playback();
        if !self.preview_source().toolpaths.contains_key(&id) {
            return;
        }
        self.before_preview = Some(super::super::BeforePreview(
            self.simulation.clone(),
            self.preview_side,
        ));
        // cut into the preview so far (on a block Job Setup has not changed since)
        let grid = self.preview_grid();
        let start = (self.simulation.clone())
            .filter(|m| m.lies_on(&self.doc.job))
            .map_or_else(|| self.fresh_material(), |m| m.including(grid.area));
        self.preview_in_background(&[id], start, |_| {});
    }

    /// The Preview Control while a preview plays: Run / Pause, Single Step, Run to
    /// Retract and Stop, the speed, the tool's place along the whole path and close up
    /// (drag to scrub), and how long is left. Over both views, so Stop is always to hand; Esc
    /// stops it too. Once stopped or played through they stay, to scrub it again, with
    /// Close in place of Stop, till another preview, Reset Preview or another job.
    pub(in super::super) fn playback_bar(&mut self, ui: &mut egui::Ui) {
        cut_craft_bug_report::mark(ui, "Toolpath playback bar");
        let kept = self.player.playback.is_none();
        let Some(playback) = self.player.playback.as_ref().or(self.player.kept.as_ref()) else {
            return;
        };
        let (at, length) = (playback.at(), playback.length());
        // from the playback itself: while it plays there is no finished simulation
        let fine_bit = crate::display::fine_bit_note(
            playback.tools(),
            playback.material().cell,
            self.display.detail,
        );
        let guessed_shank = playback
            .tool()
            .filter(|(_, tool)| tool.shank_diameter.is_nan() || tool.shank_diameter <= 0.0)
            .map(|(_, tool)| tool.name.clone());
        let (mut stop, mut run, mut goal) = (false, false, None);
        ui.horizontal(|ui| {
            let label = if self.player.playing { "Pause" } else { "Run" };
            run = ui.button(label).clicked();
            if ui.button(tr!("panels-single-step")).clicked() {
                goal = Some(playback.next_step());
            }
            if ui.button(tr!("panels-run-to-retract")).clicked() {
                goal = Some(playback.next_retract());
            }
            stop = if kept {
                ui.button(tr!("panels-close"))
                    .on_hover_hint(tr!("panels-hide-these-controls-the-preview"))
                    .clicked()
            } else {
                ui.button(tr!("panels-stop"))
                    .on_hover_hint(tr!("panels-stop-the-preview-here-keeping"))
                    .clicked()
            };
            ui.add(
                egui::Slider::new(&mut self.player.speed, PLAYBACK_SPEEDS)
                    .logarithmic(true)
                    .prefix("×")
                    .custom_formatter(|x, _| times(x))
                    .text("Speed"),
            )
            .on_hover_hint(tr!("panels-1-plays-at-the-machine"));
        });
        if let Some(to) = self.scrub_sliders(ui, self.player.goal.unwrap_or(at), length) {
            goal = Some(to);
        }
        ui.weak(playback_note(
            at,
            length,
            self.player.goal,
            self.player.playing,
            kept,
            self.player.speed,
        ));
        if let Some(name) = guessed_shank {
            ui.weak(tr!("panels-shank-guessed", name = name.as_str()));
        }
        if let Some(note) = fine_bit {
            ui.weak(note);
        }
        if (run || goal.is_some()) && kept {
            self.resume_playback();
        }
        if run {
            let playback = self.player.playback.as_mut().expect("resumed");
            // Run at the end plays it again from the start
            if !self.player.playing && playback.finished() {
                playback.seek(0.0);
            }
            self.player.playing = !self.player.playing;
            self.player.goal = None;
        }
        if let Some(goal) = goal {
            // sent somewhere: it pauses there, cut a frame's worth at a time
            (self.player.playing, self.player.goal) = (false, Some(goal));
        }
        if stop && kept {
            self.player.kept = None;
        } else if stop {
            self.finish_playback();
        }
        ui.separator();
    }

    /// The preview's Progress (the whole path) and Close up (the minutes either side of the
    /// tool, kept where it was while dragged) sliders at `shown` of `length` seconds: where
    /// the tool is sent, if either was dragged.
    pub(super) fn scrub_sliders(
        &mut self,
        ui: &mut egui::Ui,
        shown: f64,
        length: f64,
    ) -> Option<f64> {
        let mut sent = None;
        // the whole path: drag it to move the tool on through the toolpaths, or back
        let percent = move |d: f64| {
            if length > 0.0 {
                d / length * 100.0
            } else {
                100.0
            }
        };
        let row = |ui: &mut egui::Ui, label: &str, slider: egui::Slider<'_>, hint: &str| {
            ui.horizontal(|ui| {
                ui.label(label);
                ui.spacing_mut().slider_width =
                    ui.available_width() - ui.spacing().interact_size.x * 2.5;
                ui.add(slider).on_hover_hint(hint)
            })
            .inner
        };
        let mut whole = shown;
        if row(
            ui,
            "Progress",
            egui::Slider::new(&mut whole, 0.0..=length)
                .custom_formatter(move |d, _| format!("{:.1}%", percent(d)))
                .custom_parser(move |text| {
                    let p: f64 = text.trim().trim_end_matches('%').trim().parse().ok()?;
                    Some(p / 100.0 * length)
                }),
            "Where the tool is along the toolpaths: drag to move it on or back",
        )
        .changed()
        {
            sent = Some(whole);
        }
        // close up: the minutes either side of the tool, kept where it was while dragged
        let centre = self.close_up.unwrap_or(shown);
        let (from, to) = (
            (centre - CLOSE_UP).max(0.0),
            (centre + CLOSE_UP).min(length),
        );
        let mut near = shown.clamp(from, to);
        let close = row(
            ui,
            "Close up",
            egui::Slider::new(&mut near, from..=to)
                .custom_formatter(|d, _| super::super::hms(d / 60.0))
                .custom_parser(|_| None),
            "The few minutes of machine time either side of the tool: drag to go back and \
             forth over one spot",
        );
        self.close_up = (close.is_pointer_button_down_on() || close.dragged()).then_some(centre);
        if close.changed() {
            sent = Some(near);
        }
        sent
    }

    /// A stopped preview's playback taken up again, to play or scrub it: what it shows is
    /// its material again.
    pub(super) fn resume_playback(&mut self) {
        if self.player.resume() {
            self.simulation = None;
            (self.simulation_coarse, self.fine_3d, self.surface_gpu) = (None, None, None);
        }
    }
}
