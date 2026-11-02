//! The Clipart tab: a library of folders, and what is in the chosen one as thumbnails
//! (pictures) or names (other clipart). A double-click imports one into the job.

mod preview;

use cut_craft_bug_report::hint::HoverHint;
use eframe::egui;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

/// A thumbnail's edge, in rows of the theme's interactive height.
// ponytail: one size; Aspire's three view sizes can come with a setting
const THUMBNAIL_ROWS: f32 = 4.0;
/// Files shown as their own picture.
const PICTURES: [&str; 6] = ["png", "jpg", "jpeg", "gif", "bmp", "svg"];
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
    /// Only files whose names have this in them are shown.
    pub search: String,
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
}

/// What the tab asks the app to do.
pub enum Picked {
    AddFolder,
    /// Show the chosen folder in the desktop's file manager.
    OpenFolder,
    Import(PathBuf),
    /// Import to Level: onto one of the job's levels, or a new one.
    ImportToLevel(PathBuf, ToLevel),
}

/// Which level Import to Level puts clipart on.
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
        Self {
            folders,
            store,
            previews: preview::Previews::new(crate::files::clipart_previews_path()),
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
}

/// The files in `folder` that something imports, by name; a model's picture goes with it.
fn list(folder: &Path, importable: &dyn Fn(&str) -> bool) -> Listing {
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
        .map(|file| Entry {
            file: file.clone(),
            picture: picture_of(file).cloned(),
        })
        .collect();
    Listing { entries, locked }
}

fn name(path: &Path) -> String {
    path.file_name()
        .map_or_else(String::new, |n| n.to_string_lossy().into_owned())
}

/// Draws the tab; what was asked for, if anything. `levels` are the job's, by id and name,
/// for a thumbnail's Import to Level menu. A thumbnail dragged out carries its file, for the
/// views to import where it is dropped.
pub fn panel(
    ui: &mut egui::Ui,
    clipart: &mut Clipart,
    importable: &dyn Fn(&str) -> bool,
    levels: &[(u64, String)],
) -> Option<Picked> {
    let mut picked = None;
    ui.strong("Library");
    for (i, folder) in clipart.folders.iter().enumerate() {
        let row = ui
            .selectable_label(clipart.chosen == Some(i), name(folder))
            .on_hover_hint(folder.display().to_string());
        if row.clicked() {
            clipart.chosen = Some(i);
        }
    }
    if clipart.folders.is_empty() {
        ui.weak("No folders yet");
    }
    let mut remove = false;
    ui.horizontal(|ui| {
        if ui.button("Add Folder...").clicked() {
            picked = Some(Picked::AddFolder);
        }
        remove = ui
            .add_enabled(clipart.chosen.is_some(), egui::Button::new("Remove Folder"))
            .clicked();
        if ui
            .add_enabled(clipart.chosen.is_some(), egui::Button::new("Open Folder"))
            .clicked()
        {
            picked = Some(Picked::OpenFolder);
        }
    });
    if remove && let Err(e) = clipart.remove_chosen() {
        ui.colored_label(ui.visuals().error_fg_color, e);
    }
    ui.separator();
    let Some(folder) = clipart.chosen.and_then(|i| clipart.folders.get(i)).cloned() else {
        return picked;
    };
    let Clipart {
        previews,
        listing,
        search,
        ..
    } = clipart;
    previews.update(ui.ctx());
    let moved = listing.as_ref().is_some_and(|(f, ..)| *f != folder);
    if moved {
        previews.forget_waiting();
    }
    if moved
        || listing
            .as_ref()
            .is_none_or(|(_, read, _)| read.elapsed() > RELIST)
    {
        let read = list(&folder, importable);
        *listing = Some((folder, std::time::Instant::now(), read));
    }
    let Some((_, _, listing)) = listing.as_ref() else {
        return picked;
    };
    ui.add(
        egui::TextEdit::singleline(search)
            .hint_text("Search by name")
            .desired_width(f32::INFINITY),
    );
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
    let wanted = search.trim().to_lowercase();
    let shown: Vec<&Entry> = (listing.entries.iter())
        .filter(|e| wanted.is_empty() || name(&e.file).to_lowercase().contains(&wanted))
        .collect();
    if shown.is_empty() {
        ui.weak(if listing.entries.is_empty() {
            "No clipart in this folder. 3D models (STL, OBJ, .ccclip), pictures and drawings \
             (SVG, DXF) show up here."
                .to_owned()
        } else {
            format!("Nothing here has \"{}\" in its name.", search.trim())
        });
    }
    let edge = ui.spacing().interact_size.y * THUMBNAIL_ROWS;
    ui.horizontal_wrapped(|ui| {
        for entry in shown {
            // sized up front, so the wrapped row knows when it is full (`vertical` doesn't)
            let cell = egui::vec2(edge, edge + ui.spacing().interact_size.y);
            ui.allocate_ui_with_layout(cell, egui::Layout::top_down(egui::Align::Min), |ui| {
                if let Some(p) = clip(ui, previews, entry, edge, levels) {
                    picked = Some(p);
                }
            });
        }
    });
    picked
}

