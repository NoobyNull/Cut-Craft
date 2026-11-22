//! A tool form's contents: the settings, suggestions, textures, extras and the Depth add-on's
//! section.

use super::machine_setup::bed;
use super::{
    CutCraftApp, OpenForm, SECTION_PICTURE_ROWS, Value, brush_dialog, egui, form, rest_section,
    saving_path, selector_section, texture_dialog, tool_picker,
};
use cut_craft_bug_report::hint::HoverHint;
use cut_craft_i18n::tr;

impl CutCraftApp {
    /// The open form's heading and settings; [`Self::form_panel`] pins its buttons under it.
    #[expect(
        clippy::too_many_lines,
        reason = "one form's closures (tool, group ends, pictures) set up in one place"
    )]
    pub(super) fn command_form(&mut self, ui: &mut egui::Ui) {
        let form = self.form.as_ref().map_or("none", |f| f.label);
        cut_craft_bug_report::mark(ui, &format!("Tool form: {form}"));
        let slots = self.slots();
        self.form_heading(ui);
        let reach = self.form_reach();
        let room = self.placement_room();
        let Some(open) = &mut self.form else { return };
        // a form that sets the units (Job Setup) shows its lengths in them as they change
        let edge = SECTION_PICTURE_ROWS * ui.spacing().interact_size.y;
        let picture = |path: &str| {
            self.icons.find(path, "").map(|p| {
                self.icons
                    .image(p)
                    .fit_to_exact_size(egui::vec2(edge, edge))
            })
        };
        let units = form::units_chosen(&open.params).unwrap_or(self.doc.job.units);
        // the tool block, where the form has its tool: what was picked, and Edit…
        let asked = std::cell::Cell::new(None);
        // a list of tools whose Select…, Edit… or Add a tool… was clicked
        let list_ask = std::cell::RefCell::new(None);
        let listed = std::cell::Cell::new(false);
        let tools = &self.tools;
        let layers: Vec<String> = self.doc.layers.iter().map(|l| l.name.clone()).collect();
        let finishing_smallest = open.id != "vcarve_engrave_toolpath";
        // a form that only uses its tool's shape (Relief from Photo) has no passes to cut
        let cuts = open.id != "relief_from_photo";
        // a surfacing toolpath flattens all it covers, 3D Pencil has its own bits that
        // finished before and follows the model, and 4-Axis Rotary Machining goes all the way
        // round: no rest machining or vector selector
        let extras = cuts
            && ![
                "surfacing_toolpath",
                "3d_pencil_toolpath",
                "rotary_machining_toolpath",
                "rotary_lathe_toolpath",
            ]
            .contains(&open.id);
        let suggested = open.suggested.clone();
        let form_id = open.id;
        let tool = |ui: &mut egui::Ui, key: &str, params: &mut Value| {
            if key == "clearance_tools" {
                let (changed, ask) = super::clearing_section(
                    ui,
                    params,
                    &slots,
                    finishing_smallest,
                    suggested.as_deref(),
                    units,
                );
                listed.set(listed.get() | changed);
                if let Some(ask) = ask {
                    *list_ask.borrow_mut() = Some((super::ToolList::Clearing, ask));
                }
                return;
            }
            if key != "tool" {
                // the form's tools in the order they cut, each picked as the form's own tool
                let (changed, ask) = super::tools_section(ui, key, params, tools, &slots, units);
                listed.set(listed.get() | changed);
                if let Some(ask) = ask {
                    *list_ask.borrow_mut() = Some((super::ToolList::of_key(key), ask));
                }
                return;
            }
            ui.group(|ui| {
                ui.set_width(ui.available_width());
                ui.strong(tr!("actions-tool"));
                asked.set(tool_picker(ui, form_id, params, tools, &slots, units));
                if let Some(line) = &reach {
                    super::reach::show(ui, line);
                }
                listed.set(listed.get() | super::passes::edit_passes(ui, params, units));
            });
            if extras
                && super::shows_rest(params)
                && let Some(ask) = rest_section(ui, params, tools, &slots, units)
            {
                *list_ask.borrow_mut() = Some((super::ToolList::Rest, ask));
            }
            if extras {
                selector_section(ui, params, &layers, units);
            }
        };
        let stock = form_stock(&self.doc, &open.params, units, bed(&self.machines));
        let group_end = |ui: &mut egui::Ui, group: &str, params: &mut Value| {
            super::tabs::group_end(ui, group, params, stock)
                | super::placement::section_end(ui, group, params, room)
                | super::placement::reset_end(ui, group, params)
        };
        let look = form::Look {
            units,
            picture: &picture,
            material: form::material(&self.doc, units),
            tool: Some(&tool),
            group_end: Some(&group_end),
            materials: Some(&self.materials),
            pattern: Some(pattern_maker()),
        };
        let icon = |path: &str| self.icons.find(path, "").map(|p| self.icons.image(p));
        // Trace Bitmap's settings are in its trace window, beside the preview
        if open.id != "trace_bitmap" {
            open.dirty |= show_form(ui, open, &look, &self.doc.job, &icon);
        }
        open.dirty |= listed.get();
        self.form_extras(ui, form_id);
        if let Some(ask) = asked.get() {
            let name = self
                .form
                .as_ref()
                .and_then(|f| f.params["tool"]["name"].as_str().map(str::to_owned));
            match ask {
                super::ToolAsk::Select => self.select_tool(name.as_deref().unwrap_or_default()),
                super::ToolAsk::Edit => self.edit_tool(ui.ctx(), name),
            }
        }
        if let Some((which, ask)) = list_ask.into_inner() {
            self.ask_listed(ui.ctx(), which, ask);
        }
    }

    /// A list of tools' Select…, Add a tool… or Edit…: the Tool Database, to pick a tool of
    /// list `which` in or change one.
    pub(super) fn ask_listed(
        &mut self,
        ctx: &egui::Context,
        which: super::ToolList,
        ask: super::ListAsk,
    ) {
        match ask {
            super::ListAsk::Select(at) => {
                let current = match at {
                    super::ListSlot::At(i) => self.form.as_ref().and_then(|form| {
                        let list = match which {
                            super::ToolList::Clearing => &form.params["clearance_tools"],
                            super::ToolList::Tools => &form.params["tools"],
                            super::ToolList::Before => &form.params["bits_before"],
                            super::ToolList::Rest => &form.params["rest"]["after"],
                        };
                        list[i]["name"].as_str().map(str::to_owned)
                    }),
                    super::ListSlot::New => None,
                };
                self.select_tool(&current.unwrap_or_default());
                self.picking = Some(super::PickFor::Listed(which, at));
            }
            super::ListAsk::Edit(name) => self.edit_tool(ctx, Some(name)),
            super::ListAsk::Suggest => self.suggest_clearing(),
        }
    }

    /// The bits Suggest may pick from: the user's own (not a read-only catalogue's, nor a
    /// laser), each with whether the tool changer holds it.
    pub(super) fn suggest_candidates(&self) -> Vec<cmd_pocket_toolpath::Candidate> {
        let slots = self.slots();
        self.tools
            .my_tools()
            .filter(|tool| tool.shape != cut_craft_core::toolpath::ToolShape::Laser)
            .map(|tool| cmd_pocket_toolpath::Candidate {
                tool: self.pick_tool(&tool.name).unwrap_or_else(|| tool.clone()),
                loaded: slots
                    .iter()
                    .any(|s| s.as_deref() == Some(tool.name.as_str())),
            })
            .collect()
    }

    /// Pocket's Suggest clearing bits: every plan of one or two bigger bits calculated and
    /// timed off the window's thread; the quickest becomes the form's clearing bits, and
    /// the form says how long it takes against the finishing bit alone.
    pub(super) fn suggest_clearing(&mut self) {
        let Some(form) = &self.form else { return };
        let params: cmd_pocket_toolpath::Params = match serde_json::from_value(form.params.clone())
        {
            Ok(p) => p,
            Err(e) => {
                self.status
                    .set_message(&format!("Suggest could not read the form: {e}"));
                return;
            }
        };
        let candidates = self.suggest_candidates();
        let finishing_loaded = self
            .slots()
            .iter()
            .any(|s| s.as_deref() == Some(params.tool.name.as_str()));
        let changes = cmd_pocket_toolpath::Changes {
            by_hand: self.options.toolpaths.change_by_hand,
            by_changer: self.options.toolpaths.change_by_changer,
        };
        let (doc, rapid) = (self.doc.clone(), self.summary.rapid);
        let finishing = params.tool.name.clone();
        self.in_background(
            "Suggesting clearing bits",
            "suggest-clearing",
            move || {
                cmd_pocket_toolpath::suggest(
                    &doc,
                    &params,
                    &candidates,
                    finishing_loaded,
                    rapid,
                    changes,
                )
                .map_err(|e| e.to_string())
            },
            move |app, made| {
                let Some(form) = &mut app.form else { return };
                match made {
                    Ok(s) => {
                        let note = suggestion_note(&s, &finishing);
                        form.params["clearance_tools"] =
                            serde_json::to_value(&s.clearing).expect("tools serialize");
                        form.dirty = true;
                        form.suggested = Some(note.clone());
                        app.status.set_message(&note);
                    }
                    Err(e) => form.suggested = Some(format!("Suggest: {e}")),
                }
            },
        );
    }

    /// Texture Toolpath's Load… or Save…: the texture as a `.vtexture` file, in the textures
    /// folder unless taken elsewhere.
    pub(super) fn texture_file(&mut self, save: bool) {
        let extension = cmd_texture_toolpath::TEXTURE_EXTENSION;
        let picked = if save {
            texture_dialog()
                .set_file_name(format!("texture.{extension}"))
                .save_file()
                .and_then(|path| saving_path(&path, &[extension]))
        } else {
            texture_dialog().pick_file()
        };
        let Some(path) = picked else { return };
        let result = if save {
            self.save_texture(&path)
        } else {
            self.load_texture(&path)
        };
        if let Err(e) = result {
            self.status.set_message(&e);
        }
    }

    /// Writes the open Texture Toolpath form's texture to `path`.
    pub(super) fn save_texture(&mut self, path: &std::path::Path) -> Result<(), String> {
        let texture = self.texture_params()?.to_vtexture();
        std::fs::write(path, texture)
            .map_err(|e| format!("cannot write {}: {e}", path.display()))?;
        self.status
            .set_message(&format!("Saved texture {}", path.display()));
        Ok(())
    }

    /// Reads the texture in `path` into the open Texture Toolpath form; its name, tool,
    /// seed and boundary offset stay.
    pub(super) fn load_texture(&mut self, path: &std::path::Path) -> Result<(), String> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
        let params = self.texture_params()?.with_vtexture(&text)?;
        let Some(form) = &mut self.form else {
            return Err("no Texture Toolpath form is open".into());
        };
        let mut loaded = serde_json::to_value(params).expect("texture settings serialise");
        // what the form holds besides the texture's settings (its Vector Selector) stays
        if let (Some(loaded), Some(old)) = (loaded.as_object_mut(), form.params.as_object()) {
            for (key, value) in old {
                loaded.entry(key.clone()).or_insert_with(|| value.clone());
            }
        }
        form.params = loaded;
        form.dirty = true;
        self.status
            .set_message(&format!("Loaded texture {}", path.display()));
        Ok(())
    }

    /// The open Texture Toolpath form's settings.
    fn texture_params(&self) -> Result<cmd_texture_toolpath::Params, String> {
        let form = self.form.as_ref().filter(|f| f.id == "texture_toolpath");
        let form = form.ok_or("no Texture Toolpath form is open")?;
        serde_json::from_value(form.params.clone())
            .map_err(|e| format!("the form's settings do not read: {e}"))
    }

    /// Set Size with Link XY: the box typed in sets the other, keeping the selection's
    /// proportions (in Percent, the same share), so the form always shows the size it makes,
    /// and sizes go by it. Items one by one each keep their own proportions, so there the
    /// other box (in millimetres) is left as it is.
    fn link_set_size(&mut self) {
        let size = self
            .doc
            .selection_bounds()
            .map(cut_craft_core::geometry::Rect::size);
        let Some(form) = self.form.as_mut().filter(|f| f.id == "set_size") else {
            return;
        };
        let p = &mut form.params;
        let (Some(mut w), Some(mut h)) = (p["width"].as_f64(), p["height"].as_f64()) else {
            return;
        };
        let seen = form.shown_size.map(|s| (s[0], s[1]));
        let same = |a: f64, b: f64| (a - b).abs() <= 1e-9 * a.abs().max(1.0);
        let linked = p["keep_proportions"].as_bool() == Some(true);
        if linked && let Some((sw, sh)) = seen {
            // the box typed last is the one sizes go by
            if same(w, sw) && !same(h, sh) {
                p["by_height"] = true.into();
            } else if !same(w, sw) {
                p["by_height"] = false.into();
            }
        }
        let items = p["mode"].as_str() == Some("scale_items_individually");
        let ratio = if p["percent"].as_bool() == Some(true) {
            Some(1.0)
        } else if items {
            None
        } else {
            size.filter(|s| s.x > 0.0 && s.y > 0.0).map(|s| s.y / s.x)
        };
        if let Some(ratio) = ratio.filter(|_| linked) {
            let height_typed = p["by_height"].as_bool() == Some(true);
            if height_typed {
                w = h / ratio;
                p["width"] = w.into();
            } else if !same(h, w * ratio) {
                h = w * ratio;
                p["height"] = h.into();
            }
        }
        form.shown_size = Some([w, h, 0.0]);
    }

    /// What some forms have under their fields: Texture Toolpath's Load / Save Texture, and
    /// Quick Engrave's Immediate Output.
    fn form_extras(&mut self, ui: &mut egui::Ui, id: &str) {
        match id {
            "texture_toolpath" => match texture_buttons(ui) {
                (true, _) => self.texture_file(false),
                (_, true) => self.texture_file(true),
                _ => {}
            },
            "draw_text" => self.spelling(ui),
            "job_setup" => self.setup_material_extras(ui),
            "set_size" => self.link_set_size(),
            super::generate_border::ID => self.border_section(ui),
            "relief_from_photo" => {
                self.depth_section(ui);
                self.bit_note(ui);
            }
            "profile_toolpath" => {
                self.edit_tabs(ui);
                self.deep_cut_warning(ui.ctx());
            }
            "trace_bitmap" => self.trace_form(ui),
            "sculpt" => {
                let (load, round) = ui
                    .horizontal(|ui| {
                        let load = ui
                            .button(tr!("sculpt-load-brush"))
                            .on_hover_hint(tr!("sculpt-load-brush.tooltip"))
                            .clicked();
                        let round = ui.button(tr!("sculpt-round-brush")).clicked();
                        (load, round)
                    })
                    .inner;
                let brush = if round {
                    Some(String::new())
                } else if load {
                    brush_dialog().pick_file().map(|p| p.display().to_string())
                } else {
                    None
                };
                if let (Some(brush), Some(form)) = (brush, self.form.as_mut()) {
                    form.params["brush"] = brush.into();
                }
            }
            "quick_engrave_toolpath" => {
                let choices = self.post_choices();
                let default = self.output_post().map(|p| p.name);
                let two_sided = self.doc.job.double_sided.is_some();
                let ready = self.immediate_output_toolpath().is_some();
                let output = &mut self.immediate_output;
                if super::views::immediate_output(ui, output, &default, &choices, two_sided, ready)
                {
                    self.immediate_save();
                }
            }
            _ => {}
        }
    }

    /// Relief from Photo, with Only What the Bit Can Carve ticked: says when the job's 3D
    /// model is too coarse to hold what the bit can cut, and what makes it finer.
    fn bit_note(&self, ui: &mut egui::Ui) {
        use cut_craft_core::document::Resolution;
        let Some(form) = &self.form else { return };
        if form.params["only_what_the_bit_can_carve"] != true {
            return;
        }
        let tip = form.params["tool"]["diameter"].as_f64().unwrap_or(0.0);
        let job = &self.doc.job;
        let finest = (job.width * job.height / Resolution::Highest.points()).sqrt();
        let (per, unit) = form::shown_in("mm", job.units).unwrap_or((1.0, "mm"));
        let places = if unit == "in" { 3 } else { 2 };
        let length = |mm: f64| format!("{:.places$} {unit}", mm / per);
        if let Some(note) =
            cmd_relief_from_photo::resolution_note(job.model_cell(), finest, tip, length)
        {
            ui.label(note);
        }
    }

    /// Relief from Photo's Depth: Estimate Depth when the add-on is installed, else how to
    /// get it.
    fn depth_section(&mut self, ui: &mut egui::Ui) {
        let installed = self.addons.dir().and_then(crate::depth::find);
        let chosen = self
            .doc
            .selected_bitmaps
            .iter()
            .rev()
            .copied()
            .find(|id| self.doc.bitmaps.contains_key(id))
            .or_else(|| self.doc.bitmaps.keys().next_back().copied());
        let has_depth = chosen.is_some_and(|id| {
            let b = &self.doc.bitmaps[&id];
            !b.grey.is_empty() && b.depth.len() == b.grey.len()
        });
        let from_picture = self
            .form
            .as_ref()
            .is_some_and(|f| f.params["depth_from"] == "picture");
        let mut start = false;
        ui.group(|ui| {
            ui.set_width(ui.available_width());
            ui.strong(tr!("depth-title"));
            if from_picture {
                ui.label(tr!("depth-from-picture"));
                return;
            }
            let Some(_) = &installed else {
                ui.label(tr!("depth-needs-addon"));
                self.depth_install(ui);
                return;
            };
            if self.depth_estimate.is_some() {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(tr!("depth-estimating"));
                });
                return;
            }
            ui.label(tr!(if has_depth {
                "depth-has-depth"
            } else {
                "depth-estimate-first"
            }));
            start = ui
                .add_enabled(chosen.is_some(), egui::Button::new(tr!("depth-estimate")))
                .on_hover_hint(tr!("depth-estimate.tooltip"))
                .on_disabled_hover_text(tr!("depth-estimate.disabled"))
                .clicked();
            self.depth_detail(
                ui,
                installed
                    .as_ref()
                    .and_then(|i| i.detailed.as_ref())
                    .is_some(),
            );
        });
        let name = "The Depth add-on";
        let high_detail = installed
            .as_ref()
            .is_some_and(|i| self.depth_high_detail.unwrap_or(i.detailed.is_some()));
        if let (true, Some(installed), Some(id)) = (start, installed, chosen)
            && self.addons.may_run(crate::depth::ADDON_ID, name)
        {
            let installed = installed.with_high_detail(high_detail);
            match crate::depth::Estimating::start(&installed, id, &self.doc.bitmaps[&id], ui.ctx())
            {
                Ok(running) => self.depth_estimate = Some(running),
                Err(e) => self.status.set_message(&e),
            }
        }
    }

    /// Relief from Photo's High detail: the tick once the model is here (`have`), else the
    /// offer to fetch it, or how its download is going.
    fn depth_detail(&mut self, ui: &mut egui::Ui, have: bool) {
        if let Some(answer) = self
            .depth_detail_fetch
            .as_ref()
            .and_then(crate::depth::FetchingDetail::answer)
        {
            self.depth_detail_fetch = None;
            match answer {
                Ok(()) => {
                    self.depth_detail_failed = None;
                    self.depth_high_detail = Some(true);
                }
                Err(e) => self.depth_detail_failed = Some(e),
            }
        }
        if let Some(fetch) = &self.depth_detail_fetch {
            ui.label(tr!("depth-downloading-high-detail"));
            ui.add(egui::ProgressBar::new(fetch.fraction()).show_percentage());
            return;
        }
        if have {
            let mut on = self.depth_high_detail.unwrap_or(true);
            if ui
                .checkbox(&mut on, tr!("depth-high-detail"))
                .on_hover_hint(crate::depth::DETAILED_NOTE)
                .changed()
            {
                self.depth_high_detail = Some(on);
            }
            if on {
                ui.label(tr!("depth-high-detail-note"));
            }
            return;
        }
        ui.label(tr!("depth-want-finer"));
        if let Some(e) = &self.depth_detail_failed {
            ui.colored_label(ui.visuals().error_fg_color, e);
        }
        if ui
            .button(tr!("depth-get-high-detail"))
            .on_hover_hint(crate::depth::DETAILED_NOTE)
            .clicked()
            && let Some(addons) = self.addons.dir().map(std::path::Path::to_path_buf)
        {
            match crate::depth::FetchingDetail::start(&addons, ui.ctx()) {
                Ok(fetch) => {
                    self.depth_detail_failed = None;
                    self.depth_detail_fetch = Some(fetch);
                }
                Err(e) => self.depth_detail_failed = Some(e),
            }
        }
    }

    /// Install Depth Add-on, or how its install is going (the Add-ons window's install).
    fn depth_install(&mut self, ui: &mut egui::Ui) {
        use crate::addons::Phase;
        match &self.addons.phase {
            Phase::Finding(_) => {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(tr!("depth-addon-finding"));
                });
            }
            Phase::Installing { done, total, .. } => {
                ui.label(tr!("depth-addon-downloading"));
                #[expect(clippy::cast_precision_loss, reason = "a fraction for the bar")]
                let fraction = if *total > 0 {
                    *done as f32 / *total as f32
                } else {
                    0.0
                };
                ui.add(egui::ProgressBar::new(fraction.min(1.0)).show_percentage());
            }
            Phase::Asking(_) => {
                ui.label(tr!("depth-addon-asking"));
            }
            phase => {
                if let Phase::Failed(e) = phase {
                    ui.colored_label(ui.visuals().error_fg_color, e);
                }
                let can = self.addons.dir().is_some();
                if ui
                    .add_enabled(can, egui::Button::new(tr!("depth-addon-install")))
                    .on_hover_hint(tr!("depth-addon-install.tooltip"))
                    .on_disabled_hover_text(tr!("depth-addon-install.disabled"))
                    .clicked()
                {
                    self.install_depth(ui.ctx());
                }
            }
        }
    }

    /// Starts installing the official Depth add-on.
    pub(super) fn install_depth(&mut self, ctx: &egui::Context) {
        let depth = cut_craft_updater::addon::OFFICIAL
            .iter()
            .find(|a| a.id == crate::depth::ADDON_ID)
            .expect("Depth is an official add-on");
        self.addons.install_from(ctx, depth.repo);
    }

    /// Gives the picture its depth once the Depth add-on answers (undoable), and shows the
    /// form's preview again.
    pub(super) fn take_depth(&mut self) {
        let Some(answer) = self
            .depth_estimate
            .as_ref()
            .and_then(crate::depth::Estimating::answer)
        else {
            return;
        };
        let sent = self.depth_estimate.take().expect("answered");
        let id = sent.bitmap;
        let result = answer.and_then(|depth| {
            let bitmap = self
                .doc
                .bitmaps
                .get(&id)
                .ok_or("the photo was deleted while its depth was estimated")?;
            // another job opened, or the photo edited, while the add-on worked
            if *bitmap != sent.photo || depth.len() != bitmap.grey.len() {
                return Err("the photo changed while its depth was estimated; try again".into());
            }
            let after = crate::depth::with_depth(bitmap, depth);
            let mut c = cut_craft_core::Change::builder(&self.doc);
            c.replace_bitmap(id, after);
            let change = c.finish();
            self.history
                .apply(&mut self.doc, change)
                .map_err(|e| e.to_string())
        });
        match result {
            Ok(()) => {
                self.status.set_message("Depth estimated");
                self.relief_after_estimate();
            }
            Err(e) => self.status.set_message(&e),
        }
    }
}

