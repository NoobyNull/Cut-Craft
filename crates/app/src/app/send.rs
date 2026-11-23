//! Send to Machine: Save Toolpaths' files (and the job sheet) saved straight into the
//! project's own folder in the machine's files, when the computer can see them (a wireless
//! controller's shared card, or its card or stick plugged in here); and Machine ▸ Machine
//! Files, to see what is there, upload, rename, delete and make folders.
// ponytail: the files are read and written as the window is used, so a share that hangs
// holds the window until the system gives up on it (as the system's file dialog does)

use super::{CutCraftApp, SaveForm, dialog_size, message_dialog};
use crate::options::EarlierGcode;
use cut_craft_i18n::tr;
use cut_craft_machines::send::{
    Folder, Item, SendTo, Store, distinct_names, safe_file_name, safe_name, size_text,
};
use cut_craft_machines::webdav;
use eframe::egui;
use std::path::{Path, PathBuf};

/// Says no machine folder is set, and where to set it.
pub(super) const NO_MACHINE_FOLDER: &str = "This machine's files folder is not set: set it in \
     Machine ▸ Machine Configuration, under G-code Files";

impl CutCraftApp {
    /// Save Toolpaths' Send to Machine: the checks before cutting, then the form's files and
    /// (when ticked) the job sheet, straight into the project's folder in the active
    /// machine's files. Files an earlier send left there (the same names, and old numbered,
    /// tile and side files) are replaced or archived as Options say, or asked about.
    pub(super) fn send_toolpaths(&mut self, form: &SaveForm) {
        let Some(post) = self.ready_to_save(form, true) else {
            return;
        };
        let choice = self.options.confirmations.earlier_gcode;
        self.send_choosing(form, post, choice);
    }

    /// Send to Machine once the checks are done, doing `choice` with an earlier send's
    /// files; Ask holds it for the question ([`Self::answer_earlier`] sends on).
    pub(super) fn send_choosing(
        &mut self,
        form: &SaveForm,
        post: cut_craft_post::Post,
        choice: EarlierGcode,
    ) {
        let Some(store) = self.machines.active().and_then(|m| store_of(&m.send_to)) else {
            return self.status.set_message(NO_MACHINE_FOLDER);
        };
        let asked = std::cell::RefCell::new(vec![]);
        let decide = |names: &[String]| {
            if choice == EarlierGcode::Ask {
                names.clone_into(&mut asked.borrow_mut());
                None
            } else {
                Some(choice)
            }
        };
        let result = self.send_to(form, &post, store.as_ref(), &decide);
        let earlier = asked.into_inner();
        if !earlier.is_empty() {
            self.earlier_ask = Some(super::EarlierAsk {
                form: form.clone(),
                post,
                path: PathBuf::new(),
                earlier: earlier.into_iter().map(PathBuf::from).collect(),
                remember: false,
                sending: true,
            });
            return;
        }
        let left_out = self.left_out(form);
        let said = result.map(|said| super::output::with_left_out(said, &left_out));
        self.status.set_message(&said.unwrap_or_else(|e| e));
    }

