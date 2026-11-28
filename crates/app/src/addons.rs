//! Gadgets ▸ Add-ons: the official add-ons with an Install button each, and Install from
//! GitHub for any other, each from its repository's newest release. Installs and updates
//! run on a thread of their own so the window keeps drawing; an add-on that is not signed
//! with PowerWhittle's release key is installed only after a warning. The work is
//! [`cut_craft_updater::addon`]'s.

use cut_craft_i18n::tr;
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
    /// Remove pressed for add-on `id` (called `name`): waiting for Remove or Keep, as it
    /// has to be downloaded again to be used.
    Removing {
        id: String,
        name: String,
        gadget: bool,
    },
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
    /// The languages with a pack to download, by code.
    Languages(Result<Vec<String>, String>),
}

type Find = fn(&Repo, Option<&str>) -> Result<Offer, String>;
type Languages = fn() -> Result<Vec<String>, String>;
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
    /// Asks which languages have a pack to download.
    languages: Languages,
    /// The languages with a pack to download, once asked: each its own install.
    offered: Option<Result<Vec<String>, String>>,
    /// How much of PowerWhittle each installed language pack covers, by id, with the version it
    /// was worked out for: read once, not every frame.
    coverage: std::cell::RefCell<HashMap<String, (String, u32)>>,
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
            languages: addon::languages_offered,
            offered: None,
            coverage: std::cell::RefCell::default(),
            install: |dir, offer, progress| addon::install(dir, offer, &usable, progress),
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
            languages: || Ok(vec![]),
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

    /// Removes an installed add-on. Whether there was a folder to remove from. (Whatever is
    /// put in its place is new code: asked about afresh before it runs, `crate::trust`.)
    fn remove(&mut self, id: &str) -> bool {
        let Some(dir) = &self.dir else { return false };
        let (name, gadget) = self.about(id);
        self.phase = match addon::remove(dir, id) {
            Ok(()) if gadget => Phase::Done(format!(
                "{name} is removed. It leaves the Gadgets menu when PowerWhittle restarts."
            )),
            Ok(()) => Phase::Done(format!("{name} is removed.")),
            Err(e) => Phase::Failed(e),
        };
        self.updates.remove(id);
        true
    }

    /// The installed add-on `id`'s name (its id when its manifest cannot be read), and
    /// whether it is a gadget.
    fn about(&self, id: &str) -> (String, bool) {
        let manifest = self
            .dir
            .as_deref()
            .and_then(|d| std::fs::read_to_string(d.join(id).join(addon::MANIFEST)).ok())
            .and_then(|text| Manifest::parse(&text).ok());
        match manifest {
            Some(m) => (m.name, m.kind == "gadget"),
            None => (id.to_owned(), false),
        }
    }

    /// The installed add-on `id` (called `name`), whose program is `program`, as code about
    /// to run: signed by its install record, else (an unsigned release, or no record: put in
    /// place by hand) asked about first, [`crate::trust`].
    pub fn code(&self, id: &str, name: &str, program: &Path) -> crate::trust::Code {
        let signed = self
            .dir
            .as_deref()
            .and_then(|d| addon::installed(d, id))
            .is_some_and(|r| r.signed);
        crate::trust::Code::add_on(id, name, signed, program)
    }

    /// Opens the window, and looks for newer releases of what is installed.
    pub fn show(&mut self, ctx: &egui::Context) {
        self.open = true;
        if !matches!(self.offered, Some(Ok(_))) {
            let (tx, ctx, languages) = (self.tx.clone(), ctx.clone(), self.languages);
            std::thread::spawn(move || {
                let _ = tx.send(Msg::Languages(languages()));
                ctx.request_repaint();
            });
        }
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
        self.install_named(ctx, repo, None);
    }

    /// Install the zip called `named` from `repo`'s newest release (one of several add-ons it
    /// holds, as language packs are), or the one for this computer with `None`.
    pub fn install_named(&mut self, ctx: &egui::Context, repo: &str, named: Option<&str>) {
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
        let named = named.map(str::to_owned);
        std::thread::spawn(move || {
            let _ = tx.send(Msg::Found(find(&repo, named.as_deref())));
            ctx.request_repaint();
        });
    }

    /// Installs what was found (the user has said yes if it is not signed).
    fn start(&mut self, ctx: &egui::Context, offer: Offer) {
        let Some(dir) = self.dir.clone() else {
            self.phase = Phase::Failed("PowerWhittle has no settings folder to put it in".into());
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
                    self.phase = Phase::Done(installed_words(&m));
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
                Msg::Languages(offered) => self.offered = Some(offered),
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
        changed |= self.press(ctx, pressed);
        if !open {
            self.open = false;
            if !self.busy() && !matches!(self.phase, Phase::Asking(_)) {
                self.phase = Phase::Idle;
            }
        }
        changed
    }

    /// Does what was pressed; whether an add-on was removed.
    fn press(&mut self, ctx: &egui::Context, pressed: Press) -> bool {
        let mut changed = false;
        match pressed {
            Press::Nothing => {}
            Press::Install(repo) => self.install_from(ctx, &repo),
            Press::InstallLanguage(code) => {
                self.install_named(ctx, addon::TRANSLATIONS, Some(&addon::language_zip(&code)));
            }
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
                let (name, gadget) = self.about(&id);
                self.phase = Phase::Removing { id, name, gadget };
            }
            Press::RemoveAnyway => {
                if let Phase::Removing { id, .. } = &self.phase {
                    let id = id.clone();
                    changed = self.remove(&id);
                }
            }
        }
        changed
    }

    fn body(&mut self, ui: &mut egui::Ui) -> Press {
        let mut press = Press::Nothing;
        let busy = self.busy() || matches!(self.phase, Phase::Asking(_) | Phase::Removing { .. });
        let dir = self.dir.clone();
        let installed = |id: &str| dir.as_deref().and_then(|d| addon::installed(d, id));

        ui.heading(tr!("addons-cut-craft-add-ons"));
        ui.label(tr!("addons-extra-tools-that-are-too"));
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

        self.languages_section(ui, dir.as_deref(), busy, &mut press);

        // add-ons installed from elsewhere
        let official =
            |id: &str| OFFICIAL.iter().any(|a| a.id == id) || id.starts_with(LANGUAGE_PREFIX);
        let others: Vec<(String, Record)> = dir
            .as_deref()
            .map_or_default(installed_ids)
            .into_iter()
            .filter(|id| !official(id))
            .filter_map(|id| installed(&id).map(|r| (id, r)))
            .collect();
        if !others.is_empty() {
            ui.add_space(8.0);
            ui.strong(tr!("addons-installed-from-github"));
            for (id, record) in &others {
                ui.group(|ui| {
                    ui.set_width(ui.available_width());
                    ui.label(tr!(
                        "addons-from-repo",
                        id = id.as_str(),
                        repo = record.repo.to_string()
                    ));
                    let repo = record.repo.to_string();
                    self.row(ui, id, &repo, Some(record), busy, &mut press);
                });
            }
        }

        ui.add_space(8.0);
        ui.strong(tr!("addons-install-from-github"));
        ui.label(tr!("addons-a-gadget-or-other-add"));
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.repo_text)
                    .hint_text("owner/name")
                    .desired_width(300.0),
            );
            let ok = !busy && !self.repo_text.trim().is_empty();
            if ui
                .add_enabled(ok, egui::Button::new(tr!("addons-install")))
                .clicked()
            {
                press = Press::Install(self.repo_text.trim().to_owned());
            }
        });

        ui.add_space(8.0);
        if let Some(asked) = self.phase_view(ui) {
            press = asked;
        }
        press
    }

    /// Where an install or remove has got to, with its buttons; what was pressed there.
    fn phase_view(&self, ui: &mut egui::Ui) -> Option<Press> {
        let mut pressed = Press::Nothing;
        let press = &mut pressed;
        match &self.phase {
            Phase::Idle => {}
            Phase::Finding(repo) => {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(tr!("addons-asking-repo", repo = repo.as_str()));
                });
            }
            Phase::Asking(offer) => {
                ui.group(|ui| {
                    ui.strong(tr!("addons-not-official-heading"));
                    ui.label(unsigned_warning(offer));
                    ui.horizontal(|ui| {
                        if ui.button(tr!("addons-install-anyway")).clicked() {
                            *press = Press::InstallAnyway;
                        }
                        if ui.button(tr!("addons-cancel")).clicked() {
                            *press = Press::Cancel;
                        }
                    });
                });
            }
            Phase::Removing { name, .. } => {
                ui.group(|ui| {
                    ui.colored_label(ui.visuals().warn_fg_color, remove_question(name));
                    ui.horizontal(|ui| {
                        if ui.button(tr!("addons-remove")).clicked() {
                            *press = Press::RemoveAnyway;
                        }
                        if ui.button(tr!("addons-keep")).clicked() {
                            *press = Press::Cancel;
                        }
                    });
                });
            }
            Phase::Installing { name, done, total } => {
                ui.label(tr!("addons-downloading", name = name.as_str()));
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
        (pressed != Press::Nothing).then_some(pressed)
    }

    /// Languages: one row for each language with a pack to download, installed or not,
    /// each installed on its own; any installed pack no longer offered is listed too, so it
    /// can still be removed.
    fn languages_section(
        &self,
        ui: &mut egui::Ui,
        dir: Option<&Path>,
        busy: bool,
        press: &mut Press,
    ) {
        let installed = dir.map_or_default(crate::languages::installed);
        let mut codes: Vec<String> = match &self.offered {
            Some(Ok(codes)) => codes.clone(),
            _ => vec![],
        };
        for pack in &installed {
            if !codes.contains(&pack.code) {
                codes.push(pack.code.clone());
            }
        }
        codes.sort();
        ui.add_space(8.0);
        ui.strong(tr!("addons-languages"));
        ui.label(tr!("addons-languages-about"));
        match &self.offered {
            None => {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(tr!("addons-languages-asking"));
                });
            }
            Some(Err(e)) if codes.is_empty() => {
                ui.weak(e);
            }
            _ => {}
        }
        for code in codes {
            let pack = installed.iter().find(|p| p.code == code);
            ui.group(|ui| {
                ui.set_width(ui.available_width());
                ui.strong(pack.map_or_else(|| language_name(&code), |p| p.name.clone()));
                if let Some(pack) = pack {
                    let percent = dir.map(|d| self.coverage_of(d, pack));
                    for line in crate::languages::support_lines(pack, percent) {
                        ui.weak(line);
                    }
                    let had = dir.and_then(|d| addon::installed(d, &pack.id));
                    self.row(ui, &pack.id, addon::TRANSLATIONS, had.as_ref(), busy, press);
                } else {
                    // every pack published with PowerWhittle's translations is signed
                    ui.weak(tr!("language-community-supported"));
                    if ui
                        .add_enabled(!busy, egui::Button::new(tr!("addons-install")))
                        .clicked()
                    {
                        *press = Press::InstallLanguage(code.clone());
                    }
                }
            });
        }
    }

    /// How much of PowerWhittle `pack` (installed in `dir`) covers, worked out once for each
    /// version of it.
    fn coverage_of(&self, dir: &Path, pack: &crate::languages::Pack) -> u32 {
        let mut known = self.coverage.borrow_mut();
        match known.get(&pack.id) {
            Some((version, percent)) if *version == pack.version => *percent,
            _ => {
                let percent = crate::languages::coverage(dir, pack);
                known.insert(pack.id.clone(), (pack.version.clone(), percent));
                percent
            }
        }
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
                    .add_enabled(!busy, egui::Button::new(tr!("addons-install")))
                    .clicked()
                {
                    *press = Press::Install(repo.to_owned());
                }
            }
            Some(record) => {
                ui.label(installed_line(&record.version, record.signed));
                if let Some(offer) = self.updates.get(id) {
                    let words = format!("Update to {}", offer.release.version);
                    if ui.add_enabled(!busy, egui::Button::new(words)).clicked() {
                        *press = Press::Update(id.to_owned());
                    }
                }
                if ui
                    .add_enabled(!busy, egui::Button::new(tr!("addons-remove")))
                    .clicked()
                {
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
    /// A language's pack, by code, from the translations' repository.
    InstallLanguage(String),
    Update(String),
    InstallAnyway,
    Cancel,
    Remove(String),
    /// Remove, said again.
    RemoveAnyway,
}

/// The start of every language pack's add-on id: `language-ru`.
const LANGUAGE_PREFIX: &str = "language-";

/// A language's name in its own words, by code, for a pack not installed yet (an installed
/// pack's own name shows instead); the code itself for a language not listed here.
fn language_name(code: &str) -> String {
    let name = match code {
        "ru" => "Русский",
        "de" => "Deutsch",
        "fr" => "Français",
        "es" => "Español",
        "it" => "Italiano",
        "pt" => "Português",
        "pt-BR" => "Português (Brasil)",
        "nl" => "Nederlands",
        "pl" => "Polski",
        "uk" => "Українська",
        "cs" => "Čeština",
        "sv" => "Svenska",
        "tr" => "Türkçe",
        _ => return code.to_owned(),
    };
    format!("{name} ({code})")
}

/// What Remove asks before add-on `name` goes.
fn remove_question(name: &str) -> String {
    format!("Remove {name}? Its folder is deleted, so you'll need to download it again to use it.")
}

/// The tool add-ons this PowerWhittle has tools for: a tool add-on only works through a Cut
/// Craft tool made for it (Relief from Photo for Depth, Generate Border for Borders), never
/// with its own interface.
pub const TOOLS: &[&str] = &[crate::depth::ADDON_ID, crate::borders::ADDON_ID];

/// Whether this PowerWhittle can use the add-on `m` describes, or why not, said before it is
/// installed.
pub fn usable(m: &Manifest) -> Result<(), String> {
    match m.kind.as_str() {
        "gadget" => Ok(()),
        "language" if m.id.starts_with(LANGUAGE_PREFIX) => Ok(()),
        "language" => Err(format!(
            "{} is a language pack whose id does not start with \"{LANGUAGE_PREFIX}\", so \
             PowerWhittle would not list it with the languages.",
            m.name
        )),
        "tool" if TOOLS.contains(&m.id.as_str()) => Ok(()),
        "tool" => Err(format!(
            "{} is a tool this version of PowerWhittle has no place for. Tool add-ons work \
             through PowerWhittle's own tools, so only those on the official list install.",
            m.name
        )),
        "content" => Err(format!(
            "{} is a content pack, which this version of PowerWhittle cannot use yet. Update \
             PowerWhittle (Help ▸ Check for Updates) and try again.",
            m.name
        )),
        kind => Err(format!(
            "{} is a kind of add-on (\"{kind}\") this version of PowerWhittle does not know. \
             Update PowerWhittle and try again.",
            m.name
        )),
    }
}

/// What to say once `m` is installed.
fn installed_words(m: &Manifest) -> String {
    let more = match m.kind.as_str() {
        "gadget" => " It is in the Gadgets menu the next time PowerWhittle starts.",
        "language" => " Pick it in Edit ▸ Options ▸ Language.",
        _ => "",
    };
    format!("{} {} is installed.{more}", m.name, m.version)
}

/// The tool the installed add-on `id` brings, when PowerWhittle knows it: the Depth add-on's
/// is Relief from Photo, the Borders add-on's Generate Border.
pub fn tool_of(id: &str) -> Option<&'static str> {
    match id {
        crate::depth::ADDON_ID => Some("relief_from_photo"),
        crate::borders::ADDON_ID => Some("generate_border"),
        _ => None,
    }
}

