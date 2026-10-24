//! Job Setup: the material block, where the machine's zero is on it, how finely the model is
//! worked out and how the material looks, laid out in sections as Aspire's form is.

use cut_craft_core::document::{Along, Datum, Document, Job, Resolution, Rotary, Units, ZZero};
use cut_craft_core::geometry::Point;
use cut_craft_core::glam::DAffine2;
use cut_craft_core::{Change, Command, CommandError, Meta, ensure_positive};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct JobSetup;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum JobType {
    #[default]
    #[schemars(title = "Single Sided")]
    SingleSided,
    /// Not supported yet
    #[schemars(title = "Double Sided")]
    DoubleSided,
    /// A cylinder turned by a rotary axis
    Rotary,
}

/// For a rotary job: its length is the Width (along X) or the Height (along Y)
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct RotarySize {
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub diameter: f64,
    /// The direction the cylinder's axis runs
    pub orientation: Along,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Size {
    #[schemars(title = "Width (X)", extend("x-unit" = "mm"), range(min = 0))]
    pub width: f64,
    #[schemars(title = "Height (Y)", extend("x-unit" = "mm"), range(min = 0))]
    pub height: f64,
    #[schemars(title = "Thickness (Z)", extend("x-unit" = "mm"), range(min = 0))]
    pub thickness: f64,
    /// Lengths are shown in these; the job is always stored in millimetres
    #[schemars(extend("x-widget" = "row"))]
    pub units: Units,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct DatumPosition {
    /// The point of the material at the machine's X0, Y0
    #[schemars(extend("x-widget" = "corners"))]
    pub position: Datum,
    /// Give that point coordinates other than 0, 0
    pub use_offset: bool,
    #[schemars(title = "X", extend("x-unit" = "mm"))]
    pub x: f64,
    #[schemars(title = "Y", extend("x-unit" = "mm"))]
    pub y: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    #[schemars(
        title = "Job Type",
        extend("x-section" = true),
        extend("x-icon" = "job_setup/{}_job.svg")
    )]
    pub job_type: JobType,
    #[schemars(
        title = "Job Size",
        extend("x-section" = true),
        extend("x-icon" = "job_setup/material_size_xyz.svg")
    )]
    pub size: Size,
    #[schemars(
        title = "Z Zero Position",
        extend("x-section" = true),
        extend("x-icon" = "job_setup/z_zero_{}.svg")
    )]
    pub z_zero: ZZero,
    #[schemars(
        title = "XY Datum Position",
        extend("x-section" = true),
        extend("x-icon" = "job_setup/xy_datum_{position}.svg")
    )]
    pub datum: DatumPosition,
    #[schemars(title = "Rotary", extend("x-section" = true))]
    pub rotary: RotarySize,
    /// How finely 3D shapes are worked out over the job; Standard suits most parts
    #[schemars(
        title = "Modeling Resolution",
        extend("x-section" = true),
        extend("x-widget" = "drop-down")
    )]
    pub resolution: Resolution,
    /// The material's colour in the 3D view
    #[schemars(
        title = "Material Settings",
        extend("x-section" = true),
        extend("x-kind" = "colour")
    )]
    pub colour: [u8; 3],
    /// Safe height above the material for rapid moves
    #[schemars(
        title = "Clearance (Z1)",
        extend("x-section" = true),
        extend("x-icon" = "job_setup/clearance_z1.svg"),
        extend("x-unit" = "mm"),
        range(min = 0)
    )]
    pub clearance: f64,
}

impl From<&Job> for Params {
    fn from(job: &Job) -> Self {
        // ponytail: a new rotary job starts as a 50 mm cylinder along X
        let rotary = job.rotary.unwrap_or(Rotary {
            diameter: 50.0,
            along: Along::X,
        });
        Self {
            job_type: if job.rotary.is_some() {
                JobType::Rotary
            } else {
                JobType::SingleSided
            },
            size: Size {
                width: job.width,
                height: job.height,
                thickness: job.thickness,
                units: job.units,
            },
            z_zero: job.z_zero,
            datum: DatumPosition {
                position: job.datum,
                use_offset: job.offset != Point::ZERO,
                x: job.offset.x,
                y: job.offset.y,
            },
            rotary: RotarySize {
                diameter: rotary.diameter,
                orientation: rotary.along,
            },
            resolution: job.resolution,
            colour: job.colour,
            clearance: job.clearance,
        }
    }
}

