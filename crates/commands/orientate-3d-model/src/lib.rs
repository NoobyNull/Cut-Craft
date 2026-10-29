//! Orientate 3D Model: Aspire's step between picking an STL and having a component. Which
//! face is up, a turn about Z, the model's size and units, centring, how high above the bed
//! it sits, and whether anything below the zero plane is kept.

use cut_craft_core::component::{Combine, Component, Source};
use cut_craft_core::document::{Along, Document, Rotary};
use cut_craft_core::geometry::{Point, Rect};
use cut_craft_core::glam::{DMat3, DVec3};
use cut_craft_core::params::Xy;
use cut_craft_core::{Change, Command, CommandError, Meta};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct Orientate3dModel;

/// Which face of the model as drawn is the top.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Face {
    #[default]
    Top,
    Bottom,
    Front,
    Back,
    Left,
    Right,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum FileUnits {
    #[default]
    Mm,
    Inches,
}

/// Aspire's Initial Orientation: which face is up, then a turn about Z.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Orientation {
    /// The face of the model as drawn that becomes its top
    #[schemars(
        title = "Face up",
        extend("x-widget" = "faces"),
        extend("x-icon" = "import/initial_orientation.png")
    )]
    pub face: Face,
    #[schemars(
        title = "Rotation about Z Axis",
        extend("x-unit" = "deg"),
        extend("x-steps" = [-180, -90, 0, 90, 180])
    )]
    pub rotation: f64,
}

/// Aspire's Model Size: the model's size, the ratio lock and the file's units.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct ModelSize {
    /// Keep the model's proportions: the size changed sets the others
    #[schemars(title = "Lock XYZ ratio")]
    pub lock_ratio: bool,
    /// The model's length once placed; 0 keeps the file's size
    #[schemars(title = "X", extend("x-unit" = "mm"))]
    pub x: f64,
    /// The model's width once placed; 0 keeps the file's size
    #[schemars(title = "Y", extend("x-unit" = "mm"))]
    pub y: f64,
    /// The model's height once placed; 0 keeps the file's size
    #[schemars(title = "Z", extend("x-unit" = "mm"))]
    #[serde(default)]
    pub z: f64,
    /// The numbers in the file are in these units
    #[schemars(extend("x-widget" = "row"))]
    pub units: FileUnits,
}

/// Aspire's second page, Position relative to Modeling Plane.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Placing {
    #[schemars(title = "Center model")]
    pub center: bool,
    /// Where the model's lower-left corner goes when not centred
    pub position: Xy,
    /// How high the model's lowest point sits above the bed (the material's underside);
    /// below 0 sinks it into the zero plane
    #[schemars(title = "Height above bed", extend("x-unit" = "mm"))]
    #[serde(default)]
    pub height_above_bed: f64,
    #[schemars(title = "Discard data below zero plane")]
    pub discard_below_zero: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    /// The STL file (binary or ASCII)
    pub file: String,
    #[schemars(title = "Initial Orientation", extend("x-section" = true))]
    pub orientation: Orientation,
    #[schemars(
        title = "Model Size",
        extend("x-section" = true),
        extend("x-icon" = "import/model_size_xyz.png")
    )]
    pub size: ModelSize,
    #[schemars(
        title = "Position relative to Modeling Plane",
        extend("x-section" = true),
        extend("x-icon" = "import/position_relative_to_modeling_plane.png")
    )]
    pub placing: Placing,
    /// Rotary jobs: a full 3D model is wrapped round the axis, its distance out at each angle
    /// becoming the height; a flat model is placed on the unrolled surface like clipart
    #[schemars(title = "Imported model type", extend("x-hide-when" = {"job": "flat"}))]
    #[serde(default)]
    pub model_type: ModelType,
    /// Full 3D models: scaled to the material's length and diameter (else Model Size's X is
    /// its length; 0 keeps the file's size)
    #[schemars(
        title = "Scale model to fit material",
        extend("x-hide-when" = {"job": "flat"})
    )]
    #[serde(default = "yes")]
    pub fit_material: bool,
    /// The job's kind, so the form shows the rotary choices only in a rotary job; never shown
    #[schemars(extend("x-hide-when" = {"job": ["flat", "rotary"]}))]
    #[serde(default)]
    pub job: JobKind,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum JobKind {
    #[default]
    Flat,
    Rotary,
}

fn yes() -> bool {
    true
}

/// What a model imported into a rotary job is.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ModelType {
    /// A model to machine as it is, all the way round
    #[schemars(title = "Full 3D model")]
    Full,
    /// Part of a design, like clipart
    #[default]
    #[schemars(title = "Flat model")]
    Flat,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            file: String::new(),
            orientation: Orientation {
                face: Face::Top,
                rotation: 0.0,
            },
            size: ModelSize {
                lock_ratio: true,
                x: 0.0,
                y: 0.0,
                z: 0.0,
                units: FileUnits::Mm,
            },
            placing: Placing {
                center: true,
                position: Xy::new(0.0, 0.0),
                height_above_bed: 0.0,
                discard_below_zero: true,
            },
            model_type: ModelType::Flat,
            fit_material: true,
            job: JobKind::Flat,
        }
    }
}

