#[test]
fn a_hidden_toolpath_is_left_out_of_the_job_s_run_and_timed_on_its_own() {
    use crate::toolpath::{Move, MoveKind};
    let mut doc = crate::testing::with_toolpath();
    let first = doc.toolpaths.values().next().unwrap().clone();
    let mut far = first.clone();
    far.moves = vec![
        Move {
            kind: MoveKind::Rapid,
            to: glam::DVec3::new(5000.0, 5000.0, 5.0),
        },
        Move {
            kind: MoveKind::Cut,
            to: glam::DVec3::new(5010.0, 5000.0, 5.0),
        },
    ];
    far.visible = false;
    far.tool.name = "Another bit".into();
    let ctx = super::Context::default();
    let (_, before) = super::job_minutes(&doc, &ctx);
    let id = doc.next_id();
    doc.toolpaths.insert(id, far.clone());
    let (each, after) = super::job_minutes(&doc, &ctx);
    assert!((after - before).abs() < 1e-12, "{before} then {after}");
    let alone = crate::cut_time::toolpath(&far, &ctx.motion).total() / 60.0;
    assert!((each[&id] - alone).abs() < 1e-12);
}

use super::*;
use crate::document::EntityId;

fn sample_document() -> Document {
    Document::default()
}

fn span(text: &str) -> Span {
    Span {
        text: text.into(),
        ..Span::default()
    }
}

#[test]
fn plain_old_notes_read_as_plain_text() {
    let lines = parse("Glue up first\nthen sand");
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0].kind, Kind::Text);
    assert_eq!(lines[0].spans, vec![span("Glue up first")]);
    // the open mark is not shown
    assert_eq!(parse(".Read me")[0].text(), "Read me");
    assert!(opens_with_job(".Read me"));
    assert!(!opens_with_job("Read me"));
}

#[test]
fn headings_lists_and_check_boxes_are_known_by_how_the_line_starts() {
    let lines = parse(
        "# Big\n## Less\n### Least\n- one\n* two\n3. three\n- [ ] todo\n- [x] done\n[ ] bare",
    );
    let kinds: Vec<Kind> = lines.iter().map(|l| l.kind.clone()).collect();
    assert_eq!(
        kinds,
        vec![
            Kind::Heading(1),
            Kind::Heading(2),
            Kind::Heading(3),
            Kind::Bullet,
            Kind::Bullet,
            Kind::Numbered("3".into()),
            Kind::Check(false),
            Kind::Check(true),
            Kind::Check(false),
        ]
    );
    assert_eq!(lines[6].text(), "todo");
    assert_eq!(lines[5].text(), "three");
}

#[test]
fn bold_italic_and_links_are_runs_and_unpaired_marks_stay_as_typed() {
    let spans = &parse("a **bold** and *slanted* [site](https://cutcraft.dev) 2*3")[0].spans;
    assert!(
        spans
            .iter()
            .any(|s| s.text == "bold" && s.bold && !s.italic)
    );
    assert!(spans.iter().any(|s| s.text == "slanted" && s.italic));
    assert!(
        spans
            .iter()
            .any(|s| s.text == "site" && s.link.as_deref() == Some("https://cutcraft.dev"))
    );
    assert!(
        parse("a **bold** and *slanted* [site](https://cutcraft.dev) 2*3")[0]
            .text()
            .ends_with("site 2*3")
    );
}

#[test]
fn html_is_never_read_and_only_web_links_open() {
    let notes = "<script>alert(1)</script> [x](javascript:alert(1)) [y](file:///etc/passwd)";
    let line = &parse(notes)[0];
    assert!(line.spans.iter().all(|s| s.link.is_none()));
    let html = html(notes, &[]);
    assert!(!html.contains("<script"), "{html}");
    assert!(html.contains("&lt;script&gt;"), "{html}");
    assert!(!html.contains("href"), "{html}");
    assert!(safe_link("mailto:me@example.com"));
    assert!(!safe_link("https://"));
    // a link's address is escaped in its tag too
    let html = super::html("[a](https://x.dev/\"onmouseover=\"bad)", &[]);
    assert!(!html.contains("\"onmouseover"), "{html}");
}

