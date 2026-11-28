//! Settings forms generated from a command's parameter schema, so no command writes UI.

use cut_craft_bug_report::hint::HoverHint;
use cut_craft_bug_report::number::Number;
use cut_craft_core::document::Units;
use cut_craft_i18n::tr;
use eframe::egui;
use serde_json::Value;
mod inputs;
pub use inputs::*;
mod layout;
pub use layout::*;
pub mod form_tabs;

/// Choices with this many options or fewer are radio buttons; more make a drop-down.
const MOST_RADIOS: usize = 5;
/// The corner picker's size, in rows of the theme's interactive height.
// ponytail: fixed ratio until a preferences dialog exists
const CORNER_PICKER_ROWS: f32 = 3.0;
/// The picture the face radio buttons sit round, in rows of the theme's interactive height.
// ponytail: fixed ratio until a preferences dialog exists
const FACES_PICTURE_ROWS: f32 = 4.0;
/// The tallest a measurements picture (`x-diagram`) is drawn, in rows of the theme's
/// interactive height; narrower side pages draw it smaller.
// ponytail: fixed ratio until a preferences dialog exists
const DIAGRAM_ROWS: f32 = 7.0;
/// A measurements picture's width over its height (they are drawn 448 x 224).
const DIAGRAM_ASPECT: f32 = 2.0;
/// How tall a measurements picture is beside the fields it marks (Options ▸ Appearance ▸
/// Form pictures), in rows of the theme's interactive height.
// ponytail: fixed ratio until a preferences dialog exists
const BESIDE_ROWS: f32 = 3.5;
/// The most of a form's width a picture beside the fields takes.
const BESIDE_SHARE: f32 = 0.45;
/// The least room the fields keep beside a picture, in rows of the theme's interactive
/// height; narrower, the picture goes above them, still small.
const FIELDS_LEAST_ROWS: f32 = 7.0;

/// Where measurement pictures go, as Options ▸ Appearance ▸ Form pictures says: kept in
/// egui's memory, so every form, and every window a form opens, follows it.
pub fn set_pictures(ctx: &egui::Context, pictures: crate::options::FormPictures) {
    ctx.data_mut(|d| d.insert_temp(pictures_id(), pictures));
}

/// Where measurement pictures go (see [`set_pictures`]).
fn pictures(ctx: &egui::Context) -> crate::options::FormPictures {
    ctx.data(|d| d.get_temp(pictures_id())).unwrap_or_default()
}

fn pictures_id() -> egui::Id {
    egui::Id::unique("form_pictures")
}
/// The callout letters' colour: the pictures' magenta (`assets/icons/diagrams`) in the light
/// theme, a lighter magenta on the dark theme's panel, where the pictures' own would be faint.
/// Magenta stays apart from the wood and the blue bit with any kind of colour blindness.
fn dim_colour(dark: bool) -> egui::Color32 {
    if dark {
        egui::Color32::from_rgb(0xe8, 0x6b, 0xb8)
    } else {
        egui::Color32::from_rgb(0xb0, 0x18, 0x7e)
    }
}
/// A field's own picture beside its label (small width / radius pictures), in rows
/// of the theme's interactive height.
// ponytail: fixed ratio until a preferences dialog exists
const FIELD_PICTURE_ROWS: f32 = 1.5;

#[derive(Debug, PartialEq)]
pub struct Field {
    /// JSON key in the params object.
    pub key: String,
    pub label: String,
    /// Doc comment of the parameter, shown on hover.
    pub help: Option<String>,
    pub kind: Kind,
    /// Marked `x-section`: drawn as a framed block with a heading.
    pub section: bool,
    /// `x-icon`: the section's picture, which may follow its value (see [`icon_path`]).
    pub icon: Option<String>,
    /// Only shown while another choice says so.
    pub when: Option<When>,
    /// Greyed out unless another choice says so (`x-enable-when`, written as `x-show-when`),
    /// so a section's settings stay greyed until it is ticked.
    pub enabled: Option<When>,
    /// `x-group`: neighbouring fields with the same group share a framed block under it as a
    /// heading (Cutting Depths, Machine Vectors…).
    pub group: Option<String>,
    /// `x-tab`: neighbouring fields with a tab show as a strip of tabs, a tab's fields under
    /// it (Ramp, Leads, Order, Start At, Corners).
    pub tab: Option<String>,
    /// `x-dim`: the measurement's callout letter (`D`, `D2`, `W`, `Dia`), drawn on the picture
    /// above it and in front of its name, so the two read together.
    pub dim: Option<String>,
    /// `x-diagram`: a picture of the measurements, shown once at the top of the field's group
    /// (or above the field when it has none), each one marked with its [`Field::dim`] letter.
    pub diagram: Option<String>,
    /// The axis a number is about, its name then drawn in that axis's colour as everywhere
    /// else: a number keyed `x`, `y` or `z`, or one marked `x-axis`.
    pub axis: Option<crate::axes::Axis>,
    /// `x-titles`: other names it takes while another choice says so (`{ "key": { "value":
    /// "title" } }`), as a rotary blank's size is its Diameter, Side or Thickness.
    pub titles: Option<Titles>,
}

/// A field's other names, one for each value of another choice ([`Field::titles`]).
#[derive(Debug, PartialEq)]
pub struct Titles {
    pub key: String,
    pub by: Vec<(String, String)>,
}

impl Titles {
    fn from_schema(s: &Value) -> Option<Self> {
        let (key, by) = s.get("x-titles")?.as_object()?.iter().next()?;
        let by = by
            .as_object()?
            .iter()
            .filter_map(|(value, title)| Some((value.clone(), title.as_str()?.to_owned())))
            .collect();
        Some(Self {
            key: key.clone(),
            by,
        })
    }
}

