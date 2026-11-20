//! Setup Material (File Operations): Job Setup with only the job type and the material's
//! size, as a blank job needs; the rest of a job's settings come from Options ▸ Job
//! Defaults, and "More settings for this job…" shows them on the same form.

use cut_craft_core::document::{Datum, Job, ZZero};

use cut_craft_bug_report::hint::HoverHint;
use cut_craft_core::document::Document;
use serde_json::Value;

use super::CutCraftApp;

/// Options ▸ Job Defaults, under Classic Startup Flow: Job Setup's settings for the job new
/// jobs start with, with pictures and the Materials Library as on Job Setup's own form, and
/// why a change was refused in red under them. True when one changed.
pub(super) fn job_defaults_section(
    ui: &mut egui::Ui,
    fields: &[crate::form::Field],
    params: &mut Value,
    error: Option<&str>,
    icons: &crate::icons::Icons,
    materials: &cut_craft_core::material_library::MaterialLibrary,
) -> bool {
    let edge = super::SECTION_PICTURE_ROWS * ui.spacing().interact_size.y;
    let picture = |path: &str| {
        icons
            .find(path, "")
            .map(|p| icons.image(p).fit_to_exact_size(egui::vec2(edge, edge)))
    };
    let job = Job::default();
    let units = crate::form::units_chosen(params).unwrap_or(job.units);
    let empty = Document {
        job,
        ..Document::default()
    };
    let look = crate::form::Look {
        picture: &picture,
        materials: Some(materials),
        ..crate::form::Look::plain(units, crate::form::material(&empty, units))
    };
    ui.separator();
    ui.label(
        "Every new job starts with these. Changing them leaves the job that is open as it is: \
         its own are in Setup Material ▸ More settings for this job…",
    );
    let changed = crate::form::show(ui, fields, params, &look);
    if let Some(error) = error {
        ui.colored_label(ui.visuals().error_fg_color, error);
    }
    changed
}

/// The action, its button and its form's heading.
pub(super) const ID: &str = "setup_material";
pub(super) const LABEL: &str = "Setup Material";
/// Its links to the rest of the job's settings, for this job and for every new one.
pub(super) const MORE: &str = "More settings for this job…";
pub(super) const DEFAULTS: &str = "Change my defaults…";
/// The command whose form Setup Material is.
const JOB_SETUP: &str = "job_setup";
/// Job Setup's settings Setup Material shows: what kind of job and its size, and what only
/// one kind needs (how a double-sided board turns over, a rotary blank's axis and zero).
pub(super) const BRIEF: [&str; 5] = ["job_type", "size", "sides", "orientation", "rotary_z_zero"];

/// The material in one line: its size and kind, where Z zero and X0 Y0 are.
pub(super) fn summary(job: &Job) -> String {
    let units = job.units;
    let length = |mm: f64| super::trimmed(mm / units.per());
    let (size, kind) = match job.rotary {
        Some(r) => (
            format!(
                "{} {u} long, {} {u} across",
                length(match r.along {
                    cut_craft_core::document::Along::X => job.width,
                    cut_craft_core::document::Along::Y => job.height,
                }),
                length(r.diameter),
                u = units.suffix()
            ),
            "rotary",
        ),
        None => (
            format!(
                "{} × {} × {} {}",
                length(job.width),
                length(job.height),
                length(job.thickness),
                units.suffix()
            ),
            if job.double_sided.is_some() {
                "double sided"
            } else {
                "single sided"
            },
        ),
    };
    let z = match (job.z_zero, job.rotary.is_some()) {
        (ZZero::MaterialSurface, false) => "Z zero on top",
        (ZZero::MachineBed, false) => "Z zero on the bed",
        (ZZero::MaterialSurface, true) => "Z zero on the cylinder's top",
        (ZZero::MachineBed, true) => "Z zero on the axis",
    };
    let datum = match job.datum {
        Datum::BottomLeft => "bottom left",
        Datum::TopLeft => "top left",
        Datum::TopRight => "top right",
        Datum::BottomRight => "bottom right",
        Datum::Centre => "in the middle",
    };
    format!("{size}, {kind}, {z}, X0 Y0 {datum}")
}

impl CutCraftApp {
    /// Setup Material: Job Setup's form showing only [`BRIEF`].
    pub(super) fn open_setup_material(&mut self) {
        self.start_command(JOB_SETUP);
        if let Some(form) = self.form.as_mut().filter(|f| f.id == JOB_SETUP) {
            form.fields.retain(|f| BRIEF.contains(&f.key.as_str()));
            form.label = LABEL;
            form.brief = true;
            LABEL.clone_into(&mut self.status.current_tool);
        }
    }

