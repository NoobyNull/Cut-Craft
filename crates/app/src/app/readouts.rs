//! The words the app shows for sizes, measurements, inspected points, times and the summary.

use super::*;

/// A sheet's size in the Sheets List: one decimal in mm, three in inches.
pub(super) fn sheet_length(mm: f64, units: cut_craft_core::document::Units) -> String {
    match units {
        cut_craft_core::document::Units::Mm => format!("{mm:.1}"),
        cut_craft_core::document::Units::Inch => format!("{:.3}", mm / 25.4),
    }
}

/// A length in the job's units.
pub(super) fn length(mm: f64, units: cut_craft_core::document::Units) -> String {
    match units {
        cut_craft_core::document::Units::Mm => format!("{mm:.3} mm"),
        cut_craft_core::document::Units::Inch => format!("{:.4} in", mm / 25.4),
    }
}

/// Measure between two points, as rows of the Measure / Inspect form: the distance, the angle
/// from horizontal, the X and Y distances and the two points.
pub(super) fn measured_rows(
    from: Point,
    to: Point,
    units: cut_craft_core::document::Units,
) -> Vec<(String, String)> {
    let d = to - from;
    let point = |p: Point| format!("{}, {}", length(p.x, units), length(p.y, units));
    vec![
        ("Distance".into(), length(d.length(), units)),
        (
            "Angle".into(),
            format!("{:.2}°", d.y.atan2(d.x).to_degrees()),
        ),
        ("X distance".into(), length(d.x.abs(), units)),
        ("Y distance".into(), length(d.y.abs(), units)),
        ("First point".into(), point(from)),
        ("Second point".into(), point(to)),
    ]
}

/// The first four rows of [`measured_rows`] on one line, for the status bar.
pub(super) fn measured(from: Point, to: Point, units: cut_craft_core::document::Units) -> String {
    let rows = measured_rows(from, to, units);
    format!(
        "Distance {}, angle {}, X {}, Y {}",
        rows[0].1, rows[1].1, rows[2].1, rows[3].1
    )
}

/// The clicked span's type, length, ends (and an arc's centre and radius), and its vector's
/// perimeter, area and span count, as rows of the Measure / Inspect form.
pub(super) fn inspected_rows(
    path: &Path,
    at: Point,
    reach: f64,
    units: cut_craft_core::document::Units,
) -> Vec<(String, String)> {
    use cut_craft_core::geometry::Segment;
    let point = |p: Point| format!("{}, {}", length(p.x, units), length(p.y, units));
    let span = |k: usize| {
        let from = if k == 0 {
            path.start
        } else {
            path.segments[k - 1].end()
        };
        Path {
            start: from,
            segments: vec![path.segments[k]],
            closed: false,
        }
    };
    let measure = |p: &Path| {
        p.flatten(cut_craft_core::polygon::TOLERANCE)
            .windows(2)
            .map(|w| w[0].distance(w[1]))
            .sum::<f64>()
    };
    let mut rows = vec![];
    if let Some(k) = cut_craft_core::nodes::span_near(path, at, reach) {
        let one = span(k);
        let kind = match path.segments[k] {
            Segment::Line { .. } => "Line",
            Segment::Arc { .. } => "Arc",
            Segment::Cubic { .. } => "Bezier Curve",
        };
        rows.push(("Span".into(), kind.into()));
        rows.push(("Span length".into(), length(measure(&one), units)));
        rows.push(("Start".into(), point(one.start)));
        rows.push(("End".into(), point(path.segments[k].end())));
        if let Segment::Arc { center, .. } = path.segments[k] {
            rows.push(("Centre".into(), point(center)));
            rows.push(("Radius".into(), length(center.distance(one.start), units)));
        }
    }
    rows.push(("Vector perimeter".into(), length(measure(path), units)));
    if path.closed {
        let area = cut_craft_core::polygon::area(path);
        rows.push((
            "Vector area".into(),
            match units {
                cut_craft_core::document::Units::Mm => format!("{area:.3} mm²"),
                cut_craft_core::document::Units::Inch => format!("{:.4} in²", area / 645.16),
            },
        ));
    }
    rows.push(("Number of spans".into(), path.segments.len().to_string()));
    rows
}

/// [`inspected_rows`] on one line, for the status bar.
pub(super) fn inspected(
    path: &Path,
    at: Point,
    reach: f64,
    units: cut_craft_core::document::Units,
) -> String {
    let rows = inspected_rows(path, at, reach, units);
    let get = |name: &str| rows.iter().find(|r| r.0 == name).map(|r| r.1.as_str());
    let span = match (get("Span"), get("Span length")) {
        (Some(kind), Some(len)) => format!("{kind}, length {len}; "),
        _ => String::new(),
    };
    let area = get("Vector area").map_or_else(String::new, |a| format!(", area {a}"));
    let perimeter = get("Vector perimeter").unwrap_or("");
    format!("{span}vector perimeter {perimeter}{area}")
}

/// Hours, minutes and seconds.
pub(super) fn hms(minutes: f64) -> String {
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "a positive time"
    )]
    let s = (minutes * 60.0).round() as u64;
    format!("{}:{:02}:{:02}", s / 3600, s / 60 % 60, s % 60)
}