impl Default for Params {
    fn default() -> Self {
        (&Job::default()).into()
    }
}

impl Command for JobSetup {
    const ID: &'static str = "job_setup";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Job Setup",
            group: "File Operations",
            icon: "file/job_setup.svg",
            shortcut: "",
        }
    }

    fn initial_params(doc: &Document) -> Params {
        (&doc.job).into()
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        ensure_positive("clearance", p.clearance)?;
        let shape = match p.job_type {
            JobType::DoubleSided => {
                return Err(CommandError::InvalidParams(
                    "double-sided jobs are not supported yet".into(),
                ));
            }
            JobType::Rotary => {
                let along = p.rotary.orientation;
                let length = match along {
                    Along::X => p.size.width,
                    Along::Y => p.size.height,
                };
                ensure_positive("length", length)?;
                ensure_positive("diameter", p.rotary.diameter)?;
                Job::rotary(length, p.rotary.diameter, along)
            }
            JobType::SingleSided => {
                ensure_positive("width", p.size.width)?;
                ensure_positive("height", p.size.height)?;
                ensure_positive("thickness", p.size.thickness)?;
                Job {
                    width: p.size.width,
                    height: p.size.height,
                    thickness: p.size.thickness,
                    ..Job::default()
                }
            }
        };
        let job = Job {
            units: p.size.units,
            clearance: p.clearance,
            z_zero: p.z_zero,
            datum: p.datum.position,
            offset: if p.datum.use_offset {
                Point::new(p.datum.x, p.datum.y)
            } else {
                Point::ZERO
            },
            resolution: p.resolution,
            colour: p.colour,
            ..shape
        };
        let mut c = Change::builder(doc);
        // a new datum or offset moves the coordinates, not the design on the material
        let old = doc.job;
        let moved = Job {
            width: old.width,
            height: old.height,
            ..job
        }
        .area()
        .min - old.area().min;
        if moved != Point::ZERO {
            move_everything(doc, &mut c, moved);
        }
        c.set_job(job);
        Ok(c.finish())
    }
}

