//! The file and message dialogs, each starting in its own folder and kept over the window.

use super::{APP_NAME, Dialog, files};

/// Job templates, starting in Cut Craft's `templates` folder.
/// A file dialog opening in one of the user's settings folders (made if need be).
pub(super) fn settings_dialog(which: Dialog<'_>, folder: &str) -> rfd::FileDialog {
    let dialog = filtered_dialog(which, &cut_craft_core::Registry::default());
    match files::config_dir().map(|d| d.join(folder)) {
        Some(dir) if std::fs::create_dir_all(&dir).is_ok() => dialog.set_directory(dir),
        _ => dialog,
    }
}

pub(super) fn template_dialog() -> rfd::FileDialog {
    settings_dialog(Dialog::JobTemplate, "templates")
}

/// Sculpting brushes, kept in the brushes folder.
pub(super) fn brush_dialog() -> rfd::FileDialog {
    settings_dialog(Dialog::SculptingBrush, "brushes")
}

/// Texture Toolpath's textures, kept in the textures folder.
pub(super) fn texture_dialog() -> rfd::FileDialog {
    settings_dialog(Dialog::Texture, "textures")
}

/// Toolpath templates, in the same `templates` folder.
pub(super) fn toolpath_template_dialog() -> rfd::FileDialog {
    settings_dialog(Dialog::ToolpathTemplate, "templates")
}

/// A toolpath form's saved settings, in the same `templates` folder.
pub(super) fn form_settings_dialog() -> rfd::FileDialog {
    settings_dialog(Dialog::FormSettings, "templates")
}

pub(super) fn job_dialog() -> rfd::FileDialog {
    filtered_dialog(Dialog::OpenJob, &cut_craft_core::Registry::default())
}

/// The system's file dialog with `which`'s file types, the one its action wants first
/// (see [`crate::file_filters`]).
pub(super) fn filtered_dialog(
    which: Dialog<'_>,
    registry: &cut_craft_core::Registry,
) -> rfd::FileDialog {
    crate::file_filters::apply(file_dialog(), which, registry)
}

/// The system's file dialog. Tests must never open one: it lands on the desktop of whoever
/// runs them and waits for an answer.
pub(super) fn file_dialog() -> rfd::FileDialog {
    on_the_desktop();
    let dialog = rfd::FileDialog::new();
    let dialog = match dialog_parent() {
        Some(parent) => dialog.set_parent(&*parent),
        None => dialog,
    };
    match DIALOG_FOLDER.lock().ok().and_then(|f| f.clone()) {
        Some(folder) => dialog.set_directory(folder),
        None => dialog,
    }
}

/// Where file dialogs open (Options ▸ File dialog default), kept up to date each frame; the
/// dialogs are opened from places that do not have the app to hand.
pub(super) static DIALOG_FOLDER: std::sync::Mutex<Option<std::path::PathBuf>> =
    std::sync::Mutex::new(None);

/// A message box from the system, as the window's own like the file dialogs.
pub(crate) fn message_dialog() -> rfd::MessageDialog {
    on_the_desktop();
    let dialog = rfd::MessageDialog::new();
    match dialog_parent() {
        Some(parent) => dialog.set_parent(&*parent),
        None => dialog,
    }
}

/// Where a save dialog's answer is saved: the name typed with one of `extensions` on its end
/// (a dot in the name stays), or `None` when that adds the extension, a file of that name is
/// there already and the user does not want it replaced (the dialog only asked about the
/// name typed).
pub(super) fn saving_path(
    dialog: &std::path::Path,
    extensions: &[&str],
) -> Option<std::path::PathBuf> {
    let named = files::named_for_saving(dialog, extensions);
    if !files::replaces_unasked(dialog, &named) {
        return Some(named);
    }
    let answer = message_dialog()
        .set_title(APP_NAME)
        .set_level(rfd::MessageLevel::Warning)
        .set_description(format!(
            "{} already exists. Replace it?",
            named.file_name().map_or_default(|n| n.to_string_lossy())
        ))
        .set_buttons(rfd::MessageButtons::OkCancelCustom(
            "Replace".into(),
            "Cancel".into(),
        ))
        .show();
    let replace = matches!(&answer, rfd::MessageDialogResult::Custom(c) if c == "Replace")
        || answer == rfd::MessageDialogResult::Ok;
    replace.then_some(named)
}

/// Cut Craft's window, kept up to date each frame: the system's dialogs open as its own,
/// kept above it and brought forward with it, rather than as windows of their own.
pub(super) static DIALOG_PARENT: std::sync::Mutex<Option<std::sync::Arc<dyn Window>>> =
    std::sync::Mutex::new(None);

/// A window a dialog can belong to (eframe's is winit's).
pub(super) trait Window:
    raw_window_handle::HasWindowHandle + raw_window_handle::HasDisplayHandle + Send + Sync
{
}

impl<W: raw_window_handle::HasWindowHandle + raw_window_handle::HasDisplayHandle + Send + Sync>
    Window for W
{
}

/// The window a file dialog or message box opened now belongs to: the dialog window that has
/// the keyboard (its button opened it), otherwise Cut Craft's main window. A file dialog that
/// belonged to the main window could open behind a dialog window kept above it.
pub(super) fn dialog_parent() -> Option<std::sync::Arc<dyn Window>> {
    if let Some(dialog) = cut_craft_window_owner::focused_dialog() {
        return Some(std::sync::Arc::new(dialog));
    }
    DIALOG_PARENT.lock().ok()?.clone()
}

/// Every window or program the app puts on the desktop passes here; in tests it fails.
pub(super) fn on_the_desktop() {
    #[cfg(test)]
    panic!("a test opened a real dialog or program: list the control in needs_a_person");
}
