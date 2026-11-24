//! A toolpath form's Tool, Clearing, Selector and Rest sections.

use super::{
    Align, HoverHint, Layout, Tool, ToolDatabase, Units, Value, egui, form, passes, reach,
    surfacing, tabs, tr,
};

/// What a toolpath form's Tool section asks for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ToolAsk {
    /// Select…: the Bit Library to pick the tool in, opened on this one.
    Select,
    /// Edit…: the Bit Library to change this tool in.
    Edit,
}

/// What a drag knife form says of its tool, in place of a bit's size and pass: a knife
/// has no flutes; it presses into the top by a share of its spring travel.
pub(crate) const KNIFE_LINE: &str = "Drag knife: zero Z with the blade tip touching the \
                                     material. It presses in by the Pressure, a share of the \
                                     Spring Travel, so no flute length is needed";

/// A toolpath form's tool, kept short: its name (with its tool changer holder), its size and
/// pass in one line, Select… to pick it in the Bit Library (where the tool changer's
/// holders are too, and every setting) and Edit… to change it there. What was asked for.
/// `laser`: the form is a laser toolpath's, whose tool is a laser.
pub(super) fn tool_picker(
    ui: &mut egui::Ui,
    id: &str,
    laser: bool,
    top: &Value,
    tools: &ToolDatabase,
    slots: &[Option<String>],
    units: cut_craft_core::document::Units,
) -> Option<ToolAsk> {
    let tool = &top["tool"];
    let current = tool["name"].as_str().unwrap_or_default();
    let picked = serde_json::from_value::<Tool>(tool.clone())
        .ok()
        .filter(|_| !current.is_empty());
    match &picked {
        Some(tool) => {
            // The Passes: how many the cut depth takes, as set in Edit Passes
            #[expect(clippy::cast_precision_loss, reason = "a small count")]
            let passes = passes::cut_depth(top)
                .filter(|_| tool.pass_depth > 0.0)
                .map(|depth| tool.pass_depths(depth).len() as f64);
            ui.strong(held_name(current, slots));
            if id == cut_craft_core::drag_knife::STRATEGY {
                ui.label(KNIFE_LINE);
            } else if laser && tool.is_laser() {
                // a laser from before lasers were a tool type reads as the laser it is
                let laser = Tool {
                    shape: cut_craft_core::toolpath::ToolShape::Laser,
                    ..tool.clone()
                };
                ui.label(tool_line(&laser, None, units));
            } else {
                ui.label(tool_line(tool, passes, units));
            }
            if surfacing::wants_surfacing_bit(top, tool) {
                ui.weak(surfacing::NOT_A_SURFACING_BIT);
            }
            if tools.get(current).is_none() {
                ui.colored_label(
                    ui.visuals().warn_fg_color,
                    "Not in the Bit Library: Select… one, or add it there",
                );
            }
        }
        None => {
            ui.colored_label(ui.visuals().warn_fg_color, "No bit picked yet: Select… one");
        }
    }
    ui.horizontal(|ui| {
        let select = ui.button(tr!("tool-select"));
        crate::marks::note(ui, "button:Select…", select.rect);
        let select = select
            .on_hover_hint(tr!(if laser {
                "tool-select.tooltip-laser"
            } else {
                "tool-select.tooltip"
            }))
            .clicked();
        let edit = ui
            .button(tr!("tool-edit"))
            .on_hover_hint(tr!("tool-edit.tooltip"))
            .clicked();
        if select {
            Some(ToolAsk::Select)
        } else {
            edit.then_some(ToolAsk::Edit)
        }
    })
    .inner
}

/// Which tool of a list a pick is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ListSlot {
    /// One more.
    New,
    /// In place of this one.
    At(usize),
}

