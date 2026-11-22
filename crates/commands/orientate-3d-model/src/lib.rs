//! Orientate 3D Model: stand a 3D model the right way up, size it and place it in the material
//! before it becomes a component. Which
//! face is up, a turn about Z, the model's size and units, centring, how high above the bed
//! it sits, and whether anything below the zero plane is kept.

use cut_craft_core::component::{Combine, Component, Source};
use cut_craft_core::document::{Along, Document, Job, ModelPosition, Rotary};
use cut_craft_core::geometry::{Point, Rect};
use cut_craft_core::glam::{DMat3, DVec3};
use cut_craft_core::params::Xy;
use cut_craft_core::{Change, Command, CommandError, EMPTY_MODEL, Meta};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::time::Instant;

pub struct Orientate3dModel;

/// The last model file read, kept so the live form's changes do not read and parse it
/// again: its path, its size and modified time (a file written again is read again), its
/// MD5 and its triangles as drawn. Kept only when the file's last change is [`SETTLED`]
/// older than the read: a file saved again within the clock's tick, the same size, keeps
/// its date (Windows dates changes to a tick of about 16 ms, FAT to 2 s).
// ponytail: one file kept; picking another file drops it
struct ReadFile {
    path: String,
    stamp: Option<(u64, std::time::SystemTime)>,
    md5: String,
    triangles: Arc<Vec<[DVec3; 3]>>,
}

static READ: Mutex<Option<ReadFile>> = Mutex::new(None);

/// How much older than its read a file's last change must be for the read to be kept.
const SETTLED: std::time::Duration = std::time::Duration::from_secs(3);

/// A file's triangles as drawn, shared with the cache, and its MD5.
type Read = (Arc<Vec<[DVec3; 3]>>, String);

/// The file's triangles as drawn and its MD5, read once per file.
fn read_file(path: &str) -> Result<Read, CommandError> {
    let stamp = std::fs::metadata(path)
        .ok()
        .and_then(|m| Some((m.len(), m.modified().ok()?)));
    if let Ok(kept) = READ.lock()
        && let Some(r) = kept
            .as_ref()
            .filter(|r| r.path == path && r.stamp == stamp && stamp.is_some())
    {
        return Ok((Arc::clone(&r.triangles), r.md5.clone()));
    }
    let started = Instant::now();
    let read_at = std::time::SystemTime::now();
    let bytes = std::fs::read(path).map_err(|e| CommandError::Failed(format!("{path}: {e}")))?;
    // a generated border from the Library: its depth map, at the size its recipe says
    let tile = std::path::Path::new(path);
    let triangles = if cut_craft_border_tile::of_depth_map(tile).is_some() {
        cut_craft_border_tile::triangles(tile).map_err(CommandError::Failed)?
    } else {
        fmt_import_stl::triangles(&bytes).map_err(CommandError::Failed)?
    };
    let md5 = Source::md5(&bytes);
    log::info!(
        "Orientate 3D Model: read {path}: {} triangles in {} ms",
        triangles.len(),
        started.elapsed().as_millis()
    );
    let triangles = Arc::new(triangles);
    let settled = stamp.is_some_and(|(_, modified)| modified + SETTLED <= read_at);
    if let Ok(mut kept) = READ.lock() {
        *kept = settled.then(|| ReadFile {
            path: path.to_owned(),
            stamp,
            md5: md5.clone(),
            triangles: Arc::clone(&triangles),
        });
    }
    Ok((triangles, md5))
}

/// Which face of the model as drawn is the top.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Face {
    /// The top as drawn stays on top
    #[default]
    Top,
    /// Turned over, so its underside faces up
    Bottom,
    /// The side facing you as drawn turns up
    Front,
    /// The side facing away as drawn turns up
    Back,
    /// The left side as drawn turns up
    Left,
    /// The right side as drawn turns up
    Right,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum FileUnits {
    /// The file was drawn in millimetres
    #[default]
    Mm,
    /// The file was drawn in inches
    Inches,
}

/// Which face is up, then a turn about Z.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Orientation {
    /// The face of the model as drawn that becomes its top
    #[schemars(
        title = "Face up",
        extend("x-widget" = "faces"),
        extend("x-icon" = "import/initial_orientation.png")
    )]
    pub face: Face,
    /// Then a turn about the upright axis, in degrees
    #[schemars(
        title = "Rotation about Z Axis",
        extend("x-unit" = "deg", "x-axis" = "z"),
        extend("x-steps" = [-180, -90, 0, 90, 180])
    )]
    pub rotation: f64,
}

