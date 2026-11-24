//! What the user installed, gadgets and add-ons: listed in Gadgets ▸ My Gadgets, and given
//! a Gadgets section of the tools (in every window layout) once there is one. Nothing
//! already there moves for it: it comes after every other group.

use super::CutCraftApp;

/// The tools section of what the user installed.
pub(super) const GADGETS: &str = "Gadgets";
/// After the name of a gadget or add-on that is not signed with Cut Craft's release key.
const NOT_OFFICIAL: &str = "\u{26a0} (not official)";

impl CutCraftApp {
    /// The add-ons folder, and what is installed there.
    pub(super) fn load_addons(&mut self) {
        self.addons = crate::addons::Addons::new(crate::depth::addons_dir());
        self.border.keys = Some(crate::borders::KeyStore::system());
        self.refresh_installed();
        let pack = self.options.language.pack.clone();
        if !pack.is_empty() {
            self.use_language(&pack);
        }
    }

    /// Shows every word in the language pack `pack` (its id; empty for English) from now
    /// on; one that is gone or will not read leaves English, and the status bar says why.
    /// Options then says English too, so picking the pack again once it is back is a change
    /// that Apply uses.
    pub(super) fn use_language(&mut self, pack: &str) {
        if let Err(e) = crate::languages::use_chosen(self.addons.dir(), pack) {
            log::warn!("{e}");
            self.status.set_message(&e);
            if self.options.language.pack == pack {
                self.options.language.pack.clear();
                self.save_options();
            }
        }
    }

    /// Reads which add-ons are installed again, and lists them with the gadgets in Gadgets ▸
    /// My Gadgets.
    pub(super) fn refresh_installed(&mut self) {
        self.installed_addons = self
            .addons
            .dir()
            .map_or_default(crate::addons::installed_tools);
        // a pack installed or removed shows in Options ▸ Language
        self.language_packs = None;
        self.relist_my_gadgets();
    }

    /// Gadgets ▸ My Gadgets listed again, as [`Self::installed_items`] has them now.
    fn relist_my_gadgets(&mut self) {
        let items = self.installed_items();
        self.menus.set_my_gadgets(&items);
        self.start_menus.set_my_gadgets(&items);
    }

    /// Whether gadget `id` may run: Cut Craft's own and signed ones always; an unsigned one
    /// once the user has said yes to it ([`crate::trust`]), asked now if they have not. One
    /// still unread is read now, and takes its own name in the menus; one that turns out
    /// broken says why in the status bar.
    pub(super) fn may_run_gadget(&mut self, id: &str) -> bool {
        let Some(gadget) = self.unsigned_gadgets.get(id) else {
            return true;
        };
        if !self.trust.may_run(&gadget.code) {
            return false;
        }
        let Some(waiting) = &gadget.waiting else {
            return true;
        };
        match waiting.open() {
            Ok(label) => {
                if let Some(gadget) = self.unsigned_gadgets.get_mut(id) {
                    label.clone_into(&mut gadget.code.name);
                    gadget.waiting = None;
                }
                for (name, gadget) in &mut self.my_gadgets {
                    if *gadget == id {
                        label.clone_into(name);
                    }
                }
                self.relist_my_gadgets();
                true
            }
            Err(e) => {
                let file = &gadget.code.name;
                self.status
                    .set_message(&format!("{file} could not be used: {e}"));
                false
            }
        }
    }

    /// Whether the add-on `id` (called `name`) may start its program `program`: a signed
    /// one always; an unsigned one once the user has said yes to this very program, asked
    /// now if they have not ([`crate::trust`]).
    pub(super) fn may_run_add_on(
        &mut self,
        id: &str,
        name: &str,
        program: &std::path::Path,
    ) -> bool {
        let code = self.addons.code(id, name, program);
        self.trust.may_run(&code)
    }

    /// Why gadget `id` may not run without asking, when it is unsigned and the user has not
    /// said yes to it: for the Web API, which never asks.
    pub(super) fn not_allowed_yet(&self, id: &str) -> Option<String> {
        let gadget = self.unsigned_gadgets.get(id)?;
        (gadget.waiting.is_some() || !self.trust.allowed(&gadget.code))
            .then(|| crate::trust::refusal(&gadget.code))
    }

    /// Gadgets ▸ My Gadgets: the gadgets, then the add-ons, each with the action that runs
    /// it; an add-on whose tool Cut Craft does not know opens Add-ons. One that is not
    /// official (not signed with Cut Craft's release key) is marked so.
    pub(super) fn installed_items(&self) -> Vec<(String, String)> {
        let named = |name: &str, official: bool| {
            if official {
                name.to_owned()
            } else {
                format!("{name} {NOT_OFFICIAL}")
            }
        };
        let gadgets = self.my_gadgets.iter().map(|(label, id)| {
            let official = !self.unsigned_gadgets.contains_key(*id);
            (named(label, official), (*id).to_owned())
        });
        let addons = self.installed_addons.iter().map(|(name, tool, signed)| {
            (named(name, *signed), tool.unwrap_or("addons").to_owned())
        });
        gadgets.chain(addons).collect()
    }

    /// The Gadgets section's tools: the installed gadgets and the tools add-ons bring. None,
    /// and so no section, until something is installed.
    pub(super) fn gadget_tools(&self) -> Vec<&'static str> {
        let mut tools: Vec<&'static str> = self.my_gadgets.iter().map(|&(_, id)| id).collect();
        for tool in self
            .installed_addons
            .iter()
            .filter_map(|&(_, tool, _)| tool)
        {
            if !tools.contains(&tool) {
                tools.push(tool);
            }
        }
        tools
    }

    /// Gadgets ▸ My Gadgets ▸ Open My Gadgets Folder: made if it is not there yet, so the
    /// user has somewhere to put `.lua` gadget files.
    pub(super) fn open_gadgets_folder(&mut self) {
        let Some(dir) = crate::gadgets::my_gadgets_dir() else {
            self.status.set_message(
                "Cut Craft has no settings folder on this computer to keep gadgets in",
            );
            return;
        };
        if let Err(e) = std::fs::create_dir_all(&dir).and_then(|()| crate::files::show_folder(&dir))
        {
            self.status.set_message(&format!(
                "Could not open the My Gadgets folder, {} ({e})",
                dir.display()
            ));
        }
    }

    /// Classic's Design page, under the sub-tabs: the Gadgets section, when there is one.
    pub(super) fn gadgets_section(&mut self, ui: &mut eframe::egui::Ui) {
        let tools = self.gadget_tools();
        if tools.is_empty() {
            return;
        }
        ui.separator();
        self.tool_sections(ui, &[(GADGETS, tools)], super::Page::Design, false);
    }
}
