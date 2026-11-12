//! Send to Machine: Save Toolpaths' files (and the job sheet) saved straight into the
//! project's own folder in the machine's files, when the computer can see them (a wireless
//! controller's shared card, or its card or stick plugged in here); and Machine ▸ Machine
//! Files, to see what is there, upload, rename, delete and make folders.
// ponytail: the files are read and written as the window is used, so a share that hangs
// holds the window until the system gives up on it (as the system's file dialog does)

use super::{CutCraftApp, SaveForm, dialog_size, file_dialog, message_dialog};
use cut_craft_machines::send::{Folder, Item, SendTo, Store, safe_file_name, safe_name, size_text};
use cut_craft_machines::webdav;
use eframe::egui;
use std::path::{Path, PathBuf};

/// Says no machine folder is set, and where to set it.
pub(super) const NO_MACHINE_FOLDER: &str = "This machine's files folder is not set: set it in \
     Machine ▸ Machine Configuration, under G-code Files";

impl CutCraftApp {
    /// Save Toolpaths' Send to Machine: the checks before cutting, then the form's files and
    /// (when ticked) the job sheet, straight into the project's folder in the active
    /// machine's files, asking first before replacing any already there.
    pub(super) fn send_toolpaths(&mut self, form: &SaveForm) {
        let Some(store) = self.machines.active().and_then(|m| store_of(&m.send_to)) else {
            return self.status.set_message(NO_MACHINE_FOLDER);
        };
        let Some((post, told)) = self.ready_to_save(form) else {
            return;
        };
        let result = self.send_to(form, &post, store.as_ref(), &|names| ask_replace(names));
        self.status
            .set_message(&result.map_or_else(|e| e, |done| format!("{done}{told}")));
    }

    /// Writes the form's files through `post` (and the job sheet) and sends them to the
    /// project's folder in `store`, once `replace` says yes to replacing those there
    /// already; what was sent. Send to Machine short of its dialogs.
    pub(super) fn send_to(
        &mut self,
        form: &SaveForm,
        post: &cut_craft_post::Post,
        store: &dyn Store,
        replace: &dyn Fn(&[String]) -> bool,
    ) -> Result<String, String> {
        let (key, name) = self.project();
        let Some(send_to) = self.machines.active_mut().map(|m| &mut m.send_to) else {
            return Err(NO_MACHINE_FOLDER.to_owned());
        };
        let folder = send_to.project_folder(&key, &name);
        let short = send_to.short_names;
        self.machines_changed("");
        let staging = staging_folder();
        let sent = self.stage_and_send(form, post, store, &folder, short, &staging, replace);
        let _ = std::fs::remove_dir_all(&staging);
        sent
    }

    #[allow(clippy::too_many_arguments)]
    fn stage_and_send(
        &mut self,
        form: &SaveForm,
        post: &cut_craft_post::Post,
        store: &dyn Store,
        folder: &str,
        short: bool,
        staging: &Path,
        replace: &dyn Fn(&[String]) -> bool,
    ) -> Result<String, String> {
        // made safe before it is written here too: Windows refuses `:` and the like
        let stem = format!(
            "{}.{}",
            safe_name(&self.output_name(form), short),
            post.extension
        );
        let written = self.write_toolpath_files(form, post, &staging.join(stem))?;
        // each file as it goes on the card, in the order they are run
        let mut files: Vec<(PathBuf, String)> = written
            .iter()
            .map(|p| (p.clone(), safe_file_name(&super::file_name(p), short)))
            .collect();
        if form.job_sheet {
            let outputs = files.iter().map(|(_, n)| (n.clone(), true)).collect();
            let name = self.project().1;
            let sheet = self.job_sheet_with(outputs, staging, &name)?;
            let on_card = safe_file_name(&sheet, short);
            files.push((staging.join(sheet), on_card));
        }
        if let Some(busy) = store.busy() {
            return Err(format!("Not sent: {busy}"));
        }
        let at = [folder.to_owned()];
        let mut there = vec![];
        for (_, name) in &files {
            if store.exists(&at, name)? {
                there.push(name.clone());
            }
        }
        if !there.is_empty() && !replace(&there) {
            return Err("Not sent: the files on the machine were kept".to_owned());
        }
        for (path, name) in &files {
            let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
            store.upload(&at, name, &bytes)?;
        }
        let machine = self.machines.active().map_or("", |m| m.name.as_str());
        Ok(match files.as_slice() {
            [(_, one)] => format!("Sent {one} to {machine}, in its folder {folder}"),
            _ => format!(
                "Sent {} files to {machine}, in its folder {folder}",
                files.len()
            ),
        })
    }

