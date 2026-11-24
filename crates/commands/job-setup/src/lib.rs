//! Job Setup: the material block, where the machine's zero is on it, how finely the model is
//! worked out and how the material looks.

use cut_craft_core::document::{
    Along, Blank, Datum, Document, DoubleSided, Home, Job, Resolution, Rotary, Support, Units,
    ZZero,
};
use cut_craft_core::geometry::Point;
use cut_craft_core::glam::DAffine2;
use cut_craft_core::{Change, Command, CommandError, Form, Meta, ensure_positive};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct JobSetup;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum JobType {
    /// Cut from the top face only
    #[default]
    #[schemars(title = "Single Sided")]
    SingleSided,
    /// Both faces machined, the material turned over between them
    #[schemars(title = "Double Sided")]
    DoubleSided,
    /// A cylinder turned by a rotary axis
    Rotary,
}

/// The shape of the material on a rotary job, across its axis.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Shape {
    /// A round blank: a dowel, a turned spindle or a log. Diameter (D) is across it
    #[default]
    Round,
    /// A square blank, a face flat on top at A=0. Diameter (D) is its side
    Square,
    /// A flat board, lying flat at A=0, for carving both faces with A turning it over.
    /// Diameter (D) is its thickness, and Board Width (W) how wide it is
    Board,
}

/// Where Z = 0 is on a rotary job: Job Setup and Material Setup show this in place of
/// [`ZZero`] (its bed choice is the cylinder's axis there).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RotaryZZero {
    /// The tool is zeroed on the top of the cylinder: touch the bit off on the highest point
    /// of the blank. Cut depths go below zero, so a 3 mm deep cut is Z -3. Easiest when the
    /// blank is round and true; set zero again for each new blank
    #[default]
    #[schemars(title = "Cylinder Surface")]
    CylinderSurface,
    /// The tool is zeroed on the cylinder's centre line, the rotary axis (touch off on the
    /// axis, not on the machine bed or the rotary's base). Every Z is a distance from the
    /// axis, so on a 50 mm blank a 3 mm deep cut is Z 22. Zero stays put when you change
    /// blanks, and an out-of-round blank still cuts to the right finished size
    #[schemars(title = "Cylinder Axis")]
    CylinderAxis,
}

impl From<ZZero> for RotaryZZero {
    fn from(z: ZZero) -> Self {
        match z {
            ZZero::MaterialSurface => Self::CylinderSurface,
            ZZero::MachineBed => Self::CylinderAxis,
        }
    }
}

