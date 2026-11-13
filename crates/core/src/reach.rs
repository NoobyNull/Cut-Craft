//! Can the bit reach everything? One check every toolpath shares, so the form, Calculate and
//! Save Toolpaths all say the same thing the same way: whether the cut is deeper than the
//! bit's flutes, and whether there are spots the bit is too wide to get into. Each message
//! says what is missed and what to do about it.

use crate::document::Units;
use crate::geometry::Path;
use crate::polygon::{self, Corners};
use crate::simulation::Material;
use crate::toolpath::{Tool, ToolShape};

/// Which side of the shapes the bit cuts: inside (a pocket, a hole) or outside (a part).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    Inside,
    Outside,
}

/// What the bit cannot get into.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Missed {
    /// How many separate spots.
    pub spots: usize,
    /// Their area together, mm².
    pub area: f64,
    /// The most material left in any of them, mm; 0 for a flat (2D) cut, where it is the
    /// whole depth.
    pub most: f64,
}

impl Missed {
    /// Whether anything is missed.
    pub fn any(&self) -> bool {
        self.spots > 0
    }
}

/// What the user can do about a missed spot, for the toolpath it is in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fix {
    /// A pocket: a smaller finishing bit, the big one kept to clear.
    Pocket,
    /// A profile: a smaller bit, or V-Carve for sharp corners.
    Profile,
    /// A 3D finish: a bit with a smaller tip, after this one.
    Finish,
}

/// The smallest uncut area that counts as a spot the bit cannot get into, mm²: half the
/// bit's own footprint. Less is the rounding every inside corner gets from a round bit,
/// which is expected (a 90° corner leaves a fifth of that); a corner sharper than about
/// 40°, or a gap narrower than the bit, leaves more.
fn smallest_spot(tool: &Tool) -> f64 {
    let r = tool.diameter / 2.0;
    std::f64::consts::PI * r * r / 2.0
}

/// The spots inside (or outside) the closed `shapes` that `tool` is too wide to get into,
/// cutting flat (a pocket or a profile): what is left of the shapes once a disc the bit's
/// width has been everywhere it fits.
pub fn flat(shapes: &[Path], tool: &Tool, side: Side) -> Missed {
    let r = tool.diameter / 2.0;
    let closed: Vec<Path> = shapes.iter().filter(|p| p.closed).cloned().collect();
    if r <= 0.0 || closed.is_empty() {
        return Missed::default();
    }
    let shape = polygon::union(&closed);
    let left = match side {
        // everywhere the bit's disc fits inside, against the whole inside
        Side::Inside => {
            let fits = polygon::offset(
                &polygon::offset(&shape, -r, Corners::Round),
                r,
                Corners::Round,
            );
            polygon::difference(&shape, &fits)
        }
        // the notches outside narrower than the bit, which the bit's disc going round
        // bridges over
        Side::Outside => {
            let bridged = polygon::offset(
                &polygon::offset(&shape, r, Corners::Round),
                -r,
                Corners::Round,
            );
            polygon::difference(&bridged, &shape)
        }
    };
    // a hair's width along an edge is the offsets' rounding, not a spot
    let hair = (polygon::TOLERANCE * 2.0).min(r / 4.0);
    let smallest = smallest_spot(tool);
    left.iter()
        .filter(|piece| polygon::area(piece) >= smallest)
        .filter(|piece| {
            !polygon::offset(std::slice::from_ref(*piece), -hair, Corners::Round).is_empty()
        })
        .fold(Missed::default(), |m, piece| Missed {
            spots: m.spots + 1,
            area: m.area + polygon::area(piece),
            most: 0.0,
        })
}

