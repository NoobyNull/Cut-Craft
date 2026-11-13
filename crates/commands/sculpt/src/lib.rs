//! Sculpting: a brush dragged over the selected component (several are baked into one)
//! smooths, smudges, adds or removes height, or brings back the shape it had before
//! sculpting. Each drag is one stroke and one undoable change. Deposit and Remove can press a
//! saved component in instead of the round brush: a custom brush.

use cut_craft_core::component::{Component, chosen_or_baked, height_or};
use cut_craft_core::document::Document;
use cut_craft_core::geometry::Point;
use cut_craft_core::params::Xy;
use cut_craft_core::{Change, Command, CommandError, Drag, Interactive, Meta, ensure_positive};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct Sculpt;

/// What the brush does (keys 1 to 5).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Tool {
    /// Averages the highs and lows under the brush
    #[default]
    Smooth,
    /// Drags the material along the stroke, like clay under a thumb
    Smudge,
    /// Adds height where you drag
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
#[expect(clippy::struct_excessive_bools, reason = "the tick boxes")]
pub struct Params {
    pub tool: Tool,
    /// How wide the brush is
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
    /// With the Undo Brush: remove the shape under the brush altogether (hold Shift)
    pub erase: bool,
    /// Deposit and Remove: a component saved as a sculpting brush (a `.ccclip` file, Load
    /// Brush… below) pressed into the surface, its larger side the diameter; empty for the
    /// round brush
    #[serde(default)]
    #[schemars(title = "Custom brush")]
    pub brush: String,
    /// The custom brush's turn, degrees anticlockwise
    #[serde(default)]
    #[schemars(title = "Angle", extend("x-unit" = "°"))]
    pub brush_angle: f64,
    /// The custom brush turns to follow the stroke
    #[serde(default)]
    #[schemars(title = "Dynamic Rotation")]
    pub dynamic_rotation: bool,
    /// A custom brush's stroke merges with the strokes before it (the higher wins) instead of
    /// being added onto them
    #[serde(default)]
    pub merge: bool,
    /// How far apart the brush presses along a stroke, as a share of its diameter. Closer
    /// gives a smoother stroke, but each one takes longer and works the surface harder
    #[serde(default = "dab_spacing")]
    #[schemars(
        title = "Dab spacing",
        extend("x-unit" = "%", "x-group" = "Brush Feel"),
        range(min = 1, max = 100)
    )]
    pub dab_spacing: f64,
    /// How much Deposit and Remove change the middle of each press at full strength, as a
    /// share of the diameter. Higher builds up or takes away faster
    #[serde(default = "deposit_rate")]
    #[schemars(
        title = "Build-up rate",
        extend("x-unit" = "%", "x-group" = "Brush Feel"),
        range(min = 0.1, max = 50)
    )]
    pub deposit_rate: f64,
    /// How deep a custom brush presses at full strength, as a share of its diameter
    #[serde(default = "brush_depth")]
    #[schemars(
        title = "Custom brush depth",
        extend("x-unit" = "%", "x-group" = "Brush Feel"),
        range(min = 1, max = 100)
    )]
    pub brush_depth: f64,
    /// The stroke; drag over the component in the 2D view to sculpt
    #[schemars(skip)]
    #[serde(default)]
    pub points: Vec<Xy>,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            tool: Tool::Smooth,
            // ponytail: starting values [inf]; the sliders could be sized from the job
            diameter: 10.0,
            strength: 20.0,
            smoothness: 50.0,
            preserve_transparency: true,
            mode: Mode::Normal,
            erase: false,
            brush: String::new(),
            brush_angle: 0.0,
            dynamic_rotation: false,
            merge: false,
            dab_spacing: dab_spacing(),
            deposit_rate: deposit_rate(),
            brush_depth: brush_depth(),
            points: vec![Xy::new(16.0, 20.0), Xy::new(24.0, 20.0)],
        }
    }
}