impl From<RotaryZZero> for ZZero {
    fn from(z: RotaryZZero) -> Self {
        match z {
            RotaryZZero::CylinderSurface => Self::MaterialSurface,
            RotaryZZero::CylinderAxis => Self::MachineBed,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Size {
    /// The material's size across, left to right
    #[schemars(
        title = "Width (X)",
        extend("x-unit" = "mm", "x-hide-when" = {"job_type": "rotary"}),
        range(min = 0)
    )]
    pub width: f64,
    /// The material's size up the job, front to back
    #[schemars(
        title = "Height (Y)",
        extend("x-unit" = "mm", "x-hide-when" = {"job_type": "rotary"}),
        range(min = 0)
    )]
    pub height: f64,
    /// How thick the material is. Changing it after making toolpaths marks them out of date
    #[schemars(
        title = "Thickness (Z)",
        extend("x-unit" = "mm", "x-hide-when" = {"job_type": "rotary"}),
        range(min = 0)
    )]
    pub thickness: f64,
    /// Rotary jobs: the cylinder's length along its axis
    #[schemars(
        title = "Length (L)",
        extend("x-unit" = "mm", "x-show-when" = {"job_type": "rotary"}),
        range(min = 0)
    )]
    pub length: f64,
    /// Rotary jobs: the material's shape across its axis. A square or a board swings out
    /// further than its top as it turns, by half its diagonal: the bit lifts that much
    /// higher before A turns, and the checks before cutting see that it clears the bed
    #[schemars(
        title = "Shape",
        extend("x-show-when" = {"job_type": "rotary"}, "x-widget" = "row")
    )]
    #[serde(default)]
    pub shape: Shape,
    /// Rotary jobs: the material's size straight up through the axis at A=0: a round
    /// blank's diameter, a square's side, a board's thickness
    #[schemars(
        title = "Diameter (D)",
        extend("x-unit" = "mm", "x-show-when" = {"job_type": "rotary"}),
        range(min = 0)
    )]
    pub diameter: f64,
    /// Rotary jobs on a board: how wide the board is, across the axis, lying flat at A=0
    #[schemars(
        title = "Board Width (W)",
        extend("x-unit" = "mm", "x-show-when" = {"job_type": "rotary", "size.shape": "board"}),
        range(min = 0)
    )]
    #[serde(default)]
    pub board_width: f64,
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
    /// The X the datum point is given, with Use offset ticked
    #[schemars(
        title = "X",
        extend("x-unit" = "mm", "x-enable-when" = {"use_offset": "true"})
    )]
    pub x: f64,
    /// The Y the datum point is given, with Use offset ticked
    #[schemars(
        title = "Y",
        extend("x-unit" = "mm", "x-enable-when" = {"use_offset": "true"})
    )]
    pub y: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    /// Cut from one face, both faces, or round a cylinder
    #[schemars(
        title = "Job Type",
        extend("x-section" = true),
        extend("x-icon" = "job_setup/{}_job.png")
    )]
    pub job_type: JobType,
    /// The size of the material you are cutting
    #[schemars(
        title = "Job Size",
        extend("x-section" = true),
        extend("x-icon" = "job_setup/material_size_xyz.png")
    )]
    pub size: Size,
    /// Where the tool is zeroed in Z: on top of the material, or under it on the machine bed
    #[schemars(
        title = "Z Zero Position",
        extend("x-section" = true, "x-hide-when" = {"job_type": "rotary"}),
        extend("x-icon" = "job_setup/z_zero_{}.png")
    )]
    pub z_zero: ZZero,
    /// Rotary jobs: where the tool is zeroed in Z, on top of the cylinder or on its axis. The
    /// picture shows where. Pick the one you touch the bit off on at the machine: the G-code
    /// heights depend on it, and a wrong pick cuts too deep or in the air
    #[schemars(
        title = "Z Zero Position",
        extend("x-section" = true, "x-show-when" = {"job_type": "rotary"}),
        extend("x-icon" = "job_setup/z_zero_{}.png")
    )]
    #[serde(default)]
    pub rotary_z_zero: RotaryZZero,
    /// Which point of the material is the machine's X0, Y0. Changing it moves only the
    /// coordinates: the design, toolpaths, pictures, clamp zones and guides stay where they
    /// are on the material
    #[schemars(
        title = "XY Datum Position",
        extend("x-section" = true),
        extend("x-icon" = "job_setup/xy_datum_{position}.png")
    )]
    pub datum: DatumPosition,
    /// Rotary jobs: the direction the cylinder's axis runs
    #[schemars(
        title = "Orientation",
        extend("x-section" = true, "x-show-when" = {"job_type": "rotary"})
    )]
    pub orientation: Along,
    /// Rotary jobs: what of the material the rotary toolpaths leave to hold the part while
    /// it is cut
    #[schemars(
        title = "Holding the Part",
        extend("x-section" = true, "x-show-when" = {"job_type": "rotary"})
    )]
    #[serde(default)]
    pub support: Support,
    /// Double-sided jobs: how the material is turned over between sides
    #[schemars(
        title = "Flip Direction Between Sides",
        extend("x-section" = true, "x-show-when" = {"job_type": "double_sided"})
    )]
    #[serde(default)]
    pub sides: DoubleSided,
    /// How finely 3D shapes are worked out over the job; Standard suits most parts
    #[schemars(
        title = "Modeling Resolution",
        extend("x-section" = true),
        extend("x-widget" = "drop-down")
    )]
    pub resolution: Resolution,
    /// What the material is, from the Materials Library: toolpaths pick their bits' feeds
    /// and speeds for it (a layer naming its own material overrides it), and the 3D view
    /// shows it in the matching colour from Toolpaths ▸ Preview Colors. Left empty, each
    /// bit's own feeds
    #[serde(default)]
    #[schemars(title = "Material", extend("x-kind" = "material"))]
    pub material: String,
    /// The material's colour in the 3D view. How the preview colours what the toolpaths cut
    /// is Toolpaths ▸ Preview Colors, which this leaves as it is
    #[schemars(
        title = "Material Settings",
        extend("x-section" = true),
        extend("x-kind" = "colour")
    )]
    pub colour: [u8; 3],
    /// How high the tool travels between cuts
    #[schemars(
        title = "Rapid Z Gaps above Material",
        extend("x-section" = true),
        extend("x-icon" = "job_setup/clearance_z1.png")
    )]
    pub gaps: RapidGaps,
    /// Where the tool starts from and goes back to at the end
    #[schemars(title = "Home / Start Position", extend("x-section" = true))]
    pub home: Home,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct RapidGaps {
    /// Safe height above the material for rapid moves
    #[schemars(title = "Clearance (Z1)", extend("x-unit" = "mm"), range(min = 0))]
    pub clearance: f64,
    /// Plunges go down to this gap above the material at the rapid rate, then at the plunge
    /// rate; the same as the clearance means no rapid plunges
    #[schemars(title = "Plunge (Z2)", extend("x-unit" = "mm"), range(min = 0))]
    pub plunge: f64,
}

