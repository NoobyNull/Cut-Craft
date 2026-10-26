//! Sculpting: a brush dragged over the selected component (several are baked into one)
//! smooths, smudges, adds or removes height, or brings back the shape it had before
//! sculpting. Each drag is one stroke and one undoable change.

use cut_craft_core::component::{Component, chosen_or_baked};
use cut_craft_core::document::Document;
use cut_craft_core::geometry::Point;
use cut_craft_core::params::Xy;
use cut_craft_core::{Change, Command, CommandError, Drag, Interactive, Meta, ensure_positive};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct Sculpt;

/// What the brush does (Aspire's keys 1 to 5).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Tool {
    /// Averages the highs and lows under the brush
    #[default]
    Smooth,
    /// Drags the material along the stroke, like clay under a thumb
    Smudge,
    /// Adds height
    Deposit,
    /// Takes height away
    Remove,
    /// Brings back the shape from before sculpting
    #[schemars(title = "Undo Brush")]
    UndoBrush,
}

/// Which heights the brush may change.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    /// Up or down as the tool goes
    #[default]
    Normal,
    /// Only up: fills holes
    Raise,
    /// Only down: takes off spikes
    Lower,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    pub tool: Tool,
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub diameter: f64,
    /// How much each stroke changes
    #[schemars(extend("x-unit" = "%"), range(min = 0, max = 100))]
    pub strength: f64,
    /// How gently the brush's effect fades toward its edge
    #[schemars(extend("x-unit" = "%"), range(min = 0, max = 100))]
    pub smoothness: f64,
    /// Sculpt only inside the component's shape, not into the plane round it
    #[schemars(title = "Preserve Transparency")]
    pub preserve_transparency: bool,
    pub mode: Mode,
    /// With the Undo Brush: remove the shape under the brush altogether (Aspire's Shift)
    pub erase: bool,
    /// The stroke; drag over the component in the 2D view to sculpt
    #[schemars(skip)]
    #[serde(default)]
    pub points: Vec<Xy>,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            tool: Tool::Smooth,
            // ponytail: starting values [inf]; Aspire sizes its sliders from the job
            diameter: 10.0,
            strength: 20.0,
            smoothness: 50.0,
            preserve_transparency: true,
            mode: Mode::Normal,
            erase: false,
            points: vec![Xy::new(16.0, 20.0), Xy::new(24.0, 20.0)],
        }
    }
}

/// Dabs along a stroke, this fraction of the brush diameter apart.
// ponytail: an eighth of the diameter [inf]; Aspire's dab spacing is not described
const DAB_SPACING: f64 = 0.125;
/// Deposit and Remove change a dab's centre by this fraction of the diameter at full
/// strength.
// ponytail: [inf]; Aspire gives no rate for its strength slider
const DEPOSIT_RATE: f64 = 0.05;

impl Params {
    /// The brush's weight at `distance` from its centre: full strength in its hard middle,
    /// fading smoothly to nothing at its edge.
    fn weight(&self, distance: f64) -> f64 {
        let t = distance / (self.diameter / 2.0);
        if t >= 1.0 {
            return 0.0;
        }
        let hard = 1.0 - self.smoothness / 100.0;
        let fade = if t <= hard {
            1.0
        } else {
            let s = (t - hard) / (1.0 - hard);
            1.0 - s * s * (3.0 - 2.0 * s)
        };
        fade * self.strength / 100.0
    }

    /// The stroke as dab centres, evenly spaced, the first point included.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss,
        reason = "a small dab count"
    )]
    fn dabs(&self) -> Vec<Point> {
        let points: Vec<Point> = self.points.iter().map(|&p| p.into()).collect();
        let step = self.diameter * DAB_SPACING;
        let mut dabs = points.first().copied().into_iter().collect::<Vec<_>>();
        for pair in points.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            let n = (a.distance(b) / step).ceil() as usize;
            dabs.extend((1..=n).map(|i| a.lerp(b, i as f64 / n as f64)));
        }
        dabs
    }
}

