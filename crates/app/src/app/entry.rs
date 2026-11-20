//! Entry: how a toolpath goes down into the material (helix, zig-zag, ramp or straight
//! down), in a line at the foot of its cut settings and a window to change it, so the form
//! stays short.

use crate::form;
use cut_craft_bug_report::hint::HoverHint;
use cut_craft_bug_report::number::Number;
use cut_craft_core::document::Units;
use cut_craft_core::entry::{EntryMode, EntrySettings, Ramp, Ramping};
use cut_craft_i18n::tr;
use eframe::egui;
use serde_json::Value;

/// The groups an entry line goes under: Profile's and Pocket's cut settings, by a field
/// only that form has.
fn ours(group: &str, params: &Value) -> bool {
    match group {
        "Machine Vectors…" => params.get("side").is_some(),
        "Clear Pocket" => params.get("rings").is_some(),
        // V-Carve's
        form::CUTTING_DEPTHS => params.get("flat_depth").is_some(),
        _ => false,
    }
}

/// Whether a helix is offered: not on a V-Carve, where it would carve outside the groove.
fn helix_offered(params: &Value) -> bool {
    params.get("flat_depth").is_none()
}

/// The toolpath's entry: its own, or for a toolpath made before entry modes the nearest to
/// how it ramped (and whether it is that older setting).
pub(super) fn read(params: &Value) -> (EntrySettings, bool) {
    if let Ok(settings) = serde_json::from_value::<EntrySettings>(params["entry"].clone())
        && params["entry"].is_object()
    {
        return (settings, false);
    }
    let pass_depth = params["tool"]["pass_depth"].as_f64().unwrap_or(3.0);
    let (ramp, ramping) = if params.get("ramp_plunge_moves").is_some() {
        // Pocket's tick: a zig zag over the ramp distance
        let ramp = if params["ramp_plunge_moves"] == true {
            Ramp::ZigZag
        } else {
            Ramp::None
        };
        let distance = params["ramp_distance"]
            .as_f64()
            .unwrap_or(Ramping::default().distance);
        (
            ramp,
            Ramping {
                distance,
                ..Ramping::default()
            },
        )
    } else {
        (
            serde_json::from_value(params["ramp"].clone()).unwrap_or_default(),
            serde_json::from_value(params["ramping"].clone()).unwrap_or_default(),
        )
    };
    (EntrySettings::from_old(ramp, ramping, pass_depth), true)
}

/// `settings` in one line, for a bit `diameter` across taking passes `pass_depth` deep.
pub(super) fn summary(
    settings: &EntrySettings,
    diameter: f64,
    pass_depth: f64,
    units: Units,
) -> String {
    match settings.mode {
        EntryMode::Plunge => tr!("entry-summary-plunge"),
        EntryMode::Helix => tr!(
            "entry-summary-helix",
            across = units.length(diameter * settings.helix_diameter / 100.0, 1),
            angle = super::trimmed(settings.helix_angle)
        ),
        EntryMode::ZigZag => tr!(
            "entry-summary-zigzag",
            angle = super::trimmed(settings.zigzag_angle),
            passes = even(settings.zigzag_passes),
            legs = units.length(leg(settings, pass_depth), 1)
        ),
        EntryMode::Ramp => tr!(
            "entry-summary-ramp",
            distance = units.length(settings.ramp_distance, 1),
            angle = super::trimmed(ramp_angle(settings, pass_depth).round())
        ),
    }
}

/// The passes a zig-zag makes: even, so it ends where it began.
fn even(passes: u32) -> u32 {
    let passes = passes.max(2);
    passes + passes % 2
}

/// How long each zig-zag leg is, going down a full pass.
fn leg(settings: &EntrySettings, pass_depth: f64) -> f64 {
    let slope = settings.zigzag_angle.clamp(0.1, 89.0).to_radians().tan();
    pass_depth / (f64::from(even(settings.zigzag_passes)) * slope)
}

/// How steep a ramp is, going down a full pass, in degrees.
fn ramp_angle(settings: &EntrySettings, pass_depth: f64) -> f64 {
    pass_depth
        .atan2(settings.ramp_distance.max(1e-9))
        .to_degrees()
}

