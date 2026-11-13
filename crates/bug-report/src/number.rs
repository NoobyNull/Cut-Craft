//! Every number box in Cut Craft: a value to type in, with small up and down buttons on its
//! right that nudge it a step at a time (held down, they keep going). Dragging sideways on
//! the value still changes it too. Kept here, the one egui crate every window shares, so each
//! form's numbers look and work the same.

use std::ops::RangeInclusive;

use eframe::egui;
use eframe::egui::emath::Numeric;

/// How long a button is held before it starts repeating, and how often it then repeats (s).
const REPEAT_AFTER: f64 = 0.4;
const REPEAT_EVERY: f64 = 0.06;

/// What the status bar says over the value itself.
pub const VALUE_HINT: &str =
    "Type a number, click the arrows on the right to nudge it, or drag sideways";

/// How much one click of an arrow changes a value shown with `unit` (as typed after the
/// number, " mm" or "in/min"): a step that means something in that unit. `integer` values
/// never step by less than 1.
pub fn step_for(unit: &str, integer: bool) -> f64 {
    let step: f64 = match unit.trim().to_lowercase().as_str() {
        "mm" | "px" | "×" | "x" | "m/min" | "inches/sec" | "in/sec" | "in/s" => 0.1,
        "in" | "inch" | "inches" | "\"" => 0.01,
        "mm/min" => 100.0,
        "mm/sec" | "mm/s" | "frames/s" | "µm" | "%" | "°" | "deg" | "s" | "sec" | "min"
        | "days" => 1.0,
        "inches/min" | "in/min" => 5.0,
        "feet/min" | "ft/min" => 0.5,
        "rpm" | "r.p.m." => 1000.0,
        "mb" => 10.0,
        _ if integer => 1.0,
        _ => 0.1,
    };
    if integer { step.max(1.0).round() } else { step }
}

/// `value` moved one `step` up (or down), kept in `range`, without the float dust that adding
/// tenths leaves (6.35 + 0.1 is 6.45, not 6.449999).
pub fn nudged(value: f64, step: f64, up: bool, range: &RangeInclusive<f64>) -> f64 {
    let moved = if up { value + step } else { value - step };
    let clean = (moved * 1e9).round() / 1e9;
    let clean = if clean.is_finite() { clean } else { moved };
    clean.clamp(*range.start(), *range.end())
}

/// Reads typed text as a number.
type Parser<'a> = Box<dyn 'a + Fn(&str) -> Option<f64>>;

/// A number box with up and down buttons; use it as [`egui::DragValue`] is used.
#[must_use = "add it to a ui with `ui.add(...)`"]
pub struct Number<'a, N: Numeric> {
    value: &'a mut N,
    range: RangeInclusive<f64>,
    speed: Option<f64>,
    prefix: String,
    suffix: String,
    max_decimals: Option<usize>,
    fixed_decimals: Option<usize>,
    step: Option<f64>,
    parser: Option<Parser<'a>>,
}

impl<'a, N: Numeric> Number<'a, N> {
    pub fn new(value: &'a mut N) -> Self {
        let range = if N::INTEGRAL {
            N::MIN.to_f64()..=N::MAX.to_f64()
        } else {
            f64::NEG_INFINITY..=f64::INFINITY
        };
        Self {
            value,
            range,
            speed: None,
            prefix: String::new(),
            suffix: String::new(),
            max_decimals: None,
            fixed_decimals: None,
            step: None,
            parser: None,
        }
    }

    /// The lowest and highest the value may be; the arrows stop there.
    pub fn range<R: Numeric>(mut self, range: RangeInclusive<R>) -> Self {
        self.range = range.start().to_f64()..=range.end().to_f64();
        self
    }

    /// How much dragging sideways by one point changes the value.
    pub fn speed(mut self, speed: impl Into<f64>) -> Self {
        self.speed = Some(speed.into());
        self
    }

    pub fn prefix(mut self, prefix: impl Into<String>) -> Self {
        self.prefix = prefix.into();
        self
    }

    /// The unit shown after the value; it also picks the arrows' step (see [`step_for`]).
    pub fn suffix(mut self, suffix: impl Into<String>) -> Self {
        self.suffix = suffix.into();
        self
    }

    pub fn max_decimals(mut self, decimals: usize) -> Self {
        self.max_decimals = Some(decimals);
        self
    }

    pub fn fixed_decimals(mut self, decimals: usize) -> Self {
        self.fixed_decimals = Some(decimals);
        self
    }

    /// The arrows' step, in place of the one its unit gives.
    pub fn step(mut self, step: f64) -> Self {
        self.step = Some(step);
        self
    }

