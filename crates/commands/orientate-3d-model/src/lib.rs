//! Orientate 3D Model: Aspire's step between picking an STL and having a component. Which
//! face is up, a turn about Z, the model's size and units, centring, how far its top sits
//! below the material surface, and whether anything below the zero plane is kept.

use cut_craft_core::component::{Combine, Component};
use cut_craft_core::document::Document;
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

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    /// The STL file (binary or ASCII)
    pub file: String,
    #[schemars(title = "Initial orientation")]
    pub orientation: Face,
    #[schemars(title = "Rotation about Z axis", extend("x-unit" = "deg"))]
    pub rotation: f64,
    /// The numbers in the file are in these units
    pub units: FileUnits,
    /// The model's footprint once placed; 0 keeps the file's size
    #[schemars(title = "Model size")]
    pub size: Xy,
    /// Scale Z with X and Y; otherwise only the footprint changes
    #[schemars(title = "Lock XYZ ratio")]
    pub lock_ratio: bool,
    #[schemars(title = "Center model")]
    pub center: bool,
    /// Where the model's lower-left corner goes when not centred
    pub position: Xy,
    /// How far the model's highest point sits below the material surface
    #[schemars(title = "Depth below top", extend("x-unit" = "mm"))]
    pub depth_below_top: f64,
    #[schemars(title = "Discard data below zero plane")]
    pub discard_below_zero: bool,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            file: String::new(),
            orientation: Face::Top,
            rotation: 0.0,
            units: FileUnits::Mm,
            size: Xy::new(0.0, 0.0),
            lock_ratio: true,
            center: true,
            position: Xy::new(0.0, 0.0),
            depth_below_top: 0.0,
            discard_below_zero: true,
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
            icon: "modeling/import_component_3d_model.svg",
            shortcut: "",
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        if p.file.is_empty() {
            return Err(CommandError::InvalidParams("choose an STL file".into()));
        }
        let bytes =
            std::fs::read(&p.file).map_err(|e| CommandError::Failed(format!("{}: {e}", p.file)))?;
        let triangles = fmt_import_stl::triangles(&bytes).map_err(CommandError::Failed)?;
        let turn = DMat3::from_rotation_z(p.rotation.to_radians()) * face_up(p.orientation);
        let unit = match p.units {
            FileUnits::Mm => 1.0,
            FileUnits::Inches => 25.4,
        };
        let mut tris: Vec<[DVec3; 3]> = triangles
            .iter()
            .map(|t| t.map(|v| turn * v * unit))
            .collect();
        let (lo, hi) =
            bounds(&tris).ok_or_else(|| CommandError::Failed("the model is empty".into()))?;
        let size = hi - lo;
        if size.x <= 0.0 || size.y <= 0.0 {
            return Err(CommandError::Failed(
                "the model has no area seen from above".into(),
            ));
        }
        // size: 0 keeps the file's; with the ratio locked X sets all, else X and Y each
        let (sx, sy) = (
            if p.size.x > 0.0 {
                p.size.x / size.x
            } else {
                1.0
            },
            if p.size.y > 0.0 {
                p.size.y / size.y
            } else {
                1.0
            },
        );
        let scale = if p.lock_ratio {
            DVec3::splat(sx)
        } else {
            DVec3::new(sx, sy, 1.0)
        };
        let area = doc.job.area();
        let footprint = Point::new(size.x * scale.x, size.y * scale.y);
        let corner = if p.center {
            area.center() - footprint / 2.0
        } else {
            p.position.into()
        };
        let top = doc.job.thickness - p.depth_below_top;
        let place = |v: DVec3| {
            DVec3::new(
                corner.x + (v.x - lo.x) * scale.x,
                corner.y + (v.y - lo.y) * scale.y,
                top - (hi.z - v.z) * scale.z,
            )
        };
        for t in &mut tris {
            *t = t.map(place);
        }
        let rect = Rect::new(corner, corner + footprint);
        let mut k = Component::from_fn(
            "Imported model",
            Combine::Add,
            rect,
            doc.job.model_cell(),
            |_| None,
        );
        fmt_import_stl::top_down(&mut k, &tris);
        if p.discard_below_zero {
            for h in &mut k.heights {
                *h = h.filter(|z| *z >= 0.0);
            }
        }
        if k.height_range().is_none() {
            return Err(CommandError::Failed(
                "nothing of the model is above the zero plane".into(),
            ));
        }
        let mut c = Change::builder(doc);
        c.add_component(k);
        Ok(c.finish())
    }
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
    fn centred_at_the_file_size_with_its_top_at_the_surface() {
        let (doc, k) = imported("centred", &Params::default());
        let middle = doc.job.area().center();
        let top = k.height_at(middle).unwrap();
        assert!((top - doc.job.thickness).abs() < 1e-6, "{top}");
        let e = k.extent();
        assert!(
            (e.size().x - 20.0).abs() < 2.0 * k.cell && (e.size().y - 10.0).abs() < 2.0 * k.cell
        );
        assert!((e.center() - middle).length() < 2.0 * k.cell);
    }

    #[test]
    fn front_face_up_turns_depth_into_height_and_size_rescales() {
        let (_, k) = imported(
            "front",
            &Params {
                orientation: Face::Front,
                size: Xy::new(40.0, 0.0),
                depth_below_top: 2.0,
                discard_below_zero: false,
                ..Params::default()
            },
        );
        let e = k.extent();
        // 20 wide becomes 40 (ratio locked: the 5 deep front face is 10 tall now)
        assert!((e.size().x - 40.0).abs() < 2.0 * k.cell, "{e:?}");
        assert!((e.size().y - 10.0).abs() < 2.0 * k.cell, "{e:?}");
        let (_, hi) = k.height_range().unwrap();
        assert!((hi - 18.0).abs() < 1e-6, "top 2 below a 20 thick job: {hi}");
    }

    #[test]
    fn inches_scale_and_discarding_below_zero() {
        let (_, k) = imported(
            "inches",
            &Params {
                units: FileUnits::Inches,
                ..Params::default()
            },
        );
        assert!((k.extent().size().x - 20.0 * 25.4).abs() < 2.0 * k.cell);
        // top at 20 − 100 = −80: everything is below the zero plane and would be discarded,
        // so the import refuses
        let p = with_file(
            "deep",
            &Params {
                depth_below_top: 100.0,
                ..Params::default()
            },
        );
        assert!(Orientate3dModel::run(&Document::default(), &p).is_err());
    }
}