/// Where the window's open state is kept.
fn window_id() -> egui::Id {
    egui::Id::unique("edit_entry")
}

/// The entry line and Entry… at the foot of Profile's and Pocket's cut settings, and the
/// Entry window while it is open. Whether the settings changed.
pub(super) fn group_end(ui: &mut egui::Ui, group: &str, params: &mut Value, units: Units) -> bool {
    if !ours(group, params) {
        return false;
    }
    let (mut settings, older) = read(params);
    let diameter = params["tool"]["diameter"].as_f64().unwrap_or(6.35);
    let pass_depth = params["tool"]["pass_depth"].as_f64().unwrap_or(3.0);
    let id = window_id();
    let mut open = ui.data(|d| d.get_temp::<bool>(id)).unwrap_or(false);
    ui.horizontal_wrapped(|ui| {
        ui.label(tr!("entry-entry-2"));
        ui.label(summary(&settings, diameter, pass_depth, units));
        if older {
            ui.weak(tr!("entry-as-set-before-entry-modes"));
        }
        if ui
            .button(tr!("entry-entry-3"))
            .on_hover_hint(tr!("entry-how-the-bit-goes-down-2"))
            .clicked()
        {
            open = true;
        }
    });
    let before = settings;
    if open {
        egui::Window::new(tr!("entry-entry"))
            .id(egui::Id::unique("entry_window"))
            .open(&mut open)
            .resizable(false)
            .collapsible(false)
            .show(ui.ctx(), |ui| {
                window(
                    ui,
                    &mut settings,
                    (diameter, pass_depth),
                    helix_offered(params),
                    units,
                );
            });
    }
    ui.data_mut(|d| d.insert_temp(id, open));
    if settings != before {
        params["entry"] = serde_json::to_value(settings).expect("entry settings serialise");
        return true;
    }
    false
}

/// The window's inside: the four ways down, the chosen one's sizes, and how each place fits.
fn window(
    ui: &mut egui::Ui,
    settings: &mut EntrySettings,
    (diameter, pass_depth): (f64, f64),
    helix: bool,
    units: Units,
) {
    ui.label(tr!("entry-how-the-bit-goes-down"));
    let ways = [
        (EntryMode::Helix, "entry-way-helix"),
        (EntryMode::ZigZag, "entry-way-zigzag"),
        (EntryMode::Ramp, "entry-way-ramp"),
        (EntryMode::Plunge, "entry-way-plunge"),
    ];
    ui.horizontal(|ui| {
        for (mode, key) in ways {
            if mode == EntryMode::Helix && !helix {
                continue;
            }
            ui.radio_value(&mut settings.mode, mode, tr!(key))
                .on_hover_hint(tr!(&format!("{key}.tooltip")));
        }
    });
    if let Some((_, key)) = ways.iter().find(|w| w.0 == settings.mode) {
        ui.weak(tr!(&format!("{key}.tooltip")));
    }
    ui.separator();
    sizes(ui, settings, diameter, pass_depth, units);
    settings.ramp_distance = settings.ramp_distance.max(0.1);
    if settings.mode != EntryMode::Plunge {
        ui.separator();
        ui.weak(if helix {
            tr!("entry-each-place-the-bit-goes")
        } else {
            tr!("entry-it-slides-in-along-the")
        });
    }
}