impl Field {
    /// Its name as the form's settings stand: its own section's (`local`) first, then the
    /// form's top-level ones (`top`), as [`When::judged`] reads them.
    pub fn label_for(&self, local: &Value, top: &Value) -> &str {
        self.titles
            .as_ref()
            .and_then(|t| {
                let setting = local.get(&t.key).unwrap_or(&top[&t.key]);
                let value = value_text(setting)?;
                t.by.iter()
                    .find(|(v, _)| *v == value)
                    .map(|(_, title)| title.as_str())
            })
            .unwrap_or(&self.label)
    }
}

/// When a field shows (`x-show-when` / `x-hide-when`: `{ "key": "value" or ["values"] }`,
/// a value written as text, a bool or a number),
/// judged on the form's top-level parameters: the forms change with what is chosen. A rule
/// naming more than one key holds when every one of them does.
#[derive(Clone, Debug, PartialEq)]
pub struct When {
    pub key: String,
    pub values: Vec<String>,
    /// Further keys and their values that must hold too.
    pub also: Vec<(String, Vec<String>)>,
    /// Shown when the value is one of `values`; else hidden then.
    pub show: bool,
}

impl When {
    fn from_schema(s: &Value) -> Option<Self> {
        let (rule, show) = match (s.get("x-show-when"), s.get("x-hide-when")) {
            (Some(rule), _) => (rule, true),
            (None, Some(rule)) => (rule, false),
            (None, None) => return None,
        };
        Self::from_rule(rule, show)
    }

    fn from_rule(rule: &Value, show: bool) -> Option<Self> {
        // a tick box's rule may be written as a bool or as "true" / "false", a count's as a
        // number or as text
        let text = value_text;
        let mut rules = rule.as_object()?.iter().map(|(key, values)| {
            let values = match values {
                Value::Array(list) => list.iter().filter_map(text).collect(),
                one => vec![text(one)?],
            };
            Some((key.clone(), values))
        });
        let (key, values) = rules.next()??;
        Some(Self {
            key,
            values,
            also: rules.collect::<Option<_>>()?,
            show,
        })
    }

    /// Whether the rule holds for a field inside a section: on the section's own settings
    /// (`local`) when its key is one of them (Job Setup's datum offset greyed by the datum's
    /// own tick), else on the form's top-level ones.
    pub fn judged(&self, local: &Value, top: &Value) -> bool {
        if local.get(&self.key).is_some() {
            self.shown(local)
        } else {
            self.shown(top)
        }
    }

    /// Whether the field shows, given the form's top-level `params`. A key with dots reads
    /// a setting inside another (`tool.shape`: the form's tool's shape), and `count` after a
    /// list how many it holds (`tools.count`: how many tools the form has; none when the
    /// list is not there).
    pub fn shown(&self, params: &Value) -> bool {
        let holds = |key: &String, values: &[String]| {
            let mut setting = params;
            let mut count = None;
            for part in key.split('.') {
                match (setting, part) {
                    (Value::Array(list), "count") => count = Some(Value::from(list.len())),
                    // a list not there holds none
                    (Value::Null, "count") => count = Some(Value::from(0)),
                    _ => setting = &setting[part],
                }
            }
            // a tick box reads as "true" or "false", a number as its digits ("1", not "1.0")
            let value = value_text(count.as_ref().unwrap_or(setting));
            value.is_some_and(|v| values.contains(&v))
        };
        let is = holds(&self.key, &self.values) && self.also.iter().all(|(k, v)| holds(k, v));
        is == self.show
    }
}

/// How a rule or a setting reads when they are compared: a tick box as "true" or "false", a
/// number as its digits (1 and 1.0 both "1"), text as it is.
fn value_text(v: &Value) -> Option<String> {
    match v {
        Value::Bool(ticked) => Some(ticked.to_string()),
        Value::Number(n) => n.as_f64().map(|n| n.to_string()),
        v => v.as_str().map(String::from),
    }
}

/// Draws a form's tool block given the parameters holding the tool (see [`Look::tool`]).
pub type DrawTool<'a> = dyn Fn(&mut egui::Ui, &str, &mut Value) + 'a;

/// Draws more at the foot of a group, given its name and the settings holding it; whether
/// they changed. See [`Look::group_end`].
pub type GroupEnd<'a> = dyn Fn(&mut egui::Ui, &str, &mut Value) -> bool + 'a;

/// The fields that are lists of Bit Library tools, drawn by [`Look::tool`] as the form's
/// own tool is.
pub const TOOL_LISTS: [&str; 3] = ["tools", "clearance_tools", "bits_before"];

/// Whether [`Look::tool`] draws the field keyed `key`: the bit, a list of bits
/// ([`TOOL_LISTS`]), or what follows the bit under the depths ([`AFTER_TOOL`]).
pub fn drawn_by_tool(key: &str) -> bool {
    key == "tool" || key == AFTER_TOOL || TOOL_LISTS.contains(&key)
}