impl Command for Orientate3dModel {
    const ID: &'static str = "orientate_3d_model";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Orientate 3D Model",
            group: "Modeling",
            icon: "modeling/import_component_3d_model.png",
            shortcut: "",
        }
    }

    /// A rotary job starts on a full 3D model.
    fn initial_params(doc: &Document) -> Params {
        let rotary = doc.job.rotary.is_some();
        Params {
            model_type: if rotary {
                ModelType::Full
            } else {
                ModelType::Flat
            },
            job: if rotary {
                JobKind::Rotary
            } else {
                JobKind::Flat
            },
            ..Params::default()
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        let (tris, md5) = oriented(p)?;
        // what imports it again, from the same file
        let source = Some(Source {
            path: p.file.clone().into(),
            relative: None,
            md5,
            settings: serde_json::to_value(p).expect("params serialize"),
        });
        if let Some(rotary) = doc.job.rotary
            && p.model_type == ModelType::Full
        {
            let k = wrapped(doc, rotary, &tris, p)?;
            let mut c = Change::builder(doc);
            c.add_component(Component { source, ..k });
            return Ok(c.finish());
        }
        let (tris, rect) = placed(doc, p, tris)?;
        let mut k = Component::from_fn(
            "Imported model",
            Combine::Add,
            rect,
            doc.job.model_cell(),
            |_| None,
        );
        fmt_import_stl::top_down(&mut k, &tris);
        if p.placing.discard_below_zero {
            for h in &mut k.heights {
                if *h < 0.0 {
                    *h = f64::NAN;
                }
            }
        }
        if k.height_range().is_none() {
            return Err(CommandError::Failed(
                "nothing of the model is above the zero plane".into(),
            ));
        }
        k.source = source;
        let mut c = Change::builder(doc);
        c.add_component(k);
        let hidden = fmt_import_stl::undercut(&tris);
        // ponytail: under a square millimetre is mesh noise, not an undercut
        if hidden >= 1.0 {
            c.note(format!(
                "the model has undercuts: {:.1} cm² of it faces down, which a cutter from \
                 above cannot reach; it is cut as straight walls there",
                hidden / 100.0
            ));
        }
        Ok(c.finish())
    }
}

/// The file's triangles turned to the face chosen and in millimetres, and the file's MD5.
fn oriented(p: &Params) -> Result<(Vec<[DVec3; 3]>, String), CommandError> {
    if p.file.is_empty() {
        return Err(CommandError::InvalidParams("choose an STL file".into()));
    }
    let bytes =
        std::fs::read(&p.file).map_err(|e| CommandError::Failed(format!("{}: {e}", p.file)))?;
    let triangles = fmt_import_stl::triangles(&bytes).map_err(CommandError::Failed)?;
    let turn =
        DMat3::from_rotation_z(p.orientation.rotation.to_radians()) * face_up(p.orientation.face);
    let unit = match p.size.units {
        FileUnits::Mm => 1.0,
        FileUnits::Inches => 25.4,
    };
    let turned = triangles
        .iter()
        .map(|t| t.map(|v| turn * v * unit))
        .collect();
    Ok((turned, Source::md5(&bytes)))
}

/// `tris` sized and placed in a flat job (heights up from the bed), and the footprint
/// they cover.
fn placed(
    doc: &Document,
    p: &Params,
    mut tris: Vec<[DVec3; 3]>,
) -> Result<(Vec<[DVec3; 3]>, Rect), CommandError> {
    let (lo, hi) =
        bounds(&tris).ok_or_else(|| CommandError::Failed("the model is empty".into()))?;
    let size = hi - lo;
    if size.x <= 0.0 || size.y <= 0.0 {
        return Err(CommandError::Failed(
            "the model has no area seen from above".into(),
        ));
    }
    // size: 0 keeps the file's; with the ratio locked the first size given sets all
    let wanted = |want: f64, have: f64| (want > 0.0 && have > 0.0).then(|| want / have);
    let each = [
        wanted(p.size.x, size.x),
        wanted(p.size.y, size.y),
        wanted(p.size.z, size.z),
    ];
    let scale = if p.size.lock_ratio {
        DVec3::splat(each.into_iter().flatten().next().unwrap_or(1.0))
    } else {
        let [x, y, z] = each.map(|s| s.unwrap_or(1.0));
        DVec3::new(x, y, z)
    };
    let area = doc.job.area();
    let footprint = Point::new(size.x * scale.x, size.y * scale.y);
    let corner = if p.placing.center {
        area.center() - footprint / 2.0
    } else {
        p.placing.position.into()
    };
    let bottom = p.placing.height_above_bed;
    let place = |v: DVec3| {
        DVec3::new(
            corner.x + (v.x - lo.x) * scale.x,
            corner.y + (v.y - lo.y) * scale.y,
            bottom + (v.z - lo.z) * scale.z,
        )
    };
    for t in &mut tris {
        *t = t.map(place);
    }
    Ok((tris, Rect::new(corner, corner + footprint)))
}

