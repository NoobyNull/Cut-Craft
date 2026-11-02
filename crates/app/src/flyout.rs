//! Flyouts: tools that do one job different ways share one button in the icon toolboxes. The
//! button shows the one picked last; press and hold it, click the little arrow in its corner, or
//! press Down while it has the keyboard focus, and the rest open beside it, each with its name.
//!
//! Aspire lists every one of these as an icon of its own; Cut Craft groups them so a beginner
//! faces fewer, clearer buttons (`re/SPEC.md`, Toolbox flyouts). Find Tool, the Rail layout's
//! lists and Favourites still name every tool on its own.

use cut_craft_core::Drag;
use eframe::egui;
use std::collections::BTreeMap;

/// How long a press on a flyout's button opens it instead of running the tool shown, in seconds.
pub const HOLD_SECONDS: f64 = 0.4;

/// The share of a button's side its corner arrow takes, measured from the bottom-right corner.
pub const ARROW_SHARE: f32 = 0.35;

/// Each flyout's tools, the one shown until another is picked first. A tool is in one flyout
/// at most; a flyout only holds tools that do one job different ways.
pub const FLYOUTS: &[&[&str]] = &[
    // Drawing tab
    &[
        "import_vectors_bitmap",
        "import_vectors",
        "import_component_3d_model",
    ],
    &["zoom_to_job", "zoom_box", "zoom_selected"],
    &["draw_polyline", "draw_curve", "draw_arc", "draw_freehand"],
    &["draw_rectangle", "draw_rectangle_from_center"],
    &["draw_circle", "draw_circle_from_edge", "draw_ellipse"],
    &["draw_polygon", "draw_star"],
    &[
        "draw_text",
        "edit_text",
        "text_on_a_curve",
        "auto_layout_text",
        "convert_text_to_curves",
    ],
    &[
        "join_vectors",
        "join_straight_line",
        "join_smooth_curve",
        "join_move_end_points",
        "close_extend_vector",
    ],
    &["weld_vectors", "subtract_vectors", "overlap_intersect"],
    &["interactive_vector_trim", "trim_objects"],
    &["edit_picture", "crop_bitmap"],
    &[
        "center_in_material",
        "center_horizontally_in_material",
        "center_vertically_in_material",
    ],
    &["center", "center_horizontally", "center_vertically"],
    &["align_left", "align_right", "align_top", "align_bottom"],
    &[
        "left_to_right",
        "right_to_left",
        "top_to_bottom",
        "bottom_to_top",
    ],
    &["nest_parts", "plate_production"],
    &["array_copy", "circular_copy", "copy_along_vectors"],
    // Modeling tab
    &[
        "create_shape_round",
        "create_shape_angled",
        "create_shape_smooth",
        "create_shape_flat",
    ],
];

/// The flyout `id` is in, if any.
pub fn flyout_of(id: &str) -> Option<&'static [&'static str]> {
    FLYOUTS.iter().copied().find(|f| f.contains(&id))
}

/// The tool each flyout shows, by the flyout's first tool: the one picked from it last.
pub type Faces = BTreeMap<String, String>;

/// One button of a toolbox section: a tool on its own, or a flyout's face with the tools
/// under it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Button {
    /// The tool the button runs and shows.
    pub face: &'static str,
    /// The flyout's tools in this section, `face` among them, in the flyout's order; empty for
    /// a tool on its own.
    pub members: Vec<&'static str>,
}