/// Dabs along a stroke, this percent of the brush diameter apart, unless the form says
/// otherwise.
// ponytail: an eighth of the diameter [inf]
fn dab_spacing() -> f64 {
    12.5
}

/// Deposit and Remove change a dab's centre by this percent of the diameter at full
/// strength, unless the form says otherwise.
// ponytail: [inf]; our own rate for the strength slider
fn deposit_rate() -> f64 {
    5.0
}

/// How deep a custom brush presses at full strength, as a percent of its diameter, unless
/// the form says otherwise.
// ponytail: a fifth [inf]; our own custom brush depth
fn brush_depth() -> f64 {
    20.0
}

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
        let step = self.diameter * self.dab_spacing.clamp(1.0, 100.0) / 100.0;
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
            icon: "modeling/sculpting.png",
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
        let custom = matches!(p.tool, Tool::Deposit | Tool::Remove) && !p.brush.trim().is_empty();
        let brush = custom.then(|| Brush::load(p.brush.trim())).transpose()?;
        let (ids, before) = chosen_or_baked(doc).ok_or_else(|| {
            CommandError::Failed("there is no component to sculpt: create one first".into())
        })?;
        let after = match &brush {
            Some(brush) => pressed(&before, p, brush),
            None => sculpted(&before, p),
        };
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
    let rate = p.diameter * p.deposit_rate.clamp(0.1, 50.0) / 100.0;
    let dabs = p.dabs();
    let mut previous = dabs[0];
    for &centre in &dabs {
        let snapshot = out.heights.clone();
        let at = |column: isize, row: isize| -> Option<f64> {
            (column >= 0 && row >= 0 && column < columns && row < rows)
                .then(|| snapshot[(row * columns + column) as usize])
                .filter(|h| !h.is_nan())
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
                if here.is_nan() && p.preserve_transparency {
                    continue;
                }
                if p.tool == Tool::UndoBrush && p.erase {
                    out.heights[index] = f64::NAN;
                    continue;
                }
                let z = height_or(here, 0.0);
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
                    Tool::UndoBrush => height_or(original[index], 0.0),
                };
                let mut new = z + (target - z) * w;
                match p.mode {
                    Mode::Normal => {}
                    Mode::Raise => new = new.max(z),
                    Mode::Lower => new = new.min(z),
                }
                out.heights[index] = new;
            }
        }
        previous = centre;
    }
    out
}

/// A custom brush: a saved component, its heights taken as 0 (its lowest) to 1 (its highest).
pub struct Brush {
    shape: Component,
    low: f64,
    high: f64,
}

impl Brush {
    /// The brush saved at `path`.
    // ponytail: read for every stroke (and preview); keep it with the form if that is slow
    pub fn load(path: &str) -> Result<Self, CommandError> {
        let bytes = std::fs::read(path)
            .map_err(|e| CommandError::Failed(format!("the brush {path}: {e}")))?;
        let shape = fmt_export_as_3d_clipart::read_clip(&bytes)?;
        let (low, high) = shape
            .height_range()
            .filter(|(low, high)| high > low)
            .ok_or_else(|| CommandError::Failed("the brush has no height to press with".into()))?;
        Ok(Self { shape, low, high })
    }

    /// 0 to 1, `offset` (the brush's own millimetres) from its middle; 0 off it.
    fn at(&self, offset: Point) -> f64 {
        let middle = self.shape.extent().center();
        self.shape.height_at(middle + offset).map_or(0.0, |h| {
            ((h - self.low) / (self.high - self.low)).clamp(0.0, 1.0)
        })
    }

    /// Its larger side.
    fn size(&self) -> f64 {
        self.shape.extent().size().max_element()
    }
}

