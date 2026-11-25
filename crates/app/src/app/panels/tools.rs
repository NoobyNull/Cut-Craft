//! The Materials Library and Tool Database windows, and the Design page's tool buttons and fly-
//! outs.

use super::{
    CutCraftApp, PIN_ICON, PINNED_ICON, Page, TOOL_BOX_MARGIN, TOOL_ICON_ROWS, egui, file_name,
    files, flyout_arrow, gui, picked_message, tool_cell, tool_cell_size, tr,
};
use crate::tool_model::Place;

impl CutCraftApp {
    /// The Materials Library window; its edits (to the library and the bits' feeds) are
    /// kept on Apply or OK.
    pub(in super::super) fn materials_library_window(&mut self, ctx: &egui::Context) {
        use cut_craft_tool_database::materials::MaterialsOutcome;
        let Some(window) = &mut self.materials_window else {
            return;
        };
        // the tool selector is open over it
        if self.tool_manager.is_some() {
            return;
        }
        let Some(outcome) = window.show(ctx) else {
            return;
        };
        match outcome {
            MaterialsOutcome::Apply | MaterialsOutcome::Ok => {
                self.materials_kept(outcome == MaterialsOutcome::Ok);
            }
            MaterialsOutcome::Cancel => self.materials_window = None,
            MaterialsOutcome::AddTool => {
                // on the database as kept: its own edits are kept as the Tool Database's
                // Select keeps them, the library's stay unkept until its OK
                let mut manager = cut_craft_tool_database::Manager::selecting(&self.tools, "");
                manager.library = self.materials.clone();
                self.open_tool_manager(manager);
                self.picking = Some(super::super::PickFor::Material);
            }
        }
    }

    /// The Materials Library's Apply or OK (`close`): its library and its own edits to the
    /// bits' feeds kept and saved.
    pub(in super::super) fn materials_kept(&mut self, close: bool) {
        let Some(window) = &mut self.materials_window else {
            return;
        };
        self.materials = window.library.clone();
        // the library's own edits onto the database as it is now, not the copy it
        // opened on: edits in the Tool Database since then stay
        self.tools = window.tools_onto(&self.tools);
        window.applied(&self.tools);
        if close {
            self.materials_window = None;
        }
        // the bits' feeds live in tools.json: saved whatever became of materials.json
        let library = self.save_materials();
        self.tools_changed("Materials Library saved");
        if let Err(e) = library {
            let tools = if self.status.message == "Materials Library saved" {
                "the bits' feeds were saved to the tool database".to_owned()
            } else {
                self.status.message.clone()
            };
            self.status.set_message(&format!("{e}. {tools}"));
        }
    }

    /// The manager's outcomes that belong to the Materials Library: a bit picked for its Add
    /// Tool goes to its window, and the manager's Materials Library button opens it. True when
    /// the outcome was handled here.
    pub(in super::super) fn library_outcome(
        &mut self,
        outcome: &cut_craft_tool_database::Outcome,
    ) -> bool {
        use cut_craft_tool_database::Outcome;
        let Some(manager) = &mut self.tool_manager else {
            return false;
        };
        match (outcome, self.picking) {
            (Outcome::Select(name), Some(super::super::PickFor::Material)) => {
                // the picker's own edits (a catalogue bit copied into My Tools, a tool
                // added or edited) kept as the Tool Database's Select keeps them
                self.tools = manager.draft.clone();
                let renamed = manager.take_renamed();
                for (from, to) in &renamed {
                    self.tools.note_renamed(from, to);
                }
                self.tool_manager = None;
                self.picking = None;
                self.tools_kept("Tool database saved", &renamed);
                self.refresh_form_tool();
                if let Some(window) = &mut self.materials_window {
                    window.rebase(&self.tools);
                    window.add_tool(name);
                }
                return true;
            }
            (Outcome::Cancel, Some(super::super::PickFor::Material)) => self.picking = None,
            (Outcome::MaterialsLibrary, _) => {
                // the edits so far kept, as Apply, then the library on them
                self.tools = manager.draft.clone();
                self.tools_changed("Tool database saved");
                self.materials_window =
                    Some(cut_craft_tool_database::materials::MaterialsWindow::new(
                        &self.materials,
                        &self.tools,
                    ));
            }
            _ => return false,
        }
        self.tool_manager = None;
        true
    }

