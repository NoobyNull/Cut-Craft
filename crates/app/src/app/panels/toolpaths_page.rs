//! The Toolpaths page: its list of toolpaths, the arrange buttons, the checks, Preview and Save,
//! and the status bar.

use super::{
    Align, CutCraftApp, EntityId, HoverHint, LIST_SHARE, Layout, MORE_LIST_ACTIONS, MachinedArea,
    Page, Places, SAVE_FOR_MACHINE, SAVE_NEEDS, SELECTION_COUNT_HINT, Screen, Units, VIEW_GCODE,
    VIEW_GCODE_ABOUT, egui, gui, step_heading, toolpath_row, tr,
};

impl CutCraftApp {
    /// The Toolpaths page: the job and its tools and the sub-tabs Make, Preview and Save
    /// held at the top, the open sub-tab scrolling under them, and the toolpath list always
    /// in its own bottom part, a strip over it dragging it taller or shorter; or any form
    /// opened from them.
    pub(in super::super) fn toolpaths_page(&mut self, ui: &mut egui::Ui) {
        cut_craft_bug_report::mark(ui, "Toolpaths page");
        self.page_header(ui, Page::Toolpaths, "Toolpaths");
        if self
            .form
            .as_ref()
            .is_some_and(|f| f.page == Page::Toolpaths)
        {
            self.form_panel(ui);
            return;
        }
        // a job's material is set up before its first toolpath: the first press anywhere on
        // the page opens Material Setup, once per job, with what Job Setup chose filled in
        let page = ui.max_rect();
        let pressed = ui.input(|i| {
            i.pointer.primary_pressed()
                && i.pointer.interact_pos().is_some_and(|p| page.contains(p))
        });
        if pressed && self.ask_material && !self.doc.material_checked {
            self.doc.material_checked = true;
            self.invoke(ui.ctx(), "material_setup");
            return;
        }
        let row = ui.spacing().interact_size.y;
        let room = ui.available_height();
        let strip = ui.spacing().item_spacing.y * 2.0;
        let list = self
            .list_height
            .unwrap_or(room * LIST_SHARE)
            .clamp(row * 4.0, (room - row * 8.0).max(row * 4.0));
        let width = ui.available_width();
        ui.allocate_ui(egui::vec2(width, room - list - strip), |ui| {
            ui.set_min_height(room - list - strip);
            cut_craft_bug_report::mark(ui, "Toolpaths sub-tabs");
            let open = self.subtab_head(ui, Page::Toolpaths);
            egui::ScrollArea::vertical()
                .id_salt("toolpaths page")
                .auto_shrink(false)
                .show(ui, |ui| self.subtab_body(ui, Page::Toolpaths, open));
        });
        let (rect, divider) = ui.allocate_exact_size(egui::vec2(width, strip), egui::Sense::drag());
        if divider.dragged() {
            self.list_height = Some(list - divider.drag_delta().y);
        }
        if divider.hovered() || divider.dragged() {
            ui.set_cursor_icon(egui::CursorIcon::ResizeVertical);
        }
        let line = if divider.hovered() || divider.dragged() {
            ui.visuals().widgets.active.bg_stroke
        } else {
            ui.visuals().widgets.noninteractive.bg_stroke
        };
        ui.painter().hline(rect.x_range(), rect.center().y, line);
        self.toolpath_list_body(ui, false);
    }

    /// The Analyse sub-tab: the checks Save Toolpaths makes before cutting, run on the shown
    /// toolpaths in the order they cut, each with how to put it right; then any toolpath
    /// out of date, with the button that brings them all up to date.
    pub(in super::super) fn analyse(&mut self, ui: &mut egui::Ui) {
        step_heading(ui, 3, "Analyse: check before cutting");
        let shown: Vec<EntityId> = self
            .doc
            .toolpaths
            .iter()
            .filter(|(_, t)| t.visible)
            .map(|(&id, _)| id)
            .collect();
        if shown.is_empty() {
            ui.weak(tr!("panels-make-a-toolpath-or-tick"));
            return;
        }
        let plan = [shown];
        let changes = crate::files::ToolChanges::SeparateFiles;
        let found = self.save_checks(&plan, changes).map(<[_]>::to_vec);
        let skipped = self.not_checked(&plan, changes);
        if self.check_list(ui, found.as_deref(), &skipped) {
            self.check_again();
        }
        let stale = self.doc.toolpaths.values().filter(|t| t.stale).count();
        ui.add_space(ui.spacing().item_spacing.y);
        ui.strong(tr!("panels-out-of-date"));
        if stale == 0 {
            ui.label(tr!("panels-every-toolpath-matches-what-it"));
        } else {
            ui.colored_label(
                ui.visuals().warn_fg_color,
                format!(
                    "⚠ {stale} toolpath{} changed since {} made: recalculate before cutting",
                    if stale == 1 { "" } else { "s" },
                    if stale == 1 { "it was" } else { "they were" },
                ),
            );
            let id = "recalculate_all_toolpaths";
            if ui
                .add_enabled(self.available(id), egui::Button::new(self.tool_label(id)))
                .clicked()
            {
                self.invoke(ui.ctx(), id);
            }
        }
    }

