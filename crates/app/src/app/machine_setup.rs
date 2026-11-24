//! Machines and posts: loading the machine list, installing machine packages and posts, and their
//! windows.

use super::{CutCraftApp, Dialog, egui, file_dialog, files, filtered_dialog, on_the_desktop};

impl CutCraftApp {
    /// The active machine's tool changer slots (empty without one), for `[T]`.
    pub(super) fn slots(&self) -> Vec<Option<String>> {
        self.machines.active().map_or_default(|m| m.slots.clone())
    }

    /// Reads the machines and finds the custom posts, kept in the config folder.
    pub(super) fn load_machines(&mut self) {
        let dir = files::config_dir();
        if let Some(path) = dir.as_ref().map(|d| d.join("machines.json")) {
            self.load_machines_at(&path);
        }
        self.posts = cut_craft_machines::Library::new(dir.map(|d| d.join("posts")));
    }

    /// Reads the machines kept at `path`. One that cannot be read is said, copied aside to
    /// `machines.json.bad`, and never saved over this run: machine changes are not saved
    /// until it is put right or moved away and Cut Craft is started again.
    pub(super) fn load_machines_at(&mut self, path: &std::path::Path) {
        match cut_craft_machines::Machines::load(path) {
            Ok(machines) => {
                self.machines = machines;
                self.machines_path = Some(path.to_owned());
            }
            Err(e) => {
                log::warn!("machines not read: {e}");
                let aside = path.with_extension("json.bad");
                let kept = match std::fs::copy(path, &aside) {
                    Ok(_) => format!("a copy is kept as {}", aside.display()),
                    Err(_) => "it is left as it is".to_owned(),
                };
                self.status.set_message(&format!(
                    "Your machines could not be read ({e}); {kept}. Machine changes will not \
                     be saved until it is fixed or moved away and Cut Craft is restarted"
                ));
                self.machines = cut_craft_machines::Machines::default();
                self.machines_path = None;
                self.machines_unsaved = true;
            }
        }
    }

    /// The active machine's tool changer was loaded differently: the machines are saved,
    /// and each tool now in a slot takes the slot as its bit number.
    pub(super) fn tool_changer_changed(&mut self) {
        // the checks made so far used the slots just changed
        self.forget_save_checks();
        self.machines_changed("Tool changer changed");
        let slots = self.machines.active().map_or_default(|m| m.slots.clone());
        if self.tools.number_by_slots(&slots) {
            self.tools_changed("Tool changer changed; bits numbered by their slots");
            self.refresh_form_tool();
        }
    }

    /// Writes the machines, if they have a file, and reports `done` or the failure.
    pub(super) fn machines_changed(&mut self, done: &str) {
        if self.machines_unsaved {
            self.status.set_message(&format!(
                "{done}, but not saved: the machines file could not be read when Cut Craft \
                 started, so it is not saved over"
            ));
            return;
        }
        let result = self
            .machines_path
            .as_deref()
            .map_or(Ok(()), |path| self.machines.save(path));
        self.status
            .set_message(&result.map_or_else(|e| e, |()| done.to_owned()));
    }

    /// Install Offline Package: the machines in a package file (`machines/` in the repository)
    /// that are not here already by name.
    pub(super) fn install_package(&mut self, path: &std::path::Path) {
        let Some(package) = self.read_package(path) else {
            return;
        };
        let (added, here) = self.machines.install(package);
        self.machines_changed(&installed_machines(added, here));
    }

    /// Machine Configuration's Install Offline Package: the package's machines added to the
    /// machines being edited, kept with Apply or OK like any other edit there.
    pub(super) fn machine_outcome_install(&mut self, path: &std::path::Path) {
        let Some(package) = self.read_package(path) else {
            return;
        };
        let Some(window) = self.machine_window.as_mut() else {
            return self.install_package(path);
        };
        let (added, here) = window.draft.install(package);
        let done = installed_machines(added, here);
        self.status
            .set_message(&format!("{done}; Apply or OK keeps them"));
    }

    /// The machine package at `path`, read; `None` with the status bar saying why not.
    fn read_package(&mut self, path: &std::path::Path) -> Option<cut_craft_machines::Machines> {
        let package = std::fs::read(path)
            .map_err(|e| crate::messages::could_not_read(path, &e))
            .and_then(|bytes| {
                serde_json::from_slice(&bytes).map_err(|e| {
                    log::warn!("{}: not a machine package ({e})", path.display());
                    crate::messages::not_a_machine_package(path)
                })
            });
        match package {
            Ok(package) => Some(package),
            Err(message) => {
                self.status.set_message(&message);
                None
            }
        }
    }

    /// Install Post-Processor: copied into the custom posts. A machine with no posts yet (or
    /// none at all) takes it, so toolpaths save through it as soon as it is installed.
    pub(super) fn install_post(&mut self, path: &std::path::Path) {
        let entry = match self.posts.install(path) {
            Ok(entry) => entry,
            Err(e) => return self.status.set_message(&e),
        };
        if self.machines.active().is_none() {
            self.machines.add();
        }
        let machine = self.machines.active_mut().expect("a machine was just made");
        if machine.posts.is_empty() {
            machine.posts.push(entry.name.clone());
            let done = format!(
                "Installed {}; {} saves toolpaths through it",
                entry.name, machine.name
            );
            self.machines_changed(&done);
        } else if entry.name.contains(" (copy ") {
            self.status.set_message(&format!(
                "Installed as {}: a post with its name was already listed, so this one is \
                 kept beside it. Pick it in Machine Configuration to use it",
                entry.name
            ));
        } else {
            self.status
                .set_message(&format!("Installed {}", entry.name));
        }
    }