/// Each toolpath's estimated time, the visible ones' total, and the rate (in the units
/// picked) and scale they use; a toolpath clicked has its notes written below, and Apply
/// gives back the notes to keep with it.
/// The Machine Acceleration: the summary's times allow for the active `machine`'s
/// accelerations (its name and X, Y, Z from Machine Configuration), while there is one.
pub(super) fn acceleration_tick(
    ui: &mut egui::Ui,
    summary: &mut Summary,
    machine: Option<(String, [f64; 3])>,
) {
    let (name, acceleration) = machine.unzip();
    let tip = match acceleration {
        Some([x, y, z]) => format!(
            "Speeds up, slows down and slows for corners as {} does: {x:.0}, {y:.0}, \
             {z:.0} mm/s² and its Cornering (Machine Configuration)",
            name.unwrap_or_default()
        ),
        None => "Needs an active machine with its accelerations in Machine Configuration".into(),
    };
    ui.add_enabled(
        acceleration.is_some(),
        egui::Checkbox::new(&mut summary.acceleration, tr!("summary-acceleration")),
    )
    .on_hover_hint(&tip)
    .on_disabled_hover_text(&tip);
}

/// The Toolpath Summary's own settings: the rapid rate and the time scale factor.
pub(super) fn summary_fields() -> [crate::form::Field; 2] {
    [
        crate::form::Field {
            key: "rapid_rate".into(),
            label: "Rapid rate".into(),
            help: Some(
                "The machine's G0 speed, where Machine Configuration gives no max rate".into(),
            ),
            kind: crate::form::Kind::Number {
                unit: None,
                min: Some(0.001),
                max: None,
                integer: false,
                steps: vec![],
                slider: false,
                percent_when: None,
            },
            section: false,
            icon: None,
            when: None,
            enabled: None,
            group: None,
            tab: None,
            dim: None,
            diagram: None,
            axis: None,
        },
        crate::form::Field {
            key: "scale_factor".into(),
            label: "Scale factor".into(),
            help: Some(
                "Multiplies every time, for a machine slower than its programmed feeds".into(),
            ),
            kind: crate::form::Kind::Number {
                unit: None,
                min: Some(0.1),
                max: None,
                integer: false,
                steps: vec![],
                slider: false,
                percent_when: None,
            },
            section: false,
            icon: None,
            when: None,
            enabled: None,
            group: None,
            tab: None,
            dim: None,
            diagram: None,
            axis: None,
        },
    ]
}

/// The Toolpath Summary: each toolpath's time, the job's `total` (minutes), its settings and
/// the picked toolpath's notes.
pub(super) fn summary_table(
    ui: &mut egui::Ui,
    rows: &[SummaryRow],
    total: f64,
    summary: &mut Summary,
    units: cut_craft_core::document::Units,
    machine: Option<(String, [f64; 3])>,
) -> Option<(EntityId, String)> {
    ui.strong(tr!("summary-total", time = hms(total)))
        .on_hover_hint(tr!("summary-total.tooltip"));
    ui.separator();
    egui::Grid::new("summary").striped(true).show(ui, |ui| {
        for row in rows {
            let picked = summary.picked == Some(row.id);
            let name = ui
                .add_enabled(row.visible, egui::Button::selectable(picked, &row.name))
                .on_hover_hint(tr!("summary-write-notes"));
            if name.clicked() {
                summary.picked = Some(row.id);
                summary.notes.clone_from(&row.notes);
            }
            ui.label(hms(row.minutes));
            ui.weak(row.notes.lines().next().unwrap_or_default());
            ui.end_row();
        }
    });
    ui.separator();
    ui.weak(tr!("summary-each-time"));
    let per = summary.units.mm_per_min();
    let fields = summary_fields();
    let mut values = serde_json::json!({
        "rapid_rate": summary.rapid / per,
        "scale_factor": summary.scale,
    });
    let picture = |_: &str| None;
    let look = crate::form::Look {
        units,
        picture: &picture,
        material: cut_craft_core::calc::Values::default(),
        tool: None,
        group_end: None,
        materials: None,
        pattern: None,
    };
    if crate::form::show(ui, &fields, &mut values, &look) {
        summary.rapid = values["rapid_rate"]
            .as_f64()
            .map_or(summary.rapid, |v| v * per);
        summary.scale = values["scale_factor"].as_f64().unwrap_or(summary.scale);
    }
    egui::ComboBox::from_label("Rapid rate units")
        .selected_text(summary.units.label())
        .show_ui(ui, |ui| {
            for u in cut_craft_core::toolpath::RateUnits::ALL {
                ui.selectable_value(&mut summary.units, u, u.label());
            }
        });
    acceleration_tick(ui, summary, machine);
    let picked = rows.iter().find(|r| Some(r.id) == summary.picked)?;
    ui.separator();
    ui.label(tr!("summary-notes-for", toolpath = picked.name.as_str()));
    ui.add(egui::TextEdit::multiline(&mut summary.notes).desired_rows(3));
    ui.button(tr!("summary-save-notes"))
        .on_hover_hint(tr!("summary-save-notes.tooltip"))
        .clicked()
        .then(|| (picked.id, summary.notes.clone()))
}