/// A triangle of the previewed model, and whether a cutter from above cannot reach it.
pub type Seen = ([DVec3; 3], bool);

/// The model as a flat job places it, as triangles in job millimetres, each with whether
/// a cutter from above cannot reach it (the model is over it: an overhang's underside, or
/// what is tucked under one): what the 3D view shows while the form is open. A full 3D
/// model in a rotary job is shown turned round the job's axis instead (x along it, y and z
/// across from it), with nothing marked: it is cut all the way round.
pub fn preview(doc: &Document, params: &Params) -> Result<Option<Vec<Seen>>, CommandError> {
    // ponytail: a tenth of a millimetre clear of the base counts, as fmt_import_stl::undercut
    const ABOVE: f64 = 0.1;
    if let Some(rotary) = doc.job.rotary
        && params.model_type == ModelType::Full
    {
        let tris = oriented(params)?.0;
        let turning = Turning::of(doc, rotary, &tris, params)?;
        return Ok(Some(
            tris.iter()
                .map(|t| (t.map(|v| turning.placed(v)), false))
                .collect(),
        ));
    }
    let (tris, rect) = placed(doc, params, oriented(params)?.0)?;
    let mut top = Component::from_fn("", Combine::Add, rect, doc.job.model_cell(), |_| None);
    fmt_import_stl::top_down(&mut top, &tris);
    let lowest = tris
        .iter()
        .flatten()
        .map(|v| v.z)
        .fold(f64::INFINITY, f64::min);
    let hidden = |t: &[DVec3; 3]| {
        let [a, b, c] = *t;
        let middle = (a + b + c) / 3.0;
        // a cell out from the face, so a wall is judged by the air beside it
        let out = (b - a).cross(c - a).normalize_or_zero() * top.cell;
        let over = top.height_at(Point::new(middle.x + out.x, middle.y + out.y));
        let clear = t.iter().map(|v| v.z).fold(f64::INFINITY, f64::min) > lowest + ABOVE;
        clear && over.is_some_and(|z| middle.z < z - top.cell)
    };
    Ok(Some(tris.iter().map(|t| (*t, hidden(t))).collect()))
}

/// The turn that brings `face` to the top (looking down the Z axis, Y up the screen).
fn face_up(face: Face) -> DMat3 {
    use std::f64::consts::FRAC_PI_2;
    match face {
        Face::Top => DMat3::IDENTITY,
        Face::Bottom => DMat3::from_rotation_x(2.0 * FRAC_PI_2),
        // the face's outward normal turned to +Z: -Y up for Front, +Y for Back, and so on
        Face::Front => DMat3::from_rotation_x(-FRAC_PI_2),
        Face::Back => DMat3::from_rotation_x(FRAC_PI_2),
        Face::Left => DMat3::from_rotation_y(FRAC_PI_2),
        Face::Right => DMat3::from_rotation_y(-FRAC_PI_2),
    }
}

/// How a full 3D model sits in a rotary job: its axis along the job's, through the middle
/// of its box across, scaled to the material (or to Model Size's X as its length), from
/// where it starts along the job.
struct Turning {
    x_axis: bool,
    /// The model's middle across (its y, z for an X axis; x, z for a Y axis), and where it
    /// starts along, before scaling.
    middle: Point,
    start: f64,
    scale: f64,
    /// Where the scaled model starts along the job, in job mm.
    first: f64,
    unrolled: Unrolled,
}

