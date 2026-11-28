//! The file-type filters on every open, import and save dialog, in one place.
//!
//! The rule: a dialog's first filter (the one the system picks when it opens) is exactly
//! what that action reads or writes. Importing a model starts on 3D models, Open Job on job
//! files, and so on. Dialogs that take several formats then list each format on its own,
//! and the general import dialogs end with "All supported files" for anything the app can
//! import. Nothing starts on a type the action does not want (it used to be the cut list's
//! CSV, the first importer registered).
//!
//! The filters are worked out here without opening anything, so tests check every dialog.

use cut_craft_core::Registry;
use cut_craft_core::file::{JOB_EXTENSION, JOB_EXTENSIONS, OLD_JOB_EXTENSION, TEMPLATE_EXTENSION};

/// One entry in a dialog's file-type list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Filter {
    pub label: String,
    /// Without the dot; `*` matches every file.
    pub extensions: Vec<String>,
}

impl Filter {
    fn new(label: &str, extensions: &[&str]) -> Self {
        Self {
            label: labelled(label, extensions),
            extensions: extensions.iter().map(|e| (*e).to_owned()).collect(),
        }
    }
}

/// A label with its extensions shown, so the person sees what each choice lets through on
/// every system (GNOME's portal shows only the name): "3D models (.stl, .obj, .ccclip)".
fn labelled(label: &str, extensions: &[&str]) -> String {
    if extensions.contains(&"*") {
        return label.to_owned();
    }
    let list: Vec<_> = extensions.iter().map(|e| format!(".{e}")).collect();
    format!("{label} ({})", list.join(", "))
}

/// Importers that make vectors, in the order they are offered.
pub const VECTOR_IMPORTERS: [&str; 3] = ["import_dxf", "import_svg", "import_crv3d"];
/// Importers that bring in a picture.
pub const PICTURE_IMPORTERS: [&str; 1] = ["import_bitmap"];
/// Importers that make components; Import Component / 3D Model offers these.
pub const MODEL_IMPORTERS: [&str; 2] = ["import_stl", "import_3d_clipart"];
/// Export Vectors: the vector formats, DXF first since it is what a file with no
/// extension is saved as.
pub const VECTOR_EXPORTERS: [&str; 5] = [
    "export_dxf",
    "export_svg",
    "export_pdf",
    "export_eps",
    "export_ai",
];

/// The G-code file types Import Toolpaths reads (and the machine's files offer first).
pub const GCODE_EXTENSIONS: [&str; 8] = ["nc", "tap", "gcode", "ngc", "cnc", "txt", "gc", "nc1"];
/// Pictures the bug report and the job sheet's logo take.
const PICTURE_EXTENSIONS: [&str; 5] = ["png", "jpg", "jpeg", "gif", "bmp"];

/// Every file dialog in the app. Add a variant (and a line in [`Dialog::ALL`]) for a new one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dialog<'a> {
    OpenJob,
    SaveJob,
    JobTemplate,
    ImportVectorsOrPicture,
    ImportModel,
    /// The Import Vectors icon beside the menus.
    ImportVectors,
    ImportIntoShape,
    ImportCutList,
    SegmentModel,
    ImportToolpaths,
    InstallPostProcessor,
    MachinePackage,
    OpenToolDatabase,
    SaveToolDatabase,
    SaveShadedImage,
    SculptingBrush,
    Texture,
    ToolpathTemplate,
    FormSettings,
    /// One exporter, by id.
    Export(&'a str),
    ExportVectors,
    /// Save Toolpaths: the post's name and its file extension.
    SaveToolpaths {
        post: &'a str,
        extension: &'a str,
    },
    JobSheet,
    SaveStl,
    BugReportPictures,
    JobSheetLogo,
    /// Upload on the machine's files window.
    UploadToMachine,
    /// Help ▸ Back Up Settings' Save Backup….
    SaveBackup,
}

#[cfg(test)]
impl Dialog<'static> {
    /// Every dialog, for the tests; exporters are covered by [`Dialog::Export`] on each one.
    pub const ALL: [Dialog<'static>; 27] = [
        Dialog::OpenJob,
        Dialog::SaveJob,
        Dialog::JobTemplate,
        Dialog::ImportVectorsOrPicture,
        Dialog::ImportModel,
        Dialog::ImportVectors,
        Dialog::ImportIntoShape,
        Dialog::ImportCutList,
        Dialog::SegmentModel,
        Dialog::ImportToolpaths,
        Dialog::InstallPostProcessor,
        Dialog::MachinePackage,
        Dialog::OpenToolDatabase,
        Dialog::SaveToolDatabase,
        Dialog::SaveShadedImage,
        Dialog::SculptingBrush,
        Dialog::Texture,
        Dialog::ToolpathTemplate,
        Dialog::FormSettings,
        Dialog::ExportVectors,
        Dialog::SaveToolpaths {
            post: "GRBL (mm)",
            extension: "nc",
        },
        Dialog::JobSheet,
        Dialog::SaveStl,
        Dialog::BugReportPictures,
        Dialog::JobSheetLogo,
        Dialog::UploadToMachine,
        Dialog::SaveBackup,
    ];
}

