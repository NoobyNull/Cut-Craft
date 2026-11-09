//! The Library tab: folders of the user's own clipart as a tree, and what is in the one
//! picked (and the folders in it; in all of them when none is) as tiles: pictures and 3D
//! models as previews, drawings by their kind. Tiles are picked with a click (Ctrl or
//! Shift for more), added to the project with a double-click or by dragging, and a
//! right-click menu adds, locates, tags or deletes the ones picked, or shows one's Properties.

mod preview;
mod properties;
mod tags;

use cut_craft_bug_report::hint::HoverHint;
use eframe::egui;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::{Duration, Instant};

/// A thumbnail's edge, in rows of the theme's interactive height.
// ponytail: one size; Aspire's three view sizes can come with a setting
const THUMBNAIL_ROWS: f32 = 4.0;
/// Pictures drawn as themselves (at the tile's size); the rest ([`BITMAPS`]) are shrunk
/// once and kept, as a model's preview is, so a folder of big photos doesn't fill the
/// memory, nor a huge scan go past the graphics card's largest texture (which stops the app).
const DRAWN: [&str; 1] = ["svg"];
/// Pictures made of pixels (a depth map is one of these).
const BITMAPS: [&str; 7] = ["png", "jpg", "jpeg", "gif", "bmp", "tif", "tiff"];
/// Vectric's clipart, which is locked to Vectric's own software: counted, not shown.
const LOCKED: [&str; 2] = ["3dclip", "v3m"];
/// How often the folders are read again, for files added, moved or taken away meanwhile.
const RELIST: Duration = Duration::from_secs(5);
/// At most this many files are listed, so a folder as big as a whole drive can't swamp it.
const MOST: usize = 100_000;
/// The folder tree's height at most, in rows of the theme's interactive height.
const TREE_ROWS: f32 = 10.0;

/// The extensions of files something imports, in lower case.
pub type Importable = HashSet<String>;
/// Whether each picture is greyscale (a depth map), by the [`preview::stamp`] it had.
type Greys = HashMap<PathBuf, (u64, bool)>;
/// Folders read in the background: which, what is in them, and the greys learnt.
type Read = (Vec<PathBuf>, Listing, Greys);

/// The library's folders, kept between runs, and the one being looked in.
#[derive(Default)]
pub struct Clipart {
    /// The folders added to the library: the tree's roots.
    pub folders: Vec<PathBuf>,
    /// The folder looked in, with the folders in it; `None`: all of them.
    pub open: Option<PathBuf>,
    /// Where the folders are kept; `None` keeps them only for this run.
    store: Option<PathBuf>,
    /// Models' and pictures' previews, made in the background.
    previews: preview::Previews,
    /// Everything in the folders as last read: which folders, when, and what.
    listing: Option<(Vec<PathBuf>, Instant, Listing)>,
    /// The folders being read in the background.
    reading: Option<mpsc::Receiver<Read>>,
    /// Read the folders again as soon as they can be (a file was deleted).
    again: bool,
    /// Only files whose names or tags have this in them are shown.
    pub search: String,
    /// Only files of this kind are shown.
    pub show: Show,
    /// The files picked, in the order they were.
    pub selected: Vec<PathBuf>,
    /// The file a Shift+click picks from.
    anchor: Option<PathBuf>,
    tags: tags::Tags,
    /// Add Tag: the files being tagged, and the tag as typed so far.
    tagging: Option<(Vec<PathBuf>, String)>,
    /// Kept between readings, so each picture's colours are read once.
    greys: Greys,
    /// The last thing that went wrong, until the next action.
    trouble: Option<String>,
    /// Properties: the file shown in the window, while it is open.
    properties: Option<properties::Properties>,
    /// How many tiles were drawn last.
    #[cfg(test)]
    drawn: usize,
}

/// What a file is, for the Show filter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// STL, OBJ, Cut Craft clipart
    Model,
    /// SVG, DXF and other vector files
    Drawing,
    Picture,
    /// A greyscale picture, or one tagged "depth map": heights as shades
    DepthMap,
}

/// Which files the tab shows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Show {
    #[default]
    Everything,
    Only(Kind),
}

impl Show {
    const ALL: [Self; 5] = [
        Self::Everything,
        Self::Only(Kind::Model),
        Self::Only(Kind::Drawing),
        Self::Only(Kind::Picture),
        Self::Only(Kind::DepthMap),
    ];

    fn label(self) -> &'static str {
        match self {
            Self::Everything => "Everything",
            Self::Only(Kind::Model) => "3D models",
            Self::Only(Kind::Drawing) => "2D drawings",
            Self::Only(Kind::Picture) => "Pictures",
            Self::Only(Kind::DepthMap) => "Depth maps",
        }
    }
}

/// What is in a folder, or in folders and the folders in them.
#[derive(Debug, Default, PartialEq)]
struct Listing {
    entries: Vec<Entry>,
    /// Vectric clipart files, which can't be opened.
    locked: Vec<PathBuf>,
    /// Each folder with something in it, and how many files are in it and the folders in it.
    counts: BTreeMap<PathBuf, usize>,
    /// There were more than [`MOST`] files: the rest aren't listed.
    full: bool,
}

/// A file the tab shows.
#[derive(Debug, PartialEq)]
struct Entry {
    file: PathBuf,
    /// A picture of a model, beside it with the same name (`mydesign.png` for
    /// `mydesign.stl`): shown as its preview, and not as a tile of its own.
    picture: Option<PathBuf>,
    /// Its preview's [`preview::stamp`] as the folder was read: when it changes, the
    /// preview is made again.
    stamp: u64,
    /// What it is, by its extension and (a picture) its colours; a tag can make a picture a
    /// depth map too.
    kind: Kind,
}

/// What the tab asks the app to do.
pub enum Picked {
    AddFolder,
    /// Show this folder in the desktop's file manager.
    ShowFolder(PathBuf),
    /// A double-click: this file into the project.
    Import(PathBuf),
    /// Add to Project: these files, one after another.
    AddToProject(Vec<PathBuf>),
    /// Add to Level: these files onto one of the job's levels, or new ones.
    AddToLevel(Vec<PathBuf>, ToLevel),
    /// Locate: show this file in the desktop's file manager.
    Locate(PathBuf),
    /// Delete from Disk: these files (models with their pictures), once confirmed.
    Delete(Vec<PathBuf>),
}

/// Which level Add to Level puts clipart on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToLevel {
    /// One of the job's levels, by id
    Existing(u64),
    /// A new level, named after the clipart
    New,
}

/// `change` with the components it adds or remakes on `level`.
pub fn onto_level(mut change: cut_craft_core::Change, level: u64) -> cut_craft_core::Change {
    use cut_craft_core::Edit;
    for edit in &mut change.edits {
        if let Edit::AddComponent { component, .. }
        | Edit::ReplaceComponent {
            after: component, ..
        } = edit
        {
            component.level = level;
        }
    }
    change
}

impl Clipart {
    /// The library kept in the config folder.
    pub fn load() -> Self {
        let cache = crate::files::clipart_previews_path();
        if let Some(cache) = cache.clone() {
            let _ = std::thread::Builder::new()
                .name("clipart preview pruning".into())
                .spawn(move || preview::prune(&cache, preview::UNUSED));
        }
        Self {
            previews: preview::Previews::new(cache),
            tags: tags::Tags::load(crate::files::library_tags_path()),
            ..Self::kept(crate::files::clipart_folders_path())
        }
    }