impl Turning {
    fn of(
        doc: &Document,
        rotary: Rotary,
        tris: &[[DVec3; 3]],
        params: &Params,
    ) -> Result<Self, CommandError> {
        let (lo, hi) =
            bounds(tris).ok_or_else(|| CommandError::Failed("the model is empty".into()))?;
        let x_axis = rotary.along == Along::X;
        let probe = Self {
            x_axis,
            middle: Point::ZERO,
            start: 0.0,
            scale: 1.0,
            first: 0.0,
            unrolled: Unrolled {
                along_from: 0.0,
                round_from: 0.0,
                round: 0.0,
                x_axis,
            },
        };
        let middle = probe.across((lo + hi) / 2.0);
        let length = probe.along(hi) - probe.along(lo);
        let reach = tris
            .iter()
            .flatten()
            .map(|&v| (probe.across(v) - middle).length())
            .fold(0.0, f64::max);
        if length <= 0.0 || reach <= 0.0 {
            return Err(CommandError::Failed(
                "the model has no length or width".into(),
            ));
        }
        let job = doc.job.area();
        let unrolled = if x_axis {
            Unrolled {
                along_from: job.min.x,
                round_from: job.min.y,
                round: job.size().y,
                x_axis,
            }
        } else {
            Unrolled {
                along_from: job.min.y,
                round_from: job.min.x,
                round: job.size().x,
                x_axis,
            }
        };
        let job_length = if x_axis { job.size().x } else { job.size().y };
        let scale = if params.fit_material {
            (rotary.diameter / 2.0 / reach).min(job_length / length)
        } else if params.size.x > 0.0 {
            params.size.x / length
        } else {
            1.0
        };
        let first = if params.placing.center {
            unrolled.along_from + (job_length - length * scale) / 2.0
        } else if x_axis {
            params.placing.position.x
        } else {
            params.placing.position.y
        };
        Ok(Self {
            middle,
            start: probe.along(lo),
            scale,
            first,
            unrolled,
            ..probe
        })
    }

    fn along(&self, v: DVec3) -> f64 {
        if self.x_axis { v.x } else { v.y }
    }

    fn across(&self, v: DVec3) -> Point {
        if self.x_axis {
            Point::new(v.y, v.z)
        } else {
            Point::new(v.x, v.z)
        }
    }

    /// A vertex as (along, angle round from straight up, out from the axis), in job mm.
    fn cylindrical(&self, v: DVec3) -> (f64, f64, f64) {
        let off = self.across(v) - self.middle;
        (
            self.first + (self.along(v) - self.start) * self.scale,
            off.x.atan2(off.y),
            off.length() * self.scale,
        )
    }

    /// A vertex where the turned model has it: along the job, across and up from the axis.
    fn placed(&self, v: DVec3) -> DVec3 {
        let off = (self.across(v) - self.middle) * self.scale;
        let along = self.first + (self.along(v) - self.start) * self.scale;
        if self.x_axis {
            DVec3::new(along, off.x, off.y)
        } else {
            DVec3::new(off.x, along, off.y)
        }
    }
}

/// A full 3D model wrapped round a rotary job's axis: each cell of the unrolled job holding
/// how far out from the axis the model reaches at that point along and that angle round.
/// Angle 0 is straight up, turning toward +Y for an X axis or +X for a Y axis [inf].
fn wrapped(
    doc: &Document,
    rotary: Rotary,
    tris: &[[DVec3; 3]],
    params: &Params,
) -> Result<Component, CommandError> {
    use std::f64::consts::{PI, TAU};
    let turning = Turning::of(doc, rotary, tris, params)?;
    let unrolled = &turning.unrolled;
    let mut made = Component::from_fn(
        "Imported model",
        Combine::Add,
        doc.job.area(),
        doc.job.model_cell(),
        |_| None,
    );
    let cylindrical = |v: DVec3| turning.cylindrical(v);
    for tri in tris {
        let mut corners = tri.map(cylindrical);
        // one side of the seam: the angles taken within half a turn of the first
        for i in 1..3 {
            let turn = corners[i].1 - corners[0].1;
            corners[i].1 -= TAU * ((turn + PI) / TAU).floor();
        }
        let corners = corners
            .map(|(a, angle, out)| (a, unrolled.round_from + angle / TAU * unrolled.round, out));
        unrolled.draw(&mut made, corners);
    }
    Ok(made)
}

/// The unrolled job's axes: where the axis and the circumference start, how far round it
/// is, and which way the axis runs.
struct Unrolled {
    along_from: f64,
    round_from: f64,
    round: f64,
    x_axis: bool,
}