#[test]
fn variables_fill_in_and_a_typo_stays_as_typed() {
    let vars = vec![Variable {
        key: "material.thickness".into(),
        group: "Material",
        under: String::new(),
        label: "Thickness".into(),
        description: String::new(),
        value: "18 mm".into(),
    }];
    assert_eq!(
        resolve(
            "Stock {{material.thickness}}, {{material.thicknes}} {{ }}",
            &vars
        ),
        "Stock 18 mm, {{material.thicknes}} {{ }}"
    );
    let line = &parse_resolved("**{{material.thickness}}** thick", &vars)[0];
    assert_eq!(line.spans[0].text, "18 mm");
    assert!(line.spans[0].bold);
    assert_eq!(
        line.spans[0].variable.as_deref(),
        Some("material.thickness")
    );
    // a value holding marks is shown as it is, never read as marks
    let vars = vec![Variable {
        value: "*not italic* <b>".into(),
        ..vars[0].clone()
    }];
    assert_eq!(plain("{{material.thickness}}", &vars), "*not italic* <b>");
    assert!(html("{{material.thickness}}", &vars).contains("*not italic* &lt;b&gt;"));
}

#[test]
fn every_variable_has_a_known_group_a_unique_key_and_words() {
    let mut doc = sample_document();
    doc.variables.insert("depth".into(), 6.5);
    let vars = variables(&doc, &Context::default());
    let mut keys = std::collections::HashSet::new();
    for v in &vars {
        assert!(GROUPS.contains(&v.group), "{}", v.key);
        assert!(keys.insert(v.key.clone()), "{} twice", v.key);
        assert!(
            !v.label.is_empty() && !v.description.is_empty(),
            "{}",
            v.key
        );
        assert!(
            variable_spans(&format!("{{{{{}}}}}", v.key)).len() == 1,
            "{}",
            v.key
        );
    }
    for group in GROUPS {
        assert!(vars.iter().any(|v| v.group == group), "{group} is empty");
    }
    assert_eq!(value(&vars, "var.depth"), Some("6.5"));
    assert_eq!(value(&vars, "job.name"), Some("Untitled"));
}

#[test]
fn material_values_are_in_the_job_s_units() {
    let mut doc = sample_document();
    doc.job.width = 609.6;
    doc.job.height = 304.8;
    doc.job.thickness = 19.05;
    doc.job.units = Units::Inch;
    let vars = variables(&doc, &Context::default());
    assert_eq!(value(&vars, "material.thickness"), Some("0.75 in"));
    assert_eq!(value(&vars, "material.size"), Some("24 × 12 × 0.75 in"));
    doc.job.units = Units::Mm;
    let vars = variables(&doc, &Context::default());
    assert_eq!(value(&vars, "material.thickness"), Some("19.05 mm"));
}

#[test]
fn model_gaps_add_up_to_the_material_less_the_model() {
    let mut doc = sample_document();
    doc.job.thickness = 20.0;
    doc.job.model_position.gap = 3.0;
    let ctx = Context {
        model_height: Some(12.0),
        ..Context::default()
    };
    let vars = variables(&doc, &ctx);
    assert_eq!(value(&vars, "model.gap_above"), Some("3 mm"));
    assert_eq!(value(&vars, "model.gap_below"), Some("5 mm"));
    let vars = variables(&doc, &Context::default());
    assert_eq!(value(&vars, "model.height"), Some("No model"));
}

#[test]
fn toolpaths_and_tools_are_numbered_and_listed() {
    let mut doc = sample_document();
    let tool = |name: &str| Tool {
        name: name.into(),
        diameter: 6.0,
        stepover: 0.4,
        feed_rate: 1500.0,
        ..Tool::default()
    };
    let path = |name: &str, tool: Tool| Toolpath {
        name: name.into(),
        strategy: "profile_toolpath".into(),
        params: serde_json::Value::Null,
        tool,
        vectors: vec![],
        moves: vec![],
        visible: true,
        power: vec![],
        stale: false,
        group: None,
        drill_bank: None,
        fallbacks: vec![],
        across: vec![],
    };
    doc.toolpaths
        .insert(EntityId(901), path("Pocket", tool("6 mm End Mill")));
    doc.toolpaths
        .insert(EntityId(902), path("Profile", tool("3 mm End Mill")));
    doc.toolpaths
        .insert(EntityId(903), path("Edge", tool("6 mm End Mill")));
    let vars = variables(&doc, &Context::default());
    assert_eq!(value(&vars, "toolpaths.count"), Some("3"));
    assert_eq!(value(&vars, "tools.count"), Some("2"));
    assert_eq!(value(&vars, "toolpath.2.tool"), Some("3 mm End Mill"));
    assert_eq!(value(&vars, "toolpath.2.tool_number"), Some("T2"));
    assert_eq!(value(&vars, "toolpath.3.tool_number"), Some("T1"));
    assert_eq!(value(&vars, "tool.1.stepover"), Some("2.4 mm (40%)"));
    assert_eq!(value(&vars, "tool.1.feed"), Some("1500 mm/min"));
    assert_eq!(
        value(&vars, "tools.list"),
        Some("T1: 6 mm End Mill\nT2: 3 mm End Mill")
    );
    assert!(
        value(&vars, "toolpaths.list")
            .unwrap()
            .starts_with("1. Pocket (T1, 6 mm End Mill)")
    );
    // on a machine with a tool changer, a tool's number is its slot
    let ctx = Context {
        slots: vec![None, Some("3 mm End Mill".into())],
        ..Context::default()
    };
    let vars = variables(&doc, &ctx);
    assert_eq!(value(&vars, "toolpath.2.tool_number"), Some("T2"));
    assert_eq!(value(&vars, "toolpath.1.tool_number"), Some("by hand"));
    assert_eq!(value(&vars, "machine.tool_changer"), Some("2 slots"));
}