/// Whether a toolpath form shows Rest Machining: every toolpath that cuts with a tool can
/// rest machine after the tools that cut before it, except 3D Finishing (its own, as `after`)
/// and a form with clearing bits, which does that with them (an older job's list still
/// shows, so it can be changed).
pub(super) fn shows_rest(params: &Value) -> bool {
    params.get("after").is_none()
        && (params.get("clearance_tools").is_none()
            || cut_craft_core::rest::Rest::of(params).is_some())
}

/// What a form's list of tools asks for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum ListAsk {
    /// Select Bit, for a tool of the list.
    Select(ListSlot),
    /// The Bit Library to change this tool in.
    Edit(String),
    /// Suggest: the quickest clearing bits from the user's own tools.
    Suggest,
}

/// A form's lists of tools, each picked in Select Bit as the form's own tool is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ToolList {
    /// The clearing bits (`clearance_tools`), largest first.
    Clearing,
    /// The form's tools, in the order they cut (3D Finishing's `tools`).
    Tools,
    /// Rest Machining's earlier tools (`rest.after`).
    Rest,
    /// The bits that finished before a 3D Pencil toolpath (`bits_before`).
    Before,
}

impl ToolList {
    /// The list a form's `key` holds, for a list drawn by [`tools_section`].
    pub(super) fn of_key(key: &str) -> Self {
        match key {
            "bits_before" => Self::Before,
            _ => Self::Tools,
        }
    }
}

/// What Select Bit picks for, other than the open form's own tool.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum PickFor {
    /// A tool of one of the open form's lists.
    Listed(ToolList, ListSlot),
    /// The active machine's tool changer holder (counted from 1).
    Holder(usize),
    /// The Materials Library's Add Tool: the bit gets feeds for its material or class.
    Material,
}

/// A form's clearing bits (`clearance_tools`): off, the form's tool cuts it all; on, bigger
/// bits cut first, largest first, each picked with Select… in the Bit Library, and the
/// form's tool cuts only what they left. `finishing_smallest` (Pocket) wants each bigger than
/// the form's tool; V-Carve's V-bit follows end mills of any size. Whether the list changed,
/// and what was asked for.
pub(super) fn clearing_section(
    ui: &mut egui::Ui,
    params: &mut Value,
    slots: &[Option<String>],
    finishing_smallest: bool,
    suggested: Option<&str>,
    units: cut_craft_core::document::Units,
) -> (bool, Option<ListAsk>) {
    if !params["clearance_tools"].is_array() {
        params["clearance_tools"] = Value::Array(vec![]);
    }
    let finishing: Option<Tool> = serde_json::from_value(params["tool"].clone()).ok();
    let mut list: Vec<Tool> =
        serde_json::from_value(params["clearance_tools"].clone()).unwrap_or_default();
    let (mut changed, mut ask) = (false, None);
    ui.group(|ui| {
        ui.set_width(ui.available_width());
        let mut on = !list.is_empty();
        let (label, hint, about) = if finishing_smallest {
            (
                "Clear with bigger bits first",
                "Bigger bits take out most of the material first, so the Tool above only \
                 cuts what they could not reach: quicker, and easier on a small bit",
                "Biggest first. The Tool above finishes: it cuts only what these left.",
            )
        } else {
            (
                "Clear the flat bottom with end mills first",
                "End mills clear the flat bottom first, so the V-bit only carves the slopes \
                 and what they could not reach",
                "Biggest first. The V-bit finishes: it carves only what these left.",
            )
        };
        if ui.checkbox(&mut on, label).on_hover_hint(hint).changed() {
            if on {
                ask = Some(ListAsk::Select(ListSlot::New));
            } else {
                list.clear();
                changed = true;
            }
        }
        // Pocket: the quickest bits from the user's own, tool changer bits first
        if finishing_smallest
            && finishing.is_some()
            && ui
                .button(tr!("clearing-suggest"))
                .on_hover_hint(tr!("clearing-suggest.tooltip"))
                .clicked()
        {
            ask = Some(ListAsk::Suggest);
        }
        if let Some(note) = suggested {
            ui.weak(note);
        }
        if list.is_empty() {
            return;
        }
        ui.weak(about);
        let depth = params["depth"].as_f64().filter(|d| *d > 0.0);
        let mut remove = None;
        for (i, tool) in list.iter().enumerate() {
            match ui
                .push_id(("clearing", i), |ui| {
                    clearing_bit(ui, i, tool, slots, depth, units)
                })
                .inner
            {
                Some(BitAsk::List(asked)) => ask = Some(asked),
                Some(BitAsk::Remove) => remove = Some(i),
                None => {}
            }
        }
        if let Some(i) = remove {
            list.remove(i);
            changed = true;
        }
        if ui
            .button(tr!("clearing-add-smaller"))
            .on_hover_hint(tr!("clearing-add-smaller.tooltip"))
            .clicked()
        {
            ask = Some(ListAsk::Select(ListSlot::New));
        }
        if let Some(finishing) = &finishing
            && let Err(e) =
                cut_craft_core::rest::check_clearing(finishing, &list, finishing_smallest)
        {
            ui.colored_label(ui.visuals().error_fg_color, e.to_string());
        }
    });
    if changed {
        params["clearance_tools"] = serde_json::to_value(&list).expect("tools serialize");
    }
    (changed, ask)
}