/// The buttons for a section's `tools`: a flyout stands where the first of its tools in the
/// section comes, showing the one `faces` remembers (else its first here). A flyout with only
/// one of its tools in the section is that tool on its own.
pub fn buttons(tools: &[&'static str], faces: &Faces) -> Vec<Button> {
    let mut out: Vec<Button> = Vec::new();
    for &id in tools {
        let Some(flyout) = flyout_of(id) else {
            out.push(Button {
                face: id,
                members: vec![],
            });
            continue;
        };
        if out.iter().any(|b| b.members.contains(&id)) {
            continue;
        }
        let members: Vec<&'static str> = flyout
            .iter()
            .copied()
            .filter(|m| tools.contains(m))
            .collect();
        if members.len() < 2 {
            out.push(Button {
                face: id,
                members: vec![],
            });
            continue;
        }
        let face = faces
            .get(flyout[0])
            .and_then(|f| members.iter().copied().find(|m| m == f))
            .unwrap_or(members[0]);
        out.push(Button { face, members });
    }
    out
}

/// Remembers `id` as its flyout's face; a tool in no flyout changes nothing.
pub fn remember(faces: &mut Faces, id: &str) {
    if let Some(flyout) = flyout_of(id) {
        faces.insert(flyout[0].to_owned(), id.to_owned());
    }
}

/// A flyout button's tooltip: what the tool shown does, then the others under it by name.
pub fn tooltip(about: &str, others: &[String]) -> String {
    if others.is_empty() {
        return about.to_owned();
    }
    format!(
        "{about}\n\nHold, or click the corner arrow, for: {}",
        others.join(", ")
    )
}

/// Whether a click at `at` on a button covering `rect` is on its corner arrow.
pub fn on_arrow(rect: egui::Rect, at: egui::Pos2) -> bool {
    let side = rect.width().min(rect.height()) * ARROW_SHARE;
    rect.contains(at) && at.x >= rect.right() - side && at.y >= rect.bottom() - side
}

/// How a drawing tool reads the drag that draws it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DragWay {
    /// As the tool always has.
    #[default]
    AsIs,
    /// From the middle out, as Alt does.
    FromCenter,
    /// From one side straight across to the other: the middle of the drag is the centre.
    AcrossEdge,
}

impl DragWay {
    /// The drag as the tool reads it the usual way.
    pub fn apply(self, drag: Drag) -> Drag {
        match self {
            Self::AsIs => drag,
            Self::FromCenter => Drag {
                from_center: true,
                ..drag
            },
            Self::AcrossEdge => Drag {
                start: drag.start.lerp(drag.end, 0.5),
                from_center: false,
                ..drag
            },
        }
    }
}

/// A drawing tool started another way: its command's form, its drag read as `way`.
#[derive(Debug, PartialEq, Eq)]
pub struct Variant {
    pub id: &'static str,
    pub command: &'static str,
    pub way: DragWay,
    pub label: &'static str,
    /// The status line's hint while it is drawing.
    pub hint: &'static str,
}

pub const VARIANTS: [Variant; 2] = [
    Variant {
        id: "draw_rectangle_from_center",
        command: "draw_rectangle",
        way: DragWay::FromCenter,
        label: "Draw Rectangle from Center",
        hint: "Draw Rectangle from Center: drag from the middle out to a corner",
    },
    Variant {
        id: "draw_circle_from_edge",
        command: "draw_circle",
        way: DragWay::AcrossEdge,
        label: "Draw Circle from Edge",
        hint: "Draw Circle from Edge: drag from one side straight across to the other",
    },
];

