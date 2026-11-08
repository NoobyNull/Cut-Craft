//! Help ▸ Add-ons: the add-ons Cut Craft can install, each with its installed version and
//! the newest on GitHub, and Install, Update and Remove. The downloading and checking is
//! [`cut_craft_updater::addons`]'s, on a thread of its own so the window keeps drawing.

use cut_craft_updater::Release;
use cut_craft_updater::Step;
use cut_craft_updater::addons::{self, Manifest, SOURCES, Source};
use eframe::egui;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, channel};

/// What is known of an add-on's newest release.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Newest {
    Unknown,
    Checking,
    Found(Release),
    Working(Step),
    Failed(String),
}

enum Msg {
    Checked(Result<Release, String>),
    Step(Step),
    Installed(Result<Manifest, String>),
}

struct Row {
    source: Source,
    installed: Option<Manifest>,
    newest: Newest,
    rx: Option<Receiver<Msg>>,
}

/// The Add-ons window.
pub struct AddOns {
    pub open: bool,
    rows: Vec<Row>,
}

impl Default for AddOns {
    fn default() -> Self {
        Self {
            open: false,
            rows: SOURCES
                .iter()
                .map(|&source| Row {
                    source,
                    installed: None,
                    newest: Newest::Unknown,
                    rx: None,
                })
                .collect(),
        }
    }
}

/// A button pressed in a row.
enum Press {
    Install,
    Remove,
}

impl AddOns {
    /// Opens the window: reads what is installed and asks GitHub what is newest.
    pub fn show(&mut self, ctx: &egui::Context, folder: Option<&Path>) {
        self.open = true;
        for row in &mut self.rows {
            row.installed = folder.and_then(|f| addons::installed(f, row.source.id));
            if matches!(row.newest, Newest::Unknown | Newest::Failed(_)) {
                check(row, ctx);
            }
        }
    }

    /// Draws the window while it is open; true when an add-on was installed or removed.
    pub fn window(&mut self, ctx: &egui::Context, folder: Option<&Path>) -> bool {
        let mut changed = false;
        for row in &mut self.rows {
            changed |= take(row);
        }
        if !self.open {
            return changed;
        }
        let mut open = true;
        let mut pressed = None;
        let size = egui::vec2(480.0, 320.0);
        cut_craft_bug_report::dialog(
            ctx,
            egui::Id::unique("add_ons"),
            "Add-ons",
            size,
            &mut open,
            |ui| {
                ui.label(
                    "Add-ons are programs that work with Cut Craft but download on their own, so \
                 Cut Craft stays small. Each is checked against Cut Craft's release key before \
                 it is installed.",
                );
                for (i, row) in self.rows.iter().enumerate() {
                    ui.separator();
                    if let Some(press) = row_ui(ui, row, folder.is_some()) {
                        pressed = Some((i, press));
                    }
                }
                ui.separator();
                if let Some(folder) = folder
                    && ui
                        .button("Open Add-ons Folder")
                        .on_hover_text(folder.display().to_string())
                        .clicked()
                {
                    let shown = std::fs::create_dir_all(folder)
                        .and_then(|()| crate::files::show_folder(folder));
                    if let Err(e) = shown {
                        log::warn!("cannot open {}: {e}", folder.display());
                    }
                }
            },
        );
        self.open = open;
        if let (Some((i, press)), Some(folder)) = (pressed, folder) {
            let row = &mut self.rows[i];
            match press {
                Press::Install => install(row, ctx, folder.to_path_buf()),
                Press::Remove => {
                    match addons::remove(folder, row.source.id) {
                        Ok(()) => row.installed = None,
                        Err(e) => row.newest = Newest::Failed(e),
                    }
                    changed = true;
                }
            }
        }
        changed
    }
}