/// The spots of a 3D `model` that `tool` cannot get into finishing it, leaving more than
/// `left` mm over what its own rounding leaves anyway. A ball nose (or the corner of any bit)
/// always leaves a fillet up to its radius at the foot of a wall, which is expected; more
/// than that is a pit, groove or steep spot too narrow for the bit, or held off by its taper.
#[expect(
    clippy::cast_precision_loss,
    reason = "a cell count, far below f64's exact range"
)]
pub fn surface(model: &Material, tool: &Tool, left: f64) -> Missed {
    if tool.diameter <= 0.0 || model.heights.is_empty() {
        return Missed::default();
    }
    let finished = model.left_by(tool);
    let (columns, rows) = (model.columns, model.rows);
    let height = |c: usize, r: usize| model.heights[r * columns + c];
    // tool positions only at cell centres make what is left up to about slope x cell too
    // high on a slope (as rest machining allows)
    let slope = |c: usize, r: usize| {
        let along = |lo: usize, hi: usize, h: &dyn Fn(usize) -> f64| {
            if hi > lo {
                (h(hi) - h(lo)) / ((hi - lo) as f64 * model.cell)
            } else {
                0.0
            }
        };
        let dx = along(c.saturating_sub(1), (c + 1).min(columns - 1), &|c| {
            height(c, r)
        });
        let dy = along(r.saturating_sub(1), (r + 1).min(rows - 1), &|r| {
            height(c, r)
        });
        dx.hypot(dy)
    };
    let expected = tool.diameter / 2.0 + left;
    let over: Vec<f64> = (0..rows * columns)
        .map(|i| {
            let (c, r) = (i % columns, i / columns);
            let extra = finished.heights[i] - height(c, r);
            let allowed = expected + slope(c, r) * model.cell;
            if extra > allowed { extra } else { 0.0 }
        })
        .collect();
    // separate spots: cells joined side to side
    let mut seen = vec![false; over.len()];
    let mut missed = Missed::default();
    let cell_area = model.cell * model.cell;
    for start in 0..over.len() {
        if over[start] <= 0.0 || seen[start] {
            continue;
        }
        seen[start] = true;
        let (mut cells, mut most, mut next) = (0_usize, 0.0_f64, vec![start]);
        while let Some(i) = next.pop() {
            cells += 1;
            most = most.max(over[i]);
            let (c, r) = (i % columns, i / columns);
            let neighbours = [
                (c > 0).then(|| i - 1),
                (c + 1 < columns).then_some(i + 1),
                (r > 0).then(|| i - columns),
                (r + 1 < rows).then_some(i + columns),
            ];
            for n in neighbours.into_iter().flatten() {
                if over[n] > 0.0 && !seen[n] {
                    seen[n] = true;
                    next.push(n);
                }
            }
        }
        // a lone cell is the sampling, not a spot
        if cells < 2 {
            continue;
        }
        missed.spots += 1;
        missed.area += cells as f64 * cell_area;
        missed.most = missed.most.max(most);
    }
    missed
}

/// An area for a sentence in `units`: mm² when small, cm² otherwise; in² in inches.
fn area(mm2: f64, units: Units) -> String {
    match units {
        Units::Inch => units.area(mm2, 1),
        Units::Mm if mm2 < 100.0 => format!("{mm2:.1} mm²"),
        Units::Mm => format!("{:.1} cm²", mm2 / 100.0),
    }
}

/// What `name`'s bit misses, as one sentence that says what to do about it.
pub fn missed_sentence(name: &str, tool: &Tool, missed: &Missed, fix: Fix, units: Units) -> String {
    let spots = if missed.spots == 1 {
        "1 spot".to_owned()
    } else {
        format!("{} spots", missed.spots)
    };
    let why = if tool.tapered().is_some() {
        "too narrow or steep for its tip and taper"
    } else {
        "too narrow for it"
    };
    let how = match fix {
        Fix::Pocket => {
            "Pick a smaller bit as the Tool, and keep this one under Clear with bigger bits \
             first so the pocket stays quick"
        }
        Fix::Profile => "Pick a smaller bit, or cut the sharp corners with V-Carve",
        Fix::Finish => {
            "Add a 3D Finishing toolpath after this one with a smaller bit, or pick a smaller \
             bit here"
        }
    };
    let left = if missed.most > 0.0 {
        format!(", leaving up to {}", units.length(missed.most, 1))
    } else {
        String::new()
    };
    format!(
        "{name}: {} can't reach {spots} ({}), {why}{left}. {how}",
        tool.name,
        area(missed.area, units)
    )
}

/// Whether cutting `depth` mm deep is beyond `tool`'s flutes, as one sentence in `units` that
/// says what to do about it; `None` when it reaches, or the flute length is not set.
pub fn too_deep(name: &str, tool: &Tool, depth: f64, units: Units) -> Option<String> {
    let flutes = tool.cutting_length()?;
    (depth > flutes + 1e-6).then(|| {
        if tool.shape == ToolShape::VBit {
            return format!(
                "{name} cuts {} deep, but the V of {} is only {} deep, so past that its \
                 shank rubs the sides: pick a wider V-bit, or cut less deep",
                units.length(depth, 1),
                tool.name,
                units.length(flutes, 1)
            );
        }
        format!(
            "{name} cuts {} deep, but the flutes of {} are only {} long: pick a bit with \
             longer flutes, or cut less deep",
            units.length(depth, 1),
            tool.name,
            units.length(flutes, 1)
        )
    })
}