/// How a form looks: the units lengths are shown in, and the pictures sections show.
pub struct Look<'a> {
    pub units: Units,
    /// A picture by icon path, sized for a section; `None` when the icon is missing.
    pub picture: &'a dyn Fn(&str) -> Option<egui::Image<'static>>,
    /// The material's size in the units shown, for sums typed into number fields (W, H, T).
    pub material: cut_craft_core::calc::Values,
    /// Draws a `tool` field, or a list of tools ([`TOOL_LISTS`]), in its place (the Bit
    /// Library's tools, which the form does not edit), and what follows the bit
    /// ([`AFTER_TOOL`]), given the field's key and the parameters holding it; `None` leaves
    /// them out.
    pub tool: Option<&'a DrawTool<'a>>,
    /// Draws more at the foot of an `x-group` block, given its name (Profile's Edit Tabs…
    /// under Tabs, Edit Loops… under a trochoidal toolpath's cut settings); `None` for
    /// nothing.
    pub group_end: Option<&'a GroupEnd<'a>>,
    /// The Materials Library, for `x-kind: material` fields; `None` shows them as text.
    pub materials: Option<&'a cut_craft_core::material_library::MaterialLibrary>,
    /// Draws a Sequence's samples and makes up a random one; `None` shows the box alone.
    pub pattern: Option<PatternMaker<'a>>,
}

/// A sample's outlines: rings of points, along x and across y.
pub type SampleRings = Vec<Vec<[f64; 2]>>;

/// What the pattern maker under a Sequence box needs from its command.
pub struct PatternMaker<'a> {
    /// A sample's outlines, along x and across y, for the form's settings and one entry
    /// (`2`, `R1/4`…).
    pub sample: &'a dyn Fn(&Value, &str) -> SampleRings,
    /// A Sequence made up from a number.
    pub random: &'a dyn Fn(u32) -> String,
}

impl Look<'_> {
    /// A look without pictures, tools, group ends or the Materials Library.
    pub fn plain(units: Units, material: cut_craft_core::calc::Values) -> Self {
        Look {
            units,
            picture: &no_picture,
            material,
            tool: None,
            group_end: None,
            materials: None,
            pattern: None,
        }
    }
}

fn no_picture(_: &str) -> Option<egui::Image<'static>> {
    None
}

/// Millimetres in one of `units`.
pub fn unit_mm(units: Units) -> f64 {
    match units {
        Units::Inch => 25.4,
        Units::Mm => 1.0,
    }
}

/// The job's material, in `units`, and its document variables, for [`Look::material`].
pub fn material(
    doc: &cut_craft_core::document::Document,
    units: Units,
) -> cut_craft_core::calc::Values {
    let factor = unit_mm(units);
    let job = &doc.job;
    cut_craft_core::calc::Values {
        width: job.width / factor,
        height: job.height / factor,
        thickness: job.thickness / factor,
        variables: doc.variables.clone(),
    }
}

#[derive(Debug, PartialEq)]
pub enum Kind {
    Number {
        unit: Option<String>,
        min: Option<f64>,
        max: Option<f64>,
        integer: bool,
        /// `x-steps`: only these values, as radio buttons side by side.
        steps: Vec<f64>,
        /// `x-widget: slider` (with a minimum and a maximum): a slider along the range.
        slider: bool,
        /// `x-percent-when`: a length that is a percentage instead while another choice says
        /// so (Set Size's Percent), shown as `%` and never converted then.
        percent_when: Option<When>,
    },
    Bool,
    /// A pattern's seed (`x-widget: seed`): its number, and a New variation button that picks
    /// another.
    Seed,
    /// A pattern's Sequence (`x-widget: sequence`): its entries typed in, and the pattern
    /// maker's sample buttons that add them ([`Look::pattern`]).
    Sequence,
    /// A string; `multiline` (`x-widget: multiline`) is a box of several lines.
    Text {
        multiline: bool,
    },
    Choice(Vec<Choice>, Widget),
    /// A nested object, e.g. a point.
    Group(Vec<Field>),
    /// An `[r, g, b]` array marked `x-kind: colour`.
    Colour,
    /// A material's name marked `x-kind: material`: a drop-down of the Materials Library
    /// ([`Look::materials`]) by class, or none.
    Material,
    /// A list whose items all have this shape.
    List(Box<Kind>),
    /// Anything the form cannot edit yet; shown read-only.
    Other,
}