    /// Where the checks before cutting are not ready: that they are running, or that they
    /// were cancelled, with Check Again; whether it was pressed.
    pub(in super::super) fn checks_waiting(&self, ui: &mut egui::Ui) -> bool {
        if self.checking() {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(tr!("panels-checking-the-toolpaths-a-big"));
            });
            return false;
        }
        // other work running: the checks start once it is done, nothing was cancelled
        if self.busy.is_some() {
            ui.label(tr!("panels-the-checks-wait-for-the-work-running"));
            return false;
        }
        ui.label(tr!("panels-the-checks-were-cancelled-so"));
        ui.button(tr!("panels-check-again"))
            .on_hover_hint(tr!("panels-run-the-checks-before-cutting"))
            .clicked()
    }

    /// What Toolpaths sub-tab `i` shows: Make's strategies (or the favourites, when the page
    /// shows only those) with the hovered one's tip, the previews, or the Save button and
    /// the paperwork.
    pub(in super::super) fn toolpath_subtab(&mut self, ui: &mut egui::Ui, i: usize) {
        let Some(tab) = gui::TOOLPATH_SUBTABS.get(i) else {
            return;
        };
        let groups = self.toolbar.toolpath_subtab_groups(i);
        let (save, rest): (Vec<_>, Vec<_>) = groups
            .into_iter()
            .partition(|g| g.tools.contains(&"save_toolpaths"));
        match tab.name {
            "Create" => {
                step_heading(ui, 1, "Create a toolpath: pick how to cut");
                // favourites stand in for the strategies only: the list, previews and
                // saving stay where they are
                let all = self.toolbar.toolpath_groups.clone();
                if self.options.favourites.of(Page::Toolpaths).1 {
                    self.tools_or_favourites(ui, &all, Page::Toolpaths);
                } else {
                    self.tool_groups(ui, &rest, Page::Toolpaths);
                }
                // the hovered tool's guidance under the icons, scrolling with them
                self.tip_contents(ui);
            }
            "Preview" => {
                self.preview_buttons(ui, true, true);
                ui.add_space(ui.spacing().item_spacing.y);
                self.tool_groups(ui, &rest, Page::Toolpaths);
            }
            "Analyse" => {
                self.analyse(ui);
                ui.add_space(ui.spacing().item_spacing.y);
                self.tool_groups(ui, &rest, Page::Toolpaths);
            }
            "Output" => {
                if !save.is_empty() {
                    self.save_button(ui, true);
                }
                let (mut files, mut view) = (false, false);
                ui.horizontal_wrapped(|ui| {
                    files = ui
                        .add_enabled(
                            self.available("machine_files"),
                            egui::Button::new(self.tool_label("machine_files")),
                        )
                        .on_hover_hint(gui::about("machine_files").unwrap_or_default())
                        .on_disabled_hover_text(tr!("panels-give-the-machine-a-folder"))
                        .clicked();
                    // beside Machine Files, so nothing under it moves
                    view = ui
                        .add_enabled(
                            self.available("save_toolpaths"),
                            egui::Button::new(VIEW_GCODE),
                        )
                        .on_hover_hint(VIEW_GCODE_ABOUT)
                        .on_disabled_hover_text(SAVE_NEEDS)
                        .clicked();
                });
                if files {
                    self.invoke(ui.ctx(), "machine_files");
                }
                if view {
                    self.view_gcode(self.default_save_form());
                }
                ui.add_space(ui.spacing().item_spacing.y);
                self.tool_groups(ui, &rest, Page::Toolpaths);
            }
            _ => self.tool_groups(ui, &rest, Page::Toolpaths),
        }
    }

    /// The buttons over the toolpath list's rows that arrange it: the picked toolpath or
    /// group up or down the cut order (a group with everything inside it), delete, put the
    /// picked toolpaths (or the picked group) in a new group, inside the group they are in,
    /// take them (or the picked group) out one level, or make an empty group for the next
    /// toolpaths made, inside the picked group if there is one.
    pub(super) fn toolpath_arrange_buttons(&mut self, ui: &mut egui::Ui) {
        use crate::toolpath_order::{Arranged, Item, Listing};
        let listing = Listing::of(&self.doc);
        let picked = self.picked_toolpaths();
        let group = self.selected_group.clone();
        let moved = |up: bool| {
            let item = match (&group, picked.as_slice()) {
                (Some(name), _) => Item::Group(name.clone()),
                (None, [id]) => Item::Toolpath(*id),
                _ => return None,
            };
            listing.moved(&item, up)
        };
        let (up, down) = (moved(true), moved(false));
        let in_group = picked.iter().any(|id| {
            self.doc
                .toolpaths
                .get(id)
                .is_some_and(|t| t.group.is_some())
        });
        let mut clicked = None;
        ui.horizontal_wrapped(|ui| {
            // each button is known by its id, the same in any language; its words' key
            // is the id with hyphens
            let mut button = |ui: &mut egui::Ui, can: bool, id: &'static str| {
                let key = id.replace('_', "-");
                let response = ui
                    .add_enabled(can, egui::Button::new(tr!(&key)))
                    .on_hover_hint(tr!(&format!("{key}.tooltip")));
                crate::marks::note(ui, &format!("button:{id}"), response.rect);
                if response.clicked() {
                    clicked = Some(id);
                }
            };
            button(ui, up.is_some(), "toolpaths_move_up");
            button(ui, down.is_some(), "toolpaths_move_down");
            button(
                ui,
                picked.len() == 1 && self.available("delete_toolpath"),
                "toolpaths_delete",
            );
        });
        ui.horizontal_wrapped(|ui| {
            // each button is known by its id, the same in any language; its words' key
            // is the id with hyphens
            let mut button = |ui: &mut egui::Ui, can: bool, id: &'static str| {
                let key = id.replace('_', "-");
                let response = ui
                    .add_enabled(can, egui::Button::new(tr!(&key)))
                    .on_hover_hint(tr!(&format!("{key}.tooltip")));
                crate::marks::note(ui, &format!("button:{id}"), response.rect);
                if response.clicked() {
                    clicked = Some(id);
                }
            };
            button(ui, !picked.is_empty() || group.is_some(), "toolpaths_group");
            button(ui, in_group || group.is_some(), "toolpaths_ungroup");
            button(ui, true, "toolpaths_new_group");
        });
        let order_only = |order: Option<Vec<EntityId>>| {
            order.map(|order| Arranged {
                order,
                ..Arranged::default()
            })
        };
        match clicked {
            Some("toolpaths_move_up") => self.arrange(order_only(up)),
            Some("toolpaths_move_down") => self.arrange(order_only(down)),
            Some("toolpaths_delete") => self.invoke(ui.ctx(), "delete_toolpath"),
            Some("toolpaths_group") => {
                let name = self.doc.next_toolpath_group_name();
                let arranged = match &group {
                    Some(group) => listing.group_wrapped(group, &name),
                    None => listing.grouped(&picked, &name),
                };
                self.arrange(arranged);
                self.selected_group = None;
            }
            Some("toolpaths_ungroup") => {
                if let Some(name) = group {
                    let params = serde_json::json!({ "group": name, "delete_toolpaths": false });
                    if let Err(e) = self.apply("ungroup_toolpaths", &params) {
                        self.status.set_message(&e);
                    }
                    self.selected_group = None;
                } else {
                    self.arrange(Some(listing.ungrouped(&picked)));
                }
            }
            Some("toolpaths_new_group") => {
                let params = serde_json::json!({
                    "toolpaths": [],
                    "group": self.doc.next_toolpath_group_name(),
                    "inside": group.unwrap_or_default(),
                });
                if let Err(e) = self.apply("group_toolpaths", &params) {
                    self.status.set_message(&e);
                }
            }
            _ => {}
        }
    }

    /// The toolpaths picked in the list, in cut order: the selected one and any Ctrl-clicked
    /// with it.
    pub(in super::super) fn picked_toolpaths(&self) -> Vec<EntityId> {
        self.doc
            .toolpaths
            .keys()
            .copied()
            .filter(|id| self.selected_toolpath == Some(*id) || self.more_toolpaths.contains(id))
            .collect()
    }

    /// Puts the toolpaths as `arranged` says (none: nothing to do): their cut order, the
    /// group each named toolpath goes in and the group each named group goes inside, as
    /// one step to undo; the picked toolpaths stay picked in their new places.
    pub(super) fn arrange(&mut self, arranged: Option<crate::toolpath_order::Arranged>) {
        let Some(arranged) = arranged else { return };
        let to = &arranged.order;
        let slots: Vec<EntityId> = self.doc.toolpaths.keys().copied().collect();
        let place = |id: EntityId| to.iter().position(|&t| t == id).map(|i| slots[i]);
        let params = serde_json::json!({
            "order": to,
            "groups": arranged
                .groups
                .iter()
                .map(|(id, g)| (id.0.to_string(), serde_json::json!(g)))
                .collect::<serde_json::Map<_, _>>(),
            "parents": arranged
                .parents
                .iter()
                .map(|(g, at)| (g.clone(), serde_json::json!(at)))
                .collect::<serde_json::Map<_, _>>(),
        });
        let selected = self.selected_toolpath.and_then(place);
        let more = self
            .more_toolpaths
            .iter()
            .filter_map(|&id| place(id))
            .collect();
        match self.apply("arrange_toolpaths", &params) {
            Ok(()) => {
                self.selected_toolpath = selected;
                self.more_toolpaths = more;
            }
            Err(e) => self.status.set_message(&e),
        }
    }

    /// The toolpath list's tick showing or hiding a toolpath, or its swatch colouring its
    /// preview, as a Toolpath Properties edit.
    pub(super) fn list_properties(
        &mut self,
        shown: Option<(EntityId, bool)>,
        coloured: Option<(EntityId, [u8; 3])>,
    ) {
        // a tick shows or hides a toolpath, a swatch colours its preview
        let properties = shown
            .and_then(|(id, visible)| Some((id, self.doc.toolpaths.get(&id)?, visible, None)))
            .or_else(|| {
                let (id, rgb) = coloured?;
                let t = self.doc.toolpaths.get(&id)?;
                Some((id, t, t.visible, Some(serde_json::json!({ "colour": rgb }))))
            })
            .map(|(id, t, visible, preview)| {
                let mut params =
                    serde_json::json!({ "toolpath": id, "name": t.name, "visible": visible });
                if let Some(preview) = preview {
                    params["preview"] = preview;
                }
                params
            });
        if let Some(params) = properties
            && let Err(e) = self.apply("toolpath_properties", &params)
        {
            self.status.set_message(&e);
        }
    }

    /// A group's tick, name and (closed) count in the toolpath list: the tick shows or
    /// hides everything inside it, a click on the name picks it (again: unpicks), a
    /// right-click picks it and opens its menu. The new tick if changed, whether it was
    /// picked, the menu action chosen, and the name's response.
    pub(super) fn group_row(
        &self,
        ui: &mut egui::Ui,
        group: &str,
        closed_count: Option<usize>,
    ) -> (Option<bool>, Option<bool>, Option<String>, egui::Response) {
        let under = self.doc.toolpaths_under_group(group);
        let shown = under
            .iter()
            .filter(|id| self.doc.toolpaths.get(id).is_some_and(|t| t.visible))
            .count();
        let mut all = !under.is_empty() && shown == under.len();
        let tick = ui
            .add_enabled(
                !under.is_empty(),
                egui::Checkbox::without_text(&mut all)
                    .indeterminate(shown > 0 && shown < under.len()),
            )
            .on_hover_hint(tr!("panels-show-or-hide-this-group"));
        crate::marks::note(ui, &format!("toolpath_group_tick:{group}"), tick.rect);
        let ticked = tick.changed().then_some(all);
        let picked = self.selected_group.as_deref() == Some(group);
        let name = egui::RichText::new(group).strong();
        let label = ui
            .selectable_label(picked, name)
            .on_hover_hint(tr!("panels-a-toolpath-group-its-toolpaths"));
        crate::marks::note(ui, &format!("toolpath_group:{group}"), label.rect);
        if let Some(count) = closed_count {
            ui.weak(tr!("panels-group-count", count = count));
        }
        let pick = if label.clicked() {
            Some(!picked)
        } else if label.secondary_clicked() {
            Some(true)
        } else {
            None
        };
        let action = self.popup(&label, || gui::toolpath_group_menu(group));
        (ticked, pick, action, label)
    }

    /// The toolpath list, with what comes after making toolpaths pinned under it, so it never
    /// scrolls away: preview them, then save them for the machine. `steps` numbers those as
    /// the Toolpaths page numbers making one.
    pub(in super::super) fn toolpath_list(&mut self, ui: &mut egui::Ui, steps: bool) {
        egui::Panel::bottom("toolpath_next_steps").show(ui, |ui| {
            self.preview_and_save(ui, steps);
        });
        self.toolpath_list_body(ui, true);
    }

    /// The toolpath list itself (see [`Self::toolpath_list_rows`]). What it changes while
    /// a form is open beside it is not the form's to take back: the form notes it, and its
    /// Cancel makes it again after undoing everything since it opened.
    pub(in super::super) fn toolpath_list_body(&mut self, ui: &mut egui::Ui, templates: bool) {
        if let Some(form) = self.form.as_mut() {
            form.in_list = true;
        }
        self.toolpath_list_rows(ui, templates);
        if let Some(form) = self.form.as_mut() {
            form.in_list = false;
        }
        if let Some(form) = self.form_behind.as_mut() {
            form.in_list = false;
        }
    }

    /// The toolpath list's heading, buttons (the template ones too with `templates`) and
    /// the buttons arranging it, then the rows in the order they cut as a tree, each group's
    /// heading over what it holds, with guide lines. A click picks a toolpath, Ctrl-click
    /// picks several; a row dragged by its grip (≡) or name onto a toolpath goes just before
    /// it, in its group; onto a group's name goes in at its end (a group goes in as a
    /// sub-group), or onto the top edge of a group's row just before that group.
    #[expect(
        clippy::too_many_lines,
        reason = "the list's rows, picking, dragging and menus"
    )]
    pub(super) fn toolpath_list_rows(&mut self, ui: &mut egui::Ui, templates: bool) {
        use super::toolpath_tree::{LEVEL, drop_mark, grip, guide_lines, tree_room};
        use crate::toolpath_order::{Item, Listing, Moving, Target};
        cut_craft_bug_report::mark(ui, "Toolpath list");
        ui.strong(tr!("panels-toolpath-list"));
        self.toolpath_list_buttons(ui, templates);
        self.toolpath_arrange_buttons(ui);
        if self.doc.toolpaths.is_empty() {
            ui.weak(tr!("panels-no-toolpaths-yet-each-one"));
        }
        let mut chosen = None;
        let mut shown = None;
        let mut group_shown = None;
        let mut edit = None;
        let mut coloured = None;
        let mut dropped = None;
        let mut opened = None;
        let listing = Listing::of(&self.doc);
        let closed = self.doc.toolpath_groups_closed.clone();
        let rows = listing.rows(&closed);
        let places: Places = self
            .doc
            .toolpaths
            .keys()
            .enumerate()
            .map(|(n, &id)| (id, n))
            .collect();
        let mut group_picked = None;
        let mut clicked = None;
        let ctrl = ui.input(|i| i.modifiers.command || i.modifiers.shift);
        // any press elsewhere hands Del back to the drawing; a row's click takes it below
        if ui.input(|i| i.pointer.any_pressed()) {
            self.toolpath_list_clicked = false;
        }
        let dragging = egui::DragAndDrop::has_payload_of_type::<Moving>(ui.ctx());
        // each toolpath's colour in the preview, when the machined areas take them
        let swatches = self.doc.job.machined_area == MachinedArea::ToolpathColor;
        let picked = self.picked_toolpaths();
        egui::ScrollArea::vertical()
            .id_salt("toolpath rows")
            .auto_shrink([false, true])
            .show(ui, |ui| {
                for row in &rows {
                    let open = matches!(&row.item, Item::Group(g) if !closed.contains(g));
                    let line = ui.horizontal(|ui| {
                        let held = grip(ui);
                        let (room, arrow) = tree_room(ui, row, open);
                        let name = match &row.item {
                            Item::Group(g) => g.clone(),
                            Item::Toolpath(id) => self.doc.toolpaths[id].name.clone(),
                        };
                        crate::marks::note(ui, &format!("toolpath_grip:{name}"), held.rect);
                        if let Some(arrow) = &arrow {
                            crate::marks::note(ui, &format!("toolpath_arrow:{name}"), arrow.rect);
                        }
                        if arrow.is_some_and(|a| a.clicked())
                            && let Item::Group(g) = &row.item
                        {
                            opened = Some((g.clone(), !open));
                        }
                        let (label, moving) = match &row.item {
                            Item::Group(group) => {
                                let count = (!open).then_some(row.count);
                                let (ticked, pick, action, label) =
                                    self.group_row(ui, group, count);
                                if let Some(on) = ticked {
                                    group_shown = Some((group.clone(), on));
                                }
                                if let Some(on) = pick {
                                    group_picked = Some((group.clone(), on));
                                }
                                if action.is_some() {
                                    chosen = action;
                                }
                                (label, Moving::Group(group.clone()))
                            }
                            Item::Toolpath(id) => {
                                let id = *id;
                                let toolpath = &self.doc.toolpaths[&id];
                                let selected = picked.contains(&id);
                                let swatch = swatches.then_some((places[&id], self.doc.job.colour));
                                let collided = self
                                    .collisions
                                    .get(&id)
                                    .filter(|c| {
                                        c.moves == super::super::background::fingerprint(toolpath)
                                    })
                                    .map(|c| c.sentence.as_str());
                                let (label, ticked, rgb) =
                                    toolpath_row(ui, toolpath, (selected, collided), swatch);
                                crate::marks::note(
                                    ui,
                                    &format!("toolpath:{}", toolpath.name),
                                    label.rect,
                                );
                                if let Some(visible) = ticked {
                                    shown = Some((id, visible));
                                }
                                if let Some(rgb) = rgb {
                                    coloured = Some((id, rgb));
                                }
                                if label.clicked() {
                                    clicked = Some((id, selected));
                                }
                                // a double-click opens the toolpath's form
                                if label.double_clicked() {
                                    edit = Some(id);
                                }
                                // The toolpath menu acts on the row right-clicked, so it
                                // selects it
                                if label.secondary_clicked() {
                                    self.toolpath_list_clicked = true;
                                    self.selected_toolpath = Some(id);
                                    self.more_toolpaths.clear();
                                    self.selected_group = None;
                                }
                                if let Some(action) = self
                                    .popup(&label, || gui::toolpath_menu(&self.doc, id, toolpath))
                                {
                                    chosen = Some(action);
                                }
                                // the picked ones go together when this is one of them
                                let moving = if picked.contains(&id) {
                                    picked.clone()
                                } else {
                                    vec![id]
                                };
                                (label, Moving::Toolpaths(moving))
                            }
                        };
                        // dragged by its grip or its name
                        let label = label.interact(egui::Sense::drag());
                        if held.drag_started() || label.drag_started() {
                            egui::DragAndDrop::set_payload(ui.ctx(), moving);
                        }
                        (room, label.rect)
                    });
                    let (room, name) = line.inner;
                    let rect = line.response.rect;
                    guide_lines(ui, row, room, rect);
                    // where a drop here goes: a group's name takes it in, the top edge of a
                    // group's row (or any toolpath's row) puts it just before
                    let pointer = ui.ctx().pointer_hover_pos();
                    let target = match &row.item {
                        Item::Group(g)
                            if pointer.is_none_or(|p| p.y > rect.top() + rect.height() / 4.0) =>
                        {
                            Target::Into(g.clone())
                        }
                        item => Target::Before(item.clone()),
                    };
                    let hover = line.response.contains_pointer();
                    if dragging && hover {
                        #[expect(clippy::cast_precision_loss, reason = "a handful of levels")]
                        let from = room.left() + LEVEL * row.depth as f32;
                        let into = matches!(target, Target::Into(_)).then_some(name);
                        drop_mark(ui, rect, from, into);
                    }
                    if hover && let Some(moving) = line.response.dnd_release_payload::<Moving>() {
                        dropped = Some(((*moving).clone(), target));
                    }
                }
                if dragging {
                    // room under the rows: dropped there, it goes last, in no group
                    let end = ui.add(
                        egui::Label::new(
                            egui::RichText::new(tr!("panels-drop-here-to-cut-it")).weak(),
                        )
                        .selectable(false),
                    );
                    if let Some(moving) = end.dnd_release_payload::<Moving>() {
                        dropped = Some(((*moving).clone(), Target::End));
                    }
                }
            });
        if let Some((id, selected)) = clicked {
            self.toolpath_list_clicked = true;
            if ctrl {
                // Ctrl-click: one more picked, or one less
                if self.selected_toolpath.is_none() {
                    self.selected_toolpath = Some(id);
                } else if self.selected_toolpath == Some(id) {
                    self.selected_toolpath = self.more_toolpaths.pop_first();
                } else if !self.more_toolpaths.remove(&id) {
                    self.more_toolpaths.insert(id);
                }
            } else {
                self.selected_toolpath = if selected && picked.len() == 1 {
                    None
                } else {
                    Some(id)
                };
                self.more_toolpaths.clear();
            }
            self.selected_group = None;
        }
        if let Some((group, on)) = group_picked {
            self.selected_group = on.then_some(group);
            self.selected_toolpath = None;
            self.more_toolpaths.clear();
        }
        // opening or closing a group is how it is looked at, not a change to undo
        if let Some((group, open)) = opened {
            if open {
                self.doc.toolpath_groups_closed.remove(&group);
            } else {
                self.doc.toolpath_groups_closed.insert(group);
            }
        }
        if let Some((moving, target)) = dropped {
            self.arrange(listing.dropped(&moving, &target));
        }
        if let Some((group, visible)) = group_shown {
            let params = serde_json::json!({ "group": group, "visible": visible });
            if let Err(e) = self.apply("show_toolpath_group", &params) {
                self.status.set_message(&e);
            }
        }
        // one change, so one Undo takes a tick back
        self.list_properties(shown, coloured);
        if let Some(id) = edit {
            self.selected_toolpath = Some(id);
            self.more_toolpaths.clear();
            self.invoke(ui.ctx(), "edit_toolpath");
        }
        if let Some(action) = chosen {
            self.invoke(ui.ctx(), &action);
        }
    }

    /// Under the toolpath list: Preview All and Reset Preview, the other previews and the
    /// recalculating in More…, then one wide Save Toolpaths button, the step that ends every
    /// job. `steps` numbers them 2 and 3.
    pub(super) fn preview_and_save(&mut self, ui: &mut egui::Ui, steps: bool) {
        ui.add_space(ui.spacing().item_spacing.y);
        self.preview_buttons(ui, steps, false);
        ui.add_space(ui.spacing().item_spacing.y);
        self.save_button(ui, steps);
        ui.add_space(ui.spacing().item_spacing.y);
    }

    /// Preview All and Reset Preview (Recalculate All first while any is out of date), then
    /// the other previews and recalculating: in a More… menu, or with `spread` as buttons of
    /// their own under a heading, so nothing hides on the Preview sub-tab. `steps` numbers
    /// the heading 2.
    pub(super) fn preview_buttons(&mut self, ui: &mut egui::Ui, steps: bool, spread: bool) {
        fn button(app: &CutCraftApp, ui: &mut egui::Ui, id: &'static str) -> Option<&'static str> {
            ui.add_enabled(app.available(id), egui::Button::new(app.tool_label(id)))
                .on_hover_hint(gui::about(id).unwrap_or_default())
                .clicked()
                .then_some(id)
        }
        let mut chosen = None;
        if steps {
            step_heading(ui, 2, &tr!("panels-preview-watch-it-cut"));
        } else {
            ui.strong(tr!("panels-preview-watch-it-cut"));
        }
        let stale = self.doc.toolpaths.values().any(|t| t.stale);
        // out of date ones are called out in the list; their fix is at hand
        let more = MORE_LIST_ACTIONS
            .into_iter()
            .filter(|&id| !stale || id != "recalculate_all_toolpaths");
        ui.horizontal_wrapped(|ui| {
            let mut shown = vec!["preview_all_toolpaths", "reset_preview"];
            if stale {
                shown.insert(0, "recalculate_all_toolpaths");
            }
            for id in shown {
                chosen = button(self, ui, id).or(chosen);
            }
            if !spread {
                ui.menu_button(tr!("panels-more"), |ui| {
                    for id in more.clone() {
                        if let Some(id) = button(self, ui, id) {
                            chosen = Some(id);
                            ui.close();
                        }
                    }
                });
            }
        });
        // always to hand: how finely the preview is worked out now
        self.detail_slider(ui);
        if spread {
            ui.add_space(ui.spacing().item_spacing.y);
            ui.strong(tr!("panels-more-previews"));
            ui.horizontal_wrapped(|ui| {
                // deleting is the list's job, beside its other row buttons
                for id in more.filter(|&id| id != "delete_toolpath") {
                    chosen = button(self, ui, id).or(chosen);
                }
            });
        }
        if let Some(id) = chosen {
            self.invoke(ui.ctx(), id);
        }
    }

    /// One wide Save Toolpaths button, the step that ends every job, with what it waits for
    /// when there is nothing to save. `steps` numbers its heading 3.
    pub(super) fn save_button(&mut self, ui: &mut egui::Ui, steps: bool) {
        if steps {
            step_heading(ui, 4, "Output: the file your machine runs");
        } else {
            ui.strong(tr!("panels-save-the-file-your-machine"));
        }
        let ready = self.available("save_toolpaths");
        let row = ui.spacing().interact_size.y;
        let label = egui::RichText::new(SAVE_FOR_MACHINE).strong();
        let save = ui
            .add_enabled(
                ready,
                egui::Button::new(label)
                    .fill(ui.visuals().selection.bg_fill)
                    .min_size(egui::vec2(ui.available_width(), row * 1.5)),
            )
            .on_hover_hint(tr!("panels-check-the-toolpaths-and-write"));
        crate::marks::note(ui, "tool:save_toolpaths", save.rect);
        if !ready {
            ui.weak(SAVE_NEEDS);
        }
        if save.clicked() {
            self.invoke(ui.ctx(), "save_toolpaths");
        }
        ui.add_space(ui.spacing().item_spacing.y);
    }

    /// One fixed-height row: the hint (or last message) on the left, cut short with … when
    /// it does not fit, its whole text on hover; the tool, units and version on the right.
    pub(in super::super) fn status_bar(&mut self, ui: &mut egui::Ui) {
        cut_craft_bug_report::mark(ui, "Status bar");
        let mut swap = false;
        // what the pointer is over or the next step, else the last message; while the
        // pointer is on the bar, to read it, what it said last stays
        let kept = egui::Id::unique("status_bar_shown");
        let fresh = cut_craft_bug_report::hint::current(ui.ctx())
            .unwrap_or_else(|| self.status.message.clone());
        let on_bar = ui.rect_contains_pointer(ui.max_rect());
        let full = match ui.data(|d| d.get_temp::<String>(kept)) {
            Some(last) if on_bar => last,
            _ => fresh,
        };
        ui.data_mut(|d| d.insert_temp(kept, full.clone()));
        let row = ui.spacing().interact_size.y;
        ui.allocate_ui_with_layout(
            egui::vec2(ui.available_width(), row),
            Layout::right_to_left(Align::Center),
            |ui| {
                ui.set_height(row);
                // the version, at the far right, as Help ▸ About gives it
                ui.weak(tr!("panels-version", version = env!("CARGO_PKG_VERSION")))
                    .on_hover_hint(tr!("panels-cut-craft-s-version-help"));
                ui.separator();
                // the Web API's state, once it is switched on, in a slot of its own width
                if self.web_api_indicator(ui) {
                    ui.separator();
                }
                if self.screen == Screen::Job {
                    // the job's units, and the one-click way to swap them
                    let (unit, hint) = match self.doc.job.units {
                        Units::Mm => ("mm", tr!("panels-switch-to-inches")),
                        Units::Inch => ("in", tr!("panels-switch-to-millimetres")),
                    };
                    swap = ui.small_button(unit).on_hover_hint(hint).clicked();
                    if let Some((figures, _)) = self.coordinates() {
                        ui.monospace(figures);
                    }
                    ui.separator();
                }
                ui.label(tr!(
                    "panels-current-tool",
                    tool = self.status.current_tool.as_str()
                ));
                ui.separator();
                if self.screen == Screen::Job
                    && let Some(count) = self.doc.selected_vector_count().text()
                {
                    ui.label(count).on_hover_hint(SELECTION_COUNT_HINT);
                    ui.separator();
                }
                if self.screen == Screen::Job && self.options.view_3d.show_frame_rate {
                    ui.weak(self.frame_rate_note())
                        .on_hover_hint(tr!("panels-how-finely-the-3d-view"));
                    ui.separator();
                }
                // the rest of the row, never more than one line
                ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                    let galley = egui::WidgetText::from(gui::one_line(&full)).into_galley(
                        ui,
                        Some(egui::TextWrapMode::Truncate),
                        ui.available_width(),
                        egui::TextStyle::Body,
                    );
                    let cut = galley.elided || full.contains('\n');
                    let label = ui.add(egui::Label::new(galley).selectable(false));
                    if cut {
                        label.on_hover_text(full);
                    }
                });
            },
        );
        if swap {
            let to = match self.doc.job.units {
                Units::Mm => "units_inch",
                Units::Inch => "units_mm",
            };
            self.set_units(to);
        }
    }
}
