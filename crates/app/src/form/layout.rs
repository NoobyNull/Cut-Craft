//! Laying a form out: its tabs, plain rows, grids, diagram and hover help.

use super::{
    DIAGRAM_ASPECT, DIAGRAM_ROWS, DIM_COLOUR, FIELD_PICTURE_ROWS, Field, HoverHint, Kind, Look,
    Number, TOOL_LISTS, Units, Value, Widget, edit, egui, round, tr,
};

/// [`show`] for `params` somewhere inside the form, whose top-level parameters are `top`.
pub(super) fn show_within(
    ui: &mut egui::Ui,
    fields: &[&Field],
    params: &mut Value,
    look: &Look,
    top: &Value,
) -> bool {
    let shown: Vec<&Field> = fields
        .iter()
        .copied()
        // a section's own settings decide first, as for greying ([`super::When::judged`]):
        // a setting shown while its neighbour is ticked follows that neighbour
        .filter(|f| f.when.as_ref().is_none_or(|w| w.judged(params, top)))
        .collect();
    let mut changed = false;
    // neighbours in one tab strip, or one group, make a block; the rest stand as they are
    let mut rest = shown.as_slice();
    while let Some(first) = rest.first() {
        let together = |f: &&&Field| match (&first.tab, &f.tab) {
            (Some(_), Some(_)) => true,
            (None, None) => first.group.is_some() && f.group == first.group,
            _ => false,
        };
        let n = rest.iter().take_while(together).count().max(1);
        let (block, after) = rest.split_at(n);
        changed |= if first.tab.is_some() {
            tabs(ui, block, params, look, top)
        } else if let Some(group) = &first.group {
            ui.group(|ui| {
                ui.set_width(ui.available_width());
                ui.strong(super::setting_text(group));
                diagram(ui, block, look);
                let mut changed = plain(ui, block, params, look, top);
                if let Some(end) = look.group_end {
                    changed |= end(ui, group, params);
                }
                changed
            })
            .inner
        } else {
            diagram(ui, block, look);
            plain(ui, block, params, look, top)
        };
        rest = after;
    }
    changed
}

/// The measurements picture of the first of `fields` that has one (`x-diagram`), as wide as
/// there is room for, up to [`DIAGRAM_ROWS`] tall, at [`DIAGRAM_ASPECT`].
pub(super) fn diagram(ui: &mut egui::Ui, fields: &[&Field], look: &Look) {
    let Some(picture) = fields
        .iter()
        .find_map(|f| f.diagram.as_deref())
        .and_then(|path| (look.picture)(path))
    else {
        return;
    };
    let tallest = ui.spacing().interact_size.y * DIAGRAM_ROWS;
    let width = ui.available_width().min(tallest * DIAGRAM_ASPECT);
    ui.add(picture.fit_to_exact_size(egui::vec2(width, width / DIAGRAM_ASPECT)));
}

/// A measurement's callout letter, as the picture above it marks it.
pub(super) fn dim_letter(ui: &mut egui::Ui, dim: &str) {
    ui.label(egui::RichText::new(dim).strong().color(DIM_COLOUR))
        .on_hover_hint(tr!("form-marked-on-picture", dim = dim));
}

/// A strip of tabs over `fields`, the picked tab's fields under it; the pick is kept with
/// the form's other state in egui's memory.
pub(super) fn tabs(
    ui: &mut egui::Ui,
    fields: &[&Field],
    params: &mut Value,
    look: &Look,
    top: &Value,
) -> bool {
    let mut names: Vec<&str> = vec![];
    for f in fields {
        let name = f.tab.as_deref().unwrap_or_default();
        if !names.contains(&name) {
            names.push(name);
        }
    }
    let id = ui.scope_id().with(("form_tabs", &fields[0].key));
    let mut picked: usize = ui.data(|d| d.get_temp(id)).unwrap_or_default();
    ui.horizontal_wrapped(|ui| {
        for (i, name) in names.iter().enumerate() {
            ui.selectable_value(&mut picked, i, super::setting_text(name));
        }
    });
    ui.data_mut(|d| d.insert_temp(id, picked));
    let name = names.get(picked).copied().unwrap_or_default();
    let shown: Vec<&Field> = fields
        .iter()
        .copied()
        .filter(|f| f.tab.as_deref() == Some(name))
        .collect();
    ui.group(|ui| {
        ui.set_width(ui.available_width());
        plain(ui, &shown, params, look, top)
    })
    .inner
}