    /// The folders kept at `store`, looking in the folder looked in last (all of them, if
    /// none was or it is no longer in the library).
    fn kept(store: Option<PathBuf>) -> Self {
        let loaded: Vec<PathBuf> = store
            .as_deref()
            .map(crate::files::load_recent)
            .unwrap_or_default();
        let folders = outermost(&loaded);
        if folders != loaded
            && let Some(store) = &store
        {
            // only tidying: kept as it was, it is tidied again next time
            let _ = crate::files::save_recent(&folders, store);
        }
        let open = store
            .as_deref()
            .and_then(|s| std::fs::read(open_path(s)).ok())
            .and_then(|bytes| serde_json::from_slice::<Option<PathBuf>>(&bytes).ok())
            .flatten()
            .filter(|open| folders.iter().any(|f| open.starts_with(f)));
        Self {
            folders,
            open,
            store,
            ..Self::default()
        }
    }

    /// Looks in `folder` and the folders in it (`None`: in all of them), and remembers it
    /// for next time.
    pub fn choose(&mut self, folder: Option<PathBuf>) {
        if self.open != folder {
            self.previews.forget_waiting();
            self.selected.clear();
            self.anchor = None;
        }
        self.open = folder;
        if let Some(store) = &self.store
            && let Ok(bytes) = serde_json::to_vec(&self.open)
        {
            // only a convenience: not remembered, all the folders are shown next time
            let _ = cut_craft_core::file::write_atomic(&open_path(store), &bytes);
        }
    }

    /// Adds `folder` and looks in it. One inside a folder already added is only looked in
    /// (it is in the tree already); the folders inside one added are in it, so they go.
    pub fn add(&mut self, folder: PathBuf) -> Result<(), String> {
        let saved = if self.folders.iter().any(|f| folder.starts_with(f)) {
            Ok(())
        } else {
            self.folders.push(folder.clone());
            self.folders = outermost(&self.folders);
            self.save()
        };
        self.choose(Some(folder));
        saved
    }

    /// Takes `folder` out of the library (the folder itself stays on disk).
    pub fn remove(&mut self, folder: &Path) -> Result<(), String> {
        self.folders.retain(|f| f != folder);
        if self.open.as_ref().is_some_and(|o| o.starts_with(folder)) {
            self.choose(None);
        }
        self.save()
    }

    fn save(&self) -> Result<(), String> {
        match &self.store {
            Some(path) => crate::files::save_recent(&self.folders, path),
            None => Ok(()),
        }
    }

    /// Takes in the folders read in the background, and starts reading them again once
    /// they've changed, a file was deleted, or it's been [`RELIST`] since.
    fn refresh(&mut self, ctx: &egui::Context, importable: &Importable) {
        if let Some(reading) = &self.reading {
            match reading.try_recv() {
                Ok(read) => self.read(read),
                Err(mpsc::TryRecvError::Disconnected) => self.reading = None,
                Err(mpsc::TryRecvError::Empty) => {}
            }
        }
        let stale = self.again
            || (self.listing.as_ref())
                .is_none_or(|(roots, at, _)| *roots != self.folders || at.elapsed() > RELIST);
        if stale && self.reading.is_none() {
            self.again = false;
            let roots = self.folders.clone();
            let importable = importable.clone();
            let mut greys = std::mem::take(&mut self.greys);
            let (send, receive) = mpsc::channel();
            let ctx = ctx.clone();
            let spawned = std::thread::Builder::new()
                .name("library reading".into())
                .spawn(move || {
                    let listing = read_all(&roots, &importable, &mut greys);
                    let _ = send.send((roots, listing, greys));
                    ctx.request_repaint();
                });
            if spawned.is_ok() {
                self.reading = Some(receive);
            }
        }
        ctx.request_repaint_after(RELIST);
    }

    /// Folders read: shown from now on.
    fn read(&mut self, (roots, listing, greys): Read) {
        self.reading = None;
        self.greys = greys;
        // files gone since (deleted or moved elsewhere) are no longer picked
        let there: HashSet<&Path> = (listing.entries.iter()).map(|e| e.file.as_path()).collect();
        self.selected.retain(|f| there.contains(f.as_path()));
        self.listing = Some((roots, Instant::now(), listing));
    }

    /// The folders read now, not in the background.
    #[cfg(test)]
    pub(crate) fn read_now(&mut self, importable: &Importable) {
        let roots = self.folders.clone();
        let mut greys = std::mem::take(&mut self.greys);
        let listing = read_all(&roots, importable, &mut greys);
        self.read((roots, listing, greys));
    }

    /// `file` clicked: picked alone, or (`toggle`, Ctrl) in or out of what is picked, or
    /// (`range`, Shift) along with everything between it and the last one clicked, in the
    /// order `shown`.
    fn click(&mut self, file: &Path, toggle: bool, range: bool, shown: &[&Path]) {
        let at = |f: &Path| shown.iter().position(|s| *s == f);
        let from = self.anchor.as_deref().and_then(at);
        match (range, from, at(file)) {
            (true, Some(from), Some(to)) => {
                if !toggle {
                    self.selected.clear();
                }
                for f in &shown[from.min(to)..=from.max(to)] {
                    if !self.selected.iter().any(|s| s == f) {
                        self.selected.push(f.to_path_buf());
                    }
                }
                return; // the anchor stays, for another Shift+click
            }
            _ if toggle => {
                if let Some(i) = self.selected.iter().position(|s| s == file) {
                    self.selected.remove(i);
                } else {
                    self.selected.push(file.to_owned());
                }
            }
            _ => self.selected = vec![file.to_owned()],
        }
        self.anchor = Some(file.to_owned());
    }