    /// Writes the form's files through `post` (and the job sheet) and sends them to the
    /// project's folder in `store`. When files of an earlier send are there (the same
    /// names, or ones an earlier layout of tiles, numbered or side files left), `earlier`
    /// says what to do with them: Replace (the new ones over them, the rest deleted),
    /// Archive (moved into a dated folder under Archive first), or `None` to send nothing.
    /// What was sent. Send to Machine short of its dialogs.
    pub(super) fn send_to(
        &mut self,
        form: &SaveForm,
        post: &cut_craft_post::Post,
        store: &dyn Store,
        earlier: &dyn Fn(&[String]) -> Option<EarlierGcode>,
    ) -> Result<String, String> {
        let (key, name) = self.project();
        let Some(send_to) = self.machines.active_mut().map(|m| &mut m.send_to) else {
            return Err(NO_MACHINE_FOLDER.to_owned());
        };
        let folder = send_to.project_folder(&key, &name);
        let short = send_to.short_names;
        self.machines_changed("");
        let staging = staging_folder();
        let sent = self.stage_and_send(form, post, store, &folder, short, &staging, earlier);
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
        earlier: &dyn Fn(&[String]) -> Option<EarlierGcode>,
    ) -> Result<String, String> {
        // made safe before it is written here too: Windows refuses `:` and the like
        let base = safe_name(&self.output().output_name(form), short);
        let stem = format!("{base}.{}", post.extension);
        let written = self.write_toolpath_files(form, post, &staging.join(stem))?;
        // each file as it goes on the card, in the order they are run
        let mut files: Vec<(PathBuf, String)> = written
            .iter()
            .map(|p| (p.clone(), safe_file_name(&super::file_name(p), short)))
            .collect();
        number_clashes(&mut files, short);
        if form.job_sheet {
            let outputs = files.iter().map(|(_, n)| (n.clone(), true)).collect();
            let name = self.project().1;
            let sheet = self.job_sheet_with(outputs, staging, &name)?;
            let on_card = safe_file_name(&sheet, short);
            files.push((staging.join(sheet), on_card));
            number_clashes(&mut files, short);
        }
        if let Some(busy) = store.busy() {
            return Err(format!("Not sent: {busy}"));
        }
        let at = [folder.to_owned()];
        // what an earlier send left: the names about to be sent, and its other G-code (old
        // tiles, numbered or side files no longer made)
        let listed = store.list(&at).unwrap_or_default();
        let mut there: Vec<String> = listed
            .iter()
            .filter(|i| !i.folder)
            .map(|i| i.name.clone())
            .filter(|n| crate::files::of_earlier_save(n, &base, &post.extension))
            .collect();
        for (_, name) in &files {
            if !there.contains(name) && store.exists(&at, name)? {
                there.push(name.clone());
            }
        }
        there.sort();
        let mut archived = String::new();
        if !there.is_empty() {
            match earlier(&there) {
                None | Some(EarlierGcode::Ask) => {
                    return Err("Not sent: the files on the machine were kept".to_owned());
                }
                Some(EarlierGcode::Archive) => {
                    let into = archive_on_machine(store, &at, &base, &there)?;
                    archived = format!("; the earlier files are in {}", into.join("/"));
                    there.clear();
                }
                Some(EarlierGcode::Replace) => {}
            }
        }
        for (path, name) in &files {
            let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
            store.upload(&at, name, &bytes)?;
        }
        // replaced: what the new files did not write over goes, so no old tile is left to run
        for name in there.iter().filter(|n| !files.iter().any(|(_, f)| f == *n)) {
            store.delete(&at, name)?;
        }
        let machine = self.machines.active().map_or("", |m| m.name.as_str());
        Ok(match files.as_slice() {
            [(_, one)] => format!("Sent {one} to {machine}, in its folder {folder}{archived}"),
            _ => format!(
                "Sent {} files to {machine}, in its folder {folder}{archived}",
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
        if upload
            && let Some(paths) =
                super::filtered_dialog(super::Dialog::UploadToMachine, &self.registry).pick_files()
        {
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

/// Moves `names` (an earlier send's files in folder `at`) into a new dated folder under
/// Archive there, `Archive/<base> <when>` (` (2)` added when one of that name is there);
/// that folder.
fn archive_on_machine(
    store: &dyn Store,
    at: &[String],
    base: &str,
    names: &[String],
) -> Result<Vec<String>, String> {
    let archive: Vec<String> = at.iter().cloned().chain(["Archive".to_owned()]).collect();
    if !store
        .list(at)?
        .iter()
        .any(|i| i.folder && i.name == "Archive")
    {
        store.make_folder(at, "Archive")?;
    }
    let when = chrono::Local::now().naive_local().format("%Y-%m-%d %H%M%S");
    let taken: Vec<String> = store.list(&archive)?.into_iter().map(|i| i.name).collect();
    let first = format!("{base} {when}");
    // one more name than there are taken is always free
    let name = std::iter::once(first.clone())
        .chain((2..=taken.len() + 2).map(|n| format!("{first} ({n})")))
        .find(|n| !taken.contains(n))
        .unwrap_or(first);
    store.make_folder(&archive, &name)?;
    let into: Vec<String> = archive.into_iter().chain([name]).collect();
    for file in names {
        store.move_into(at, file, &into)?;
    }
    Ok(into)
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

    /// Upload Files: `paths` copied into the folder shown, asking first before any replaces
    /// a file already there (the machine may be cutting it).
    pub(super) fn upload(&mut self, paths: &[PathBuf]) {
        self.upload_asking(paths, &ask_replace);
    }

    /// [`MachineFiles::upload`], asking `replace` about the files already there.
    pub(super) fn upload_asking(&mut self, paths: &[PathBuf], replace: &dyn Fn(&[String]) -> bool) {
        if !self.free() {
            return;
        }
        let named: Vec<(&PathBuf, String)> = paths
            .iter()
            .map(|p| (p, safe_file_name(&super::file_name(p), self.short)))
            .collect();
        let mut there = vec![];
        for (_, name) in &named {
            match self.store.exists(&self.at, name) {
                Ok(true) => there.push(name.clone()),
                Ok(false) => {}
                Err(e) => return self.done(Err(e)),
            }
        }
        if !there.is_empty() && !replace(&there) {
            return self.done(Err(
                "Not uploaded: the files on the machine were kept".to_owned()
            ));
        }
        let mut sent = 0;
        for (path, name) in named {
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
        ui.weak(tr!(
            "send-machine-files-at",
            place = self.store.place().clone()
        ));
        self.path_row(ui);
        ui.separator();
        let mut asked = None;
        match &self.items {
            Err(e) => {
                ui.colored_label(ui.visuals().error_fg_color, e);
            }
            Ok(items) if items.is_empty() => {
                ui.weak(tr!("send-nothing-here-yet-send-to"));
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
                ui.label(tr!("send-new-folder-2"));
                let edit = ui.text_edit_singleline(name);
                make = ui.button(tr!("send-make")).clicked()
                    || (edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)));
                cancel = ui.button(tr!("send-cancel")).clicked();
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
                .add_enabled(reachable, egui::Button::new(tr!("send-upload-files")))
                .on_hover_text(tr!("send-copy-files-from-this-computer"))
                .clicked();
            if ui
                .add_enabled(reachable, egui::Button::new(tr!("send-new-folder")))
                .clicked()
            {
                self.new_folder = Some(String::new());
            }
            if ui.button(tr!("send-refresh")).clicked() {
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
                .add_enabled(!self.at.is_empty(), egui::Button::new(tr!("send-up")))
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
                            if ui.button(tr!("send-ok")).clicked() || enter {
                                asked = Some(Ask::Rename(from.clone(), to.clone()));
                            }
                            if ui.button(tr!("send-cancel")).clicked() {
                                asked = Some(Ask::StopRenaming);
                            }
                        });
                        ui.end_row();
                        continue;
                    }
                    if item.folder {
                        if ui
                            .link(format!("🗀 {}", item.name))
                            .on_hover_text(tr!("send-open-this-folder"))
                            .clicked()
                        {
                            asked = Some(Ask::Open(item.name.clone()));
                        }
                        ui.weak(tr!("send-folder"));
                    } else {
                        ui.label(&item.name);
                        ui.weak(size_text(item.size));
                    }
                    ui.horizontal(|ui| {
                        if ui.button(tr!("send-rename")).clicked() {
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
                            if ui.button(tr!("send-keep")).clicked() {
                                asked = Some(Ask::KeepIt);
                            }
                        } else if ui
                            .button(tr!("send-delete"))
                            .on_hover_text(tr!("send-asks-once-more-before-deleting"))
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

/// `files`' names on the card made different where they clash: short names can cut two
/// files to the same 8 letters, and the second would overwrite the first.
fn number_clashes(files: &mut [(PathBuf, String)], short: bool) {
    let names: Vec<String> = files.iter().map(|(_, n)| n.clone()).collect();
    for ((_, name), distinct) in files.iter_mut().zip(distinct_names(&names, short)) {
        *name = distinct;
    }
}