    /// Under Setup Material's settings: the rest of the job's settings in a line, with
    /// links to change them for this job (the whole of Job Setup) or for every new job
    /// (Options ▸ Job Defaults).
    pub(super) fn setup_material_extras(&mut self, ui: &mut egui::Ui) {
        if !self.form.as_ref().is_some_and(|f| f.brief) {
            return;
        }
        ui.separator();
        ui.label(summary(&self.doc.job));
        let (more, defaults) = ui
            .horizontal_wrapped(|ui| {
                let more = ui.link(MORE).on_hover_hint(
                    "Show every Job Setup setting here: Z zero, X0 Y0, the rapid heights, \
                     home and the material, for this job only",
                );
                crate::marks::note(ui, &format!("button:{MORE}"), more.rect);
                let defaults = ui
                    .link(DEFAULTS)
                    .on_hover_hint("Edit ▸ Options ▸ Job Defaults: what every new job starts with");
                crate::marks::note(ui, &format!("button:{DEFAULTS}"), defaults.rect);
                (more.clicked(), defaults.clicked())
            })
            .inner;
        if more {
            self.show_all_job_settings();
        }
        if defaults {
            self.open_job_defaults();
        }
    }

    /// Setup Material's form becomes the whole of Job Setup, with what was set kept.
    pub(super) fn show_all_job_settings(&mut self) {
        let Some(cmd) = self.registry.get(JOB_SETUP) else {
            return;
        };
        let (fields, label) = (crate::form::fields(&cmd.schema()), cmd.meta().label);
        if let Some(form) = self.form.as_mut().filter(|f| f.brief) {
            form.fields = fields;
            form.label = label;
            form.brief = false;
        }
    }

    /// Options ▸ Job Defaults' job settings as being edited: Job Setup's fields, their
    /// values (from the job new jobs start with, until one is changed) and why the last
    /// change was refused, if it was.
    pub(super) fn job_defaults_state(&self) -> (Vec<crate::form::Field>, Value, Option<String>) {
        let Some(cmd) = self.registry.get(JOB_SETUP) else {
            return (vec![], Value::Null, None);
        };
        let fields = crate::form::fields(&cmd.schema());
        let (params, error) = self.job_defaults_form.clone().unwrap_or_else(|| {
            let empty = Document {
                job: self.edited_options().default_job.unwrap_or_default(),
                ..Document::default()
            };
            (cmd.initial_params(&empty), None)
        });
        (fields, params, error)
    }

    /// A Job Defaults setting changed: the job it makes is what new jobs will start with,
    /// once Apply is pressed; one refused (Plunge over Clearance) is kept as typed, with
    /// why, until it is put right.
    pub(super) fn job_defaults_edited(&mut self, params: Value) {
        // what is not on the form (where the model sits, the clearing default) is kept
        let empty = Document {
            job: self.edited_options().default_job.unwrap_or_default(),
            ..Document::default()
        };
        let made = self
            .registry
            .get(JOB_SETUP)
            .ok_or_else(|| "Job Setup is missing".to_owned())
            .and_then(|cmd| cmd.run_json(&empty, &params).map_err(|e| e.to_string()))
            .and_then(|change| {
                change
                    .job()
                    .copied()
                    .ok_or_else(|| "no job made".to_owned())
            });
        match made {
            Ok(job) => {
                let mut edited = self.edited_options().clone();
                edited.default_job = Some(job);
                self.options_edited(edited);
                self.job_defaults_form = Some((params, None));
            }
            Err(e) => self.job_defaults_form = Some((params, Some(e))),
        }
    }

    /// Edit ▸ Options at its Job Defaults page.
    pub(super) fn open_job_defaults(&mut self) {
        self.options_goto = Some(crate::options::JOB_DEFAULTS);
        self.options_window = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::document::{Along, DoubleSided, Units};

    #[test]
    fn the_summary_names_the_size_kind_and_both_zeros() {
        assert_eq!(
            summary(&Job::default()),
            "600 × 400 × 20 mm, single sided, Z zero on top, X0 Y0 bottom left"
        );
        let inches = Job {
            width: 24.0 * 25.4,
            height: 12.0 * 25.4,
            thickness: 0.75 * 25.4,
            units: Units::Inch,
            double_sided: Some(DoubleSided::default()),
            z_zero: ZZero::MachineBed,
            datum: Datum::Centre,
            ..Job::default()
        };
        assert_eq!(
            summary(&inches),
            "24 × 12 × 0.75 in, double sided, Z zero on the bed, X0 Y0 in the middle"
        );
        assert_eq!(
            summary(&Job::rotary(300.0, 50.0, Along::X)),
            "300 mm long, 50 mm across, rotary, Z zero on the cylinder's top, X0 Y0 bottom left"
        );
    }
}