    /// Reads what is typed (a sum, say) in place of a plain number.
    pub fn custom_parser(mut self, parser: impl 'a + Fn(&str) -> Option<f64>) -> Self {
        self.parser = Some(Box::new(parser));
        self
    }

    fn integer(&self) -> bool {
        N::INTEGRAL || self.fixed_decimals == Some(0) || self.max_decimals == Some(0)
    }

    /// One click's step: its own, else its unit's.
    pub fn nudge_step(&self) -> f64 {
        self.step
            .unwrap_or_else(|| step_for(&self.suffix, self.integer()))
    }
}

impl<N: Numeric> egui::Widget for Number<'_, N> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let step = self.nudge_step();
        let integer = self.integer();
        let Self {
            value,
            range,
            speed,
            prefix,
            suffix,
            max_decimals,
            fixed_decimals,
            parser,
            ..
        } = self;
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 1.0;
            let mut drag = egui::DragValue::new(&mut *value).range(range.clone());
            if let Some(speed) = speed {
                drag = drag.speed(speed);
            }
            if !prefix.is_empty() {
                drag = drag.prefix(prefix);
            }
            if !suffix.is_empty() {
                drag = drag.suffix(suffix.clone());
            }
            if let Some(d) = max_decimals {
                drag = drag.max_decimals(d);
            }
            if let Some(d) = fixed_decimals {
                drag = drag.fixed_decimals(d);
            }
            if let Some(parser) = parser {
                drag = drag.custom_parser(parser);
            }
            let field = ui.add(drag);
            if field.hovered() {
                crate::hint::hint(ui.ctx(), VALUE_HINT);
            }
            let now = value.to_f64();
            let (up, down) = arrows(
                ui,
                field.id,
                field.rect.height(),
                (now < *range.end(), now > *range.start()),
            );
            let big = ui.input(|i| i.modifiers.shift);
            let unit = suffix.trim();
            let by = if big { step * 10.0 } else { step };
            let words = |which: &str| {
                format!(
                    "{which} by {}{}{unit}; hold Shift for {}{}{unit}",
                    trimmed(step),
                    if unit.is_empty() { "" } else { " " },
                    trimmed(step * 10.0),
                    if unit.is_empty() { "" } else { " " },
                )
            };
            if up.response.hovered() {
                crate::hint::hint(ui.ctx(), &words("Up"));
            }
            if down.response.hovered() {
                crate::hint::hint(ui.ctx(), &words("Down"));
            }
            let mut response = field | up.response | down.response;
            let times = up.times.cast_signed() - down.times.cast_signed();
            if times != 0 {
                let mut n = now;
                for _ in 0..times.unsigned_abs() {
                    n = nudged(n, by, times > 0, &range);
                }
                if integer {
                    n = n.round();
                }
                if n.to_bits() != now.to_bits() {
                    *value = N::from_f64(n);
                    response.mark_changed();
                }
            }
            response
        })
        .inner
    }
}

/// A step as the status bar writes it: 0.1, 5, 1000.
fn trimmed(n: f64) -> String {
    let s = format!("{n:.4}");
    s.trim_end_matches('0').trim_end_matches('.').to_owned()
}

/// A button's response and how many steps it asked for this frame.
struct Pressed {
    response: egui::Response,
    times: u32,
}

/// The up and down buttons, stacked as high as the value beside them; each may be off (at
/// its limit).
fn arrows(
    ui: &mut egui::Ui,
    id: egui::Id,
    height: f32,
    (can_up, can_down): (bool, bool),
) -> (Pressed, Pressed) {
    let width = (height * 0.75).max(10.0);
    let (whole, _) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::hover());
    let half = egui::vec2(width, height / 2.0);
    let top = egui::Rect::from_min_size(whole.min, half);
    let bottom = egui::Rect::from_min_size(whole.min + egui::vec2(0.0, height / 2.0), half);
    let up = arrow(ui, top, id.with("number_up"), true, can_up);
    let down = arrow(ui, bottom, id.with("number_down"), false, can_down);
    (up, down)
}

/// When a button was first pressed and when it last stepped, while it is held.
#[derive(Clone, Copy)]
struct Held {
    since: f64,
    last: f64,
}