fn row_ui(ui: &mut egui::Ui, row: &Row, can_install: bool) -> Option<Press> {
    let mut press = None;
    ui.strong(row.source.name);
    ui.label(row.source.about);
    let release = match &row.newest {
        Newest::Found(r) => Some(r),
        _ => None,
    };
    let newer = match (&row.installed, release) {
        (Some(m), Some(r)) => {
            cut_craft_updater::version_of(&m.version).is_none_or(|v| r.version > v)
        }
        (None, Some(_)) => true,
        _ => false,
    };
    ui.horizontal(|ui| {
        match &row.installed {
            Some(m) => ui.label(format!("Installed: version {}", m.version)),
            None => ui.label("Not installed"),
        };
        match &row.newest {
            Newest::Unknown | Newest::Failed(_) => {}
            Newest::Checking => {
                ui.spinner();
                ui.label("Asking GitHub…");
            }
            Newest::Found(r) if newer => {
                ui.label(format!("· version {} is available", r.version));
            }
            Newest::Found(_) => {
                ui.label("· up to date");
            }
            Newest::Working(step) => {
                ui.spinner();
                ui.label(match step {
                    Step::Downloading { done, total } => {
                        format!(
                            "Downloading {} of {} MB",
                            done / 1_000_000,
                            total.max(&1) / 1_000_000
                        )
                    }
                    Step::Checking => "Checking the download…".into(),
                    Step::Installing => "Installing…".into(),
                });
            }
        }
    });
    if let Newest::Failed(e) = &row.newest {
        ui.colored_label(ui.visuals().warn_fg_color, e);
    }
    let ready = release.map(|r| addons::download_for(r, addons::platform()));
    if let Some(Err(why)) = &ready
        && newer
    {
        ui.label(why);
    }
    ui.horizontal(|ui| {
        let busy = matches!(row.newest, Newest::Working(_) | Newest::Checking);
        let can = can_install && !busy && newer && matches!(ready, Some(Ok(_)));
        let word = if row.installed.is_some() {
            "Update"
        } else {
            "Install"
        };
        if ui
            .add_enabled(can, egui::Button::new(word))
            .on_hover_text("Downloads it from GitHub, checks it and installs it for you alone")
            .clicked()
        {
            press = Some(Press::Install);
        }
        if row.installed.is_some() && ui.add_enabled(!busy, egui::Button::new("Remove")).clicked() {
            press = Some(Press::Remove);
        }
        if ui.button("Release Page").clicked() {
            ui.ctx().open_url(egui::OpenUrl::new_tab(row.source.page));
        }
    });
    press
}

fn check(row: &mut Row, ctx: &egui::Context) {
    let (tx, rx) = channel();
    let (source, ctx) = (row.source, ctx.clone());
    row.newest = Newest::Checking;
    row.rx = Some(rx);
    let spawned = std::thread::Builder::new()
        .name("add-on check".into())
        .spawn(move || {
            let _ = tx.send(Msg::Checked(addons::latest(&source)));
            ctx.request_repaint();
        });
    if let Err(e) = spawned {
        row.newest = Newest::Failed(e.to_string());
    }
}

fn install(row: &mut Row, ctx: &egui::Context, folder: PathBuf) {
    let Newest::Found(release) = row.newest.clone() else {
        return;
    };
    let (tx, rx) = channel();
    let (source, ctx) = (row.source, ctx.clone());
    row.newest = Newest::Working(Step::Downloading { done: 0, total: 0 });
    row.rx = Some(rx);
    let spawned = std::thread::Builder::new()
        .name("add-on install".into())
        .spawn(move || {
            let step = |s| {
                let _ = tx.send(Msg::Step(s));
                ctx.request_repaint();
            };
            let done = addons::install(&source, &release, &folder, &step);
            let _ = tx.send(Msg::Installed(done));
            ctx.request_repaint();
        });
    if let Err(e) = spawned {
        row.newest = Newest::Failed(e.to_string());
    }
}

/// Takes in what the row's thread said; true when it installed something.
fn take(row: &mut Row) -> bool {
    let mut installed = false;
    while let Some(msg) = row.rx.as_ref().and_then(|rx| rx.try_recv().ok()) {
        match msg {
            Msg::Checked(Ok(release)) => row.newest = Newest::Found(release),
            Msg::Checked(Err(e)) => row.newest = Newest::Failed(e),
            Msg::Step(step) => row.newest = Newest::Working(step),
            Msg::Installed(result) => {
                row.rx = None;
                match result {
                    Ok(manifest) => {
                        row.installed = Some(manifest);
                        installed = true;
                        // the release it came from, so the row reads up to date
                        row.newest = Newest::Unknown;
                    }
                    Err(e) => row.newest = Newest::Failed(e),
                }
            }
        }
    }
    installed
}