impl From<&Job> for Params {
    fn from(job: &Job) -> Self {
        // ponytail: a new rotary job starts as a 50 mm cylinder along X
        let rotary = job.rotary.unwrap_or(Rotary::round(50.0, Along::X));
        Self {
            job_type: match (job.rotary, job.double_sided) {
                (Some(_), _) => JobType::Rotary,
                (None, Some(_)) => JobType::DoubleSided,
                (None, None) => JobType::SingleSided,
            },
            size: Size {
                width: job.width,
                height: job.height,
                thickness: job.thickness,
                length: match (job.rotary, rotary.along) {
                    (Some(_), Along::X) => job.width,
                    (Some(_), Along::Y) => job.height,
                    // ponytail: a new rotary job starts 300 mm long
                    (None, _) => 300.0,
                },
                shape: match rotary.blank {
                    Blank::Round => Shape::Round,
                    Blank::Square => Shape::Square,
                    Blank::Board { .. } => Shape::Board,
                },
                diameter: rotary.diameter,
                // ponytail: a new board starts three times as wide as it is thick
                board_width: match rotary.blank {
                    Blank::Board { width } => width,
                    _ => rotary.diameter * 3.0,
                },
                units: job.units,
            },
            z_zero: job.z_zero,
            rotary_z_zero: job.z_zero.into(),
            datum: DatumPosition {
                position: job.datum,
                use_offset: job.offset != Point::ZERO,
                x: job.offset.x,
                y: job.offset.y,
            },
            orientation: rotary.along,
            support: rotary.support,
            sides: job.double_sided.unwrap_or_default(),
            resolution: job.resolution,
            material: String::new(),
            colour: job.colour,
            gaps: RapidGaps {
                clearance: job.clearance,
                plunge: job.plunge_gap(),
            },
            home: job.home(),
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
            icon: "file/job_setup.png",
            shortcut: "",
            form: Form::Apply,
        }
    }