impl CutCraftApp {
    /// Once a photo has its depth: the relief Relief from Photo already made follows it,
    /// and with none made yet, one is made, as Estimate Depth is the step before it.
    fn relief_after_estimate(&mut self) {
        let made = self.drawn_shape().is_some();
        let Some(form) = self.form.as_mut() else {
            return;
        };
        form.dirty = true;
        let waiting = form.id == "relief_from_photo" && form.params["depth_from"] == "estimated";
        if waiting && !made {
            form.dirty = false;
            let params = form.params.clone();
            self.calculate_checked("relief_from_photo", &params);
        }
    }
}

/// Texture Toolpath's Load… and Save…; whether each was clicked.
pub(super) fn texture_buttons(ui: &mut egui::Ui) -> (bool, bool) {
    let mut clicked = (false, false);
    ui.horizontal(|ui| {
        clicked.0 = ui
            .button(tr!("texture-load"))
            .on_hover_hint(tr!("texture-load.tooltip"))
            .clicked();
        clicked.1 = ui
            .button(tr!("texture-save"))
            .on_hover_hint(tr!("texture-save.tooltip"))
            .clicked();
    });
    clicked
}

/// The open form's fields, and Import 3D Model's alignment and Update Undercuts; true when
/// anything changed.
pub(super) fn show_form(
    ui: &mut egui::Ui,
    open: &mut OpenForm,
    look: &form::Look,
    job: &cut_craft_core::document::Job,
    icon: &dyn Fn(&str) -> Option<egui::Image<'static>>,
) -> bool {
    // what the pointer was on last frame; the fields note it again this frame
    form::forget_help(ui.ctx());
    let changed = form::show(ui, &open.fields, &mut open.params, look);
    changed
        | align_in_material(ui, open, job, icon)
        | update_undercuts(ui, open)
        | scale_to_fit(ui, open, job)
}