#[test]
fn the_collet_nut_spindle_and_stick_out_come_from_the_machine_and_the_bit() {
    use crate::collision::{Holder, Holding};
    let mut doc = sample_document();
    let tool = |name: &str, stickout: f64, holder: Option<Holder>| Tool {
        name: name.into(),
        diameter: 6.0,
        stickout,
        holder,
        ..Tool::default()
    };
    let path = |tool: Tool| Toolpath {
        name: "Cut".into(),
        strategy: "profile_toolpath".into(),
        params: serde_json::Value::Null,
        tool,
        vectors: vec![],
        moves: vec![],
        visible: true,
        power: vec![],
        stale: false,
        group: None,
        drill_bank: None,
        fallbacks: vec![],
        across: vec![],
    };
    let small = Holder {
        nut_diameter: 25.0,
        nut_length: 15.0,
        spindle_diameter: 52.0,
    };
    doc.toolpaths
        .insert(EntityId(901), path(tool("Long", 30.0, Some(small))));
    doc.toolpaths
        .insert(EntityId(902), path(tool("Short", 0.0, None)));
    let vars = variables(&doc, &Context::default());
    assert_eq!(
        value(&vars, "machine.collet_nut"),
        Some("35 mm across, 20 mm tall")
    );
    assert_eq!(value(&vars, "machine.spindle_diameter"), Some("65 mm"));
    assert_eq!(value(&vars, "machine.holder_per_bit"), Some("No"));
    assert_eq!(value(&vars, "tool.1.stick_out"), Some("30 mm"));
    let short = value(&vars, "tool.2.stick_out").unwrap();
    assert!(short.ends_with("(not set: guessed)"), "{short}");
    // its own holder counts only when the machine takes holders per bit
    assert_eq!(
        value(&vars, "tool.1.holder"),
        Some("35 mm across, 20 mm tall nut, 65 mm spindle")
    );
    let ctx = Context {
        holding: Holding {
            holder: Holder::default(),
            per_bit: true,
        },
        ..Context::default()
    };
    let vars = variables(&doc, &ctx);
    assert_eq!(value(&vars, "machine.holder_per_bit"), Some("Yes"));
    assert_eq!(
        value(&vars, "tool.1.holder"),
        Some("25 mm across, 15 mm tall nut, 52 mm spindle")
    );
    assert_eq!(
        value(&vars, "tool.2.holder"),
        Some("35 mm across, 20 mm tall nut, 65 mm spindle")
    );
}

#[test]
fn durations_read_like_a_clock() {
    assert_eq!(duration(0.5), "30 s");
    assert_eq!(duration(12.2), "12 min");
    assert_eq!(duration(65.0), "1 h 05 min");
}

#[test]
fn plain_text_keeps_lists_reading_as_lists() {
    let notes = ".# Setup\n- clamp **corners**\n1. zero Z\n- [x] sand\nsee [docs](https://a.dev)";
    assert_eq!(
        plain(notes, &[]),
        "Setup\n- clamp corners\n1. zero Z\n[x] sand\nsee docs (https://a.dev)"
    );
}

#[test]
fn the_numbered_button_counts_on_and_takes_the_numbers_off_again() {
    let (out, _) = Format::Numbered.apply("Clamp\nZero\nCut", 0..15);
    assert_eq!(out, "1. Clamp\n2. Zero\n3. Cut");
    let (out, _) = Format::Numbered.apply(&out, 0..24);
    assert_eq!(out, "Clamp\nZero\nCut");
    // a line under a numbered one carries on from it
    let (out, _) = Format::Numbered.apply("4. Clamp\nZero", 10..10);
    assert_eq!(out, "4. Clamp\n5. Zero");
}

