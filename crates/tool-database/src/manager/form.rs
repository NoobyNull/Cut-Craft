//! The Tool Database's tool form: each section of a tool's settings.

use super::{
    BTreeMap, Changer, CuttingData, HoverHint, INCH, Look, NOTES_ROWS, Number, RateUnits, Tool,
    ToolShape, Units, egui, tool_type,
};

/// Which slot of the tool changer `tool` is loaded in, to change: the slot picked (0 for
/// taking it out), if one was.
pub(super) fn load_slot(ui: &mut egui::Ui, changer: &Changer, tool: &str) -> Option<usize> {
    let now = changer.slot_of(tool);
    let shown = now.map_or_else(|| "(not loaded)".to_owned(), |n| format!("T{n}"));
    let mut chosen = None;
    ui.horizontal(|ui| {
        ui.label(cut_craft_i18n::tr!(
            "tool-database-loaded-in",
            machine = changer.machine.clone()
        ));
        egui::ComboBox::from_id_salt("tool_changer_slot")
            .selected_text(shown)
            .show_ui(ui, |ui| {
                if ui
                    .selectable_label(now.is_none(), cut_craft_i18n::t!("(not loaded)"))
                    .clicked()
                {
                    chosen = Some(0);
                }
                for (i, slot) in changer.slots.iter().enumerate() {
                    let holding = slot
                        .as_deref()
                        .map_or_else(|| "empty".to_owned(), str::to_owned);
                    if ui
                        .selectable_label(now == Some(i + 1), format!("T{} ({holding})", i + 1))
                        .clicked()
                    {
                        chosen = Some(i + 1);
                    }
                }
            });
    });
    ui.weak(cut_craft_i18n::t!(
        "The slot it is loaded in becomes its tool number"
    ));
    chosen
}

/// A length (mm) shown in the tool's units; whether it changed.
/// `name` is what a screen reader calls the box: its label's words.
pub(crate) fn length(ui: &mut egui::Ui, name: &str, value: &mut f64, units: Units) -> bool {
    let (per, unit) = if units == Units::Inch {
        (INCH, "in")
    } else {
        (1.0, "mm")
    };
    number(ui, name, value, per, unit)
}

/// A rate (mm/min) shown in the tool's rate units; whether it changed.
pub(crate) fn rate(ui: &mut egui::Ui, name: &str, value: &mut f64, units: RateUnits) -> bool {
    number(ui, name, value, units.mm_per_min(), units.label())
}

/// `value` shown as so many of `per`, with its unit; whether it changed.
pub(super) fn number(ui: &mut egui::Ui, name: &str, value: &mut f64, per: f64, unit: &str) -> bool {
    let mut shown = *value / per;
    // ponytail: drag speed is a hundredth of the value, so small and large tools both drag well
    let speed = (shown.abs() / 100.0).max(f64::EPSILON);
    let changed = ui
        .add(
            Number::new(&mut shown)
                .speed(speed)
                .range(0.0..=f64::MAX)
                .max_decimals(4)
                .suffix(format!(" {unit}"))
                .name(name),
        )
        .changed();
    if changed {
        *value = shown * per;
    }
    changed
}

/// A framed block with a heading (Tool Info, Cutting Parameters, Feeds and
/// Speeds).
pub(super) fn section(ui: &mut egui::Ui, title: &str, body: impl FnOnce(&mut egui::Ui)) {
    ui.group(|ui| {
        ui.set_width(ui.available_width());
        ui.strong(title);
        body(ui);
    });
}