/// The model's size, the ratio lock and the file's units.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct ModelSize {
    /// Keep the model's proportions: the size changed sets the others
    #[schemars(title = "Lock XYZ ratio", extend("x-enable-when" = {"change_size": "true"}))]
    pub lock_ratio: bool,
    /// The model's length once placed. Typing one scales the model once; turning it after
    /// keeps that scale
    #[schemars(title = "X", extend("x-unit" = "mm", "x-enable-when" = {"change_size": "true"}))]
    pub x: f64,
    /// The model's width once placed. Typing one scales the model once; turning it after
    /// keeps that scale
    #[schemars(title = "Y", extend("x-unit" = "mm", "x-enable-when" = {"change_size": "true"}))]
    pub y: f64,
    /// The model's height once placed. Typing one scales the model once; turning it after
    /// keeps that scale
    #[schemars(title = "Z", extend("x-unit" = "mm", "x-enable-when" = {"change_size": "true"}))]
    #[serde(default)]
    pub z: f64,
    /// The numbers in the file are in these units
    #[schemars(extend("x-widget" = "row"))]
    pub units: FileUnits,
    /// Unlocks the size: X, Y, Z and Scale to Fit Material can change it. Off, the model
    /// keeps the size it has, so nothing resizes it by accident
    #[schemars(title = "Change size")]
    #[serde(default)]
    pub change_size: bool,
    /// How much the model is scaled from the file's size, along the job's X, Y and Z: set
    /// once by a size typed or Scale to Fit Material, and kept while the model is turned.
    /// Not in the form: the size boxes show what it makes
    #[schemars(skip)]
    #[serde(default = "unscaled")]
    pub scale: [f64; 3],
}

fn unscaled() -> [f64; 3] {
    [1.0; 3]
}

/// Where the model sits relative to the modelling plane.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Placing {
    /// Centred on the material; the form shows it as the position it gives and clears it,
    /// so it is not in the form (the alignment buttons centre it)
    #[schemars(skip)]
    pub center: bool,
    /// Where the model's lower-left corner goes when not centred
    pub position: Xy,
    /// Where Align in Material put the model; kept while it is resized (Scale model to fit
    /// material, or a new size), so it stays where the alignment put it. Not in the form:
    /// the buttons set it, and a position or depth typed clears it
    #[schemars(skip)]
    #[serde(default)]
    pub align: Align,
    /// How far below the model's top the zero plane (the material's surface) cuts it: what
    /// is above the plane becomes the component; 0 keeps the whole model, sitting on the
    /// plane. More than the model's height lifts it off the plane, up to the material's top
    #[schemars(title = "Depth Below Top", extend("x-unit" = "mm"))]
    #[serde(default)]
    pub depth_below_top: f64,
    /// In a double-sided job, the model seen from below goes to the other side too
    #[schemars(title = "Create both sides")]
    #[serde(default)]
    pub both_sides: bool,
    /// Throw away the part of the model below the zero plane, keeping only what is above
    #[schemars(title = "Discard data below zero plane")]
    pub discard_below_zero: bool,
}

/// Where Align in Material put the model, each as a share of the room the material leaves
/// beside it, measured from the material's corner: across 0 is its left side on the
/// material's left and 1 its right side on the material's right; up likewise from the
/// material's bottom edge; height 0 is its bottom on the zero plane and 1 its top at the
/// material's top. `None` leaves that one to the position or depth given.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Align {
    /// Left to right
    #[serde(default)]
    pub across: Option<f64>,
    /// Front to back (the material's bottom edge to its top edge)
    #[serde(default)]
    pub up: Option<f64>,
    /// In the material's thickness
    #[serde(default)]
    pub height: Option<f64>,
}

impl Align {
    /// Where the lower-left corner of a model `footprint` big goes in `area`: aligned where
    /// an alignment is kept, else at `position`.
    #[must_use]
    pub fn corner(&self, area: Rect, footprint: Point, position: Point) -> Point {
        let at = |share: Option<f64>, from: f64, room: f64, need: f64, typed: f64| {
            share.map_or(typed, |a| from + (room - need) * a)
        };
        Point::new(
            at(
                self.across,
                area.min.x,
                area.size().x,
                footprint.x,
                position.x,
            ),
            at(self.up, area.min.y, area.size().y, footprint.y, position.y),
        )
    }