/// How a choice is drawn (`x-widget`); the forms use each of these.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Widget {
    /// Radio buttons, one under another, for a few options; a drop-down for more.
    #[default]
    Auto,
    /// Radio buttons side by side (`row`).
    Row,
    /// A drop-down with the chosen option's description under it (`drop-down`).
    DropDown,
    /// A square with a spot at each corner and the centre, as the XY datum picker (`corners`).
    Corners,
    /// A radio button for each face of a model round its picture (`faces`, with `x-icon`),
    /// as the Initial Orientation.
    Faces { picture: Option<String> },
    /// A row of picture buttons, one an option, the chosen one pressed (`icons`, with
    /// `x-icon` naming each picture by the value, `{}`), like Combine Mode; with
    /// `named-icons` larger, each with its name under it.
    Icons {
        picture: Option<String>,
        named: bool,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct Choice {
    pub value: String,
    pub label: String,
    pub help: Option<String>,
    /// Only offered while another choice says so (the variant's `x-show-when` /
    /// `x-hide-when`), judged on the params round the field.
    pub when: Option<When>,
}

/// The `options` offered with `around` as it is: those whose rule holds.
pub fn offered(options: &[Choice], around: &Value) -> Vec<Choice> {
    options
        .iter()
        .filter(|o| o.when.as_ref().is_none_or(|w| w.shown(around)))
        .cloned()
        .collect()
}

/// Edits `params` through its fields; true when anything changed.
pub fn show(ui: &mut egui::Ui, fields: &[Field], params: &mut Value, look: &Look) -> bool {
    let top = params.clone();
    let all: Vec<&Field> = fields.iter().collect();
    show_within(ui, &all, params, look, &top)
}

/// Edits `params` through some of its fields (a tab's, [`form_tabs::on_tab`]); true when
/// anything changed. A setting shown only while another says so reads all of `params`.
pub fn show_some(ui: &mut egui::Ui, fields: &[&Field], params: &mut Value, look: &Look) -> bool {
    let top = params.clone();
    show_within(ui, fields, params, look, &top)
}

/// The Sequence box, and under it the pattern maker: a turn to add with, a sample button for
/// each digit drawn with the form's settings, Undo, Clear and Random.
#[expect(
    clippy::too_many_lines,
    reason = "the box, then the pattern maker, in order"
)]
fn sequence(ui: &mut egui::Ui, value: &mut Value, params: &Value, look: &Look) -> bool {
    let mut text = value.as_str().unwrap_or_default().to_owned();
    let changed = ui
        .vertical(|ui| {
            let mut changed = ui
                .add(
                    egui::TextEdit::singleline(&mut text)
                        .hint_text(tr!("form-empty-1-over-and-over"))
                        .desired_width(ui.available_width().min(260.0)),
                )
                .changed();
            let Some(maker) = &look.pattern else {
                return changed;
            };
            let id = ui.scope_id().with("pattern_maker");
            let mut with: AddWith = ui.data(|d| d.get_temp(id)).unwrap_or_default();
            ui.horizontal(|ui| {
                ui.label(tr!("form-turn-180"))
                    .on_hover_hint(tr!("form-added-samples-are-turned-by"));
                let turns = Number::new(&mut with.div).name(tr!("form-turn-180"));
                ui.add(turns.range(0..=999));
                ui.weak(turn_name(with.div));
                ui.checkbox(&mut with.rev, tr!("form-other-way-r"))
                    .on_hover_hint(tr!("form-turn-the-other-way-with"));
            });
            ui.horizontal(|ui| {
                ui.checkbox(&mut with.size, tr!("form-random-size-s"))
                    .on_hover_hint(tr!("form-each-time-this-entry-comes"));
                ui.checkbox(&mut with.lean, tr!("form-random-lean-t"))
                    .on_hover_hint(tr!("form-each-time-this-entry-comes-2"));
            });
            let sample = |ui: &mut egui::Ui, code: u8| {
                let entry = entry_text(code, with);
                let hint = tr!(
                    "form-sample-add",
                    entry = entry.clone(),
                    what = tr!(SAMPLES[usize::from(code)])
                );
                sample_button(ui, maker, params, &entry, code)
                    .on_hover_hint(hint)
                    .clicked()
                    .then_some(entry)
            };
            // the samples in quadrants: left to right flips LR (1 to 3), top to bottom flips
            // UD (1 to 2); the space beside them
            let mut add = None;
            ui.horizontal(|ui| {
                egui::Grid::new(id.with("quadrants"))
                    .spacing(egui::vec2(2.0, 2.0))
                    .show(ui, |ui| {
                        for row in [[1, 3], [2, 4]] {
                            for code in row {
                                if let Some(e) = sample(ui, code) {
                                    add = Some(e);
                                }
                            }
                            ui.end_row();
                        }
                    });
                if let Some(e) = sample(ui, 0) {
                    add = Some(e);
                }
            });
            if let Some(entry) = add {
                let mut list = entries(&text);
                list.push(entry);
                text = join_entries(&list);
                changed = true;
            }
            ui.horizontal(|ui| {
                if ui
                    .button(tr!("form-undo"))
                    .on_hover_hint(tr!("form-take-off-the-last-entry"))
                    .clicked()
                {
                    let mut list = entries(&text);
                    list.pop();
                    text = join_entries(&list);
                    changed = true;
                }
                if ui
                    .button(tr!("form-clear"))
                    .on_hover_hint(tr!("form-empty-the-sequence-1-over"))
                    .clicked()
                {
                    text.clear();
                    changed = true;
                }
                if ui
                    .button(tr!("form-random"))
                    .on_hover_hint(tr!("form-make-up-a-sequence-that"))
                    .clicked()
                {
                    let frame = ui.ctx().cumulative_frame_nr() % u64::from(u32::MAX);
                    let seed = u32::try_from(frame).unwrap_or(7);
                    text = (maker.random)(seed);
                    changed = true;
                }
            });
            ui.data_mut(|d| d.insert_temp(id, with));
            changed
        })
        .inner;
    if changed {
        *value = Value::String(text);
    }
    changed
}

/// The pieces a sample draws, kept while the form's settings stay the same.
type Outlines = std::sync::Arc<SampleRings>;