fn arrow(ui: &mut egui::Ui, rect: egui::Rect, id: egui::Id, up: bool, on: bool) -> Pressed {
    let sense = if on && ui.is_enabled() {
        egui::Sense::click()
    } else {
        egui::Sense::hover()
    };
    let response = ui.interact(rect, id, sense);
    let mut times = 0;
    let now = ui.input(|i| i.time);
    if on && response.is_pointer_button_down_on() {
        let held = ui.data(|d| d.get_temp::<Held>(id));
        match held {
            None => {
                times = 1;
                ui.data_mut(|d| {
                    d.insert_temp(
                        id,
                        Held {
                            since: now,
                            last: now,
                        },
                    )
                });
            }
            Some(h) if now - h.since > REPEAT_AFTER && now - h.last >= REPEAT_EVERY => {
                times = 1;
                ui.data_mut(|d| d.insert_temp(id, Held { last: now, ..h }));
            }
            Some(_) => {}
        }
        ui.ctx().request_repaint();
    } else {
        ui.data_mut(|d| d.remove::<Held>(id));
    }
    if ui.is_rect_visible(rect) {
        let visuals = if on && ui.is_enabled() {
            *ui.style().interact(&response)
        } else {
            ui.style().visuals.widgets.noninteractive
        };
        let painter = ui.painter();
        let corner = if up {
            egui::CornerRadius {
                ne: 2,
                ..Default::default()
            }
        } else {
            egui::CornerRadius {
                se: 2,
                ..Default::default()
            }
        };
        painter.rect(
            rect.shrink(0.5),
            corner,
            visuals.weak_bg_fill,
            visuals.bg_stroke,
            egui::StrokeKind::Inside,
        );
        let c = rect.center();
        let w = (rect.width() * 0.25).min(rect.height() * 0.4);
        let h = w * 0.8;
        let points = if up {
            vec![
                egui::pos2(c.x - w, c.y + h / 2.0),
                egui::pos2(c.x + w, c.y + h / 2.0),
                egui::pos2(c.x, c.y - h / 2.0),
            ]
        } else {
            vec![
                egui::pos2(c.x - w, c.y - h / 2.0),
                egui::pos2(c.x, c.y + h / 2.0),
                egui::pos2(c.x + w, c.y - h / 2.0),
            ]
        };
        let colour = if on && ui.is_enabled() {
            visuals.fg_stroke.color
        } else {
            ui.style().visuals.weak_text_color()
        };
        painter.add(egui::Shape::convex_polygon(
            points,
            colour,
            egui::Stroke::NONE,
        ));
    }
    Pressed { response, times }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_unit_steps_by_an_amount_that_means_something_in_it() {
        assert_eq!(step_for(" mm", false), 0.1);
        assert_eq!(step_for(" in", false), 0.01);
        assert_eq!(step_for(" %", false), 1.0);
        assert_eq!(step_for("°", false), 1.0);
        assert_eq!(step_for(" mm/min", false), 100.0);
        assert_eq!(step_for("inches/min", false), 5.0);
        assert_eq!(step_for(" in/min", false), 5.0);
        assert_eq!(step_for(" r.p.m.", false), 1000.0);
        assert_eq!(step_for(" rpm", false), 1000.0);
        assert_eq!(step_for("", true), 1.0);
        assert_eq!(step_for("", false), 0.1);
    }

    #[test]
    fn whole_numbers_never_step_by_less_than_one() {
        assert_eq!(step_for(" mm", true), 1.0);
        assert_eq!(step_for(" in", true), 1.0);
        assert_eq!(step_for(" rpm", true), 1000.0);
    }

    #[test]
    fn a_nudge_leaves_no_float_dust() {
        let any = f64::NEG_INFINITY..=f64::INFINITY;
        assert_eq!(nudged(6.35, 0.1, true, &any), 6.45);
        assert_eq!(nudged(0.25, 0.01, false, &any), 0.24);
        assert_eq!(nudged(0.3, 0.1, false, &any), 0.2);
    }

    #[test]
    fn a_nudge_stops_at_the_fields_limits() {
        assert_eq!(nudged(99.5, 1.0, true, &(0.0..=100.0)), 100.0);
        assert_eq!(nudged(0.05, 0.1, false, &(0.0..=100.0)), 0.0);
        assert_eq!(nudged(1.0, 1.0, false, &(1.0..=12.0)), 1.0);
    }

    #[test]
    fn its_own_step_wins_over_its_units() {
        let mut v = 1.0;
        assert_eq!(
            Number::new(&mut v).suffix(" mm").step(0.5).nudge_step(),
            0.5
        );
        let mut v = 1.0;
        assert_eq!(
            Number::new(&mut v)
                .suffix(" mm")
                .fixed_decimals(0)
                .nudge_step(),
            1.0
        );
        let mut flutes = 2_u32;
        assert_eq!(Number::new(&mut flutes).nudge_step(), 1.0);
    }

    /// Runs one frame of a number box over `value`, the pointer at `pointer`, pressed or not;
    /// the box's rectangle.
    fn frame(
        ctx: &egui::Context,
        value: &mut f64,
        range: RangeInclusive<f64>,
        pointer: Option<(egui::Pos2, bool)>,
        time: f64,
    ) -> egui::Rect {
        let mut input = egui::RawInput {
            time: Some(time),
            ..Default::default()
        };
        if let Some((at, pressed)) = pointer {
            input.events.push(egui::Event::PointerMoved(at));
            input.events.push(egui::Event::PointerButton {
                pos: at,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: egui::Modifiers::NONE,
            });
        }
        let mut rect = egui::Rect::NOTHING;
        // no renderer takes a headless frame's texture changes
        ctx.run_ui(input, |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                rect = ui
                    .add(Number::new(value).range(range.clone()).suffix(" mm"))
                    .rect;
            });
        })
        .textures_delta
        .clear();
        rect
    }

    fn up_spot(rect: egui::Rect) -> egui::Pos2 {
        egui::pos2(rect.right() - 3.0, rect.top() + 3.0)
    }

    fn down_spot(rect: egui::Rect) -> egui::Pos2 {
        egui::pos2(rect.right() - 3.0, rect.bottom() - 3.0)
    }

    #[test]
    fn clicking_the_arrows_nudges_the_value_by_one_step() {
        let ctx = egui::Context::default();
        let mut value = 6.35;
        let rect = frame(&ctx, &mut value, 0.0..=100.0, None, 0.0);
        frame(
            &ctx,
            &mut value,
            0.0..=100.0,
            Some((up_spot(rect), true)),
            0.1,
        );
        frame(
            &ctx,
            &mut value,
            0.0..=100.0,
            Some((up_spot(rect), false)),
            0.15,
        );
        assert_eq!(value, 6.45);
        frame(
            &ctx,
            &mut value,
            0.0..=100.0,
            Some((down_spot(rect), true)),
            0.2,
        );
        frame(
            &ctx,
            &mut value,
            0.0..=100.0,
            Some((down_spot(rect), false)),
            0.25,
        );
        assert_eq!(value, 6.35);
    }

    #[test]
    fn holding_an_arrow_keeps_nudging() {
        let ctx = egui::Context::default();
        let mut value = 1.0;
        let rect = frame(&ctx, &mut value, 0.0..=100.0, None, 0.0);
        frame(
            &ctx,
            &mut value,
            0.0..=100.0,
            Some((up_spot(rect), true)),
            0.1,
        );
        // held, the pointer still: no events, only time passing
        for i in 1..=10 {
            frame(
                &ctx,
                &mut value,
                0.0..=100.0,
                None,
                0.1 + f64::from(i) * 0.1,
            );
        }
        assert!(
            value >= 1.4,
            "held for a second it should have gone on: {value}"
        );
    }

    #[test]
    fn the_arrow_does_nothing_at_the_limit() {
        let ctx = egui::Context::default();
        let mut value = 100.0;
        let rect = frame(&ctx, &mut value, 0.0..=100.0, None, 0.0);
        frame(
            &ctx,
            &mut value,
            0.0..=100.0,
            Some((up_spot(rect), true)),
            0.1,
        );
        frame(
            &ctx,
            &mut value,
            0.0..=100.0,
            Some((up_spot(rect), false)),
            0.15,
        );
        assert_eq!(value, 100.0);
    }

    /// Every number box is this one, so they all have the arrows: none is a bare `DragValue`.
    #[test]
    fn no_form_uses_a_number_box_without_arrows() {
        let crates = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
        let mut bare = Vec::new();
        let mut stack = vec![crates];
        while let Some(dir) = stack.pop() {
            for entry in std::fs::read_dir(&dir).expect("crates folder").flatten() {
                let path = entry.path();
                if path.is_dir() {
                    if path.file_name().is_some_and(|n| n != "target") {
                        stack.push(path);
                    }
                } else if path.extension().is_some_and(|e| e == "rs")
                    && !path.ends_with("bug-report/src/number.rs")
                {
                    let text = std::fs::read_to_string(&path).unwrap_or_default();
                    for (i, line) in text.lines().enumerate() {
                        if line.contains("DragValue::new(")
                            || line.contains("DragValue::from_get_set(")
                        {
                            bare.push(format!("{}:{}", path.display(), i + 1));
                        }
                    }
                }
            }
        }
        assert!(
            bare.is_empty(),
            "use cut_craft_bug_report::number::Number, so the box has its up and down \
             buttons:\n{}",
            bare.join("\n")
        );
    }
}
