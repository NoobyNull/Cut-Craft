//! The pack's tutorial: a ready-made two-sided job that cuts the knight from 1½″ wood,
//! with two alignment pins so the second side lines up with the first. Its steps open by
//! themselves (job notes starting with ".") when the job is opened.
//!
//! The knight is the same on both faces, so each side cuts the same half down to the
//! wood's middle. A frame of uncut wood round the edge holds two small tabs that keep the
//! knight in place until both sides are done.

use crate::profile::Piece;
use crate::shape::Shape;
use cut_craft_core::Change;
use cut_craft_core::component::{Combine, Component};
use cut_craft_core::document::{Document, DoubleSided, Flip, GapFrom, Job, ModelPosition};
use cut_craft_core::geometry::{Path, Point, Rect};
use cut_craft_core::toolpath::{Tool, ToolShape};
use cut_craft_core::{Command as _, CommandError};

/// The wood: 1½″ thick, just big enough for the knight, its tabs and the frame.
pub const THICKNESS: f64 = 38.1;
const WIDTH: f64 = 112.0;
const HEIGHT: f64 = 83.0;
/// The uncut frame round the edge, which the pins go through and the clamps hold.
pub const FRAME: f64 = 16.0;
/// Room between the knight and the frame for the roughing cutter.
const GAP: f64 = 10.0;
/// The pins: 1/4″ dowels.
pub const PIN: f64 = 6.35;
/// How deep the pin holes go into the wood (from the first side) and the spoilboard.
pub const PIN_IN_WOOD: f64 = 15.0;
pub const PIN_IN_SPOILBOARD: f64 = 10.0;
/// The tabs: how wide, and how far either side of the middle.
const TAB_WIDTH: f64 = 5.0;
const TAB_HALF: f64 = 2.0;
/// How finely the model is kept.
const CELL: f64 = 0.2;

/// The step-by-step that opens with the job.
pub const STEPS: &str = "\
.Two-sided knight: your first two-sided job

This job cuts a whole chess knight from both sides of one piece of wood. You cut the \
top, turn the wood over onto two pins, and cut the other side. The pins put the wood back \
exactly where it was, so the two halves meet.

You need:
- Wood 112 x 83 mm and 38 mm (1½″) thick, flat on both faces. Hardwood such as maple or \
walnut carves best.
- A 1/4″ end mill and a 1/8″ ball nose.
- Two 1/4″ wooden dowels, about 25 mm long.

Side 1 (the top):
1. Clamp the wood down by its corners only. The middle and the pin holes must stay clear.
2. Set X and Y zero at the wood's bottom left corner, and Z zero on its top. Don't move \
X and Y zero again until the job is finished.
3. Save and run the toolpaths in order: 1 Pin holes, 2 Roughing, 3 Finishing.
4. Take the wood off, leaving the zero where it is. Run 4 Spoilboard pin holes with \
nothing on the table: it drills two holes into the spoilboard. Tap a dowel into each one.

Side 2 (the bottom):
5. Turn the wood over left to right, as you would turn a page, and press its pin holes \
onto the dowels. Clamp it by the corners again.
6. Swap to the bottom side (the side button at the top), then save and run 5 Roughing \
and 6 Finishing. The wood is the same thickness, so Z zero stays where it was.

Finishing off:
7. Two small tabs hold the knight in the frame. Cut them with a fine saw or a sharp \
knife, then sand the marks smooth.

Tips:
- Use Preview All Toolpaths to see both sides cut before you start.
- If your wood is not exactly 38.1 mm thick, measure it and change the thickness in Job \
Setup, then recalculate all the toolpaths.
";

/// Where the knight's foot is, and its middle line, on the top side.
fn knight_at() -> (f64, f64) {
    (FRAME + GAP, HEIGHT / 2.0)
}

/// The pin holes: on the line the wood turns over about, in the frame, so turning it over
/// puts each hole back where it was.
pub fn pins() -> [Point; 2] {
    [
        Point::new(WIDTH / 2.0, FRAME / 2.0),
        Point::new(WIDTH / 2.0, HEIGHT - FRAME / 2.0),
    ]
}

fn area() -> Rect {
    Rect::new(Point::ZERO, Point::new(WIDTH, HEIGHT))
}