/// Import 3D Model's Scale to Fit Material, the form's last choice: the model scaled once
/// to be as big as the material allows, turned as it is now, and kept at that scale while
/// it is turned after. Greyed until Change size is ticked, so the size never changes
/// unseen; true when clicked.
pub(super) fn scale_to_fit(
    ui: &mut egui::Ui,
    open: &mut OpenForm,
    job: &cut_craft_core::document::Job,
) -> bool {
    if open.id != "orientate_3d_model" {
        return false;
    }
    let unlocked = open.params["size"]["change_size"]
        .as_bool()
        .unwrap_or(false);
    let clicked = ui
        .add_enabled(unlocked, egui::Button::new(tr!("scale-to-fit")))
        .on_hover_hint(tr!("scale-to-fit.tooltip"))
        .on_disabled_hover_text(tr!("scale-to-fit.disabled"))
        .clicked();
    if !clicked {
        return false;
    }
    let doc = cut_craft_core::document::Document {
        job: *job,
        ..cut_craft_core::document::Document::default()
    };
    let fits = serde_json::from_value(open.params.clone())
        .map_err(|e| e.to_string())
        .and_then(|p| cmd_orientate_3d_model::fit_scale(&doc, &p).map_err(|e| e.to_string()));
    match fits {
        Ok(s) => {
            open.params["size"]["scale"] = serde_json::json!([s, s, s]);
            open.typed_size = [None; 3];
            true
        }
        Err(e) => {
            open.error = Some(e);
            false
        }
    }
}

