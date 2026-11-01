//! The Clipart tab: a library of folders, and what is in the chosen one as thumbnails
//! (pictures) or names (other clipart). A double-click imports one into the job.

use cut_craft_bug_report::hint::HoverHint;
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
    let Some(folder) = clipart.chosen.and_then(|i| clipart.folders.get(i)) else {
        return picked;
    };
    let edge = ui.spacing().interact_size.y * THUMBNAIL_ROWS;
    ui.horizontal_wrapped(|ui| {
        for file in entries(folder, importable) {
            let label = name(&file);
            // sized up front, so the wrapped row knows when it is full (`vertical` doesn't)
            let cell = egui::vec2(edge, edge + ui.spacing().interact_size.y);
            ui.allocate_ui_with_layout(cell, egui::Layout::top_down(egui::Align::Min), |ui| {
                ui.set_width(edge);
                let picture = file.extension().is_some_and(|e| {
                    PICTURES.contains(&e.to_string_lossy().to_lowercase().as_str())
                });
                let thumbnail = if picture {
                    let image = egui::Image::new(format!("file://{}", file.display()))
                        .fit_to_exact_size(egui::vec2(edge, edge));
                    ui.add(egui::Button::image(image))
                } else {
                    ui.add_sized(egui::vec2(edge, edge), egui::Button::new("3D"))
                }
                .interact(egui::Sense::click_and_drag());
                thumbnail.dnd_set_drag_payload(file.clone());
                thumbnail.context_menu(|ui| {
                    ui.menu_button("Import to Level", |ui| {
                        for (id, level) in levels {
                            if ui.button(level).clicked() {
                                picked = Some(Picked::ImportToLevel(
                                    file.clone(),
                                    ToLevel::Existing(*id),
                                ));
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
                thumbnail.on_hover_hint(&label);
            });
        }
    });
    picked
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
            store: None,
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
}