    /// The Depth Below Top that puts a model `tall` high at the kept height in material
    /// `thickness` thick, if a height is kept.
    #[must_use]
    pub fn depth(&self, tall: f64, thickness: f64) -> Option<f64> {
        self.height.map(|up| tall + (thickness - tall) * up)
    }
}

/// Fine turns about the job's axes after the initial
/// orientation, which a drag in the 3D view adds to while the drag turns the model.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Turn {
    /// What a left drag in the 3D view turns: the view, or the model about all three axes
    /// or about one (the view then looks along it)
    #[schemars(
        title = "Drag turns",
        extend("x-widget" = "named-icons"),
        extend("x-icon" = "import/turn_{}.png")
    )]
    pub mode: TurnMode,
    /// How far the model is turned about the X axis, in degrees
    #[schemars(title = "X", extend("x-unit" = "deg"))]
    pub x: f64,
    /// How far the model is turned about the Y axis, in degrees
    #[schemars(title = "Y", extend("x-unit" = "deg"))]
    pub y: f64,
    /// How far the model is turned about the Z axis, in degrees
    #[schemars(title = "Z", extend("x-unit" = "deg"))]
    pub z: f64,
}

/// What a drag in the 3D view turns while Orientate 3D Model is open.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TurnMode {
    /// The view, as always
    #[default]
    #[schemars(title = "XYZ view")]
    View,
    /// The model about all three axes: sideways about Z, up and down about X
    #[schemars(title = "XYZ model")]
    Xyz,
    /// The model about the X axis only (tips it forward and back)
    #[schemars(title = "X")]
    X,
    /// The model about the Y axis only (rolls it side to side)
    #[schemars(title = "Y")]
    Y,
    /// The model about the Z axis only (spins it flat)
    #[schemars(title = "Z")]
    Z,
}

/// A full 3D model off a rotary job's axis, which sits
/// through the middle of the model by default.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct AxisMove {
    /// A pan drag in the 3D view moves the model off the axis instead of moving the view
    #[schemars(title = "Drag moves the axis")]
    pub drag: bool,
    /// How far the model's middle sits across from the axis
    #[schemars(title = "Across", extend("x-unit" = "mm"))]
    pub across: f64,
    /// How far the model's middle sits above the axis
    #[schemars(title = "Up", extend("x-unit" = "mm"))]
    pub up: f64,
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
    /// Colour the faces a cutter from above cannot reach (turned down, or under an
    /// overhang) in the 3D view while the form is open
    #[schemars(title = "Highlight undercuts")]
    #[serde(default = "yes")]
    pub show_undercuts: bool,
    /// A full 3D model is kept whole: in a double-sided job its underside goes to the other
    /// side, and in a rotary job it is wrapped round the axis, its distance out at each
    /// angle becoming the height. A flat model is its top seen from above, placed like
    /// clipart. Shown when Options ▸ Importing and Exporting offers both
    #[schemars(title = "Imported model type", extend("x-hide-when" = {"offered": ["full", "flat"]}))]
    #[serde(default)]
    pub model_type: ModelType,
    /// Scaled to fit the material again on every change, keeping its proportions: as big as
    /// its length, width and thickness allow (a rotary job's length and diameter). Not in the
    /// form, which scales to fit once instead (its Scale to Fit Material button, kept in
    /// `size.scale`); kept for settings saved before, and for scripts
    #[schemars(extend("x-hide-when" = {"rotary": ["true", "false"]}))]
    #[serde(default)]
    pub fit_material: bool,
    /// What Options ▸ Importing and Exporting offers; never shown. Set by the app when the form
    /// opens; one kind alone also sets `model_type`
    #[schemars(extend("x-hide-when" = {"offered": ["full", "flat", "both"]}))]
    #[serde(default)]
    pub offered: Offered,
    #[schemars(title = "Interactive Rotation", extend("x-section" = true))]
    #[serde(default)]
    pub turn: Turn,
    /// Shown in a rotary job
    #[schemars(
        title = "Rotation Axis Movement",
        extend("x-section" = true),
        extend("x-show-when" = {"rotary": ["true"]})
    )]
    #[serde(default)]
    pub axis: AxisMove,
    /// Where the model sits in the material's thickness when it is cut: its top flush with
    /// the material's top or a gap under it (Gap Above Model), or its bottom on the
    /// material's bottom or a gap over it (Gap Below Model). The gap above and the gap below
    /// add up to the material's thickness less the model's. The same setting as Material
    /// Setup's, for the whole job
    #[schemars(
        title = "Model Position in Material",
        extend("x-section" = true, "x-hide-when" = {"rotary": ["true"]})
    )]
    #[serde(default)]
    pub in_material: ModelPosition,
    /// The job is rotary; set by the app when the form opens, never shown
    #[schemars(extend("x-hide-when" = {"rotary": ["true", "false"]}))]
    #[serde(default)]
    pub rotary: bool,
}