impl Command for Sculpt {
    const ID: &'static str = "sculpt";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Sculpting",
            group: "Modeling",
            icon: "modeling/sculpting.svg",
            shortcut: "",
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        ensure_positive("diameter", p.diameter)?;
        if p.points.is_empty() {
            return Err(CommandError::InvalidParams(
                "drag over the component to sculpt".into(),
            ));
        }
        let (ids, before) = chosen_or_baked(doc).ok_or_else(|| {
            CommandError::Failed("there is no component to sculpt: create one first".into())
        })?;
        let after = sculpted(&before, p);
        let mut c = Change::builder(doc);
        if let [one] = ids.as_slice() {
            c.replace_component(*one, after);
        } else {
            for id in ids {
                c.remove_component(id);
            }
            c.add_component(after);
        }
        Ok(c.finish())
    }
}

/// `k` after one stroke of the brush.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "cell indices are small"
)]
fn sculpted(k: &Component, p: &Params) -> Component {
    let mut out = k.clone();
    let original = out
        .unsculpted
        .get_or_insert_with(|| k.heights.clone())
        .clone();
    let (columns, rows) = (k.columns as isize, k.rows as isize);
    let radius = p.diameter / 2.0;
    let reach = (radius / k.cell).ceil() as isize;
    let rate = p.diameter * DEPOSIT_RATE;
    let dabs = p.dabs();
    let mut previous = dabs[0];
    for &centre in &dabs {
        let snapshot = out.heights.clone();
        let at = |column: isize, row: isize| -> Option<f64> {
            (column >= 0 && row >= 0 && column < columns && row < rows)
                .then(|| snapshot[(row * columns + column) as usize])
                .flatten()
        };
        let local = (centre - k.origin) / k.cell - 0.5;
        let (middle_column, middle_row) = (local.x.round() as isize, local.y.round() as isize);
        // smudging pulls each cell's height from where the brush just was
        let pull = ((centre - previous) / k.cell).round();
        for row in (middle_row - reach).max(0)..=(middle_row + reach).min(rows - 1) {
            for column in (middle_column - reach).max(0)..=(middle_column + reach).min(columns - 1)
            {
                let index = (row * columns + column) as usize;
                let w = p.weight(k.centre_of(index).distance(centre));
                if w <= 0.0 {
                    continue;
                }
                let here = snapshot[index];
                if here.is_none() && p.preserve_transparency {
                    continue;
                }
                if p.tool == Tool::UndoBrush && p.erase {
                    out.heights[index] = None;
                    continue;
                }
                let z = here.unwrap_or(0.0);
                let target = match p.tool {
                    Tool::Smooth => {
                        let around: Vec<f64> = (-1..=1)
                            .flat_map(|dr| (-1..=1).map(move |dc| (dc, dr)))
                            .map(|(dc, dr)| at(column + dc, row + dr).unwrap_or(0.0))
                            .collect();
                        around.iter().sum::<f64>() / around.len() as f64
                    }
                    Tool::Smudge => {
                        at(column - pull.x as isize, row - pull.y as isize).unwrap_or(z)
                    }
                    Tool::Deposit => z + rate,
                    Tool::Remove => z - rate,
                    Tool::UndoBrush => original[index].unwrap_or(0.0),
                };
                let mut new = z + (target - z) * w;
                match p.mode {
                    Mode::Normal => {}
                    Mode::Raise => new = new.max(z),
                    Mode::Lower => new = new.min(z),
                }
                out.heights[index] = Some(new);
            }
        }
        previous = centre;
    }
    out
}