/// What a clearing bit's own buttons ask for.
pub(super) enum BitAsk {
    List(ListAsk),
    Remove,
}

/// The `i`th clearing bit: its name (with its holder), one line, whether it reaches `depth`,
/// and Select…, Edit… and Remove.
pub(super) fn clearing_bit(
    ui: &mut egui::Ui,
    i: usize,
    tool: &Tool,
    slots: &[Option<String>],
    depth: Option<f64>,
    units: cut_craft_core::document::Units,
) -> Option<BitAsk> {
    ui.strong(held_name(&tool.name, slots));
    ui.label(tool_line(tool, None, units));
    // can this bit reach the depth? the same line the form's own Tool has
    if let Some(line) = cut_craft_core::reach::form_line(tool, depth, None, units) {
        reach::show(ui, &line);
    }
    ui.horizontal(|ui| {
        if ui
            .button(tr!("tool-select"))
            .on_hover_hint(tr!("tool-select.tooltip-another-bit"))
            .clicked()
        {
            return Some(BitAsk::List(ListAsk::Select(ListSlot::At(i))));
        }
        if ui
            .button(tr!("tool-edit"))
            .on_hover_hint(tr!("tool-edit.tooltip-bit"))
            .clicked()
        {
            return Some(BitAsk::List(ListAsk::Edit(tool.name.clone())));
        }
        ui.small_button("×")
            .on_hover_hint(tr!("clearing-remove-bit"))
            .clicked()
            .then_some(BitAsk::Remove)
    })
    .inner
}

/// `params`' list `which` with `tool` put `at` its slot: clearing bits kept largest first,
/// the others in the order they were picked.
pub(super) fn put_listed(params: &mut Value, which: ToolList, at: ListSlot, tool: &Tool) {
    let list = match which {
        ToolList::Clearing => &mut params["clearance_tools"],
        ToolList::Tools => &mut params["tools"],
        ToolList::Before => &mut params["bits_before"],
        ToolList::Rest => {
            if !params["rest"].is_object() {
                params["rest"] = serde_json::to_value(cut_craft_core::rest::Rest::default())
                    .expect("rest settings serialise");
            }
            &mut params["rest"]["after"]
        }
    };
    let mut tools: Vec<Tool> = serde_json::from_value(list.clone()).unwrap_or_default();
    match at {
        ListSlot::At(i) if i < tools.len() => tools[i] = tool.clone(),
        _ => tools.push(tool.clone()),
    }
    if which == ToolList::Clearing {
        tools.sort_by(|a, b| b.diameter.total_cmp(&a.diameter));
    }
    *list = serde_json::to_value(&tools).expect("tools serialize");
}

