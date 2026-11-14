//! Where a drag knife can cut: on a flat surface only. Its blade trails behind and cuts a
//! fixed depth below the top it rides on, so the top must be the material's untouched
//! surface (the knife cut first) or one a Surfacing toolpath has just flattened. Anything
//! else cut before it (a pocket, a V-carve, a 3D finish) leaves the top uneven, and the
//! knife would dig in or lift out. The user can say otherwise: the knife following the 3D
//! model (a dome, curved work), or a surface they know is flat.

use crate::document::{Document, EntityId};
use crate::geometry::Point;
use crate::toolpath::{Toolpath, cut_points};

/// The Drag Knife strategy's id.
pub const STRATEGY: &str = "drag_knife_toolpath";

/// The Surfacing strategy's id: what flattens the top.
pub const SURFACING: &str = "surfacing_toolpath";

/// What the knife is told to do whatever is cut before it, by the user.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Overrides {
    /// It follows the 3D model's surface (a dome, curved or rotary work).
    pub follows_model: bool,
    /// The user knows the surface is flat under the knife.
    pub uneven_allowed: bool,
}

impl Overrides {
    /// The overrides a drag knife toolpath's (or form's) settings ask for.
    pub fn of(params: &serde_json::Value) -> Self {
        Self {
            follows_model: params["project_onto_model"] == true,
            uneven_allowed: params["allow_uneven_surface"] == true,
        }
    }
}

/// How to tell the knife to cut anyway, for the end of a refusal.
const OVERRIDE: &str = "If the surface under the knife really is flat, tick \"Cut on an \
                        uneven surface anyway\"; to follow a curved surface (a dome, rotary \
                        work), tick \"Project toolpath onto 3D Model\"";

/// The height of the top the knife rides on (0 is the material's surface as set up; lower
/// once Surfacing took some off), given the toolpaths cut before it (`before`, in list
/// order) and the vectors it cuts; or why it must not cut, saying what to do.
pub fn surface_top(
    doc: &Document,
    name: &str,
    before: &[&Toolpath],
    vectors: &[EntityId],
    overrides: Overrides,
) -> Result<f64, String> {
    if overrides.follows_model {
        // the model is the surface: the knife's depth is below it
        return Ok(0.0);
    }
    // other drag knives before it cut the top without changing it
    let last = before.iter().rev().find(|t| t.strategy != STRATEGY);
    let surfaced = before.iter().rev().find(|t| t.strategy == SURFACING);
    match last {
        None => Ok(0.0),
        Some(surfacing) if surfacing.strategy == SURFACING => {
            match flattened(doc, surfacing, vectors) {
                Ok(top) => Ok(top),
                Err(_) if overrides.uneven_allowed => Ok(0.0),
                Err(outside) => Err(format!(
                    "{name} runs outside the shapes {} flattened, where the top is {} \
                     higher, so the blade would ride over the step. Flatten the whole \
                     material in {}, or cut only inside the flattened shapes. {OVERRIDE}",
                    surfacing.name,
                    doc.job.units.length(outside, 2),
                    surfacing.name
                )),
            }
        }
        Some(_) if overrides.uneven_allowed => {
            Ok(surfaced.map_or(0.0, |s| flattened(doc, s, vectors).unwrap_or(0.0)))
        }
        Some(other) => Err(format!(
            "{name} needs a flat surface, but {} is cut before it and leaves the top uneven, \
             so the blade would dig in or lift out. Move {name} to the top of the toolpath \
             list, or straight after a Surfacing toolpath. {OVERRIDE}",
            other.name
        )),
    }
}

/// The top Surfacing toolpath `surfacing` leaves under the knife's `vectors`: its depth
/// below the surface; or, where part of them is outside the shapes it flattened, the height
/// of the step there.
fn flattened(doc: &Document, surfacing: &Toolpath, vectors: &[EntityId]) -> Result<f64, f64> {
    let params = &surfacing.params;
    let depth = params["depth"].as_f64().unwrap_or(0.0).max(0.0);
    match params["area"].as_str() {
        // the spoilboard is flattened with nothing on it: the material goes on after
        Some("spoilboard") => Ok(0.0),
        Some("selected_vectors") => {
            let rings: Vec<Vec<Point>> = surfacing
                .vectors
                .iter()
                .filter_map(|id| doc.vectors.get(id))
                .filter(|p| p.closed)
                .map(cut_points)
                .collect();
            let all_in = vectors
                .iter()
                .filter_map(|id| doc.vectors.get(id))
                .flat_map(cut_points)
                .all(|p| crate::geometry::inside(&rings, p));
            if all_in { Ok(-depth) } else { Err(depth) }
        }
        _ => Ok(-depth),
    }
}