/// The chosen way's sizes, each with what it comes to for this bit and pass depth.
fn sizes(
    ui: &mut egui::Ui,
    settings: &mut EntrySettings,
    diameter: f64,
    pass_depth: f64,
    units: Units,
) {
    egui::Grid::new("entry_sizes")
        .num_columns(2)
        .show(ui, |ui| match settings.mode {
            EntryMode::Helix => {
                ui.label(tr!("entry-diameter"))
                    .on_hover_hint(tr!("entry-across-the-circle-the-bit"));
                ui.horizontal(|ui| {
                    ui.add(
                        Number::new(&mut settings.helix_diameter)
                            .range(10.0..=400.0)
                            .speed(1.0)
                            .suffix(" %")
                            .name("Helix diameter"),
                    );
                    ui.weak(units.length(diameter * settings.helix_diameter / 100.0, 1));
                });
                ui.end_row();
                ui.label(tr!("entry-angle"))
                    .on_hover_hint(tr!("entry-how-steeply-it-goes-down"));
                ui.add(
                    Number::new(&mut settings.helix_angle)
                        .name("Helix angle")
                        .range(0.1..=45.0)
                        .speed(0.1)
                        .suffix("°"),
                );
                ui.end_row();
            }
            EntryMode::ZigZag => {
                ui.label(tr!("entry-angle"))
                    .on_hover_hint(tr!("entry-how-steeply-each-leg-goes"));
                ui.add(
                    Number::new(&mut settings.zigzag_angle)
                        .name("Zig-zag angle")
                        .range(0.1..=45.0)
                        .speed(0.1)
                        .suffix("°"),
                );
                ui.end_row();
                ui.label(tr!("entry-passes"))
                    .on_hover_hint(tr!("entry-how-many-legs-to-go"));
                ui.add(
                    Number::new(&mut settings.zigzag_passes)
                        .range(1..=100)
                        .name(tr!("entry-passes")),
                );
                ui.end_row();
                ui.label(tr!("entry-each-leg"));
                ui.label(units.length(leg(settings, pass_depth), 1));
                ui.end_row();
            }
            EntryMode::Ramp => {
                ui.label(tr!("entry-distance"))
                    .on_hover_hint(tr!("entry-how-far-along-the-cut"));
                super::tabs::length(
                    ui,
                    &tr!("entry-ramp-distance-name"),
                    &mut settings.ramp_distance,
                    units,
                );
                ui.end_row();
                ui.label(tr!("entry-angle"));
                ui.label(tr!(
                    "entry-ramp-angle",
                    angle = super::trimmed(ramp_angle(settings, pass_depth).round())
                ));
                ui.end_row();
            }
            EntryMode::Plunge => {}
        });
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_new_toolpath_reads_its_own_entry() {
        let params = json!({
            "side": "outside",
            "entry": {"mode": "zig_zag", "zigzag_angle": 8.0, "zigzag_passes": 3},
            "tool": {"diameter": 6.35, "pass_depth": 3.0}
        });
        let (s, older) = read(&params);
        assert!(!older);
        assert_eq!(s.mode, EntryMode::ZigZag);
        assert_eq!(s.zigzag_passes, 3);
        // the rest of a partial entry is the defaults
        assert!((s.helix_angle - EntrySettings::default().helix_angle).abs() < 1e-9);
    }

    #[test]
    fn older_toolpaths_read_as_the_nearest_entry() {
        let profile = json!({"side": "outside", "ramp": "smooth",
            "ramping": {"by": "distance", "distance": 12.0, "angle": 10.0},
            "tool": {"pass_depth": 3.0}});
        let (s, older) = read(&profile);
        assert!(older);
        assert_eq!(s.mode, EntryMode::Ramp);
        assert!((s.ramp_distance - 12.0).abs() < 1e-9);
        let pocket = json!({"rings": "middle_out", "ramp_plunge_moves": true,
            "ramp_distance": 4.0, "tool": {"pass_depth": 3.0}});
        assert_eq!(read(&pocket).0.mode, EntryMode::ZigZag);
        let plain = json!({"rings": "middle_out", "ramp_plunge_moves": false});
        assert_eq!(read(&plain).0.mode, EntryMode::Plunge);
    }

    #[test]
    fn the_summary_says_the_sizes_in_the_job_s_units() {
        let helix = EntrySettings {
            mode: EntryMode::Helix,
            ..EntrySettings::default()
        };
        assert_eq!(
            summary(&helix, 6.35, 3.0, Units::Mm),
            "Helix 4.8 mm across at 3°, smaller where there is less room"
        );
        let zig = EntrySettings {
            mode: EntryMode::ZigZag,
            zigzag_passes: 3,
            ..EntrySettings::default()
        };
        // an odd number of passes is made even
        assert!(summary(&zig, 6.35, 3.0, Units::Mm).contains("in 4 passes"));
        let ramp = EntrySettings {
            mode: EntryMode::Ramp,
            ramp_distance: 25.4,
            ..EntrySettings::default()
        };
        assert!(summary(&ramp, 6.35, 3.0, Units::Inch).contains("1.000 in"));
    }
}