pub(super) fn tool_details(
    ui: &mut egui::Ui,
    tool: &mut Tool,
    (groups, formats): (&[String], &mut BTreeMap<String, String>),
    (look, loaded, taken): (&Look, Option<(&str, usize)>, &[String]),
    material: &str,
    library: Option<&cut_craft_core::material_library::MaterialLibrary>,
) {
    // the drawing in a card of its own right of Tool Info, so it does not push the cutting
    // data down; under it when the window is too narrow for both
    let gap = ui.spacing().item_spacing.x;
    if let Some((info, card)) = split(ui.available_width(), gap) {
        ui.horizontal_top(|ui| {
            ui.vertical(|ui| {
                ui.set_width(info);
                section(ui, "Tool Info", |ui| {
                    tool_info(ui, tool, (groups, formats), look, (loaded, taken));
                });
            });
            ui.vertical(|ui| {
                ui.set_width(card);
                section(ui, "Tool Shape", |ui| geometry(ui, tool));
            });
        });
    } else {
        section(ui, "Tool Info", |ui| {
            tool_info(ui, tool, (groups, formats), look, (loaded, taken));
        });
        section(ui, "Tool Shape", |ui| geometry(ui, tool));
    }
    // the cutting data shown is the active material's
    let own = tool
        .materials
        .iter()
        .any(|m| m.material.eq_ignore_ascii_case(material));
    if !material.is_empty() && !own {
        ui.weak(cut_craft_i18n::tr!(
            "tool-database-no-data-yet",
            material = material.to_string()
        ));
    }
    copy_settings_from(ui, tool, material);
    let mut shown = match library {
        Some(library) => tool.for_material_in(material, library),
        None => tool.for_material(material),
    };
    let before = shown.cutting("");
    section(ui, "Cutting Parameters", |ui| cutting(ui, &mut shown));
    section(ui, "Feeds and Speeds", |ui| feeds(ui, &mut shown));
    keep_cutting(tool, &shown, material, &before);
    tool.renamed(material);
}

/// The Copy Settings From: the tool's cutting data for another material (or its
/// default) copied as its data for `material`, where it has any to copy.
pub(super) fn copy_settings_from(ui: &mut egui::Ui, tool: &mut Tool, material: &str) {
    let others: Vec<String> = std::iter::once(String::new())
        .chain(tool.materials.iter().map(|m| m.material.clone()))
        .filter(|m| !m.eq_ignore_ascii_case(material))
        .collect();
    if others.is_empty() {
        return;
    }
    ui.horizontal(|ui| {
        ui.label(cut_craft_i18n::t!("Copy settings from"));
        egui::ComboBox::from_id_salt("copy_cutting_from")
            .selected_text("another material…")
            .show_ui(ui, |ui| {
                for from in &others {
                    let label = if from.is_empty() { "(default)" } else { from };
                    if ui.selectable_label(false, label).clicked() {
                        tool.copy_cutting(from, material);
                    }
                }
            });
    });
}

/// Keeps what was edited in `shown` (the tool as it cuts `material`) as the tool's data for
/// that material, or its default for none; rate units are the tool's whatever the material.
pub(super) fn keep_cutting(tool: &mut Tool, shown: &Tool, material: &str, before: &CuttingData) {
    tool.rate_units = shown.rate_units;
    // only a change made here becomes the material's own data: data shown from its class
    // or the default stays borrowed while it is only looked at
    if shown.cutting("") != *before {
        tool.set_cutting(CuttingData {
            material: material.to_owned(),
            ..shown.cutting("")
        });
    }
}

/// The tool's name format (Name Format): `{Variable|Modifier}` filled in from the
/// tool, a variable picked from the list put on the end; Set as Default makes it its type's
/// for new tools, Restore puts the type's back. A tool with a format is named by it.
pub(super) fn name_format(
    ui: &mut egui::Ui,
    tool: &mut Tool,
    formats: &mut BTreeMap<String, String>,
) {
    use cut_craft_core::tool_names::type_key;
    let key = type_key(tool.shape);
    ui.vertical(|ui| {
        ui.add(
            egui::TextEdit::singleline(&mut tool.name_format)
                .hint_text("{Tool Type} {Diameter|F}")
                .desired_width(f32::INFINITY),
        )
        .on_hover_hint(cut_craft_i18n::t!(
            "The name made from the tool's own sizes: leave empty to type the name. \
             Modifiers: |U upper case, |L lower case, |0.00 decimals, |F inches as a \
             fraction"
        ));
        ui.horizontal(|ui| name_format_buttons(ui, tool, formats, &key));
    });
}

/// Add variable, Set as Default and Restore, under the name format.
pub(super) fn name_format_buttons(
    ui: &mut egui::Ui,
    tool: &mut Tool,
    formats: &mut BTreeMap<String, String>,
    key: &str,
) {
    use cut_craft_core::tool_names::VARIABLES;
    egui::ComboBox::from_id_salt("name_variable")
        .selected_text("Add variable")
        .show_ui(ui, |ui| {
            for v in VARIABLES {
                if ui.selectable_label(false, v).clicked() {
                    tool.name_format.extend(["{", v, "}"]);
                }
            }
        });
    if ui
        .button(cut_craft_i18n::t!("Set as Default"))
        .on_hover_hint(cut_craft_i18n::t!(
            "New tools of this type are named by this format"
        ))
        .clicked()
    {
        formats.insert(key.to_owned(), tool.name_format.clone());
    }
    if let Some(default) = formats.get(key).filter(|d| **d != tool.name_format)
        && ui
            .button(cut_craft_i18n::t!("Restore"))
            .on_hover_hint(cut_craft_i18n::tr!(
                "tool-database-back-to-default",
                default = default.clone()
            ))
            .clicked()
    {
        tool.name_format.clone_from(default);
    }
}

