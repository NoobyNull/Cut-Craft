//! Gadgets ▸ Add-ons: the official add-ons with an Install button each, and Install from
//! GitHub for any other, each from its repository's newest release. Installs and updates
//! run on a thread of their own so the window keeps drawing; an add-on that is not signed
//! with Cut Craft's release key is installed only after a warning. The work is
//! [`cut_craft_updater::addon`]'s.

use cut_craft_updater::addon::{self, Manifest, OFFICIAL, Offer, Record, Repo};
use eframe::egui;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, Sender, channel};

/// Where an install has got to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Phase {
    Idle,
    /// Asking GitHub for this repository's newest release.
    Finding(String),
    /// Found, but not signed: waiting for the user's yes.
    Asking(Box<Offer>),
    /// Downloading: bytes so far of the whole.
    Installing {
        name: String,
        done: u64,
        total: u64,
    },
    /// Installed: what it said.
    Done(String),
    Failed(String),
}

enum Msg {
    Found(Result<Offer, String>),
    Progress(u64, u64),
    Installed(Result<Manifest, String>),
    /// An installed add-on's newest release, if newer.
    Newer(String, Result<Option<Offer>, String>),
}

type Find = fn(&Repo) -> Result<Offer, String>;
type Install = fn(&Path, &Offer, &dyn Fn(u64, u64)) -> Result<Manifest, String>;
type Newer = fn(&Path, &str) -> Result<Option<Offer>, String>;

pub struct Addons {
    pub open: bool,
    pub phase: Phase,
    /// What is typed under Install from GitHub.
    pub repo_text: String,
    /// Where add-ons go; `None` in tests and where there is no settings folder.
    dir: Option<PathBuf>,
    /// Installed add-ons with a newer release, by id.
    pub updates: HashMap<String, Offer>,
    tx: Sender<Msg>,
    rx: Receiver<Msg>,
    find: Find,
    install: Install,
    newer: Newer,
}

impl Addons {
    pub fn new(dir: Option<PathBuf>) -> Self {
        let (tx, rx) = channel();
        Self {
            open: false,
            phase: Phase::Idle,
            repo_text: String::new(),
            dir,
            updates: HashMap::new(),
            tx,
            rx,
            find: addon::find,
            install: addon::install,
            newer: addon::update_for,
        }
    }

    /// The same, asking `find` and installing with `install` instead of GitHub.
    #[cfg(test)]
    pub fn with(dir: PathBuf, find: Find, install: Install, newer: Newer) -> Self {
        Self {
            find,
            install,
            newer,
            ..Self::new(Some(dir))
        }
    }

    pub fn busy(&self) -> bool {
        matches!(self.phase, Phase::Finding(_) | Phase::Installing { .. })
    }

    /// Where add-ons are installed.
    pub fn dir(&self) -> Option<&Path> {
        self.dir.as_deref()
    }

    /// Opens the window, and looks for newer releases of what is installed.
    pub fn show(&mut self, ctx: &egui::Context) {
        self.open = true;
        let Some(dir) = self.dir.clone() else { return };
        for record_id in installed_ids(&dir) {
            let (tx, ctx, newer, dir) = (self.tx.clone(), ctx.clone(), self.newer, dir.clone());
            std::thread::spawn(move || {
                let answer = newer(&dir, &record_id);
                let _ = tx.send(Msg::Newer(record_id, answer));
                ctx.request_repaint();
            });
        }
    }

    /// Install (or update) from `repo`: asks GitHub for its newest release first.
    pub fn install_from(&mut self, ctx: &egui::Context, repo: &str) {
        if self.busy() {
            return;
        }
        let repo = match Repo::parse(repo) {
            Ok(repo) => repo,
            Err(e) => {
                self.phase = Phase::Failed(e);
                return;
            }
        };
        self.phase = Phase::Finding(repo.to_string());
        let (tx, ctx, find) = (self.tx.clone(), ctx.clone(), self.find);
        std::thread::spawn(move || {
            let _ = tx.send(Msg::Found(find(&repo)));
            ctx.request_repaint();
        });
    }

    /// Installs what was found (the user has said yes if it is not signed).
    fn start(&mut self, ctx: &egui::Context, offer: Offer) {
        let Some(dir) = self.dir.clone() else {
            self.phase = Phase::Failed("Cut Craft has no settings folder to put it in".into());
            return;
        };
        self.phase = Phase::Installing {
            name: offer.repo.to_string(),
            done: 0,
            total: offer.zip.size,
        };
        let (tx, ctx, install) = (self.tx.clone(), ctx.clone(), self.install);
        std::thread::spawn(move || {
            let progress = |done, total| {
                let _ = tx.send(Msg::Progress(done, total));
                ctx.request_repaint();
            };
            let done = install(&dir, &offer, &progress);
            let _ = tx.send(Msg::Installed(done));
            ctx.request_repaint();
        });
    }

