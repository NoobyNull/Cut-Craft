//! Unsigned code: gadgets and add-on programs not signed with the app.

use super::super::*;

impl Harness {
    // ---- unsigned code ----

    /// The answer the user gives whenever PowerWhittle asks whether to run unsigned code.
    pub(in super::super) fn answer_unsigned_with(
        &mut self,
        answer: fn(&crate::trust::Code) -> crate::trust::Answer,
    ) {
        self.app.trust.ask = answer;
    }

    /// Puts the unsigned gadget `source`, from the file `file`, in My Gadgets as PowerWhittle
    /// finds it on starting: unread, by its file's name. Its command id.
    pub(in super::super) fn given_unsigned_gadget(
        &mut self,
        file: &str,
        source: &str,
    ) -> &'static str {
        let commands = std::sync::Arc::new(crate::commands::registry());
        let (gadget, waiting) = cut_craft_scripting::waiting_gadget(file, source, commands);
        let id = cut_craft_core::DynCommand::id(&gadget);
        std::sync::Arc::get_mut(&mut self.app.registry)
            .expect("nothing else holds the commands between frames")
            .add_dyn(Box::new(gadget));
        self.app.my_gadgets.push((file.to_owned(), id));
        let code = crate::trust::Code::gadget(id, file, source.as_bytes());
        let waiting = Some(waiting);
        self.app
            .unsigned_gadgets
            .insert(id.to_owned(), crate::gadgets::Unsigned { code, waiting });
        id
    }

    /// My Gadgets, as its list names them.
    pub(in super::super) fn my_gadgets(&self) -> Vec<String> {
        self.app
            .installed_items()
            .into_iter()
            .map(|(n, _)| n)
            .collect()
    }

    /// Add-ons kept in `folder`, as a settings folder holds them.
    pub(in super::super) fn given_add_ons_folder(&mut self, folder: std::path::PathBuf) {
        self.app.addons = crate::addons::Addons::new(Some(folder));
    }

    /// Starting the add-on `id`'s `program`: whether it may run (asking if unsigned).
    pub(in super::super) fn may_run_add_on(
        &mut self,
        id: &str,
        name: &str,
        program: &std::path::Path,
    ) -> bool {
        self.app.may_run_add_on(id, name, program)
    }
}