/// The filters for `dialog`, the default first.
pub fn filters(dialog: Dialog<'_>, registry: &Registry) -> Vec<Filter> {
    let importers = |ids: &[&str]| -> Vec<Filter> {
        ids.iter()
            .filter_map(|id| registry.importer(id))
            .map(|i| Filter::new(i.format.label, i.format.extensions))
            .collect()
    };
    match dialog {
        Dialog::OpenJob => vec![Filter::new("Cut Craft job", &JOB_EXTENSIONS)],
        Dialog::SaveJob => {
            vec![Filter::new(
                "PowerWhittle job",
                &[JOB_EXTENSION, OLD_JOB_EXTENSION],
            )]
        }
        Dialog::JobTemplate => vec![Filter::new(
            "PowerWhittle job template",
            &[TEMPLATE_EXTENSION],
        )],
        Dialog::ImportVectorsOrPicture => {
            let ids: Vec<_> = VECTOR_IMPORTERS
                .iter()
                .chain(&PICTURE_IMPORTERS)
                .copied()
                .collect();
            choices("Vectors and pictures", importers(&ids), registry)
        }
        Dialog::ImportModel => choices("3D models", importers(&MODEL_IMPORTERS), registry),
        Dialog::ImportVectors => choices("Vectors", importers(&VECTOR_IMPORTERS), registry),
        Dialog::ImportIntoShape => {
            // fitted into a shape: drawings and pictures only, so no "all"
            let ids: Vec<_> = VECTOR_IMPORTERS
                .iter()
                .chain(&PICTURE_IMPORTERS)
                .copied()
                .collect();
            let each = importers(&ids);
            let mut out = vec![combined("Vectors and pictures", &each)];
            out.extend(each);
            out
        }
        Dialog::ImportCutList => importers(&["import_cut_list"]),
        Dialog::SegmentModel => vec![Filter::new("STL or OBJ model", &["stl", "obj"])],
        Dialog::ImportToolpaths => vec![Filter::new("G-code", &GCODE_EXTENSIONS)],
        Dialog::InstallPostProcessor => vec![Filter::new("Post-processor", &["pp"])],
        Dialog::MachinePackage => vec![Filter::new("Machine package", &["json"])],
        Dialog::OpenToolDatabase => vec![Filter::new(
            "Bit libraries",
            &crate::files::TOOL_FILE_EXTENSIONS,
        )],
        Dialog::SaveToolDatabase => vec![Filter::new("PowerWhittle bit library", &["json"])],
        Dialog::SaveShadedImage => vec![Filter::new("PNG image", &["png"])],
        Dialog::SculptingBrush => vec![Filter::new("Sculpting brush", &["ccclip"])],
        Dialog::Texture => vec![Filter::new(
            "Texture settings",
            &[cmd_texture_toolpath::TEXTURE_EXTENSION],
        )],
        Dialog::ToolpathTemplate => vec![Filter::new(
            "Toolpath template",
            &[crate::files::TOOLPATH_TEMPLATE_EXTENSION],
        )],
        Dialog::FormSettings => vec![Filter::new(
            "Toolpath settings",
            &[crate::files::FORM_SETTINGS_EXTENSION],
        )],
        Dialog::Export(id) => registry
            .exporter(id)
            .map_or_default(|e| vec![Filter::new(e.format.label, e.format.extensions)]),
        Dialog::ExportVectors => VECTOR_EXPORTERS
            .iter()
            .filter_map(|id| registry.exporter(id))
            .map(|e| Filter::new(e.format.label, e.format.extensions))
            .collect(),
        Dialog::SaveToolpaths { post, extension } => vec![Filter::new(post, &[extension])],
        Dialog::JobSheet => vec![Filter::new("Job sheet", &["html"])],
        Dialog::SaveStl => vec![Filter::new("STL", &["stl"])],
        Dialog::BugReportPictures => vec![Filter::new("Pictures", &PICTURE_EXTENSIONS)],
        Dialog::JobSheetLogo => vec![Filter::new("Logo", &["png", "jpg", "jpeg", "gif"])],
        Dialog::UploadToMachine => vec![
            Filter::new("G-code programs", &GCODE_EXTENSIONS),
            Filter::new("All files", &["*"]),
        ],
        Dialog::SaveBackup => vec![Filter::new(
            "PowerWhittle backup",
            &[cut_craft_backup::EXTENSION],
        )],
    }
}

