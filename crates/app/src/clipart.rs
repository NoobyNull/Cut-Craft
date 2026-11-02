//! The Library tab: folders of the user's own clipart, and what is in the chosen one as
//! tiles: pictures as themselves, 3D models as previews, drawings by their kind. Tiles are
//! picked with a click (Ctrl or Shift for more), added to the project with a double-click
//! or by dragging, and a right-click menu adds, locates, tags or deletes the ones picked.

mod preview;
mod tags;

use cut_craft_bug_report::hint::HoverHint;
use eframe::egui;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

/// A thumbnail's edge, in rows of the theme's interactive height.
// ponytail: one size; Aspire's three view sizes can come with a setting
const THUMBNAIL_ROWS: f32 = 4.0;
/// Files shown as their own picture.
const PICTURES: [&str; 6] = ["png", "jpg", "jpeg", "gif", "bmp", "svg"];
/// Pictures made of pixels (a depth map is one of these).
const BITMAPS: [&str; 7] = ["png", "jpg", "jpeg", "gif", "bmp", "tif", "tiff"];
/// Vectric's clipart, which is locked to Vectric's own software: counted, not shown.
const LOCKED: [&str; 2] = ["3dclip", "v3m"];
/// How often the chosen folder is read again, for files added or taken away meanwhile.
const RELIST: std::time::Duration = std::time::Duration::from_secs(2);

/// The library's folders, kept between runs, and the one being looked in.
#[derive(Default)]
pub struct Clipart {
    pub folders: Vec<PathBuf>,
    pub chosen: Option<usize>,
    /// Where the folders are kept; `None` keeps them only for this run.
    store: Option<PathBuf>,
    /// Models' previews, made in the background.
    previews: preview::Previews,
    /// The chosen folder as last read, and when.
    listing: Option<(PathBuf, std::time::Instant, Listing)>,
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
    /// Whether each picture is greyscale (a depth map), by the [`preview::stamp`] it had.
    greys: HashMap<PathBuf, (u64, bool)>,
    /// The last thing that went wrong, until the next action.
    trouble: Option<String>,
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

/// What is in a folder.
#[derive(Debug, Default, PartialEq)]
struct Listing {
    entries: Vec<Entry>,
    /// Vectric clipart files, which can't be opened.
    locked: usize,
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
    /// Show the chosen folder in the desktop's file manager.
    OpenFolder,
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
        let store = crate::files::clipart_folders_path();
        let folders = store
            .as_deref()
            .map(crate::files::load_recent)
            .unwrap_or_default();
        let cache = crate::files::clipart_previews_path();
        if let Some(cache) = cache.clone() {
            let _ = std::thread::Builder::new()
                .name("clipart preview pruning".into())
                .spawn(move || preview::prune(&cache, preview::UNUSED));
        }
        Self {
            folders,
            store,
            previews: preview::Previews::new(cache),
            tags: tags::Tags::load(crate::files::library_tags_path()),
            ..Self::default()
        }
    }

    /// Adds `folder` (once) and looks in it.
    pub fn add(&mut self, folder: PathBuf) -> Result<(), String> {
        let at = self.folders.iter().position(|f| *f == folder);
        self.chosen = Some(at.unwrap_or_else(|| {
            self.folders.push(folder);
            self.folders.len() - 1
        }));
        self.save()
    }

    /// Takes the chosen folder out of the library (the folder itself stays on disk).
    pub fn remove_chosen(&mut self) -> Result<(), String> {
        if let Some(at) = self.chosen.take().filter(|&at| at < self.folders.len()) {
            self.folders.remove(at);
        }
        self.save()
    }