    /// Takes in what the threads have said; whether an add-on was installed.
    pub fn poll(&mut self, ctx: &egui::Context) -> bool {
        let mut installed = false;
        while let Ok(msg) = self.rx.try_recv() {
            match msg {
                Msg::Found(Err(e)) => self.phase = Phase::Failed(e),
                Msg::Found(Ok(offer)) if offer.signed => self.start(ctx, offer),
                Msg::Found(Ok(offer)) => {
                    self.phase = Phase::Asking(Box::new(offer));
                    self.open = true;
                }
                Msg::Progress(d, t) => {
                    if let Phase::Installing { done, total, .. } = &mut self.phase {
                        (*done, *total) = (d, t);
                    }
                }
                Msg::Installed(Ok(m)) => {
                    self.updates.remove(&m.id);
                    self.phase = Phase::Done(format!("{} {} is installed.", m.name, m.version));
                    installed = true;
                }
                Msg::Installed(Err(e)) => {
                    log::warn!("an add-on did not install: {e}");
                    self.phase = Phase::Failed(e);
                }
                Msg::Newer(id, Ok(Some(offer))) => {
                    self.updates.insert(id, offer);
                }
                Msg::Newer(_, Ok(None)) => {}
                Msg::Newer(id, Err(e)) => log::info!("no update check for add-on {id}: {e}"),
            }
        }
        installed
    }

    /// The window, when open; whether an add-on was installed or removed.
    pub fn window(&mut self, ctx: &egui::Context) -> bool {
        let mut changed = self.poll(ctx);
        if !self.open {
            return changed;
        }
        let mut open = true;
        let id = egui::Id::unique("addons_window");
        let pressed = cut_craft_bug_report::dialog(
            ctx,
            id,
            "Add-ons",
            egui::vec2(520.0, 420.0),
            &mut open,
            |ui| self.body(ui),
        )
        .unwrap_or(Press::Nothing);
        match pressed {
            Press::Nothing => {}
            Press::Install(repo) => self.install_from(ctx, &repo),
            Press::Update(id) => {
                if let Some(offer) = self.updates.get(&id).cloned() {
                    if offer.signed {
                        self.start(ctx, offer);
                    } else {
                        self.phase = Phase::Asking(Box::new(offer));
                    }
                }
            }
            Press::InstallAnyway => {
                if let Phase::Asking(offer) = &self.phase {
                    let offer = (**offer).clone();
                    self.start(ctx, offer);
                }
            }
            Press::Cancel => self.phase = Phase::Idle,
            Press::Remove(id) => {
                if let Some(dir) = &self.dir {
                    self.phase = match addon::remove(dir, &id) {
                        Ok(()) => Phase::Done("The add-on is removed.".into()),
                        Err(e) => Phase::Failed(e),
                    };
                    self.updates.remove(&id);
                    changed = true;
                }
            }
        }
        if !open {
            self.open = false;
            if !self.busy() && !matches!(self.phase, Phase::Asking(_)) {
                self.phase = Phase::Idle;
            }
        }
        changed
    }