/// Import 3D Model's alignment, as Align Objects' in the material: the model against the
/// material's sides or centred on it, and in height on the zero plane, in the middle of the
/// material or at its top. The position numbers are set to match; true when clicked.
pub(super) fn align_in_material(
    ui: &mut egui::Ui,
    open: &mut OpenForm,
    job: &cut_craft_core::document::Job,
    icon: &dyn Fn(&str) -> Option<egui::Image<'static>>,
) -> bool {
    /// Each button: its icon, hint, and how far across and up the room it puts the model.
    type Align = (&'static str, &'static str, Option<f64>, Option<f64>);
    const ACROSS: [Align; 7] = [
        ("left", "Left side on the material's left", Some(0.0), None),
        (
            "center_horizontally_in_material",
            "Centred left to right",
            Some(0.5),
            None,
        ),
        (
            "right",
            "Right side on the material's right",
            Some(1.0),
            None,
        ),
        ("top", "Top side on the material's top", None, Some(1.0)),
        (
            "center_vertically_in_material",
            "Centred top to bottom",
            None,
            Some(0.5),
        ),
        (
            "bottom",
            "Bottom side on the material's bottom",
            None,
            Some(0.0),
        ),
        (
            "center_in_material",
            "Centred on the material",
            Some(0.5),
            Some(0.5),
        ),
    ];
    const HEIGHT: [(&str, &str, f64); 3] = [
        ("Bottom", "The model's bottom on the zero plane", 0.0),
        (
            "Middle",
            "The model in the middle of the material's thickness",
            0.5,
        ),
        ("Top", "The model's top at the material's top", 1.0),
    ];
    if open.id != "orientate_3d_model" {
        return false;
    }
    let size = |key: &str| open.params["size"][key].as_f64().unwrap_or(0.0);
    let (sx, sy, tall) = (size("x"), size("y"), size("z"));
    let area = job.area();
    let round = |v: f64| (v * 100.0).round() / 100.0;
    let mut clicked = false;
    ui.group(|ui| {
        ui.set_width(ui.available_width());
        ui.strong(tr!("align-in-material"));
        let edge = ui.spacing().interact_size.y;
        ui.horizontal_wrapped(|ui| {
            for (name, hint, ax, ay) in ACROSS {
                let path = format!("drawing/align/{name}.png");
                let button = match icon(&path) {
                    Some(image) => {
                        egui::Button::image(image.fit_to_exact_size(egui::vec2(edge, edge)))
                    }
                    None => egui::Button::new(name),
                };
                if !ui.add(button).on_hover_hint(hint).clicked() {
                    continue;
                }
                // kept as a share of the room beside it, from the material's corner, so
                // it stays aligned when the model is scaled after
                let placing = &mut open.params["placing"];
                placing["center"] = Value::Bool(false);
                if let Some(ax) = ax {
                    let x = area.min.x + (area.size().x - sx) * ax;
                    placing["position"]["x"] = Value::from(round(x));
                    placing["align"]["across"] = Value::from(ax);
                    open.shown_place[0] = Some(round(x));
                }
                if let Some(ay) = ay {
                    let y = area.min.y + (area.size().y - sy) * ay;
                    placing["position"]["y"] = Value::from(round(y));
                    placing["align"]["up"] = Value::from(ay);
                    open.shown_place[1] = Some(round(y));
                }
                clicked = true;
            }
        });
        ui.horizontal(|ui| {
            ui.label(tr!("align-height"));
            for (label, hint, up) in HEIGHT {
                if ui.button(label).on_hover_hint(hint).clicked() {
                    // Depth Below Top is the model's top above the zero plane
                    let depth = round(tall + (job.thickness - tall) * up);
                    open.params["placing"]["depth_below_top"] = Value::from(depth);
                    open.params["placing"]["align"]["height"] = Value::from(up);
                    open.shown_place[2] = Some(depth);
                    clicked = true;
                }
            }
        });
    });
    clicked
}