    /// Machine ▸ Machine Files (and its buttons): the active machine's files, opened in the
    /// project's folder when it has one there.
    pub(super) fn open_machine_files(&mut self, name: &str, send_to: &SendTo) {
        let Some(store) = store_of(send_to) else {
            return self.status.set_message(NO_MACHINE_FOLDER);
        };
        let (key, _) = self.project();
        let mut files = MachineFiles::new(name, store, send_to.short_names);
        if let Some(folder) = send_to.projects.get(&key) {
            files.open_folder(folder);
            if files.items.is_err() {
                files.at.clear();
                files.refresh();
            }
        }
        self.machine_files = Some(files);
    }

    /// Machine Files' window, while it is open.
    pub(super) fn machine_files_window(&mut self, ctx: &egui::Context) {
        let Some(files) = self.machine_files.as_mut() else {
            return;
        };
        let mut open = true;
        let mut upload = false;
        let (id, size) = (egui::Id::unique("machine_files"), dialog_size(ctx));
        let title = format!("Machine Files: {}", files.machine);
        cut_craft_bug_report::dialog(ctx, id, &title, size, &mut open, |ui| {
            upload = files.show(ui);
        });
        if upload && let Some(paths) = file_dialog().pick_files() {
            files.upload(&paths);
        }
        if !open {
            self.machine_files = None;
        }
    }
}

/// The machine's files as this computer reaches them now: a web address as it is, and a
/// share kept by its network location (`smb://...`, as a desktop's share mounts are) found
/// where it is mounted now.
fn store_of(send_to: &SendTo) -> Option<Box<dyn Store>> {
    let place = send_to.folder.trim();
    if !send_to.is_set() || webdav::is_address(place) {
        return send_to.store();
    }
    let kept = PathBuf::from(place);
    let found = crate::clipart::network::find(&kept).unwrap_or(kept);
    Some(Box::new(Folder::new(found)))
}

/// A fresh folder of this computer's to write the files in before they are sent.
fn staging_folder() -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    std::env::temp_dir().join(format!("cut-craft-send-{}-{nanos}", std::process::id()))
}

/// Asks before replacing `names` on the machine, which may be cutting one of them.
fn ask_replace(names: &[String]) -> bool {
    message_dialog()
        .set_title(super::APP_NAME)
        .set_level(rfd::MessageLevel::Warning)
        .set_description(format!(
            "These files are on the machine already:\n\n• {}\n\nReplace them?\n\nIf the \
             machine is cutting one of them now, choose No and send again once it has \
             finished.",
            names.join("\n• ")
        ))
        .set_buttons(rfd::MessageButtons::YesNo)
        .show()
        == rfd::MessageDialogResult::Yes
}

/// Machine Files: a folder of the machine's, what is in it, and what is being done to it.
pub(super) struct MachineFiles {
    machine: String,
    store: Box<dyn Store>,
    short: bool,
    /// The folder shown, from the top of the machine's files.
    pub(super) at: Vec<String>,
    pub(super) items: Result<Vec<Item>, String>,
    /// The thing being renamed and its new name so far.
    renaming: Option<(String, String)>,
    /// The thing Delete was pressed on once; a second press deletes it.
    deleting: Option<String>,
    /// New Folder's name so far.
    new_folder: Option<String>,
    /// What the last change did, or why it couldn't.
    pub(super) message: Result<String, String>,
}

impl MachineFiles {
    pub(super) fn new(machine: &str, store: Box<dyn Store>, short: bool) -> Self {
        let mut files = Self {
            machine: machine.to_owned(),
            store,
            short,
            at: vec![],
            items: Ok(vec![]),
            renaming: None,
            deleting: None,
            new_folder: None,
            message: Ok(String::new()),
        };
        files.refresh();
        files
    }

    pub(super) fn refresh(&mut self) {
        self.items = self.store.list(&self.at);
        self.renaming = None;
        self.deleting = None;
    }