/// A button with a picture of the pattern drawn as `entry`, its digit under it.
#[expect(
    clippy::cast_possible_truncation,
    reason = "millimetres scaled to a picture a few dozen points across"
)]
fn sample_button(
    ui: &mut egui::Ui,
    maker: &PatternMaker,
    params: &Value,
    entry: &str,
    code: u8,
) -> egui::Response {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    params.to_string().hash(&mut h);
    entry.hash(&mut h);
    let key = h.finish();
    let slot = egui::Id::unique(("pattern_sample", code));
    let rings: Outlines = match ui.data(|d| d.get_temp::<(u64, Outlines)>(slot)) {
        Some((k, rings)) if k == key => rings,
        _ => {
            let rings = Outlines::new((maker.sample)(params, entry));
            ui.data_mut(|d| d.insert_temp(slot, (key, rings.clone())));
            rings
        }
    };
    let size = egui::vec2(72.0, 52.0);
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
    let visuals = ui.style().interact(&response);
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 4.0, visuals.bg_fill);
    painter.rect_stroke(rect, 4.0, visuals.bg_stroke, egui::StrokeKind::Inside);
    let label_h = 14.0;
    let pic = egui::Rect::from_min_max(
        rect.min + egui::vec2(4.0, 4.0),
        rect.max - egui::vec2(4.0, 4.0 + label_h),
    );
    let (mut lo, mut hi) = ([f64::INFINITY; 2], [f64::NEG_INFINITY; 2]);
    for q in rings.iter().flatten() {
        for i in 0..2 {
            lo[i] = lo[i].min(q[i]);
            hi[i] = hi[i].max(q[i]);
        }
    }
    if lo[0] < hi[0] {
        let span = [(hi[0] - lo[0]).max(1e-6), (hi[1] - lo[1]).max(1e-6)];
        let k = (f64::from(pic.width()) / span[0]).min(f64::from(pic.height()) / span[1]);
        let c = pic.center();
        let mid = [f64::midpoint(lo[0], hi[0]), f64::midpoint(lo[1], hi[1])];
        let stroke = egui::Stroke::new(1.0, visuals.fg_stroke.color);
        for ring in rings.iter() {
            let pts: Vec<egui::Pos2> = ring
                .iter()
                .map(|q| {
                    egui::pos2(
                        c.x + ((q[0] - mid[0]) * k) as f32,
                        // across the line is up the picture
                        c.y - ((q[1] - mid[1]) * k) as f32,
                    )
                })
                .collect();
            painter.add(egui::Shape::closed_line(pts, stroke));
        }
    }
    painter.text(
        egui::pos2(rect.center().x, rect.max.y - 2.0),
        egui::Align2::CENTER_BOTTOM,
        entry,
        egui::FontId::proportional(12.0),
        visuals.text_color(),
    );
    response
}

/// A slider along `range` for a number with no length unit (a percentage, a 1 to 10
/// setting).
fn slide(
    ui: &mut egui::Ui,
    name: &str,
    range: std::ops::RangeInclusive<f64>,
    integer: bool,
    factor: f64,
    unit: Option<&str>,
    value: &mut Value,
) -> bool {
    // a length slides in the units shown, `factor` millimetres each; its value is the
    // number box beside it, with its up and down buttons
    let mut n = value.as_f64().unwrap_or_default() / factor;
    let shown = range.start() / factor..=range.end() / factor;
    let changed = ui
        .horizontal(|ui| {
            let mut slider = egui::Slider::new(&mut n, shown.clone()).show_value(false);
            if integer && factor == 1.0 {
                slider = slider.integer();
            }
            let slid = ui.add(slider).accessible_name(name).changed();
            let mut number = Number::new(&mut n).range(shown).name(name);
            if integer && factor == 1.0 {
                number = number.fixed_decimals(0);
            }
            if let Some(unit) = unit {
                number = number.suffix(format!(" {unit}"));
            }
            slid | ui.add(number).changed()
        })
        .inner;
    if changed {
        *value = if integer {
            Value::from(round(n * factor))
        } else {
            Value::from(n * factor)
        };
    }
    changed
}

/// The seed after `seed`: well away from it, the same every time, so a variation can be
/// found again by stepping from the first.
pub fn next_seed(seed: u32) -> u32 {
    let mut random = cut_craft_core::random::Random(u64::from(seed));
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "a unit times u32::MAX fits u32"
    )]
    let next = (random.unit() * f64::from(u32::MAX)) as u32;
    if next == seed {
        next.wrapping_add(1)
    } else {
        next
    }
}

/// The Materials Library's materials in a drop-down, class by class and softest first,
/// with (none) for leaving it unnamed; without the library, the name as text.
fn pick_material(
    ui: &mut egui::Ui,
    value: &mut Value,
    library: Option<&cut_craft_core::material_library::MaterialLibrary>,
) -> bool {
    use cut_craft_core::material_class::MaterialClass;
    let mut name = value.as_str().unwrap_or_default().to_owned();
    let Some(library) = library else {
        let changed = ui.text_edit_singleline(&mut name).changed();
        if changed {
            *value = Value::String(name);
        }
        return changed;
    };
    let shown = if name.is_empty() {
        NO_MATERIAL
    } else {
        name.as_str()
    };
    let before = name.clone();
    egui::ComboBox::from_id_salt(ui.next_auto_id())
        .selected_text(shown)
        .show_ui(ui, |ui| {
            ui.selectable_value(&mut name, String::new(), NO_MATERIAL);
            for class in MaterialClass::ALL {
                let materials = library.in_class(class);
                if materials.is_empty() {
                    continue;
                }
                ui.separator();
                ui.weak(class.name());
                for m in materials {
                    ui.selectable_value(&mut name, m.name.clone(), &m.name);
                }
            }
        });
    let changed = name != before;
    if changed {
        *value = Value::String(name);
    }
    changed
}

/// How an unnamed material shows in its drop-down.
pub const NO_MATERIAL: &str = "(none)";

/// A new item of `kind` with nothing set: numbers at their least, the first choice, every
/// field of an object so.
fn blank(kind: &Kind) -> Value {
    match kind {
        Kind::Number { min, .. } => Value::from(min.unwrap_or(0.0).max(0.0)),
        Kind::Seed => Value::from(1),
        Kind::Bool => Value::Bool(false),
        Kind::Text { .. } | Kind::Material | Kind::Sequence => Value::from(""),
        Kind::Choice(options, _) => options
            .first()
            .map_or(Value::Null, |c| Value::from(c.value.as_str())),
        Kind::Group(fields) => Value::Object(
            fields
                .iter()
                .map(|f| (f.key.clone(), blank(&f.kind)))
                .collect(),
        ),
        Kind::List(_) => Value::Array(vec![]),
        Kind::Colour | Kind::Other => Value::Null,
    }
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "integer params are small counts"
)]
fn round(n: f64) -> i64 {
    n.round() as i64
}

/// The heading of the group a toolpath's depths go in; [`arrange`] puts it first.
pub const CUTTING_DEPTHS: &str = "Cutting Depths";