/// The form tool's name as the tool changer has it: `T3  name` when loaded in holder 3.
pub(super) fn held_name(name: &str, slots: &[Option<String>]) -> String {
    match slots.iter().position(|s| s.as_deref() == Some(name)) {
        Some(i) => format!("T{}  {name}", i + 1),
        None => name.to_owned(),
    }
}

/// A tool in one line for its form: how wide, its angle or tip if it has one, how deep a
/// pass, and how many passes the cut takes (`passes`, when known). Lengths in the job's
/// `units`: a tool keeps its own units in the Bit Library, but a toolpath shows it in the
/// job's.
pub(super) fn tool_line(
    tool: &Tool,
    passes: Option<f64>,
    units: cut_craft_core::document::Units,
) -> String {
    use cut_craft_core::toolpath::ToolShape;
    let length = |mm: f64| format!("{} {}", trimmed(mm / units.per()), units.suffix());
    let across = length(tool.diameter);
    // a laser burns: its kerf and how fast it moves, no pass
    if tool.shape == ToolShape::Laser {
        return format!(
            "Laser, {across} kerf, {} {}/min",
            trimmed(tool.feed_rate / units.per()),
            units.suffix()
        );
    }
    let line = match tool.shape {
        ToolShape::EndMill => format!("{across} end mill"),
        ToolShape::BallNose => format!("{across} ball nose"),
        ToolShape::VBit => format!("{}° V-bit, {across} across", trimmed(tool.angle)),
        ToolShape::RadiusedEndMill => {
            format!("{across} end mill, {} tip radius", length(tool.tip_radius))
        }
        ToolShape::ThreadMill => format!("{across} thread mill"),
        ToolShape::FormTool => format!("{across} form bit"),
        ToolShape::SurfacingBit => format!("{across} surfacing bit"),
        ToolShape::Laser => unreachable!("said above"),
    };
    let pass = length(tool.pass_depth);
    let line = format!("{line}, {pass} a pass");
    match passes {
        Some(passes) => {
            let s = if (passes - 1.0).abs() < f64::EPSILON {
                ""
            } else {
                "es"
            };
            format!("{line}, {} pass{s}", trimmed(passes))
        }
        None => line,
    }
}

/// A list of tools (a set of finishing tools, rest machining's earlier tools), each as the
/// form's own tool is shown: its name (with its tool changer holder) and size in one line,
/// Select… to pick another in the Bit Library, Edit… to change it there, and × to remove
/// it; Add a tool… picks one more. `empty` is said while there are none. Whether it changed
/// (a removal), and what was asked for.
pub(super) fn tool_list(
    ui: &mut egui::Ui,
    key: &str,
    list: &mut Value,
    tools: &ToolDatabase,
    slots: &[Option<String>],
    empty: &str,
    units: cut_craft_core::document::Units,
) -> (bool, Option<ListAsk>) {
    let mut items: Vec<Tool> = serde_json::from_value(list.clone()).unwrap_or_default();
    let (mut ask, mut remove) = (None, None);
    if items.is_empty() {
        ui.weak(empty);
    }
    for (i, tool) in items.iter().enumerate() {
        ui.push_id((key, i), |ui| {
            ui.strong(held_name(&tool.name, slots));
            ui.label(tool_line(tool, None, units));
            if tools.get(&tool.name).is_none() {
                ui.colored_label(
                    ui.visuals().warn_fg_color,
                    "Not in the Bit Library: Select… one, or add it there",
                );
            }
            ui.horizontal(|ui| {
                if ui
                    .button(tr!("tool-select"))
                    .on_hover_hint(tr!("tool-select.tooltip-another"))
                    .clicked()
                {
                    ask = Some(ListAsk::Select(ListSlot::At(i)));
                }
                if ui
                    .button(tr!("tool-edit"))
                    .on_hover_hint(tr!("tool-edit.tooltip"))
                    .clicked()
                {
                    ask = Some(ListAsk::Edit(tool.name.clone()));
                }
                if ui
                    .small_button("×")
                    .on_hover_hint(tr!("tool-list-remove"))
                    .clicked()
                {
                    remove = Some(i);
                }
            });
        });
    }
    if ui
        .push_id((key, "add"), |ui| {
            ui.button(tr!("tool-list-add"))
                .on_hover_hint(tr!("tool-list-add.tooltip"))
        })
        .inner
        .clicked()
    {
        ask = Some(ListAsk::Select(ListSlot::New));
    }
    let changed = remove.is_some();
    if let Some(i) = remove {
        items.remove(i);
        *list = serde_json::to_value(&items).expect("tools serialize");
    }
    (changed, ask)
}

