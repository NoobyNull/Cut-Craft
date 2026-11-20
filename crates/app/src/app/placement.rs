//! Model Position in Material, on Material Setup's form and Import 3D Model's: under the
//! From and Gap settings, a picture of the model in the material drawn to scale as they
//! change, both gaps said (they share what the model leaves of the material), and one-click
//! Top at material top and Bottom on material bottom. A model that pokes out of the
//! material is said in red at the foot of the form, never refused.

use cut_craft_bug_report::hint::HoverHint;
use cut_craft_core::document::{GapFrom, ModelPosition, Poke, Units};
use cut_craft_i18n::tr;
use eframe::egui;
use serde_json::Value;

/// The section's title on both forms.
pub(super) const SECTION: &str = "Model Position in Material";

/// What the section needs from outside its form: the job's material and the model.
#[derive(Clone, Copy, Debug)]
pub(super) struct Room {
    /// The job's material thickness, mm (Material Setup's own field wins over it).
    pub thickness: f64,
    /// The model's height from its lowest point to its highest, mm; 0 with no model.
    pub tall: f64,
    pub units: Units,
    /// A rotary job: the gaps are round the axis, so no flat picture is drawn.
    pub rotary: bool,
}

/// The key the form keeps the position under, the material's thickness and the model's
/// height as the form has them now; `None` on any other form.
fn reading(params: &Value, room: Room) -> Option<(&'static str, f64, f64)> {
    if params.get("in_material").is_some() {
        return Some(("in_material", room.thickness, room.tall));
    }
    if params.get("position").is_some() && params.get("model_thickness").is_some() {
        let thickness = params["thickness"].as_f64().unwrap_or(room.thickness);
        // a Model Thickness typed in scales the model to it on Apply
        let typed = params["model_thickness"].as_f64().unwrap_or(0.0);
        let tall = if typed > 0.0 { typed } else { room.tall };
        return Some(("position", thickness, tall));
    }
    None
}

fn position(params: &Value, key: &str) -> Option<ModelPosition> {
    serde_json::from_value(params[key].clone()).ok()
}

/// The red words for a model that pokes out of the material, with what to do about it;
/// `None` when it fits, there is no model yet, or the form has no Model Position.
pub(super) fn warning(params: &Value, room: Room) -> Option<String> {
    if room.rotary {
        return None;
    }
    let (key, thickness, tall) = reading(params, room)?;
    if tall <= 0.0 {
        return None;
    }
    let p = position(params, key)?;
    let len = |mm: f64| room.units.length(mm, 1);
    if tall > thickness + 1e-6 {
        return Some(format!(
            "⚠ The model is {} tall but the material is only {} thick, so it sticks out of \
             the material whatever the gap. Use thicker material, make the model lower \
             (Model Thickness), or cut it into boards with Model ▸ Slice Model.",
            len(tall),
            len(thickness)
        ));
    }
    let room_left = thickness - tall;
    match p.sticks_out(thickness, tall)? {
        Poke::Top(by) => Some(format!(
            "⚠ The model sticks {} out of the top of the material: the cutter would only \
             cut air there. The two gaps can add up to {} at most; make the gap {} or less.",
            len(by),
            len(room_left),
            len(room_left)
        )),
        Poke::Bottom(by) => Some(format!(
            "⚠ The model goes {} past the bottom of the material: the bit would cut into \
             your spoilboard there. The two gaps can add up to {} at most; make the gap {} \
             or less.",
            len(by),
            len(room_left),
            len(room_left)
        )),
    }
}

/// Draws the foot of the Model Position in Material section: the picture, both gaps and
/// the two one-click places. Whether the settings changed.
pub(super) fn section_end(
    ui: &mut egui::Ui,
    section: &str,
    params: &mut Value,
    room: Room,
) -> bool {
    if section != SECTION || room.rotary {
        return false;
    }
    let Some((key, thickness, tall)) = reading(params, room) else {
        return false;
    };
    let Some(p) = position(params, key) else {
        return false;
    };
    let len = |mm: f64| room.units.length(mm, 2);
    picture(ui, p, thickness, tall);
    if tall > 0.0 {
        let (above, below) = p.gaps(thickness, tall);
        let hint = "The two gaps always add up to the material's thickness less the model's \
                    height: change one and the other follows";
        let said = |gap: f64| {
            if gap < 0.0 {
                format!("{} (sticks out)", len(gap))
            } else {
                len(gap)
            }
        };
        ui.label(tr!("placement-gap-above", gap = said(above)))
            .on_hover_hint(hint);
        ui.label(tr!("placement-gap-below", gap = said(below)))
            .on_hover_hint(hint);
    } else {
        ui.weak(tr!("placement-no-3d-model-yet-the"));
    }
    let mut picked = None;
    ui.horizontal_wrapped(|ui| {
        if ui
            .button(tr!("placement-top-at-material-top"))
            .on_hover_hint(tr!("placement-the-model-s-highest-point"))
            .clicked()
        {
            picked = Some(ModelPosition {
                from: GapFrom::Above,
                gap: 0.0,
            });
        }
        if ui
            .button(tr!("placement-bottom-on-material-bottom"))
            .on_hover_hint(tr!("placement-the-model-s-lowest-point"))
            .clicked()
        {
            picked = Some(ModelPosition {
                from: GapFrom::Below,
                gap: 0.0,
            });
        }
    });
    match picked {
        Some(new) if Some(new) != position(params, key) => {
            params[key] = serde_json::to_value(new).expect("a position serializes");
            true
        }
        _ => false,
    }
}