/// The top side's model: the knight's half, the frame and the tabs, heights above the
/// wood's middle; seen from the bottom it is the same, turned over.
fn components(bottom: bool) -> Vec<Component> {
    let shape = Shape::new(Piece::Knight);
    let (length, half) = shape.extent();
    let (x0, yc) = knight_at();
    let mirror = Flip::Horizontal.mirror(area());
    // a point of this side, as the top side sees it
    let top = move |p: Point| {
        if bottom {
            mirror.transform_point2(p)
        } else {
            p
        }
    };
    let box_of = |r: Rect| {
        if bottom {
            let (a, b) = (
                mirror.transform_point2(r.min),
                mirror.transform_point2(r.max),
            );
            Rect::new(a.min(b), a.max(b))
        } else {
            r
        }
    };
    let knight_box = Rect::new(
        Point::new(x0 - 1.0, yc - half - 3.0),
        Point::new(x0 + length + 1.0, yc + half + 3.0),
    );
    let knight = Component::from_fn("Knight", Combine::Merge, box_of(knight_box), CELL, |p| {
        let q = top(p);
        let s = shape.sample(q.x - x0, q.y - yc);
        (s.inside > 0.0).then_some(s.height)
    });
    let frame = Component::from_fn("Frame", Combine::Merge, area(), CELL, |p| {
        let edge = p.x.min(p.y).min(WIDTH - p.x).min(HEIGHT - p.y);
        (edge < FRAME).then_some(THICKNESS / 2.0)
    });
    // one tab into the knight's foot (sanded flat anyway), one into the back of its mane
    let tabs = [
        Rect::new(
            Point::new(FRAME - 1.0, yc - TAB_WIDTH / 2.0),
            Point::new(x0 + 2.0, yc + TAB_WIDTH / 2.0),
        ),
        Rect::new(
            Point::new(x0 + 36.0 - TAB_WIDTH / 2.0, FRAME - 1.0),
            Point::new(x0 + 36.0 + TAB_WIDTH / 2.0, yc - 8.0),
        ),
    ];
    let tabs = tabs.map(|t| {
        Component::from_fn("Tab", Combine::Merge, box_of(t), CELL, |p| {
            let q = top(p);
            (q.x >= t.min.x && q.x <= t.max.x && q.y >= t.min.y && q.y <= t.max.y)
                .then_some(TAB_HALF)
        })
    });
    let mut all = vec![frame, knight];
    all.extend(tabs);
    all
}

fn tool(name: &str, shape: ToolShape, diameter: f64, pass_depth: f64) -> Tool {
    Tool {
        name: name.to_owned(),
        shape,
        diameter,
        pass_depth,
        stepover: 0.4,
        finish_stepover: 0.1,
        // gentle rates a hobby router manages in hardwood
        feed_rate: 1500.0,
        plunge_rate: 400.0,
        spindle_speed: 18000.0,
        ..Tool::default()
    }
}

fn end_mill() -> Tool {
    tool("1/4\" End Mill", ToolShape::EndMill, 6.35, 3.0)
}

fn ball_nose() -> Tool {
    tool("1/8\" Ball Nose", ToolShape::BallNose, 3.175, 1.5)
}

fn apply(doc: &mut Document, change: Result<Change, CommandError>) -> Result<(), String> {
    let change = change.map_err(|e| e.to_string())?;
    doc.apply(&change).map_err(|e| format!("{e:?}"))
}

/// Roughing then finishing, named for their place in the steps.
fn carve(doc: &mut Document, first: u32) -> Result<(), String> {
    let rough = cmd_3d_roughing_toolpath::Params {
        name: format!("{first} Roughing"),
        tool: end_mill(),
        ..Default::default()
    };
    let change = cmd_3d_roughing_toolpath::RoughingToolpath3d::run(doc, &rough);
    apply(doc, change)?;
    let finish = cmd_3d_finishing_toolpath::Params {
        name: format!("{} Finishing", first + 1),
        tools: vec![ball_nose()],
        ..Default::default()
    };
    let change = cmd_3d_finishing_toolpath::FinishingToolpath3d::run(doc, &finish);
    apply(doc, change)
}

fn drill(doc: &mut Document, name: &str, depth: f64) -> Result<(), String> {
    let params = cmd_drilling_toolpath::Params {
        name: name.to_owned(),
        depth,
        tool: end_mill(),
        ..Default::default()
    };
    let change = cmd_drilling_toolpath::DrillingToolpath::run(doc, &params);
    apply(doc, change)
}

