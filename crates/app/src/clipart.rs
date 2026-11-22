//! The Library tab: folders of the user's own clipart as a tree, and what is in the one
//! picked (and the folders in it; in all of them when none is) as tiles: pictures and 3D
//! models as previews, drawings by their kind. Tiles are picked with a click (Ctrl or
//! Shift for more), added to the project with a double-click or by dragging, and a
//! right-click menu adds, locates, tags or deletes the ones picked, or shows one's Properties.

pub(crate) mod network;
pub(crate) mod preview;
mod properties;
mod tags;

use cut_craft_bug_report::hint::HoverHint;
use cut_craft_i18n::tr;
use eframe::egui;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::{Duration, Instant};

/// A thumbnail's edge, in rows of the theme's interactive height.
// ponytail: one size; the three view sizes can come with a setting
const THUMBNAIL_ROWS: f32 = 4.0;
/// The pop-up preview's edge, in rows of buttons.
const LARGE_ROWS: f32 = 16.0;
/// Pictures drawn as themselves (at the tile's size); the rest ([`BITMAPS`]) are shrunk
/// once and kept, as a model's preview is, so a folder of big photos doesn't fill the
/// memory, nor a huge scan go past the graphics card's largest texture (which stops the app).
const DRAWN: [&str; 1] = ["svg"];
/// Pictures made of pixels (a depth map is one of these).
const BITMAPS: [&str; 7] = ["png", "jpg", "jpeg", "gif", "bmp", "tif", "tiff"];
/// Vectric's clipart, which is locked to Vectric's own software: counted, not shown.
const LOCKED: [&str; 2] = ["3dclip", "v3m"];
/// How often the folders are read again, for files added, moved or taken away meanwhile,
/// unless Options ▸ Advanced says otherwise.
pub const RELIST: Duration = Duration::from_secs(5);
/// How often a network folder that couldn't be reached is tried again.
const RETRY: Duration = Duration::from_secs(30);
/// At most this many files are listed, so a folder as big as a whole drive can't swamp it,
/// unless Options ▸ Advanced says otherwise.
pub const MOST: usize = 100_000;

/// What Options ▸ Advanced sets for the Library, read wherever it is used; the app sets them
/// when the options load or change.
pub mod limits {
    use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
    use std::time::Duration;

    static RELIST: AtomicU64 = AtomicU64::new(super::RELIST.as_secs());
    static MOST: AtomicUsize = AtomicUsize::new(super::MOST);
    static UNUSED_DAYS: AtomicU64 = AtomicU64::new(super::preview::UNUSED_DAYS);
    static KEEP: AtomicUsize = AtomicUsize::new(super::preview::KEEP);

    /// Sets how often the folders are read again, the most files listed, how many days an
    /// unused preview is kept, and how many previews stay on the graphics card; each at
    /// least 1.
    pub fn set(relist_seconds: u64, most: usize, unused_days: u64, keep: usize) {
        RELIST.store(relist_seconds.max(1), Ordering::Relaxed);
        MOST.store(most.max(1), Ordering::Relaxed);
        UNUSED_DAYS.store(unused_days.max(1), Ordering::Relaxed);
        KEEP.store(keep.max(1), Ordering::Relaxed);
    }

    /// How often the folders are read again.
    pub fn relist() -> Duration {
        Duration::from_secs(RELIST.load(Ordering::Relaxed))
    }

    /// The most files listed.
    pub fn most() -> usize {
        MOST.load(Ordering::Relaxed)
    }

    /// How long an unused preview is kept.
    pub fn unused() -> Duration {
        Duration::from_hours(24 * UNUSED_DAYS.load(Ordering::Relaxed))
    }

    /// How many previews stay on the graphics card.
    pub fn keep() -> usize {
        KEEP.load(Ordering::Relaxed)
    }
}
/// The folder tree's height at most, in rows of the theme's interactive height.
const TREE_ROWS: f32 = 10.0;

/// The extensions of files something imports, in lower case.
pub type Importable = HashSet<String>;
/// Whether each picture is greyscale (a depth map), by the [`preview::stamp`] it had.
type Greys = HashMap<PathBuf, (u64, bool)>;
/// Folders read in the background: which, what is in them, the greys learnt, and each
/// folder that wasn't there looked for again (a network share): where it is now, if found.
type Read = (
    Vec<PathBuf>,
    Listing,
    Greys,
    Vec<(PathBuf, Option<PathBuf>)>,
);

