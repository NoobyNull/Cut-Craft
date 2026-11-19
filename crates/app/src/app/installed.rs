//! What the user installed, gadgets and add-ons: listed in Gadgets ▸ My Gadgets, and given
//! a Gadgets section of the tools (in every window layout) once there is one. Nothing
//! already there moves for it: it comes after every other group.

use super::CutCraftApp;

/// The tools section of what the user installed.
pub(super) const GADGETS: &str = "Gadgets";

impl CutCraftApp {
    /// The add-ons folder, and what is installed there.
    pub(super) fn load_addons(&mut self) {
        self.addons = crate::addons::Addons::new(crate::depth::addons_dir());
        self.refresh_installed();
    }

    /// Reads which add-ons are installed again, and lists them with the gadgets in Gadgets ▸
    /// My Gadgets.
    pub(super) fn refresh_installed(&mut self) {
        self.installed_addons = self
            .addons
            .dir()
            .map_or_default(crate::addons::installed_tools);
        let items = self.installed_items();
        self.menus.set_my_gadgets(&items);
        self.start_menus.set_my_gadgets(&items);
    }

    /// Gadgets ▸ My Gadgets: the gadgets, then the add-ons, each with the action that runs
    /// it; an add-on whose tool Cut Craft does not know opens Add-ons.
    pub(super) fn installed_items(&self) -> Vec<(String, String)> {
        let gadgets = self
            .my_gadgets
            .iter()
            .map(|(label, id)| (label.clone(), (*id).to_owned()));
        let addons = self
            .installed_addons
            .iter()
            .map(|(name, tool)| (name.clone(), tool.unwrap_or("addons").to_owned()));
        gadgets.chain(addons).collect()
    }

    /// The Gadgets section's tools: the installed gadgets and the tools add-ons bring. None,
    /// and so no section, until something is installed.
    pub(super) fn gadget_tools(&self) -> Vec<&'static str> {
        let mut tools: Vec<&'static str> = self.my_gadgets.iter().map(|&(_, id)| id).collect();
        for tool in self.installed_addons.iter().filter_map(|&(_, tool)| tool) {
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