#[test]
fn a_numbered_list_keeps_its_first_number_and_a_link_fills_in_its_variables() {
    assert_eq!(
        html("3. Clamp\n7. Zero", &[]),
        "<ol><li value=\"3\">Clamp</li><li value=\"7\">Zero</li></ol>"
    );
    assert_eq!(
        html("1. A\n2. B", &[]),
        "<ol><li value=\"1\">A</li><li value=\"2\">B</li></ol>"
    );
    assert_eq!(
        html("3. Clamp\n1. Zero", &[]),
        "<ol><li value=\"3\">Clamp</li><li value=\"1\">Zero</li></ol>"
    );
    assert_eq!(
        html("0. Check\n007. Cut", &[]),
        "<ol><li value=\"0\">Check</li><li value=\"7\">Cut</li></ol>"
    );
    let vars = vec![Variable {
        key: "job.name".into(),
        group: "Job",
        under: String::new(),
        label: "Name".into(),
        description: "x".into(),
        value: "Sign".into(),
    }];
    let text = "See [the {{job.name}} page](https://example.com) now";
    assert_eq!(
        plain(text, &vars),
        "See the Sign page (https://example.com) now"
    );
    assert!(
        html(text, &vars).contains("<a href=\"https://example.com\">Sign</a>"),
        "{}",
        html(text, &vars)
    );
}

#[test]
fn html_lists_and_headings() {
    let html = html("# Setup\n- a\n- [ ] b\n1. c\ntext", &[]);
    assert_eq!(
        html,
        "<h3>Setup</h3><ul><li>a</li><li class=\"check\">&#9744; b</li></ul><ol><li value=\"1\">c</li></ol><p>text</p>"
    );
}

#[test]
fn gcode_lines_cannot_end_a_comment_or_run_as_code() {
    let vars = vec![Variable {
        key: "x".into(),
        group: "Job",
        under: String::new(),
        label: "x".into(),
        description: "x".into(),
        value: "two\nG0 Z-50".into(),
    }];
    let lines = gcode_lines("Clamp (corners); then {{x}} — ok\n\n", &vars);
    assert_eq!(lines, vec!["Clamp [corners], then two", "G0 Z-50 - ok"]);
    let lines = gcode_lines("Clamp firmly! Ready? ~ok", &[]);
    assert_eq!(lines, vec!["Clamp firmly. Ready. -ok"]);
    assert!(lines.iter().all(|l| !l.contains(['!', '~', '?'])));
    for line in gcode_lines(&"word ".repeat(40), &[]) {
        assert!(line.len() <= GCODE_COMMENT_WIDTH, "{line}");
    }
    let long = "x".repeat(150);
    assert!(
        gcode_lines(&long, &[])
            .iter()
            .all(|l| l.len() <= GCODE_COMMENT_WIDTH)
    );
}

#[test]
fn the_toolbar_marks_the_selection_or_leaves_the_cursor_between_marks() {
    assert_eq!(
        Format::Bold.apply("glue up", 0..4),
        ("**glue** up".into(), 2..6)
    );
    assert_eq!(
        Format::Italic.apply("glue up", 5..5),
        ("glue **up".into(), 6..6)
    );
    assert_eq!(Format::Bold.apply("é b", 0..1), ("**é** b".into(), 2..3));
    let (text, picked) = Format::Link.apply("see docs", 4..8);
    assert_eq!(text, "see [docs](https://)");
    assert_eq!(&text[picked.start..picked.end], "https://");
}

#[test]
fn the_toolbar_starts_each_picked_line_and_a_second_click_takes_it_off() {
    let (text, _) = Format::Bullet.apply("a\nb\nc", 0..3);
    assert_eq!(text, "- a\n- b\nc");
    let (text, _) = Format::Bullet.apply(&text, 0..5);
    assert_eq!(text, "a\nb\nc");
    // a bullet made a check box, not both
    let (text, _) = Format::Check.apply("- a", 1..1);
    assert_eq!(text, "- [ ] a");
    let (text, at) = Format::Heading.apply("x\ntitle", 3..3);
    assert_eq!(text, "x\n## title");
    assert_eq!(at, 6..6);
}

#[test]
fn variables_go_in_at_the_cursor_and_check_boxes_tick() {
    assert_eq!(insert_at("ab", 1, "{{date}}"), ("a{{date}}b".into(), 9));
    assert_eq!(insert_at("ab", 9, "!"), ("ab!".into(), 3));
    assert_eq!(
        toggle_check(".x\n- [ ] sand\n[x] oil", 1),
        ".x\n- [x] sand\n[x] oil"
    );
    assert_eq!(
        toggle_check("x\n- [ ] sand\n[x] oil", 2),
        "x\n- [ ] sand\n[ ] oil"
    );
    assert_eq!(toggle_check("plain", 0), "plain");
}