/// The tutorial job, toolpaths calculated, top side showing.
pub fn job() -> Result<Document, String> {
    let mut doc = Document::default();
    let mut c = Change::builder(&doc);
    c.set_job(Job {
        width: WIDTH,
        height: HEIGHT,
        thickness: THICKNESS,
        double_sided: Some(DoubleSided {
            flip: Flip::Horizontal,
            zero_same_side: false,
        }),
        // the model's top (the frame) at the wood's top: its base is then the middle
        model_position: ModelPosition {
            from: GapFrom::Above,
            gap: 0.0,
        },
        ..doc.job
    });
    for component in components(false) {
        c.add_component(component);
    }
    let pins: Vec<_> = pins()
        .iter()
        .map(|&p| c.add(Path::circle(p, PIN / 2.0)))
        .collect();
    c.select(pins);
    c.set_notes(STEPS);
    doc.apply(&c.finish()).map_err(|e| format!("{e:?}"))?;

    drill(&mut doc, "1 Pin holes", PIN_IN_WOOD)?;
    carve(&mut doc, 2)?;
    drill(
        &mut doc,
        "4 Spoilboard pin holes (wood off the table)",
        THICKNESS + PIN_IN_SPOILBOARD,
    )?;

    let mut c = Change::builder(&doc);
    c.swap_sides();
    doc.apply(&c.finish()).map_err(|e| format!("{e:?}"))?;
    let mut c = Change::builder(&doc);
    for component in components(true) {
        c.add_component(component);
    }
    doc.apply(&c.finish()).map_err(|e| format!("{e:?}"))?;
    carve(&mut doc, 5)?;

    let mut c = Change::builder(&doc);
    c.swap_sides();
    c.select(Vec::new());
    doc.apply(&c.finish()).map_err(|e| format!("{e:?}"))?;
    Ok(doc)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::file;

    fn deepest(doc: &Document, name_starts: &str) -> f64 {
        let tp = (doc.toolpaths.values())
            .find(|t| t.name.starts_with(name_starts))
            .unwrap_or_else(|| panic!("no toolpath {name_starts}"));
        tp.moves
            .iter()
            .map(|m| m.to.z)
            .fold(f64::INFINITY, f64::min)
    }

    #[test]
    fn the_tutorial_cuts_each_side_to_the_middle() {
        let doc = job().unwrap();
        assert!(doc.job.double_sided.is_some() && !doc.bottom_side);
        assert!(doc.notes.starts_with('.'), "the steps open with the job");
        // the top side: pins, roughing, finishing, spoilboard pins
        let names: Vec<_> = doc.toolpaths.values().map(|t| t.name.as_str()).collect();
        assert_eq!(names.len(), 4, "{names:?}");
        let middle = -THICKNESS / 2.0;
        for side in ["2 Roughing", "3 Finishing"] {
            let z = deepest(&doc, side);
            assert!(z >= middle - 0.01, "{side} goes {z} deep, past the middle");
            assert!(z < middle + 1.0, "{side} only reaches {z}");
        }
        assert!((deepest(&doc, "1 Pin") + PIN_IN_WOOD).abs() < 1e-6);
        let spoil = deepest(&doc, "4 Spoilboard");
        assert!(
            (spoil + THICKNESS + PIN_IN_SPOILBOARD).abs() < 1e-6,
            "{spoil}"
        );

        // the bottom side carves the same, to the middle
        let bottom = &doc.other_side;
        assert_eq!(bottom.toolpaths.len(), 2);
        assert_eq!(bottom.components.len(), components(true).len());
        for tp in bottom.toolpaths.values() {
            let z = tp
                .moves
                .iter()
                .map(|m| m.to.z)
                .fold(f64::INFINITY, f64::min);
            assert!(z >= middle - 0.01 && z < middle + 1.0, "{}: {z}", tp.name);
        }
    }

    #[test]
    fn the_pins_turn_over_onto_themselves_and_miss_the_knight() {
        let mirror = Flip::Horizontal.mirror(area());
        for pin in pins() {
            assert!((mirror.transform_point2(pin) - pin).length() < 1e-9);
            assert!(pin.y - PIN / 2.0 > 1.0 && pin.y + PIN / 2.0 < HEIGHT - 1.0);
            assert!(pin.y.min(HEIGHT - pin.y) < FRAME, "in the frame");
        }
    }

    #[test]
    fn the_bottom_is_the_top_turned_over() {
        let top = components(false);
        let bottom = components(true);
        let mirror = Flip::Horizontal.mirror(area());
        let knight = |cs: &[Component], p: Point| cs[1].height_at(p);
        let (x0, yc) = knight_at();
        // cell centres, which turn over onto cell centres
        for p in [Point::new(x0 + 37.1, yc + 13.0), Point::new(x0 + 5.1, yc)] {
            let a = knight(&top, p).unwrap();
            let b = knight(&bottom, mirror.transform_point2(p)).unwrap();
            assert!((a - b).abs() < 1e-9, "{p}: {a} {b}");
        }
    }

    #[test]
    fn the_job_saves_and_opens_again() {
        let doc = job().unwrap();
        let again = file::load_job(&file::save_job(&doc)).unwrap();
        assert_eq!(again.toolpaths.len(), doc.toolpaths.len());
        assert_eq!(again.other_side.toolpaths.len(), 2);
    }
}