/// The bit's reach as one short line for its toolpath form: what it can't reach, or that it
/// reaches everything; nothing before a bit is picked. `depth` is how deep the form cuts, and `missed` the spots it is too
/// wide for, when the form's shapes are known. `None` when there is nothing to say yet.
pub fn form_line(
    tool: &Tool,
    depth: Option<f64>,
    missed: Option<&Missed>,
    units: Units,
) -> Option<Line> {
    if tool.name.is_empty() || tool.diameter <= 0.0 {
        return None;
    }
    let mut problems = vec![];
    if let (Some(depth), Some(flutes)) = (depth, tool.cutting_length())
        && depth > flutes + 1e-6
    {
        let part = if tool.shape == ToolShape::VBit {
            "the V"
        } else {
            "its flutes"
        };
        problems.push(format!(
            "too deep: the cut is {}, {part} {}",
            units.length(depth, 1),
            units.length(flutes, 1)
        ));
    }
    if let Some(m) = missed.filter(|m| m.any()) {
        let spots = if m.spots == 1 {
            "1 spot".to_owned()
        } else {
            format!("{} spots", m.spots)
        };
        problems.push(format!(
            "too wide for {spots} ({}), left uncut",
            area(m.area, units)
        ));
    }
    if !problems.is_empty() {
        return Some(Line::Short(format!(
            "Can't reach everything: {}",
            problems.join("; ")
        )));
    }
    if depth.is_some() && tool.cutting_length().is_none() {
        let fits = if missed.is_some() {
            "Fits every spot, but depth"
        } else {
            "Depth"
        };
        return Some(Line::Unknown(format!(
            "{fits} not checked: set this bit's Flute Length in the Tool Database"
        )));
    }
    (depth.is_some() || missed.is_some()).then(|| Line::Reaches("Reaches everything".into()))
}

/// A form's reach line, and how it looks.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Line {
    /// Everything is reached: plain.
    Reaches(String),
    /// Something is not: in red.
    Short(String),
    /// It could not be checked: in the warning colour.
    Unknown(String),
}