/// A form tool's row in Tool Info: Use Selected Vector takes its shape from the job's
/// selected open vector, and how many points its profile has.
pub(super) fn form_row(ui: &mut egui::Ui, tool: &mut Tool, look: &Look) {
    if tool.shape != ToolShape::FormTool {
        return;
    }
    ui.label(cut_craft_i18n::t!("Form"));
    ui.horizontal(|ui| {
        let use_it = ui
            .add_enabled(
                look.profile.is_some(),
                egui::Button::new(cut_craft_i18n::t!("Use Selected Vector")),
            )
            .on_hover_hint(cut_craft_i18n::t!(
                "Half the tool's section, drawn upright: the axis at its left, the \
                 tip at its lowest point"
            ))
            .on_disabled_hover_text(cut_craft_i18n::t!(
                "Select one open vector in the job first"
            ));
        if use_it.clicked()
            && let Some(path) = look.profile
        {
            tool.set_form(path);
        }
        ui.weak(if tool.form.is_empty() {
            cut_craft_i18n::t!("no profile yet")
        } else {
            cut_craft_i18n::tr!("tool-database-points", count = tool.form.len())
        });
    });
    ui.end_row();
}

/// Name, type, group and notes, then the geometry beside the tool's picture.
pub(super) fn tool_info(
    ui: &mut egui::Ui,
    tool: &mut Tool,
    (groups, formats): (&[String], &mut BTreeMap<String, String>),
    look: &Look,
    (loaded, taken): (Option<(&str, usize)>, &[String]),
) {
    egui::Grid::new("tool_info").num_columns(2).show(ui, |ui| {
        ui.label(cut_craft_i18n::t!("Name"));
        // a name made from its name format is not typed
        ui.add_enabled(
            tool.name_format.is_empty(),
            egui::TextEdit::singleline(&mut tool.name).desired_width(f32::INFINITY),
        );
        ui.end_row();
        // under the name it makes (rather than in a dialog of its own)
        ui.label(cut_craft_i18n::t!("Name Format"));
        name_format(ui, tool, formats);
        ui.end_row();
        ui.label(cut_craft_i18n::t!("Manufacturer"))
            .on_hover_hint(cut_craft_i18n::t!("Who makes the bit, for the name format's {Manufacturer}; leave it empty if you don't know"));
        ui.add(
            egui::TextEdit::singleline(&mut tool.manufacturer)
                .hint_text(cut_craft_i18n::t!("e.g. Amana, Whiteside"))
                .desired_width(f32::INFINITY),
        );
        ui.end_row();
        ui.label(cut_craft_i18n::t!("Tool Type"));
        ui.horizontal(|ui| {
            tool_type(ui, tool);
            // the tool's own units beside it, whatever the job's: a 6 mm end mill reads
            // 0.2362 in
            egui::ComboBox::from_id_salt("tool_units")
                .selected_text(if tool.units == Units::Inch {
                    "inches"
                } else {
                    "mm"
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut tool.units, Units::Mm, cut_craft_i18n::t!("mm"));
                    ui.selectable_value(&mut tool.units, Units::Inch, cut_craft_i18n::t!("inches"));
                })
                .response
                .on_hover_hint(cut_craft_i18n::t!("Units: how this tool's lengths read; the tool stays the same size"));
            ui.checkbox(&mut tool.drill_only, cut_craft_i18n::t!("Drill only")).on_hover_hint(cut_craft_i18n::t!("A drill (twist or brad-point) cuts only straight down. Any toolpath but \
                 Drilling that uses it is warned of in red, as moved sideways it can snap or \
                 tear the work"),
            );
        });
        ui.end_row();
        // stands in for dragging a tool between groups
        ui.label(cut_craft_i18n::t!("Group"));
        egui::ComboBox::from_id_salt("tool_group")
            .selected_text(if tool.group.is_empty() {
                "(top)"
            } else {
                &tool.group
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut tool.group, String::new(), cut_craft_i18n::t!("(top)"));
                for g in groups {
                    ui.selectable_value(&mut tool.group, g.clone(), g);
                }
            });
        ui.end_row();
        ui.label(cut_craft_i18n::t!("Tool Number"))
            .on_hover_hint(cut_craft_i18n::t!("The post's [T] for this tool without a tool changer, and its holder for Load by Tool Number; 0 for none"));
        // a tool in a tool changer is numbered by its slot: change the slot, not the number
        match loaded {
            Some((machine, slot)) => {
                ui.horizontal(|ui| {
                    ui.add_enabled(false, Number::new(&mut tool.number).name("Tool Number"));
                    ui.weak(cut_craft_i18n::tr!("tool-database-from-slot", slot = slot, machine = machine.to_string()));
                });
            }
            None => {
                ui.add(Number::new(&mut tool.number).range(0..=999).name("Tool Number"));
            }
        }
        ui.end_row();
        // never refused: said in red, as the checks before cutting say it
        if let Some(note) = number_taken(tool.number, taken) {
            ui.label("");
            let red = egui::RichText::new(note).color(ui.visuals().error_fg_color);
            // wrapped within the column the number box is in, never widening the window
            ui.vertical(|ui| {
                ui.set_max_width(ui.available_width().min(INFO_MIN));
                ui.add(egui::Label::new(red).wrap());
            });
            ui.end_row();
        }
        form_row(ui, tool, look);
        ui.label(cut_craft_i18n::t!("Notes"));
        ui.add(
            egui::TextEdit::multiline(&mut tool.notes)
                .desired_rows(NOTES_ROWS)
                .desired_width(f32::INFINITY),
        );
        ui.end_row();
    });
}