    /// The Machine menu's windows, and what they ask for.
    pub(super) fn machine_windows(&mut self, ctx: &egui::Context) {
        let icons = &self.icons;
        let picture = |path: &str| icons.find(path, "").map(|p| icons.image(p));
        let look = cut_craft_machines::Look { picture: &picture };
        let machine = self.machine_window.as_mut().and_then(|w| {
            w.show(ctx, &self.posts, &look)
                .map(|o| (o, w.draft.clone()))
        });
        let post = self
            .post_window
            .as_mut()
            .and_then(|w| w.show(ctx, &self.posts, &look));
        if let Some((outcome, draft)) = machine {
            use cut_craft_machines::MachineOutcome;
            match outcome {
                MachineOutcome::Apply | MachineOutcome::Ok => {
                    self.machines = draft;
                    // the checks made so far used the holder and limits just replaced
                    self.forget_save_checks();
                    if outcome == MachineOutcome::Ok {
                        self.machine_window = None;
                    } else if let Some(w) = &mut self.machine_window {
                        w.applied();
                    }
                    self.machines_changed("Machines saved");
                    if self.sync_machine_keep_out() {
                        self.keep_out_changed();
                    }
                }
                MachineOutcome::Cancel => self.machine_window = None,
                MachineOutcome::ManagePosts => {
                    // the machine as edited, not yet applied: the one Select changes
                    self.post_window =
                        Some(cut_craft_machines::PostManager::for_machine(draft.active()));
                }
                MachineOutcome::PickFolder(which) => {
                    if let Some(folder) = file_dialog().pick_folder()
                        && let Some(machine) =
                            (self.machine_window.as_mut()).and_then(|w| w.draft.active_mut())
                    {
                        let kept = crate::clipart::network::kept(&folder).display().to_string();
                        match which {
                            cut_craft_machines::FolderFor::Programs => {
                                machine.send_to.programs = kept;
                            }
                            cut_craft_machines::FolderFor::Machine => machine.send_to.folder = kept,
                        }
                    }
                }
                MachineOutcome::InstallPackage => {
                    let dialog = filtered_dialog(Dialog::MachinePackage, &self.registry);
                    if let Some(path) = dialog.pick_file() {
                        self.machine_outcome_install(&path);
                    }
                }
                MachineOutcome::ManageFiles => {
                    if let Some(machine) = draft.active() {
                        let (name, send_to) = (machine.name.clone(), machine.send_to.clone());
                        self.open_machine_files(&name, &send_to);
                    }
                }
            }
        }
        if let Some(outcome) = post {
            self.post_outcome(outcome);
        }
    }

    pub(super) fn post_outcome(&mut self, outcome: cut_craft_machines::PostOutcome) {
        use cut_craft_machines::PostOutcome;
        match outcome {
            PostOutcome::Install => {
                if let Some(path) =
                    filtered_dialog(Dialog::InstallPostProcessor, &self.registry).pick_file()
                {
                    self.install_post(&path);
                }
            }
            PostOutcome::Select(name) => {
                let wraps = cut_craft_machines::wraps_for(&self.doc.job);
                let picked = self
                    .posts
                    .load(&name)
                    .ok()
                    .map(|p| p.wraps().map(str::to_owned));
                // into Machine Configuration's edits when it is open, else the machines
                let machines = match &mut self.machine_window {
                    Some(window) => &mut window.draft,
                    None => &mut self.machines,
                };
                if machines.active().is_none() {
                    machines.add();
                }
                let machine = machines.active_mut().expect("a machine was just made");
                if !machine.posts.contains(&name) {
                    machine.posts.push(name.clone());
                }
                // the post picked is the one toolpaths save through from now on
                machine.default_post = Some(name.clone());
                let mut done = format!("{} now saves through {name}", machine.name);
                // the default is used only for jobs it suits (flat, or wrapping the same
                // side): a job it does not suit saves through the machine's first that does
                if let Some(picked) = picked.filter(|p| p.as_deref() != wraps) {
                    let kind = match &picked {
                        None => "flat jobs".to_owned(),
                        Some(axis) => format!("rotary jobs wrapping {axis} onto A"),
                    };
                    let job = if wraps.is_some() {
                        "this job wraps round"
                    } else {
                        "this job is flat"
                    };
                    let used = (machine.posts.iter())
                        .find(|n| self.posts.load(n).is_ok_and(|p| p.wraps() == wraps));
                    done = match used {
                        Some(used) => format!(
                            "{} now saves {kind} through {name}; {job}, so it saves through \
                             {used}",
                            machine.name
                        ),
                        None => format!(
                            "{} now saves {kind} through {name}; {job}, and none of its posts \
                             suits it: add one in Machine Configuration",
                            machine.name
                        ),
                    };
                }
                if self.machine_window.is_some() {
                    self.status
                        .set_message(&format!("{done} once Machine Configuration is applied"));
                } else {
                    self.machines_changed(&done);
                }
            }
            PostOutcome::OpenFolder(dir) => {
                on_the_desktop();
                if let Err(e) = files::show_folder(&dir) {
                    self.status.set_message(&format!("{}: {e}", dir.display()));
                }
            }
            PostOutcome::Message(m) => self.status.set_message(&m),
            PostOutcome::Close => self.post_window = None,
        }
    }
}

/// The active machine's bed, width and height (none without a machine).
pub(super) fn bed(machines: &cut_craft_machines::Machines) -> [f64; 2] {
    machines.active().map_or_default(|m| [m.width, m.height])
}

/// What Install Offline Package did: `added` machines, `here` already here by name.
pub(super) fn installed_machines(added: usize, here: usize) -> String {
    let here = if here > 0 {
        format!(", {here} already here")
    } else {
        String::new()
    };
    format!("Installed {added} machines{here}")
}