/// One file in the grid: its tile and name, to double-click, drag or right-click.
fn clip(
    ui: &mut egui::Ui,
    previews: &mut preview::Previews,
    entry: &Entry,
    edge: f32,
    levels: &[(u64, String)],
) -> Option<Picked> {
    let mut picked = None;
    let file = &entry.file;
    let label = name(file);
    ui.set_width(edge);
    let (thumbnail, trouble) = tile(ui, previews, entry, edge);
    let thumbnail = thumbnail.interact(egui::Sense::click_and_drag());
    thumbnail.dnd_set_drag_payload(file.clone());
    thumbnail.context_menu(|ui| {
        ui.menu_button("Import to Level", |ui| {
            for (id, level) in levels {
                if ui.button(level).clicked() {
                    picked = Some(Picked::ImportToLevel(file.clone(), ToLevel::Existing(*id)));
                }
            }
            if ui.button("New Level").clicked() {
                picked = Some(Picked::ImportToLevel(file.clone(), ToLevel::New));
            }
        });
    });
    let caption = ui.add(
        egui::Label::new(egui::RichText::new(&label).small())
            .truncate()
            .sense(egui::Sense::click_and_drag()),
    );
    caption.dnd_set_drag_payload(file.clone());
    if thumbnail.double_clicked() || caption.double_clicked() {
        picked = Some(Picked::Import(file.clone()));
    }
    let mut hint = format!("{label}\nDouble-click to add it to the job, or drag it onto a view.");
    if let Some(trouble) = trouble {
        hint = format!("{hint}\nNo preview: {trouble}");
    }
    thumbnail.on_hover_hint(hint);
    picked
}

/// A file's tile: its picture, its model's preview (asked for only once it is on screen),
/// or what kind of file it is; and why a model has no preview, if it hasn't.
fn tile(
    ui: &mut egui::Ui,
    previews: &mut preview::Previews,
    entry: &Entry,
    edge: f32,
) -> (egui::Response, Option<String>) {
    let size = egui::vec2(edge, edge);
    let extension = preview::extension(&entry.file);
    if PICTURES.contains(&extension.as_str()) {
        let image =
            egui::Image::new(format!("file://{}", entry.file.display())).fit_to_exact_size(size);
        return (ui.add(egui::Button::image(image)), None);
    }
    let model = preview::MODELS.contains(&extension.as_str());
    let on_screen = ui.is_rect_visible(egui::Rect::from_min_size(ui.cursor().min, size));
    if !model || !on_screen {
        let kind = if model {
            "3D".into()
        } else {
            extension.to_uppercase()
        };
        return (ui.add_sized(size, egui::Button::new(kind)), None);
    }
    match previews.get(&entry.file, entry.picture.as_deref()) {
        preview::Preview::Ready(texture) => {
            let image = egui::Image::new(texture).fit_to_exact_size(size);
            (ui.add(egui::Button::image(image)), None)
        }
        preview::Preview::Waiting => {
            let button = ui.add_sized(size, egui::Button::new(""));
            egui::Spinner::new().paint_at(ui, button.rect.shrink(edge / 3.0));
            (button, None)
        }
        preview::Preview::Failed(e) => {
            let e = e.to_owned();
            (ui.add_sized(size, egui::Button::new("3D")), Some(e))
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
        let listing = list(&dir, &importable);
        let entry = |file: &str, picture: Option<&str>| Entry {
            file: dir.join(file),
            picture: picture.map(|p| dir.join(p)),
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
}