impl Unrolled {
    /// Draws one triangle given as (along, round, out) into `made`'s cells, keeping the
    /// farthest out; cells past the circumference wrap round.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_precision_loss,
        reason = "cell indices are small"
    )]
    fn draw(&self, made: &mut Component, corners: [(f64, f64, f64); 3]) {
        let cell = made.cell;
        let [first, second, third] = corners;
        let det =
            (second.0 - first.0) * (third.1 - first.1) - (third.0 - first.0) * (second.1 - first.1);
        if det.abs() < 1e-12 {
            return;
        }
        let span = |pick: fn(&(f64, f64, f64)) -> f64| {
            corners
                .iter()
                .map(pick)
                .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), x| {
                    (lo.min(x), hi.max(x))
                })
        };
        let (along_lo, along_hi) = span(|c| c.0);
        let (round_lo, round_hi) = span(|c| c.1);
        let index = |x: f64, from: f64| ((x - from) / cell - 0.5).ceil() as isize;
        let rows_round = ((self.round / cell).ceil() as isize).max(1);
        for i in index(along_lo, self.along_from)..=index(along_hi, self.along_from) {
            let at_along = self.along_from + (i as f64 + 0.5) * cell;
            for j in index(round_lo, self.round_from)..=index(round_hi, self.round_from) {
                let at_round = self.round_from + (j as f64 + 0.5) * cell;
                let w_second = ((at_along - first.0) * (third.1 - first.1)
                    - (third.0 - first.0) * (at_round - first.1))
                    / det;
                let w_third = ((second.0 - first.0) * (at_round - first.1)
                    - (at_along - first.0) * (second.1 - first.1))
                    / det;
                let w_first = 1.0 - w_second - w_third;
                if w_first < -1e-9 || w_second < -1e-9 || w_third < -1e-9 {
                    continue;
                }
                let out = w_first * first.2 + w_second * second.2 + w_third * third.2;
                let wrapped_row = j.rem_euclid(rows_round);
                let (column, row) = if self.x_axis {
                    (i, wrapped_row)
                } else {
                    (wrapped_row, i)
                };
                let (Ok(column), Ok(row)) = (usize::try_from(column), usize::try_from(row)) else {
                    continue;
                };
                if column >= made.columns || row >= made.rows {
                    continue;
                }
                let h = &mut made.heights[row * made.columns + column];
                // the higher of what is there and this (NaN, no shape yet, loses to any height)
                *h = h.max(out);
            }
        }
    }
}

