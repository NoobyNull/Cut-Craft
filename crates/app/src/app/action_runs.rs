//! What the built-in actions ([`super::built_ins`]) run and wait for, where it is more than a
//! line and has no area file of its own: undo and redo, the previews, the guides, the
//! machine windows, and the menu items that are an existing tool reached another way.

use super::{
    BeforePreview, Change, CutCraftApp, DESIGN_TABS, Dialog, EntityId, Info, Page, Screen, Value,
    ViewTool, egui, filtered_dialog, setup_material,
};
use cut_craft_i18n::tr;

impl CutCraftApp {
    pub(super) fn undo(&mut self) {
        if self.history.undo(&mut self.doc) {
            self.status.set_message(&tr!("status-undone"));
        }
    }

    pub(super) fn redo(&mut self) {
        if self.history.redo(&mut self.doc) {
            self.status.set_message(&tr!("status-redone"));
        }
    }

    /// A job is open, not the start page.
    pub(super) fn job_open(&self) -> bool {
        self.screen == Screen::Job
    }

    /// A tool box variant can be started: its command is there and a job is open to show
    /// its form in.
    pub(super) fn variant_ready(&self, id: &str) -> bool {
        crate::flyout::variant(id).is_some_and(|v| self.registry.get(v.command).is_some())
            && self.screen != Screen::Start
    }

    /// A text (a group Draw Text made) is among the picked vectors.
    pub(super) fn text_picked(&self) -> bool {
        self.doc.selection.iter().any(|id| {
            self.doc.vector_groups.get(id).is_some_and(|g| {
                self.doc
                    .texts
                    .get(g)
                    .is_some_and(|t| t.get("text").is_some())
            })
        })
    }

    /// A toolpath is ticked to show in the Toolpath List.
    pub(super) fn any_toolpath_shown(&self) -> bool {
        self.doc.toolpaths.values().any(|t| t.visible)
    }

    /// Machine Files: the active machine's card or shared folder.
    pub(super) fn open_active_machine_files(&mut self) {
        match self.machines.active() {
            Some(machine) => {
                let (name, send_to) = (machine.name.clone(), machine.send_to.clone());
                self.open_machine_files(&name, &send_to);
            }
            None => self.status.set_message(&tr!("status-no-machine-yet")),
        }
    }

    /// Install Offline Package: a machine package picked from the disk.
    pub(super) fn install_package_picked(&mut self) {
        let dialog = filtered_dialog(Dialog::MachinePackage, &self.registry);
        if let Some(path) = dialog.pick_file() {
            self.install_package(&path);
        }
    }

    /// The current layer becomes the selected vectors' (the first picked).
    pub(super) fn activate_layer_of_selection(&mut self, ctx: &egui::Context) {
        if let Some(&id) = self.doc.selection.iter().next() {
            let layer = self.doc.layer_of(id);
            self.layer_action(ctx, layer, "set_current_layer");
        }
    }

    /// The three Join icons: Join / Close Vectors their way.
    pub(super) fn join_ends(&mut self, method: &str) {
        let params = serde_json::json!({ "method": method });
        if let Err(e) = self.apply("join_close_vectors", &params) {
            self.status.set_message(&e);
        }
    }

    /// The Design page open on its `tab` tab (Library, Components), as its icon shows.
    pub(super) fn open_design_tab(&mut self, tab: &str) {
        self.set_design_tab(
            DESIGN_TABS
                .iter()
                .position(|&t| t == tab)
                .unwrap_or_default(),
        );
        self.pages.open(Page::Design);
        self.show_inspector_tab(tab);
    }

    /// The four Create Shape icons: the form with that `profile` chosen.
    pub(super) fn create_shape_with(&mut self, profile: &str) {
        self.start_command("create_shape");
        if let Some(form) = self.form.as_mut().filter(|f| f.id == "create_shape") {
            form.params["profile"] = Value::from(profile);
        }
    }

    /// The Align Objects group's spacing buttons, by pick `order`.
    pub(super) fn space_in_order(&mut self, order: &str) {
        let command = if order.contains("left") {
            "space_horizontally"
        } else {
            "space_vertically"
        };
        if let Err(e) = self.apply(command, &serde_json::json!({ "order": order })) {
            self.status.set_message(&e);
        }
    }

    /// Every guide locked, or every guide free to move.
    pub(super) fn lock_guides(&mut self, locked: bool) {
        let guides = self
            .doc
            .guides
            .iter()
            .map(|g| cut_craft_core::document::Guide { locked, ..*g })
            .collect();
        self.set_guides(guides);
    }

    pub(super) fn check_for_updates(&mut self, ctx: &egui::Context) {
        let skipped = self.options.updates.skipped_version.clone();
        self.updates.check(ctx, true, skipped);
    }

    /// The Select tool: put away whatever tool is in hand.
    pub(super) fn put_tool_away(&mut self, _: &egui::Context) {
        (self.form, self.view_tool) = (None, None);
        self.status.current_tool = self.tool_label("select");
    }

    /// Select All Vectors On Current Layer: Select Vectors on Layer, on the current one.
    pub(super) fn select_current_layer(&mut self, _: &egui::Context) {
        let layer = serde_json::json!({ "layer": self.doc.current_layer });
        if let Err(e) = self.apply("select_layer_vectors", &layer) {
            self.status.set_message(&e);
        }
    }

    pub(super) fn show_toolpaths_tab(&mut self, _: &egui::Context) {
        self.pages.open(Page::Toolpaths);
        self.show_inspector_tab("Toolpaths");
    }