    /// The open Tool Database learns which tools the job's toolpaths cut with, so its
    /// Delete can say when it would take one.
    pub(super) fn tool_manager_sees_the_job(&mut self) {
        if let Some(manager) = &mut self.tool_manager {
            manager.job_tools.clear();
            let names = self.doc.toolpaths.values().map(|t| t.tool.name.clone());
            manager.job_tools.extend(names);
        }
    }

    /// The Tool Database manager; its edits are kept on Apply or OK.
    pub(in super::super) fn tool_database_window(&mut self, ctx: &egui::Context) {
        use super::super::{Dialog, PickFor, filtered_dialog};
        use cut_craft_tool_database::Outcome;
        self.materials_library_window(ctx);
        self.tool_manager_sees_the_job();
        let Some(manager) = &mut self.tool_manager else {
            return;
        };
        let icons = &self.icons;
        let picture = |path: &str| icons.find(path, "").map(|p| icons.image(p));
        // a form tool takes its shape from the one open vector selected
        let profile = match self.doc.selection.iter().collect::<Vec<_>>()[..] {
            [id] => self.doc.vectors.get(id).filter(|p| !p.closed),
            _ => None,
        };
        let look = cut_craft_tool_database::Look {
            picture: &picture,
            profile,
        };
        let Some(outcome) = manager.show(ctx, &look) else {
            return;
        };
        if self.library_outcome(&outcome) {
            return;
        }
        let Some(manager) = &mut self.tool_manager else {
            return;
        };
        match outcome {
            Outcome::MaterialsLibrary => {}
            Outcome::Apply | Outcome::Ok | Outcome::Select(_) => {
                self.tools = manager.draft.clone();
                let renamed = manager.take_renamed();
                for (from, to) in &renamed {
                    self.tools.note_renamed(from, to);
                }
                let mut changer = manager.changer.clone();
                // the Tool Changer window's Select…: the tool goes in that holder
                if let (Outcome::Select(name), Some(PickFor::Holder(slot)), Some(c)) =
                    (&outcome, self.picking, &mut changer)
                {
                    c.load(name, Some(slot));
                }
                if let Some(changer) = &changer {
                    self.tools.number_by_slots(&changer.slots);
                }
                if matches!(outcome, Outcome::Ok | Outcome::Select(_)) {
                    self.tool_manager = None;
                }
                self.tools_kept("Tool database saved", &renamed);
                self.refresh_form_tool();
                if let Outcome::Select(name) = &outcome {
                    let picking = self.picking;
                    self.selected_tool(name);
                    if let Some(form) = &mut self.form {
                        form.dirty = true;
                    }
                    self.status.set_message(&picked_message(picking, name));
                }
                if let Some(changer) = changer
                    && let Some(machine) = self
                        .machines
                        .machines
                        .iter_mut()
                        .find(|m| m.name == changer.machine)
                    && machine.slots != changer.slots
                {
                    machine.slots = changer.slots;
                    // the checks made so far used the slots just changed
                    self.forget_save_checks();
                    self.machines_changed(&format!(
                        "Tool database saved; {} loaded",
                        changer.machine
                    ));
                }
            }
            Outcome::Cancel => {
                self.tool_manager = None;
                self.picking = None;
            }
            Outcome::Import => {
                if let Some(path) =
                    filtered_dialog(Dialog::OpenToolDatabase, &self.registry).pick_file()
                {
                    self.import_tools(&path);
                }
            }
            Outcome::Export(tools) => {
                if let Some(path) =
                    filtered_dialog(Dialog::SaveToolDatabase, &self.registry).save_file()
                {
                    let done = format!(
                        "Exported {} tools to {}",
                        tools.tools.len(),
                        file_name(&path)
                    );
                    let result = files::save_tools(&tools, &path);
                    self.status
                        .set_message(&result.map_or_else(|e| e, |()| done));
                }
            }
        }
    }