/// The other tools in `tools` (catalogues aside: they are not loaded) with the same tool
/// number as the `i`th, by name; none for 0, which means the bit is changed by hand.
pub fn sharing_number(tools: &[Tool], i: usize) -> Vec<String> {
    let Some(tool) = tools.get(i).filter(|t| t.number != 0) else {
        return vec![];
    };
    tools
        .iter()
        .enumerate()
        .filter(|&(j, t)| j != i && t.number == tool.number && t.name != tool.name)
        .filter(|(_, t)| !t.group.starts_with(cut_craft_core::toolpath::CATALOGUES))
        .map(|(_, t)| t.name.clone())
        .collect()
}

/// The red note under Tool Number when other tools have it: a tool changer picks bits by
/// number, so it would pick up the wrong one.
pub fn number_taken(number: u32, taken: &[String]) -> Option<String> {
    let (first, more) = taken.split_first()?;
    let also = match more.len() {
        0 => String::new(),
        1 => " and 1 more".to_owned(),
        n => format!(" and {n} more"),
    };
    Some(format!(
        "Tool number {number} is also {first}'s{also}. A tool changer would pick up the \
         wrong bit: give this one a number of its own, or 0 if you change it by hand"
    ))
}

/// The narrowest Tool Info beside the drawing's card.
pub(super) const INFO_MIN: f32 = 420.0;
/// The drawing's card: its share of the width, between these.
pub(super) const SHAPE_SHARE: f32 = 0.45;
pub(super) const SHAPE_MIN: f32 = 380.0;
pub(super) const SHAPE_MAX: f32 = 500.0;

/// Tool Info's and the drawing card's widths side by side in `width` (`gap` between),
/// or `None` when both do not fit and the card goes under Tool Info.
pub(super) fn split(width: f32, gap: f32) -> Option<(f32, f32)> {
    let card = (width * SHAPE_SHARE).clamp(SHAPE_MIN, SHAPE_MAX);
    let info = width - card - gap;
    (info >= INFO_MIN).then_some((info, card))
}

