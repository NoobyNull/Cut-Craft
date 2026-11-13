//! Entry: how a toolpath goes down into the material (helix, zig-zag, ramp or straight
//! down), in a line at the foot of its cut settings and a window to change it, so the form
//! stays short.

use crate::form;
use cut_craft_bug_report::hint::HoverHint;
use cut_craft_bug_report::number::Number;
use cut_craft_core::document::Units;
use cut_craft_core::entry::{EntryMode, EntrySettings, Ramp, Ramping};
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
        EntryMode::Plunge => "Straight down into the material".into(),
        EntryMode::Helix => format!(
            "Helix {} across at {}°, smaller where there is less room",
            units.length(diameter * settings.helix_diameter / 100.0, 1),
            super::trimmed(settings.helix_angle)
        ),
        EntryMode::ZigZag => format!(
            "Zig-zag at {}° in {} passes ({} legs)",
            super::trimmed(settings.zigzag_angle),
            even(settings.zigzag_passes),
            units.length(leg(settings, pass_depth), 1)
        ),
        EntryMode::Ramp => format!(
            "Ramp down a pass over {} (about {}°)",
            units.length(settings.ramp_distance, 1),
            super::trimmed(ramp_angle(settings, pass_depth).round())
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
        ui.label("Entry:");
        ui.label(summary(&settings, diameter, pass_depth, units));
        if older {
            ui.weak("(as set before entry modes)");
        }
        if ui
            .button("Entry…")
            .on_hover_hint(
                "How the bit goes down into the material: in a helix, a zig-zag, a ramp or \
                 straight down",
            )
            .clicked()
        {
            open = true;
        }
    });
    let before = settings;
    if open {
        egui::Window::new("Entry")
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
    ui.label("How the bit goes down into the material each time a cut starts:");
    let ways = [
        (
            EntryMode::Helix,
            "Helix",
            "Down in circles, like a corkscrew: the gentlest on the bit, for pockets and holes",
        ),
        (
            EntryMode::ZigZag,
            "Zig-zag",
            "Back and forth along the start of the cut, a little lower each time: for narrow \
             slots and lines",
        ),
        (
            EntryMode::Ramp,
            "Ramp",
            "Down along the cut in one straight slope",
        ),
        (
            EntryMode::Plunge,
            "Straight down",
            "Like a drill: quickest, but hardest on the bit, and only for bits that cut at \
             their centre",
        ),
    ];
    ui.horizontal(|ui| {
        for (mode, name, help) in ways {
            if mode == EntryMode::Helix && !helix {
                continue;
            }
            ui.radio_value(&mut settings.mode, mode, name)
                .on_hover_hint(help);
        }
    });
    if let Some((_, _, help)) = ways.iter().find(|w| w.0 == settings.mode) {
        ui.weak(*help);
    }
    ui.separator();
    sizes(ui, settings, diameter, pass_depth, units);
    settings.ramp_distance = settings.ramp_distance.max(0.1);
    if settings.mode != EntryMode::Plunge {
        ui.separator();
        ui.weak(if helix {
            "Each place the bit goes down fits this to the room it has, never steeper: a \
             helix gets smaller, then turns into a zig-zag along the cut; a zig-zag or ramp \
             takes shorter legs. Only where there is no room at all does it go straight \
             down, and Checks before cutting says where."
        } else {
            "It slides in along the groove's own centre line, never deeper than the groove's \
             width sets. Where a groove is too short for it, it goes straight down there, as \
             a V-carve usually does."
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
                ui.label("Diameter").on_hover_hint(
                    "Across the circle the bit's centre goes round, as a share of the bit",
                );
                ui.horizontal(|ui| {
                    ui.add(
                        Number::new(&mut settings.helix_diameter)
                            .range(10.0..=400.0)
                            .speed(1.0)
                            .suffix(" %"),
                    );
                    ui.weak(units.length(diameter * settings.helix_diameter / 100.0, 1));
                });
                ui.end_row();
                ui.label("Angle")
                    .on_hover_hint("How steeply it goes down, from level");
                ui.add(
                    Number::new(&mut settings.helix_angle)
                        .range(0.1..=45.0)
                        .speed(0.1)
                        .suffix("°"),
                );
                ui.end_row();
            }
            EntryMode::ZigZag => {
                ui.label("Angle")
                    .on_hover_hint("How steeply each leg goes down, from level");
                ui.add(
                    Number::new(&mut settings.zigzag_angle)
                        .range(0.1..=45.0)
                        .speed(0.1)
                        .suffix("°"),
                );
                ui.end_row();
                ui.label("Passes").on_hover_hint(
                    "How many legs to go down a pass in; there and back is two, and an odd \
                     number is made even so the bit ends where it began",
                );
                ui.add(Number::new(&mut settings.zigzag_passes).range(1..=100));
                ui.end_row();
                ui.label("Each leg");
                ui.label(units.length(leg(settings, pass_depth), 1));
                ui.end_row();
            }
            EntryMode::Ramp => {
                ui.label("Distance")
                    .on_hover_hint("How far along the cut it takes to go down a pass");
                super::tabs::length(ui, &mut settings.ramp_distance, units);
                ui.end_row();
                ui.label("Angle");
                ui.label(format!(
                    "about {}° for a full pass",
                    super::trimmed(ramp_angle(settings, pass_depth).round())
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