/// Fields as they stand: runs of ordinary fields share a grid; each section is a framed
/// block of its own, and the tool draws itself.
pub(super) fn plain(
    ui: &mut egui::Ui,
    fields: &[&Field],
    params: &mut Value,
    look: &Look,
    top: &Value,
) -> bool {
    let mut changed = false;
    let own_block =
        |f: &&Field| f.section || f.key == "tool" || TOOL_LISTS.contains(&f.key.as_str());
    let mut rest = fields;
    while !rest.is_empty() {
        let run = rest.iter().position(own_block).unwrap_or(rest.len());
        let (plain, after) = rest.split_at(run);
        if !plain.is_empty() {
            changed |= grid(ui, plain, params, look, top);
        }
        let Some((section, after)) = after.split_first() else {
            break;
        };
        rest = after;
        if (section.key == "tool" || TOOL_LISTS.contains(&section.key.as_str())) && !section.section
        {
            if let Some(tool) = look.tool {
                let place = ui.scope(|ui| tool(ui, &section.key, params)).response.rect;
                crate::marks::note(ui, &format!("field:{}", section.key), place);
            }
            continue;
        }
        let enabled = section.enabled.as_ref().is_none_or(|w| w.shown(top));
        let shown = ui.add_enabled_ui(enabled, |ui| {
            ui.group(|ui| {
                ui.set_width(ui.available_width());
                hover(ui.strong(&section.label), section.help.as_ref());
                let value = &mut params[&section.key];
                let picture = section
                    .icon
                    .as_ref()
                    .and_then(|template| (look.picture)(&icon_path(template, value)));
                let edited = ui
                    .horizontal(|ui| {
                        if let Some(picture) = picture {
                            ui.add(picture);
                        }
                        ui.vertical(|ui| match &section.kind {
                            Kind::Group(inner) => {
                                let inner: Vec<&Field> = inner.iter().collect();
                                show_within(ui, &inner, value, look, top)
                            }
                            kind => edit(ui, &section.label, kind, value, &Value::Null, look),
                        })
                        .inner
                    })
                    .inner;
                // more under the section's settings (Model Position in Material's
                // picture), given the settings holding the section
                let ended = look
                    .group_end
                    .is_some_and(|end| end(ui, &section.label, params));
                edited | ended
            })
            .inner
        });
        crate::marks::note(ui, &format!("field:{}", section.key), shown.response.rect);
        changed |= shown.inner;
    }
    changed
}

pub(super) fn grid(
    ui: &mut egui::Ui,
    fields: &[&Field],
    params: &mut Value,
    look: &Look,
    top: &Value,
) -> bool {
    let mut changed = false;
    egui::Grid::new(ui.next_auto_id())
        .num_columns(2)
        .show(ui, |ui| {
            for f in fields {
                // the faces and icons widgets draw their pictures themselves
                let own = !matches!(
                    f.kind,
                    Kind::Choice(_, Widget::Faces { .. } | Widget::Icons { .. })
                );
                let picture = f
                    .icon
                    .as_ref()
                    .filter(|_| own)
                    .and_then(|template| (look.picture)(&icon_path(template, &params[&f.key])));
                let enabled = f.enabled.as_ref().is_none_or(|w| w.judged(params, top));
                let label_rect = ui
                    .add_enabled_ui(enabled, |ui| {
                        ui.horizontal(|ui| {
                            if let Some(picture) = picture {
                                let edge = ui.spacing().interact_size.y * FIELD_PICTURE_ROWS;
                                ui.add(picture.fit_to_exact_size(egui::vec2(edge, edge)));
                            }
                            if let Some(dim) = &f.dim {
                                dim_letter(ui, dim);
                            }
                            let name = f.label_for(params, top);
                            let label = match f.axis {
                                Some(axis) => ui.label(crate::axes::label(name, axis)),
                                None => ui.label(name),
                            };
                            hover(label, f.help.as_ref());
                        });
                    })
                    .response
                    .rect;
                // a picture named by the fields beside it reads them as they were this frame
                let around = match &f.kind {
                    Kind::Choice(
                        _,
                        Widget::Icons {
                            picture: Some(t), ..
                        },
                    ) if t.replace("{}", "").contains('{') => params.clone(),
                    // the pattern maker draws its samples with the form's settings
                    Kind::Sequence => params.clone(),
                    // a length that turns into a percentage reads the tick beside it
                    Kind::Number {
                        percent_when: Some(w),
                        ..
                    } => serde_json::json!({ &w.key: params[&w.key] }),
                    // a choice offered only while another says so reads the form's settings,
                    // its own section's first
                    Kind::Choice(options, _) if options.iter().any(|o| o.when.is_some()) => {
                        within(params, top)
                    }
                    _ => Value::Null,
                };
                let edited = ui.add_enabled_ui(enabled, |ui| {
                    edit(ui, &f.label, &f.kind, &mut params[&f.key], &around, look)
                });
                if let Some(help) = &f.help
                    && edited.response.contains_pointer()
                {
                    note_help(ui.ctx(), help);
                }
                changed |= edited.inner;
                let place = label_rect.union(edited.response.rect);
                crate::marks::note(ui, &format!("field:{}", f.key), place);
                ui.end_row();
            }
        });
    changed
}

/// The form's top-level settings `top` with those of the section being drawn (`local`) laid
/// over them, so a rule reads a setting as [`When::judged`] does.
pub(super) fn within(local: &Value, top: &Value) -> Value {
    let mut all = top.clone();
    if let (Some(all), Some(local)) = (all.as_object_mut(), local.as_object()) {
        all.extend(local.iter().map(|(k, v)| (k.clone(), v.clone())));
    }
    all
}