    /// Adds the tools of a tool database file in a group named after it: to the open Tool
    /// Database window, kept on its OK, else to the database, saved now.
    pub(in super::super) fn import_tools(&mut self, path: &std::path::Path) {
        let name = path.file_stem().map_or_else(
            || "Imported".to_owned(),
            |s| s.to_string_lossy().into_owned(),
        );
        match files::read_tool_file(path) {
            Ok(tools) => {
                let n = tools.tools.len();
                let done = format!("Imported {n} tools from {}", file_name(path));
                // a bit listed once per material becomes one bit: what was not kept is said
                let said = |report: Vec<String>| match report.first() {
                    Some(first) => format!("; {first}"),
                    None => String::new(),
                };
                if let Some(manager) = &mut self.tool_manager {
                    let report = manager.import(tools, &name);
                    self.status
                        .set_message(&format!("{done}; OK keeps them{}", said(report)));
                } else {
                    cut_craft_tool_database::tree::import(&mut self.tools, tools, &name);
                    let report = self.tools.join_material_copies();
                    self.tools_changed(&format!("{done}{}", said(report)));
                }
            }
            Err(e) => self.status.set_message(&e),
        }
    }

    /// A tool page's tools: all its groups, or only its favourites when it shows those.
    pub(in super::super) fn tools_or_favourites(
        &mut self,
        ui: &mut egui::Ui,
        groups: &[gui::ToolGroup],
        page: Page,
    ) {
        if !self.options.favourites.of(page).1 {
            self.tool_groups(ui, groups, page);
            return;
        }
        // in the order picked; a tool no longer on the page is left out
        let tools = self.tool_model().favourites(page, &self.options.favourites);
        if tools.is_empty() {
            ui.weak(tr!("panels-no-favourites-yet-right-click"));
            if ui.button(tr!("panels-show-all-tools")).clicked() {
                let action = format!("{}{}:false", gui::FAVOURITES_ONLY, gui::page_key(page));
                self.invoke(ui.ctx(), &action);
            }
            return;
        }
        self.tool_sections(ui, &[("Favourites", tools)], page, true);
    }

