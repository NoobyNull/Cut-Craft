//! The Clipart tab: a library of folders, and what is in the chosen one as thumbnails
//! (pictures) or names (other clipart). A double-click imports one into the job.

use eframe::egui;
use std::path::{Path, PathBuf};

/// A thumbnail's edge, in rows of the theme's interactive height.
// ponytail: one size; Aspire's three view sizes can come with a setting
const THUMBNAIL_ROWS: f32 = 4.0;
/// Files shown as their own picture.
const PICTURES: [&str; 6] = ["png", "jpg", "jpeg", "gif", "bmp", "svg"];

/// The library's folders, kept between runs, and the one being looked in.
#[derive(Default)]
pub struct Clipart {
    pub folders: Vec<PathBuf>,
    pub chosen: Option<usize>,
    /// Where the folders are kept; `None` keeps them only for this run.
    store: Option<PathBuf>,
}

/// What the tab asks the app to do.
pub enum Picked {
    AddFolder,
    /// Show the chosen folder in the desktop's file manager.
    OpenFolder,
    Import(PathBuf),
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
            chosen: None,
            store,
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

/// The files in `folder` that something imports, by name.
// ponytail: read every frame; cache by folder if large folders are slow
fn entries(folder: &Path, importable: &dyn Fn(&str) -> bool) -> Vec<PathBuf> {
    let Ok(read) = std::fs::read_dir(folder) else {
        return vec![];
    };
    let mut files: Vec<PathBuf> = read
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_file())
        .filter(|p| {
            p.extension()
                .is_some_and(|e| importable(&e.to_string_lossy().to_lowercase()))
        })
        .collect();
    files.sort();
    files
}

fn name(path: &Path) -> String {
    path.file_name()
        .map_or_else(String::new, |n| n.to_string_lossy().into_owned())
}

/// Draws the tab; what was asked for, if anything.
pub fn panel(
    ui: &mut egui::Ui,
    clipart: &mut Clipart,
    importable: &dyn Fn(&str) -> bool,
) -> Option<Picked> {
    let mut picked = None;
    ui.strong("Library");
    for (i, folder) in clipart.folders.iter().enumerate() {
        let row = ui
            .selectable_label(clipart.chosen == Some(i), name(folder))
            .on_hover_text(folder.display().to_string());
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
    let Some(folder) = clipart.chosen.and_then(|i| clipart.folders.get(i)) else {
        return picked;
    };
    let edge = ui.spacing().interact_size.y * THUMBNAIL_ROWS;
    ui.horizontal_wrapped(|ui| {
        for file in entries(folder, importable) {
            let label = name(&file);
            ui.vertical(|ui| {
                ui.set_width(edge);
                let picture = file.extension().is_some_and(|e| {
                    PICTURES.contains(&e.to_string_lossy().to_lowercase().as_str())
                });
                let thumbnail = if picture {
                    let image = egui::Image::new(format!("file://{}", file.display()))
                        .fit_to_exact_size(egui::vec2(edge, edge));
                    ui.add(egui::ImageButton::new(image))
                } else {
                    ui.add_sized(egui::vec2(edge, edge), egui::Button::new("3D"))
                };
                let caption = ui.add(
                    egui::Label::new(egui::RichText::new(&label).small())
                        .truncate(true)
                        .sense(egui::Sense::click()),
                );
                if thumbnail.double_clicked() || caption.double_clicked() {
                    picked = Some(Picked::Import(file.clone()));
                }
                thumbnail.on_hover_text(&label);
            });
        }
    });
    picked
}