/// Import 3D Model's Update Undercuts: the model is judged again (it already follows every
/// change, so this is the same judging once more); true when clicked.
pub(super) fn update_undercuts(ui: &mut egui::Ui, open: &OpenForm) -> bool {
    if open.id != "orientate_3d_model" {
        return false;
    }
    let on = open.params["show_undercuts"].as_bool().unwrap_or(false);
    ui.add_enabled(on, egui::Button::new(tr!("undercuts-update")))
        .on_hover_hint(tr!("undercuts-update.tooltip"))
        .on_disabled_hover_text(tr!("undercuts-update.disabled"))
        .clicked()
}

/// The material a profile's tabs are judged against: how thick, whether a closed shape
/// is selected, and how many shapes the form's hand-placed tabs leave out.
pub(super) fn form_stock(
    doc: &cut_craft_core::document::Document,
    params: &Value,
    units: cut_craft_core::document::Units,
    bed: [f64; 2],
) -> super::tabs::Stock {
    let selected: Vec<_> = doc.selection.iter().copied().collect();
    super::tabs::Stock {
        thickness: doc.job.thickness,
        closed: doc
            .selection
            .iter()
            .any(|id| doc.vectors.get(id).is_some_and(|v| v.closed)),
        units,
        bed,
        clearing: doc.job.clearing,
        untabbed: cut_craft_core::checks::shapes_without_placed_tabs(doc, &selected, params),
    }
}

/// What Suggest found, in a sentence for the form.
pub(super) fn suggestion_note(s: &cmd_pocket_toolpath::Suggestion, finishing: &str) -> String {
    let minutes = |m: f64| format!("{:.0}", m.max(1.0).round());
    if s.clearing.is_empty() {
        return format!(
            "Suggest: {finishing} alone is quickest here, about {} min; no bigger bit of yours \
             saves time",
            minutes(s.alone)
        );
    }
    let names: Vec<&str> = s.clearing.iter().map(|t| t.name.as_str()).collect();
    format!(
        "Suggest: {} then {finishing}, about {} min against {} min with {finishing} alone",
        names.join(", "),
        minutes(s.minutes),
        minutes(s.alone)
    )
}

/// Embellish's samples and random sequences, for the pattern maker under its Sequence box.
pub(super) fn pattern_maker() -> form::PatternMaker<'static> {
    form::PatternMaker {
        sample: &embellish_sample,
        random: &cmd_embellish::random_sequence,
    }
}

pub(super) fn embellish_sample(params: &Value, entry: &str) -> form::SampleRings {
    cmd_embellish::sample(params, entry)
        .into_iter()
        .map(|ring| ring.into_iter().map(|q| [q.x, q.y]).collect())
        .collect()
}