    fn initial_params(doc: &Document) -> Params {
        Params {
            material: doc.job_material().to_owned(),
            ..(&doc.job).into()
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        ensure_positive("clearance", p.gaps.clearance)?;
        if !(0.0..=p.gaps.clearance).contains(&p.gaps.plunge) {
            return Err(CommandError::InvalidParams(
                "set Plunge (Z2) between 0 and the clearance (Z1)".into(),
            ));
        }
        let shape = match p.job_type {
            JobType::Rotary => {
                ensure_positive("length", p.size.length)?;
                ensure_positive("diameter", p.size.diameter)?;
                let blank = match p.size.shape {
                    Shape::Round => Blank::Round,
                    Shape::Square => Blank::Square,
                    Shape::Board => {
                        ensure_positive("board width", p.size.board_width)?;
                        Blank::Board {
                            width: p.size.board_width,
                        }
                    }
                };
                if !(0.0..p.size.diameter).contains(&p.support.beam) {
                    return Err(CommandError::InvalidParams(
                        "set Centre Stock Diameter less than the material's diameter, or 0 \
                         to leave no centre stock"
                            .into(),
                    ));
                }
                let mut job = Job::rotary(p.size.length, p.size.diameter, p.orientation);
                job.rotary = job.rotary.map(|r| Rotary {
                    blank,
                    support: p.support,
                    ..r
                });
                job
            }
            JobType::SingleSided | JobType::DoubleSided => {
                ensure_positive("width", p.size.width)?;
                ensure_positive("height", p.size.height)?;
                ensure_positive("thickness", p.size.thickness)?;
                Job {
                    width: p.size.width,
                    height: p.size.height,
                    thickness: p.size.thickness,
                    double_sided: (p.job_type == JobType::DoubleSided).then_some(p.sides),
                    ..Job::default()
                }
            }
        };
        let job = Job {
            units: p.size.units,
            clearance: p.gaps.clearance,
            plunge: Some(p.gaps.plunge),
            home: Some(home_for(doc, p)),
            z_zero: if p.job_type == JobType::Rotary {
                p.rotary_z_zero.into()
            } else {
                p.z_zero
            },
            datum: p.datum.position,
            offset: if p.datum.use_offset {
                Point::new(p.datum.x, p.datum.y)
            } else {
                Point::ZERO
            },
            resolution: p.resolution,
            colour: preview_colour(doc, p),
            // not on this form: kept as Material Setup, Clearing Settings and Toolpaths ▸
            // Preview Colors set them
            model_position: doc.job.model_position,
            clearing: doc.job.clearing,
            machined_area: doc.job.machined_area,
            fill_colour: doc.job.fill_colour,
            lithophane_light: doc.job.lithophane_light,
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
        let material = p.material.trim();
        if material != doc.job_material() {
            let mut sheets = c.sheets().to_vec();
            if let Some(sheet) = sheets.get_mut(doc.active_sheet) {
                material.clone_into(&mut sheet.material);
            }
            c.set_sheets(sheets, doc.active_sheet);
        }
        Ok(c.finish())
    }
}

/// The material's colour in the 3D view: the colour picked on the form, or, when only the
/// Material changed, the matching one from Preview Colors (Walnut's for "Walnut").
fn preview_colour(doc: &Document, p: &Params) -> [u8; 3] {
    let material = p.material.trim();
    if p.colour != doc.job.colour || material == doc.job_material() {
        return p.colour;
    }
    cmd_materials::Material::matching(material)
        .and_then(cmd_materials::Material::colour)
        .unwrap_or(p.colour)
}

/// Home as `p` sets it, but with a Home Z still at the old clearance (never set by hand)
/// following the clearance up or down: raising Clearance (Z1) to clear tall clamps must
/// not leave the start and end moves travelling at the old, lower height.
fn home_for(doc: &Document, p: &Params) -> Home {
    let mut home = p.home;
    if (home.z - doc.job.clearance).abs() < 1e-9 {
        home.z = p.gaps.clearance;
    }
    home
}

/// Everything placed on the material shifted by `by`: vectors, toolpaths, components,
/// pictures, keep-out (clamp) zones and guides.
fn move_everything(doc: &Document, c: &mut cut_craft_core::ChangeBuilder, by: Point) {
    let shift = DAffine2::from_translation(by);
    let zones = doc
        .keep_out
        .iter()
        .map(|z| cut_craft_core::keep_out::KeepOutZone {
            path: z.path.transformed(&shift),
            ..z.clone()
        })
        .collect();
    c.set_keep_out(zones);
    for (&id, bitmap) in &doc.bitmaps {
        let mut moved = bitmap.clone();
        moved.rect =
            cut_craft_core::geometry::Rect::new(bitmap.rect.min + by, bitmap.rect.max + by);
        c.replace_bitmap(id, moved);
    }
    let guides = doc
        .guides
        .iter()
        .map(|g| cut_craft_core::document::Guide {
            at: g.at + if g.vertical { by.x } else { by.y },
            ..*g
        })
        .collect();
    c.set_guides(guides);
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
    use cut_craft_core::geometry::{Path, Rect};
    use cut_craft_core::testing::{after, after_err, run_on, sample_document};
    use cut_craft_core::toolpath::{Move, MoveKind, Tool, Toolpath};

    cut_craft_core::contract_tests!(JobSetup);

    #[test]
    fn names_the_active_sheet_s_material_and_shows_it_again() {
        let mut doc = sample_document();
        run_on::<JobSetup>(&mut doc, &with(|p| p.material = " Hard Maple ".into()));
        assert_eq!(doc.job_material(), "Hard Maple");
        assert_eq!(JobSetup::initial_params(&doc).material, "Hard Maple");
    }

    #[test]
    fn picking_a_material_shows_it_in_its_colour_unless_a_colour_is_picked_too() {
        use cmd_materials::Material;
        let mut doc = sample_document();
        let walnut = Material::Walnut.colour().unwrap();
        let mut p = JobSetup::initial_params(&doc);
        p.material = "Walnut".into();
        run_on::<JobSetup>(&mut doc, &p);
        assert_eq!(doc.job.colour, walnut, "the 3D view shows walnut");
        let mut p = JobSetup::initial_params(&doc);
        p.material = "Hard Maple".into();
        p.colour = [1, 2, 3];
        run_on::<JobSetup>(&mut doc, &p);
        assert_eq!(
            doc.job.colour,
            [1, 2, 3],
            "a colour picked on the form wins"
        );
        let mut p = JobSetup::initial_params(&doc);
        p.material = "Titanium Grade 5".into();
        run_on::<JobSetup>(&mut doc, &p);
        assert_eq!(doc.job.colour, [1, 2, 3], "no preview colour for it: kept");
    }

    #[test]
    fn applying_it_keeps_the_preview_colours_set_elsewhere() {
        use cut_craft_core::document::MachinedArea;
        let mut doc = sample_document();
        doc.job.machined_area = MachinedArea::Lithophane;
        doc.job.fill_colour = [1, 2, 3];
        doc.job.lithophane_light = 0.9;
        let mut p = JobSetup::initial_params(&doc);
        p.size.width += 10.0;
        run_on::<JobSetup>(&mut doc, &p);
        assert_eq!(
            (
                doc.job.machined_area,
                doc.job.fill_colour,
                doc.job.lithophane_light
            ),
            (MachinedArea::Lithophane, [1, 2, 3], 0.9)
        );
    }

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
    fn home_z_follows_a_raised_clearance_until_it_is_set_by_hand() {
        let mut doc = sample_document();
        // raised in the same dialog, Home Z (shown at the old clearance) goes up with it
        run_on::<JobSetup>(&mut doc, &with(|p| p.gaps.clearance = 25.0));
        assert!(
            (doc.job.home().z - 25.0).abs() < 1e-9,
            "{:?}",
            doc.job.home()
        );
        // and again later
        let mut p = JobSetup::initial_params(&doc);
        p.gaps.clearance = 30.0;
        run_on::<JobSetup>(&mut doc, &p);
        assert!((doc.job.home().z - 30.0).abs() < 1e-9);
        // a Home Z of its own is kept
        let mut p = JobSetup::initial_params(&doc);
        p.home.z = 50.0;
        run_on::<JobSetup>(&mut doc, &p);
        let mut p = JobSetup::initial_params(&doc);
        p.gaps.clearance = 10.0;
        run_on::<JobSetup>(&mut doc, &p);
        assert!((doc.job.home().z - 50.0).abs() < 1e-9);
    }

    #[test]
    fn a_new_datum_moves_clamp_zones_pictures_and_guides_with_the_design() {
        let mut doc = sample_document();
        let mut c = Change::builder(&doc);
        let zone = Path::polyline(
            &[Point::ZERO, Point::new(50.0, 0.0), Point::new(50.0, 50.0)],
            true,
        );
        c.set_keep_out(vec![cut_craft_core::keep_out::KeepOutZone::new(
            "Clamp", zone,
        )]);
        let picture = c.add_bitmap(cut_craft_core::bitmap::Bitmap {
            name: "b".into(),
            columns: 2,
            rows: 1,
            grey: vec![0, 255],
            rect: Rect::new(Point::ZERO, Point::new(20.0, 10.0)),
            depth: Vec::new(),
            rgb: Vec::new(),
        });
        c.set_guides(vec![
            cut_craft_core::document::Guide {
                vertical: true,
                at: 100.0,
                locked: false,
                tilt: 0.0,
            },
            cut_craft_core::document::Guide {
                vertical: false,
                at: 40.0,
                locked: false,
                tilt: 0.0,
            },
        ]);
        let change = c.finish();
        doc.apply(&change).unwrap();
        run_on::<JobSetup>(&mut doc, &with(|p| p.datum.position = Datum::Centre));
        let moved = Point::new(-300.0, -200.0);
        assert_eq!(
            doc.keep_out[0].path.bounds().min,
            moved,
            "the clamp stays on the clamp"
        );
        assert_eq!(doc.bitmaps[&picture].rect.min, moved);
        assert!((doc.guides[0].at - (100.0 - 300.0)).abs() < 1e-9);
        assert!((doc.guides[1].at - (40.0 - 200.0)).abs() < 1e-9);
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
            power: vec![],
            stale: false,
            group: None,
            drill_bank: None,
            fallbacks: vec![],
            across: vec![],
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
    fn a_double_sided_job_keeps_its_flip_direction() {
        use cut_craft_core::document::Flip;
        let doc = after::<JobSetup>(&with(|p| {
            p.job_type = JobType::DoubleSided;
            p.sides.flip = Flip::Vertical;
        }));
        assert_eq!(doc.job.double_sided.unwrap().flip, Flip::Vertical);
        assert_eq!(
            JobSetup::initial_params(&doc).job_type,
            JobType::DoubleSided
        );
        let single = after::<JobSetup>(&with(|p| p.job_type = JobType::SingleSided));
        assert!(single.job.double_sided.is_none());
    }

    #[test]
    fn a_rotary_job_keeps_how_the_part_is_held_and_asks_for_a_beam_inside_the_material() {
        use cut_craft_core::document::Joined;
        let held = |joined, beam| {
            with(|p| {
                p.job_type = JobType::Rotary;
                p.size.length = 300.0;
                p.size.diameter = 80.0;
                p.support = Support { joined, beam };
            })
        };
        let doc = after::<JobSetup>(&held(Joined::Chuck, 10.0));
        let support = doc.job.rotary.unwrap().support;
        assert_eq!(
            support,
            Support {
                joined: Joined::Chuck,
                beam: 10.0
            }
        );
        assert_eq!(Params::from(&doc.job).support, support, "shown again");
        for beam in [80.0, -1.0] {
            let refused = JobSetup::run(&Document::default(), &held(Joined::Neither, beam));
            assert!(
                matches!(&refused, Err(CommandError::InvalidParams(why)) if why.contains("Centre Stock Diameter")),
                "{beam}: {:?}",
                refused.err()
            );
        }
    }

    #[test]
    fn a_board_on_the_rotary_keeps_its_width_and_swings_by_half_its_diagonal() {
        use cut_craft_core::document::Blank;
        let doc = after::<JobSetup>(&with(|p| {
            p.job_type = JobType::Rotary;
            p.size.length = 300.0;
            p.size.shape = Shape::Board;
            p.size.diameter = 20.0;
            p.size.board_width = 150.0;
        }));
        let rotary = doc.job.rotary.unwrap();
        assert_eq!(rotary.blank, Blank::Board { width: 150.0 });
        assert!((rotary.swing() - 75f64.hypot(10.0)).abs() < 1e-9);
        // unrolled round its thickness, the bit lifts clear of its edges before A turns
        assert!((doc.job.thickness - 10.0).abs() < 1e-12);
        assert!((doc.job.turning_gap() - (doc.job.clearance + rotary.swing() - 10.0)).abs() < 1e-9);
        let reopened = Params::from(&doc.job);
        assert_eq!(
            (reopened.size.shape, reopened.size.board_width),
            (Shape::Board, 150.0)
        );
        // a board with no width is asked for one
        let none = JobSetup::run(
            &Document::default(),
            &with(|p| {
                p.job_type = JobType::Rotary;
                p.size.shape = Shape::Board;
                p.size.board_width = 0.0;
            }),
        );
        assert!(none.is_err());
    }

    #[test]
    fn a_rotary_job_unrolls_the_cylinder_round_the_axis_chosen() {
        use cut_craft_core::document::{Along, Rotary};
        let doc = after::<JobSetup>(&with(|p| {
            p.job_type = JobType::Rotary;
            p.size.length = 300.0;
            p.size.diameter = 50.0;
            p.orientation = Along::X;
            p.rotary_z_zero = RotaryZZero::CylinderAxis;
        }));
        let job = doc.job;
        assert_eq!(job.rotary, Some(Rotary::round(50.0, Along::X)));
        assert!(
            (job.width - 300.0).abs() < 1e-12
                && (job.height - 50.0 * std::f64::consts::PI).abs() < 1e-9
                && (job.thickness - 25.0).abs() < 1e-12,
            "300 long, round the 50 mm circumference, the radius deep: {job:?}"
        );
        assert_eq!(job.z_zero, ZZero::MachineBed);
        let reopened = Params::from(&job);
        assert_eq!(
            (
                reopened.job_type,
                reopened.size.diameter,
                reopened.size.length
            ),
            (JobType::Rotary, 50.0, 300.0)
        );
        assert_eq!(reopened.rotary_z_zero, RotaryZZero::CylinderAxis);
    }

    #[test]
    fn sets_the_plunge_gap_and_the_home_position() {
        let home = Home {
            x: 10.0,
            y: 20.0,
            z: 30.0,
        };
        let doc = after::<JobSetup>(&with(|p| {
            p.gaps.clearance = 15.0;
            p.gaps.plunge = 2.0;
            p.home = home;
        }));
        assert_eq!(
            (doc.job.clearance, doc.job.plunge_gap(), doc.job.home()),
            (15.0, 2.0, home)
        );
        let reopened = Params::from(&doc.job);
        assert_eq!((reopened.gaps.plunge, reopened.home), (2.0, home));
    }

    #[test]
    fn a_new_job_plunges_from_the_clearance_and_starts_above_x0_y0() {
        let p = Params::default();
        assert!((p.gaps.plunge - p.gaps.clearance).abs() < 1e-12);
        assert_eq!((p.home.x, p.home.y, p.home.z), (0.0, 0.0, p.gaps.clearance));
    }

    #[test]
    fn the_plunge_gap_must_be_under_the_clearance() {
        let why = after_err::<JobSetup>(&with(|p| p.gaps.plunge = p.gaps.clearance + 1.0));
        assert!(why.contains("Plunge (Z2)"), "{why}");
        let why = after_err::<JobSetup>(&with(|p| p.gaps.plunge = -1.0));
        assert!(why.contains("Plunge (Z2)"), "{why}");
    }

    #[test]
    fn rejects_a_zero_thickness() {
        assert_eq!(
            after_err::<JobSetup>(&with(|p| p.size.thickness = 0.0)),
            "Set thickness to more than 0."
        );
    }
}

#[cfg(test)]
mod form_tests {
    use super::*;

    /// The schema's `x-show-when` / `x-hide-when` rule on `field`, as `(key, value)`.
    fn rule(field: &serde_json::Value, which: &str) -> Option<String> {
        field.get(which).map(|r| r["job_type"].to_string())
    }

    #[test]
    fn rotary_fields_show_only_for_rotary_jobs_and_flat_sizes_only_for_flat_ones() {
        let schema = serde_json::to_value(schemars::schema_for!(Params)).unwrap();
        let size = &schema["$defs"]["Size"]["properties"];
        for flat in ["width", "height", "thickness"] {
            assert_eq!(
                rule(&size[flat], "x-hide-when").as_deref(),
                Some("\"rotary\""),
                "{flat}"
            );
        }
        for round in ["length", "diameter"] {
            assert_eq!(
                rule(&size[round], "x-show-when").as_deref(),
                Some("\"rotary\""),
                "{round}"
            );
        }
        // Z zero reads as the cylinder's axis or surface on a rotary job, never the bed
        assert_eq!(
            rule(&schema["properties"]["z_zero"], "x-hide-when").as_deref(),
            Some("\"rotary\"")
        );
        let round = &schema["properties"]["rotary_z_zero"];
        assert_eq!(rule(round, "x-show-when").as_deref(), Some("\"rotary\""));
        let titles: Vec<&str> = schema["$defs"]["RotaryZZero"]["oneOf"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|v| v["title"].as_str())
            .collect();
        assert_eq!(titles, ["Cylinder Surface", "Cylinder Axis"]);
        let orientation = &schema["properties"]["orientation"];
        assert_eq!(
            rule(orientation, "x-show-when").as_deref(),
            Some("\"rotary\"")
        );
    }
}