/// Vectors, toolpaths and components all shifted by `by`.
fn move_everything(doc: &Document, c: &mut cut_craft_core::ChangeBuilder, by: Point) {
    let shift = DAffine2::from_translation(by);
    for (&id, path) in &doc.vectors {
        c.replace(id, path.transformed(&shift));
    }
    for (&id, toolpath) in &doc.toolpaths {
        let mut toolpath = toolpath.clone();
        for m in &mut toolpath.moves {
            m.to += by.extend(0.0);
        }
        c.replace_toolpath(id, toolpath);
    }
    for (&id, component) in &doc.components {
        let mut component = component.clone();
        component.origin += by;
        c.replace_component(id, component);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::component::{Combine, Component};
    use cut_craft_core::geometry::Rect;
    use cut_craft_core::testing::{after, after_err, run_on, sample_document};
    use cut_craft_core::toolpath::{Move, MoveKind, Tool, Toolpath};

    cut_craft_core::contract_tests!(JobSetup);

    fn with(change: impl FnOnce(&mut Params)) -> Params {
        let mut p = Params::default();
        change(&mut p);
        p
    }

    #[test]
    fn sets_the_material_block_and_everything_else_on_the_form() {
        let doc = after::<JobSetup>(&with(|p| {
            p.size.width = 1000.0;
            p.size.thickness = 12.0;
            p.size.units = Units::Inch;
            p.z_zero = ZZero::MachineBed;
            p.resolution = Resolution::High;
            p.colour = [10, 20, 30];
        }));
        let job = doc.job;
        assert_eq!(
            (job.width, job.height, job.thickness, job.units),
            (1000.0, 400.0, 12.0, Units::Inch)
        );
        assert_eq!(
            (job.z_zero, job.resolution, job.colour),
            (ZZero::MachineBed, Resolution::High, [10, 20, 30])
        );
    }

    #[test]
    fn the_form_starts_from_the_current_job() {
        let mut doc = sample_document();
        let mut c = Change::builder(&doc);
        c.set_job(Job {
            width: 123.0,
            offset: Point::new(5.0, 0.0),
            ..doc.job
        });
        doc.apply(&c.finish()).unwrap();
        let p = JobSetup::initial_params(&doc);
        assert!((p.size.width - 123.0).abs() < 1e-12);
        assert!(p.datum.use_offset && (p.datum.x - 5.0).abs() < 1e-12);
    }

    #[test]
    fn an_offset_gives_the_datum_its_coordinates_and_is_ignored_unless_used() {
        let offset = |use_offset| {
            after::<JobSetup>(&with(|p| {
                p.datum = DatumPosition {
                    position: Datum::BottomLeft,
                    use_offset,
                    x: 10.0,
                    y: 20.0,
                };
            }))
            .job
            .offset
        };
        assert_eq!(offset(true), Point::new(10.0, 20.0));
        assert_eq!(offset(false), Point::ZERO);
    }

    #[test]
    fn a_new_datum_keeps_the_design_on_the_material() {
        let mut doc = sample_document();
        let mut c = Change::builder(&doc);
        let area = Rect::new(Point::new(10.0, 10.0), Point::new(20.0, 20.0));
        let component = c.add_component(Component::from_fn("c", Combine::Add, area, 1.0, |_| {
            Some(1.0)
        }));
        let toolpath = c.add_toolpath(Toolpath {
            name: "t".into(),
            strategy: "profile_toolpath".into(),
            params: serde_json::Value::Null,
            tool: Tool::default(),
            vectors: vec![],
            moves: vec![Move {
                kind: MoveKind::Cut,
                to: Point::new(10.0, 10.0).extend(-1.0),
            }],
            visible: true,
        });
        let change = c.finish();
        doc.apply(&change).unwrap();
        let (vector, before) = doc
            .vectors
            .iter()
            .next()
            .map(|(&id, p)| (id, p.bounds()))
            .unwrap();
        run_on::<JobSetup>(&mut doc, &with(|p| p.datum.position = Datum::Centre));
        // the 600 x 400 material now has its centre at 0, 0
        let moved = Point::new(-300.0, -200.0);
        assert_eq!(doc.vectors[&vector].bounds().min, before.min + moved);
        assert_eq!(
            doc.toolpaths[&toolpath].moves[0].to,
            (Point::new(10.0, 10.0) + moved).extend(-1.0)
        );
        assert_eq!(
            doc.components[&component].origin,
            Point::new(10.0, 10.0) + moved
        );
    }

    #[test]
    fn resizing_leaves_the_design_where_it_is() {
        let mut doc = sample_document();
        let before = doc.clone();
        run_on::<JobSetup>(&mut doc, &with(|p| p.size.width = 900.0));
        assert_eq!(doc.vectors, before.vectors);
    }

    #[test]
    fn double_sided_jobs_are_not_supported_yet() {
        let why = after_err::<JobSetup>(&with(|p| p.job_type = JobType::DoubleSided));
        assert!(why.contains("not supported yet"), "{why}");
    }

    #[test]
    fn a_rotary_job_unrolls_the_cylinder_round_the_axis_chosen() {
        use cut_craft_core::document::{Along, Rotary};
        let doc = after::<JobSetup>(&with(|p| {
            p.job_type = JobType::Rotary;
            p.size.width = 300.0;
            p.rotary.diameter = 50.0;
            p.rotary.orientation = Along::X;
            p.z_zero = ZZero::MachineBed;
        }));
        let job = doc.job;
        assert_eq!(
            job.rotary,
            Some(Rotary {
                diameter: 50.0,
                along: Along::X
            })
        );
        assert!(
            (job.width - 300.0).abs() < 1e-12
                && (job.height - 50.0 * std::f64::consts::PI).abs() < 1e-9
                && (job.thickness - 25.0).abs() < 1e-12,
            "300 long, round the 50 mm circumference, the radius deep: {job:?}"
        );
        assert_eq!(job.z_zero, ZZero::MachineBed);
        let reopened = Params::from(&job);
        assert_eq!(
            (reopened.job_type, reopened.rotary.diameter),
            (JobType::Rotary, 50.0)
        );
    }

    #[test]
    fn rejects_a_zero_thickness() {
        assert_eq!(
            after_err::<JobSetup>(&with(|p| p.size.thickness = 0.0)),
            "invalid parameters: thickness must be greater than 0"
        );
    }
}