/// A general import: everything the action takes first, each format on its own next, and
/// every format the app imports last.
fn choices(label: &str, each: Vec<Filter>, registry: &Registry) -> Vec<Filter> {
    let mut out = vec![combined(label, &each)];
    if each.len() > 1 {
        out.extend(each);
    }
    let all: Vec<_> = registry
        .importers()
        .flat_map(|i| i.format.extensions.iter().copied())
        .collect();
    out.push(combined_from("All supported files", &all));
    out
}

/// One filter letting through every extension of `each`.
fn combined(label: &str, each: &[Filter]) -> Filter {
    let all: Vec<&str> = each
        .iter()
        .flat_map(|f| f.extensions.iter().map(String::as_str))
        .collect();
    combined_from(label, &all)
}

fn combined_from(label: &str, extensions: &[&str]) -> Filter {
    let mut unique: Vec<&str> = Vec::new();
    for e in extensions {
        if !unique.contains(e) {
            unique.push(e);
        }
    }
    Filter::new(label, &unique)
}

/// The extensions as handed to the system dialog. Linux file dialogs (GTK and the desktop
/// portal) match patterns case-sensitively, so `Part.STL` would be hidden by `*.stl`; there
/// the upper-case spelling is added too. Windows and macOS ignore case already.
pub fn system_extensions(filter: &Filter, case_sensitive: bool) -> Vec<String> {
    let mut out = filter.extensions.clone();
    if case_sensitive {
        for e in &filter.extensions {
            let upper = e.to_uppercase();
            if !out.contains(&upper) {
                out.push(upper);
            }
        }
    }
    out
}

/// Whether this system's file dialogs match extensions case-sensitively.
pub const CASE_SENSITIVE_DIALOGS: bool = cfg!(not(any(windows, target_os = "macos")));

/// `dialog` with `which`'s filters added, the default first.
pub fn apply(
    mut dialog: rfd::FileDialog,
    which: Dialog<'_>,
    registry: &Registry,
) -> rfd::FileDialog {
    for filter in filters(which, registry) {
        dialog = dialog.add_filter(
            &filter.label,
            &system_extensions(&filter, CASE_SENSITIVE_DIALOGS),
        );
    }
    dialog
}

#[cfg(test)]
mod tests {
    use super::*;

    fn registry() -> Registry {
        crate::commands::registry()
    }

    fn first(dialog: Dialog<'_>) -> Filter {
        filters(dialog, &registry())
            .into_iter()
            .next()
            .unwrap_or_else(|| panic!("{dialog:?} has no filters"))
    }

    fn exts(f: &Filter) -> Vec<&str> {
        f.extensions.iter().map(String::as_str).collect()
    }

    #[test]
    fn every_dialog_starts_on_the_type_its_action_wants() {
        let expected: Vec<(Dialog<'static>, &[&str])> = vec![
            (Dialog::OpenJob, &["ccjob", "pwjob", "ajob"]),
            (Dialog::SaveJob, &["ccjob", "ajob"]),
            (Dialog::JobTemplate, &["ccjobt"]),
            (
                Dialog::ImportVectorsOrPicture,
                &[
                    "dxf", "svg", "crv3d", "crv", "png", "jpg", "jpeg", "gif", "bmp", "tif", "tiff",
                ],
            ),
            (Dialog::ImportModel, &["stl", "obj", "ccclip"]),
            (Dialog::ImportVectors, &["dxf", "svg", "crv3d", "crv"]),
            (
                Dialog::ImportIntoShape,
                &[
                    "dxf", "svg", "crv3d", "crv", "png", "jpg", "jpeg", "gif", "bmp", "tif", "tiff",
                ],
            ),
            (Dialog::ImportCutList, &["csv"]),
            (Dialog::SegmentModel, &["stl", "obj"]),
            (Dialog::ImportToolpaths, &GCODE_EXTENSIONS),
            (Dialog::InstallPostProcessor, &["pp"]),
            (Dialog::MachinePackage, &["json"]),
            (Dialog::OpenToolDatabase, &["vtdb", "tools", "json"]),
            (Dialog::SaveToolDatabase, &["json"]),
            (Dialog::SaveShadedImage, &["png"]),
            (Dialog::SculptingBrush, &["ccclip"]),
            (Dialog::Texture, &["vtexture"]),
            (Dialog::ToolpathTemplate, &["cctpt"]),
            (Dialog::FormSettings, &["ccsettings"]),
            (Dialog::ExportVectors, &["dxf"]),
            (
                Dialog::SaveToolpaths {
                    post: "GRBL (mm)",
                    extension: "nc",
                },
                &["nc"],
            ),
            (Dialog::JobSheet, &["html"]),
            (Dialog::SaveStl, &["stl"]),
            (
                Dialog::BugReportPictures,
                &["png", "jpg", "jpeg", "gif", "bmp"],
            ),
            (Dialog::JobSheetLogo, &["png", "jpg", "jpeg", "gif"]),
            (Dialog::UploadToMachine, &GCODE_EXTENSIONS),
            (Dialog::SaveBackup, &["ccbackup"]),
        ];
        assert_eq!(
            expected.len(),
            Dialog::ALL.len(),
            "every dialog in Dialog::ALL needs an expected default here"
        );
        for (dialog, want) in expected {
            assert!(Dialog::ALL.contains(&dialog), "{dialog:?} missing from ALL");
            assert_eq!(exts(&first(dialog)), want, "{dialog:?} default filter");
        }
    }