/// The tool's shape: drawn from its sizes, each typed on its dimension line, then what the
/// drawing does not show (flutes, a thread mill's tooth).
pub(super) fn geometry(ui: &mut egui::Ui, tool: &mut Tool) {
    // a laser has no bit to draw: only the width it burns
    if tool.shape == ToolShape::Laser {
        egui::Grid::new("tool_geometry")
            .num_columns(2)
            .show(ui, |ui| {
                ui.label(cut_craft_i18n::t!("Kerf"))
                    .on_hover_hint(cut_craft_i18n::t!(
                        "The width the laser burns (its diameter): Cut Outside and Cut Inside \
                     keep half of it off the line"
                    ));
                length(ui, "Kerf", &mut tool.diameter, tool.units);
                ui.end_row();
            });
        return;
    }
    crate::profile::editor(ui, tool);
    egui::Grid::new("tool_geometry")
        .num_columns(2)
        .show(ui, |ui| {
            if tool.shape == ToolShape::ThreadMill {
                thread_mill(ui, tool);
            }
            ui.label(cut_craft_i18n::t!("Number of Flutes"))
                .on_hover_hint(cut_craft_i18n::t!(
                    "How many cutting edges the bit has, for the chip load"
                ));
            ui.add(
                Number::new(&mut tool.flutes)
                    .range(1..=12)
                    .name("Number of Flutes"),
            );
            ui.end_row();
            stickout_row(ui, tool);
        });
    own_holder(ui, tool);
}

/// Stick-out: how far the bit pokes out below the collet nut, with what is taken while it
/// is not set.
pub(super) fn stickout_row(ui: &mut egui::Ui, tool: &mut Tool) {
    ui.label(cut_craft_i18n::t!("Stick-out"))
        .on_hover_hint(cut_craft_i18n::t!(
            "How far the bit pokes out below the collet nut, from its tip up to the nut's face. \
         Measure it once the bit is tightened in. On a deep cut the collet nut and spindle \
         can hit material beside the cut: the preview and Save Toolpaths warn of it. 0 for \
         not set"
        ));
    ui.horizontal(|ui| {
        length(ui, "Stickout", &mut tool.stickout, tool.units);
        let (taken, guessed) = cut_craft_core::collision::stickout(tool);
        if guessed {
            let shown = if tool.units == Units::Inch {
                format!("{:.3} in", taken / INCH)
            } else {
                format!("{taken:.1} mm")
            };
            ui.weak(cut_craft_i18n::tr!(
                "tool-database-taken-as",
                shown = shown.clone()
            ));
        }
    });
    ui.end_row();
}

/// The bit's own collet nut and spindle sizes, for a machine with Holder per bit ticked
/// (a tool changer): folded, as most bits use the machine's.
pub(super) fn own_holder(ui: &mut egui::Ui, tool: &mut Tool) {
    let summary = if tool.holder.is_some() {
        "Own holder: set"
    } else {
        "Own holder: the machine's"
    };
    egui::CollapsingHeader::new(summary)
        .id_salt("tool_own_holder")
        .show(ui, |ui| {
            ui.weak(cut_craft_i18n::t!(
                "Only for a machine with Holder per bit ticked in Machine Configuration (a \
                 tool changer, each bit in a holder of its own). Otherwise the machine's \
                 collet nut and spindle are used"
            ));
            let mut own = tool.holder.is_some();
            if ui
                .checkbox(&mut own, cut_craft_i18n::t!("This bit has its own holder"))
                .changed()
            {
                tool.holder = own.then(cut_craft_core::collision::Holder::default);
            }
            if let Some(holder) = tool.holder.as_mut() {
                egui::Grid::new("tool_own_holder_sizes")
                    .num_columns(2)
                    .show(ui, |ui| {
                        for (label, value) in [
                            ("Collet nut across", &mut holder.nut_diameter),
                            ("Collet nut height", &mut holder.nut_length),
                            ("Spindle across", &mut holder.spindle_diameter),
                        ] {
                            ui.label(label);
                            length(ui, label, value, tool.units);
                            ui.end_row();
                        }
                    });
            }
        });
}

/// A single-point thread mill's tooth: size, height (which changes the size, the angle
/// staying) and offset.
pub(super) fn thread_mill(ui: &mut egui::Ui, tool: &mut Tool) {
    ui.label(cut_craft_i18n::t!("Tooth Size"));
    length(ui, "Tooth Size", &mut tool.tooth_size, tool.units);
    ui.end_row();
    ui.label(cut_craft_i18n::t!("Tooth Height"));
    let mut height = tool.tooth_height();
    if length(ui, "Tooth Height", &mut height, tool.units) {
        tool.set_tooth_height(height);
    }
    ui.end_row();
    ui.label(cut_craft_i18n::t!("Tooth Offset"))
        .on_hover_hint(cut_craft_i18n::t!(
            "From the bottom of the tool up to the tip of the tooth"
        ));
    length(ui, "Tooth Offset", &mut tool.tooth_offset, tool.units);
    ui.end_row();
}