    /// Deletes `files` from disk, for good, and forgets them (picked, tagged, previewed).
    /// Stops at the first that can't be deleted, saying which and why; the ones before it
    /// are gone.
    pub fn delete(&mut self, files: &[PathBuf]) -> Result<(), String> {
        let mut gone = vec![];
        let result = files
            .iter()
            .try_for_each(|f| match std::fs::remove_file(f) {
                Ok(()) => {
                    gone.push(f.clone());
                    Ok(())
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
                Err(e) => Err(format!("Couldn't delete {}: {e}", name(f))),
            });
        self.selected.retain(|s| !files.contains(s));
        // gone from the tab at once; the folders are read again for the counts
        if let Some((_, _, listing)) = &mut self.listing {
            listing.entries.retain(|e| !gone.contains(&e.file));
        }
        self.again = true;
        let forgotten = self.tags.forget(files);
        result.and(forgotten)
    }

    /// The tab's kind of `entry`, its tags counted.
    fn kind(&self, entry: &Entry) -> Kind {
        if entry.kind == Kind::Picture && self.tags.is_depth_map(&entry.file) {
            Kind::DepthMap
        } else {
            entry.kind
        }
    }
}

/// `folders` without any inside another of them (or the same again): each is in the
/// library once, as part of the outermost, which keeps its place.
fn outermost(folders: &[PathBuf]) -> Vec<PathBuf> {
    let inside = |i: usize, folder: &PathBuf| {
        (folders.iter().enumerate())
            .any(|(j, other)| j != i && folder.starts_with(other) && (folder != other || j < i))
    };
    (folders.iter().enumerate())
        .filter(|&(i, f)| !inside(i, f))
        .map(|(_, f)| f.clone())
        .collect()
}

/// Where the folder looked in last is kept, beside the folders kept at `store`.
fn open_path(store: &Path) -> PathBuf {
    store.with_file_name("library-open.json")
}

/// Whether the picture at `path` is stored in shades of grey (read from its header only).
fn is_grey(path: &Path) -> bool {
    use image::ImageDecoder as _;
    image::ImageReader::open(path)
        .ok()
        .and_then(|r| r.with_guessed_format().ok())
        .and_then(|r| r.into_decoder().ok())
        .is_some_and(|d| {
            use image::ColorType::{L8, L16, La8, La16};
            matches!(d.color_type(), L8 | La8 | L16 | La16)
        })
}

/// Everything in `roots` and the folders in them, however deep. Hidden folders are passed
/// over, and so are links to folders, which can lead round in a circle; a folder added
/// inside another one added is read once, as part of it.
fn read_all(roots: &[PathBuf], importable: &Importable, greys: &mut Greys) -> Listing {
    let mut all = Listing::default();
    for root in &outermost(roots) {
        let mut waiting = vec![root.clone()];
        while let Some(folder) = waiting.pop() {
            if all.entries.len() >= MOST {
                all.full = true;
                break;
            }
            let mut read = list(&folder, importable, greys);
            for entry in &read.entries {
                let up = entry.file.ancestors().skip(1);
                for folder in up.take_while(|a| a.starts_with(root)) {
                    *all.counts.entry(folder.to_path_buf()).or_default() += 1;
                }
            }
            all.entries.append(&mut read.entries);
            all.locked.append(&mut read.locked);
            waiting.extend(subfolders(&folder));
        }
    }
    all.entries.sort_by(|a, b| a.file.cmp(&b.file));
    all
}

/// The folders in `folder`, but not hidden ones or links to folders.
fn subfolders(folder: &Path) -> Vec<PathBuf> {
    let Ok(read) = std::fs::read_dir(folder) else {
        return vec![];
    };
    read.flatten()
        // the entry's own type: a link to a folder isn't one
        .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
        .filter(|e| !e.file_name().to_string_lossy().starts_with('.'))
        .map(|e| e.path())
        .collect()
}

/// The files in `folder` (not the folders in it) that something imports, by name; a
/// model's picture goes with it. `greys` remembers which pictures are greyscale, so each
/// is read once.
fn list(folder: &Path, importable: &Importable, greys: &mut Greys) -> Listing {
    let Ok(read) = std::fs::read_dir(folder) else {
        return Listing::default();
    };
    let mut locked = vec![];
    let mut files: Vec<PathBuf> = read
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_file())
        .filter(|p| {
            let extension = preview::extension(p);
            if LOCKED.contains(&extension.as_str()) {
                locked.push(p.clone());
            }
            importable.contains(&extension)
        })
        .collect();
    files.sort();
    locked.sort();
    let stem = |p: &Path| p.file_stem().map(|s| s.to_string_lossy().to_lowercase());
    let is = |p: &Path, kinds: &[&str]| kinds.contains(&preview::extension(p).as_str());
    let pictures: HashMap<String, &PathBuf> = (files.iter())
        .filter(|p| is(p, &preview::PICTURES))
        .filter_map(|p| Some((stem(p)?, p)))
        .collect();
    let picture_of = |p: &Path| {
        is(p, &preview::MODELS)
            .then(|| pictures.get(&stem(p)?).copied())
            .flatten()
    };
    let taken: HashSet<&PathBuf> = files.iter().filter_map(|p| picture_of(p)).collect();
    let entries = (files.iter())
        .filter(|p| !taken.contains(p))
        .map(|file| {
            let picture = picture_of(file).cloned();
            let stamp = preview::stamp(picture.as_deref().unwrap_or(file)).unwrap_or(0);
            let kind = if is(file, &preview::MODELS) {
                Kind::Model
            } else if is(file, &BITMAPS) {
                let grey = match greys.get(file) {
                    Some(&(had, grey)) if had == stamp => grey,
                    _ => {
                        let grey = is_grey(file);
                        greys.insert(file.clone(), (stamp, grey));
                        grey
                    }
                };
                if grey { Kind::DepthMap } else { Kind::Picture }
            } else {
                Kind::Drawing
            };
            Entry {
                stamp,
                file: file.clone(),
                picture,
                kind,
            }
        })
        .collect();
    Listing {
        entries,
        locked,
        ..Listing::default()
    }
}

fn name(path: &Path) -> String {
    path.file_name()
        .map_or_else(String::new, |n| n.to_string_lossy().into_owned())
}

/// Draws the tab; what was asked for, if anything. `levels` are the job's, by id and name,
/// for the Add to Level menu. A tile dragged out carries its file, for the views to import
/// where it is dropped.
pub fn panel(
    ui: &mut egui::Ui,
    clipart: &mut Clipart,
    importable: &Importable,
    levels: &[(u64, String)],
) -> Option<Picked> {
    clipart.previews.update(ui.ctx());
    clipart.refresh(ui.ctx(), importable);
    #[cfg(test)]
    {
        clipart.drawn = 0;
    }
    // taken out while drawn, so the tiles can change the rest of the tab
    let taken = clipart.listing.take();
    let none = Listing::default();
    let listing = taken.as_ref().map_or(&none, |(_, _, l)| l);
    let mut picked = folders(ui, clipart, listing);
    if !clipart.folders.is_empty() {
        let reading = taken.is_none();
        picked = files(ui, clipart, listing, reading, levels).or(picked);
    }
    if clipart.listing.is_none() {
        clipart.listing = taken;
    }
    match properties::window(ui.ctx(), &mut clipart.properties) {
        Some(properties::Asked::Locate(file)) => Some(Picked::Locate(file)),
        None => picked,
    }
}

/// What the folder tree asks for.
#[derive(Default)]
struct Tree {
    /// Look in this folder.
    choose: Option<PathBuf>,
    /// Look in all the folders.
    all: bool,
    /// Take this folder out of the library.
    remove: Option<PathBuf>,
    /// Show this folder in the file manager.
    show: Option<PathBuf>,
}

/// The library's folders as a tree, to look in one (and the folders in it) or in all of
/// them; with Add, Remove and Open Folder.
fn folders(ui: &mut egui::Ui, clipart: &mut Clipart, listing: &Listing) -> Option<Picked> {
    let mut picked = None;
    ui.strong("Folders");
    let mut tree = Tree::default();
    if clipart.folders.is_empty() {
        ui.weak("No folders yet. Add a folder of your models and pictures to see them here.");
    } else {
        let most = ui.spacing().interact_size.y * TREE_ROWS;
        egui::ScrollArea::vertical()
            .id_salt("library_tree")
            .max_height(most)
            .auto_shrink([false, true])
            .show(ui, |ui| {
                let all = format!("All folders ({})", listing.entries.len());
                if ui
                    .selectable_label(clipart.open.is_none(), all)
                    .on_hover_hint("Shows what's in every folder at once.")
                    .clicked()
                {
                    tree.all = true;
                }
                for root in &clipart.folders {
                    node(ui, root, true, listing, clipart.open.as_deref(), &mut tree);
                }
            });
    }
    let root = (clipart.open.as_ref()).filter(|o| clipart.folders.contains(o));
    let mut remove = None;
    ui.horizontal(|ui| {
        if ui.button("Add Folder...").clicked() {
            picked = Some(Picked::AddFolder);
        }
        if ui
            .add_enabled(root.is_some(), egui::Button::new("Remove Folder"))
            .on_hover_hint("Takes the folder out of the Library; its files stay on disk.")
            .on_disabled_hover_text("Click one of your folders in the list first.")
            .clicked()
        {
            remove = root.cloned();
        }
        if ui
            .add_enabled(clipart.open.is_some(), egui::Button::new("Open Folder"))
            .on_hover_hint("Shows the folder in your file manager.")
            .on_disabled_hover_text("Click a folder in the list first.")
            .clicked()
            && let Some(open) = &clipart.open
        {
            picked = Some(Picked::ShowFolder(open.clone()));
        }
    });
    ui.separator();
    if tree.all {
        clipart.choose(None);
    } else if let Some(folder) = tree.choose {
        clipart.choose(Some(folder));
    }
    if let Some(folder) = tree.remove.or(remove)
        && let Err(e) = clipart.remove(&folder)
    {
        clipart.trouble = Some(e);
    }
    tree.show.map(Picked::ShowFolder).or(picked)
}

