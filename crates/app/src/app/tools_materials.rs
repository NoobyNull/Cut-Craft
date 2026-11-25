//! The tool and material lists: loading and saving them, picking a tool for a form and keeping
//! toolpaths' tools up to date.

use super::{Change, CutCraftApp, Value, egui, files};

impl CutCraftApp {
    /// Reads the user's Materials Library (the starter list when there is none); a broken
    /// one is reported, kept on disk and not saved over.
    pub(super) fn load_materials(&mut self) {
        self.load_materials_at(files::materials_path());
    }

    /// [`Self::load_materials`] from `path`.
    pub(super) fn load_materials_at(&mut self, path: Option<std::path::PathBuf>) {
        self.materials_path = path;
        if let Some(path) = &self.materials_path {
            match files::load_materials(path) {
                Ok(lib) => self.materials = lib,
                Err(e) => {
                    self.status.set_message(&cut_craft_i18n::tr!(
                        "status-materials-not-saved",
                        error = e.clone()
                    ));
                    self.materials_unreadable = self.materials_path.take();
                }
            }
        }
    }

    /// Writes the Materials Library, if it has a file; an error when it could not be
    /// written, or when its file could not be read at start and so is not saved over.
    pub(super) fn save_materials(&self) -> Result<(), String> {
        if let Some(path) = &self.materials_unreadable {
            return Err(not_saved("Materials Library", path));
        }
        self.materials_path.as_deref().map_or(Ok(()), |p| {
            files::write_creating(p, &self.materials.to_json())
        })
    }

    /// Tools renamed in the Bit Library, (old name, new name): the job's toolpaths that
    /// cut with them take the new name too (one undoable step), so the file still finds the
    /// tool changer slot the tool is loaded in under its new name. Nothing is recalculated.
    /// The status bar says so after `done`.
    pub(super) fn rename_toolpath_tools(&mut self, renamed: &[(String, String)], done: &str) {
        let mut change = Change::builder(&self.doc);
        let mut count = 0;
        for (&id, toolpath) in &self.doc.toolpaths {
            let Some((_, to)) = renamed.iter().find(|(from, _)| *from == toolpath.tool.name) else {
                continue;
            };
            let mut after = toolpath.clone();
            after.tool.name.clone_from(to);
            if let Some(name) = after.params.pointer_mut("/tool/name") {
                *name = to.as_str().into();
            }
            change.replace_toolpath(id, after);
            count += 1;
        }
        if count == 0 {
            return;
        }
        let what = if count == 1 { "toolpath" } else { "toolpaths" };
        self.record(
            Ok(change.finish()),
            &format!("{done}; {count} {what} use the renamed bit's new name"),
        );
    }

    /// Reads the user's bit library; a broken one is reported and left untouched on disk.
    pub(super) fn load_tools(&mut self) {
        self.load_tools_at(files::tool_database_path());
    }

    /// [`Self::load_tools`] from `path`.
    pub(super) fn load_tools_at(&mut self, path: Option<std::path::PathBuf>) {
        self.tools_path = path;
        if let Some(path) = &self.tools_path {
            match files::load_tools_joining(path) {
                Ok((tools, report)) => {
                    self.tools = tools;
                    // a database from before lasers were a tool type gets the starter laser
                    self.tools.add_laser_if_none();
                    // sizes a bit's copies gave differently, not kept: said once
                    if let Some(first) = report.first() {
                        for line in &report {
                            log::warn!("joining a bit's per-material copies: {line}");
                        }
                        self.status.set_message(&cut_craft_i18n::tr!(
                            "status-bits-joined",
                            first = first.clone(),
                            more = report.len() - 1
                        ));
                    }
                }
                Err(e) => {
                    self.status.set_message(&cut_craft_i18n::tr!(
                        "status-tools-not-saved",
                        error = e.clone()
                    ));
                    self.tools_unreadable = self.tools_path.take();
                }
            }
        }
    }

    /// A database tool as a toolpath over the selected vectors cuts with it: for the hardest
    /// material they are on (their layers', else Job Setup's), else the active material.
    pub(super) fn pick_tool(&self, name: &str) -> Option<cut_craft_core::toolpath::Tool> {
        let material = self
            .doc
            .cutting_material(self.doc.selection.iter().copied(), &self.materials);
        self.tools.pick_in(name, &material, &self.materials)
    }

    /// Copies a database tool, cutting the material under the selection, into the open form.
    pub(super) fn choose_tool(&mut self, name: &str) {
        let picked = self.pick_tool(name);
        if let (Some(form), Some(tool)) = (&mut self.form, picked) {
            form.params["tool"] = serde_json::to_value(tool).expect("tools serialize");
        }
    }