pub(super) fn cutting(ui: &mut egui::Ui, tool: &mut Tool) {
    if tool.shape == ToolShape::Laser {
        ui.weak(LASER_CUTTING);
        return;
    }
    egui::Grid::new("tool_cutting")
        .num_columns(2)
        .show(ui, |ui| {
            ui.label(cut_craft_i18n::t!("Pass Depth"));
            length(ui, "Pass Depth", &mut tool.pass_depth, tool.units);
            ui.end_row();
            ui.label(cut_craft_i18n::t!("Clearing Stepover"))
                .on_hover_hint(cut_craft_i18n::t!(
                    "Stepover is how far the tool moves sideways between passes, as a share of \
                 its width. Clearing (roughing) removes the bulk of the wood, in pockets, \
                 flat bottoms and 3D roughing: a big step is faster. 40% suits most end mills"
                ));
            percent(ui, "Clearing Stepover", &mut tool.stepover);
            ui.end_row();
            ui.label(cut_craft_i18n::t!("Finishing Stepover"))
                .on_hover_hint(cut_craft_i18n::t!(
                    "How far the tool moves sideways between finishing passes, which leave the \
                 surface you see: 3D finishing, V-carved edges, chamfers and mouldings. A \
                 smaller step leaves finer ridges but takes longer; 8-10% suits a ball nose"
                ));
            ui.horizontal(|ui| {
                let mut finish = tool.finishing_stepover();
                if percent(ui, "Finishing Stepover", &mut finish) {
                    tool.finish_stepover = finish;
                }
                if tool.finish_stepover <= 0.0 {
                    ui.weak(cut_craft_i18n::t!("same as clearing until changed"));
                }
            });
            ui.end_row();
        });
}

/// A stepover (a fraction of the diameter) shown as a percentage; whether it changed.
pub(crate) fn percent(ui: &mut egui::Ui, name: &str, fraction: &mut f64) -> bool {
    let mut percent = *fraction * 100.0;
    let changed = ui
        .add(
            Number::new(&mut percent)
                .range(0.0..=100.0)
                .max_decimals(2)
                .suffix(" %")
                .name(name),
        )
        .changed();
    if changed {
        *fraction = percent / 100.0;
    }
    changed
}

/// What a laser's Cutting Parameters say: it has none of a bit's.
pub const LASER_CUTTING: &str = "A laser has no pass depth or stepover: Laser Cut / Fill \
                                 sets its passes and power, Laser Picture its line spacing";

pub(super) fn feeds(ui: &mut egui::Ui, tool: &mut Tool) {
    let laser = tool.shape == ToolShape::Laser;
    egui::Grid::new("tool_feeds").num_columns(2).show(ui, |ui| {
        // a laser has no spindle or plunge: only how fast it moves
        if !laser {
            ui.label(cut_craft_i18n::t!("Spindle Speed"));
            ui.add(
                Number::new(&mut tool.spindle_speed)
                    .range(0.0..=f64::MAX)
                    .suffix(" r.p.m.")
                    .name("Spindle Speed"),
            );
            ui.end_row();
        }
        // the rate units beside the feed rate
        ui.label(if laser {
            cut_craft_i18n::t!("Speed")
        } else {
            cut_craft_i18n::t!("Feed Rate")
        });
        ui.horizontal(|ui| {
            rate(
                ui,
                if laser { "Speed" } else { "Feed Rate" },
                &mut tool.feed_rate,
                tool.rate_units,
            );
            egui::ComboBox::from_id_salt("tool_rate_units")
                .selected_text(tool.rate_units.label())
                .show_ui(ui, |ui| {
                    for units in RateUnits::ALL {
                        ui.selectable_value(&mut tool.rate_units, units, units.label());
                    }
                })
                .response
                .on_hover_hint(cut_craft_i18n::t!("Rate Units"));
        });
        ui.end_row();
        if !laser {
            ui.label(cut_craft_i18n::t!("Plunge Rate"));
            rate(ui, "Plunge Rate", &mut tool.plunge_rate, tool.rate_units);
            ui.end_row();
        }
    });
    // the feed per tooth, to check against the cutter maker's figure
    if let Some(load) = tool.chip_load() {
        let (per, unit, places) = if tool.units == Units::Inch {
            (INCH, "in", 4)
        } else {
            (1.0, "mm", 3)
        };
        ui.label(cut_craft_i18n::tr!(
            "tool-database-chip-load",
            load = format!("{:.places$} {unit}", load / per)
        ));
    }
}