    pub(super) fn open_folder(&mut self, name: &str) {
        self.at.push(name.to_owned());
        self.refresh();
    }

    fn done(&mut self, result: Result<String, String>) {
        self.message = result;
        self.refresh();
    }

    /// `typed` as the card can hold it, saying so when it had to change.
    fn safe(&self, typed: &str, folder: bool) -> (String, String) {
        let safe = if folder {
            safe_name(typed, self.short)
        } else {
            safe_file_name(typed, self.short)
        };
        let note = if safe == typed.trim() {
            String::new()
        } else {
            format!(" (as {safe}: the machine's card can't hold the name as typed)")
        };
        (safe, note)
    }

    /// Whether the machine can be changed now; else says why not.
    fn free(&mut self) -> bool {
        match self.store.busy() {
            Some(busy) => {
                self.message = Err(busy);
                false
            }
            None => true,
        }
    }

    pub(super) fn upload(&mut self, paths: &[PathBuf]) {
        if !self.free() {
            return;
        }
        let mut sent = 0;
        for path in paths {
            let name = safe_file_name(&super::file_name(path), self.short);
            let result = std::fs::read(path)
                .map_err(|e| format!("{}: {e}", path.display()))
                .and_then(|bytes| self.store.upload(&self.at, &name, &bytes));
            if let Err(e) = result {
                return self.done(Err(e));
            }
            sent += 1;
        }
        self.done(Ok(format!(
            "Uploaded {sent} file{}",
            if sent == 1 { "" } else { "s" }
        )));
    }

    pub(super) fn rename(&mut self, from: &str, to: &str) {
        if !self.free() {
            return;
        }
        let folder = self
            .items
            .as_ref()
            .is_ok_and(|items| items.iter().any(|i| i.name == from && i.folder));
        let (to, note) = self.safe(to, folder);
        let result = self.store.rename(&self.at, from, &to);
        self.done(result.map(|()| format!("Renamed {from} to {to}{note}")));
    }

    pub(super) fn delete(&mut self, name: &str) {
        if !self.free() {
            return;
        }
        let result = self.store.delete(&self.at, name);
        self.done(result.map(|()| format!("Deleted {name}")));
    }

    pub(super) fn make_folder(&mut self, name: &str) {
        if !self.free() {
            return;
        }
        let (name, note) = self.safe(name, true);
        let result = self.store.make_folder(&self.at, &name);
        self.done(result.map(|()| format!("Made the folder {name}{note}")));
    }