/// The group that picks which side of the vectors a profile cuts, shown just after the cutting
/// depths rather than lower down, under Machine Vectors.
pub const WHICH_SIDE: &str = cmd_profile_toolpath::SIDE_GROUP;

/// The Drag Knife's own settings (blade offset, tolerance angle): how the blade cuts, so they
/// come right under its depths, as Profile's side does, not at the foot of the form.
pub const KNIFE: &str = "Knife";

/// Not a setting: where a toolpath form's Rest Machining, vector selector and locked faces
/// go, under the depths and the side to cut, as [`arrange`] puts it. [`Look::tool`] draws
/// them, given this key.
pub const AFTER_TOOL: &str = "after_tool";

/// The bits the form's toolpath cuts with: the clearing bits before its own and 3D
/// Finishing's list, which [`arrange`] puts right under its Bit block.
const CUTTING_BITS: [&str; 2] = ["clearance_tools", "tools"];

/// Whether a form picks a bit in the Bit Library: a toolpath's, put in the standard order
/// by [`arrange`].
pub fn picks_bits(fields: &[Field]) -> bool {
    fields
        .iter()
        .any(|f| f.key == "tool" || CUTTING_BITS.contains(&f.key.as_str()))
}

/// A toolpath form's fields in the standard order: the bit first (what cuts sets how deep a
/// pass goes and how far a V reaches), the bits that clear before it, the cutting depths and
/// the side to cut, then [`AFTER_TOOL`], then the rest as declared with each group's (and the
/// tab strip's) fields gathered where the first of them is, and Project onto 3D Model and
/// the name last.
pub fn arrange(fields: Vec<Field>) -> Vec<Field> {
    let rank = |f: &Field| match f.key.as_str() {
        "tool" => 0,
        key if CUTTING_BITS.contains(&key) => 1,
        // Surfacing's What to Flatten, then its depth: what is cut comes before how deep
        _ if f.group.as_deref() == Some(cmd_surfacing_toolpath::AREA_GROUP) => 2,
        _ if f.group.as_deref() == Some(CUTTING_DEPTHS) => 2,
        // Profile's Outside / Inside / On, right under the depth where a beginner looks
        _ if f.group.as_deref() == Some(WHICH_SIDE) => 3,
        _ if f.group.as_deref() == Some(KNIFE) => 3,
        AFTER_TOOL => 4,
        "project_onto_model" => 6,
        "name" => 7,
        _ => 5,
    };
    let mut fields = fields;
    if fields.iter().any(|f| f.key == "tool") {
        fields.push(after_tool());
    }
    // where each field's company first appears: its group's or the tab strip's first field,
    // else its own place
    let mut firsts: Vec<((Option<String>, bool), usize)> = vec![];
    let mut keyed: Vec<(u8, usize, usize, Field)> = fields
        .into_iter()
        .enumerate()
        .map(|(i, f)| {
            let company = (f.group.clone(), f.tab.is_some());
            let at = if company == (None, false) {
                i
            } else if let Some(&(_, first)) = firsts.iter().find(|(c, _)| *c == company) {
                first
            } else {
                firsts.push((company, i));
                i
            };
            (rank(&f), at, i, f)
        })
        .collect();
    keyed.sort_by_key(|&(rank, at, i, _)| (rank, at, i));
    keyed.into_iter().map(|(.., f)| f).collect()
}

/// The [`AFTER_TOOL`] place.
fn after_tool() -> Field {
    Field {
        key: AFTER_TOOL.into(),
        label: String::new(),
        help: None,
        kind: Kind::Group(vec![]),
        section: false,
        icon: None,
        when: None,
        enabled: None,
        group: None,
        tab: None,
        dim: None,
        diagram: None,
        axis: None,
        titles: None,
    }
}

/// The fields of a params schema, in declaration order, their words in the language in use.
pub fn fields(schema: &Value) -> Vec<Field> {
    let mut fields = english_fields(schema);
    fields.iter_mut().for_each(translate);
    fields
}

/// The fields of a params schema with the code's English words. A group's, a tab's and a
/// section's name stay English in [`fields`] too, as the code finds them by it (a section's
/// end, such as Model Size's Scale to Fit); they are translated where they are drawn.
pub fn english_fields(schema: &Value) -> Vec<Field> {
    object_fields(schema, schema)
}

/// A field's name, tooltip, other names and choices in the language in use.
fn translate(f: &mut Field) {
    if !f.section {
        f.label = setting_text(&f.label);
    }
    f.help = f.help.as_deref().map(setting_text);
    if let Some(t) = &mut f.titles {
        for (_, title) in &mut t.by {
            *title = setting_text(title);
        }
    }
    if !is_font(f) {
        translate_kind(&mut f.kind);
    }
}

/// A font choice (Draw Text's, Auto Layout Text's): its choices are the fonts installed, each
/// named as it is installed in every language, so they are neither translated nor in Fluent.
pub fn is_font(f: &Field) -> bool {
    f.key == "font"
}

fn translate_kind(kind: &mut Kind) {
    match kind {
        Kind::Choice(options, _) => {
            for o in options {
                o.label = setting_text(&o.label);
                o.help = o.help.as_deref().map(setting_text);
            }
        }
        Kind::Group(inner) => inner.iter_mut().for_each(translate),
        Kind::List(item) => translate_kind(item),
        _ => {}
    }
}