/// A folder in the tree, named with how many files are in it (and the folders in it), and
/// the folders in it below it, folded away until opened.
fn node(
    ui: &mut egui::Ui,
    folder: &Path,
    root: bool,
    listing: &Listing,
    open: Option<&Path>,
    tree: &mut Tree,
) {
    let count = listing.counts.get(folder).copied().unwrap_or(0);
    let text = if root && !folder.is_dir() {
        egui::RichText::new(format!("{} (not found)", name(folder))).weak()
    } else {
        egui::RichText::new(format!("{} ({count})", name(folder)))
    };
    // the folders in it come straight after it, in order
    let inside: Vec<&PathBuf> = (listing.counts.range(folder.to_path_buf()..))
        .map(|(f, _)| f)
        .take_while(|f| f.starts_with(folder))
        .filter(|f| f.parent() == Some(folder))
        .collect();
    let label = |ui: &mut egui::Ui, tree: &mut Tree| {
        let row = ui
            .selectable_label(open == Some(folder), text)
            .on_hover_hint(folder.display().to_string());
        if row.clicked() {
            tree.choose = Some(folder.to_path_buf());
        }
        cut_craft_bug_report::context_menu(&row, |ui| {
            if ui.button("Open Folder").clicked() {
                tree.show = Some(folder.to_path_buf());
            }
            if root
                && ui
                    .button("Remove Folder")
                    .on_hover_hint("Takes the folder out of the Library; its files stay on disk.")
                    .clicked()
            {
                tree.remove = Some(folder.to_path_buf());
            }
        });
    };
    if inside.is_empty() {
        ui.horizontal(|ui| {
            ui.add_space(ui.spacing().icon_width + ui.spacing().icon_spacing);
            label(ui, tree);
        });
        return;
    }
    let id = ui.make_persistent_id(("library_folder", folder));
    // the way to the folder looked in is open
    let towards = open.is_some_and(|o| o != folder && o.starts_with(folder));
    egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, towards)
        .show_header(ui, |ui| label(ui, tree))
        .body(|ui| {
            for f in inside {
                node(ui, f, false, listing, open, tree);
            }
        });
}

/// What is in the folder looked in, as tiles to pick from: those of the kind shown with
/// the search in their names or tags.
fn files(
    ui: &mut egui::Ui,
    clipart: &mut Clipart,
    listing: &Listing,
    reading: bool,
    levels: &[(u64, String)],
) -> Option<Picked> {
    if let Some(open) = &clipart.open
        && !open.is_dir()
    {
        ui.colored_label(
            ui.visuals().warn_fg_color,
            format!(
                "{} isn't there. If it's on a drive that isn't plugged in, plug it in; if it \
                 was moved, use Remove Folder and add it again from where it is now.",
                open.display()
            ),
        );
        return None;
    }
    let under = |f: &Path| match &clipart.open {
        Some(open) => f.starts_with(open),
        None => clipart.folders.iter().any(|root| f.starts_with(root)),
    };
    let here: Vec<&Entry> = listing.entries.iter().filter(|e| under(&e.file)).collect();
    let locked = listing.locked.iter().filter(|f| under(f)).count();
    let wanted = clipart.search.trim().to_lowercase();
    let shown: Vec<&Entry> = (here.iter().copied())
        .filter(|e| match clipart.show {
            Show::Everything => true,
            Show::Only(kind) => clipart.kind(e) == kind,
        })
        .filter(|e| {
            wanted.is_empty()
                || name(&e.file).to_lowercase().contains(&wanted)
                || clipart.tags.matches(&e.file, &wanted)
        })
        .collect();
    filters(ui, clipart, locked, listing.full);
    let place = clipart.open.as_deref().map_or_else(
        || "all your folders".to_owned(),
        |open| format!("{} and the folders in it", name(open)),
    );
    let files = |n: usize| {
        if n == 1 {
            "1 file".into()
        } else {
            format!("{n} files")
        }
    };
    if shown.is_empty() {
        if reading {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.weak("Reading your folders...");
            });
        } else if here.is_empty() {
            ui.weak(
                "Nothing here yet. 3D models (STL, OBJ, .ccclip), pictures and drawings (SVG, \
                 DXF) show up here.",
            );
        } else {
            ui.weak("Nothing here matches. Clear the search or choose Everything.");
        }
        return None;
    }
    ui.weak(if shown.len() == here.len() {
        format!("{} in {place}", files(here.len()))
    } else {
        format!("{} of {} in {place}", shown.len(), files(here.len()))
    });
    grid(ui, clipart, &shown, levels)
}

/// Search, the Show filter, what is picked, Add Tag's box, and anything gone wrong;
/// `locked` Vectric files where the tab looks, and whether there were too many files to list.
fn filters(ui: &mut egui::Ui, clipart: &mut Clipart, locked: usize, full: bool) {
    ui.horizontal(|ui| {
        egui::ComboBox::from_id_salt("library_show")
            .selected_text(clipart.show.label())
            .show_ui(ui, |ui| {
                for show in Show::ALL {
                    ui.selectable_value(&mut clipart.show, show, show.label());
                }
            })
            .response
            .on_hover_hint(
                "Show one kind of file. Depth maps are greyscale pictures, or pictures you've \
                 tagged \"depth map\".",
            );
        ui.add(
            egui::TextEdit::singleline(&mut clipart.search)
                .hint_text("Search names and tags")
                .desired_width(f32::INFINITY),
        );
    });
    if locked > 0 {
        ui.label(
            egui::RichText::new(format!(
                "{locked} Vectric clipart file{} here can't be shown. Vectric locks .3dclip \
                 and .v3m files to its own software. If you have Aspire, export them as STL \
                 to use them here.",
                if locked == 1 { "" } else { "s" },
            ))
            .weak(),
        );
    }
    if full {
        ui.weak(format!(
            "There are more than {MOST} files in your folders, so only the first {MOST} are \
             shown. Add smaller folders to see the rest."
        ));
    }
    if clipart.selected.len() > 1 {
        ui.horizontal(|ui| {
            ui.label(format!("{} picked", clipart.selected.len()));
            if ui.small_button("Clear").clicked() {
                clipart.selected.clear();
            }
        });
    }
    let mut done = None;
    if let Some((files, tag)) = &mut clipart.tagging {
        ui.horizontal(|ui| {
            ui.label(if files.len() == 1 {
                "Tag:".to_owned()
            } else {
                format!("Tag for {}:", files.len())
            });
            let field = ui.add(
                egui::TextEdit::singleline(tag)
                    .hint_text("e.g. animals, depth map")
                    .desired_width(ui.available_width() - 110.0),
            );
            field.request_focus();
            let enter = field.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            if ui.button("Add").clicked() || enter {
                done = Some(true);
            }
            if ui.button("Cancel").clicked() || ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                done = Some(false);
            }
        });
    }
    if let Some(add) = done
        && let Some((files, tag)) = clipart.tagging.take()
        && add
        && let Err(e) = clipart.tags.add(&files, &tag)
    {
        clipart.trouble = Some(e);
    }
    if let Some(e) = &clipart.trouble {
        ui.colored_label(ui.visuals().error_fg_color, e);
    }
}