impl Interactive for Sculpt {
    /// The app fills in the stroke's points; a drag with none is a straight stroke.
    fn from_drag(drag: Drag, current: &Params) -> Params {
        let mut p = current.clone();
        if p.points.len() < 2 {
            p.points = vec![
                Xy::new(drag.start.x, drag.start.y),
                Xy::new(drag.end.x, drag.end.y),
            ];
        }
        p
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::component::Combine;
    use cut_craft_core::document::EntityId;
    use cut_craft_core::geometry::Rect;
    use cut_craft_core::testing::{domed, rejection, run_on};

    cut_craft_core::contract_tests!(Sculpt, domed);

    /// A 40 mm square job holding one component: `z` over the middle 20 mm.
    fn plateau(z: impl Fn(Point) -> f64) -> Document {
        let mut doc = domed();
        let area = Rect::new(Point::splat(10.0), Point::splat(30.0));
        let k = Component::from_fn("plateau", Combine::Add, area, 0.5, |p| Some(z(p)));
        let mut c = Change::builder(&doc);
        c.replace_component(EntityId(1), k);
        doc.apply(&c.finish()).unwrap();
        doc
    }

    fn height(doc: &Document, x: f64, y: f64) -> Option<f64> {
        doc.components[&EntityId(1)].height_at(Point::new(x, y))
    }

    fn stroke(tool: Tool) -> Params {
        Params {
            tool,
            strength: 100.0,
            points: vec![Xy::new(14.0, 20.0), Xy::new(26.0, 20.0)],
            ..Params::default()
        }
    }

    #[test]
    fn deposit_raises_under_the_stroke_only() {
        let mut doc = plateau(|_| 2.0);
        run_on::<Sculpt>(&mut doc, &stroke(Tool::Deposit));
        assert!(height(&doc, 20.0, 20.0).unwrap() > 3.0);
        assert!(
            (height(&doc, 20.0, 28.0).unwrap() - 2.0).abs() < 1e-9,
            "outside the brush"
        );
    }

    #[test]
    fn remove_lowers_under_the_stroke() {
        let mut doc = plateau(|_| 5.0);
        run_on::<Sculpt>(&mut doc, &stroke(Tool::Remove));
        assert!(height(&doc, 20.0, 20.0).unwrap() < 4.0);
    }

    #[test]
    fn smooth_flattens_a_spike_and_raise_mode_keeps_its_top() {
        let spike = |p: Point| {
            if p.distance(Point::splat(20.0)) < 0.6 {
                6.0
            } else {
                1.0
            }
        };
        let mut smoothed = plateau(spike);
        let p = Params {
            points: vec![Xy::new(20.0, 20.0)],
            ..stroke(Tool::Smooth)
        };
        run_on::<Sculpt>(&mut smoothed, &p);
        assert!(height(&smoothed, 20.0, 20.0).unwrap() < 5.0);
        let mut raised = plateau(spike);
        run_on::<Sculpt>(
            &mut raised,
            &Params {
                mode: Mode::Raise,
                ..p
            },
        );
        assert!((height(&raised, 20.0, 20.0).unwrap() - 6.0).abs() < 1e-9);
    }

    #[test]
    fn smudge_drags_the_high_side_into_the_low() {
        let step = |p: Point| if p.x < 20.0 { 5.0 } else { 1.0 };
        let mut doc = plateau(step);
        run_on::<Sculpt>(
            &mut doc,
            &Params {
                points: vec![Xy::new(17.0, 20.0), Xy::new(23.0, 20.0)],
                ..stroke(Tool::Smudge)
            },
        );
        assert!(height(&doc, 21.0, 20.0).unwrap() > 2.0);
    }

    #[test]
    fn the_undo_brush_brings_back_the_shape_from_before_sculpting() {
        let mut doc = plateau(|_| 2.0);
        run_on::<Sculpt>(&mut doc, &stroke(Tool::Deposit));
        for _ in 0..3 {
            run_on::<Sculpt>(&mut doc, &stroke(Tool::UndoBrush));
        }
        assert!((height(&doc, 20.0, 20.0).unwrap() - 2.0).abs() < 1e-6);
        let erase = Params {
            erase: true,
            ..stroke(Tool::UndoBrush)
        };
        run_on::<Sculpt>(&mut doc, &erase);
        assert_eq!(height(&doc, 20.0, 20.0), None);
    }

    #[test]
    fn preserve_transparency_keeps_the_brush_inside_the_shape() {
        let mut doc = domed();
        let edge = Params {
            points: vec![Xy::new(26.5, 26.5)],
            ..stroke(Tool::Deposit)
        };
        run_on::<Sculpt>(&mut doc, &edge);
        assert_eq!(
            height(&doc, 26.5, 26.5),
            None,
            "a corner of the grid outside the dome"
        );
        let open = Params {
            preserve_transparency: false,
            ..edge
        };
        run_on::<Sculpt>(&mut doc, &open);
        assert!(height(&doc, 26.5, 26.5).is_some_and(|z| z > 0.0));
    }

    #[test]
    fn needs_a_stroke_and_a_component() {
        let none = Params {
            points: vec![],
            ..Params::default()
        };
        assert!(Sculpt::run(&domed(), &none).is_err());
        assert!(rejection::<Sculpt>(&Params::default()).contains("no component"));
    }
}