    /// The Tool Database window, `manager`, with the active machine's tool changer, its
    /// tools loaded with the tools.
    pub(super) fn open_tool_manager(&mut self, mut manager: cut_craft_tool_database::Manager) {
        manager.library = self.materials.clone();
        manager.changer = self
            .machines
            .active()
            .filter(|m| !m.slots.is_empty())
            .map(|m| cut_craft_tool_database::Changer {
                machine: m.name.clone(),
                slots: m.slots.clone(),
            });
        // tools loaded before slots set their numbers show those numbers now
        if let Some(changer) = &manager.changer {
            manager.draft.number_by_slots(&changer.slots);
        }
        // what it opens on, slots and all, is as kept: nothing waits for Apply yet
        manager.applied();
        self.tool_manager = Some(manager);
    }

    /// A toolpath form's Select…: the Tool Database as Select Tool, on `current`; its Select
    /// gives the form the tool shown, with its own feeds, speeds and pass depth. A toolpath
    /// form's offers only lasers for a laser toolpath, and only bits for any other.
    pub(super) fn select_tool(&mut self, current: &str) {
        let mut manager = cut_craft_tool_database::Manager::selecting(&self.tools, current);
        manager.lasers = self
            .form
            .as_ref()
            .filter(|f| f.kind.is_toolpath())
            .map(|f| f.kind.needs() == cut_craft_core::Needs::Laser);
        self.open_tool_manager(manager);
        self.picking = None;
    }

    /// Select Tool's Select: `name` for what it was opened for (a tool of one of the form's
    /// lists, or a tool changer holder), or else as the open form's tool.
    pub(super) fn selected_tool(&mut self, name: &str) {
        match self.picking.take() {
            Some(super::PickFor::Listed(which, at)) => {
                let picked = self.pick_tool(name);
                if let (Some(form), Some(tool)) = (&mut self.form, picked) {
                    super::put_listed(&mut form.params, which, at, &tool);
                }
            }
            Some(super::PickFor::Holder(slot)) => {
                if let Some(machine) = self.machines.active_mut() {
                    machine.load_tool(name, slot);
                }
                self.tool_changer_changed();
            }
            // handled by the Materials Library window
            Some(super::PickFor::Material) => {}
            None => self.choose_tool(name),
        }
    }

    /// The Tool Database, opened on the open form's tool; saving it updates the form.
    pub(super) fn edit_tool(&mut self, ctx: &egui::Context, name: Option<String>) {
        self.invoke(ctx, "tool_database");
        if let Some(manager) = &mut self.tool_manager {
            manager.pick = name
                .and_then(|n| manager.draft.index_of(&n))
                .map(cut_craft_tool_database::Pick::Tool);
        }
    }

    /// The open form's tool again from the database, after the database changed.
    pub(super) fn refresh_form_tool(&mut self) {
        let name = self
            .form
            .as_ref()
            .and_then(|f| f.params.get("tool")?["name"].as_str().map(str::to_owned));
        if let Some(name) = name.filter(|n| self.tools.get(n).is_some()) {
            self.choose_tool(&name);
            if let Some(form) = &mut self.form {
                form.dirty = true;
            }
        }
        // the form's lists of tools too, with what the database has for them now
        let material = self
            .doc
            .cutting_material(self.doc.selection.iter().copied(), &self.materials);
        let Some(form) = &mut self.form else { return };
        for at in ["/clearance_tools", "/tools", "/rest/after"] {
            let Some(list) = form.params.pointer_mut(at).and_then(Value::as_array_mut) else {
                continue;
            };
            for item in list {
                let name = item["name"].as_str().unwrap_or_default().to_owned();
                if let Some(tool) = self.tools.pick_in(&name, &material, &self.materials) {
                    *item = serde_json::to_value(tool).expect("tools serialize");
                }
            }
        }
    }

    /// [`Self::tools_changed`] after the Tool Database's Apply, OK or Select, with the
    /// job's toolpaths following the tools `renamed`: the status bar says what became of
    /// the save (never "saved" when it was not) and then the renamed toolpaths.
    pub(super) fn tools_kept(&mut self, done: &str, renamed: &[(String, String)]) {
        self.tools_changed(done);
        let said = self.status.message.clone();
        self.rename_toolpath_tools(renamed, &said);
        // a warning on the renamed toolpaths still leaves the save's outcome said
        if !self.status.message.starts_with(&said) {
            let message = format!("{said}. {}", self.status.message);
            self.status.set_message(&message);
        }
    }

    /// Writes the tool database, if it has a file, and reports `done` or the failure.
    pub(super) fn tools_changed(&mut self, done: &str) {
        if let Some(path) = &self.tools_unreadable {
            let message = not_saved("Tool database", path);
            self.status.set_message(&message);
            return;
        }
        let result = self
            .tools_path
            .as_deref()
            .map_or(Ok(()), |path| files::save_tools(&self.tools, path));
        self.status
            .set_message(&result.map_or_else(|e| e, |()| done.to_owned()));
    }
}

/// What a save says when `what`'s file, at `path`, could not be read when Cut Craft started:
/// the changes are kept until it closes, but not written over that file.
pub(super) fn not_saved(what: &str, path: &std::path::Path) -> String {
    format!(
        "{what} not saved: {} could not be read when Cut Craft started, so it is not written \
         over. Fix or rename that file, then restart Cut Craft; your changes are kept only \
         until it closes",
        path.display()
    )
}