fn object_fields(root: &Value, schema: &Value) -> Vec<Field> {
    let Some(props) = resolve(root, schema)["properties"].as_object() else {
        return vec![];
    };
    props
        .iter()
        .map(|(key, s)| Field {
            key: key.clone(),
            label: s["title"]
                .as_str()
                .map_or_else(|| sentence_case(key), String::from),
            help: s["description"]
                .as_str()
                .or_else(|| resolve(root, s)["description"].as_str())
                .map(String::from),
            kind: kind(root, s),
            section: s.get("x-section") == Some(&Value::Bool(true))
                || resolve(root, s).get("x-section") == Some(&Value::Bool(true)),
            icon: s["x-icon"]
                .as_str()
                .or_else(|| resolve(root, s)["x-icon"].as_str())
                .map(String::from),
            when: When::from_schema(s),
            enabled: s
                .get("x-enable-when")
                .and_then(|rule| When::from_rule(rule, true)),
            group: s["x-group"].as_str().map(String::from),
            tab: s["x-tab"].as_str().map(String::from),
            dim: s["x-dim"].as_str().map(String::from),
            diagram: s["x-diagram"]
                .as_str()
                .or_else(|| resolve(root, s)["x-diagram"].as_str())
                .map(String::from),
            axis: s["x-axis"].as_str().map_or_else(
                || {
                    matches!(kind(root, s), Kind::Number { .. })
                        .then(|| crate::axes::Axis::named(key))
                        .flatten()
                },
                crate::axes::Axis::named,
            ),
            titles: Titles::from_schema(s),
        })
        .collect()
}

/// Follows a local `$ref` into `$defs`, where schemars puts shared types.
fn resolve<'a>(root: &'a Value, s: &'a Value) -> &'a Value {
    match s["$ref"].as_str().and_then(|r| r.strip_prefix("#/$defs/")) {
        Some(name) => &root["$defs"][name],
        None => s,
    }
}

fn kind(root: &Value, property: &Value) -> Kind {
    let s = resolve(root, property);
    // marks may sit on the property or on the shared type it refers to
    let marked = |key: &str| property.get(key).or_else(|| s.get(key));
    match marked("x-kind").and_then(Value::as_str) {
        Some("colour") => return Kind::Colour,
        Some("material") => return Kind::Material,
        _ => {}
    }
    if let Some(options) = choices(s) {
        let widget = match marked("x-widget").and_then(Value::as_str) {
            Some("row") => Widget::Row,
            Some("drop-down") => Widget::DropDown,
            Some("corners") => Widget::Corners,
            Some("faces") => Widget::Faces {
                picture: marked("x-icon").and_then(Value::as_str).map(String::from),
            },
            Some(w @ ("icons" | "named-icons")) => Widget::Icons {
                picture: marked("x-icon").and_then(Value::as_str).map(String::from),
                named: w == "named-icons",
            },
            _ => Widget::Auto,
        };
        return Kind::Choice(options, widget);
    }
    match s["type"].as_str() {
        Some("number" | "integer")
            if marked("x-widget").and_then(Value::as_str) == Some("seed") =>
        {
            Kind::Seed
        }
        Some(t @ ("number" | "integer")) => Kind::Number {
            unit: marked("x-unit").and_then(Value::as_str).map(String::from),
            min: s["minimum"].as_f64(),
            max: s["maximum"].as_f64(),
            integer: t == "integer",
            steps: marked("x-steps")
                .and_then(Value::as_array)
                .map_or_default(|steps| steps.iter().filter_map(Value::as_f64).collect()),
            slider: marked("x-widget").and_then(Value::as_str) == Some("slider"),
            percent_when: marked("x-percent-when").and_then(|rule| When::from_rule(rule, true)),
        },
        Some("boolean") => Kind::Bool,
        Some("string") if marked("x-widget").and_then(Value::as_str) == Some("sequence") => {
            Kind::Sequence
        }
        Some("string") => Kind::Text {
            multiline: marked("x-widget").and_then(Value::as_str) == Some("multiline"),
        },
        Some("object") => Kind::Group(object_fields(root, s)),
        Some("array") => {
            let mut item = kind(root, &s["items"]);
            // a list of lengths marks its unit on the list (Slice's thicknesses)
            if let Kind::Number {
                unit: unit @ None, ..
            } = &mut item
            {
                *unit = marked("x-unit").and_then(Value::as_str).map(String::from);
            }
            Kind::List(Box::new(item))
        }
        _ => Kind::Other,
    }
}

/// String enums: schemars writes `enum: [...]`, or `oneOf` of single values when variants are
/// documented; a variant's `title` is its label and its doc comment its hover help.
fn choices(s: &Value) -> Option<Vec<Choice>> {
    let choice = |value: &str, about: Option<&Value>| Choice {
        value: value.into(),
        label: about
            .and_then(|o| o["title"].as_str())
            .map_or_else(|| sentence_case(value), String::from),
        help: about
            .and_then(|o| o["description"].as_str())
            .map(String::from),
        when: about.and_then(When::from_schema),
    };
    match s["enum"].as_array() {
        Some(values) => values
            .iter()
            .map(|v| Some(choice(v.as_str()?, None)))
            .collect(),
        None => s["oneOf"]
            .as_array()?
            .iter()
            .map(|o| {
                let value = o["const"].as_str().or_else(|| o["enum"][0].as_str())?;
                Some(choice(value, Some(o)))
            })
            .collect(),
    }
}

/// A form's text in the language in use, found by its English words (a setting's label, its
/// tooltip, a choice, a group's or a tab's heading): the forms are made from the commands'
/// own descriptions, so these have no names of their own. `locales/en/settings.ftl` holds
/// every one (a test checks), under [`cut_craft_i18n::text_key`].
pub fn setting_text(english: &str) -> String {
    cut_craft_i18n::tr_or(&setting_key(english), english)
}