/// `shown` as tiles in rows across the tab. Only the rows on screen are drawn, so a folder
/// of thousands is as quick as a folder of ten.
#[expect(
    clippy::cast_precision_loss,
    reason = "whole numbers of tiles, small enough"
)]
fn grid(
    ui: &mut egui::Ui,
    clipart: &mut Clipart,
    shown: &[&Entry],
    levels: &[(u64, String)],
) -> Option<Picked> {
    let mut picked = None;
    let order: Vec<&Path> = shown.iter().map(|e| e.file.as_path()).collect();
    let pictures: HashMap<&Path, &Path> = (shown.iter())
        .filter_map(|e| Some((e.file.as_path(), e.picture.as_deref()?)))
        .collect();
    let edge = ui.spacing().interact_size.y * THUMBNAIL_ROWS;
    let cell = egui::vec2(edge, edge + ui.spacing().interact_size.y);
    let gap = ui.spacing().item_spacing;
    let step = cell + gap;
    let width = ui.available_width();
    let (columns, rows, first, last) = layout(
        shown.len(),
        width,
        step,
        gap.x,
        ui.clip_rect().y_range(),
        ui.cursor().top(),
    );
    let (whole, _) = ui.allocate_exact_size(
        egui::vec2(width, step.y * rows as f32),
        egui::Sense::hover(),
    );
    for (i, entry) in shown.iter().enumerate().take(last).skip(first) {
        let at =
            whole.min + egui::vec2(step.x * (i % columns) as f32, step.y * (i / columns) as f32);
        // named by its file, so its menu stays open as the rows above come and go
        let builder = egui::UiBuilder::new()
            .id_salt(&entry.file)
            .max_rect(egui::Rect::from_min_size(at, cell))
            .layout(egui::Layout::top_down(egui::Align::Min));
        ui.scope_builder(builder, |ui| {
            let tile = Tile {
                entry,
                edge,
                levels,
                order: &order,
                pictures: &pictures,
            };
            if let Some(p) = clip(ui, clipart, &tile) {
                picked = Some(p);
            }
        });
        #[cfg(test)]
        {
            clipart.drawn += 1;
        }
    }
    picked
}

/// `tiles` in a grid `width` across, each `step` (its size and the gap after it) from the
/// next, the grid's top at `top`: how many columns and rows, and the tiles (first, and one
/// past the last) in the rows within `seen`, top to bottom.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "whole numbers of tiles, small enough"
)]
fn layout(
    tiles: usize,
    width: f32,
    step: egui::Vec2,
    gap: f32,
    seen: egui::Rangef,
    top: f32,
) -> (usize, usize, usize, usize) {
    // the last column needs no gap after it
    let columns = (((width + gap) / step.x).floor() as usize).max(1);
    let rows = tiles.div_ceil(columns);
    let row = |y: f32| ((y - top) / step.y).max(0.0);
    let first = (row(seen.min).floor() as usize).min(rows);
    let last = (row(seen.max).ceil() as usize).min(rows);
    (columns, rows, first * columns, (last * columns).min(tiles))
}

/// What a tile needs from the grid around it.
struct Tile<'a> {
    entry: &'a Entry,
    edge: f32,
    /// The job's levels, by id and name, for Add to Level.
    levels: &'a [(u64, String)],
    /// The files shown, in order, for Shift+click.
    order: &'a [&'a Path],
    /// Each model's picture beside it, deleted along with it.
    pictures: &'a HashMap<&'a Path, &'a Path>,
}

/// One file in the grid: its tile and name, to click, double-click, drag or right-click.
fn clip(ui: &mut egui::Ui, clipart: &mut Clipart, tile: &Tile) -> Option<Picked> {
    let mut picked = None;
    let file = &tile.entry.file;
    let label = name(file);
    ui.set_width(tile.edge);
    let chosen = clipart.selected.contains(file);
    let (thumbnail, trouble) = thumbnail(ui, &mut clipart.previews, tile.entry, tile.edge, chosen);
    let thumbnail = thumbnail.interact(egui::Sense::click_and_drag());
    thumbnail.dnd_set_drag_payload(file.clone());
    let caption = ui.add(
        egui::Label::new(egui::RichText::new(&label).small().strong_if(chosen))
            .truncate()
            .sense(egui::Sense::click_and_drag()),
    );
    caption.dnd_set_drag_payload(file.clone());
    for response in [&thumbnail, &caption] {
        if response.clicked() {
            let (toggle, range) = ui.input(|i| (i.modifiers.command, i.modifiers.shift));
            clipart.click(file, toggle, range, tile.order);
        }
        if response.secondary_clicked() && !clipart.selected.contains(file) {
            clipart.click(file, false, false, tile.order);
        }
        if response.double_clicked() {
            picked = Some(Picked::Import(file.clone()));
        }
    }
    // the name under a tile has the tile's menu too
    for response in [&thumbnail, &caption] {
        cut_craft_bug_report::context_menu(response, |ui| {
            if let Some(p) = menu(ui, clipart, tile) {
                picked = Some(p);
            }
        });
    }
    let mut hint = label.clone();
    let tags: Vec<&str> = clipart.tags.of(file).collect();
    if !tags.is_empty() {
        hint = format!("{hint}\nTags: {}", tags.join(", "));
    }
    hint = format!(
        "{hint}\nDouble-click to add it to the project, or drag it onto a view. Click to pick \
         it (Ctrl or Shift for more), right-click for more."
    );
    if let Some(trouble) = trouble {
        hint = format!("{hint}\nNo preview: {trouble}");
    }
    thumbnail.on_hover_hint(hint);
    if picked.is_some() {
        clipart.trouble = None;
    }
    picked
}

/// The right-click menu, for the files picked (the one right-clicked among them).
fn menu(ui: &mut egui::Ui, clipart: &mut Clipart, tile: &Tile) -> Option<Picked> {
    let mut picked = None;
    let files = clipart.selected.clone();
    let many = files.len() > 1;
    let count = |what: &str| {
        if many {
            format!("{what} ({})", files.len())
        } else {
            what.to_owned()
        }
    };
    if ui.button(count("Add to Project")).clicked() {
        picked = Some(Picked::AddToProject(files.clone()));
    }
    ui.menu_button(count("Add to Level"), |ui| {
        for (id, level) in tile.levels {
            if ui.button(level).clicked() {
                picked = Some(Picked::AddToLevel(files.clone(), ToLevel::Existing(*id)));
            }
        }
        if ui
            .button("New Level")
            .on_hover_hint("Each on a new level named after it.")
            .clicked()
        {
            picked = Some(Picked::AddToLevel(files.clone(), ToLevel::New));
        }
    });
    ui.separator();
    if ui
        .add_enabled(!many, egui::Button::new("Locate"))
        .on_hover_hint("Shows the file in its folder.")
        .clicked()
    {
        picked = Some(Picked::Locate(tile.entry.file.clone()));
    }
    if ui.button(count("Add Tag...")).clicked() {
        clipart.tagging = Some((files.clone(), String::new()));
    }
    let tags = clipart.tags.of_all(&files);
    if !tags.is_empty() {
        ui.menu_button("Remove Tag", |ui| {
            for tag in &tags {
                if ui.button(tag).clicked()
                    && let Err(e) = clipart.tags.remove(&files, tag)
                {
                    clipart.trouble = Some(e);
                }
            }
        });
    }
    ui.separator();
    let delete =
        egui::RichText::new(count("Delete from Disk...")).color(ui.visuals().error_fg_color);
    if ui
        .button(delete)
        .on_hover_hint("Deletes the file for good. You're asked first.")
        .clicked()
    {
        let mut doomed = files.clone();
        for f in &files {
            if let Some(picture) = tile.pictures.get(f.as_path()) {
                doomed.push(picture.to_path_buf());
            }
        }
        picked = Some(Picked::Delete(doomed));
    }
    ui.separator();
    if ui
        .button("Properties")
        .on_hover_hint("Its size, date, tags and where it is.")
        .clicked()
    {
        let file = &tile.entry.file;
        clipart.properties = Some(properties::Properties::new(
            file,
            clipart.kind(tile.entry),
            tile.entry.picture.as_deref(),
            clipart.tags.of(file).map(str::to_owned).collect(),
            &clipart.folders,
            Some(ui.ctx()),
        ));
    }
    picked
}