/// A form's own list of tools (3D Finishing's `tools`, in the order they cut), under its
/// heading: see [`tool_list`].
pub(super) fn tools_section(
    ui: &mut egui::Ui,
    key: &str,
    params: &mut Value,
    tools: &ToolDatabase,
    slots: &[Option<String>],
    units: cut_craft_core::document::Units,
) -> (bool, Option<ListAsk>) {
    ui.group(|ui| {
        ui.set_width(ui.available_width());
        let (heading, empty) = match ToolList::of_key(key) {
            ToolList::Before => (
                "Bits that finished before",
                "None: the pencil bit runs along every valley. Add the bit your 3D Finishing \
                 toolpath used, to cut only what it left",
            ),
            _ => ("Bits", "No bit picked yet: Add a bit… to pick one"),
        };
        ui.strong(heading);
        tool_list(ui, key, &mut params[key], tools, slots, empty, units)
    })
    .inner
}

/// `value` to three places at most, without trailing zeros.
pub(super) fn trimmed(value: f64) -> String {
    let text = format!("{value:.3}");
    text.trim_end_matches('0').trim_end_matches('.').to_owned()
}

/// The Rest Machining section of a toolpath form: the tools that cut before this one
/// (picked from the database, each removable), and, once there are any, the minimum
/// detail and overlap. Kept as `rest` in the form's settings.
/// A toolpath form's Vector Selector: associated, the toolpath takes the vectors that pass
/// it (open, closed, on which layers) each time it is calculated, as templates need.
pub(super) fn selector_section(
    ui: &mut egui::Ui,
    params: &mut Value,
    layer_names: &[String],
    units: Units,
) {
    use cut_craft_core::selector::{Circles, ClosedKind, Layers, Selector};
    let before = Selector::of(params);
    let mut on = before.is_some();
    let mut selector = before.clone().unwrap_or_default();
    ui.group(|ui| {
        ui.set_width(ui.available_width());
        ui.checkbox(&mut on, tr!("selector-associate"))
            .on_hover_hint(tr!("selector-associate.tooltip"));
        if !on {
            return;
        }
        ui.checkbox(&mut selector.open_vectors, tr!("selector-open-vectors"));
        ui.checkbox(&mut selector.closed_vectors, tr!("selector-closed-vectors"));
        // which closed vectors: all, only circles, or only circles of a size
        ui.add_enabled_ui(selector.closed_vectors, |ui| {
            ui.indent("closed_kind", |ui| {
                ui.radio_value(
                    &mut selector.closed_kind,
                    ClosedKind::AllClosedVectors,
                    tr!("selector-all-closed"),
                );
                ui.radio_value(
                    &mut selector.closed_kind,
                    ClosedKind::OnlyCircles,
                    tr!("selector-only-circles"),
                )
                .on_hover_hint(tr!("selector-only-circles.tooltip"));
                if selector.closed_kind != ClosedKind::OnlyCircles {
                    return;
                }
                ui.indent("circles", |ui| {
                    ui.radio_value(
                        &mut selector.circles,
                        Circles::SelectAllCircles,
                        tr!("selector-all-circles"),
                    );
                    ui.radio_value(
                        &mut selector.circles,
                        Circles::SelectCirclesMatching,
                        tr!("selector-circles-of-diameter"),
                    );
                    if selector.circles == Circles::SelectCirclesMatching {
                        egui::Grid::new("circle_size")
                            .num_columns(2)
                            .show(ui, |ui| {
                                ui.label(tr!("selector-diameter"));
                                tabs::length(
                                    ui,
                                    &tr!("selector-diameter"),
                                    &mut selector.diameter,
                                    units,
                                );
                                ui.end_row();
                                ui.label(tr!("selector-tolerance"))
                                    .on_hover_hint(tr!("selector-tolerance.tooltip"));
                                tabs::length(
                                    ui,
                                    &tr!("selector-tolerance"),
                                    &mut selector.tolerance,
                                    units,
                                );
                                ui.end_row();
                            });
                    }
                });
            });
        });
        for (layers, label) in [
            (Layers::AllVisibleLayers, "All visible layers"),
            (Layers::CurrentLayerOnly, "Current layer only"),
            (Layers::SelectedLayers, "Selected layers"),
        ] {
            ui.radio_value(&mut selector.layers, layers, label);
        }
        if selector.layers == Layers::SelectedLayers {
            for name in layer_names {
                let mut ticked = selector.layer_names.contains(name);
                if ui.checkbox(&mut ticked, name).changed() {
                    if ticked {
                        selector.layer_names.push(name.clone());
                    } else {
                        selector.layer_names.retain(|n| n != name);
                    }
                }
            }
        }
    });
    let after = on.then_some(selector);
    if after != before {
        match after {
            Some(selector) => {
                params["selector"] =
                    serde_json::to_value(&selector).expect("selector settings serialise");
            }
            None => {
                if let Some(map) = params.as_object_mut() {
                    map.remove("selector");
                }
            }
        }
    }
}