/// The add-ons installed in `dir` that are not gadgets (a gadget lists itself), by name in
/// the order of their ids, each with the tool it brings when PowerWhittle knows it, and whether
/// it is official (signed with PowerWhittle's release key).
pub fn installed_tools(dir: &Path) -> Vec<(String, Option<&'static str>, bool)> {
    installed_ids(dir)
        .into_iter()
        .filter_map(|id| {
            let text = std::fs::read_to_string(dir.join(&id).join(addon::MANIFEST)).ok()?;
            let manifest = Manifest::parse(&text).ok()?;
            let signed = addon::installed(dir, &id).is_some_and(|r| r.signed);
            (!matches!(manifest.kind.as_str(), "gadget" | "language"))
                .then(|| (manifest.name, tool_of(&id), signed))
        })
        .collect()
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

/// An installed add-on's line in the Add-ons window: its version, and whether it is official
/// (signed with PowerWhittle's release key) or not, marked so it is seen.
pub fn installed_line(version: &str, signed: bool) -> String {
    let official = if signed {
        "official"
    } else {
        "\u{26a0} not official (not signed)"
    };
    format!("Version {version} installed, {official}")
}

/// What the user is told before installing an add-on that is not signed. It still installs
/// once they say so.
pub fn unsigned_warning(offer: &Offer) -> String {
    format!(
        "{} {} is not an official PowerWhittle add-on: it is not signed with PowerWhittle's release \
         key, so nobody has checked what it does. {NOT_RESPONSIBLE} Install it only if you \
         trust {}.",
        offer.repo, offer.release.tag, offer.repo.owner
    )
}

/// What every unsigned add-on warning says.
pub const NOT_RESPONSIBLE: &str = "PowerWhittle and its author are not responsible for what an \
    unsigned add-on does or can do, including anything harmful or malicious. A tool add-on \
    is a program that runs on this computer with your permissions.";

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
            named: None,
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
    fn signed_repo(repo: &Repo, _: Option<&str>) -> Result<Offer, String> {
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
    fn only_add_ons_cut_craft_can_use_install() {
        let m = |id: &str, kind: &str| {
            Manifest::parse(&format!(
                "id = \"{id}\"\nname = \"Thing\"\nkind = \"{kind}\"\nversion = \"1.0.0\""
            ))
            .unwrap()
        };
        assert_eq!(usable(&m("depth", "tool")), Ok(()));
        assert_eq!(
            usable(&m("borders", "tool")),
            Ok(()),
            "Generate Border's add-on"
        );
        assert_eq!(tool_of("borders"), Some("generate_border"));
        assert_eq!(usable(&m("anything", "gadget")), Ok(()));
        assert!(
            usable(&m("other", "tool"))
                .unwrap_err()
                .contains("official list")
        );
        assert!(
            usable(&m("pack", "content"))
                .unwrap_err()
                .contains("cannot use yet")
        );
        assert!(
            usable(&m("x", "theme"))
                .unwrap_err()
                .contains("does not know")
        );
        assert!(installed_words(&m("g", "gadget")).contains("Gadgets menu"));
        let ru = Manifest::parse(
            "id = \"language-ru\"\nname = \"Русский\"\nkind = \"language\"\nversion = \
             \"0.1.0\"\nlanguage = \"ru\"",
        )
        .unwrap();
        assert_eq!(usable(&ru), Ok(()));
        assert!(installed_words(&ru).contains("Options ▸ Language"));
    }

    #[test]
    fn remove_asks_first_and_says_a_gadget_leaves_its_menu_at_restart() {
        let dir = std::env::temp_dir().join(format!("cutcraft-addon-rm-{}", std::process::id()));
        let folder = dir.join("snowflake");
        std::fs::create_dir_all(&folder).unwrap();
        std::fs::write(
            folder.join(addon::MANIFEST),
            "id = \"snowflake\"\nname = \"Snowflake\"\nkind = \"gadget\"\nversion = \"1.0.0\"",
        )
        .unwrap();
        let ctx = egui::Context::default();
        let mut a = Addons::with(dir.clone(), signed_repo, installs, nothing_newer);
        assert!(
            !a.press(&ctx, Press::Remove("snowflake".into())),
            "nothing removed yet"
        );
        assert!(folder.exists());
        let Phase::Removing { name, .. } = &a.phase else {
            panic!("asks first: {:?}", a.phase)
        };
        assert_eq!(name, "Snowflake");
        assert!(remove_question(name).contains("download it again"));
        assert!(!a.press(&ctx, Press::Cancel), "Keep keeps it");
        assert!(folder.exists() && a.phase == Phase::Idle);
        a.press(&ctx, Press::Remove("snowflake".into()));
        assert!(a.press(&ctx, Press::RemoveAnyway));
        assert!(!folder.exists());
        let Phase::Done(words) = &a.phase else {
            panic!("{:?}", a.phase)
        };
        assert!(words.contains("Snowflake is removed"), "{words}");
        assert!(words.contains("Gadgets menu"), "{words}");
        std::fs::remove_dir_all(&dir).ok();
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
        assert!(
            words.contains("not an official PowerWhittle add-on"),
            "{words}"
        );
        assert_eq!(
            installed_line("0.1.0", false),
            "Version 0.1.0 installed, \u{26a0} not official (not signed)"
        );
        assert_eq!(
            installed_line("0.2.0", true),
            "Version 0.2.0 installed, official"
        );
        // nothing more happens until the user says yes
        a.poll(&ctx);
        assert!(matches!(a.phase, Phase::Asking(_)));
        a.start(&ctx, *o);
        wait(&mut a, &ctx, |a| matches!(a.phase, Phase::Done(_)));
    }

    #[test]
    fn an_add_on_is_signed_only_by_a_signed_install_record() {
        let dir = std::env::temp_dir().join(format!("cut-craft-run-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let record = |id: &str, signed: bool| {
            std::fs::create_dir_all(dir.join(id)).unwrap();
            let r = Record {
                repo: Repo::parse("o/r").unwrap(),
                tag: "v1.0.0".into(),
                version: "1.0.0".into(),
                signed,
                zip: None,
            };
            std::fs::write(
                dir.join(id).join(addon::RECORD),
                serde_json::to_string(&r).unwrap(),
            )
            .unwrap();
        };
        record("signed", true);
        record("unsigned", false);
        let a = Addons::with(dir.clone(), signed_repo, installs, nothing_newer);
        let program = dir.join("program");
        assert!(a.code("signed", "Signed", &program).signed);
        assert!(!a.code("unsigned", "Unsigned", &program).signed);
        // put in place by hand: no record, so unsigned
        assert!(!a.code("loose", "Loose", &program).signed);
        assert_eq!(a.code("loose", "Loose", &program).key, "addon:loose");
        std::fs::remove_dir_all(dir).unwrap();
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
        let offline = |_: &Repo, _: Option<&str>| -> Result<Offer, String> {
            Err("Could not reach GitHub".into())
        };
        let mut a = Addons::with(PathBuf::from("."), offline, installs, nothing_newer);
        a.install_from(&ctx, "someone/thing");
        wait(&mut a, &ctx, |a| {
            a.phase == Phase::Failed("Could not reach GitHub".into())
        });
    }

    #[test]
    fn each_language_installs_on_its_own_from_the_translations() {
        let ctx = egui::Context::default();
        // the zip asked for comes back in the error, to see which was asked for
        let asked = |repo: &Repo, named: Option<&str>| -> Result<Offer, String> {
            Err(format!("{repo} {}", named.unwrap_or("none")))
        };
        let mut a = Addons::with(PathBuf::from("."), asked, installs, nothing_newer);
        a.languages = || Ok(vec!["de".into(), "ru".into()]);
        a.show(&ctx);
        wait(&mut a, &ctx, |a| a.offered.is_some());
        assert_eq!(a.offered, Some(Ok(vec!["de".into(), "ru".into()])));
        a.press(&ctx, Press::InstallLanguage("ru".into()));
        wait(&mut a, &ctx, |a| matches!(a.phase, Phase::Failed(_)));
        assert_eq!(
            a.phase,
            Phase::Failed(format!("{} language-ru.zip", addon::TRANSLATIONS))
        );
        assert_eq!(language_name("ru"), "Русский (ru)");
        assert_eq!(language_name("xx"), "xx");
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

    #[test]
    fn installed_tool_add_ons_name_the_tool_they_bring() {
        let dir =
            std::env::temp_dir().join(format!("cut-craft-addon-tools-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let add = |id: &str, name: &str, kind: &str, record: Option<bool>| {
            let folder = dir.join(id);
            std::fs::create_dir_all(&folder).unwrap();
            std::fs::write(
                folder.join(addon::MANIFEST),
                format!("id = \"{id}\"\nname = \"{name}\"\nkind = \"{kind}\"\nversion = \"1.0.0\""),
            )
            .unwrap();
            if let Some(signed) = record {
                let record = format!(
                    r#"{{"repo":{{"owner":"o","name":"{id}"}},"tag":"v1.0.0","version":"1.0.0","signed":{signed}}}"#
                );
                std::fs::write(folder.join(addon::RECORD), record).unwrap();
            }
        };
        add("depth", "Depth (Relief from Photo)", "tool", Some(true));
        // gadgets list themselves; a folder without a record is not installed
        add("snowflake", "Snowflake", "gadget", Some(true));
        add("loose", "Loose", "tool", None);
        add("pack", "Chess Pieces", "content", Some(false));
        assert_eq!(
            installed_tools(&dir),
            [
                (
                    "Depth (Relief from Photo)".to_owned(),
                    Some("relief_from_photo"),
                    true
                ),
                ("Chess Pieces".to_owned(), None, false),
            ],
            "each with whether it is official (signed)"
        );
        assert_eq!(installed_tools(&dir.join("none")), []);
        std::fs::remove_dir_all(dir).unwrap();
    }
}