/// The key of a form's text: `setting-`, its first words and a check of all of it.
pub fn setting_key(english: &str) -> String {
    cut_craft_i18n::text_key("setting", english)
}

fn sentence_case(key: &str) -> String {
    let words = key.replace('_', " ");
    let mut chars = words.chars();
    chars
        .next()
        .map_or_default(|c| c.to_uppercase().chain(chars).collect())
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod words;

#[cfg(test)]
#[test]
#[ignore = "prints the form blocks for the picture audit"]
fn print_form_blocks() {
    use std::fmt::Write as _;
    let registry = crate::commands::registry();
    let mut out = String::new();
    for c in registry.iter() {
        let fields = fields(&c.schema());
        let fields = if picks_bits(&fields) {
            arrange(fields)
        } else {
            fields
        };
        let mut seen: Vec<String> = vec![];
        let mut row = |kind: &str, name: &str, icon: Option<&String>| {
            let key = format!("{kind}:{name}");
            if !seen.contains(&key) {
                seen.push(key);
                let _ = writeln!(
                    out,
                    "{}\t{}\t{}\t{}\t{}",
                    c.meta().group,
                    c.meta().label,
                    kind,
                    name,
                    icon.map_or("-", |s| s.as_str())
                );
            }
        };
        if fields.is_empty() {
            continue;
        }
        for f in &fields {
            if f.section {
                row("section", &f.label, f.icon.as_ref());
            } else if let Some(g) = &f.group {
                row("group", g, None);
            } else if let Some(t) = &f.tab {
                row("tab", t, None);
            } else if f.key == "tool" {
                row("tool", "Bit", None);
            } else if f.key != AFTER_TOOL {
                row("loose", "(fields outside a block)", None);
            }
        }
    }
    std::fs::write(std::env::var("BLOCKS_OUT").unwrap(), out).unwrap();
}

#[cfg(test)]
#[test]
fn interactive_rotation_offers_a_named_picture_for_each_turn() {
    let schema = crate::commands::registry()
        .get("orientate_3d_model")
        .unwrap()
        .schema();
    let f = fields(&schema);
    let turn = f.iter().find(|f| f.key == "turn").unwrap();
    let Kind::Group(inner) = &turn.kind else {
        panic!("a section")
    };
    let mode = inner.iter().find(|f| f.key == "mode").unwrap();
    let Kind::Choice(
        options,
        Widget::Icons {
            picture: Some(p),
            named: true,
        },
    ) = &mode.kind
    else {
        panic!("{:?}", mode.kind)
    };
    for o in options {
        let path = icon_path(p, &Value::String(o.value.clone()));
        let file = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../assets/icons")
            .join(&path);
        assert!(file.is_file(), "{path}");
    }
}

#[test]
fn every_way_of_cutting_has_a_picture_for_each_choice() {
    let registry = crate::commands::registry();
    let icons = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/icons");
    // Profile's and Pocket's are in their Cutting window, drawn there as one picture
    let wanted = [
        ("chamfer_toolpath", &["cut_direction", "ways"][..]),
        ("prism_toolpath", &["cut_direction", "ways"]),
        ("moulding_toolpath", &["cut_direction", "ways"]),
    ];
    for (id, keys) in wanted {
        let f = fields(&registry.get(id).unwrap().schema());
        for key in keys {
            let field = f.iter().find(|f| f.key == *key).expect(key);
            let Kind::Choice(
                options,
                Widget::Icons {
                    picture: Some(p),
                    named: true,
                },
            ) = &field.kind
            else {
                panic!("{id} {key}: {:?}", field.kind)
            };
            // every side a profile machines
            for side in ["outside", "inside", "on"] {
                let around = serde_json::json!({ "side": side });
                for o in options {
                    let path = icon_path(&named(p, &around), &Value::String(o.value.clone()));
                    assert!(icons.join(&path).is_file(), "{id} {key}: no {path}");
                }
            }
        }
    }
    // a profile's climb turns clockwise round a part and anticlockwise in a hole
    let path = |side: &str| {
        named(
            "toolpaths/cut_{}_{side}.png",
            &serde_json::json!({ "side": side }),
        )
    };
    assert_eq!(
        icon_path(&path("inside"), &"climb".into()),
        "toolpaths/cut_climb_inside.png"
    );
    assert_eq!(
        icon_path(&path("outside"), &"climb".into()),
        "toolpaths/cut_climb_outside.png"
    );
}

#[cfg(test)]
mod sequence_tests {
    use super::*;

    #[test]
    fn the_pattern_maker_adds_entries_and_takes_the_last_off() {
        assert_eq!(entries("1102"), ["1", "1", "0", "2"]);
        assert_eq!(entries("1,R1/4, S2"), ["1", "R1/4", "S2"]);
        assert_eq!(join_entries(&entries("1102")), "1102");
        let mut list = entries("12");
        list.push(entry_text(
            1,
            AddWith {
                div: 4,
                rev: true,
                ..AddWith::default()
            },
        ));
        assert_eq!(join_entries(&list), "1,2,R1/4");
        let with = AddWith {
            size: true,
            lean: true,
            ..AddWith::default()
        };
        assert_eq!(entry_text(3, with), "ST3");
        assert_eq!(entry_text(0, with), "0", "a space is only a space");
        let mut list = entries("1,2,R1/4");
        list.pop();
        assert_eq!(join_entries(&list), "12");
    }

    #[test]
    fn a_sequence_field_is_the_pattern_maker() {
        let schema = serde_json::json!({
            "type": "object",
            "properties": { "sequence": { "type": "string", "x-widget": "sequence" } }
        });
        let fields = fields(&schema);
        assert_eq!(fields[0].kind, Kind::Sequence);
    }
}