fn bounds(tris: &[[DVec3; 3]]) -> Option<(DVec3, DVec3)> {
    tris.iter().flatten().fold(None, |b, v| {
        Some(b.map_or((*v, *v), |(lo, hi): (DVec3, DVec3)| {
            (lo.min(*v), hi.max(*v))
        }))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A 20 × 10 × 5 box (binary STL, all six faces) in a temporary file of its own.
    fn box_file(name: &str) -> std::path::PathBuf {
        let (w, d, h) = (20.0_f32, 10.0_f32, 5.0_f32);
        let v = |x: f32, y: f32, z: f32| [x, y, z];
        let quad = |a: [f32; 3], b: [f32; 3], c: [f32; 3], e: [f32; 3]| [[a, b, c], [a, c, e]];
        let faces: Vec<[[f32; 3]; 3]> = [
            quad(v(0.0, 0.0, h), v(w, 0.0, h), v(w, d, h), v(0.0, d, h)),
            quad(
                v(0.0, 0.0, 0.0),
                v(0.0, d, 0.0),
                v(w, d, 0.0),
                v(w, 0.0, 0.0),
            ),
            quad(
                v(0.0, 0.0, 0.0),
                v(w, 0.0, 0.0),
                v(w, 0.0, h),
                v(0.0, 0.0, h),
            ),
            quad(v(0.0, d, 0.0), v(0.0, d, h), v(w, d, h), v(w, d, 0.0)),
            quad(
                v(0.0, 0.0, 0.0),
                v(0.0, 0.0, h),
                v(0.0, d, h),
                v(0.0, d, 0.0),
            ),
            quad(v(w, 0.0, 0.0), v(w, d, 0.0), v(w, d, h), v(w, 0.0, h)),
        ]
        .concat();
        let mut out = vec![0u8; 80];
        out.extend(u32::try_from(faces.len()).unwrap().to_le_bytes());
        for t in faces {
            out.extend([0u8; 12]);
            for p in t {
                for x in p {
                    out.extend(x.to_le_bytes());
                }
            }
            out.extend([0u8; 2]);
        }
        let path =
            std::env::temp_dir().join(format!("cut-craft-box-{}-{name}.stl", std::process::id()));
        std::fs::write(&path, out).unwrap();
        path
    }

    fn with_file(name: &str, p: &Params) -> Params {
        Params {
            file: box_file(name).display().to_string(),
            ..p.clone()
        }
    }

    #[test]
    fn contract() {
        cut_craft_core::testing::id_is_a_slug_and_meta_is_labelled::<Orientate3dModel>();
        cut_craft_core::testing::schema_describes_an_object::<Orientate3dModel>();
        cut_craft_core::testing::default_params_round_trip_through_json::<Orientate3dModel>();
        let doc = Document::default();
        let change =
            Orientate3dModel::run(&doc, &with_file("contract", &Params::default())).unwrap();
        let mut done = doc.clone();
        done.apply(&change).unwrap();
        done.apply(&change.inverse()).unwrap();
        assert_eq!(done, doc);
        assert!(
            Orientate3dModel::run(&doc, &Params::default()).is_err(),
            "no file"
        );
    }

    fn imported(name: &str, p: &Params) -> (Document, Component) {
        let mut doc = Document::default();
        let change = Orientate3dModel::run(&doc, &with_file(name, p)).unwrap();
        doc.apply(&change).unwrap();
        let k = doc.components.values().next().unwrap().clone();
        (doc, k)
    }

    #[test]
    fn undercuts_are_noted_and_a_plain_box_has_none() {
        let doc = Document::default();
        let plain = Orientate3dModel::run(&doc, &with_file("plain", &Params::default())).unwrap();
        assert_eq!(plain.notes, Vec::<String>::new());
        // the box on a stalk: its underside is 5 mm clear of the stalk's foot
        let (s, b) = (2.0_f32, 5.0_f32);
        let v = |x: f32, y: f32, z: f32| [x, y, z];
        let mut faces = vec![
            [v(0.0, 0.0, 0.0), v(0.0, s, 0.0), v(s, 0.0, 0.0)],
            [v(0.0, 0.0, b), v(0.0, 20.0, b), v(20.0, 0.0, b)],
            [
                v(0.0, 0.0, b + 3.0),
                v(20.0, 0.0, b + 3.0),
                v(0.0, 20.0, b + 3.0),
            ],
        ];
        faces.push([v(0.0, 0.0, 0.0), v(s, 0.0, 0.0), v(0.0, 0.0, b)]);
        let mut out = vec![0u8; 80];
        out.extend(u32::try_from(faces.len()).unwrap().to_le_bytes());
        for t in &faces {
            out.extend([0u8; 12]);
            out.extend(t.iter().flatten().flat_map(|x| x.to_le_bytes()));
            out.extend([0u8; 2]);
        }
        let path = std::env::temp_dir().join(format!("cut-craft-stalk-{}.stl", std::process::id()));
        std::fs::write(&path, out).unwrap();
        let p = Params {
            file: path.display().to_string(),
            ..Params::default()
        };
        let change = Orientate3dModel::run(&doc, &p).unwrap();
        assert_eq!(change.notes.len(), 1, "{:?}", change.notes);
        assert!(change.notes[0].contains("2.0 cm²"), "{:?}", change.notes);
    }

    #[test]
    fn centred_at_the_file_size_resting_on_the_bed() {
        let (doc, k) = imported("centred", &Params::default());
        let middle = doc.job.area().center();
        let top = k.height_at(middle).unwrap();
        assert!(
            (top - 5.0).abs() < 1e-6,
            "the 5 high box from the bed up: {top}"
        );
        let e = k.extent();
        assert!(
            (e.size().x - 20.0).abs() < 2.0 * k.cell && (e.size().y - 10.0).abs() < 2.0 * k.cell
        );
        assert!((e.center() - middle).length() < 2.0 * k.cell);
    }

    #[test]
    fn front_face_up_turns_depth_into_height_and_size_rescales_all_three() {
        let (_, k) = imported(
            "front",
            &Params {
                orientation: Orientation {
                    face: Face::Front,
                    rotation: 0.0,
                },
                size: ModelSize {
                    x: 40.0,
                    ..Params::default().size
                },
                placing: Placing {
                    height_above_bed: 2.0,
                    discard_below_zero: false,
                    ..Params::default().placing
                },
                ..Params::default()
            },
        );
        let e = k.extent();
        // 20 wide becomes 40 (ratio locked: the 5 deep front face is 10 tall now)
        assert!((e.size().x - 40.0).abs() < 2.0 * k.cell, "{e:?}");
        assert!((e.size().y - 10.0).abs() < 2.0 * k.cell, "{e:?}");
        let (_, hi) = k.height_range().unwrap();
        assert!(
            (hi - 22.0).abs() < 1e-6,
            "the 10 tall face doubled, from 2 up: {hi}"
        );
    }

    #[test]
    fn z_sets_the_height_alone_or_with_the_ratio_locked_all_three() {
        let size = |lock_ratio| ModelSize {
            lock_ratio,
            z: 10.0,
            ..Params::default().size
        };
        let (_, k) = imported(
            "z_alone",
            &Params {
                size: size(false),
                ..Params::default()
            },
        );
        assert!((k.height_range().unwrap().1 - 10.0).abs() < 1e-6);
        assert!(
            (k.extent().size().x - 20.0).abs() < 2.0 * k.cell,
            "footprint kept"
        );
        let (_, k) = imported(
            "z_locked",
            &Params {
                size: size(true),
                ..Params::default()
            },
        );
        assert!((k.height_range().unwrap().1 - 10.0).abs() < 1e-6);
        assert!(
            (k.extent().size().x - 40.0).abs() < 2.0 * k.cell,
            "doubled with it"
        );
    }

    #[test]
    fn a_plain_box_previews_whole_and_reachable() {
        // the box's bottom faces down but is its base, not an overhang
        let file = box_file("preview");
        let p = Params {
            file: file.display().to_string(),
            ..Params::default()
        };
        let doc = Document::default();
        let overhangs = |p: &Params| {
            preview(&doc, p)
                .unwrap()
                .unwrap()
                .iter()
                .filter(|(_, over)| *over)
                .count()
        };
        assert_eq!(overhangs(&p), 0, "a box's bottom rests on the bed");
        let tris = preview(&doc, &p).unwrap().unwrap();
        assert_eq!(tris.len(), 12);
        assert!(
            tris.iter()
                .flat_map(|(t, _)| t)
                .all(|v| v.z >= 0.0 && v.z <= 5.0)
        );
        let _ = std::fs::remove_file(file);
    }

    #[test]
    fn inches_scale_and_discarding_below_zero() {
        let (_, k) = imported(
            "inches",
            &Params {
                size: ModelSize {
                    units: FileUnits::Inches,
                    ..Params::default().size
                },
                ..Params::default()
            },
        );
        assert!((k.extent().size().x - 20.0 * 25.4).abs() < 2.0 * k.cell);
        // sunk 100 into the bed, the 5 high box is all below the zero plane and discarded,
        // so the import refuses
        let p = with_file(
            "deep",
            &Params {
                placing: Placing {
                    height_above_bed: -100.0,
                    ..Params::default().placing
                },
                ..Params::default()
            },
        );
        assert!(Orientate3dModel::run(&Document::default(), &p).is_err());
    }
}

#[cfg(test)]
mod rotary {
    use super::*;
    use cut_craft_core::document::Job;

    /// Binary STL of `faces` in a temporary file of its own.
    fn stl(name: &str, faces: &[[[f32; 3]; 3]]) -> String {
        let mut out = vec![0u8; 80];
        out.extend(u32::try_from(faces.len()).unwrap().to_le_bytes());
        for t in faces {
            out.extend([0u8; 12]);
            for x in t.iter().flatten() {
                out.extend(x.to_le_bytes());
            }
            out.extend([0u8; 2]);
        }
        let path = std::env::temp_dir().join(format!(
            "cut-craft-rotary-{}-{name}.stl",
            std::process::id()
        ));
        std::fs::write(&path, out).unwrap();
        path.display().to_string()
    }

    /// A closed cylinder of `radius` and `length` along X, from 48 flat sides.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_precision_loss,
        reason = "test geometry in f32"
    )]
    fn cylinder(radius: f64, length: f64) -> Vec<[[f32; 3]; 3]> {
        let sides = 48;
        let at = |i: usize, x: f64| {
            let angle = std::f64::consts::TAU * (i % sides) as f64 / sides as f64;
            [
                x as f32,
                (radius * angle.cos()) as f32,
                (radius * angle.sin()) as f32,
            ]
        };
        let mut faces = vec![];
        for i in 0..sides {
            let (near, next, next_far, far) =
                (at(i, 0.0), at(i + 1, 0.0), at(i + 1, length), at(i, length));
            faces.push([near, next, next_far]);
            faces.push([near, next_far, far]);
            faces.push([[0.0, 0.0, 0.0], next, near]);
            faces.push([[length as f32, 0.0, 0.0], far, next_far]);
        }
        faces
    }

    fn rotary_job() -> Document {
        Document {
            job: Job::rotary(100.0, 30.0, Along::X),
            ..Document::default()
        }
    }

    fn import(name: &str, faces: &[[[f32; 3]; 3]], p: &Params) -> (Document, Component) {
        let mut doc = rotary_job();
        let p = Params {
            file: stl(name, faces),
            ..p.clone()
        };
        let change = Orientate3dModel::run(&doc, &p).unwrap();
        doc.apply(&change).unwrap();
        let k = doc.components.values().next().unwrap().clone();
        (doc, k)
    }

    /// A box from `lo` to `hi`, as twelve triangles facing out.
    fn cuboid(lo: [f32; 3], hi: [f32; 3]) -> Vec<[[f32; 3]; 3]> {
        let at = |i: usize| {
            [0, 1, 2].map(|axis| {
                if i >> axis & 1 == 1 {
                    hi[axis]
                } else {
                    lo[axis]
                }
            })
        };
        // each face's corners counter-clockwise seen from outside, by corner number (bit
        // 0 x, 1 y, 2 z)
        [
            [0, 2, 3, 1],
            [4, 5, 7, 6],
            [0, 1, 5, 4],
            [2, 6, 7, 3],
            [0, 4, 6, 2],
            [1, 3, 7, 5],
        ]
        .iter()
        .flat_map(|&[a, b, c, d]| [[at(a), at(b), at(c)], [at(a), at(c), at(d)]])
        .collect()
    }

    #[test]
    fn a_full_model_previews_turned_round_the_axis_as_it_is_wrapped() {
        let p = Params {
            model_type: ModelType::Full,
            ..Params::default()
        };
        let faces = cylinder(10.0, 50.0);
        let (doc, k) = import("turned", &faces, &p);
        let p = Params {
            file: stl("turned", &faces),
            ..p
        };
        let tris = preview(&rotary_job(), &p).unwrap().expect("shown turned");
        assert!(
            tris.iter().all(|(_, hidden)| !hidden),
            "cut all the way round"
        );
        let corners = || tris.iter().flat_map(|(t, _)| t.iter().copied());
        // fitted to the 30 mm stock: out 15 from the axis, as the wrapped model reaches
        let out = corners().map(|v| v.y.hypot(v.z)).fold(0.0, f64::max);
        assert!((out - 15.0).abs() < 0.1, "{out}");
        let (_, highest) = k.height_range().unwrap();
        assert!((highest - out).abs() < 0.5, "{highest} against {out}");
        // 75 long (1.5 times), centred along the 100 mm job
        let job = doc.job.area();
        let along = |f: fn(f64, f64) -> f64, from| corners().map(|v| v.x).fold(from, f);
        let (first, last) = (
            along(f64::min, f64::INFINITY),
            along(f64::max, f64::NEG_INFINITY),
        );
        // the file is in f32
        assert!((last - first - 75.0).abs() < 1e-4, "{first}..{last}");
        assert!(
            ((first - job.min.x) - (job.max.x - last)).abs() < 1e-6,
            "centred"
        );
    }

    #[test]
    fn the_preview_marks_an_overhang_and_what_is_under_it() {
        // a post 10 high with a shelf off its top, over a low step on the bed
        let post = cuboid([0.0, 0.0, 0.0], [10.0, 10.0, 10.0]);
        let shelf = cuboid([10.0, 0.0, 8.0], [30.0, 10.0, 10.0]);
        let step = cuboid([10.0, 0.0, 0.0], [30.0, 10.0, 2.0]);
        let faces: Vec<_> = [post, shelf, step].concat();
        let file = stl("shelf", &faces);
        let p = Params {
            file: file.clone(),
            ..Params::default()
        };
        let doc = Document::default();
        let tris = preview(&doc, &p).unwrap().unwrap();
        let red = |pick: &dyn Fn(&[DVec3; 3]) -> bool| {
            tris.iter().filter(|(t, _)| pick(t)).all(|(_, red)| *red)
        };
        let on = |t: &[DVec3; 3], z: f64| t.iter().all(|v| (v.z - z).abs() < 1e-9);
        assert!(red(&|t| on(t, 8.0)), "the shelf's underside");
        assert!(red(&|t| on(t, 2.0)), "the step's top, under the shelf");
        assert!(
            tris.iter()
                .filter(|(t, _)| on(t, 10.0))
                .all(|(_, red)| !red),
            "the tops are reached"
        );
        assert!(
            tris.iter().filter(|(t, _)| on(t, 0.0)).all(|(_, red)| !red),
            "the base rests on the bed"
        );
        let _ = std::fs::remove_file(file);
    }

    #[test]
    fn a_rotary_job_starts_on_a_full_model() {
        assert_eq!(
            Orientate3dModel::initial_params(&rotary_job()).model_type,
            ModelType::Full
        );
        assert_eq!(
            Orientate3dModel::initial_params(&Document::default()).model_type,
            ModelType::Flat
        );
    }

    #[test]
    fn a_cylinder_wraps_to_its_radius_all_round_and_its_length_along() {
        let p = Params {
            model_type: ModelType::Full,
            fit_material: false,
            ..Params::default()
        };
        let (doc, k) = import("cylinder", &cylinder(10.0, 50.0), &p);
        let job = doc.job.area();
        for turn in [0.05, 0.3, 0.55, 0.8, 0.97] {
            let at = Point::new(50.0, job.min.y + job.size().y * turn);
            let z = k.height_at(at).unwrap();
            assert!((z - 10.0).abs() < 0.15, "{turn}: {z}");
        }
        assert_eq!(
            k.height_at(Point::new(20.0, 40.0)),
            None,
            "the cylinder spans x 25..75"
        );
        assert!(k.height_at(Point::new(27.0, 40.0)).is_some());
    }

    #[test]
    fn fitting_the_material_scales_it_to_the_job_diameter() {
        let p = Params {
            model_type: ModelType::Full,
            ..Params::default()
        };
        let (doc, k) = import("fit", &cylinder(10.0, 50.0), &p);
        let (_, hi) = k.height_range().unwrap();
        assert!((hi - 15.0).abs() < 0.05, "{hi}");
        // 1.5 times as long: 75 of the 100
        assert!(k.height_at(Point::new(14.0, 40.0)).is_some());
        assert_eq!(k.height_at(Point::new(11.0, 40.0)), None);
        assert!(doc.job.rotary.is_some());
    }

    #[test]
    fn a_flat_model_in_a_rotary_job_is_placed_like_clipart() {
        let p = Params {
            model_type: ModelType::Flat,
            ..Params::default()
        };
        let (doc, k) = import("flat", &cylinder(10.0, 50.0), &p);
        assert!(k.extent().size().y < doc.job.area().size().y / 2.0);
    }
}