impl Line {
    pub fn text(&self) -> &str {
        match self {
            Self::Reaches(t) | Self::Short(t) | Self::Unknown(t) => t,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Point;
    use crate::testing::square;
    use crate::toolpath::ToolShape;

    fn end_mill(diameter: f64) -> Tool {
        Tool {
            name: format!("{diameter} mm End Mill"),
            diameter,
            flute_length: 20.0,
            ..Tool::default()
        }
    }

    /// A 40 x 40 square with a slot `width` wide and 20 long cut into its top edge (a "U").
    fn with_slot(width: f64) -> Path {
        let half = width / 2.0;
        Path::polyline(
            &[
                Point::new(0.0, 0.0),
                Point::new(40.0, 0.0),
                Point::new(40.0, 40.0),
                Point::new(20.0 + half, 40.0),
                Point::new(20.0 + half, 20.0),
                Point::new(20.0 - half, 20.0),
                Point::new(20.0 - half, 40.0),
                Point::new(0.0, 40.0),
            ],
            true,
        )
    }

    /// A 40 x 40 square with a channel `width` wide and 20 long going up from its top edge.
    fn with_channel(width: f64) -> Vec<Path> {
        let half = width / 2.0;
        vec![
            square(0.0, 0.0, 40.0),
            Path::polyline(
                &[
                    Point::new(20.0 - half, 39.0),
                    Point::new(20.0 + half, 39.0),
                    Point::new(20.0 + half, 60.0),
                    Point::new(20.0 - half, 60.0),
                ],
                true,
            ),
        ]
    }

    #[test]
    fn a_square_pocket_misses_nothing_however_big_the_bit() {
        // its corners round off, which every round bit does
        for d in [3.0, 6.0, 12.0] {
            let m = flat(&[square(0.0, 0.0, 40.0)], &end_mill(d), Side::Inside);
            assert!(!m.any(), "{d} mm: {m:?}");
        }
    }

    #[test]
    fn a_channel_narrower_than_the_bit_is_missed_inside() {
        let shapes = with_channel(4.0);
        let m = flat(&shapes, &end_mill(6.0), Side::Inside);
        assert_eq!(m.spots, 1, "{m:?}");
        // the 4 x 21 channel, less a little where the bit's edge reaches up into it
        assert!(m.area > 70.0 && m.area <= 84.0 + 1e-6, "{m:?}");
        assert!(!flat(&shapes, &end_mill(3.0), Side::Inside).any());
    }

    #[test]
    fn a_slot_narrower_than_the_bit_is_missed_cutting_round_the_outside() {
        let shape = [with_slot(4.0)];
        let m = flat(&shape, &end_mill(6.0), Side::Outside);
        assert_eq!(m.spots, 1, "{m:?}");
        assert!(m.area > 70.0 && m.area <= 80.0 + 1e-6, "{m:?}");
        assert!(!flat(&shape, &end_mill(3.0), Side::Outside).any());
        // inside, the same slot is part of the outside, so nothing is missed
        assert!(!flat(&shape, &end_mill(6.0), Side::Inside).any());
    }

    #[test]
    fn a_sharp_point_is_missed_but_a_right_angle_is_not() {
        // a thin triangle: its 20° point is far from where a 6 mm bit fits
        let spike = Path::polyline(
            &[
                Point::new(0.0, 0.0),
                Point::new(60.0, 0.0),
                Point::new(60.0, 21.0),
            ],
            true,
        );
        let m = flat(&[spike], &end_mill(6.0), Side::Inside);
        assert_eq!(m.spots, 1, "the sharp point only: {m:?}");
    }

    #[test]
    fn open_vectors_and_no_bit_miss_nothing() {
        let open = Path::polyline(&[Point::new(0.0, 0.0), Point::new(10.0, 0.0)], false);
        assert!(!flat(&[open], &end_mill(6.0), Side::Inside).any());
        assert!(!flat(&with_channel(1.0), &end_mill(0.0), Side::Inside).any());
    }

    #[test]
    fn too_deep_names_the_depth_the_flutes_and_the_fix() {
        let tool = end_mill(6.0);
        assert_eq!(too_deep("Pocket 1", &tool, 20.0, Units::Mm), None);
        let said = too_deep("Pocket 1", &tool, 25.0, Units::Mm).unwrap();
        assert!(said.contains("25.0 mm deep"), "{said}");
        assert!(said.contains("only 20.0 mm"), "{said}");
        assert!(said.contains("longer flutes"), "{said}");
        let unknown = Tool {
            flute_length: 0.0,
            ..tool.clone()
        };
        assert_eq!(too_deep("Pocket 1", &unknown, 25.0, Units::Mm), None);
        // in an inch job, in inches: never 25.0 in
        let said = too_deep("Pocket 1", &tool, 25.4, Units::Inch).unwrap();
        assert!(said.contains("1.000 in deep"), "{said}");
        assert!(said.contains("only 0.787 in long"), "{said}");
    }

    /// A 3/4 inch 90° V-bit with no flute length, as V-bits come.
    fn v_bit() -> Tool {
        Tool {
            name: "3/4 in V-Bit".into(),
            shape: ToolShape::VBit,
            diameter: 19.05,
            angle: 90.0,
            flute_length: 0.0,
            ..Tool::default()
        }
    }

    #[test]
    fn a_v_bit_reaches_as_deep_as_its_v_without_a_flute_length() {
        // a V-bit has no flute length: the V itself is how deep it cuts, half its width
        // over the tangent of half its angle, 9.525 mm here
        let tool = v_bit();
        assert!((tool.cutting_length().unwrap() - 9.525).abs() < 1e-9);
        assert_eq!(too_deep("V-Carve", &tool, 2.1, Units::Mm), None);
        let said = too_deep("V-Carve", &tool, 12.0, Units::Mm).unwrap();
        assert!(said.contains("12.0 mm deep"), "{said}");
        assert!(
            said.contains("V of 3/4 in V-Bit is only 9.5 mm deep"),
            "{said}"
        );
        assert!(!said.contains("flute"), "{said}");
        // and the form never asks for a flute length
        assert_eq!(
            form_line(&tool, Some(2.1), None, Units::Mm),
            Some(Line::Reaches("Reaches everything".into()))
        );
        let Some(Line::Short(short)) = form_line(&tool, Some(12.0), None, Units::Mm) else {
            panic!("too deep is said");
        };
        assert!(short.contains("the V 9.5 mm"), "{short}");
    }

    #[test]
    fn a_missed_sentence_says_what_and_how_to_fix_it() {
        let m = Missed {
            spots: 3,
            area: 210.0,
            most: 0.0,
        };
        let said = missed_sentence("Pocket 1", &end_mill(6.0), &m, Fix::Pocket, Units::Mm);
        assert!(
            said.starts_with("Pocket 1: 6 mm End Mill can't reach 3 spots (2.1 cm²)"),
            "{said}"
        );
        assert!(said.contains("Clear with bigger bits first"), "{said}");
        let one = Missed {
            spots: 1,
            area: 12.0,
            most: 2.5,
        };
        let ball = Tool {
            shape: ToolShape::BallNose,
            taper: 10.0,
            shank_diameter: 6.0,
            ..end_mill(1.0)
        };
        let said = missed_sentence("Finish", &ball, &one, Fix::Finish, Units::Mm);
        assert!(said.contains("1 spot (12.0 mm²)"), "{said}");
        assert!(said.contains("tip and taper"), "{said}");
        assert!(said.contains("up to 2.5 mm"), "{said}");
        assert!(said.contains("smaller bit"), "{said}");
        let said = missed_sentence("Finish", &ball, &one, Fix::Finish, Units::Inch);
        assert!(said.contains("1 spot (0.019 in²)"), "{said}");
        assert!(said.contains("up to 0.098 in"), "{said}");
    }

    #[test]
    fn the_form_line_says_reaches_short_or_unknown() {
        let tool = end_mill(6.0);
        let none = Missed::default();
        assert_eq!(
            form_line(&tool, Some(10.0), Some(&none), Units::Mm),
            Some(Line::Reaches("Reaches everything".into()))
        );
        let deep = form_line(&tool, Some(25.0), None, Units::Mm).unwrap();
        assert!(matches!(deep, Line::Short(_)), "{deep:?}");
        assert!(deep.text().contains("too deep"), "{deep:?}");
        let m = Missed {
            spots: 2,
            area: 50.0,
            most: 0.0,
        };
        let wide = form_line(&tool, Some(10.0), Some(&m), Units::Mm).unwrap();
        assert!(wide.text().contains("too wide for 2 spots"), "{wide:?}");
        let deep = form_line(&tool, Some(25.4), None, Units::Inch).unwrap();
        assert!(
            deep.text()
                .contains("the cut is 1.000 in, its flutes 0.787 in"),
            "{deep:?}"
        );
        let unknown = Tool {
            flute_length: 0.0,
            ..tool.clone()
        };
        let unchecked = form_line(&unknown, Some(10.0), Some(&none), Units::Mm).unwrap();
        assert!(matches!(unchecked, Line::Unknown(_)), "{unchecked:?}");
        assert!(
            unchecked.text().starts_with("Fits every spot"),
            "{unchecked:?}"
        );
        let unpicked = Tool {
            name: String::new(),
            ..Tool::default()
        };
        assert_eq!(
            form_line(&unpicked, Some(10.0), None, Units::Mm),
            None,
            "no bit"
        );
        assert_eq!(
            form_line(&tool, None, None, Units::Mm),
            None,
            "nothing known yet"
        );
    }

    #[test]
    fn a_pit_narrower_than_a_ball_is_missed_but_the_foot_of_a_wall_is_not() {
        let doc = crate::testing::domed(); // an 8 mm dome meeting the flat at a right angle
        let ball = |d: f64| Tool {
            shape: ToolShape::BallNose,
            ..end_mill(d)
        };
        let model = crate::component::placed_model(&doc, &ball(2.0)).unwrap();
        assert!(
            !surface(&model, &ball(2.0), 0.1).any(),
            "the ball's own fillet at the dome's foot is expected"
        );
        // a pit 1 mm across and 3 mm deep in the flat
        let mut pitted = model.clone();
        let mut dug = 0;
        for row in 0..pitted.rows {
            for column in 0..pitted.columns {
                let at = pitted.centre(column, row);
                let corner = pitted.centre(0, 0);
                if at.distance(corner + Point::new(3.0, 3.0)) < 0.5 {
                    pitted.heights[row * pitted.columns + column] -= 3.0;
                    dug += 1;
                }
            }
        }
        assert!(dug >= 2, "the pit is on the grid");
        let m = surface(&pitted, &ball(2.0), 0.1);
        assert_eq!(m.spots, 1, "{m:?}");
        assert!(m.most > 1.5, "{m:?}");
    }
}