/// Rows of text the picture is tall.
const PICTURE_ROWS: f32 = 4.0;

/// The material seen from the front, the model in it to scale where the settings put it,
/// the gaps marked; anything poking out of the material in red.
fn picture(ui: &mut egui::Ui, p: ModelPosition, thickness: f64, tall: f64) {
    let height = ui.spacing().interact_size.y * PICTURE_ROWS;
    let width = ui.available_width().min(height * 3.0);
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::hover());
    if thickness <= 0.0 {
        return;
    }
    let visuals = ui.visuals();
    let (stroke, red) = (visuals.text_color(), visuals.error_fg_color);
    let painter = ui.painter_at(rect);
    // room above and below for a model that pokes out
    let (above, below) = p.gaps(thickness, tall);
    let over = (-above).max(0.0);
    let under = (-below).max(0.0);
    let span = thickness + over + under;
    #[expect(clippy::cast_possible_truncation, reason = "a picture's pixels")]
    let scale = (rect.height() - 4.0) / span as f32;
    #[expect(clippy::cast_possible_truncation, reason = "a picture's pixels")]
    let y = |z_from_top: f64| rect.top() + 2.0 + (over + z_from_top) as f32 * scale;
    let block = egui::Rect::from_x_y_ranges(
        rect.left() + width * 0.05..=rect.right() - width * 0.05,
        y(0.0)..=y(thickness),
    );
    painter.rect_filled(block, 0.0, visuals.faint_bg_color);
    painter.rect_stroke(
        block,
        0.0,
        egui::Stroke::new(1.0, stroke),
        egui::StrokeKind::Inside,
    );
    if tall <= 0.0 {
        return;
    }
    // the model: a dome, its top `above` down from the material's top
    let (top, bottom) = (y(above), y(above + tall));
    let left = block.left() + block.width() * 0.25;
    let right = block.right() - block.width() * 0.25;
    let points: Vec<egui::Pos2> = (0..=24)
        .map(|i| {
            let t = f64::from(i) / 24.0;
            let rise = (t * std::f64::consts::PI).sin().powf(0.6);
            #[expect(clippy::cast_possible_truncation, reason = "a picture's pixels")]
            let x = left + (right - left) * t as f32;
            #[expect(clippy::cast_possible_truncation, reason = "a picture's pixels")]
            let yy = bottom - (bottom - top) * rise as f32;
            egui::pos2(x, yy)
        })
        .chain([egui::pos2(right, bottom), egui::pos2(left, bottom)])
        .collect();
    let out = over > 0.0 || under > 0.0;
    let fill = if out {
        red.gamma_multiply(0.5)
    } else {
        visuals.selection.bg_fill
    };
    painter.add(egui::Shape::convex_polygon(
        points,
        fill,
        egui::Stroke::new(1.0, if out { red } else { stroke }),
    ));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn room(tall: f64) -> Room {
        Room {
            thickness: 20.0,
            tall,
            units: Units::Mm,
            rotary: false,
        }
    }

    fn import(from: &str, gap: f64) -> Value {
        serde_json::json!({ "in_material": { "from": from, "gap": gap } })
    }

    #[test]
    fn a_model_that_fits_has_no_warning() {
        assert_eq!(warning(&import("above", 0.0), room(10.0)), None);
        assert_eq!(warning(&import("below", 10.0), room(10.0)), None);
        assert_eq!(
            warning(&import("below", 18.0), room(0.0)),
            None,
            "no model yet"
        );
    }

    #[test]
    fn a_gap_past_the_room_left_says_which_face_and_the_most_it_can_be() {
        let top = warning(&import("below", 12.0), room(10.0)).unwrap();
        assert!(top.contains("2.0 mm out of the top"), "{top}");
        assert!(top.contains("10.0 mm or less"), "{top}");
        let bottom = warning(&import("above", 13.0), room(10.0)).unwrap();
        assert!(bottom.contains("3.0 mm past the bottom"), "{bottom}");
        assert!(bottom.contains("spoilboard"), "{bottom}");
    }

    #[test]
    fn a_model_taller_than_the_material_points_to_slice_model() {
        let said = warning(&import("above", 0.0), room(25.0)).unwrap();
        assert!(said.contains("Slice Model"), "{said}");
        assert!(said.contains("25.0 mm tall"), "{said}");
    }

    #[test]
    fn material_setup_reads_its_own_thickness_and_a_typed_model_thickness() {
        let p = serde_json::json!({
            "thickness": 30.0,
            "model_thickness": 0.0,
            "position": { "from": "below", "gap": 15.0 }
        });
        assert_eq!(
            warning(&p, room(10.0)),
            None,
            "30 thick on the form, not 20"
        );
        let mut p = p;
        p["model_thickness"] = 20.0.into();
        assert!(warning(&p, room(10.0)).unwrap().contains("out of the top"));
    }

    #[test]
    fn inches_are_said_in_inches() {
        let r = Room {
            units: Units::Inch,
            ..room(10.0)
        };
        let said = warning(&import("above", 12.7 + 10.0), r).unwrap();
        assert!(said.contains("0.500 in past the bottom"), "{said}");
    }

    #[test]
    fn rotary_jobs_are_left_alone() {
        let r = Room {
            rotary: true,
            ..room(10.0)
        };
        assert_eq!(warning(&import("above", 30.0), r), None);
    }
}