    fn body(&mut self, ui: &mut egui::Ui) -> Press {
        let mut press = Press::Nothing;
        let busy = self.busy() || matches!(self.phase, Phase::Asking(_));
        let dir = self.dir.clone();
        let installed = |id: &str| dir.as_deref().and_then(|d| addon::installed(d, id));

        ui.heading("Cut Craft add-ons");
        ui.label(
            "Extra tools that are too big to come with Cut Craft. Each shows up as an \
                  ordinary tool once it is installed.",
        );
        ui.add_space(4.0);
        for a in OFFICIAL {
            ui.group(|ui| {
                ui.set_width(ui.available_width());
                ui.strong(a.name);
                ui.label(a.about);
                let had = installed(a.id);
                self.row(ui, a.id, a.repo, had.as_ref(), busy, &mut press);
            });
        }

        // add-ons installed from elsewhere
        let official = |id: &str| OFFICIAL.iter().any(|a| a.id == id);
        let others: Vec<(String, Record)> = dir
            .as_deref()
            .map(installed_ids)
            .unwrap_or_default()
            .into_iter()
            .filter(|id| !official(id))
            .filter_map(|id| installed(&id).map(|r| (id, r)))
            .collect();
        if !others.is_empty() {
            ui.add_space(8.0);
            ui.strong("Installed from GitHub");
            for (id, record) in &others {
                ui.group(|ui| {
                    ui.set_width(ui.available_width());
                    ui.label(format!("{id}, from {}", record.repo));
                    let repo = record.repo.to_string();
                    self.row(ui, id, &repo, Some(record), busy, &mut press);
                });
            }
        }

        ui.add_space(8.0);
        ui.strong("Install from GitHub");
        ui.label(
            "Any add-on published on GitHub: type its repository, such as \
                  owner/name, or paste its link. Cut Craft installs its newest release.",
        );
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.repo_text)
                    .hint_text("owner/name")
                    .desired_width(300.0),
            );
            let ok = !busy && !self.repo_text.trim().is_empty();
            if ui.add_enabled(ok, egui::Button::new("Install")).clicked() {
                press = Press::Install(self.repo_text.trim().to_owned());
            }
        });

        ui.add_space(8.0);
        match &self.phase {
            Phase::Idle => {}
            Phase::Finding(repo) => {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(format!("Asking GitHub for {repo}'s newest release..."));
                });
            }
            Phase::Asking(offer) => {
                ui.group(|ui| {
                    ui.strong("Not signed");
                    ui.label(unsigned_warning(offer));
                    ui.horizontal(|ui| {
                        if ui.button("Install Anyway").clicked() {
                            press = Press::InstallAnyway;
                        }
                        if ui.button("Cancel").clicked() {
                            press = Press::Cancel;
                        }
                    });
                });
            }
            Phase::Installing { name, done, total } => {
                ui.label(format!("Downloading {name}..."));
                #[expect(clippy::cast_precision_loss, reason = "a fraction for the bar")]
                let fraction = if *total > 0 {
                    *done as f32 / *total as f32
                } else {
                    0.0
                };
                ui.add(egui::ProgressBar::new(fraction.min(1.0)).show_percentage());
            }
            Phase::Done(words) => {
                ui.label(words);
            }
            Phase::Failed(e) => {
                ui.colored_label(ui.visuals().error_fg_color, e);
            }
        }
        press
    }

    /// An add-on's state and its buttons.
    fn row(
        &self,
        ui: &mut egui::Ui,
        id: &str,
        repo: &str,
        had: Option<&Record>,
        busy: bool,
        press: &mut Press,
    ) {
        ui.horizontal(|ui| match had {
            None => {
                if ui
                    .add_enabled(!busy, egui::Button::new("Install"))
                    .clicked()
                {
                    *press = Press::Install(repo.to_owned());
                }
            }
            Some(record) => {
                let signed = if record.signed { "" } else { ", not signed" };
                ui.label(format!("Version {} installed{signed}", record.version));
                if let Some(offer) = self.updates.get(id) {
                    let words = format!("Update to {}", offer.release.version);
                    if ui.add_enabled(!busy, egui::Button::new(words)).clicked() {
                        *press = Press::Update(id.to_owned());
                    }
                }
                if ui.add_enabled(!busy, egui::Button::new("Remove")).clicked() {
                    *press = Press::Remove(id.to_owned());
                }
            }
        });
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Press {
    Nothing,
    Install(String),
    Update(String),
    InstallAnyway,
    Cancel,
    Remove(String),
}

/// The ids of the add-ons installed in `dir` (folders with an install record).
fn installed_ids(dir: &Path) -> Vec<String> {
    let mut ids: Vec<String> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.path().join(addon::RECORD).is_file())
        .filter_map(|e| e.file_name().into_string().ok())
        .filter(|id| !id.starts_with('.'))
        .collect();
    ids.sort();
    ids
}