pub(super) fn rest_section(
    ui: &mut egui::Ui,
    params: &mut Value,
    tools: &ToolDatabase,
    slots: &[Option<String>],
    units: cut_craft_core::document::Units,
) -> Option<ListAsk> {
    use cut_craft_core::rest::Rest;
    let mut rest: Rest = params
        .get("rest")
        .and_then(|r| serde_json::from_value(r.clone()).ok())
        .unwrap_or_default();
    let before = rest.clone();
    let mut ask = None;
    ui.group(|ui| {
        ui.set_width(ui.available_width());
        ui.strong(tr!("rest-title"))
            .on_hover_hint(tr!("rest-title.tooltip"));
        let mut after = serde_json::to_value(&rest.after).expect("tools serialize");
        let (changed, asked) = tool_list(
            ui,
            "rest_after",
            &mut after,
            tools,
            slots,
            &tr!("rest-off"),
            units,
        );
        ask = asked;
        if changed && let Ok(list) = serde_json::from_value(after) {
            rest.after = list;
        }
        if !rest.after.is_empty() {
            egui::Grid::new("rest_settings")
                .num_columns(2)
                .show(ui, |ui| {
                    ui.label(tr!("rest-minimum-detail"))
                        .on_hover_hint(tr!("rest-minimum-detail.tooltip"));
                    form::length_edit(
                        ui,
                        &tr!("rest-minimum-detail"),
                        &mut rest.minimum_detail,
                        units,
                        0.0,
                    );
                    ui.end_row();
                    ui.label(tr!("rest-overlap"))
                        .on_hover_hint(tr!("rest-overlap.tooltip"));
                    form::length_edit(ui, &tr!("rest-overlap"), &mut rest.overlap, units, 0.0);
                    ui.end_row();
                });
        }
    });
    if rest != before {
        params["rest"] = serde_json::to_value(&rest).expect("rest settings serialise");
    }
    ask
}

/// A category of a drop-down in a border filling its width, as menus draw theirs.
pub(super) fn bordered(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.set_min_width(ui.available_width());
        ui.with_layout(Layout::top_down_justified(Align::LEFT), add);
    });
}