/// Bold when `yes`.
trait StrongIf {
    fn strong_if(self, yes: bool) -> Self;
}

impl StrongIf for egui::RichText {
    fn strong_if(self, yes: bool) -> Self {
        if yes { self.strong() } else { self }
    }
}

/// A file's tile: its picture, its model's preview (asked for only once it is on screen),
/// or what kind of file it is, drawn picked when `chosen`; and why a model has no preview,
/// if it hasn't.
fn thumbnail(
    ui: &mut egui::Ui,
    previews: &mut preview::Previews,
    entry: &Entry,
    edge: f32,
    chosen: bool,
) -> (egui::Response, Option<String>) {
    let size = egui::vec2(edge, edge);
    let extension = preview::extension(&entry.file);
    if DRAWN.contains(&extension.as_str()) {
        let image =
            egui::Image::new(format!("file://{}", entry.file.display())).fit_to_exact_size(size);
        return (ui.add(egui::Button::image(image).selected(chosen)), None);
    }
    let model = preview::MODELS.contains(&extension.as_str());
    let picture = BITMAPS.contains(&extension.as_str());
    let on_screen = ui.is_rect_visible(egui::Rect::from_min_size(ui.cursor().min, size));
    let kind = if model {
        "3D".into()
    } else {
        extension.to_uppercase()
    };
    if !(model || picture) || !on_screen {
        let button = egui::Button::new(kind).selected(chosen);
        return (ui.add_sized(size, button), None);
    }
    // a picture is its own preview: shrunk once, and kept
    let source = if picture {
        Some(entry.file.as_path())
    } else {
        entry.picture.as_deref()
    };
    match previews.get(&entry.file, source, entry.stamp) {
        preview::Preview::Ready(texture) => {
            let image = egui::Image::new(texture).fit_to_exact_size(size);
            (ui.add(egui::Button::image(image).selected(chosen)), None)
        }
        preview::Preview::Waiting => {
            let button = ui.add_sized(size, egui::Button::new("").selected(chosen));
            egui::Spinner::new().paint_at(ui, button.rect.shrink(edge / 3.0));
            (button, None)
        }
        preview::Preview::Failed(e) => {
            let e = e.to_owned();
            let button = egui::Button::new(kind).selected(chosen);
            (ui.add_sized(size, button), Some(e))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn importable(extensions: &[&str]) -> Importable {
        extensions.iter().map(|&e| e.to_owned()).collect()
    }

    /// A folder's clipart wraps onto rows in a narrow panel, not one long line (#34).
    #[test]
    fn thumbnails_wrap() {
        let dir =
            std::env::temp_dir().join(format!("cut-craft-clipart-wrap-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        for i in 0..8 {
            std::fs::write(dir.join(format!("{i}.stl")), "").unwrap();
        }
        let stl = importable(&["stl"]);
        let mut clipart = Clipart {
            folders: vec![dir.clone()],
            ..Clipart::default()
        };
        clipart.read_now(&stl);
        let ctx = egui::Context::default();
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(300.0, 2000.0),
            )),
            ..Default::default()
        };
        let mut used = egui::Rect::NOTHING;
        let mut out = ctx.run_ui(input, |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                panel(ui, &mut clipart, &stl, &[]);
                used = ui.min_rect();
            });
        });
        // no renderer takes a headless frame's texture changes
        out.textures_delta.clear();
        std::fs::remove_dir_all(&dir).unwrap();
        assert!(used.width() <= 300.0, "{used:?} is wider than the panel");
    }

    /// A model's picture beside it (same name, any case) is its preview, not a tile of its
    /// own; a picture of nothing in particular is still a tile; Vectric's locked clipart is
    /// counted, to say why it isn't shown.
    #[test]
    fn pictures_go_with_their_models() {
        let dir =
            std::env::temp_dir().join(format!("cut-craft-clipart-list-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        for f in [
            "mydesign.stl",
            "mydesign.png",
            "Eagle.STL",
            "eagle.JPG",
            "lonely.obj",
            "sunset.png",
            "rose.3dclip",
            "tulip.V3M",
            "notes.txt",
        ] {
            std::fs::write(dir.join(f), "").unwrap();
        }
        let importable = importable(&["stl", "obj", "png", "jpg"]);
        let listing = list(&dir, &importable, &mut HashMap::new());
        let entry = |file: &str, picture: Option<&str>| {
            let picture = picture.map(|p| dir.join(p));
            let source = picture.clone().unwrap_or_else(|| dir.join(file));
            Entry {
                file: dir.join(file),
                stamp: preview::stamp(&source).unwrap(),
                picture,
                kind: if file == "sunset.png" {
                    Kind::Picture
                } else {
                    Kind::Model
                },
            }
        };
        assert_eq!(
            listing,
            Listing {
                entries: vec![
                    entry("Eagle.STL", Some("eagle.JPG")),
                    entry("lonely.obj", None),
                    entry("mydesign.stl", Some("mydesign.png")),
                    entry("sunset.png", None),
                ],
                locked: vec![dir.join("rose.3dclip"), dir.join("tulip.V3M")],
                ..Listing::default()
            }
        );
        // a picture put beside a model later becomes its preview: the model's stamp changes
        let stamp = |listing: &Listing, file: &str| {
            let at = listing.entries.iter().find(|e| e.file == dir.join(file));
            at.unwrap().stamp
        };
        let before = stamp(&listing, "lonely.obj");
        std::fs::write(dir.join("lonely.png"), "").unwrap();
        assert_ne!(
            stamp(&list(&dir, &importable, &mut HashMap::new()), "lonely.obj"),
            before
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// A picture's tile is a preview shrunk from it, not the picture itself: a scan 16 604
    /// pixels wide went past the graphics card's largest texture and stopped the app.
    #[test]
    fn pictures_are_shrunk() {
        let dir = folder("huge");
        image::RgbImage::new(9000, 2)
            .save(dir.join("scan.png"))
            .unwrap();
        image::RgbImage::new(9000, 2)
            .save(dir.join("banner.gif"))
            .unwrap();
        let pictures = importable(&["png", "gif"]);
        let mut clipart = Clipart {
            folders: vec![dir.clone()],
            ..Clipart::default()
        };
        clipart.read_now(&pictures);
        let ctx = egui::Context::default();
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                panel(ui, &mut clipart, &pictures, &[]);
            });
        });
        out.textures_delta.clear();
        assert_eq!(clipart.previews.asked(), 2, "both made into previews");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// Only the tiles on screen ask for previews: a folder of a thousand models opens at
    /// once, and the rest are made as they are scrolled to.
    #[test]
    fn previews_only_on_screen() {
        let dir =
            std::env::temp_dir().join(format!("cut-craft-clipart-lazy-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        for i in 0..200 {
            std::fs::write(dir.join(format!("{i:03}.stl")), "").unwrap();
        }
        let stl = importable(&["stl"]);
        let mut clipart = Clipart {
            folders: vec![dir.clone()],
            ..Clipart::default()
        };
        clipart.read_now(&stl);
        let ctx = egui::Context::default();
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(300.0, 400.0),
            )),
            ..Default::default()
        };
        let mut out = ctx.run_ui(input, |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    panel(ui, &mut clipart, &stl, &[]);
                });
            });
        });
        out.textures_delta.clear();
        std::fs::remove_dir_all(&dir).unwrap();
        let asked = clipart.previews.asked();
        assert!(asked > 0 && asked < 50, "{asked} previews asked for");
    }

    fn folder(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("cut-craft-library-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// Each file is a kind the Show filter picks: a greyscale picture, or one tagged so, is
    /// a depth map.
    #[test]
    fn kinds_of_file() {
        let dir = folder("kinds");
        std::fs::write(dir.join("eagle.stl"), "").unwrap();
        std::fs::write(dir.join("logo.svg"), "<svg/>").unwrap();
        std::fs::write(dir.join("plan.dxf"), "").unwrap();
        image::RgbImage::new(4, 4)
            .save(dir.join("photo.png"))
            .unwrap();
        image::RgbImage::new(4, 4)
            .save(dir.join("tagged.png"))
            .unwrap();
        image::GrayImage::new(4, 4)
            .save(dir.join("relief.png"))
            .unwrap();
        let mut clipart = Clipart::default();
        clipart
            .tags
            .add(&[dir.join("tagged.png")], "Depth Map")
            .unwrap();
        let all = importable(&["stl", "svg", "dxf", "png"]);
        let listing = list(&dir, &all, &mut clipart.greys);
        let kinds: Vec<(String, Kind)> = (listing.entries.iter())
            .map(|e| (name(&e.file), clipart.kind(e)))
            .collect();
        assert_eq!(
            kinds,
            [
                ("eagle.stl".into(), Kind::Model),
                ("logo.svg".into(), Kind::Drawing),
                ("photo.png".into(), Kind::Picture),
                ("plan.dxf".into(), Kind::Drawing),
                ("relief.png".into(), Kind::DepthMap),
                ("tagged.png".into(), Kind::DepthMap),
            ]
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// A click picks one; Ctrl+click adds or takes one away; Shift+click picks everything
    /// from the last one clicked, as file managers do.
    #[test]
    fn picking() {
        let files: Vec<PathBuf> = ["a", "b", "c", "d", "e"].map(PathBuf::from).into();
        let order: Vec<&Path> = files.iter().map(PathBuf::as_path).collect();
        let mut clipart = Clipart::default();
        let picked = |c: &Clipart| {
            let mut p: Vec<String> = c.selected.iter().map(|f| name(f)).collect();
            p.sort();
            p.join("")
        };
        clipart.click(&files[1], false, false, &order);
        assert_eq!(picked(&clipart), "b");
        clipart.click(&files[3], false, true, &order);
        assert_eq!(picked(&clipart), "bcd");
        clipart.click(&files[0], false, true, &order);
        assert_eq!(picked(&clipart), "ab", "Shift+click again goes from b");
        clipart.click(&files[4], true, false, &order);
        assert_eq!(picked(&clipart), "abe");
        clipart.click(&files[0], true, false, &order);
        assert_eq!(picked(&clipart), "be");
        clipart.click(&files[2], false, false, &order);
        assert_eq!(picked(&clipart), "c");
    }

    /// Delete from Disk takes the files away for good, and with them their tags and
    /// their place among the picked; one already gone is no trouble.
    #[test]
    fn deleting() {
        let dir = folder("delete");
        let (stl, png) = (dir.join("eagle.stl"), dir.join("eagle.png"));
        std::fs::write(&stl, "").unwrap();
        std::fs::write(&png, "").unwrap();
        let mut clipart = Clipart::default();
        clipart
            .tags
            .add(std::slice::from_ref(&stl), "birds")
            .unwrap();
        clipart.selected = vec![stl.clone(), dir.join("other.stl")];
        clipart
            .delete(&[stl.clone(), png.clone(), dir.join("never-there.stl")])
            .unwrap();
        assert!(!stl.exists() && !png.exists());
        assert_eq!(clipart.selected, [dir.join("other.stl")]);
        assert_eq!(clipart.tags.of(&stl).count(), 0);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// The Library opens on its folders as they were: kept between runs, looking in the
    /// folder looked in last; in all of them if that was all, or it's no longer added.
    #[test]
    fn opens_where_it_was() {
        let dir = folder("kept");
        let store = dir.join("clipart.json");
        let (a, b) = (dir.join("a"), dir.join("b"));
        let mut library = Clipart::kept(Some(store.clone()));
        assert_eq!(library.open, None, "nothing to look in yet");
        library.add(a.clone()).unwrap();
        library.add(b.clone()).unwrap();
        let mut again = Clipart::kept(Some(store.clone()));
        assert_eq!(again.folders, [a.clone(), b.clone()]);
        assert_eq!(
            again.open,
            Some(b.clone()),
            "the one added last is looked in"
        );
        again.choose(Some(a.join("birds")));
        assert_eq!(
            Clipart::kept(Some(store.clone())).open,
            Some(a.join("birds"))
        );
        again.choose(None);
        assert_eq!(Clipart::kept(Some(store.clone())).open, None, "all of them");
        again.choose(Some(a.join("birds")));
        again.remove(&a).unwrap();
        assert_eq!(
            again.open, None,
            "looking in a folder taken away: all of them"
        );
        assert_eq!(Clipart::kept(Some(store.clone())).folders, [b]);
        assert_eq!(Clipart::kept(Some(store)).open, None);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// A folder is in the library once: one added inside another already there is looked
    /// in, not added; one added round others takes their place; and folders kept from
    /// before this (Matthew's had both) are tidied as they are loaded.
    #[test]
    fn no_folder_twice() {
        let dir = folder("once");
        let store = dir.join("clipart.json");
        let (team, club, other) = (
            dir.join("team"),
            dir.join("team").join("club"),
            dir.join("other"),
        );
        crate::files::save_recent(&[other.clone(), team.clone(), club.clone()], &store).unwrap();
        let mut library = Clipart::kept(Some(store.clone()));
        assert_eq!(
            library.folders,
            [other.clone(), team.clone()],
            "tidied as loaded"
        );
        assert_eq!(
            Clipart::kept(Some(store.clone())).folders,
            [other.clone(), team.clone()],
            "and kept tidy"
        );
        library.add(club.clone()).unwrap();
        assert_eq!(
            library.folders,
            [other.clone(), team.clone()],
            "inside one already added"
        );
        assert_eq!(
            library.open,
            Some(club.clone()),
            "so looked in there instead"
        );
        library.add(team.clone()).unwrap();
        assert_eq!(
            library.folders,
            [other.clone(), team.clone()],
            "the same one again"
        );
        library.add(dir.clone()).unwrap();
        assert_eq!(
            library.folders,
            std::slice::from_ref(&dir),
            "round the others: they're in it"
        );
        assert_eq!(
            Clipart::kept(Some(store)).folders,
            std::slice::from_ref(&dir)
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// Where `text` is drawn in `shapes`, if it is.
    fn text_at(shapes: &[egui::epaint::ClippedShape], text: &str) -> Option<egui::Pos2> {
        shapes.iter().find_map(|c| match &c.shape {
            egui::Shape::Text(t) if t.galley.text() == text => Some(t.pos + t.galley.size() / 2.0),
            _ => None,
        })
    }

    /// Right-click ▸ Properties, at the bottom of a tile's menu, opens a window about the
    /// file: its kind, size, tags and where it is. Escape closes it.
    #[test]
    fn properties_from_the_menu() {
        let dir = folder("properties");
        let file = dir.join("eagle.stl");
        std::fs::write(&file, "").unwrap();
        let stl = importable(&["stl"]);
        let mut clipart = Clipart {
            folders: vec![dir.clone()],
            ..Clipart::default()
        };
        clipart
            .tags
            .add(std::slice::from_ref(&file), "birds")
            .unwrap();
        clipart.read_now(&stl);
        let ctx = egui::Context::default();
        let frame = |clipart: &mut Clipart, events: Vec<egui::Event>| {
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(800.0, 800.0),
                )),
                events,
                ..Default::default()
            };
            let mut out = ctx.run_ui(input, |ui| {
                egui::CentralPanel::default().show(ui, |ui| {
                    panel(ui, clipart, &stl, &[]);
                });
            });
            out.textures_delta.clear();
            out.shapes
        };
        let click = |at: egui::Pos2, button: egui::PointerButton| {
            [true, false].map(|pressed| egui::Event::PointerButton {
                pos: at,
                button,
                pressed,
                modifiers: egui::Modifiers::NONE,
            })
        };
        let shapes = frame(&mut clipart, vec![]);
        let tile = text_at(&shapes, "eagle.stl").expect("the tile's name");
        frame(&mut clipart, vec![egui::Event::PointerMoved(tile)]);
        for e in click(tile, egui::PointerButton::Secondary) {
            frame(&mut clipart, vec![e]);
        }
        // a new menu is drawn unseen for a frame while egui sizes it
        frame(&mut clipart, vec![]);
        let shapes = frame(&mut clipart, vec![]);
        let item = text_at(&shapes, "Properties").expect("Properties in the menu");
        let report = text_at(&shapes, "Report a Bug Here…").expect("the bug report item");
        assert!(item.y < report.y, "Properties comes before Report a Bug");
        frame(&mut clipart, vec![egui::Event::PointerMoved(item)]);
        for e in click(item, egui::PointerButton::Primary) {
            frame(&mut clipart, vec![e]);
        }
        clipart
            .properties
            .as_mut()
            .expect("the window is open")
            .wait();
        frame(&mut clipart, vec![]);
        let shapes = frame(&mut clipart, vec![]);
        for text in ["3D model (STL file)", "birds", "0 bytes"] {
            assert!(text_at(&shapes, text).is_some(), "{text} is shown");
        }
        assert!(
            text_at(&shapes, &file.display().to_string()).is_some(),
            "its path"
        );
        let escape = egui::Event::Key {
            key: egui::Key::Escape,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        };
        frame(&mut clipart, vec![escape]);
        assert!(clipart.properties.is_none(), "Escape closes it");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// The folders in the folders added are read too, however deep, and counted for the
    /// tree; hidden ones aren't, and a folder added inside another is read once.
    #[test]
    fn folders_in_folders() {
        let dir = folder("deep");
        let birds = dir.join("animals").join("birds");
        let hidden = dir.join(".thumbnails");
        for d in [&birds, &hidden, &dir.join("empty")] {
            std::fs::create_dir_all(d).unwrap();
        }
        for f in [
            dir.join("top.stl"),
            dir.join("animals").join("cat.stl"),
            birds.join("eagle.stl"),
            birds.join("owl.stl"),
            hidden.join("secret.stl"),
        ] {
            std::fs::write(f, "").unwrap();
        }
        // a link back up, which would go round for ever if followed
        #[cfg(unix)]
        std::os::unix::fs::symlink(&dir, birds.join("loop")).unwrap();
        let stl = importable(&["stl"]);
        let read = read_all(&[dir.clone(), birds.clone()], &stl, &mut HashMap::new());
        let names: Vec<String> = read.entries.iter().map(|e| name(&e.file)).collect();
        // by folder, then name
        assert_eq!(names, ["eagle.stl", "owl.stl", "cat.stl", "top.stl"]);
        let count = |f: &Path| read.counts.get(f).copied();
        assert_eq!(count(&dir), Some(4));
        assert_eq!(count(&dir.join("animals")), Some(3));
        assert_eq!(count(&birds), Some(2));
        assert_eq!(
            count(&dir.join("empty")),
            None,
            "nothing in it: not in the tree"
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// Thousands of files open at once: only the tiles on screen are drawn. Looking in a
    /// folder shows what is in it and the folders in it; looking in none, everything.
    #[test]
    fn thousands_of_files() {
        let root = folder("thousands");
        std::fs::create_dir_all(root.join("few")).unwrap();
        let entry = |file: PathBuf| Entry {
            file,
            picture: None,
            stamp: 0,
            kind: Kind::Drawing,
        };
        let mut entries: Vec<Entry> = (0..3000)
            .map(|i| entry(root.join(format!("{i:04}.dxf"))))
            .collect();
        entries.extend((0..5).map(|i| entry(root.join("few").join(format!("{i}.dxf")))));
        let mut clipart = Clipart {
            folders: vec![root.clone()],
            listing: Some((
                vec![root.clone()],
                Instant::now(),
                Listing {
                    entries,
                    ..Listing::default()
                },
            )),
            ..Clipart::default()
        };
        let ctx = egui::Context::default();
        let draw = |clipart: &mut Clipart| {
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(300.0, 600.0),
                )),
                ..Default::default()
            };
            let mut out = ctx.run_ui(input, |ui| {
                egui::CentralPanel::default().show(ui, |ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        panel(ui, clipart, &importable(&["dxf"]), &[]);
                    });
                });
            });
            out.textures_delta.clear();
            clipart.drawn
        };
        let drawn = draw(&mut clipart);
        assert!(drawn > 0 && drawn < 60, "{drawn} tiles drawn");
        clipart.open = Some(root.join("few"));
        assert_eq!(draw(&mut clipart), 5);
        std::fs::remove_dir_all(&root).unwrap();
    }

    /// Tiles fill the rows across, and the rows in view are the ones drawn.
    #[test]
    fn rows_in_view() {
        let step = egui::vec2(60.0, 80.0);
        let seen = |from: f32, to: f32| egui::Rangef::new(from, to);
        // 300 wide fits 5 tiles 56 across with a 4 gap between (the last needs none)
        assert_eq!(
            layout(12, 296.0, step, 4.0, seen(0.0, 100.0), 0.0),
            (5, 3, 0, 10)
        );
        assert_eq!(
            layout(12, 296.0, step, 4.0, seen(170.0, 400.0), 0.0),
            (5, 3, 10, 12)
        );
        assert_eq!(
            layout(12, 10.0, step, 4.0, seen(0.0, 10.0), 50.0),
            (1, 12, 0, 0)
        );
        assert_eq!(
            layout(0, 300.0, step, 4.0, seen(0.0, 100.0), 0.0),
            (5, 0, 0, 0)
        );
    }
}