    #[test]
    fn only_the_cut_list_starts_on_csv() {
        for dialog in Dialog::ALL {
            if dialog != Dialog::ImportCutList {
                assert!(
                    !exts(&first(dialog)).contains(&"csv"),
                    "{dialog:?} starts on CSV"
                );
            }
        }
    }

    #[test]
    fn importing_a_model_offers_only_3d_types_first() {
        let all = filters(Dialog::ImportModel, &registry());
        assert!(all[0].label.starts_with("3D models"), "{}", all[0].label);
        // each model format on its own next, then everything the app imports
        assert_eq!(all.len(), 4, "{all:?}");
        assert_eq!(exts(&all[1]), ["stl", "obj"]);
        assert_eq!(exts(&all[2]), ["ccclip"]);
        assert!(all[3].label.starts_with("All supported files"));
    }

    #[test]
    fn all_supported_files_is_the_last_choice_and_holds_every_importer() {
        let registry = registry();
        for dialog in [
            Dialog::ImportVectorsOrPicture,
            Dialog::ImportModel,
            Dialog::ImportVectors,
        ] {
            let all = filters(dialog, &registry);
            let last = all.last().unwrap();
            assert!(last.label.starts_with("All supported files"), "{dialog:?}");
            assert!(all.len() >= 2, "{dialog:?} needs its own types before all");
            for importer in registry.importers() {
                for e in importer.format.extensions {
                    assert!(last.extensions.iter().any(|x| x == e), "{dialog:?} {e}");
                }
            }
        }
    }

    #[test]
    fn every_exporter_starts_on_its_own_format() {
        let registry = registry();
        for exporter in registry.exporters() {
            let f = filters(Dialog::Export(exporter.id), &registry);
            assert_eq!(f.len(), 1, "{}", exporter.id);
            assert_eq!(exts(&f[0]), exporter.format.extensions, "{}", exporter.id);
        }
    }

    #[test]
    fn every_filter_has_a_label_and_extensions() {
        let registry = registry();
        for dialog in Dialog::ALL {
            for f in filters(dialog, &registry) {
                assert!(!f.label.is_empty(), "{dialog:?}");
                assert!(!f.extensions.is_empty(), "{dialog:?} {}", f.label);
                assert!(
                    f.extensions.iter().all(|e| !e.starts_with('.')),
                    "{dialog:?}: extensions go without the dot"
                );
            }
        }
    }

    #[test]
    fn labels_show_the_extensions() {
        assert_eq!(
            first(Dialog::ImportModel).label,
            "3D models (.stl, .obj, .ccclip)"
        );
        assert_eq!(
            filters(Dialog::UploadToMachine, &registry())[1].label,
            "All files"
        );
    }

    #[test]
    fn linux_dialogs_also_match_upper_case_extensions() {
        let f = Filter::new("STL", &["stl", "obj"]);
        assert_eq!(system_extensions(&f, true), ["stl", "obj", "STL", "OBJ"]);
        assert_eq!(system_extensions(&f, false), ["stl", "obj"]);
        let all = Filter::new("All files", &["*"]);
        assert_eq!(system_extensions(&all, true), ["*"]);
    }

    /// New dialogs go through [`filters`]: no `add_filter` anywhere else in the app.
    #[test]
    fn no_dialog_adds_its_own_filters() {
        let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut offenders = Vec::new();
        let mut stack = vec![src];
        while let Some(dir) = stack.pop() {
            for entry in std::fs::read_dir(&dir).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    stack.push(path);
                } else if path.extension().is_some_and(|e| e == "rs")
                    && !path.ends_with("file_filters.rs")
                {
                    let text = std::fs::read_to_string(&path).unwrap();
                    if text.contains(".add_filter(") {
                        offenders.push(path.display().to_string());
                    }
                }
            }
        }
        assert!(
            offenders.is_empty(),
            "use file_filters::filters for these dialogs: {offenders:?}"
        );
    }
}