/// What Options ▸ Importing and Exporting lets the form offer.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Offered {
    /// Only full 3D models are offered
    Full,
    /// Only flat models are offered
    Flat,
    /// Both kinds are offered, to pick from
    #[default]
    Both,
}

fn yes() -> bool {
    true
}

/// What an imported model is kept as.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ModelType {
    /// The whole model: both sides of a double-sided job, all the way round a rotary one
    #[default]
    #[schemars(title = "Full 3D model")]
    Full,
    /// Its top seen from above, like clipart
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
                change_size: false,
                scale: unscaled(),
            },
            placing: Placing {
                center: true,
                position: Xy::new(0.0, 0.0),
                align: Align::default(),
                depth_below_top: 0.0,
                both_sides: false,
                discard_below_zero: true,
            },
            in_material: ModelPosition::default(),
            show_undercuts: true,
            model_type: ModelType::Full,
            fit_material: false,
            offered: Offered::Both,
            turn: Turn::default(),
            axis: AxisMove::default(),
            rotary: false,
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

    /// Every job starts on a full 3D model at the file's size, its size locked: Scale to Fit
    /// Material is a step the user takes.
    fn initial_params(doc: &Document) -> Params {
        Params {
            in_material: doc.job.model_position,
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
            c.add_new_component(Component { source, ..k });
            return Ok(c.finish());
        }
        let (tris, rect) = placed(doc, p, tris)?;
        let started = Instant::now();
        let mut k = Component::from_fn(
            "Imported model",
            Combine::Add,
            rect,
            doc.job.model_cell(),
            |_| None,
        );
        fmt_import_stl::top_down(&mut k, &tris);
        log::info!(
            "Orientate 3D Model: {} triangles into {} x {} cells in {} ms",
            tris.len(),
            k.columns,
            k.rows,
            started.elapsed().as_millis()
        );
        if p.placing.discard_below_zero {
            for h in &mut k.heights {
                if *h < 0.0 {
                    *h = f64::NAN;
                }
            }
        }
        if k.height_range().is_none() {
            return Err(CommandError::Failed(
                "nothing of the model is above the zero plane. Move the model up, or the zero plane down".into(),
            ));
        }
        k.source = source;
        let mut c = Change::builder(doc);
        c.add_new_component(k);
        // where it sits in the material: the job's, as Material Setup sets it
        if doc.job.model_position != p.in_material {
            c.set_job(Job {
                model_position: p.in_material,
                ..doc.job
            });
        }
        if let Some(two) = doc.job.double_sided
            && p.model_type == ModelType::Full
            && p.placing.both_sides
        {
            // the model seen from the other side: turned over as the material is, up from
            // that side's face
            let mirror = two.flip.mirror(doc.job.area());
            let turn = |v: DVec3| {
                let xy = mirror.transform_point2(Point::new(v.x, v.y));
                DVec3::new(xy.x, xy.y, doc.job.thickness - v.z)
            };
            let under: Vec<[DVec3; 3]> = tris.iter().map(|t| t.map(turn)).collect();
            let (near, far) = (
                mirror.transform_point2(rect.min),
                mirror.transform_point2(rect.max),
            );
            let footprint = Rect::new(near.min(far), near.max(far));
            let mut other_side = Component::from_fn(
                "Imported model (other side)",
                Combine::Add,
                footprint,
                doc.job.model_cell(),
                |_| None,
            );
            fmt_import_stl::top_down(&mut other_side, &under);
            if p.placing.discard_below_zero {
                for h in &mut other_side.heights {
                    if *h < 0.0 {
                        *h = f64::NAN;
                    }
                }
            }
            if other_side.height_range().is_some() {
                let (bottom, mut objects) = c.side().clone();
                objects.components.insert(c.fresh_id(), other_side);
                c.set_side(bottom, objects);
            }
        }
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
    let (triangles, md5) = read_file(&p.file)?;
    // the fine turns after the initial orientation: about X, then Y, then Z [inf]
    let fine = DMat3::from_rotation_z(p.turn.z.to_radians())
        * DMat3::from_rotation_y(p.turn.y.to_radians())
        * DMat3::from_rotation_x(p.turn.x.to_radians());
    let turn = fine
        * DMat3::from_rotation_z(p.orientation.rotation.to_radians())
        * face_up(p.orientation.face);
    let unit = match p.size.units {
        FileUnits::Mm => 1.0,
        FileUnits::Inches => 25.4,
    };
    let turned = triangles
        .iter()
        .map(|t| t.map(|v| turn * v * unit))
        .collect();
    Ok((turned, md5))
}