    /// Icon buttons for groups of command ids: implemented ones run, the rest are greyed out.
    pub(in super::super) fn tool_groups(
        &mut self,
        ui: &mut egui::Ui,
        groups: &[gui::ToolGroup],
        page: Page,
    ) {
        let sections: Vec<(&str, Vec<&'static str>)> =
            groups.iter().map(|g| (g.name, g.tools.to_vec())).collect();
        self.tool_sections(ui, &sections, page, false);
    }

    /// Headed sections of tool buttons; a right-click on one adds it to `page`'s favourites
    /// or takes it off (`among` them, it moves along them too). Tools that do one job
    /// different ways show together in a labelled box, or share a flyout button when
    /// Options ▸ General folds tool groups (see [`crate::flyout`]).
    pub(in super::super) fn tool_sections(
        &mut self,
        ui: &mut egui::Ui,
        sections: &[(&str, Vec<&'static str>)],
        page: Page,
        among: bool,
    ) {
        // a laser or rotary toolpath only on a machine that has one; a group left with
        // nothing offered is left out whole
        let sections: Vec<(&str, Vec<&'static str>)> = sections
            .iter()
            .map(|(name, tools)| {
                let tools = tools
                    .iter()
                    .copied()
                    .filter(|id| self.offered(id))
                    .collect();
                (*name, tools)
            })
            .filter(|(_, tools): &(&str, Vec<_>)| !tools.is_empty())
            .collect();
        let sections = sections.as_slice();
        let edge = TOOL_ICON_ROWS * ui.spacing().interact_size.y;
        let folded = self.options.general.fold_tool_groups;
        let mut clicked = None;
        let mut chosen = None;
        // plain sections: a heading over its tools (none for an unnamed one, which the
        // Ribbon names under its icons)
        for (name, tools) in sections {
            if !name.is_empty() {
                ui.strong(crate::tool_names::group_name(name));
            }
            if !among && !folded {
                self.spread_tools(ui, tools, edge, &mut clicked, &mut chosen);
                ui.add_space(ui.spacing().item_spacing.y);
                continue;
            }
            ui.horizontal_wrapped(|ui| {
                // favourites name each tool on its own, flyout or not
                let buttons = if among {
                    tools
                        .iter()
                        .map(|&face| crate::flyout::Button {
                            face,
                            members: vec![],
                        })
                        .collect()
                } else {
                    crate::flyout::buttons(tools, &self.options.flyout_faces)
                };
                for button in buttons {
                    let id = button.face;
                    let response = if button.members.is_empty() {
                        let response = self.tool_button(ui, id, edge);
                        if response.clicked() {
                            clicked = Some(id);
                        }
                        response
                    } else {
                        let (response, run) = self.flyout_button(ui, &button, edge);
                        if run.is_some() {
                            clicked = run;
                        }
                        response
                    };
                    if let Some(action) = self.tool_menu_on(&response, id, among, folded) {
                        chosen = Some(action);
                    }
                }
            });
            ui.add_space(ui.spacing().item_spacing.y);
        }
        if let Some(id) = clicked {
            if let Some(flyout) = crate::flyout::flyout_of(id)
                && self.options.flyout_faces.get(flyout[0]).map(String::as_str) != Some(id)
            {
                crate::flyout::remember(&mut self.options.flyout_faces, id);
                self.save_options();
            }
            self.pick(ui.ctx(), page, id);
        }
        if let Some(action) = chosen {
            self.invoke(ui.ctx(), &action);
        }
    }

    /// A tool button's right-click menu, and the action picked from it: a grid's, or the
    /// favourites' when shown `among` them.
    pub(in super::super) fn tool_menu_on(
        &self,
        response: &egui::Response,
        id: &str,
        among: bool,
        folded: bool,
    ) -> Option<String> {
        let place = if among {
            Place::Favourites
        } else {
            Place::Grid
        };
        self.popup(response, || {
            self.tool_model()
                .menu(id, place, &self.options.favourites, folded)
        })
    }

    /// A section's tools spread out: every tool its own button, those that do one job
    /// different ways in a box captioned with what they are for, so a beginner sees them
    /// all without knowing to open anything. A box goes to the next row whole when the row
    /// has no room for it.
    pub(super) fn spread_tools(
        &self,
        ui: &mut egui::Ui,
        tools: &[&'static str],
        edge: f32,
        clicked: &mut Option<&'static str>,
        chosen: &mut Option<String>,
    ) {
        let full = ui.available_width();
        let spacing = ui.spacing().item_spacing;
        let button = tool_cell_size(ui.spacing(), edge).x;
        let margin = TOOL_BOX_MARGIN;
        let caption_font = egui::TextStyle::Small.resolve(ui.style());
        let caption_color = ui.visuals().weak_text_color();
        let caption_height = ui.fonts_mut(|f| f.row_height(&caption_font));
        let clusters = crate::flyout::clusters(tools);
        let room = full - 2.0 * margin - 2.0 + spacing.x;
        let across = (room / (button + spacing.x)).floor();
        self.buttons_across.set(across);
        let captions: Vec<String> = clusters
            .iter()
            .map(|c| {
                c.name
                    .map_or_else(String::new, crate::tool_names::group_name)
            })
            .collect();
        let widths: Vec<f32> = clusters
            .iter()
            .zip(&captions)
            .map(|(cluster, caption)| {
                let caption_width = ui
                    .painter()
                    .layout_no_wrap(caption.clone(), caption_font.clone(), caption_color)
                    .size()
                    .x;
                box_width(cluster.tools.len(), caption_width, button, spacing.x).min(full)
            })
            .collect();
        for row in box_rows(&widths, full, spacing.x) {
            // loose tools line up with the tools of a box beside them
            let boxed_row = row.iter().any(|&i| clusters[i].name.is_some());
            ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP), |ui| {
                for i in row {
                    let (cluster, width) = (&clusters[i], widths[i]);
                    let frame = egui::Frame::group(ui.style()).inner_margin(margin);
                    let frame = if cluster.name.is_some() {
                        frame
                    } else {
                        frame.stroke(egui::Stroke::NONE)
                    };
                    frame.show(ui, |ui| {
                        ui.set_max_width(width - 2.0 * margin - 2.0);
                        ui.vertical(|ui| {
                            if cluster.name.is_some() {
                                let text = egui::RichText::new(&captions[i])
                                    .small()
                                    .color(caption_color);
                                ui.add(
                                    egui::Label::new(text)
                                        .selectable(false)
                                        .wrap_mode(egui::TextWrapMode::Extend),
                                );
                            } else if boxed_row {
                                ui.add_space(caption_height + spacing.y);
                            }
                            // rows cut by count, never by where the buttons end: a
                            // hovered button is a fraction wider (see `flyout::per_row`)
                            let across = crate::flyout::per_row(
                                width - 2.0 * margin - 2.0,
                                button,
                                spacing.x,
                            );
                            for tools in cluster.tools.chunks(across) {
                                ui.horizontal(|ui| {
                                    for &id in tools {
                                        let response = self.tool_button(ui, id, edge);
                                        if response.clicked() {
                                            *clicked = Some(id);
                                        }
                                        if let Some(action) =
                                            self.tool_menu_on(&response, id, false, false)
                                        {
                                            *chosen = Some(action);
                                        }
                                    }
                                });
                            }
                        });
                    });
                }
            });
        }
    }

    /// One tool's icon button: enabled when it is built, highlighted while in hand.
    pub(in super::super) fn tool_button(
        &self,
        ui: &mut egui::Ui,
        id: &str,
        edge: f32,
    ) -> egui::Response {
        let label = self.tool_label(id);
        let explicit = self.explicit_icon(id);
        let button = self
            .icon_button(explicit, id, &label, edge)
            .selected(self.status.current_tool == label);
        let available = self.available(id);
        let response = tool_cell(ui, edge, |ui| ui.add_enabled(available, button));
        crate::marks::note(ui, &format!("tool:{id}"), response.rect);
        self.with_guidance(response, id)
            .on_disabled_hover_text(self.unavailable_why(id))
    }

    /// A flyout's button: its face as [`Self::tool_button`] with an arrow in the corner. A
    /// click on the arrow, a press held for Options ▸ General ▸ Hold to open a tool group, or Down while it
    /// has the focus opens the flyout: each of its tools by icon and name. The button's
    /// response, and the tool to run: the face on a plain click, or the one picked from the
    /// flyout.
    pub(super) fn flyout_button(
        &self,
        ui: &mut egui::Ui,
        button: &crate::flyout::Button,
        edge: f32,
    ) -> (egui::Response, Option<&'static str>) {
        use crate::flyout::{on_arrow, tooltip};
        let hold_for = self.options.general.flyout_hold();
        let id = button.face;
        let label = self.tool_label(id);
        let explicit = self.explicit_icon(id);
        let in_hand = button
            .members
            .iter()
            .any(|m| self.status.current_tool == self.tool_label(m));
        let others: Vec<String> = button
            .members
            .iter()
            .filter(|m| **m != id)
            .map(|m| self.tool_label(m))
            .collect();
        let about = gui::about(id).unwrap_or_else(|| label.clone());
        let any_available = button.members.iter().any(|m| self.available(m));
        // the tip names the tools the flyout holds too, so they are found without opening it
        let tip = self.tool_tip(id, &tooltip(&about, &others));
        let face = self
            .icon_button(explicit, id, &label, edge)
            .selected(in_hand);
        let response = tool_cell(ui, edge, |ui| ui.add_enabled(any_available, face));
        super::super::guide::hover(&response, &tip);
        for m in &button.members {
            crate::marks::note(ui, &format!("tool:{m}"), response.rect);
        }
        let response = response
            .on_hover_text(super::super::guide::tooltip(
                &tip.about,
                tip.next.as_deref(),
            ))
            .on_disabled_hover_text(self.unavailable_why(id));
        let r = response.rect;
        flyout_arrow(ui, &response);
        let popup_id = response.id.with("flyout");
        let held_id = response.id.with("flyout_held");
        let ctx = ui.ctx().clone();
        // a long press opens it; the release that follows is no click
        let mut open = false;
        if response.is_pointer_button_down_on() {
            let held = ctx.input(|i| i.pointer.press_start_time().map(|t| i.time - t));
            match held {
                Some(t) if t >= hold_for => {
                    if !ctx.data(|d| d.get_temp::<bool>(held_id).unwrap_or(false)) {
                        ctx.data_mut(|d| d.insert_temp(held_id, true));
                        open = true;
                    }
                }
                Some(t) => {
                    ctx.request_repaint_after(std::time::Duration::from_secs_f64(hold_for - t));
                }
                None => {}
            }
        }
        let mut run = None;
        if response.clicked() {
            let was_held = ctx.data_mut(|d| d.remove_temp::<bool>(held_id).unwrap_or(false));
            let at_arrow = response
                .interact_pointer_pos()
                .is_some_and(|at| on_arrow(r, at));
            // the arrow, or a face not built yet, opens the flyout; a held press already did
            if at_arrow || !self.available(id) {
                open |= !was_held;
            } else if !was_held {
                run = Some(id);
            }
        } else if !response.is_pointer_button_down_on() {
            ctx.data_mut(|d| d.remove_temp::<bool>(held_id));
        }
        if response.has_focus() && ctx.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
            open = true;
        }
        let picked = egui::Popup::from_response(&response)
            .id(popup_id)
            .kind(egui::PopupKind::Menu)
            .align(egui::RectAlign::RIGHT_START)
            .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
            .open_memory(open.then_some(egui::SetOpenCommand::Bool(true)))
            .show(|ui| self.flyout_list(ui, button, edge, open))
            .and_then(|shown| shown.inner);
        if picked.is_some() {
            egui::Popup::close_id(&ctx, popup_id);
        }
        (response, picked.or(run))
    }

    /// An open flyout: each of its tools by icon and name, the face first in focus when it
    /// has just opened; the tool clicked.
    pub(super) fn flyout_list(
        &self,
        ui: &mut egui::Ui,
        button: &crate::flyout::Button,
        edge: f32,
        opened: bool,
    ) -> Option<&'static str> {
        cut_craft_bug_report::mark(ui, "Flyout");
        let mut picked = None;
        for &member in &button.members {
            let name = self.tool_label(member);
            let explicit = self.explicit_icon(member);
            let row = match self.icons.find(explicit, member) {
                Some(path) => egui::Button::image_and_text(
                    self.icons
                        .image(path)
                        .fit_to_exact_size(egui::vec2(edge * 0.6, edge * 0.6)),
                    &name,
                ),
                None => egui::Button::new(&name),
            }
            .selected(member == button.face);
            let row = ui.add_enabled(self.available(member), row);
            let row = self
                .with_guidance(row, member)
                .on_disabled_hover_text(self.unavailable_why(member));
            if opened && member == button.face {
                row.request_focus();
            }
            if row.clicked() {
                picked = Some(member);
            }
        }
        picked
    }

    /// An icon button, or a text one when the icon is missing.
    pub(in super::super) fn icon_button(
        &self,
        icon: &str,
        id: &str,
        label: &str,
        edge: f32,
    ) -> egui::Button<'static> {
        match self.icons.find(icon, id) {
            Some(path) => egui::Button::image(
                self.icons
                    .image(path)
                    .fit_to_exact_size(egui::vec2(edge, edge)),
            ),
            None => egui::Button::new(label.to_owned()),
        }
    }