    /// The window's contents; whether Upload Files was pressed.
    fn show(&mut self, ui: &mut egui::Ui) -> bool {
        ui.weak(format!("The machine's files at {}", self.store.place()));
        self.path_row(ui);
        ui.separator();
        let mut asked = None;
        match &self.items {
            Err(e) => {
                ui.colored_label(ui.visuals().error_fg_color, e);
            }
            Ok(items) if items.is_empty() => {
                ui.weak("Nothing here yet. Send to Machine in Save Toolpaths puts files here");
            }
            Ok(items) => {
                let items = items.clone();
                asked = self.list(ui, &items);
            }
        }
        if let Some(ask) = asked {
            self.act(ask);
        }
        if let Some(name) = self.new_folder.as_mut() {
            let mut make = false;
            let mut cancel = false;
            ui.horizontal(|ui| {
                ui.label("New folder");
                let edit = ui.text_edit_singleline(name);
                make = ui.button("Make").clicked()
                    || (edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)));
                cancel = ui.button("Cancel").clicked();
            });
            if make && !name.trim().is_empty() {
                let name = name.clone();
                self.new_folder = None;
                self.make_folder(&name);
            } else if cancel {
                self.new_folder = None;
            }
        }
        ui.separator();
        let reachable = self.items.is_ok();
        let mut upload = false;
        ui.horizontal(|ui| {
            upload = ui
                .add_enabled(reachable, egui::Button::new("Upload Files…"))
                .on_hover_text("Copy files from this computer into this folder")
                .clicked();
            if ui
                .add_enabled(reachable, egui::Button::new("New Folder"))
                .clicked()
            {
                self.new_folder = Some(String::new());
            }
            if ui.button("Refresh").clicked() {
                self.message = Ok(String::new());
                self.refresh();
            }
        });
        match &self.message {
            Ok(m) if !m.is_empty() => {
                ui.label(m);
            }
            Err(e) => {
                ui.colored_label(ui.visuals().error_fg_color, e);
            }
            Ok(_) => {}
        }
        upload
    }

    /// Where the folder shown is: the top, then each folder in, each going back to it.
    fn path_row(&mut self, ui: &mut egui::Ui) {
        let mut back_to = None;
        ui.horizontal_wrapped(|ui| {
            if ui
                .add_enabled(!self.at.is_empty(), egui::Button::new("⬆ Up"))
                .clicked()
            {
                back_to = Some(self.at.len() - 1);
            }
            if ui.link(self.machine.as_str()).clicked() {
                back_to = Some(0);
            }
            for (i, part) in self.at.iter().enumerate() {
                ui.label("›");
                if ui.link(part).clicked() {
                    back_to = Some(i + 1);
                }
            }
        });
        if let Some(n) = back_to {
            self.at.truncate(n);
            self.refresh();
        }
    }

    /// The things in the folder, a row each with Rename and Delete; what was asked.
    fn list(&mut self, ui: &mut egui::Ui, items: &[Item]) -> Option<Ask> {
        let mut asked = None;
        egui::Grid::new("machine_files_list")
            .num_columns(3)
            .striped(true)
            .show(ui, |ui| {
                for item in items {
                    if let Some((from, to)) =
                        self.renaming.as_mut().filter(|(f, _)| *f == item.name)
                    {
                        let edit = ui.text_edit_singleline(to);
                        let enter =
                            edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                        ui.label("");
                        ui.horizontal(|ui| {
                            if ui.button("OK").clicked() || enter {
                                asked = Some(Ask::Rename(from.clone(), to.clone()));
                            }
                            if ui.button("Cancel").clicked() {
                                asked = Some(Ask::StopRenaming);
                            }
                        });
                        ui.end_row();
                        continue;
                    }
                    if item.folder {
                        if ui
                            .link(format!("🗀 {}", item.name))
                            .on_hover_text("Open this folder")
                            .clicked()
                        {
                            asked = Some(Ask::Open(item.name.clone()));
                        }
                        ui.weak("folder");
                    } else {
                        ui.label(&item.name);
                        ui.weak(size_text(item.size));
                    }
                    ui.horizontal(|ui| {
                        if ui.button("Rename").clicked() {
                            asked = Some(Ask::StartRenaming(item.name.clone()));
                        }
                        if self.deleting.as_deref() == Some(item.name.as_str()) {
                            let what = if item.folder {
                                "Delete it and all in it"
                            } else {
                                "Delete it"
                            };
                            let sure = egui::Button::new(
                                egui::RichText::new(what).color(ui.visuals().error_fg_color),
                            );
                            if ui.add(sure).clicked() {
                                asked = Some(Ask::Delete(item.name.clone()));
                            }
                            if ui.button("Keep").clicked() {
                                asked = Some(Ask::KeepIt);
                            }
                        } else if ui
                            .button("Delete")
                            .on_hover_text("Asks once more before deleting")
                            .clicked()
                        {
                            asked = Some(Ask::ConfirmDelete(item.name.clone()));
                        }
                    });
                    ui.end_row();
                }
            });
        asked
    }

    fn act(&mut self, ask: Ask) {
        match ask {
            Ask::Open(name) => {
                self.message = Ok(String::new());
                self.open_folder(&name);
            }
            Ask::StartRenaming(name) => {
                self.deleting = None;
                self.renaming = Some((name.clone(), name));
            }
            Ask::StopRenaming => self.renaming = None,
            Ask::Rename(from, to) => {
                if from == to.trim() || to.trim().is_empty() {
                    self.renaming = None;
                } else {
                    self.rename(&from, &to);
                }
            }
            Ask::ConfirmDelete(name) => {
                self.renaming = None;
                self.deleting = Some(name);
            }
            Ask::KeepIt => self.deleting = None,
            Ask::Delete(name) => self.delete(&name),
        }
    }
}

/// What a row of Machine Files asked for.
enum Ask {
    Open(String),
    StartRenaming(String),
    StopRenaming,
    Rename(String, String),
    ConfirmDelete(String),
    KeepIt,
    Delete(String),
}