/// `tris` sized and placed in a flat job (heights up from the bed), and the footprint
/// they cover.
fn placed(
    doc: &Document,
    p: &Params,
    mut tris: Vec<[DVec3; 3]>,
) -> Result<(Vec<[DVec3; 3]>, Rect), CommandError> {
    let (lo, hi) = bounds(&tris).ok_or_else(|| CommandError::Failed(EMPTY_MODEL.into()))?;
    let size = hi - lo;
    if size.x <= 0.0 || size.y <= 0.0 {
        return Err(CommandError::Failed(
            "the model has no area seen from above. Turn it so a wide side faces up".into(),
        ));
    }
    let area = doc.job.area();
    // size: 0 keeps the file's; with the ratio locked the first size given sets all
    let wanted = |want: f64, have: f64| (want > 0.0 && have > 0.0).then(|| want / have);
    let each = [
        wanted(p.size.x, size.x),
        wanted(p.size.y, size.y),
        wanted(p.size.z, size.z),
    ];
    let scale = if p.fit_material {
        DVec3::splat(fits_flat(doc, size))
    } else if each.iter().all(Option::is_none) {
        // the scale set once, kept whichever way the model is turned
        DVec3::from(p.size.scale)
    } else if p.size.lock_ratio {
        DVec3::splat(each.into_iter().flatten().next().unwrap_or(1.0))
    } else {
        let [x, y, z] = each.map(|s| s.unwrap_or(1.0));
        DVec3::new(x, y, z)
    };
    let footprint = Point::new(size.x * scale.x, size.y * scale.y);
    // the zero plane Depth Below Top down from the model's top: its lowest point sits that
    // far below it, less the model's height. Past the model's height, the model is lifted
    // off the plane, up to the material's top
    let tall = size.z * scale.z;
    let depth = p
        .placing
        .align
        .depth(tall, doc.job.thickness)
        .unwrap_or(p.placing.depth_below_top);
    let bottom = if depth > 0.0 {
        depth.min(tall.max(doc.job.thickness)) - tall
    } else {
        0.0
    };
    // measured from the material's corner: an alignment chosen is kept whatever the size
    let corner = if p.placing.center {
        area.center() - footprint / 2.0
    } else {
        p.placing
            .align
            .corner(area, footprint, p.placing.position.into())
    };
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

/// As big as a flat job's material allows a model `size` big, every axis at the same rate
/// until one meets it.
fn fits_flat(doc: &Document, size: DVec3) -> f64 {
    let area = doc.job.area();
    let room = [area.size().x, area.size().y, doc.job.thickness];
    let fits = room
        .into_iter()
        .zip([size.x, size.y, size.z])
        .filter(|&(have, need)| have > 0.0 && need > 0.0)
        .map(|(have, need)| have / need)
        .fold(f64::INFINITY, f64::min);
    if fits.is_finite() { fits } else { 1.0 }
}

/// Scale to Fit Material, worked out once: how much the file's model, turned as `p` turns
/// it, is scaled to be as big as the material allows (a rotary job's length and radius for
/// a full 3D model). The form keeps it as the model's scale, so turning the model after
/// does not size it again.
///
/// # Errors
/// When the file cannot be read, or the model has no size.
pub fn fit_scale(doc: &Document, p: &Params) -> Result<f64, CommandError> {
    let tris = oriented(p)?.0;
    if let Some(rotary) = doc.job.rotary
        && p.model_type == ModelType::Full
    {
        let fitting = Params {
            fit_material: true,
            ..p.clone()
        };
        return Ok(Turning::of(doc, rotary, &tris, &fitting)?.scale);
    }
    let (lo, hi) = bounds(&tris).ok_or_else(|| CommandError::Failed(EMPTY_MODEL.into()))?;
    Ok(fits_flat(doc, hi - lo))
}

/// A triangle of the previewed model, and whether a cutter from above cannot reach it.
pub type Seen = ([DVec3; 3], bool);

/// The model as a flat job places it, as triangles in job millimetres, each with whether
/// a cutter from above cannot reach it (the model is over it: an overhang's underside, or
/// what is tucked under one): what the 3D view shows while the form is open. A full 3D
/// model in a rotary job is shown turned round the job's axis instead (x along it, y and z
/// across from it), with nothing marked: it is cut all the way round.
pub fn preview(doc: &Document, params: &Params) -> Result<Option<Vec<Seen>>, CommandError> {
    preview_over(doc, params, None)
}

/// [`preview`] judging reach by `top`, the component these settings just made, rather than
/// rasterising the model again; `None` rasterises it.
pub fn preview_over(
    doc: &Document,
    params: &Params,
    top: Option<&Component>,
) -> Result<Option<Vec<Seen>>, CommandError> {
    // ponytail: a tenth of a millimetre clear of the base counts, as fmt_import_stl::undercut
    const ABOVE: f64 = 0.1;
    /// How far out from a face its reach is judged, in cells: past its own cell.
    const AWAY: f64 = 1.5;
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
    if !params.show_undercuts {
        return Ok(Some(tris.into_iter().map(|t| (t, false)).collect()));
    }
    let started = Instant::now();
    let made;
    let top = if let Some(k) = top {
        k
    } else {
        made = {
            let mut k = Component::from_fn("", Combine::Add, rect, doc.job.model_cell(), |_| None);
            fmt_import_stl::top_down(&mut k, &tris);
            k
        };
        &made
    };
    let lowest = tris
        .iter()
        .flatten()
        .map(|v| v.z)
        .fold(f64::INFINITY, f64::min);
    let hidden = |t: &[DVec3; 3]| {
        let [a, b, c] = *t;
        let (low, high) = t
            .iter()
            .map(|v| v.z)
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), z| {
                (lo.min(z), hi.max(z))
            });
        // the base, and what sits on it, has nothing under it to reach
        if low <= lowest + ABOVE {
            return false;
        }
        let normal = (b - a).cross(c - a).normalize_or_zero();
        // turned down: an overhang's underside
        if normal.z < 0.0 {
            return true;
        }
        // judged by the air beside it, a cell and a half out down the slope (a wall's own
        // cell, or a steep face's, is higher than its middle, which is no undercut): tucked
        // under when the surface there is higher than all of the face
        let middle = (a + b + c) / 3.0;
        let away = DVec3::new(normal.x, normal.y, 0.0).normalize_or_zero() * AWAY * top.cell;
        let over = top.height_at(Point::new(middle.x + away.x, middle.y + away.y));
        over.is_some_and(|z| z > high + top.cell)
    };
    let seen: Vec<Seen> = tris.iter().map(|t| (*t, hidden(t))).collect();
    log::info!(
        "Orientate 3D Model: preview judged {} of {} faces out of reach in {} ms",
        seen.iter().filter(|(_, hidden)| *hidden).count(),
        seen.len(),
        started.elapsed().as_millis()
    );
    Ok(Some(seen))
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
    /// How far the model's middle sits across and up from the axis (Rotation Axis
    /// Movement), in job mm.
    shift: Point,
    unrolled: Unrolled,
}