    fn save(&self) -> Result<(), String> {
        match &self.store {
            Some(path) => crate::files::save_recent(&self.folders, path),
            None => Ok(()),
        }
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
        self.listing = None; // read the folder again
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

/// The files in `folder` that something imports, by name; a model's picture goes with it.
/// `greys` remembers which pictures are greyscale, so each is read once.
fn list(
    folder: &Path,
    importable: &dyn Fn(&str) -> bool,
    greys: &mut HashMap<PathBuf, (u64, bool)>,
) -> Listing {
    let Ok(read) = std::fs::read_dir(folder) else {
        return Listing::default();
    };
    let mut locked = 0;
    let mut files: Vec<PathBuf> = read
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_file())
        .filter(|p| {
            let extension = preview::extension(p);
            locked += usize::from(LOCKED.contains(&extension.as_str()));
            importable(&extension)
        })
        .collect();
    files.sort();
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
    Listing { entries, locked }
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
    importable: &dyn Fn(&str) -> bool,
    levels: &[(u64, String)],
) -> Option<Picked> {
    let mut picked = folders(ui, clipart);
    let Some(folder) = clipart.chosen.and_then(|i| clipart.folders.get(i)).cloned() else {
        return picked;
    };
    clipart.previews.update(ui.ctx());
    let moved = (clipart.listing.as_ref()).is_some_and(|(f, ..)| *f != folder);
    if moved {
        clipart.previews.forget_waiting();
        clipart.selected.clear();
        clipart.anchor = None;
    }
    let stale = (clipart.listing.as_ref()).is_none_or(|(_, read, _)| read.elapsed() > RELIST);
    if moved || stale {
        let read = list(&folder, importable, &mut clipart.greys);
        // files gone since (deleted or moved elsewhere) are no longer picked
        (clipart.selected).retain(|f| read.entries.iter().any(|e| e.file == *f));
        clipart.listing = Some((folder.clone(), std::time::Instant::now(), read));
    }
    if !folder.is_dir() {
        ui.colored_label(
            ui.visuals().warn_fg_color,
            format!(
                "{} isn't there. If it's on a drive that isn't plugged in, plug it in; if it \
                 was moved, use Remove Folder and add it again from where it is now.",
                folder.display()
            ),
        );
        return picked;
    }
    // taken out while drawn, so the tiles can change the rest of the tab
    let Some((folder, read, listing)) = clipart.listing.take() else {
        return picked;
    };
    filters(ui, clipart, &listing);
    let wanted = clipart.search.trim().to_lowercase();
    let shown: Vec<&Entry> = (listing.entries.iter())
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
    if shown.is_empty() {
        ui.weak(if listing.entries.is_empty() {
            "Nothing in this folder yet. 3D models (STL, OBJ, .ccclip), pictures and drawings \
             (SVG, DXF) show up here."
                .to_owned()
        } else {
            "Nothing here matches. Clear the search or choose Everything.".to_owned()
        });
    }
    let order: Vec<&Path> = shown.iter().map(|e| e.file.as_path()).collect();
    let pictures: HashMap<&Path, &Path> = (listing.entries.iter())
        .filter_map(|e| Some((e.file.as_path(), e.picture.as_deref()?)))
        .collect();
    let edge = ui.spacing().interact_size.y * THUMBNAIL_ROWS;
    ui.horizontal_wrapped(|ui| {
        for entry in &shown {
            // sized up front, so the wrapped row knows when it is full (`vertical` doesn't)
            let cell = egui::vec2(edge, edge + ui.spacing().interact_size.y);
            ui.allocate_ui_with_layout(cell, egui::Layout::top_down(egui::Align::Min), |ui| {
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
        }
    });
    if clipart.listing.is_none() {
        clipart.listing = Some((folder, read, listing));
    }
    picked
}

/// The library's folders, with Add, Remove and Open Folder.
fn folders(ui: &mut egui::Ui, clipart: &mut Clipart) -> Option<Picked> {
    let mut picked = None;
    ui.strong("Folders");
    for (i, folder) in clipart.folders.iter().enumerate() {
        let there = folder.is_dir();
        let label = if there {
            egui::RichText::new(name(folder))
        } else {
            egui::RichText::new(format!("{} (not found)", name(folder))).weak()
        };
        let row = ui
            .selectable_label(clipart.chosen == Some(i), label)
            .on_hover_hint(folder.display().to_string());
        if row.clicked() {
            clipart.chosen = Some(i);
        }
    }
    if clipart.folders.is_empty() {
        ui.weak("No folders yet. Add a folder of your models and pictures to see them here.");
    }
    let mut remove = false;
    ui.horizontal(|ui| {
        if ui.button("Add Folder...").clicked() {
            picked = Some(Picked::AddFolder);
        }
        remove = ui
            .add_enabled(clipart.chosen.is_some(), egui::Button::new("Remove Folder"))
            .on_hover_hint("Takes the folder out of the Library; its files stay on disk.")
            .clicked();
        if ui
            .add_enabled(clipart.chosen.is_some(), egui::Button::new("Open Folder"))
            .clicked()
        {
            picked = Some(Picked::OpenFolder);
        }
    });
    if remove && let Err(e) = clipart.remove_chosen() {
        clipart.trouble = Some(e);
    }
    ui.separator();
    picked
}

/// Search, the Show filter, what is picked, Add Tag's box, and anything gone wrong.
fn filters(ui: &mut egui::Ui, clipart: &mut Clipart, listing: &Listing) {
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
    if listing.locked > 0 {
        ui.label(
            egui::RichText::new(format!(
                "{} Vectric clipart file{} here can't be shown. Vectric locks .3dclip and .v3m \
                 files to its own software. If you have Aspire, export them as STL to use them \
                 here.",
                listing.locked,
                if listing.locked == 1 { "" } else { "s" },
            ))
            .weak(),
        );
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
    thumbnail.context_menu(|ui| {
        if let Some(p) = menu(ui, clipart, tile) {
            picked = Some(p);
        }
    });
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
    if PICTURES.contains(&extension.as_str()) {
        let image =
            egui::Image::new(format!("file://{}", entry.file.display())).fit_to_exact_size(size);
        return (ui.add(egui::Button::image(image).selected(chosen)), None);
    }
    let model = preview::MODELS.contains(&extension.as_str());
    let on_screen = ui.is_rect_visible(egui::Rect::from_min_size(ui.cursor().min, size));
    if !model || !on_screen {
        let kind = if model {
            "3D".into()
        } else {
            extension.to_uppercase()
        };
        let button = egui::Button::new(kind).selected(chosen);
        return (ui.add_sized(size, button), None);
    }
    match previews.get(&entry.file, entry.picture.as_deref(), entry.stamp) {
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
            let button = egui::Button::new("3D").selected(chosen);
            (ui.add_sized(size, button), Some(e))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A folder's clipart wraps onto rows in a narrow panel, not one long line (#34).
    #[test]
    fn thumbnails_wrap() {
        let dir =
            std::env::temp_dir().join(format!("cut-craft-clipart-wrap-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        for i in 0..8 {
            std::fs::write(dir.join(format!("{i}.stl")), "").unwrap();
        }
        let mut clipart = Clipart {
            folders: vec![dir.clone()],
            chosen: Some(0),
            ..Clipart::default()
        };
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
                panel(ui, &mut clipart, &|e| e == "stl", &[]);
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
        let importable = |e: &str| ["stl", "obj", "png", "jpg"].contains(&e);
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
                locked: 2,
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
        let mut clipart = Clipart {
            folders: vec![dir.clone()],
            chosen: Some(0),
            ..Clipart::default()
        };
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
                    panel(ui, &mut clipart, &|e| e == "stl", &[]);
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
        let listing = list(&dir, &|_| true, &mut clipart.greys);
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
}