/// What the user is told before installing an add-on that is not signed.
pub fn unsigned_warning(offer: &Offer) -> String {
    format!(
        "{} {} is not signed with Cut Craft's release key, so nobody has checked what it \
         does. Cut Craft and its author are not responsible for what this add-on does. \
         Install it only if you trust {}.",
        offer.repo, offer.release.tag, offer.repo.owner
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_updater::{Asset, Release, Version};

    fn offer(repo: &Repo, signed: bool) -> Offer {
        Offer {
            repo: repo.clone(),
            release: Release {
                tag: "v1.2.0".into(),
                version: Version::new(1, 2, 0),
                notes: String::new(),
                page: String::new(),
                assets: vec![],
            },
            zip: Asset {
                name: "x.zip".into(),
                url: String::new(),
                size: 10,
                digest: None,
            },
            signed,
        }
    }

    fn manifest(id: &str) -> Manifest {
        Manifest::parse(&format!(
            "id = \"{id}\"\nname = \"Thing\"\nkind = \"tool\"\nversion = \"1.2.0\""
        ))
        .unwrap()
    }

    /// Polls until `done` says so, or gives up after a few seconds.
    fn wait(a: &mut Addons, ctx: &egui::Context, done: impl Fn(&Addons) -> bool) {
        for _ in 0..500 {
            a.poll(ctx);
            if done(a) {
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        panic!("stuck at {:?}", a.phase);
    }

    #[expect(clippy::unnecessary_wraps, reason = "stands in for asking GitHub")]
    fn signed_repo(repo: &Repo) -> Result<Offer, String> {
        Ok(offer(repo, !repo.name.contains("unsigned")))
    }

    #[expect(clippy::unnecessary_wraps, reason = "stands in for installing")]
    fn installs(_: &Path, o: &Offer, progress: &dyn Fn(u64, u64)) -> Result<Manifest, String> {
        progress(5, 10);
        Ok(manifest(&o.repo.name.to_lowercase()))
    }

    #[expect(clippy::unnecessary_wraps, reason = "stands in for asking GitHub")]
    fn nothing_newer(_: &Path, _: &str) -> Result<Option<Offer>, String> {
        Ok(None)
    }

    #[test]
    fn a_signed_add_on_installs_straight_away() {
        let ctx = egui::Context::default();
        let mut a = Addons::with(PathBuf::from("."), signed_repo, installs, nothing_newer);
        a.install_from(&ctx, "https://github.com/someone/Thing");
        assert_eq!(a.phase, Phase::Finding("someone/Thing".into()));
        wait(&mut a, &ctx, |a| matches!(a.phase, Phase::Done(_)));
        assert_eq!(a.phase, Phase::Done("Thing 1.2.0 is installed.".into()));
    }

    #[test]
    fn an_unsigned_add_on_waits_for_a_yes() {
        let ctx = egui::Context::default();
        let mut a = Addons::with(PathBuf::from("."), signed_repo, installs, nothing_newer);
        a.install_from(&ctx, "someone/unsigned");
        wait(&mut a, &ctx, |a| matches!(a.phase, Phase::Asking(_)));
        let Phase::Asking(o) = a.phase.clone() else {
            unreachable!()
        };
        let words = unsigned_warning(&o);
        assert!(
            words.contains("not responsible") && words.contains("someone"),
            "{words}"
        );
        // nothing more happens until the user says yes
        a.poll(&ctx);
        assert!(matches!(a.phase, Phase::Asking(_)));
        a.start(&ctx, *o);
        wait(&mut a, &ctx, |a| matches!(a.phase, Phase::Done(_)));
    }

    #[test]
    fn a_mistyped_repository_says_how_to_type_it() {
        let ctx = egui::Context::default();
        let mut a = Addons::with(PathBuf::from("."), signed_repo, installs, nothing_newer);
        a.install_from(&ctx, "just-a-name");
        let Phase::Failed(e) = &a.phase else {
            panic!("{:?}", a.phase)
        };
        assert!(e.contains("owner/name"), "{e}");
    }

    #[test]
    fn what_goes_wrong_is_shown() {
        let ctx = egui::Context::default();
        let refused = |_: &Path, _: &Offer, _: &dyn Fn(u64, u64)| -> Result<Manifest, String> {
            Err("the checksum does not match".into())
        };
        let mut a = Addons::with(PathBuf::from("."), signed_repo, refused, nothing_newer);
        a.install_from(&ctx, "someone/thing");
        wait(&mut a, &ctx, |a| matches!(a.phase, Phase::Failed(_)));
        let offline = |_: &Repo| -> Result<Offer, String> { Err("Could not reach GitHub".into()) };
        let mut a = Addons::with(PathBuf::from("."), offline, installs, nothing_newer);
        a.install_from(&ctx, "someone/thing");
        wait(&mut a, &ctx, |a| {
            a.phase == Phase::Failed("Could not reach GitHub".into())
        });
    }

    #[test]
    fn opening_the_window_finds_updates_for_what_is_installed() {
        let dir = std::env::temp_dir().join(format!("cut-craft-addons-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("depth")).unwrap();
        std::fs::write(dir.join("depth").join(addon::RECORD), "{}").unwrap();
        // a folder without a record is not an add-on
        std::fs::create_dir_all(dir.join("loose")).unwrap();
        assert_eq!(installed_ids(&dir), ["depth"]);
        let newer = |_: &Path, id: &str| -> Result<Option<Offer>, String> {
            Ok((id == "depth")
                .then(|| offer(&Repo::parse("NoobyNull/CutCraft-Depth").unwrap(), true)))
        };
        let ctx = egui::Context::default();
        let mut a = Addons::with(dir.clone(), signed_repo, installs, newer);
        a.show(&ctx);
        wait(&mut a, &ctx, |a| a.updates.contains_key("depth"));
        std::fs::remove_dir_all(dir).unwrap();
    }
}