    pub(super) fn open_tool_database(&mut self, _: &egui::Context) {
        let manager = cut_craft_tool_database::Manager::new(&self.tools);
        self.open_tool_manager(manager);
    }

    /// Preview Toolpath: the picked toolpath, or a picked group's toolpaths (its sub-groups'
    /// too) in list order, as one preview.
    pub(super) fn preview_selected(&mut self, _: &egui::Context) {
        if let Some(id) = self.selected_toolpath {
            self.preview_toolpath(id);
        } else if let Some(group) = &self.selected_group {
            self.preview_ids(&self.doc.toolpaths_under_group(group));
        }
    }

    pub(super) fn undo_last_preview(&mut self, _: &egui::Context) {
        if let Some(BeforePreview(material, side)) = self.before_preview.take() {
            self.simulation = material;
            self.player.kept = None;
            (self.simulation_coarse, self.fine_3d, self.surface_gpu) = (None, None, None);
            // made before Swap Sides: turned over to the side the job is on now
            self.preview_side = side;
            self.follow_side();
            self.status.set_message(&tr!("status-undid-last-preview"));
        }
    }

    pub(super) fn reset_preview(&mut self, _: &egui::Context) {
        // a playing preview stops first, keeping what it has cut, so Undo Last Preview brings
        // back what was showing
        self.finish_playback();
        self.before_preview = Some(BeforePreview(self.simulation.take(), self.preview_side));
        (self.simulation_coarse, self.fine_3d, self.surface_gpu) = (None, None, None);
        self.player.close();
        // the Preview tab's detail goes back to Options' default
        self.display.detail = self.options.performance.preview_detail.min(100);
        self.status.set_message(&tr!("status-preview-reset"));
    }

    pub(super) fn save_shaded_image_picked(&mut self, _: &egui::Context) {
        if let Some(path) = filtered_dialog(Dialog::SaveShadedImage, &self.registry)
            .set_file_name("shaded.png")
            .save_file()
        {
            self.save_shaded_image(path);
        }
    }

    /// Edit Object: the form of what the picked thing was made with, on its own settings.
    pub(super) fn edit_object(&mut self, _: &egui::Context) {
        if let Some(command) = self.made_to_edit() {
            self.form = None;
            self.start_command(command);
        } else if let Some((id, command, params)) = self.object_to_edit() {
            self.edit_shape(id, command, params);
        }
    }

    pub(super) fn open_object_properties(&mut self, ctx: &egui::Context) {
        if let Some(id) = self.object_properties() {
            self.invoke(ctx, id);
        }
    }

    /// A display switch, turned the other way.
    pub(super) fn toggle_display(&mut self, ctx: &egui::Context, id: &str) {
        self.display.toggle(id);
        ctx.request_repaint();
    }

    /// Show Construction Lines; hidden construction lines cannot stay picked, so they are
    /// let go.
    pub(super) fn show_construction_lines(&mut self, ctx: &egui::Context) {
        self.display.toggle("show_construction_lines");
        self.doc.hide_construction = !self.display.construction;
        if !self.display.construction {
            let kept: Vec<EntityId> = (self.doc.selection.iter().copied())
                .filter(|v| !self.doc.is_construction(*v))
                .collect();
            if kept.len() < self.doc.selection.len() {
                let mut c = Change::builder(&self.doc);
                c.select(kept);
                self.record(Ok(c.finish()), "Construction lines hidden");
            }
        }
        ctx.request_repaint();
    }

    /// F1: the open form's help page, else the contents.
    pub(super) fn help_on_the_open_tool(&mut self, _: &egui::Context) {
        let topic =
            self.form.as_ref().map_or(
                "contents",
                |f| {
                    if f.brief { setup_material::ID } else { f.id }
                },
            );
        self.info = Some(Info::Help(topic));
    }

    /// Pan, Measure and Zoom Box: a tool of the 2D view's own in hand, named `id` in the
    /// status bar.
    pub(super) fn pick_view_tool(&mut self, tool: ViewTool, id: &str) {
        self.view_tool = Some(tool);
        self.status.current_tool = self.tool_label(id);
    }

    /// Transform: the move and rotate handles, or back to Select.
    pub(super) fn toggle_transform(&mut self) {
        self.rotating = false;
        self.toggle_view_tool(ViewTool::Transform, "transform");
    }

    /// Node Edit (N): the nodes' handles, or back to Select.
    pub(super) fn toggle_node_edit(&mut self) {
        self.node_mirror = [false, false];
        self.toggle_view_tool(ViewTool::NodeEdit, "node_edit");
    }

    fn toggle_view_tool(&mut self, tool: ViewTool, id: &str) {
        if self.view_tool == Some(tool) {
            self.view_tool = None;
            self.status.current_tool = self.tool_label("select");
        } else {
            self.pick_view_tool(tool, id);
        }
    }

    /// Options, with no field ringed from an earlier hint.
    pub(super) fn open_options(&mut self, ctx: &egui::Context) {
        crate::marks::clear_spotlight(ctx);
        self.options_window = true;
    }

    /// The toolpaths menu's hint: Options at Advanced rotary mode, ringed.
    pub(super) fn open_advanced_rotary_mode(&mut self, ctx: &egui::Context) {
        self.options_goto = Some("toolpaths");
        crate::marks::spotlight(ctx, "field:advanced_rotary");
        self.options_window = true;
    }
}