pub(super) fn hover(label: egui::Response, help: Option<&String>) {
    if let Some(help) = help {
        if label.contains_pointer() {
            note_help(&label.ctx, help);
        }
        label.on_hover_hint(help);
    }
}

pub(super) fn help_id() -> egui::Id {
    egui::Id::unique("form_pointed_at_help")
}

/// Keeps what the setting under the pointer does, for a form that shows it in a box of its
/// own ([`pointed_at_help`]).
pub fn note_help(ctx: &egui::Context, help: &str) {
    ctx.data_mut(|d| d.insert_temp(help_id(), help.to_owned()));
}

/// Forgets the setting last pointed at: a form calls it before drawing its settings, so the
/// box shows nothing once the pointer is off them.
pub fn forget_help(ctx: &egui::Context) {
    ctx.data_mut(|d| d.remove::<String>(help_id()));
}

/// What the setting under the pointer does, as the form last drew it.
pub fn pointed_at_help(ctx: &egui::Context) -> Option<String> {
    ctx.data(|d| d.get_temp::<String>(help_id()))
}

/// A number box for a length kept in millimetres, shown and typed in `units` (`15 mm` reads
/// `0.5906 in` in an inch job, never `15 in`), at least `min`. Written back only when typed or
/// dragged, so switching units and back never changes it. `name` is what a screen reader
/// calls it: its label's words.
pub fn length_edit(
    ui: &mut egui::Ui,
    name: &str,
    mm: &mut f64,
    units: Units,
    min: f64,
) -> egui::Response {
    let per = units.per();
    let mut shown = *mm / per;
    let response = ui.add(
        Number::new(&mut shown)
            .range(min / per..=f64::INFINITY)
            .speed(0.1 / per)
            .suffix(format!(" {}", units.suffix()))
            .name(name),
    );
    if response.changed() {
        *mm = shown * per;
    }
    response
}

/// How a stored unit shows in `units`: the factor to divide by and the unit to show.
pub fn shown_in(unit: &str, units: Units) -> Option<(f64, &'static str)> {
    match (unit, units) {
        ("mm", Units::Inch) => Some((25.4, "in")),
        ("mm/min", Units::Inch) => Some((25.4, "in/min")),
        ("µm", Units::Inch) => Some((25_400.0, "in")),
        _ => None,
    }
}

/// One arrow click's step and how much a one-point drag changes a number shown in `unit`
/// (`integer`: a whole number), kept to `range`: the unit's own step, but small enough that a
/// short range (a few micrometres shown in inches) takes several clicks, and a drag a tenth of
/// a step a point (a whole number at least one), so an inch job never moves inches for a nudge of the mouse.
pub(super) fn feel(
    unit: Option<&str>,
    integer: bool,
    range: std::ops::RangeInclusive<f64>,
) -> (f64, f64) {
    let mut step = cut_craft_bug_report::number::step_for(unit.unwrap_or_default(), integer);
    let span = range.end() - range.start();
    if !integer && span.is_finite() && span > 0.0 && step > span / 5.0 {
        step = 10f64.powf((span / 5.0).log10().floor());
    }
    // whole numbers drag at least one a point
    let speed = if integer {
        (step / 10.0).max(1.0)
    } else {
        step / 10.0
    };
    (step, speed)
}

/// The decimal places `step` needs.
pub(super) fn decimals(step: f64) -> usize {
    let places = (-step.log10()).ceil().max(0.0);
    // at most 15 places: a whole number of them
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "0..=15"
    )]
    let places = places.min(15.0) as usize;
    places
}

/// A section's picture for its value: `{}` stands for the value, `{name}` for its field `name`.
pub fn icon_path(template: &str, value: &Value) -> String {
    named(
        &template.replace("{}", value.as_str().unwrap_or_default()),
        value,
    )
}

/// `template` with each `{name}` the text of `fields`' field `name` (a choice's picture that
/// follows another choice: Profile's cut direction drawn for the side it machines).
pub(super) fn named(template: &str, fields: &Value) -> String {
    let mut path = template.to_owned();
    if let Some(fields) = fields.as_object() {
        for (name, v) in fields {
            if let Some(v) = v.as_str() {
                path = path.replace(&format!("{{{name}}}"), v);
            }
        }
    }
    path
}

/// The units a form itself sets: Job Setup's, beside its sizes. A tool picked on a toolpath
/// form carries its own units too (a metric bit in an inch job), but they are the bit's, never
/// the form's: the form stays in the job's units.
pub fn units_chosen(params: &Value) -> Option<Units> {
    [&params["units"], &params["size"]["units"]]
        .into_iter()
        .find_map(|u| serde_json::from_value(u.clone()).ok())
}

/// Writes `n` into `value` when it `changed`, rounded for a whole number; whether it changed.
pub(super) fn store(changed: bool, value: &mut Value, n: f64, integer: bool) -> bool {
    if changed {
        *value = if integer {
            Value::from(round(n))
        } else {
            Value::from(n)
        };
    }
    changed
}