/// Whether drag knife toolpath `toolpath` of `doc` may not cut where it is in the list, as
/// a sentence: what Calculate refused, said again by the checks before cutting after the
/// list was reordered.
pub fn misplaced(doc: &Document, toolpath: &Toolpath) -> Option<String> {
    if toolpath.strategy != STRATEGY {
        return None;
    }
    let before: Vec<&Toolpath> = doc
        .toolpaths
        .values()
        .take_while(|t| !std::ptr::eq(*t, toolpath))
        .collect();
    surface_top(
        doc,
        &toolpath.name,
        &before,
        &toolpath.vectors,
        Overrides::of(&toolpath.params),
    )
    .err()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Path;
    use crate::testing::square;
    use crate::toolpath::Tool;

    fn made(name: &str, strategy: &str, params: serde_json::Value) -> Toolpath {
        Toolpath {
            name: name.into(),
            strategy: strategy.into(),
            params,
            tool: Tool::default(),
            vectors: vec![],
            moves: vec![],
            visible: true,
            power: vec![],
            stale: false,
            group: None,
            drill_bank: None,
            fallbacks: vec![],
        }
    }

    fn surfacing(area: &str, depth: f64) -> Toolpath {
        made(
            "Surfacing",
            SURFACING,
            serde_json::json!({ "area": area, "depth": depth }),
        )
    }

    fn knife() -> Toolpath {
        made("Drag Knife", STRATEGY, serde_json::json!({}))
    }

    fn pocket() -> Toolpath {
        made("Pocket 1", "pocket_toolpath", serde_json::json!({}))
    }

    fn top(doc: &Document, before: &[&Toolpath], overrides: Overrides) -> Result<f64, String> {
        surface_top(doc, "Drag Knife", before, &[], overrides)
    }

    #[test]
    fn the_first_toolpath_cuts_the_untouched_surface() {
        assert_eq!(
            top(&Document::default(), &[], Overrides::default()),
            Ok(0.0)
        );
    }

    #[test]
    fn after_another_cut_it_refuses_and_says_how_to_fix_it() {
        let why = top(&Document::default(), &[&pocket()], Overrides::default()).unwrap_err();
        assert!(why.contains("Pocket 1 is cut before it"), "{why}");
        assert!(why.contains("Move Drag Knife to the top"), "{why}");
        assert!(why.contains("straight after a Surfacing toolpath"), "{why}");
        assert!(why.contains("Cut on an uneven surface anyway"), "{why}");
    }

    #[test]
    fn straight_after_surfacing_it_rides_on_the_flattened_top() {
        let doc = Document::default();
        let s = surfacing("whole_material", 0.5);
        assert_eq!(top(&doc, &[&s], Overrides::default()), Ok(-0.5));
        // other knives between change nothing
        let k = knife();
        assert_eq!(top(&doc, &[&s, &k, &k], Overrides::default()), Ok(-0.5));
        // the spoilboard is flattened before the material goes on
        let board = surfacing("spoilboard", 0.5);
        assert_eq!(top(&doc, &[&board], Overrides::default()), Ok(0.0));
    }

    #[test]
    fn surfacing_earlier_in_the_list_does_not_count_once_something_else_cuts() {
        let doc = Document::default();
        let s = surfacing("whole_material", 0.5);
        assert!(top(&doc, &[&s, &pocket()], Overrides::default()).is_err());
        assert!(top(&doc, &[&pocket(), &s], Overrides::default()).is_ok());
    }

    #[test]
    fn the_user_can_say_cut_anyway_or_follow_the_model() {
        let doc = Document::default();
        let p = pocket();
        let anyway = Overrides {
            uneven_allowed: true,
            ..Overrides::default()
        };
        assert_eq!(top(&doc, &[&p], anyway), Ok(0.0));
        let s = surfacing("whole_material", 0.25);
        assert_eq!(top(&doc, &[&s, &p], anyway), Ok(-0.25));
        let model = Overrides {
            follows_model: true,
            ..Overrides::default()
        };
        assert_eq!(top(&doc, &[&p], model), Ok(0.0));
        assert_eq!(
            Overrides::of(&serde_json::json!({ "allow_uneven_surface": true })),
            anyway
        );
    }

    #[test]
    fn outside_the_shapes_surfacing_flattened_is_a_step() {
        let mut doc = Document::default();
        let (slab, inner, out) = (EntityId(1), EntityId(2), EntityId(3));
        doc.vectors.insert(slab, square(0.0, 0.0, 100.0));
        doc.vectors.insert(inner, square(10.0, 10.0, 20.0));
        doc.vectors.insert(
            out,
            Path::polyline(&[Point::new(50.0, 50.0), Point::new(150.0, 50.0)], false),
        );
        let mut s = surfacing("selected_vectors", 1.0);
        s.vectors = vec![slab];
        let o = Overrides::default();
        assert_eq!(
            surface_top(&doc, "Drag Knife", &[&s], &[inner], o),
            Ok(-1.0)
        );
        let why = surface_top(&doc, "Drag Knife", &[&s], &[out], o).unwrap_err();
        assert!(
            why.contains("outside the shapes Surfacing flattened"),
            "{why}"
        );
        assert!(why.contains("Flatten the whole material"), "{why}");
    }

    #[test]
    fn a_knife_moved_below_another_cut_is_flagged_again() {
        let mut doc = Document::default();
        doc.toolpaths.insert(EntityId(1), knife());
        doc.toolpaths.insert(EntityId(2), pocket());
        assert_eq!(misplaced(&doc, &doc.toolpaths[&EntityId(1)]), None);
        doc.toolpaths.insert(EntityId(3), knife());
        let why = misplaced(&doc, &doc.toolpaths[&EntityId(3)]).unwrap();
        assert!(why.contains("Pocket 1"), "{why}");
        // a toolpath being made again looks only above itself
        doc.remaking = Some(EntityId(1));
        assert_eq!(doc.toolpaths_before_made().len(), 0);
        doc.remaking = None;
        assert_eq!(doc.toolpaths_before_made().len(), 3);
    }
}