/// The variant `id` names, if it is one.
pub fn variant(id: &str) -> Option<&'static Variant> {
    VARIANTS.iter().find(|v| v.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::geometry::Point;

    fn faces(pairs: &[(&str, &str)]) -> Faces {
        pairs
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect()
    }

    #[test]
    fn no_tool_is_in_two_flyouts_and_each_has_two_or_more() {
        let mut seen = std::collections::HashSet::new();
        for flyout in FLYOUTS {
            assert!(flyout.len() >= 2, "{flyout:?}");
            for id in *flyout {
                assert!(seen.insert(*id), "{id} is in two flyouts");
            }
        }
    }

    #[test]
    fn a_flyout_shows_as_one_button_where_its_first_tool_comes() {
        let tools = ["select", "draw_circle", "draw_rectangle", "draw_ellipse"];
        let b = buttons(&tools, &Faces::new());
        assert_eq!(
            b,
            vec![
                Button {
                    face: "select",
                    members: vec![]
                },
                Button {
                    face: "draw_circle",
                    members: vec!["draw_circle", "draw_ellipse"]
                },
                Button {
                    face: "draw_rectangle",
                    members: vec![]
                },
            ]
        );
    }

    #[test]
    fn the_face_is_the_tool_picked_last() {
        let tools = ["draw_circle", "draw_circle_from_edge", "draw_ellipse"];
        let mut f = Faces::new();
        remember(&mut f, "draw_ellipse");
        assert_eq!(buttons(&tools, &f)[0].face, "draw_ellipse");
        remember(&mut f, "select");
        assert_eq!(f.len(), 1, "a tool in no flyout is not remembered");
    }

    #[test]
    fn a_remembered_face_missing_from_the_section_falls_back_to_the_first_there() {
        let tools = ["draw_circle_from_edge", "draw_ellipse"];
        let f = faces(&[("draw_circle", "draw_circle")]);
        assert_eq!(buttons(&tools, &f)[0].face, "draw_circle_from_edge");
    }

    #[test]
    fn every_drawing_tab_tool_is_still_reachable_from_some_button() {
        let toolbar = crate::gui::Toolbar::new();
        for group in toolbar.groups.iter().chain(&toolbar.modeling_groups) {
            let b = buttons(group.tools, &Faces::new());
            for id in group.tools {
                assert!(
                    b.iter().any(|b| b.face == *id || b.members.contains(id)),
                    "{id} lost from {}",
                    group.name
                );
            }
        }
    }

    #[test]
    fn the_drawing_tab_has_far_fewer_buttons() {
        let toolbar = crate::gui::Toolbar::new();
        let count = |groups: &[crate::gui::ToolGroup]| -> usize {
            groups
                .iter()
                .map(|g| buttons(g.tools, &Faces::new()).len())
                .sum()
        };
        assert!(count(&toolbar.groups) <= 45, "{}", count(&toolbar.groups));
        assert_eq!(count(&toolbar.modeling_groups), 20);
    }

    #[test]
    fn every_flyout_tool_is_on_a_tab() {
        let toolbar = crate::gui::Toolbar::new();
        for id in FLYOUTS.iter().flat_map(|f| f.iter()) {
            assert!(
                toolbar
                    .groups
                    .iter()
                    .chain(&toolbar.modeling_groups)
                    .any(|g| g.tools.contains(id)),
                "{id}"
            );
        }
    }

    #[test]
    fn the_tooltip_names_the_hidden_tools() {
        assert_eq!(tooltip("Draw a circle", &[]), "Draw a circle");
        let tip = tooltip(
            "Draw a circle",
            &["Draw Circle from Edge".into(), "Draw Ellipse".into()],
        );
        assert!(tip.starts_with("Draw a circle"));
        assert!(
            tip.contains(
                "Hold, or click the corner arrow, for: Draw Circle from Edge, Draw Ellipse"
            )
        );
    }

    #[test]
    fn only_the_bottom_right_corner_is_the_arrow() {
        let r = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(40.0, 40.0));
        assert!(on_arrow(r, egui::pos2(38.0, 38.0)));
        assert!(!on_arrow(r, egui::pos2(20.0, 20.0)));
        assert!(!on_arrow(r, egui::pos2(38.0, 5.0)));
        assert!(!on_arrow(r, egui::pos2(45.0, 45.0)));
    }

    fn drag(a: (f64, f64), b: (f64, f64)) -> Drag {
        Drag {
            start: Point::new(a.0, a.1),
            end: Point::new(b.0, b.1),
            from_center: false,
            constrain: false,
        }
    }

    #[test]
    fn a_circle_from_its_edge_is_centred_halfway_along_the_drag() {
        let d = DragWay::AcrossEdge.apply(drag((0.0, 0.0), (20.0, 0.0)));
        assert_eq!(d.start, Point::new(10.0, 0.0));
        assert_eq!(d.end, Point::new(20.0, 0.0));
        assert!((d.start.distance(d.end) - 10.0).abs() < 1e-9);
    }

    #[test]
    fn a_rectangle_from_its_centre_reads_the_drag_as_alt_does() {
        let d = DragWay::FromCenter.apply(drag((10.0, 10.0), (15.0, 13.0)));
        let r = d.rect();
        assert_eq!(r.min, Point::new(5.0, 7.0));
        assert_eq!(r.max, Point::new(15.0, 13.0));
        assert_eq!(
            DragWay::AsIs.apply(drag((1.0, 2.0), (3.0, 4.0))).start,
            Point::new(1.0, 2.0)
        );
    }

    #[test]
    fn variants_name_their_command() {
        assert_eq!(
            variant("draw_circle_from_edge").unwrap().command,
            "draw_circle"
        );
        assert!(variant("draw_circle").is_none());
        for v in &VARIANTS {
            assert!(flyout_of(v.id).is_some(), "{} sits in a flyout", v.id);
        }
    }
}