impl Turning {
    fn of(
        doc: &Document,
        rotary: Rotary,
        tris: &[[DVec3; 3]],
        params: &Params,
    ) -> Result<Self, CommandError> {
        let (lo, hi) = bounds(tris).ok_or_else(|| CommandError::Failed(EMPTY_MODEL.into()))?;
        let x_axis = rotary.along == Along::X;
        let probe = Self {
            x_axis,
            middle: Point::ZERO,
            start: 0.0,
            scale: 1.0,
            first: 0.0,
            shift: Point::ZERO,
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
                "the model has no length or width. Turn it so a wide side faces up".into(),
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
            // the scale set once (the same every way round the axis), kept as it turns
            params.size.scale[0]
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
            shift: Point::new(params.axis.across, params.axis.up),
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
        let off = (self.across(v) - self.middle) * self.scale + self.shift;
        (
            self.first + (self.along(v) - self.start) * self.scale,
            off.x.atan2(off.y),
            off.length(),
        )
    }

    /// A vertex in the part's own frame, which turns with A: along the job's axis, then
    /// across it and up from it at A=0.
    fn part(&self, v: DVec3) -> DVec3 {
        let off = (self.across(v) - self.middle) * self.scale + self.shift;
        let along = self.first + (self.along(v) - self.start) * self.scale;
        DVec3::new(along, off.x, off.y)
    }

    /// A vertex where the turned model has it: along the job, across and up from the axis.
    fn placed(&self, v: DVec3) -> DVec3 {
        let off = (self.across(v) - self.middle) * self.scale + self.shift;
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
    // kept as it really is round the axis too, for rotary toolpaths to cut round
    let solid: Vec<[DVec3; 3]> = tris.iter().map(|t| t.map(|v| turning.part(v))).collect();
    made.set_solid(&solid);
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
mod tests;

#[cfg(test)]
#[path = "tests/rotary.rs"]
mod rotary;

#[cfg(test)]
#[test]
#[ignore = "by hand: CUT_CRAFT_MODEL names an STL to measure"]
fn measure_an_import() {
    let Ok(file) = std::env::var("CUT_CRAFT_MODEL") else {
        return;
    };
    let doc = Document::default();
    let p = Params {
        file,
        ..Orientate3dModel::initial_params(&doc)
    };
    println!(
        "job: {} x {} x {} mm",
        doc.job.width, doc.job.height, doc.job.thickness
    );
    let seen = preview(&doc, &p).unwrap().unwrap();
    let zs: Vec<f64> = seen
        .iter()
        .flat_map(|(t, _)| t.iter().map(|v| v.z))
        .collect();
    let (lo, hi) = zs
        .iter()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), &z| {
            (a.min(z), b.max(z))
        });
    println!("preview triangles: {}, z {lo:.3} .. {hi:.3}", seen.len());
    let change = Orientate3dModel::run(&doc, &p).unwrap();
    let mut made = doc.clone();
    made.apply(&change).unwrap();
    let k = made.components.values().next().unwrap();
    let hs: Vec<f64> = k.heights.iter().copied().filter(|h| !h.is_nan()).collect();
    let (hlo, hhi) = hs
        .iter()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), &z| {
            (a.min(z), b.max(z))
        });
    println!(
        "component {} x {} cells of {:.3} mm, {} shaped, heights {hlo:.3} .. {hhi:.3}",
        k.columns,
        k.rows,
        k.cell,
        hs.len()
    );
    // cells much lower than the cells round them: needles
    let at = |c: usize, r: usize| k.heights[r * k.columns + c];
    let mut needles = 0;
    for r in 1..k.rows - 1 {
        for c in 1..k.columns - 1 {
            let h = at(c, r);
            if h.is_nan() {
                continue;
            }
            let around = [at(c - 1, r), at(c + 1, r), at(c, r - 1), at(c, r + 1)];
            if around
                .iter()
                .all(|a| !a.is_nan() && *a - h > 0.25 * (hhi - hlo))
            {
                needles += 1;
            }
        }
    }
    let edge_low = (0..k.columns)
        .flat_map(|c| [0, k.rows - 1].map(|r| at(c, r)))
        .filter(|h| !h.is_nan() && *h < hlo + 0.05 * (hhi - hlo))
        .count();
    println!(
        "needles: {needles}; low cells on the top and bottom rows: {edge_low} of {}",
        2 * k.columns
    );
    println!(
        "settings: {}",
        serde_json::to_string(&p.placing).unwrap_or_default()
    );
    // the heights up from the front edge, in the middle and a third across
    for c in [k.columns / 2, k.columns / 3] {
        let col: Vec<String> = (0..14).map(|r| format!("{:.2}", at(c, r))).collect();
        println!("column {c} from the front: {}", col.join(" "));
    }
    let r = k.rows / 2;
    let row: Vec<String> = (0..14).map(|c| format!("{:.2}", at(c, r))).collect();
    println!("row {r} from the left: {}", row.join(" "));
}