    /// The push-pin toggle: leaning while loose, upright and pushed in once pinned. Drawn
    /// rather than the 📌 emoji, which many system fonts lack and egui drew as an empty box.
    pub(in super::super) fn pin_button(&self, pinned: bool, edge: f32) -> egui::Button<'static> {
        let (icon, label) = if pinned {
            (PINNED_ICON, "Pinned")
        } else {
            (PIN_ICON, "Pin")
        };
        self.icon_button(icon, "", label, edge).selected(pinned)
    }
}

/// How wide a box of `n` tool buttons is laid out on one row, with its `caption`'s width,
/// its margin and border: as wide as the icons or the caption, whichever is wider.
fn box_width(n: usize, caption: f32, button: f32, gap: f32) -> f32 {
    let n = f32::from(u16::try_from(n).unwrap_or(u16::MAX));
    let icons = n * button + (n - 1.0).max(0.0) * gap;
    icons.max(caption) + 2.0 * TOOL_BOX_MARGIN + 2.0
}

/// The rows boxes `widths` wide go in across `full`, `gap` apart: a box going to the next
/// row whole when this one has no room for it.
fn box_rows(widths: &[f32], full: f32, gap: f32) -> Vec<Vec<usize>> {
    let mut rows: Vec<Vec<usize>> = Vec::new();
    let mut left = 0.0;
    for (i, &width) in widths.iter().enumerate() {
        if rows.is_empty() || left < width {
            rows.push(Vec::new());
            left = full;
        }
        left -= width + gap;
        rows.last_mut().expect("a row").push(i);
    }
    rows
}