/// `k` after one stroke of a custom brush: the brush pressed in at each dab (turned by the
/// angle, or along the stroke), the stroke's impression the highest of its dabs; added onto
/// the surface, or with Merge laid on the surface from before sculpting, the higher kept
/// (Remove: lowered, the lower kept).
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    reason = "cell indices are small"
)]
fn pressed(k: &Component, p: &Params, brush: &Brush) -> Component {
    let mut out = k.clone();
    let original = out
        .unsculpted
        .get_or_insert_with(|| k.heights.clone())
        .clone();
    let dabs = p.dabs();
    let depth = p.diameter * p.strength / 100.0 * p.brush_depth.clamp(1.0, 100.0) / 100.0;
    let scale = brush.size() / p.diameter;
    // the brush's square, turned, reaches its corners
    let reach_mm = p.diameter / 2.0 * std::f64::consts::SQRT_2;
    let fixed = p.brush_angle.to_radians();
    let angle = |i: usize| -> f64 {
        if !p.dynamic_rotation {
            return fixed;
        }
        let (from, to) = match (i.checked_sub(1), dabs.get(i + 1)) {
            (_, Some(&next)) => (dabs[i], next),
            (Some(before), None) => (dabs[before], dabs[i]),
            (None, None) => return fixed,
        };
        let along = to - from;
        if along.length() < 1e-9 {
            fixed
        } else {
            along.to_angle()
        }
    };
    let (columns, rows) = (k.columns as isize, k.rows as isize);
    let reach = (reach_mm / k.cell).ceil() as isize;
    let mut impression = vec![0.0_f64; k.heights.len()];
    for (i, &centre) in dabs.iter().enumerate() {
        let back = Point::from_angle(-angle(i));
        let local = (centre - k.origin) / k.cell - 0.5;
        let (middle_column, middle_row) = (local.x.round() as isize, local.y.round() as isize);
        for row in (middle_row - reach).max(0)..=(middle_row + reach).min(rows - 1) {
            for column in (middle_column - reach).max(0)..=(middle_column + reach).min(columns - 1)
            {
                let index = (row * columns + column) as usize;
                let d = k.centre_of(index) - centre;
                if d.length() > reach_mm {
                    continue;
                }
                let pressed = brush.at(back.rotate(d) * scale) * depth;
                impression[index] = impression[index].max(pressed);
            }
        }
    }
    let sign = if p.tool == Tool::Remove { -1.0 } else { 1.0 };
    for (index, &mark) in impression.iter().enumerate() {
        let here = k.heights[index];
        if mark <= 0.0 || (here.is_nan() && p.preserve_transparency) {
            continue;
        }
        let z = height_or(here, 0.0);
        let base = height_or(original[index], 0.0);
        let mut new = match (p.merge, sign > 0.0) {
            (false, _) => z + sign * mark,
            (true, true) => z.max(base + mark),
            (true, false) => z.min(base - mark),
        };
        match p.mode {
            Mode::Normal => {}
            Mode::Raise => new = new.max(z),
            Mode::Lower => new = new.min(z),
        }
        out.heights[index] = new;
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
    fn brush_feel_is_on_the_form_and_defaults_to_the_old_fixed_values() {
        let old: Params = serde_json::from_value(serde_json::json!({
            "tool": "deposit", "diameter": 10.0, "strength": 20.0, "smoothness": 50.0,
            "preserve_transparency": true, "mode": "normal", "erase": false
        }))
        .unwrap();
        assert_eq!(
            (old.dab_spacing, old.deposit_rate, old.brush_depth),
            (12.5, 5.0, 20.0)
        );
        // a faster deposit raises more under the same stroke
        let raised = |rate: f64| {
            let mut doc = plateau(|_| 2.0);
            run_on::<Sculpt>(
                &mut doc,
                &Params {
                    deposit_rate: rate,
                    ..stroke(Tool::Deposit)
                },
            );
            height(&doc, 20.0, 20.0).unwrap()
        };
        assert!(raised(10.0) > raised(5.0));
        // closer dabs along the stroke make more of them
        let dabs = |spacing: f64| {
            Params {
                dab_spacing: spacing,
                ..stroke(Tool::Deposit)
            }
            .dabs()
            .len()
        };
        assert!(dabs(5.0) > dabs(12.5));
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

    /// A 10 x 10 mm brush rising from 0 on its left to 10 on its right, saved to a file.
    fn ramp_brush(name: &str) -> String {
        let k = Component::from_fn(
            "ramp",
            Combine::Add,
            Rect::new(Point::ZERO, Point::splat(10.0)),
            0.5,
            |q| Some(q.x),
        );
        let path = std::env::temp_dir().join(format!(
            "cut-craft-brush-{}-{name}.ccclip",
            std::process::id()
        ));
        std::fs::write(&path, fmt_export_as_3d_clipart::clip_bytes(&k).unwrap()).unwrap();
        path.display().to_string()
    }

    fn stamp(brush: &str) -> Params {
        Params {
            tool: Tool::Deposit,
            strength: 100.0,
            brush: brush.to_owned(),
            points: vec![Xy::new(20.0, 20.0), Xy::new(20.0, 20.0)],
            ..Params::default()
        }
    }

    #[test]
    fn a_custom_brush_presses_its_shape_in_turned_by_its_angle() {
        let brush = ramp_brush("angle");
        let mut doc = plateau(|_| 2.0);
        run_on::<Sculpt>(&mut doc, &stamp(&brush));
        // depth 2 at full strength: high on the brush's right, low on its left
        let (right, left) = (
            height(&doc, 24.0, 20.0).unwrap(),
            height(&doc, 16.0, 20.0).unwrap(),
        );
        assert!(right > 3.5 && left < 2.5, "{right} {left}");
        assert!(
            (height(&doc, 28.0, 28.0).unwrap() - 2.0).abs() < 1e-9,
            "off the brush"
        );
        let mut turned = plateau(|_| 2.0);
        run_on::<Sculpt>(
            &mut turned,
            &Params {
                brush_angle: 180.0,
                ..stamp(&brush)
            },
        );
        let (right, left) = (
            height(&turned, 24.0, 20.0).unwrap(),
            height(&turned, 16.0, 20.0).unwrap(),
        );
        assert!(right < 2.5 && left > 3.5, "turned round: {right} {left}");
        // Remove presses down
        let mut removed = plateau(|_| 5.0);
        run_on::<Sculpt>(
            &mut removed,
            &Params {
                tool: Tool::Remove,
                ..stamp(&brush)
            },
        );
        assert!(height(&removed, 24.0, 20.0).unwrap() < 3.5);
        let _ = std::fs::remove_file(brush);
    }

    #[test]
    fn strokes_add_up_unless_merged_and_dynamic_rotation_follows_the_stroke() {
        let brush = ramp_brush("merge");
        let twice = |merge| {
            let mut doc = plateau(|_| 2.0);
            let p = Params {
                merge,
                ..stamp(&brush)
            };
            run_on::<Sculpt>(&mut doc, &p);
            run_on::<Sculpt>(&mut doc, &p);
            height(&doc, 20.0, 20.0).unwrap()
        };
        // the middle of the ramp presses about 1 deep
        let (added, merged) = (twice(false), twice(true));
        assert!((added - 4.0).abs() < 0.2, "{added}");
        assert!((merged - 3.0).abs() < 0.1, "{merged}");
        // up the job: turned to follow the stroke, the high side faces up, not right
        let up = |dynamic_rotation| {
            let mut doc = plateau(|_| 2.0);
            let p = Params {
                dynamic_rotation,
                points: vec![Xy::new(20.0, 18.0), Xy::new(20.0, 22.0)],
                ..stamp(&brush)
            };
            run_on::<Sculpt>(&mut doc, &p);
            height(&doc, 24.0, 20.0).unwrap()
        };
        assert!(up(true) < up(false) - 0.3, "{} vs {}", up(true), up(false));
        let missing = Params {
            brush: "/no/such/brush.ccclip".into(),
            ..stamp(&brush)
        };
        assert!(rejection::<Sculpt>(&missing).contains("brush"));
        let _ = std::fs::remove_file(brush);
    }
}