/// The library's folders, kept between runs, and the one being looked in.
#[derive(Default)]
pub struct Clipart {
    /// The folders added to the library: the tree's roots. One on a network share that
    /// hasn't been found on this computer yet is its location (`smb://host/share/...`).
    pub folders: Vec<PathBuf>,
    /// The folder looked in, with the folders in it; `None`: all of them.
    pub open: Option<PathBuf>,
    /// Where the folders are kept; `None` keeps them only for this run.
    store: Option<PathBuf>,
    /// Models' and pictures' previews, made in the background.
    previews: preview::Previews,
    /// Larger previews, for the pop-up over the tile the pointer rests on.
    large: preview::Enlarged,
    /// Everything in the folders as last read: which folders, when, and what.
    listing: Option<(Vec<PathBuf>, Instant, Listing)>,
    /// The folders being read in the background.
    reading: Option<mpsc::Receiver<Read>>,
    /// Read the folders again as soon as they can be (a file was deleted).
    again: bool,
    /// The network folders looked for and not reached, and when that was.
    unreached: HashSet<PathBuf>,
    tried: Option<Instant>,
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

    fn label(self) -> String {
        match self {
            Self::Everything => tr!("clipart-show-everything"),
            Self::Only(Kind::Model) => tr!("clipart-show-models"),
            Self::Only(Kind::Drawing) => tr!("clipart-show-drawings"),
            Self::Only(Kind::Picture) => tr!("clipart-show-pictures"),
            Self::Only(Kind::DepthMap) => tr!("clipart-show-depth-maps"),
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

/// Whether `files` can go onto a level: 3D clipart only, which comes in as components. A
/// drawing or picture comes in as vectors or a picture on the current layer, with no level.
pub fn takes_a_level(files: &[PathBuf]) -> bool {
    files.iter().all(|f| {
        preview::MODELS.contains(&preview::extension(f).as_str())
            || cut_craft_border_tile::of_depth_map(f).is_some()
    })
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
    /// Closes the Properties window, if it is open (Esc, which reaches the app first);
    /// false when it was not.
    pub fn close_properties(&mut self) -> bool {
        self.properties.take().is_some()
    }

    /// Opens the Properties window on `file`, as its menu item does.
    #[cfg(test)]
    pub fn open_properties(&mut self, file: &Path) {
        self.properties = Some(properties::Properties::new(
            file,
            Kind::Model,
            None,
            vec![],
            &[],
            None,
        ));
    }

    /// The library kept in the config folder.
    pub fn load() -> Self {
        let cache = crate::files::clipart_previews_path();
        if let Some(cache) = cache.clone() {
            let _ = std::thread::Builder::new()
                .name("clipart preview pruning".into())
                .spawn(move || preview::prune(&cache, limits::unused()));
        }
        Self {
            previews: preview::Previews::new(cache.clone()),
            large: preview::Enlarged::new(cache),
            tags: tags::Tags::load(crate::files::library_tags_path()),
            ..Self::kept(crate::files::clipart_folders_path())
        }
    }

    /// The folders kept at `store`, looking in the folder looked in last (all of them, if
    /// none was or it is no longer in the library).
    fn kept(store: Option<PathBuf>) -> Self {
        let loaded: Vec<PathBuf> = store.as_deref().map_or_default(crate::files::load_recent);
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
            && let Ok(bytes) = serde_json::to_vec(&self.open.as_deref().map(network::kept))
        {
            // only a convenience: not remembered, all the folders are shown next time
            let _ = cut_craft_core::file::write_atomic(&open_path(store), &bytes);
        }
    }

    /// Adds `folder` and looks in it. One inside a folder already added is only looked in
    /// (it is in the tree already); the folders inside one added are in it, so they go.
    pub fn add(&mut self, folder: PathBuf) -> Result<(), String> {
        let kept = network::kept(&folder);
        let saved = if (self.folders.iter())
            .any(|f| folder.starts_with(f) || kept.starts_with(network::kept(f)))
        {
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
            Some(path) => {
                let kept: Vec<PathBuf> = self.folders.iter().map(|f| network::kept(f)).collect();
                crate::files::save_recent(&kept, path)
            }
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
            || (self.listing.as_ref()).is_none_or(|(roots, at, _)| {
                *roots != self.folders || at.elapsed() > limits::relist()
            });
        if stale && self.reading.is_none() {
            self.again = false;
            let roots = self.folders.clone();
            let importable = importable.clone();
            let mut greys = std::mem::take(&mut self.greys);
            let look = self.tried.is_none_or(|at| at.elapsed() > RETRY);
            let (send, receive) = mpsc::channel();
            let ctx = ctx.clone();
            let spawned = std::thread::Builder::new()
                .name("library reading".into())
                .spawn(move || {
                    let found = if look { look_for(&roots) } else { Vec::new() };
                    let listing = read_all(&roots, &importable, &mut greys);
                    let _ = send.send((roots, listing, greys, found));
                    ctx.request_repaint();
                });
            if spawned.is_ok() {
                self.reading = Some(receive);
            }
        }
        ctx.request_repaint_after(limits::relist());
    }

    /// Folders read: shown from now on.
    fn read(&mut self, (roots, listing, greys, found): Read) {
        self.reading = None;
        self.greys = greys;
        self.found(found);
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
        let found = look_for(&roots);
        let listing = read_all(&roots, importable, &mut greys);
        self.read((roots, listing, greys, found));
    }

    /// Folders looked for again: each found takes the place of the location (or the old
    /// path) it was kept as, and the folder looked in moves with it; those not found are
    /// tried again after [`RETRY`].
    fn found(&mut self, found: Vec<(PathBuf, Option<PathBuf>)>) {
        if found.is_empty() {
            return;
        }
        self.unreached.clear();
        for (was, now) in found {
            let Some(now) = now else {
                self.unreached.insert(was);
                continue;
            };
            if now == was {
                continue;
            }
            for folder in &mut self.folders {
                if *folder == was {
                    folder.clone_from(&now);
                }
            }
            if let Some(open) = &mut self.open
                && let Ok(rest) = open.strip_prefix(&was)
            {
                *open = now.join(rest);
            }
        }
        self.folders = outermost(&self.folders);
        self.tried = (!self.unreached.is_empty()).then(Instant::now);
    }

    /// Whether `folder` (one of the roots) is on a network share that is being connected
    /// to (`Some(false)`) or couldn't be reached (`Some(true)`); `None` if it isn't one.
    fn unreachable(&self, folder: &Path) -> Option<bool> {
        let network = network::is_location(folder) || network::location(folder).is_some();
        network.then(|| self.unreached.contains(folder))
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
                Err(e) => Err(tr!(
                    "clipart-could-not-delete",
                    name = name(f),
                    error = e.to_string()
                )),
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

/// Where each of `roots` that isn't there (a network location, or a share's old mount)
/// is now, if it can be found.
fn look_for(roots: &[PathBuf]) -> Vec<(PathBuf, Option<PathBuf>)> {
    (roots.iter())
        .filter(|r| !there(r))
        .map(|r| (r.clone(), network::find(r)))
        .collect()
}

/// Whether `folder` is a folder on this computer now (a network location isn't, until it
/// is found mounted).
fn there(folder: &Path) -> bool {
    !network::is_location(folder) && folder.is_dir()
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
    read_most(roots, importable, greys, limits::most())
}

/// [`read_all`], stopping once `most` files are listed.
fn read_most(
    roots: &[PathBuf],
    importable: &Importable,
    greys: &mut Greys,
    most: usize,
) -> Listing {
    let mut all = Listing::default();
    for root in &outermost(roots) {
        let mut waiting = vec![root.clone()];
        while let Some(folder) = waiting.pop() {
            if all.entries.len() >= most {
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
    // a generated border (its depth map with a recipe beside it) is a model, its picture
    // the one saved with it
    let tiles: HashMap<&PathBuf, PathBuf> = (files.iter())
        .filter(|p| is(p, &["png"]))
        .filter_map(|p| Some((p, cut_craft_border_tile::of_depth_map(p)?.picture)))
        .collect();
    let picture_of = |p: &Path| {
        if let Some(picture) = tiles.get(&p.to_path_buf()) {
            return files.iter().find(|f| *f == picture);
        }
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
            let kind = if is(file, &preview::MODELS) || tiles.contains_key(file) {
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
    clipart.large.update(ui.ctx());
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
    ui.strong(tr!("clipart-folders"));
    let mut tree = Tree::default();
    if clipart.folders.is_empty() {
        ui.weak(tr!("clipart-no-folders-yet-add-a"));
    } else {
        let most = ui.spacing().interact_size.y * TREE_ROWS;
        egui::ScrollArea::vertical()
            .id_salt("library_tree")
            .max_height(most)
            .auto_shrink([false, true])
            .show(ui, |ui| {
                let all = tr!("clipart-all-folders", count = listing.entries.len());
                if ui
                    .selectable_label(clipart.open.is_none(), all)
                    .on_hover_hint(tr!("clipart-shows-what-s-in-every"))
                    .clicked()
                {
                    tree.all = true;
                }
                for root in &clipart.folders {
                    let tag = (!there(root)).then(|| match clipart.unreachable(root) {
                        Some(false) => tr!("clipart-folder-connecting"),
                        Some(true) => tr!("clipart-folder-cant-reach"),
                        None => tr!("clipart-folder-not-found"),
                    });
                    node(
                        ui,
                        root,
                        true,
                        tag.as_deref(),
                        listing,
                        clipart.open.as_deref(),
                        &mut tree,
                    );
                }
            });
    }
    let root = (clipart.open.as_ref()).filter(|o| clipart.folders.contains(o));
    let mut remove = None;
    ui.horizontal(|ui| {
        if ui.button(tr!("clipart-add-folder")).clicked() {
            picked = Some(Picked::AddFolder);
        }
        if ui
            .add_enabled(
                root.is_some(),
                egui::Button::new(tr!("clipart-remove-folder")),
            )
            .on_hover_hint(tr!("clipart-takes-the-folder-out-of"))
            .on_disabled_hover_text(tr!("clipart-click-one-of-your-folders"))
            .clicked()
        {
            remove = root.cloned();
        }
        if ui
            .add_enabled(
                clipart.open.is_some(),
                egui::Button::new(tr!("clipart-open-folder")),
            )
            .on_hover_hint(tr!("clipart-shows-the-folder-in-your"))
            .on_disabled_hover_text(tr!("clipart-click-a-folder-in-the"))
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
/// the folders in it below it, folded away until opened. `missing` says why a root folder
/// isn't there (`None` for one that is, and for the folders in a root).
fn node(
    ui: &mut egui::Ui,
    folder: &Path,
    root: bool,
    missing: Option<&str>,
    listing: &Listing,
    open: Option<&Path>,
    tree: &mut Tree,
) {
    let count = listing.counts.get(folder).copied().unwrap_or(0);
    let text = match missing {
        Some(why) => egui::RichText::new(format!("{} ({why})", name(folder))).weak(),
        None => egui::RichText::new(format!("{} ({count})", name(folder))),
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
            if ui.button(tr!("clipart-open-folder")).clicked() {
                tree.show = Some(folder.to_path_buf());
            }
            if root
                && ui
                    .button(tr!("clipart-remove-folder"))
                    .on_hover_hint(tr!("clipart-takes-the-folder-out-of"))
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
                node(ui, f, false, None, listing, open, tree);
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
        && !there(open)
    {
        let root = (clipart.folders.iter()).find(|f| open.starts_with(f));
        let warn = ui.visuals().warn_fg_color;
        match root.and_then(|r| clipart.unreachable(r)) {
            Some(false) => {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.weak(tr!(
                        "clipart-connecting",
                        place = network::kept(open).display().to_string()
                    ));
                });
            }
            Some(true) => {
                ui.colored_label(
                    warn,
                    tr!(
                        "clipart-cant-reach-on-network",
                        place = network::kept(open).display().to_string()
                    ),
                );
            }
            None => {
                ui.colored_label(
                    warn,
                    tr!(
                        "clipart-folder-is-not-there",
                        place = open.display().to_string()
                    ),
                );
            }
        }
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
        || tr!("clipart-place-everywhere"),
        |open| tr!("clipart-place-folder", folder = name(open)),
    );
    if shown.is_empty() {
        if reading {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.weak(tr!("clipart-reading-your-folders"));
            });
        } else if here.is_empty() {
            ui.weak(tr!("clipart-nothing-here-yet-3d-models"));
        } else {
            ui.weak(tr!("clipart-nothing-here-matches-clear-the"));
        }
        return None;
    }
    ui.weak(if shown.len() == here.len() {
        tr!("clipart-files-in", count = here.len(), place = place)
    } else {
        tr!(
            "clipart-some-files-in",
            shown = shown.len(),
            count = here.len(),
            place = place
        )
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
            .on_hover_hint(tr!("clipart-show-one-kind-of-file"));
        ui.add(
            egui::TextEdit::singleline(&mut clipart.search)
                .hint_text(tr!("clipart-search-names-and-tags"))
                .desired_width(f32::INFINITY),
        );
    });
    if locked > 0 {
        ui.label(
            egui::RichText::new(format!(
                "{locked} Vectric clipart file{} here can't be shown: .3dclip and .v3m \
                 files only open in the program that made them. Export them from it as STL \
                 to use them here.",
                if locked == 1 { "" } else { "s" },
            ))
            .weak(),
        );
    }
    if full {
        let most = limits::most();
        ui.weak(tr!("clipart-too-many-files", most = most));
    }
    if clipart.selected.len() > 1 {
        ui.horizontal(|ui| {
            ui.label(tr!("clipart-picked", count = clipart.selected.len()));
            if ui.small_button(tr!("clipart-clear")).clicked() {
                clipart.selected.clear();
            }
        });
    }
    let mut done = None;
    if let Some((files, tag)) = &mut clipart.tagging {
        ui.horizontal(|ui| {
            ui.label(tr!("clipart-tag-files", count = files.len()));
            let field = ui.add(
                egui::TextEdit::singleline(tag)
                    .hint_text(tr!("clipart-e-g-animals-depth-map"))
                    .desired_width(ui.available_width() - 110.0),
            );
            field.request_focus();
            let enter = field.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            if ui.button(tr!("clipart-add")).clicked() || enter {
                done = Some(true);
            }
            if ui.button(tr!("clipart-cancel")).clicked()
                || ui.input(|i| i.key_pressed(egui::Key::Escape))
            {
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
    let tags: Vec<String> = clipart.tags.of(file).map(str::to_owned).collect();
    if thumbnail.hovered() || caption.hovered() {
        cut_craft_bug_report::hint::hint(ui.ctx(), &tr!("clipart-tile-hint", name = label.clone()));
    }
    // the pop-up hangs off the tile, not the pointer, so it holds still while looked at
    for response in [thumbnail, caption] {
        response.on_hover_ui(|ui| {
            enlarged(ui, clipart, tile.entry, &tags, trouble.as_deref());
        });
    }
    if picked.is_some() {
        clipart.trouble = None;
    }
    picked
}

/// The pop-up over a tile the pointer rests on: the file larger, its whole name, its tags,
/// and why it has no preview if it hasn't. Always the same size whether its picture is in
/// yet or not, so it doesn't jump about.
fn enlarged(
    ui: &mut egui::Ui,
    clipart: &mut Clipart,
    entry: &Entry,
    tags: &[String],
    trouble: Option<&str>,
) {
    let edge = ui.spacing().interact_size.y * LARGE_ROWS;
    let square = egui::vec2(edge, edge);
    let extension = preview::extension(&entry.file);
    // a generated border's depth map is a model shown by its own picture
    let picture = BITMAPS.contains(&extension.as_str()) && entry.picture.is_none();
    let model = preview::MODELS.contains(&extension.as_str()) || entry.picture.is_some();
    if DRAWN.contains(&extension.as_str()) {
        let uri = format!("file://{}", entry.file.display());
        let (rect, _) = ui.allocate_exact_size(square, egui::Sense::hover());
        ui.put(rect, egui::Image::new(uri).fit_to_exact_size(square));
    } else if (picture || model) && trouble.is_none() {
        let source = if picture {
            Some(entry.file.as_path())
        } else {
            entry.picture.as_deref()
        };
        let (rect, _) = ui.allocate_exact_size(square, egui::Sense::hover());
        let large = match clipart.large.get(&entry.file, source, entry.stamp) {
            preview::Preview::Ready(texture) => Some(texture.clone()),
            _ => None,
        };
        // the tile's own preview, stretched, until the larger one is in
        let shown =
            large.or_else(
                || match clipart.previews.get(&entry.file, source, entry.stamp) {
                    preview::Preview::Ready(texture) => Some(texture.clone()),
                    _ => None,
                },
            );
        match shown {
            Some(texture) => {
                ui.put(rect, egui::Image::new(&texture).fit_to_exact_size(square));
            }
            None => {
                egui::Spinner::new().paint_at(ui, rect.shrink(edge / 3.0));
            }
        }
    }
    ui.set_max_width(edge.max(ui.spacing().tooltip_width));
    ui.strong(name(&entry.file));
    if !tags.is_empty() {
        ui.weak(tr!("clipart-tags", tags = tags.join(", ")));
    }
    if let Some(trouble) = trouble {
        ui.weak(tr!("clipart-no-preview", trouble = trouble.to_string()));
    }
}

/// The right-click menu, for the files picked (the one right-clicked among them).
fn menu(ui: &mut egui::Ui, clipart: &mut Clipart, tile: &Tile) -> Option<Picked> {
    let mut picked = None;
    let files = clipart.selected.clone();
    let many = files.len() > 1;
    let count = |key: &str| {
        if many {
            tr!("clipart-menu-count", what = tr!(key), count = files.len())
        } else {
            tr!(key)
        }
    };
    if ui.button(count("clipart-menu-add-to-project")).clicked() {
        picked = Some(Picked::AddToProject(files.clone()));
    }
    // only 3D clipart becomes components, which is what a level holds
    if takes_a_level(&files) {
        ui.menu_button(count("clipart-menu-add-to-level"), |ui| {
            for (id, level) in tile.levels {
                if ui.button(level).clicked() {
                    picked = Some(Picked::AddToLevel(files.clone(), ToLevel::Existing(*id)));
                }
            }
            if ui
                .button(tr!("clipart-new-level"))
                .on_hover_hint(tr!("clipart-each-model-on-a-new"))
                .clicked()
            {
                picked = Some(Picked::AddToLevel(files.clone(), ToLevel::New));
            }
        });
    }
    ui.separator();
    if ui
        .add_enabled(!many, egui::Button::new(tr!("clipart-locate")))
        .on_hover_hint(tr!("clipart-shows-the-file-in-its"))
        .clicked()
    {
        picked = Some(Picked::Locate(tile.entry.file.clone()));
    }
    if ui.button(count("clipart-menu-add-tag")).clicked() {
        clipart.tagging = Some((files.clone(), String::new()));
    }
    let tags = clipart.tags.of_all(&files);
    if !tags.is_empty() {
        ui.menu_button(tr!("clipart-remove-tag"), |ui| {
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
    let delete = egui::RichText::new(count("clipart-menu-delete-from-disk"))
        .color(ui.visuals().error_fg_color);
    if ui
        .button(delete)
        .on_hover_hint(tr!("clipart-deletes-the-file-for-good"))
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
        .button(tr!("clipart-properties"))
        .on_hover_hint(tr!("clipart-its-size-date-tags-and"))
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
        let image = egui::Image::new(format!("file://{}", entry.file.display()));
        return (pictured(ui, image, size, chosen), None);
    }
    // a generated border's depth map is a model shown by its own picture
    let model = preview::MODELS.contains(&extension.as_str()) || entry.picture.is_some();
    let picture = BITMAPS.contains(&extension.as_str()) && entry.picture.is_none();
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
            (pictured(ui, egui::Image::new(texture), size, chosen), None)
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

/// A tile `size` across showing `image`: the picture shrunk by the button's padding, so the
/// tile is no wider than the room for it and its name (cut to fit) keeps clear of the next.
fn pictured(
    ui: &mut egui::Ui,
    image: egui::Image,
    size: egui::Vec2,
    chosen: bool,
) -> egui::Response {
    let inner = (size - 2.0 * ui.spacing().button_padding).max(egui::Vec2::ZERO);
    let button = egui::Button::image(image.fit_to_exact_size(inner)).selected(chosen);
    ui.add_sized(size, button)
}

#[cfg(test)]
mod tests;
