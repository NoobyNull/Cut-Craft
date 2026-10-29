//! The app driven headlessly, one frame at a time, with synthetic input.

use super::*;
use eframe::egui::{Event, PointerButton, Pos2, RawInput, Rect, vec2};

/// Drives the real UI headlessly, one frame per call, with synthetic pointer input.
#[test]
fn cut_copy_and_paste_vectors() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 20.0, 10.0);
    let before = h.app.doc.vectors.len();
    assert!(before > 0);
    assert!(!h.app.available("paste"));
    h.app.invoke(&h.ctx, "copy");
    h.app.invoke(&h.ctx, "paste");
    assert_eq!(h.app.doc.vectors.len(), before + h.app.clipboard.len());
    assert_eq!(
        h.app.doc.selection.len(),
        h.app.clipboard.len(),
        "the pasted copies are selected"
    );
    h.app.invoke(&h.ctx, "cut");
    assert_eq!(
        h.app.doc.vectors.len(),
        before,
        "cut takes the selection away"
    );
    h.app.invoke(&h.ctx, "undo");
    assert_eq!(h.app.doc.vectors.len(), before + h.app.clipboard.len());
}

#[test]
fn the_window_layout_menu_picks_and_ticks_a_layout() {
    use crate::options::WindowLayout;
    let mut h = Harness::new();
    assert_eq!(h.app.layout_checked("layout_classic"), Some(true));
    assert_eq!(h.app.layout_checked("zoom_box"), None);
    assert!(h.app.available("layout_rail"));
    h.app.invoke(&h.ctx, "layout_rail");
    assert_eq!(h.app.options.general.layout, WindowLayout::Rail);
    assert_eq!(h.app.layout_checked("layout_rail"), Some(true));
    assert_eq!(h.app.layout_checked("layout_classic"), Some(false));
    h.frame(vec![]);
    assert!(h.text_at("Design").is_none(), "no Design page or side tab");
    assert!(h.text_at("Sheets").is_some(), "the browser's tabs");
}

#[test]
fn find_tool_lists_the_tools_whose_names_match() {
    let mut h = Harness::new();
    let names = |h: &Harness, text: &str| {
        h.app
            .search_matches(text)
            .into_iter()
            .map(|(_, label)| label)
            .collect::<Vec<_>>()
    };
    assert_eq!(names(&h, "draw circ"), vec!["Draw Circle"]);
    assert!(names(&h, "POCKET").contains(&"Pocket Toolpath".to_owned()));
    assert!(
        names(&h, "paste in place").is_empty(),
        "nothing to paste yet"
    );
    h.app.invoke(&h.ctx, "find_tool");
    assert_eq!(h.app.search.as_deref(), Some(""));
    // a new window is laid out unseen on its first frame
    h.frame(vec![]);
    h.frame(vec![]);
    assert!(h.text_at("Find Tool").is_some());
    h.frame(vec![Event::Key {
        key: Key::Escape,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers::NONE,
    }]);
    assert!(h.app.search.is_none(), "Escape closes it");
}

#[test]
fn the_rail_layout_lists_a_groups_tools_by_name() {
    let mut h = Harness::new();
    h.app.options.general.layout = crate::options::WindowLayout::Rail;
    h.frame(vec![]);
    assert!(h.text_at("Draw Circle").is_none());
    let groups = h.app.all_groups();
    let create = groups
        .iter()
        .position(|g| g.name == "Create Vectors")
        .unwrap();
    h.app.rail_group = Some(create);
    h.frame(vec![]);
    let at = h.text_at("Draw Circle").expect("the group's tools by name");
    h.frame(vec![Event::PointerMoved(at)]);
    h.press(at, true, Modifiers::NONE);
    h.press(at, false, Modifiers::NONE);
    h.frame(vec![]);
    assert_eq!(h.app.form.as_ref().map(|f| f.id), Some("draw_circle"));
    assert!(
        h.app.rail_group.is_none(),
        "an unpinned list closes on a pick"
    );
    assert!(
        h.text_at("Draw Circle").is_some(),
        "the form in the inspector"
    );
    assert!(h.text_at("Apply").is_some());
}

#[test]
fn the_ribbon_layout_shows_a_tabs_groups() {
    let mut h = Harness::new();
    h.app.options.general.layout = crate::options::WindowLayout::Ribbon;
    h.frame(vec![]);
    for name in ["Design", "Model", "Toolpaths", "Create Vectors", "Sheets"] {
        assert!(h.text_at(name).is_some(), "{name}");
    }
    assert!(h.text_at("Toolpath Operations").is_none());
    h.app.ribbon_tab = 2;
    h.frame(vec![]);
    assert!(h.text_at("Toolpath Operations").is_some());
    h.app.invoke(&h.ctx, "pocket_toolpath");
    h.frame(vec![]);
    assert!(
        h.text_at("Pocket Toolpath").is_some(),
        "the form's heading in the inspector"
    );
}

#[test]
fn notes_starting_with_a_dot_open_with_the_job() {
    let dir = std::env::temp_dir().join(format!("cut-craft-notes-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let mut h = Harness::new();
    for (notes, opens) in [(".Read me", true), ("quiet", false)] {
        h.app.doc.notes = notes.into();
        let path = dir.join("notes.ccjob");
        h.app.save_to(&path);
        h.app.form = None;
        assert!(h.app.open_path(&path));
        assert_eq!(
            h.app.form.as_ref().map(|f| f.id) == Some("notes"),
            opens,
            "{notes}"
        );
    }
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn drawing_snaps_to_nearby_corners() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 20.0, 10.0);
    h.app.invoke(&h.ctx, "draw_circle");
    h.app.invoke(&h.ctx, "snap_options");
    assert!(h.app.snap_window);
    h.frame(vec![]);
    let scale = h.app.view_transform.unwrap().scale;
    let near = 2.0 / f64::from(scale); // two pixels off the corner
    let drag = Drag {
        start: Point::new(30.0 + near, 20.0 - near),
        end: Point::new(50.0, 50.0),
        from_center: false,
        constrain: false,
    };
    let mut snapped = None;
    let ctx = h.ctx.clone();
    headless(ctx.run_ui(RawInput::default(), |ui| {
        egui::CentralPanel::default().show(ui, |ui| {
            snapped = h.app.snapped(ui, Some(Gesture::Dragging(drag)), scale);
        });
    }));
    let Some(Gesture::Dragging(d)) = snapped else {
        panic!("still a drag")
    };
    assert_eq!(d.start, Point::new(30.0, 20.0), "onto the corner");
    assert_eq!(d.end, Point::new(50.0, 50.0), "nothing near");
}

#[test]
fn clicking_a_component_in_the_2d_view_selects_it() {
    let mut h = Harness::new();
    h.app.doc = cut_craft_core::testing::domed();
    h.frame(vec![]);
    h.click(h.at(20.0, 20.0), Modifiers::NONE);
    assert_eq!(
        h.app.doc.selected_components.len(),
        1,
        "the dome under the click"
    );
    h.click(h.at(2.0, 2.0), Modifiers::NONE);
    assert!(h.app.doc.selected_components.is_empty(), "off the dome");
}

#[test]
fn freehand_follows_the_whole_drag() {
    let mut h = Harness::new();
    h.app.invoke(&h.ctx, "draw_freehand");
    let points = [(10.0, 10.0), (20.0, 30.0), (30.0, 10.0), (40.0, 30.0)];
    h.frame(vec![Event::PointerMoved(h.at(10.0, 10.0))]);
    h.press(h.at(10.0, 10.0), true, Modifiers::NONE);
    for (x, y) in points {
        h.frame(vec![Event::PointerMoved(h.at(x, y))]);
    }
    h.press(h.at(40.0, 30.0), false, Modifiers::NONE);
    h.frame(vec![]);
    let drawn = h.app.doc.vectors.values().next().expect("a stroke");
    let b = drawn.bounds();
    assert!(
        b.size().y > 15.0,
        "it went up and down, not straight: {b:?}"
    );
}

#[test]
fn a_sculpting_drag_deposits_along_the_stroke() {
    let mut h = Harness::new();
    h.app.doc = cut_craft_core::testing::domed();
    h.frame(vec![]);
    let top = |h: &Harness| {
        h.app
            .doc
            .components
            .values()
            .next()
            .unwrap()
            .height_at(Point::new(20.0, 20.0))
    };
    let before = top(&h).unwrap();
    h.app.invoke(&h.ctx, "sculpt");
    h.app.form.as_mut().unwrap().params["tool"] = Value::from("deposit");
    h.frame(vec![Event::PointerMoved(h.at(16.0, 20.0))]);
    h.press(h.at(16.0, 20.0), true, Modifiers::NONE);
    for x in [18.0, 20.0, 22.0, 24.0] {
        h.frame(vec![Event::PointerMoved(h.at(x, 20.0))]);
    }
    h.press(h.at(24.0, 20.0), false, Modifiers::NONE);
    h.frame(vec![]);
    assert!(top(&h).unwrap() > before, "the stroke raised the dome");
    assert!(h.app.doc.vectors.is_empty(), "sculpting draws no vectors");
}

/// Vendor tool databases are not part of the repo: `CUT_CRAFT_VENDOR_FILES` names a
/// folder holding some to try them.
#[test]
fn a_vendor_tool_database_imports_when_one_is_given() {
    let mut h = Harness::new();
    let before = h.app.tools.tools.len();
    h.app
        .import_tools(std::path::Path::new("/nonexistent.vtdb"));
    assert_eq!(h.app.tools.tools.len(), before, "nothing added on failure");
    let Some(dir) = std::env::var_os("CUT_CRAFT_VENDOR_FILES") else {
        return;
    };
    let Some(file) = std::fs::read_dir(dir).ok().and_then(|d| {
        d.flatten()
            .map(|e| e.path())
            .find(|p| p.extension().is_some_and(|x| x == "vtdb"))
    }) else {
        return;
    };
    h.app.import_tools(&file);
    assert!(h.app.tools.tools.len() > before + 10, "{}", file.display());
    let name = file.file_stem().unwrap().to_string_lossy();
    assert!(
        h.app
            .tools
            .tools
            .iter()
            .any(|t| t.group.starts_with(&*name)),
        "in a group named after the file"
    );
}

#[test]
fn node_editing_drags_a_corner_and_offers_its_menu() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 20.0, 10.0);
    let id = *h.app.doc.selection.iter().next().unwrap();
    h.app.invoke(&h.ctx, "node_edit");
    assert_eq!(h.app.view_tool, Some(ViewTool::NodeEdit));
    // smart snapping would line the corner up with the other top corner
    h.app.snap.smart_snapping = false;
    let (from, to) = (h.at(30.0, 20.0), h.at(40.0, 25.0));
    h.drag(from, to);
    let corners = cut_craft_core::nodes::nodes(&h.app.doc.vectors[&id]);
    assert!(
        corners
            .iter()
            .any(|q| q.distance(Point::new(40.0, 25.0)) < 0.5),
        "the corner followed the drag: {corners:?}"
    );
    assert_eq!(
        h.app.doc.vectors.len(),
        1,
        "a node drag selects nothing new"
    );
    let target = h
        .app
        .node_target(Point::new(10.0, 10.0), 1.0)
        .expect("a node there");
    let menu = gui::node_menu(&target);
    let delete = leaf(&menu, &["Delete Point"]);
    h.app.invoke(&h.ctx, &delete.action);
    assert_eq!(
        cut_craft_core::nodes::nodes(&h.app.doc.vectors[&id]).len(),
        3
    );
    h.app.invoke(&h.ctx, "node_edit");
    assert_eq!(h.app.view_tool, None, "N toggles it off");
}

#[test]
fn a_template_opens_as_a_new_unsaved_job() {
    let dir = std::env::temp_dir().join(format!("cut-craft-template-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("plate.ccjobt");
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 20.0, 10.0);
    std::fs::write(&path, cut_craft_core::file::save_job(&h.app.doc)).unwrap();
    let mut fresh = Harness::new();
    assert!(fresh.app.open_path(&path));
    assert_eq!(fresh.app.doc.vectors.len(), h.app.doc.vectors.len());
    assert_eq!(fresh.app.session.path, None, "saving asks for a new name");
    assert!(!fresh.app.recent.contains(&path));
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn measuring_reports_distances_and_spans() {
    use cut_craft_core::document::Units;
    let m = measured(Point::ZERO, Point::new(3.0, 4.0), Units::Mm);
    assert_eq!(m, "Distance 5.000 mm, angle 53.13°, X 3.000 mm, Y 4.000 mm");
    let square = cut_craft_core::testing::square(0.0, 0.0, 10.0);
    let i = inspected(&square, Point::new(5.0, 0.0), 0.5, Units::Mm);
    assert_eq!(
        i,
        "Line, length 10.000 mm; vector perimeter 40.000 mm, area 100.000 mm²"
    );
}

#[test]
fn times_read_as_hours_minutes_and_seconds() {
    assert_eq!(hms(0.0), "0:00:00");
    assert_eq!(hms(61.5), "1:01:30");
}

#[test]
fn help_items_open_their_windows() {
    let mut h = Harness::new();
    for (id, info) in [
        ("about_cut_craft", Info::About),
        ("keyboard_shortcuts", Info::Shortcuts),
        ("third_party_licenses", Info::Licenses),
        ("toolpath_summary", Info::Summary),
    ] {
        h.app.invoke(&h.ctx, id);
        assert_eq!(h.app.info, Some(info));
    }
}

#[test]
fn a_toolpath_preview_becomes_a_component() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    let params = h
        .app
        .registry
        .get("pocket_toolpath")
        .unwrap()
        .initial_params(&h.app.doc);
    h.app.apply("pocket_toolpath", &params).unwrap();
    assert!(!h.app.available("create_component_from_toolpath_preview"));
    h.app.invoke(&h.ctx, "preview_all_toolpaths");
    h.app
        .invoke(&h.ctx, "create_component_from_toolpath_preview");
    let made = h.app.doc.components.values().next().expect("a component");
    let thickness = h.app.doc.job.thickness;
    let (lo, hi) = made.height_range().unwrap();
    assert!((hi - thickness).abs() < 1e-9 && lo < hi, "{lo} {hi}");
}

#[test]
fn clicking_a_picture_selects_it_and_delete_removes_it() {
    use cut_craft_core::bitmap::Bitmap;
    use cut_craft_core::geometry::{Point, Rect};
    let mut h = Harness::new();
    let mut c = Change::builder(&h.app.doc);
    let rect = Rect::new(Point::new(20.0, 20.0), Point::new(120.0, 120.0));
    let id = c.add_bitmap(Bitmap {
        name: "b".into(),
        columns: 1,
        rows: 1,
        grey: vec![0],
        rect,
        depth: Vec::new(),
    });
    h.app.record(Ok(c.finish()), "added");
    h.frame(vec![]);
    let at = h.at(70.0, 70.0);
    h.click(at, Modifiers::NONE);
    assert_eq!(h.app.doc.selected_bitmaps, [id].into());
    h.click(h.at(5.0, 5.0), Modifiers::NONE);
    assert!(
        h.app.doc.selected_bitmaps.is_empty(),
        "clicking elsewhere lets it go"
    );
    h.click(at, Modifiers::NONE);
    h.app.invoke(&h.ctx, "delete");
    assert!(h.app.doc.bitmaps.is_empty());
}

#[test]
fn clicking_one_of_a_group_picks_the_group() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 20.0, 20.0);
    h.rectangle(60.0, 10.0, 20.0, 20.0);
    h.app.invoke(&h.ctx, "select_all_vectors");
    h.app.invoke(&h.ctx, "group");
    h.click(h.at(5.0, 5.0), Modifiers::NONE);
    assert!(h.app.doc.selection.is_empty());
    h.click(h.at(10.0, 20.0), Modifiers::NONE);
    assert_eq!(h.app.doc.selection.len(), 2, "the edge of one picks both");
    h.click(h.at(10.0, 20.0), Modifiers::SHIFT);
    assert!(
        h.app.doc.selection.is_empty(),
        "Shift-click takes the group away"
    );
    h.click(h.at(10.0, 20.0), Modifiers::NONE);
    h.app.invoke(&h.ctx, "ungroup");
    h.click(h.at(10.0, 20.0), Modifiers::NONE);
    assert_eq!(h.app.doc.selection.len(), 1, "ungrouped, one at a time");
}

/// A headless frame's output: no renderer takes its texture changes, so they are let go.
fn headless(mut out: egui::FullOutput) -> egui::FullOutput {
    out.textures_delta.clear();
    out
}

/// A file dropped on the window, as the windowing layer reports one.
#[derive(Debug)]
struct Dropped(std::path::PathBuf);

impl egui::DroppedFile for Dropped {
    fn path(&self) -> &std::path::Path {
        &self.0
    }

    fn bytes(&self) -> Result<Vec<u8>, String> {
        std::fs::read(&self.0).map_err(|e| e.to_string())
    }
}

struct Harness {
    ctx: egui::Context,
    app: CutCraftApp,
    time: f64,
    /// What the last frame drew.
    shapes: Vec<egui::epaint::ClippedShape>,
    /// What the last frame put on the system clipboard.
    copied: String,
}

impl Harness {
    /// The app as it starts: the start page, no job open.
    fn at_start() -> Self {
        let mut h = Self {
            ctx: egui::Context::default(),
            app: CutCraftApp::new(),
            time: 0.0,
            shapes: vec![],
            copied: String::new(),
        };
        // previews appear cut at once unless a test plays one
        h.app.display.animate_preview = false;
        // no browser opens from a test
        h.app.reporter.browser = false;
        // nor a question about crossing vectors: the controls are clicked with whatever is
        // selected
        let checks = &mut h.app.options.confirmations;
        checks.toolpath_intersections = crate::options::IntersectionCheck::DontCheck;
        checks.radiused_offset_intersections = crate::options::IntersectionCheck::DontCheck;
        checks.sharp_offset_intersections = crate::options::IntersectionCheck::DontCheck;
        h.frame(vec![]);
        h.frame(vec![]);
        h
    }

    /// A new job made the way a user makes one: New, then OK in Job Setup.
    fn new() -> Self {
        let mut h = Self::at_start();
        h.app.invoke(&h.ctx, "new");
        h.app.job_setup_ok();
        h.frame(vec![]);
        h.frame(vec![]);
        h
    }

    fn frame(&mut self, events: Vec<Event>) {
        self.frame_holding(Modifiers::NONE, events);
    }

    /// One frame with `files` dropped on the window (or only dragged over it).
    fn drop_files(&mut self, files: &[std::path::PathBuf], dropped: bool) {
        self.time += 0.1;
        let input = RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1280.0, 800.0))),
            time: Some(self.time),
            dropped_files: if dropped {
                files
                    .iter()
                    .map(|p| std::sync::Arc::new(Dropped(p.clone())) as egui::DroppedFileHandle)
                    .collect()
            } else {
                vec![]
            },
            hovered_files: if dropped {
                vec![]
            } else {
                files
                    .iter()
                    .map(|p| egui::HoveredFile {
                        path: Some(p.clone()),
                        ..Default::default()
                    })
                    .collect()
            },
            ..Default::default()
        };
        self.shapes = headless(self.ctx.run_ui(input, |ui| self.app.pass(ui))).shapes;
    }

    /// One frame with `modifiers` held, as the windowing layer reports them every frame.
    fn frame_holding(&mut self, modifiers: Modifiers, events: Vec<Event>) {
        self.time += 0.1;
        let mut events = events;
        events.insert(0, Event::ModifiersChanged(modifiers));
        let input = RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1280.0, 800.0))),
            time: Some(self.time),
            events,
            ..Default::default()
        };
        let out = headless(self.ctx.run_ui(input, |ui| self.app.pass(ui)));
        self.shapes = out.shapes;
        self.copied = out
            .platform_output
            .commands
            .iter()
            .find_map(|c| match c {
                egui::OutputCommand::CopyText(text) => Some(text.clone()),
                _ => None,
            })
            .unwrap_or_default();
    }

    /// Every place the last frame drew `text`.
    fn texts_at(&self, text: &str) -> Vec<Pos2> {
        let drawn = self
            .shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                egui::Shape::Text(t) if t.galley.text() == text => {
                    Some(t.pos + t.galley.size() / 2.0)
                }
                _ => None,
            });
        drawn.collect()
    }

    /// The middle of `text` where the last frame drew it.
    fn text_at(&self, text: &str) -> Option<Pos2> {
        self.shapes.iter().find_map(|clipped| match &clipped.shape {
            egui::Shape::Text(t) if t.galley.text() == text => Some(t.pos + t.galley.size() / 2.0),
            _ => None,
        })
    }

    fn right_click(&mut self, at: Pos2) {
        self.frame(vec![Event::PointerMoved(at)]);
        for pressed in [true, false] {
            let button = Event::PointerButton {
                pos: at,
                button: PointerButton::Secondary,
                pressed,
                modifiers: Modifiers::NONE,
            };
            self.frame(vec![button]);
        }
        self.frame(vec![]);
    }

    /// Screen position of a job coordinate in the 2D view.
    fn at(&self, x: f64, y: f64) -> Pos2 {
        self.app
            .view_transform
            .expect("the 2D view has been drawn")
            .to_screen(Point::new(x, y))
    }

    fn press(&mut self, pos: Pos2, pressed: bool, modifiers: Modifiers) {
        let button = Event::PointerButton {
            pos,
            button: PointerButton::Primary,
            pressed,
            modifiers,
        };
        self.frame_holding(modifiers, vec![button]);
    }

    fn drag(&mut self, from: Pos2, to: Pos2) {
        self.frame(vec![Event::PointerMoved(from)]);
        self.press(from, true, Modifiers::NONE);
        self.frame(vec![Event::PointerMoved(from.lerp(to, 0.5))]);
        self.frame(vec![Event::PointerMoved(to)]);
        self.press(to, false, Modifiers::NONE);
        self.frame(vec![]);
    }

    /// A click with `modifiers` held for the whole gesture, as a held key would be.
    fn click(&mut self, at: Pos2, modifiers: Modifiers) {
        self.frame_holding(modifiers, vec![Event::PointerMoved(at)]);
        self.press(at, true, modifiers);
        self.press(at, false, modifiers);
        self.frame_holding(modifiers, vec![]);
    }

    fn double_click(&mut self, at: Pos2) {
        self.frame(vec![Event::PointerMoved(at)]);
        for pressed in [true, false, true, false] {
            self.press(at, pressed, Modifiers::NONE);
        }
        self.frame(vec![]);
    }

    /// Clicks the item at `path` (names, top down) of the right-click menu `menu` builds.
    fn choose(&mut self, menu: impl Fn(&CutCraftApp) -> Vec<gui::MenuItem>, path: &[&str]) {
        let items = menu(&self.app);
        let item = leaf(&items, path);
        assert!(
            item.is_enabled && self.app.available(&item.action),
            "{path:?} is greyed"
        );
        self.app.invoke(&self.ctx, &item.action);
    }

    /// Draws a rectangle through the command, then closes its form.
    fn rectangle(&mut self, x: f64, y: f64, width: f64, height: f64) {
        let params = serde_json::json!({ "x": x, "y": y, "width": width, "height": height, "corners": "square", "radius": 0.0 });
        self.app.apply("draw_rectangle", &params).unwrap();
        self.app.form = None;
        self.frame(vec![]);
    }
}

/// The item at `path` (names, top down) of a menu.
fn leaf<'a>(menu: &'a [gui::MenuItem], path: &[&str]) -> &'a gui::MenuItem {
    let (name, rest) = path.split_first().expect("a path to an item");
    let item = menu
        .iter()
        .find(|i| i.name == *name)
        .unwrap_or_else(|| panic!("no {name} in the menu"));
    if rest.is_empty() {
        item
    } else {
        leaf(&item.children, rest)
    }
}

fn near(a: Point, b: Point) -> bool {
    // one screen pixel is under a millimetre at this window size
    a.distance(b) < 2.0
}

#[test]
fn dragging_with_the_rectangle_tool_draws_that_rectangle() {
    let mut h = Harness::new();
    h.app.invoke(&h.ctx, "draw_rectangle");
    h.frame(vec![]);
    let (from, to) = (h.at(100.0, 100.0), h.at(300.0, 200.0));
    h.drag(from, to);
    assert_eq!(h.app.doc.vectors.len(), 1);
    let b = h.app.doc.vectors.values().next().unwrap().bounds();
    assert!(
        near(b.min, Point::new(100.0, 100.0)) && near(b.max, Point::new(300.0, 200.0)),
        "{b:?}"
    );
    assert!(h.app.history.can_undo());
    let width = h.app.form.as_ref().unwrap().params["width"]
        .as_f64()
        .unwrap();
    assert!(
        (width - 200.0).abs() < 2.0,
        "the form shows the dragged size, got {width}"
    );
}

#[test]
fn clicking_selects_without_adding_undo_steps() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    h.click(h.at(500.0, 300.0), Modifiers::NONE);
    assert!(
        h.app.doc.selection.is_empty(),
        "clicking empty space clears the selection"
    );
    h.click(h.at(60.0, 35.0), Modifiers::NONE);
    assert_eq!(
        h.app.doc.selection.len(),
        1,
        "clicking the right edge selects it"
    );
    assert!(h.app.history.undo(&mut h.app.doc));
    assert!(
        h.app.doc.vectors.is_empty(),
        "the last undo step is the draw, not a click"
    );
}

#[test]
fn shift_click_adds_and_removes() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    h.rectangle(100.0, 10.0, 50.0, 50.0);
    h.click(h.at(10.0, 35.0), Modifiers::NONE);
    h.click(h.at(150.0, 35.0), Modifiers::SHIFT);
    assert_eq!(h.app.doc.selection.len(), 2);
    h.click(h.at(10.0, 35.0), Modifiers::SHIFT);
    assert_eq!(h.app.doc.selection.len(), 1);
}

#[test]
fn left_to_right_windows_and_right_to_left_crosses() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    h.rectangle(100.0, 10.0, 50.0, 50.0);
    h.drag(h.at(0.0, 0.0), h.at(120.0, 80.0));
    assert_eq!(
        h.app.doc.selection.len(),
        1,
        "only the first rectangle is fully inside"
    );
    h.drag(h.at(120.0, 80.0), h.at(0.0, 0.0));
    assert_eq!(
        h.app.doc.selection.len(),
        2,
        "crossing also takes the one it touches"
    );
}

#[test]
fn forms_open_with_values_from_the_document() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 50.0, 20.0);
    h.app.invoke(&h.ctx, "set_size");
    let params = &h.app.form.as_ref().unwrap().params;
    assert_eq!(
        (params["width"].as_f64(), params["height"].as_f64()),
        (Some(50.0), Some(20.0))
    );
}

fn cut_min_x(app: &CutCraftApp) -> f64 {
    cut_craft_core::testing::cut_bounds(app.doc.toolpaths.values().next().unwrap())
        .min
        .x
}

#[test]
fn guides_come_out_of_the_rulers_move_and_go_back_in() {
    let mut h = Harness::new();
    h.frame(vec![]);
    let area = h.app.view_area.unwrap();
    let ruler = h.ctx.global_style().spacing.interact_size.y / 2.0;
    let top_ruler = egui::pos2(area.center().x, area.min.y + ruler);
    h.drag(top_ruler, h.at(100.0, 80.0));
    assert_eq!(h.app.doc.guides.len(), 1, "a guide across the job");
    let g = h.app.doc.guides[0];
    assert!(!g.vertical && (g.at - 80.0).abs() < 0.5, "{g:?}");
    h.drag(h.at(150.0, 80.0), h.at(150.0, 60.0));
    assert!((h.app.doc.guides[0].at - 60.0).abs() < 0.5, "dragged along");
    h.app.invoke(&h.ctx, "lock_all_existing_guides");
    h.drag(h.at(150.0, 60.0), h.at(150.0, 40.0));
    assert!(
        (h.app.doc.guides[0].at - 60.0).abs() < 0.5,
        "a locked guide stays"
    );
    h.app.invoke(&h.ctx, "unlock_all_existing_guides");
    h.drag(h.at(150.0, 60.0), top_ruler);
    assert!(
        h.app.doc.guides.is_empty(),
        "back onto the ruler deletes it"
    );
    h.app.doc.guides = vec![cut_craft_core::document::Guide::vertical(5.0)];
    h.app.invoke(&h.ctx, "delete_all_guides");
    assert_eq!(h.app.doc.guides, [] as [cut_craft_core::document::Guide; 0]);
}

#[test]
fn transform_mode_scales_by_a_corner_and_rotates_after_a_second_click() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 20.0, 10.0);
    h.app.invoke(&h.ctx, "transform");
    assert_eq!(h.app.view_tool, Some(ViewTool::Transform));
    let (from, to) = (h.at(30.0, 20.0), h.at(50.0, 30.0));
    h.drag(from, to);
    let b = h.app.doc.selection_bounds().unwrap();
    assert!(
        (b.max.x - 50.0).abs() < 0.5 && (b.max.y - 30.0).abs() < 0.5,
        "{b:?}"
    );
    assert!((b.min.x - 10.0).abs() < 1e-6, "the opposite corner stays");
    assert!(!h.app.rotating);
    let middle = h.at(b.center().x, b.center().y);
    h.click(middle, Modifiers::NONE);
    assert!(
        h.app.rotating,
        "a second click on the middle turns rotation on"
    );
}

#[test]
fn a_toolpath_over_a_keep_out_zone_is_warned_about() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    let zone = cut_craft_core::testing::square(55.0, 20.0, 10.0);
    h.app.doc.keep_out = vec![zone];
    h.app.invoke(&h.ctx, "profile_toolpath");
    let params = h.app.form.as_ref().unwrap().params.clone();
    h.app.apply("profile_toolpath", &params).unwrap();
    assert!(
        h.app.status.message.contains("keep-out"),
        "{}",
        h.app.status.message
    );
    h.app.doc.keep_out.clear();
    h.app.apply("profile_toolpath", &params).unwrap();
    assert!(!h.app.status.message.contains("keep-out"));
}

#[test]
fn toolpaths_follow_their_vectors_when_recalculated() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    h.app.invoke(&h.ctx, "profile_toolpath");
    let params = h.app.form.as_ref().unwrap().params.clone();
    h.app.apply("profile_toolpath", &params).unwrap();
    let before = cut_min_x(&h.app);
    let shift =
        serde_json::json!({ "relative": true, "x": 100.0, "y": 0.0, "anchor": "bottom_left" });
    h.app.apply("move", &shift).unwrap();
    h.app.invoke(&h.ctx, "recalculate_all_toolpaths");
    assert!((cut_min_x(&h.app) - before - 100.0).abs() < 1e-6);
    h.app.invoke(&h.ctx, "undo");
    assert!(
        (cut_min_x(&h.app) - before).abs() < 1e-6,
        "recalculating is one undo step"
    );
}

#[test]
fn hiding_a_layer_from_the_panel_is_undoable_and_stops_picking() {
    let mut h = Harness::new();
    h.app.design_tab = DESIGN_TABS.iter().position(|&t| t == "Layers").unwrap();
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    h.app.layer_action(&h.ctx, LayerId(1), "toggle_visible");
    h.frame(vec![]);
    h.click(h.at(60.0, 35.0), Modifiers::NONE);
    assert!(h.app.doc.selection.is_empty());
    h.app.invoke(&h.ctx, "undo");
    assert!(h.app.doc.layers[0].visible);
}

#[test]
fn choosing_a_layer_makes_it_current_without_an_undo_step() {
    let mut h = Harness::new();
    h.app
        .apply("add_layer", &serde_json::json!({ "name": "Top" }))
        .unwrap();
    h.app.layer_action(&h.ctx, LayerId(1), "set_current_layer");
    assert_eq!(h.app.doc.current_layer, LayerId(1));
    h.app.invoke(&h.ctx, "undo");
    assert_eq!(
        h.app.doc.layers.len(),
        1,
        "the last undo step added the layer"
    );
}

#[test]
fn the_layer_menu_acts_on_the_layer_clicked() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    h.app
        .apply("add_layer", &serde_json::json!({ "name": "Top" }))
        .unwrap();
    let layer = |i: usize| move |app: &CutCraftApp| gui::layer_menu(&app.doc, &app.doc.layers[i]);
    h.choose(layer(1), &["Move Selection Here"]);
    assert!(
        h.app
            .doc
            .vectors
            .keys()
            .all(|&id| h.app.doc.layer_of(id) == LayerId(2))
    );
    h.choose(layer(0), &["Activate"]);
    assert_eq!(h.app.doc.current_layer, LayerId(1));
    h.choose(layer(0), &["Show", "Only This"]);
    let visible: Vec<bool> = h.app.doc.layers.iter().map(|l| l.visible).collect();
    assert_eq!(visible, [true, false]);
    h.choose(layer(0), &["Lock"]);
    let locked = layer(0)(&h.app);
    assert!(!leaf(&locked, &["Lock"]).is_enabled && leaf(&locked, &["Unlock"]).is_enabled);
    h.choose(layer(1), &["Rename"]);
    let form = h.app.form.take().unwrap();
    assert_eq!(
        (form.id, form.params["name"].as_str()),
        ("layer_properties", Some("Top"))
    );
    h.choose(layer(0), &["Delete", "Empty"]);
    let names: Vec<&str> = h.app.doc.layers.iter().map(|l| l.name.as_str()).collect();
    assert_eq!(names, ["Top"], "the first layer had nothing left on it");
}

#[test]
fn the_toolpath_menu_acts_on_the_toolpath_clicked() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    for _ in 0..2 {
        let params = h.app.registry.get("profile_toolpath").unwrap();
        let params = params.initial_params(&h.app.doc);
        h.app.apply("profile_toolpath", &params).unwrap();
    }
    let toolpath = |i: usize| {
        move |app: &CutCraftApp| {
            let (&id, t) = app.doc.toolpaths.iter().nth(i).unwrap();
            gui::toolpath_menu(id, t)
        }
    };
    let min_x = |app: &CutCraftApp| -> Vec<f64> {
        let bounds = app.doc.toolpaths.values();
        bounds
            .map(|t| cut_craft_core::testing::cut_bounds(t).min.x)
            .collect()
    };
    h.choose(toolpath(0), &["Hide", "This"]);
    let visible: Vec<bool> = h.app.doc.toolpaths.values().map(|t| t.visible).collect();
    assert_eq!(visible, [false, true]);
    let before = min_x(&h.app);
    let shift =
        serde_json::json!({ "relative": true, "x": 100.0, "y": 0.0, "anchor": "bottom_left" });
    h.app.apply("move", &shift).unwrap();
    h.choose(toolpath(1), &["Recalculate", "Visible"]);
    let after = min_x(&h.app);
    assert!(
        (after[0] - before[0]).abs() < 1e-6 && (after[1] - before[1] - 100.0).abs() < 1e-6,
        "only the shown toolpath follows its vectors: {before:?} → {after:?}"
    );
    let second = *h.app.doc.toolpaths.keys().nth(1).unwrap();
    h.choose(toolpath(1), &["Rename"]);
    let form = h.app.form.take().unwrap();
    assert_eq!(form.id, "toolpath_properties");
    assert_eq!(form.params["toolpath"], serde_json::json!(second));
    h.choose(toolpath(1), &["Delete", "Invisible"]);
    assert_eq!(
        h.app.doc.toolpaths.keys().copied().collect::<Vec<_>>(),
        [second]
    );
}

#[test]
fn the_level_menu_mirrors_and_components_move_between_levels() {
    use cut_craft_core::component::Mirror;
    let mut h = Harness::new();
    h.app.doc = cut_craft_core::testing::domed();
    let params = h.app.registry.get("insert_level").unwrap();
    let params = params.initial_params(&h.app.doc);
    h.app.apply("insert_level", &params).unwrap();
    let level_menu = |app: &CutCraftApp| {
        let (&id, level) = app.doc.levels.iter().next().unwrap();
        gui::level_menu(&app.doc, id, level)
    };
    h.choose(level_menu, &["Mirror Mode", "Left to Right"]);
    assert_eq!(h.app.doc.levels[&0].mirror, Mirror::LeftToRight);
    assert!(
        !leaf(&level_menu(&h.app), &["Wrapping"]).is_enabled,
        "flat job"
    );
    h.app.doc.selected_components = [EntityId(1)].into();
    let menu = |app: &CutCraftApp| {
        let (&id, component) = app.doc.components.iter().next().unwrap();
        gui::component_menu(&app.doc, id, component)
    };
    h.choose(menu, &["Move to Level", "Level 2"]);
    assert_eq!(h.app.doc.components[&EntityId(1)].level, 1);
    assert!(leaf(&menu(&h.app), &["Move to Level", "Level 2"]).is_checked);
}

#[test]
fn the_level_menu_selects_its_components_and_exports_them() {
    let mut h = Harness::new();
    h.app.doc = cut_craft_core::testing::two_components();
    let level_menu = |app: &CutCraftApp| {
        let (&id, level) = app.doc.levels.iter().next().unwrap();
        gui::level_menu(&app.doc, id, level)
    };
    h.choose(level_menu, &["Select Components"]);
    let all: Vec<EntityId> = h.app.doc.components.keys().copied().collect();
    assert_eq!(
        h.app
            .doc
            .selected_components
            .iter()
            .copied()
            .collect::<Vec<_>>(),
        all
    );
    let menu = level_menu(&h.app);
    let export = leaf(&menu, &["Export as 3D Clipart"]);
    assert!(export.is_enabled && h.app.available(&export.action));
}

#[test]
fn the_component_menu_acts_on_the_component_clicked() {
    use cut_craft_core::component::Combine;
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    let params = h.app.registry.get("create_shape").unwrap();
    let params = params.initial_params(&h.app.doc);
    h.app.apply("create_shape", &params).unwrap();
    let menu = |app: &CutCraftApp| {
        let (&id, component) = app.doc.components.iter().next().unwrap();
        gui::component_menu(&app.doc, id, component)
    };
    let component = |app: &CutCraftApp| app.doc.components.values().next().unwrap().clone();
    assert!(leaf(&menu(&h.app), &["Combine Mode", "Add"]).is_checked);
    h.choose(menu, &["Combine Mode", "Subtract"]);
    assert_eq!(component(&h.app).combine, Combine::Subtract);
    assert!(leaf(&menu(&h.app), &["Combine Mode", "Subtract"]).is_checked);
    h.choose(menu, &["Hide", "This"]);
    assert!(!component(&h.app).visible);
    h.choose(menu, &["Properties"]);
    let form = h.app.form.take().unwrap();
    assert_eq!(
        (form.id, form.params["combine"].as_str()),
        ("component_properties", Some("subtract"))
    );
    h.choose(menu, &["Delete"]);
    assert!(h.app.doc.components.is_empty());
}

#[test]
fn the_selection_menu_runs_edit_tools_on_the_selection() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 40.0, 40.0);
    h.rectangle(30.0, 30.0, 40.0, 40.0);
    h.app
        .apply("add_layer", &serde_json::json!({ "name": "Top" }))
        .unwrap();
    h.app.invoke(&h.ctx, "select_all_vectors");
    let menu = |app: &CutCraftApp| app.menus.selection_menu(&app.doc);
    h.choose(menu, &["Move to Layer", "Top"]);
    let doc = &h.app.doc;
    assert!(doc.vectors.keys().all(|&id| doc.layer_of(id) == LayerId(2)));
    h.choose(menu, &["Weld"]);
    assert_eq!(h.app.doc.vectors.len(), 1);
    h.choose(menu, &["Undo"]);
    assert_eq!(h.app.doc.vectors.len(), 2);
    h.choose(menu, &["Align", "Left"]);
    let left: Vec<f64> = h
        .app
        .doc
        .vectors
        .values()
        .map(|v| v.bounds().min.x)
        .collect();
    assert!((left[0] - left[1]).abs() < 1e-9, "{left:?}");
    h.choose(menu, &["Cut"]);
    assert!(h.app.doc.vectors.is_empty(), "Cut takes the selection away");
}

#[test]
fn right_clicking_the_view_opens_the_selection_menu() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 40.0, 40.0);
    h.rectangle(30.0, 30.0, 40.0, 40.0);
    h.app.invoke(&h.ctx, "select_all_vectors");
    h.right_click(h.at(90.0, 90.0));
    let weld = h.text_at("Weld").expect("the selection menu is open");
    h.click(weld, Modifiers::NONE);
    assert_eq!(h.app.doc.vectors.len(), 1, "{}", h.app.status.message);
}

#[test]
fn right_clicking_a_toolpath_selects_it_and_opens_its_menu() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    let params = h.app.registry.get("profile_toolpath").unwrap();
    let params = params.initial_params(&h.app.doc);
    h.app.apply("profile_toolpath", &params).unwrap();
    h.app.invoke(&h.ctx, "machining_layout");
    // the toolpath list's section settles its height over a frame or two
    h.frame(vec![]);
    h.frame(vec![]);
    let (&id, toolpath) = h.app.doc.toolpaths.iter().next().unwrap();
    let row = h
        .text_at(&toolpath.name.clone())
        .expect("the toolpath is listed");
    h.right_click(row);
    assert_eq!(h.app.selected_toolpath, Some(id));
    assert!(h.text_at("Recalculate").is_some(), "its menu is open");
}

#[test]
fn toolpath_forms_take_tools_from_the_database_and_save_them_back() {
    let mut h = Harness::new();
    let path =
        std::env::temp_dir().join(format!("cut-craft-tools-{}/tools.json", std::process::id()));
    h.app.tools_path = Some(path.clone());
    h.app.invoke(&h.ctx, "pocket_toolpath");
    h.app.choose_tool("3 mm End Mill");
    h.frame(vec![]);
    let params = &mut h.app.form.as_mut().unwrap().params;
    assert_eq!(params["tool"]["diameter"], 3.0);
    params["tool"]["name"] = "Mine".into();
    h.app.save_form_tool();
    assert_eq!(h.app.tools.get("Mine").map(|t| t.diameter), Some(3.0));
    assert_eq!(files::load_tools(&path).unwrap(), h.app.tools);
}

#[test]
fn a_tool_chosen_for_a_toolpath_cuts_with_the_active_material() {
    let mut h = Harness::new();
    let mut tool = h.app.tools.get("3 mm End Mill").unwrap().clone();
    tool.set_cutting(cut_craft_core::toolpath::CuttingData {
        material: "Oak".into(),
        feed_rate: 450.0,
        ..tool.cutting("")
    });
    h.app.tools.put(tool);
    h.app.tools.material = "Oak".into();
    h.app.invoke(&h.ctx, "pocket_toolpath");
    h.app.choose_tool("3 mm End Mill");
    assert_eq!(
        h.app.form.as_ref().unwrap().params["tool"]["feed_rate"],
        450.0
    );
}

/// A 20 × 10 mm, 5 mm high pyramid as an ASCII STL, in a file of its own.
fn pyramid_stl(name: &str) -> std::path::PathBuf {
    use std::fmt::Write as _;
    let base = [
        [0.0, 0.0, 0.0],
        [20.0, 0.0, 0.0],
        [20.0, 10.0, 0.0],
        [0.0, 10.0, 0.0],
    ];
    let top = [10.0, 5.0, 5.0];
    let mut faces: Vec<[[f64; 3]; 3]> = (0..4).map(|i| [base[i], base[(i + 1) % 4], top]).collect();
    faces.extend([[base[0], base[2], base[1]], [base[0], base[3], base[2]]]);
    let mut text = String::from("solid pyramid\n");
    for face in faces {
        text += "facet normal 0 0 0\n outer loop\n";
        for v in face {
            let _ = writeln!(text, "  vertex {} {} {}", v[0], v[1], v[2]);
        }
        text += " endloop\nendfacet\n";
    }
    text += "endsolid pyramid\n";
    let path = std::env::temp_dir().join(format!("cut-craft-{name}-{}.stl", std::process::id()));
    std::fs::write(&path, text).unwrap();
    path
}

#[test]
fn an_imported_model_shows_at_once_and_follows_its_form() {
    let mut h = Harness::new();
    let file = pyramid_stl("live-import");
    h.app.import_file(&file);
    assert_eq!(
        h.app.doc.components.len(),
        1,
        "issue #3: the model is in the scene at once"
    );
    let size = &h.app.form.as_ref().unwrap().params["size"];
    assert!(
        (size["x"].as_f64().unwrap() - 20.0).abs() < 1.0,
        "Model Size shows its size: {size}"
    );
    // moving it in the form moves the same model
    let form = h.app.form.as_mut().unwrap();
    form.params["placing"]["center"] = Value::Bool(false);
    form.params["placing"]["position"] = serde_json::json!({ "x": 50.0, "y": 60.0 });
    form.dirty = true;
    h.frame(vec![]);
    assert_eq!(h.app.doc.components.len(), 1, "moved, not added");
    let k = h.app.doc.components.values().next().unwrap();
    let o = h.app.doc.job.area().min;
    assert!(
        (k.origin.x - 50.0).abs() < 1.0 && (k.origin.y - 60.0).abs() < 1.0,
        "at {:?} of {o:?}",
        k.origin
    );
    // and sizing it sizes it
    let form = h.app.form.as_mut().unwrap();
    form.params["size"]["x"] = Value::from(40.0);
    form.dirty = true;
    h.frame(vec![]);
    let k = h.app.doc.components.values().next().unwrap();
    let width = k.cell * f64::from(u32::try_from(k.columns).unwrap());
    assert!((width - 40.0).abs() < 2.0 * k.cell, "{} columns", k.columns);
    let _ = std::fs::remove_file(file);
}

#[test]
fn a_drawn_shape_changes_with_its_form_until_it_is_let_go() {
    use cut_craft_core::geometry::Segment;
    let arcs = |app: &CutCraftApp| {
        let only: Vec<_> = app.doc.vectors.values().collect();
        assert_eq!(only.len(), 1, "one shape, not a second on top");
        only[0]
            .segments
            .iter()
            .filter(|s| matches!(s, Segment::Arc { .. }))
            .count()
    };
    let mut h = Harness::new();
    h.app.invoke(&h.ctx, "draw_rectangle");
    h.frame(vec![]);
    let (from, to) = (h.at(100.0, 100.0), h.at(200.0, 160.0));
    h.drag(from, to);
    assert_eq!(arcs(&h.app), 0, "drawn square");
    // issue #2: choosing rounded corners afterwards changes the rectangle just drawn, as
    // soon as it is chosen (issue #3), and Apply has nothing more to do
    let rounded = h.text_at("Rounded").expect("the corner choices");
    h.click(rounded, Modifiers::NONE);
    assert_eq!(arcs(&h.app), 4, "the same rectangle, rounded");
    let params = h.app.form.as_ref().unwrap().params.clone();
    h.app.apply_form("draw_rectangle", &params).unwrap();
    assert_eq!(arcs(&h.app), 4);
    let rect = h.app.doc.vectors.values().next().unwrap().bounds();
    // drawn from screen pixels: the same to a hundredth of a millimetre
    assert!((rect.size().x - 100.0).abs() < 1e-2, "same size: {rect:?}");
    h.app.invoke(&h.ctx, "undo");
    assert_eq!(arcs(&h.app), 0, "one undo puts the square back");
    // once let go, Apply draws a new shape
    h.app.invoke(&h.ctx, "redo");
    h.app.doc.selection.clear();
    h.app.apply_form("draw_rectangle", &params).unwrap();
    assert_eq!(h.app.doc.vectors.len(), 2);
}

#[test]
fn right_clicking_anywhere_offers_a_bug_report_saying_where() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    // the 2D view's own menu carries the item
    let canvas = h.at(80.0, 80.0);
    h.right_click(canvas);
    assert!(
        h.text_at("Report a Bug Here…").is_some(),
        "in the 2D view's menu"
    );
    h.frame(vec![Event::Key {
        key: egui::Key::Escape,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers::NONE,
    }]);
    h.frame(vec![]);
    // the status bar has no menu of its own: the small one shows
    let status = h.ctx.content_rect().left_bottom() + egui::vec2(5.0, -5.0);
    h.right_click(status);
    // a new menu is drawn unseen for a frame while egui sizes it
    h.frame(vec![]);
    let item = h.text_at("Report a Bug Here…").expect("the small menu");
    h.click(item, Modifiers::NONE);
    assert!(h.app.reporter.is_open(), "the form opens");
    assert!(h.text_at("Report a Bug").is_some());
    let state = h.app.bug_state();
    assert!(
        state.contains("Cut Craft") && state.contains("1 vectors"),
        "{state}"
    );
}

#[test]
fn the_tool_database_window_keeps_edits_only_on_ok() {
    let mut h = Harness::new();
    h.app.invoke(&h.ctx, "tool_database");
    h.frame(vec![]);
    let count = h.app.tools.tools.len();
    h.app.tool_manager.as_mut().unwrap().draft.tools.remove(0);
    h.frame(vec![]);
    assert_eq!(h.app.tools.tools.len(), count, "not until OK");
    let ok = *h.texts_at("OK").last().expect("the window has OK");
    h.click(ok, Modifiers::NONE);
    assert!(h.app.tool_manager.is_none());
    assert_eq!(h.app.tools.tools.len(), count - 1);
    h.app.invoke(&h.ctx, "tool_database");
    h.app.tool_manager.as_mut().unwrap().draft.tools.clear();
    h.frame(vec![]);
    let cancel = *h.texts_at("Cancel").last().expect("and Cancel");
    h.click(cancel, Modifiers::NONE);
    assert_eq!(h.app.tools.tools.len(), count - 1, "Cancel drops the edits");
}

#[test]
fn rest_machining_takes_its_tools_from_the_database() {
    let mut h = Harness::new();
    h.app.invoke(&h.ctx, "3d_finishing_toolpath");
    h.app.add_earlier_tool("6 mm Ball Nose");
    let form = h.app.form.as_ref().unwrap();
    // after the ball nose the form starts with
    assert_eq!(form.params["tools"][1]["name"], "6 mm Ball Nose");
}

#[test]
fn the_tool_database_window_shows_the_chip_load() {
    let mut h = Harness::new();
    h.app.tools = ToolDatabase {
        tools: vec![Tool::default()],
        ..ToolDatabase::default()
    };
    h.app.invoke(&h.ctx, "tool_database");
    h.app.tool_manager.as_mut().unwrap().pick = Some(cut_craft_tool_database::Pick::Tool(0));
    h.frame(vec![]);
    h.frame(vec![]);
    // 1000 mm/min over 18000 rpm x 2 flutes
    assert!(
        h.text_at("Chip load: 0.028 mm").is_some(),
        "the window shows the feed per tooth"
    );
}

#[test]
fn previewing_all_toolpaths_is_previewing_each_in_turn() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    for id in ["drilling_toolpath", "profile_toolpath"] {
        h.app.invoke(&h.ctx, id);
        let params = h.app.form.as_ref().unwrap().params.clone();
        h.app.apply(id, &params).unwrap();
    }
    h.app.invoke(&h.ctx, "preview_all_toolpaths");
    assert_eq!(h.app.view, View::ThreeD, "previewing shows the 3D view");
    h.frame(vec![]);
    let all = h.app.simulation.clone().expect("a preview");
    h.app.invoke(&h.ctx, "reset_preview");
    assert!(h.app.simulation.is_none());
    for id in h.app.doc.toolpaths.keys().copied().collect::<Vec<_>>() {
        h.app.selected_toolpath = Some(id);
        h.app.invoke(&h.ctx, "preview_toolpath");
    }
    assert_eq!(h.app.simulation, Some(all));
}

#[test]
fn the_3d_view_shows_the_model_until_there_is_a_preview() {
    let mut h = Harness::new();
    h.app.design_tab = DESIGN_TABS.iter().position(|&t| t == "Design").unwrap();
    assert!(
        h.app
            .toolbar
            .modeling_groups
            .iter()
            .any(|g| g.tools.contains(&"create_shape_round"))
    );
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    let shape = h.app.registry.get("create_shape").unwrap().default_params();
    h.app.apply("create_shape", &shape).unwrap();
    h.app.view = View::ThreeD;
    h.frame(vec![]);
    let raised = |app: &mut CutCraftApp| app.surface_3d().unwrap().heights.iter().any(|&z| z > 0.0);
    assert!(raised(&mut h.app), "the model");
    let profile = h
        .app
        .registry
        .get("profile_toolpath")
        .unwrap()
        .default_params();
    h.app.apply("profile_toolpath", &profile).unwrap();
    h.app.invoke(&h.ctx, "preview_all_toolpaths");
    h.frame(vec![]);
    assert!(!raised(&mut h.app), "the preview, cut down from the top");
}

#[test]
fn a_double_click_in_the_3d_view_picks_the_component_under_it() {
    let mut h = Harness::new();
    h.app.doc = cut_craft_core::testing::domed();
    h.app.orbit = view3d::Orbit::TOP;
    let fit = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(400.0, 400.0));
    let surface = h.app.surface_3d().unwrap().into_owned();
    let screen = view3d::screen_of(&surface, h.app.orbit, h.app.framing, fit, None);
    let local = |x: f64, y: f64| {
        let z = surface
            .at(cut_craft_core::geometry::Point::new(x, y))
            .unwrap();
        cut_craft_core::glam::DVec3::new(x - surface.origin.x, y - surface.origin.y, z)
    };
    assert_eq!(
        h.app.component_at_3d(fit, screen(local(20.0, 20.0))),
        Some(EntityId(1))
    );
    assert_eq!(
        h.app.component_at_3d(fit, screen(local(2.0, 2.0))),
        None,
        "off the dome"
    );
}

#[test]
fn the_window_is_titled_cut_craft() {
    let h = Harness::new();
    assert_eq!(h.app.title, "Untitled - Cut Craft");
}

#[test]
fn the_align_objects_group_spaces_by_pick_order() {
    let mut h = Harness::new();
    h.rectangle(100.0, 10.0, 10.0, 10.0);
    let first = *h.app.doc.selection.iter().next().unwrap();
    h.rectangle(10.0, 10.0, 10.0, 10.0);
    h.rectangle(50.0, 10.0, 10.0, 10.0);
    let all: Vec<EntityId> = h.app.doc.vectors.keys().copied().collect();
    h.app.doc.selection = all.into_iter().collect();
    for id in [
        "left_to_right",
        "right_to_left",
        "top_to_bottom",
        "bottom_to_top",
    ] {
        assert!(h.app.available(id), "{id}");
    }
    h.app.invoke(&h.ctx, "left_to_right");
    let left = h.app.doc.vectors[&first].bounds().min.x;
    assert!(
        (left - 10.0).abs() < 1e-6,
        "the first picked goes leftmost: {left}"
    );
    let group = h
        .app
        .toolbar
        .groups
        .iter()
        .find(|g| g.name == "Align Objects")
        .unwrap();
    assert!(group.tools.contains(&"left_to_right") && group.tools.contains(&"align_left"));
}

#[test]
fn the_create_shape_icons_open_it_with_their_profile() {
    let mut h = Harness::new();
    for (id, profile) in [
        ("create_shape_round", "round"),
        ("create_shape_angled", "angular"),
        ("create_shape_smooth", "smooth"),
        ("create_shape_flat", "flat"),
    ] {
        h.app.invoke(&h.ctx, id);
        let form = h.app.form.as_ref().expect("Create Shape's form");
        assert_eq!(
            (form.id, form.params["profile"].as_str()),
            ("create_shape", Some(profile))
        );
    }
}

#[test]
fn edit_toolpath_reopens_its_form_and_replaces_it() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    h.app.invoke(&h.ctx, "profile_toolpath");
    let params = h.app.form.as_ref().unwrap().params.clone();
    h.app.apply("profile_toolpath", &params).unwrap();
    h.app.form = None;
    let id = *h.app.doc.toolpaths.keys().next().unwrap();
    h.app.selected_toolpath = Some(id);
    h.app.doc.selection = cut_craft_core::selection::Selection::new();
    h.app.invoke(&h.ctx, "edit_toolpath");
    let form = h.app.form.as_mut().expect("the toolpath's form");
    assert_eq!((form.id, form.editing), ("profile_toolpath", Some(id)));
    form.params["name"] = Value::from("Edited");
    let params = form.params.clone();
    h.app.apply_form("profile_toolpath", &params).unwrap();
    assert_eq!(h.app.doc.toolpaths.len(), 1, "replaced, not added");
    assert_eq!(h.app.doc.toolpaths[&id].name, "Edited");
}

#[test]
fn the_file_and_component_level_icons_have_their_tools() {
    let mut h = Harness::new();
    for id in [
        "import_vectors",
        "export_vectors",
        "open_folder",
        "open_clipart_folder",
        "component_levels",
    ] {
        assert!(h.app.available(id), "{id}");
    }
    let groups = &h.app.toolbar.groups;
    let file = groups.iter().find(|g| g.name == "File Operations").unwrap();
    assert!(file.tools.contains(&"export_vectors") && file.tools.contains(&"open_folder"));
    h.app.invoke(&h.ctx, "component_levels");
    assert_eq!(DESIGN_TABS[h.app.design_tab], "Components");
    // with no library folder there is nothing to open, and it says so
    h.app.invoke(&h.ctx, "open_clipart_folder");
    assert!(
        h.app.status.message.contains("Clipart library"),
        "{}",
        h.app.status.message
    );
}

#[test]
fn a_collapsed_level_hides_its_components() {
    let mut h = Harness::new();
    h.app.doc = cut_craft_core::testing::domed();
    h.app.pages.set_pinned(Page::Design, true);
    h.app.design_tab = DESIGN_TABS.iter().position(|&t| t == "Components").unwrap();
    h.frame(vec![]);
    let k = h.app.doc.components.values().next().unwrap();
    let name = format!("{} ({:?})", k.name, k.combine);
    assert!(h.text_at(&name).is_some(), "{name} listed");
    let level = *h.app.doc.levels.keys().next().unwrap();
    h.app.collapsed_levels.insert(level);
    h.frame(vec![]);
    assert!(
        h.text_at(&name).is_none(),
        "hidden under its collapsed level"
    );
}

#[test]
fn the_join_icons_join_two_vectors_their_way() {
    let mut h = Harness::new();
    let line =
        |x0: f64, x1: f64| Path::polyline(&[Point::new(x0, 10.0), Point::new(x1, 10.0)], false);
    let change = Change::swap(&h.app.doc, [], vec![line(0.0, 10.0), line(20.0, 30.0)]);
    h.app.doc.apply(&change).unwrap();
    for id in [
        "join_straight_line",
        "join_smooth_curve",
        "join_move_end_points",
    ] {
        assert!(h.app.available(id), "{id}");
    }
    h.app.invoke(&h.ctx, "join_smooth_curve");
    assert_eq!(h.app.doc.vectors.len(), 1, "{}", h.app.status.message);
    let edit = h
        .app
        .toolbar
        .groups
        .iter()
        .find(|g| g.name == "Edit Objects")
        .unwrap();
    assert!(edit.tools.contains(&"join_move_end_points"));
}

#[test]
fn edit_text_opens_draw_text_on_the_selected_text() {
    let mut h = Harness::new();
    assert!(!h.app.available("edit_text"), "no text selected");
    let params = serde_json::json!({ "text": "HI", "font": "Test Boxes" });
    let mut full = h.app.registry.get("draw_text").unwrap().default_params();
    for (k, v) in params.as_object().unwrap() {
        full[k] = v.clone();
    }
    h.app.apply("draw_text", &full).unwrap();
    assert!(h.app.available("edit_text"));
    h.app.invoke(&h.ctx, "edit_text");
    let form = h.app.form.as_ref().expect("Draw Text's form");
    assert_eq!(
        (form.id, form.params["text"].as_str()),
        ("draw_text", Some("HI"))
    );
}

#[test]
fn a_new_job_is_set_up_over_the_2d_view() {
    let mut h = Harness::at_start();
    h.app.view = View::ThreeD;
    h.app.invoke(&h.ctx, "new");
    assert!(
        h.app.view == View::TwoD,
        "the material shows while it is set up"
    );
}

#[test]
fn it_starts_on_the_start_page_with_the_no_job_menus() {
    let h = Harness::at_start();
    assert_eq!(h.app.screen, Screen::Start);
    assert_eq!(h.app.title, "Cut Craft", "no job, so no file name");
    assert_eq!(
        h.app.menu_names(),
        ["File", "Edit", "Machine", "Toolpaths", "Gadgets", "Help"]
    );
    assert!(
        h.app.view_transform.is_none(),
        "no 2D view before there is a job"
    );
}

#[test]
fn create_a_new_file_goes_through_job_setup_to_the_job() {
    let mut h = Harness::at_start();
    h.app.invoke(&h.ctx, "new");
    h.frame(vec![]);
    assert_eq!(h.app.screen, Screen::NewJob);
    let form = h.app.form.as_mut().expect("the Job Setup form");
    assert_eq!(form.id, "job_setup");
    form.params["size"]["width"] = 300.0.into();
    assert!(
        (h.app.shown_job().width - 300.0).abs() < 1e-9,
        "the 2D view shows the job being set up"
    );
    h.app.job_setup_ok();
    h.frame(vec![]);
    assert_eq!(h.app.screen, Screen::Job);
    assert!(h.app.form.is_none());
    assert!((h.app.doc.job.width - 300.0).abs() < 1e-9);
    assert!(
        !h.app.history.can_undo(),
        "a new job starts with nothing to undo"
    );
    assert_eq!(
        h.app.status.current_tool, "Select",
        "Job Setup is done with"
    );
    assert!(h.app.menu_names().contains(&"Model"));
    assert!(h.app.view_transform.is_some());
}

#[test]
fn the_start_page_says_whats_new_and_links_out() {
    let h = Harness::at_start();
    let heading = format!("What's new in v{}", env!("CARGO_PKG_VERSION"));
    assert!(h.text_at(&heading).is_some(), "{heading}");
    let first = WHATS_NEW.lines().find(|l| !l.trim().is_empty()).unwrap();
    assert!(
        h.text_at(first.trim_start_matches("- ")).is_some(),
        "{first}"
    );
    for (label, url, _) in LINKS {
        assert!(h.text_at(label).is_some(), "{label}");
        assert!(url.starts_with("https://"), "{url}");
    }
}

#[test]
fn cancelling_job_setup_goes_back_to_the_start_page() {
    let mut h = Harness::at_start();
    h.app.invoke(&h.ctx, "new");
    h.app.job_setup_cancel();
    h.frame(vec![]);
    assert_eq!(h.app.screen, Screen::Start);
    assert!(h.app.form.is_none());
}

/// The icons built into the program.
fn icon_set() -> std::collections::HashSet<String> {
    crate::icons::built_in::BUILT_IN
        .iter()
        .map(|(key, _)| (*key).to_owned())
        .collect()
}

#[test]
fn start_page_and_form_pictures_are_in_the_icon_set() {
    let known = icon_set();
    for (_, _, icon) in STARTUP_TASKS {
        assert!(known.contains(icon), "{icon}");
    }
    assert!(known.contains(RECENT_FILE_ICON));
    // every picture these forms can show, whatever is chosen
    let forms = [
        "job_setup",
        "orientate_3d_model",
        "draw_rectangle",
        "draw_circle",
        "draw_polygon",
        "draw_star",
    ];
    let fields: Vec<form::Field> = forms
        .iter()
        .flat_map(|id| form::fields(&commands::registry().get(id).unwrap().schema()))
        .collect();
    let mut pictures = 0;
    for f in &fields {
        let Some(template) = &f.icon else { continue };
        let choices = |kind: &form::Kind| match kind {
            form::Kind::Choice(options, _) => options.iter().map(|o| o.value.clone()).collect(),
            _ => vec![],
        };
        let values: Vec<Value> = match &f.kind {
            form::Kind::Choice(..) => choices(&f.kind).into_iter().map(Value::from).collect(),
            form::Kind::Group(inner) => {
                let each: Vec<Value> = inner
                    .iter()
                    .flat_map(|i| {
                        choices(&i.kind)
                            .into_iter()
                            .map(|c| serde_json::json!({ i.key.as_str(): c }))
                    })
                    .collect();
                // a section of numbers only has the one picture
                if each.is_empty() {
                    vec![Value::Null]
                } else {
                    each
                }
            }
            _ => vec![Value::Null],
        };
        for value in values {
            let path = form::icon_path(template, &value);
            if !path.contains('{') {
                assert!(known.contains(&path), "{} shows {path}", f.key);
                pictures += 1;
            }
        }
    }
    assert!(
        pictures >= 9 + 3 + 12,
        "Job Setup's, Orientate 3D Model's and the shape tools' pictures: {pictures}"
    );
}

#[test]
fn closing_the_job_goes_back_to_the_start_page() {
    let mut h = Harness::new();
    h.app.invoke(&h.ctx, "close");
    h.frame(vec![]);
    assert_eq!(h.app.screen, Screen::Start);
}

#[test]
fn saved_and_opened_jobs_are_listed_as_recent_files() {
    let mut h = Harness::new();
    let dir = std::env::temp_dir().join(format!("cut-craft-recent-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    h.app.recent_path = Some(dir.join("recent.json"));
    let job = dir.join("first.ccjob");
    h.app.save_to(&job);
    assert_eq!(h.app.recent, std::slice::from_ref(&job));
    assert_eq!(
        files::load_recent(&dir.join("recent.json")),
        std::slice::from_ref(&job)
    );
    h.app.invoke(&h.ctx, "close");
    h.app.invoke(&h.ctx, "open_recent:0");
    assert_eq!(h.app.screen, Screen::Job);
    assert_eq!(h.app.session.path.as_deref(), Some(job.as_path()));
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_recent_file_that_has_gone_is_dropped() {
    let mut h = Harness::at_start();
    h.app.recent = vec![std::env::temp_dir().join("cut-craft-gone.ccjob")];
    h.app.invoke(&h.ctx, "open_recent:0");
    assert_eq!(h.app.screen, Screen::Start);
    assert_eq!(h.app.recent, Vec::<std::path::PathBuf>::new());
    assert!(h.app.status.message.contains("cut-craft-gone.ccjob"));
}

/// The middle of the 2D view on screen.
fn view_middle(h: &Harness) -> Pos2 {
    h.app
        .view_area
        .expect("the 2D view has been drawn")
        .center()
}

#[test]
fn the_wheel_zooms_about_the_cursor() {
    let mut h = Harness::new();
    let at = view_middle(&h) + vec2(40.0, -30.0);
    let (before, under) = {
        let t = h.app.view_transform.unwrap();
        (t.scale, t.to_job(at))
    };
    h.frame(vec![Event::PointerMoved(at)]);
    h.frame(vec![Event::MouseWheel {
        unit: egui::MouseWheelUnit::Point,
        delta: vec2(0.0, 120.0),
        phase: egui::TouchPhase::Move,
        source: egui::MouseWheelSource::Wheel,
        modifiers: Modifiers::NONE,
    }]);
    h.frame(vec![]);
    let t = h.app.view_transform.unwrap();
    assert!(t.scale > before, "scrolling up zooms in");
    assert!(t.to_job(at).distance(under) < 0.01, "about the cursor");
}

#[test]
fn the_middle_button_and_the_pan_tool_pan() {
    let mut h = Harness::new();
    let p = Point::new(100.0, 100.0);
    let before = h.app.view_transform.unwrap().to_screen(p);
    let from = view_middle(&h);
    h.frame(vec![Event::PointerMoved(from)]);
    let middle = |pressed: bool| Event::PointerButton {
        pos: from + vec2(40.0, 10.0) * f32::from(u8::from(!pressed)),
        button: PointerButton::Middle,
        pressed,
        modifiers: Modifiers::NONE,
    };
    h.frame(vec![middle(true)]);
    h.frame(vec![Event::PointerMoved(from + vec2(20.0, 5.0))]);
    h.frame(vec![Event::PointerMoved(from + vec2(40.0, 10.0))]);
    h.frame(vec![middle(false)]);
    h.frame(vec![]);
    let after = h.app.view_transform.unwrap().to_screen(p);
    assert!(
        (after - before - vec2(40.0, 10.0)).length() < 1.0,
        "{before:?} → {after:?}"
    );
    // the Pan tool does the same with the left button, and draws nothing
    h.app.invoke(&h.ctx, "pan");
    let before = h.app.view_transform.unwrap().to_screen(p);
    h.drag(from, from + vec2(-30.0, 0.0));
    let after = h.app.view_transform.unwrap().to_screen(p);
    assert!(
        (after - before - vec2(-30.0, 0.0)).length() < 1.0,
        "{before:?} → {after:?}"
    );
    assert!(h.app.doc.vectors.is_empty() && h.app.doc.selection.is_empty());
}

#[test]
fn zoom_box_zoom_to_selection_and_zoom_to_job() {
    let mut h = Harness::new();
    let fitted = h.app.view_transform.unwrap().scale;
    h.app.invoke(&h.ctx, "zoom_box");
    h.drag(h.at(100.0, 100.0), h.at(200.0, 150.0));
    let boxed = h.app.view_transform.unwrap();
    assert!(boxed.scale > 3.0 * fitted, "the box fills the view");
    assert!(h.app.doc.selection.is_empty(), "zooming selects nothing");
    let area = h.app.view_area.unwrap();
    assert!(area.contains(boxed.to_screen(Point::new(150.0, 125.0))));
    h.rectangle(400.0, 300.0, 20.0, 10.0);
    h.app.invoke(&h.ctx, "zoom_selected");
    h.frame(vec![]);
    let t = h.app.view_transform.unwrap();
    let (a, b) = (
        t.to_screen(Point::new(400.0, 300.0)),
        t.to_screen(Point::new(420.0, 310.0)),
    );
    assert!(area.contains(a) && area.contains(b));
    assert!(
        (b.x - a.x) > area.width() / 2.0,
        "the selection fills the view"
    );
    h.app.invoke(&h.ctx, "zoom_to_job");
    h.frame(vec![]);
    assert!((h.app.view_transform.unwrap().scale - fitted).abs() < 1e-3);
}

#[test]
fn the_side_tabs_are_aspire_12s_with_drawing_and_modeling_on_design() {
    assert_eq!(
        DESIGN_TABS,
        ["Design", "Sheets", "Layers", "Components", "Clipart"]
    );
    let h = Harness::new();
    let names: Vec<&str> = h
        .app
        .design_groups("Design")
        .iter()
        .map(|g| g.name)
        .collect();
    assert!(
        names.contains(&"Create Vectors") && names.contains(&"Shape Creation"),
        "{names:?}"
    );
    assert!(h.app.design_groups("Components").is_empty());
}

#[test]
fn the_components_tab_is_the_component_list() {
    let mut h = Harness::new();
    h.app.pages = Pages::design_layout();
    h.app.design_tab = DESIGN_TABS.iter().position(|&t| t == "Components").unwrap();
    h.frame(vec![]);
    assert!(h.text_at("No components yet").is_some());
}

#[test]
fn the_view_buttons_sit_at_the_top_right_of_the_2d_view_starting_with_pan() {
    let mut h = Harness::new();
    // text buttons, as where the icons are missing
    h.app.icons = Icons::load(std::path::Path::new("/no/icons"));
    h.frame(vec![]);
    h.frame(vec![]);
    let area = h.app.view_area.unwrap();
    let over_view = |at: &Pos2| area.contains(*at);
    let pan = *h
        .texts_at("Pan")
        .iter()
        .find(|p| over_view(p))
        .expect("Pan over the view");
    let selection = h.text_at("Zoom to Selection").unwrap();
    assert!(
        pan.x > area.center().x && selection.x > pan.x,
        "on the right, Pan first: {pan:?} {selection:?} in {area:?}"
    );
    assert!(pan.y < area.center().y, "along the top: {pan:?}");
    h.click(pan, Modifiers::NONE);
    assert_eq!(h.app.view_tool, Some(ViewTool::Pan));
}

#[test]
fn the_view_buttons_are_the_view_actions() {
    for (id, _, icon) in VIEW_BUTTONS {
        assert!(icon_set().contains(icon), "{icon}");
        assert!(BUILT_INS.contains(&id), "{id}");
    }
}

fn key(key: Key) -> Event {
    Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers::NONE,
    }
}

#[test]
fn tab_moves_a_visible_focus_through_the_controls() {
    let mut h = Harness::new();
    h.frame(vec![]);
    assert_eq!(focus_ring(&h.ctx), None, "nothing focused yet");
    h.frame(vec![key(Key::Tab)]);
    h.frame(vec![]);
    let first = focus_ring(&h.ctx).expect("Tab focuses a control, ringed");
    h.frame(vec![key(Key::Tab)]);
    h.frame(vec![]);
    let second = focus_ring(&h.ctx).expect("the next control");
    assert_ne!(first, second, "Tab moves on");
}

#[test]
fn a_preview_plays_pauses_scrubs_and_keeps_what_is_cut() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    h.app.invoke(&h.ctx, "profile_toolpath");
    let params = h.app.form.as_ref().unwrap().params.clone();
    h.app.apply("profile_toolpath", &params).unwrap();
    h.app.form = None;
    h.app.display.animate_preview = true;
    h.app.invoke(&h.ctx, "preview_all_toolpaths");
    assert!(h.app.playing && h.app.playback.is_some(), "it plays");
    assert_eq!(h.app.view, View::ThreeD);
    h.frame(vec![]);
    h.frame(vec![]);
    let along = h.app.playback.as_ref().unwrap().at();
    assert!(along > 0.0, "the tool moves on each frame");
    assert!(h.text_at("Pause").is_some() && h.text_at("Run to Retract").is_some());
    h.click(h.text_at("Pause").unwrap(), Modifiers::NONE);
    assert!(!h.app.playing);
    let paused = h.app.playback.as_ref().unwrap().at();
    h.frame(vec![]);
    assert_eq!(
        h.app.playback.as_ref().unwrap().at(),
        paused,
        "paused stays put"
    );
    h.click(h.text_at("Stop").unwrap(), Modifiers::NONE);
    assert!(
        h.app.playback.is_none() && h.app.simulation.is_some(),
        "kept as far as it went"
    );
}

#[test]
fn any_toolpath_rest_machines_after_the_tools_named_in_its_form() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 40.0, 40.0);
    h.app.invoke(&h.ctx, "pocket_toolpath");
    // the section, drawn on its own (it is below the fold of the pocket form)
    let ctx = egui::Context::default();
    let mut section = serde_json::json!({});
    let out = headless(ctx.run_ui(RawInput::default(), |ui| {
        egui::CentralPanel::default()
            .show(ui, |ui| rest_section(ui, &mut section, &h.app.tools.tools));
    }));
    let drawn = |text: &str| {
        out.shapes
            .iter()
            .any(|c| matches!(&c.shape, egui::Shape::Text(t) if t.galley.text() == text))
    };
    assert!(drawn("Rest Machining") && drawn("Add an earlier tool"));
    let mut params = h.app.form.as_ref().unwrap().params.clone();
    h.app.apply("pocket_toolpath", &params).unwrap();
    let whole = h.app.doc.toolpaths.values().last().unwrap().moves.len();
    let mut big = h.app.form.as_ref().unwrap().params["tool"].clone();
    big["diameter"] = Value::from(params["tool"]["diameter"].as_f64().unwrap() * 4.0);
    params["rest"] = serde_json::json!({ "after": [big], "minimum_detail": 0.0, "overlap": 0.0 });
    h.app.apply("pocket_toolpath", &params).unwrap();
    let rest = h.app.doc.toolpaths.values().last().unwrap();
    assert!(
        rest.moves.len() < whole,
        "{} of {whole} moves",
        rest.moves.len()
    );
    assert!(
        cut_craft_core::rest::Rest::of(&rest.params).is_some(),
        "kept to recalculate"
    );
}

#[test]
fn the_arrow_keys_nudge_the_selection() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 20.0, 20.0);
    let square = h.app.snap.grid_spacing;
    let at = |h: &Harness| h.app.doc.selection_bounds().unwrap().min;
    let before = at(&h);
    h.frame(vec![key(Key::ArrowRight)]);
    h.frame(vec![key(Key::ArrowUp)]);
    let moved = at(&h) - before;
    assert!(
        (moved.x - square).abs() < 1e-9 && (moved.y - square).abs() < 1e-9,
        "{moved:?}"
    );
    h.frame(vec![key(Key::ArrowLeft), key(Key::ArrowDown)]);
    assert!((at(&h) - before).length() < 1e-9, "back where it was");
    for (modifiers, share) in [
        (Modifiers::SHIFT, 0.5),
        (Modifiers::SHIFT | Modifiers::COMMAND, 0.1),
    ] {
        let from = at(&h);
        let press = Event::Key {
            key: Key::ArrowRight,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers,
        };
        h.frame_holding(modifiers, vec![press]);
        let moved = (at(&h) - from).x;
        assert!(
            (moved - square * share).abs() < 1e-9,
            "{modifiers:?}: {moved}"
        );
    }
}

#[test]
fn f11_designs_f12_machines_and_switch_layout_swaps() {
    let mut h = Harness::new();
    assert_eq!(h.app.pages, Pages::design_layout());
    h.frame(vec![key(Key::F12)]);
    assert_eq!(h.app.pages, Pages::machining_layout());
    h.frame(vec![key(Key::F11)]);
    assert_eq!(h.app.pages, Pages::design_layout());
    h.app.invoke(&h.ctx, "interface_layout");
    assert_eq!(
        h.app.pages,
        Pages::machining_layout(),
        "the Switch Layout button"
    );
}

#[test]
fn a_tool_from_a_flown_out_page_hides_it_unless_it_opens_a_form() {
    let mut h = Harness::new();
    h.app.pages = Pages::machining_layout();
    h.app.pages.open(Page::Design);
    h.app.pick(&h.ctx, Page::Design, "draw_rectangle");
    assert_eq!(
        h.app.pages.shown(Page::Design),
        Shown::Flyout,
        "its form shows there"
    );
    h.app.form = None;
    h.app.pick(&h.ctx, Page::Design, "select_all_vectors");
    assert_eq!(h.app.pages.shown(Page::Design), Shown::Hidden);
}

#[test]
fn a_tools_form_takes_the_place_of_the_page_as_in_aspire() {
    let mut h = Harness::new();
    h.app.pages.set_pinned(Page::Design, true);
    h.app.pages.set_pinned(Page::Toolpaths, true);
    h.frame(vec![]);
    assert!(h.text_at("Create Vectors").is_some());
    assert!(h.text_at("Toolpath List").is_some());
    h.app.invoke(&h.ctx, "draw_circle");
    h.frame(vec![]);
    assert!(h.text_at("Create Vectors").is_none(), "design tools hidden");
    assert!(h.text_at("Toolpath List").is_some(), "the other page stays");
    h.app.invoke(&h.ctx, "profile_toolpath");
    h.frame(vec![]);
    assert!(
        h.text_at("Create Vectors").is_some(),
        "back once its form went"
    );
    assert!(h.text_at("Toolpath List").is_none(), "toolpath page hidden");
}

#[test]
fn the_sheets_tab_lists_each_sheets_size_and_activates_one() {
    use cut_craft_core::document::Units;
    let mut h = Harness::new();
    h.app.pages.set_pinned(Page::Design, true);
    h.app.design_tab = 1;
    h.app.doc = cut_craft_core::testing::two_sheets();
    // the list's grid measures its columns on the first frame
    h.frame(vec![]);
    h.frame(vec![]);
    assert!(h.text_at("Sheets List").is_some());
    assert!(h.text_at("Sheet 2").is_some());
    assert!(
        h.text_at(&sheet_length(300.0, Units::Mm)).is_some(),
        "Sheet 2's X"
    );
    assert!(
        h.text_at(&sheet_length(12.0, Units::Mm)).is_some(),
        "Sheet 2's Z"
    );
    assert!(h.text_at("Add New Sheet...").is_some());
    let at = h.text_at("Sheet 2").unwrap();
    h.click(at, Modifiers::NONE);
    assert_eq!(h.app.doc.active_sheet, 1, "clicking a sheet activates it");
    assert_eq!(h.app.doc.job.width, 300.0);
}

#[test]
fn the_clipart_tab_shows_a_library_folder_and_imports_what_is_double_clicked() {
    let dir = std::env::temp_dir().join(format!("cut-craft-clipart-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("flower.bmp"), crate::icons::tiny_bmp()).unwrap();
    std::fs::write(dir.join("notes.txt"), "not clipart").unwrap();
    let mut h = Harness::new();
    h.app.pages.set_pinned(Page::Design, true);
    h.app.design_tab = 4;
    h.app.clipart.folders = vec![dir.clone()];
    h.app.clipart.chosen = Some(0);
    h.frame(vec![]);
    h.frame(vec![]);
    let folder = dir.file_name().unwrap().to_string_lossy().into_owned();
    assert!(h.text_at(&folder).is_some(), "the library lists the folder");
    assert!(
        h.text_at("notes.txt").is_none(),
        "only what can be imported"
    );
    let at = h
        .text_at("flower.bmp")
        .expect("the picture is in the browser");
    h.double_click(at);
    assert_eq!(h.app.doc.bitmaps.len(), 1, "double-clicking imports it");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_top_bar_splits_the_views_and_toggles_snapping() {
    let mut h = Harness::new();
    h.app.doc = cut_craft_core::testing::two_sheets();
    h.frame(vec![]);
    for name in ["Sheet 1", "Layer 1", "Level 1"] {
        assert!(h.text_at(name).is_some(), "{name} in the top bar");
    }
    h.app.invoke(&h.ctx, "split_side_by_side");
    assert_eq!(h.app.split, Some(Split::SideBySide));
    h.frame(vec![]);
    h.app.invoke(&h.ctx, "split_side_by_side");
    assert_eq!(h.app.split, None, "a second click joins them again");
    let before = h.app.snap.clone();
    for id in [
        "toggle_geometry_snapping",
        "toggle_smart_snapping",
        "toggle_grid_snapping",
    ] {
        assert!(h.app.available(id), "{id}");
        h.app.invoke(&h.ctx, id);
    }
    let s = &h.app.snap;
    assert_eq!(
        (s.geometry_snapping, s.smart_snapping, s.snap_to_grid),
        (
            !before.geometry_snapping,
            !before.smart_snapping,
            !before.snap_to_grid
        )
    );
}

#[test]
fn toolpath_forms_open_in_the_toolpaths_page() {
    let mut h = Harness::new();
    h.app.invoke(&h.ctx, "profile_toolpath");
    assert_eq!(h.app.form.as_ref().unwrap().page, Page::Toolpaths);
    assert_eq!(
        h.app.pages.shown(Page::Toolpaths),
        Shown::Flyout,
        "brought out to show its form"
    );
    h.app.invoke(&h.ctx, "draw_circle");
    assert_eq!(h.app.form.as_ref().unwrap().page, Page::Design);
}

#[test]
fn clicking_the_view_puts_a_flown_out_page_away() {
    let mut h = Harness::new();
    h.app.pages.open(Page::Toolpaths);
    h.frame(vec![]);
    h.click(h.at(20.0, 200.0), Modifiers::NONE);
    assert_eq!(h.app.pages.shown(Page::Toolpaths), Shown::Hidden);
    assert_eq!(h.app.pages.shown(Page::Design), Shown::Docked);
}

#[test]
fn menu_items_that_reuse_existing_tools() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 20.0, 20.0);
    h.app
        .apply("add_layer", &serde_json::json!({ "name": "Top" }))
        .unwrap();
    h.rectangle(50.0, 10.0, 20.0, 20.0);
    let on_top = *h.app.doc.selection.iter().next().unwrap();
    h.app.invoke(&h.ctx, "select_all_vectors");
    h.app.invoke(&h.ctx, "select_all_vectors_on_current_layer");
    assert_eq!(
        h.app.doc.selection,
        [on_top].into(),
        "Select Vectors on Layer, current layer"
    );
    h.app.invoke(&h.ctx, "job_size_and_position");
    assert_eq!(h.app.form.as_ref().map(|f| f.id), Some("job_setup"));
    h.app.invoke(&h.ctx, "show_toolpaths_tab");
    assert_ne!(h.app.pages.shown(Page::Toolpaths), Shown::Hidden);
    h.app.invoke(&h.ctx, "zoom_box");
    h.app.invoke(&h.ctx, "select");
    assert!(
        h.app.form.is_none() && h.app.view_tool.is_none(),
        "the Select tool"
    );
}

#[test]
fn run_and_form_items_apply_or_open_a_command_with_their_params() {
    let mut h = Harness::new();
    let add = gui::MenuItem::run::<cmd_add_layer::AddLayer>(
        "Add",
        &cmd_add_layer::Params { name: "Top".into() },
    );
    assert!(h.app.available(&add.action));
    h.app.invoke(&h.ctx, &add.action);
    assert_eq!(
        h.app.doc.layers.last().map(|l| l.name.as_str()),
        Some("Top")
    );
    let mut rename: cmd_layer_properties::Params = h.app.doc.layers[0].clone().into();
    rename.name = "Renamed".into();
    let rename = gui::MenuItem::form::<cmd_layer_properties::LayerProperties>("Rename", &rename);
    h.app.invoke(&h.ctx, &rename.action);
    let form = h.app.form.as_ref().unwrap();
    assert_eq!(
        (form.id, form.params["name"].as_str()),
        ("layer_properties", Some("Renamed"))
    );
    assert!(!h.app.available("run:no_such_command:{}"));
}

#[test]
fn the_model_resolution_is_changed_from_the_model_menu() {
    use cut_craft_core::document::Resolution;
    let mut h = Harness::new();
    assert_eq!(h.app.doc.job.resolution, Resolution::Standard);
    assert_eq!(
        h.app.resolution_checked("model_resolution_standard"),
        Some(true)
    );
    assert!(h.app.available("model_resolution_highest"));
    let standard = h.app.doc.job.model_cell();
    h.app.invoke(&h.ctx, "model_resolution_highest");
    assert_eq!(h.app.doc.job.resolution, Resolution::Highest);
    assert!(
        h.app.doc.job.model_cell() < standard / 2.0,
        "finer cells for new models"
    );
    assert_eq!(
        h.app.resolution_checked("model_resolution_highest"),
        Some(true)
    );
    assert_eq!(
        h.app.resolution_checked("model_resolution_standard"),
        Some(false)
    );
    assert!(
        h.app.status.message.contains("Highest"),
        "{}",
        h.app.status.message
    );
    h.app.invoke(&h.ctx, "undo");
    assert_eq!(h.app.doc.job.resolution, Resolution::Standard, "undone");
    assert_eq!(h.app.resolution_checked("preview_quality_1"), None);
}

#[test]
fn a_still_3d_view_draws_the_model_at_its_own_detail() {
    use cut_craft_core::component::{Combine, Component};
    let mut h = Harness::new();
    let o = h.app.doc.job.area().min;
    let rect = cut_craft_core::geometry::Rect::new(
        o + Point::new(150.0, 100.0),
        o + Point::new(450.0, 250.0),
    );
    let cell = h.app.doc.job.model_cell();
    let mut c = Change::builder(&h.app.doc);
    c.add_component(Component::from_fn(
        "relief",
        Combine::Add,
        rect,
        cell,
        |p| Some((p.x * 0.5).sin() + 2.0),
    ));
    h.app.record(Ok(c.finish()), "added");
    h.app.view = View::ThreeD;
    h.frame(vec![]);
    // a moment after anything moved, the view settles and is drawn finely
    h.time += 1.0;
    h.frame(vec![]);
    let fine = h
        .app
        .fine_3d
        .as_ref()
        .map(|(_, mesh)| mesh.indices.len() / 3);
    let (key, _) = h.app.fine_3d.as_ref().unwrap();
    let quick = view3d::mesh(
        &cut_craft_core::component::model_shape(&h.app.doc, MESH_CELLS),
        key.orbit,
        key.framing,
        &key.look,
        key.fit,
    );
    let quick = quick.indices.len() / 3;
    // issue: the mermaid had no detail, drawn at 2.4 mm on a 600 mm job
    assert!(
        fine.is_some_and(|t| t > 4 * quick),
        "fine {fine:?} against {quick} quick triangles"
    );
    let kept = h.app.fine_3d.as_ref().map(|(key, _)| key.clone());
    h.time += 1.0;
    h.frame(vec![]);
    assert_eq!(
        h.app.fine_3d.as_ref().map(|(key, _)| key.clone()),
        kept,
        "kept, not rebuilt"
    );
    // an edit to the model makes it again
    let mut c = Change::builder(&h.app.doc);
    c.add_component(Component::from_fn("dome", Combine::Add, rect, cell, |_| {
        Some(4.0)
    }));
    h.app.record(Ok(c.finish()), "added");
    h.time += 1.0;
    h.frame(vec![]);
    assert_ne!(h.app.fine_3d.as_ref().map(|(key, _)| key.clone()), kept);
}

#[test]
fn dragging_a_model_in_the_2d_view_moves_it() {
    use cut_craft_core::component::{Combine, Component};
    let mut h = Harness::new();
    let o = h.app.doc.job.area().min;
    let rect = cut_craft_core::geometry::Rect::new(
        o + Point::new(100.0, 100.0),
        o + Point::new(160.0, 140.0),
    );
    let mut c = Change::builder(&h.app.doc);
    c.add_component(Component::from_fn("model", Combine::Add, rect, 1.0, |_| {
        Some(2.0)
    }));
    h.app.record(Ok(c.finish()), "added");
    h.frame(vec![]);
    // issue #4: pressed on the model (not yet picked) and dragged, it is picked and moves
    let (from, to) = (h.at(130.0, 120.0), h.at(180.0, 140.0));
    h.drag(from, to);
    let k = h.app.doc.components.values().next().unwrap();
    let moved = k.extent().min - rect.min;
    assert!(
        (moved.x - 50.0).abs() < 1.0 && (moved.y - 20.0).abs() < 1.0,
        "moved by {moved:?}"
    );
    assert_eq!(h.app.doc.selected_components.len(), 1, "and picked");
    h.app.invoke(&h.ctx, "undo");
    let k = h.app.doc.components.values().next().unwrap();
    assert!(
        (k.extent().min - rect.min).length() < 1e-6,
        "one undo puts it back"
    );
    // a vector selected is dragged by pressing inside its box
    h.rectangle(10.0, 10.0, 40.0, 30.0);
    let before = h.app.doc.selection_bounds().unwrap();
    let (from, to) = (h.at(30.0, 25.0), h.at(40.0, 25.0));
    h.drag(from, to);
    let after = h.app.doc.selection_bounds().unwrap();
    assert!(
        (after.min.x - before.min.x - 10.0).abs() < 0.5,
        "{before:?} → {after:?}"
    );
    // a drag from empty space still selects by window
    h.app.doc.selection.clear();
    h.app.doc.selected_components.clear();
    let (from, to) = (h.at(5.0, 5.0), h.at(90.0, 80.0));
    h.drag(from, to);
    assert_eq!(
        h.app.doc.selection.len(),
        1,
        "the window picked the rectangle"
    );
}

#[test]
fn files_dropped_on_the_window_open_import_or_install() {
    let dir = std::env::temp_dir().join(format!("cut-craft-drop-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let picture = dir.join("photo.png");
    let mut png = std::io::Cursor::new(vec![]);
    image::GrayImage::from_pixel(4, 4, image::Luma([128]))
        .write_to(&mut png, image::ImageFormat::Png)
        .unwrap();
    std::fs::write(&picture, png.into_inner()).unwrap();
    // on the start page there is no job to import into
    let mut h = Harness::at_start();
    h.drop_files(std::slice::from_ref(&picture), true);
    assert!(h.app.doc.bitmaps.is_empty());
    assert!(
        h.app.status.message.contains("job"),
        "{}",
        h.app.status.message
    );
    // in a job a picture is imported; dragging over says what a drop will do
    let mut h = Harness::new();
    h.drop_files(std::slice::from_ref(&picture), false);
    assert!(h.text_at("Drop to open or import").is_some());
    h.drop_files(&[picture], true);
    assert_eq!(h.app.doc.bitmaps.len(), 1, "imported");
    // a post-processor is installed
    h.app.posts = cut_craft_machines::Library::new(Some(dir.join("posts")));
    let post = dir.join("router.pp");
    std::fs::write(&post, cut_craft_post::BUILT_IN[0].1).unwrap();
    h.drop_files(&[post], true);
    assert!(
        h.app
            .posts
            .entries()
            .iter()
            .any(cut_craft_machines::Entry::is_custom)
    );
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn an_installed_post_is_kept_and_the_machine_saves_through_it() {
    let mut h = Harness::new();
    let dir = std::env::temp_dir().join(format!("cut-craft-app-posts-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    h.app.posts = cut_craft_machines::Library::new(Some(dir.join("posts")));
    let file = dir.join("mine.pp");
    std::fs::create_dir_all(&dir).unwrap();
    let text = cut_craft_post::BUILT_IN[0]
        .1
        .replace("Cut Craft FluidNC ATC (mm)", "My Router (mm)");
    std::fs::write(&file, text).unwrap();
    h.app.install_post(&file);
    assert_eq!(h.app.machines.machines.len(), 1, "a machine to use it");
    assert_eq!(h.app.output_post().unwrap().name, "My Router (mm) (*.nc)");
    std::fs::remove_file(&file).unwrap();
    assert!(
        h.app.output_post().is_ok(),
        "kept in the custom posts, not read from where it was"
    );
    // the Machine menu opens its windows; the online ones stay greyed
    for id in [
        "machine_configuration",
        "post_processor_management",
        "add_machine",
    ] {
        assert!(h.app.available(id), "{id}");
        h.app.invoke(&h.ctx, id);
        h.frame(vec![]);
    }
    assert!(h.app.machine_window.is_some() && h.app.post_window.is_some());
    assert!(h.text_at("Associated Post-Processors").is_some());
    for id in [
        "install_offline_package",
        "search_for_machine_online",
        "update_post_processor_database",
    ] {
        assert!(!h.app.available(id), "{id} needs Vectric's service");
    }
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_tool_changer_loads_tools_and_the_gcode_calls_their_slots() {
    let mut h = Harness::new();
    h.app.machines.add();
    h.app.machines.active_mut().unwrap().set_slot_count(5);
    // load the 3 mm end mill into slot 3 in the Tool Database, kept on OK
    h.app.invoke(&h.ctx, "tool_database");
    let manager = h.app.tool_manager.as_mut().unwrap();
    manager
        .changer
        .as_mut()
        .expect("the machine's tool changer")
        .load("3 mm End Mill", Some(3));
    // a new window is drawn unseen for a frame while egui sizes it
    h.frame(vec![]);
    h.frame(vec![]);
    assert!(h.text_at("Tool changer: My Machine").is_some());
    let ok = *h.texts_at("OK").last().unwrap();
    h.click(ok, Modifiers::NONE);
    assert_eq!(
        h.app.machines.active().unwrap().slot_of("3 mm End Mill"),
        Some(3)
    );
    // the toolpath form offers it first
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    h.app.invoke(&h.ctx, "profile_toolpath");
    h.frame(vec![]);
    h.app.choose_tool("3 mm End Mill");
    let params = h.app.form.as_ref().unwrap().params.clone();
    h.app.apply("profile_toolpath", &params).unwrap();
    // the G-code calls slot 3
    let post = h.app.output_post().unwrap();
    let out = std::env::temp_dir().join(format!("cut-craft-atc-{}.nc", std::process::id()));
    files::save_through(&post, &h.app.slots(), &h.app.doc, &out, None).unwrap();
    let gcode = std::fs::read_to_string(&out).unwrap();
    assert!(gcode.contains("M6 T3"), "{gcode}");
    // a tool not loaded is refused
    h.app.machines.active_mut().unwrap().slots[2] = None;
    let why = files::save_through(&post, &h.app.slots(), &h.app.doc, &out, None).unwrap_err();
    assert!(why.contains("3 mm End Mill"), "{why}");
    let _ = std::fs::remove_file(out);
}

#[test]
fn toolpaths_save_through_the_default_fluidnc_post_until_one_is_installed() {
    use cut_craft_core::document::{Along, Job};
    let mut h = Harness::new();
    assert_eq!(
        h.app.output_post().unwrap().name,
        "Cut Craft FluidNC ATC (mm) (*.nc)"
    );
    h.app.doc.job = Job::rotary(300.0, 50.0, Along::X);
    assert_eq!(
        h.app.output_post().unwrap().name,
        "Cut Craft FluidNC ATC Wrap A G93 (mm) (*.nc)"
    );
}

/// Controls that wait on a person or open other programs: the system's file dialogs, the
/// save-changes question (the smoke test's job has unsaved work), the file manager and
/// quitting. Running one in a test puts a real window on the desktop.
fn needs_a_person(app: &CutCraftApp, id: &str) -> bool {
    const ASKS: [&str; 26] = [
        "add_clipart_folder",
        "segment_3d_model_file",
        "new",
        "open",
        "close",
        "save",
        "save_as",
        "increment_and_save",
        "open_application_data_folder",
        "import_vectors_bitmap",
        "import_component_3d_model",
        "save_as_template",
        "new_from_template",
        "save_selected_toolpath_as_template",
        "save_all_visible_toolpaths_as_template",
        "load_template",
        "print",
        "print_preview",
        "install_post_processor",
        "save_toolpaths",
        "exit",
        "recent_files",
        "import_vectors",
        "export_vectors",
        "open_folder",
        "open_clipart_folder",
    ];
    ASKS.contains(&id) || app.registry.exporter(id).is_some() || id.starts_with(EXPORT_LEVEL)
}

/// Every enabled menu item and tool button, run through the app as a click would be on a
/// small job with vectors, a toolpath and a 3D shape: none may panic or be a dead end.
#[test]
fn every_enabled_control_works() {
    let mut h = Harness::new();
    h.app.doc.job.width = 100.0;
    h.app.doc.job.height = 100.0;
    h.rectangle(10.0, 10.0, 40.0, 40.0);
    h.rectangle(30.0, 30.0, 40.0, 40.0);
    h.app.invoke(&h.ctx, "select_all_vectors");
    for id in ["profile_toolpath", "create_shape"] {
        let params = h.app.registry.get(id).unwrap().initial_params(&h.app.doc);
        h.app.apply(id, &params).unwrap();
    }
    h.app.selected_toolpath = h.app.doc.toolpaths.keys().next().copied();
    h.frame(vec![]);
    let toolbar = &h.app.toolbar;
    let mut ids: Vec<String> = h
        .app
        .menus
        .items()
        .iter()
        .map(|i| i.action.clone())
        .collect();
    for group in toolbar
        .groups
        .iter()
        .chain(&toolbar.modeling_groups)
        .chain(&toolbar.toolpath_groups)
    {
        ids.extend(group.tools.iter().map(|&t| t.to_owned()));
    }
    ids.extend(VIEW_BUTTONS.iter().map(|(id, ..)| (*id).to_owned()));
    // the right-click menus, on each thing they open on
    let doc = &h.app.doc;
    let mut popups = h.app.menus.selection_menu(doc);
    for layer in &doc.layers {
        popups.extend(gui::layer_menu(doc, layer));
    }
    for (&id, toolpath) in &doc.toolpaths {
        popups.extend(gui::toolpath_menu(id, toolpath));
    }
    for (&id, component) in &doc.components {
        popups.extend(gui::component_menu(doc, id, component));
    }
    for (&id, level) in &doc.levels {
        popups.extend(gui::level_menu(doc, id, level));
    }
    for group in doc.components.values().filter_map(|k| k.group.as_ref()) {
        popups.extend(gui::group_menu(group));
    }
    let enabled = gui::leaves(&popups).into_iter().filter(|i| i.is_enabled);
    ids.extend(enabled.map(|i| i.action.clone()));
    ids.sort();
    ids.dedup();
    let start = (h.app.doc.clone(), h.app.selected_toolpath);
    let mut ran = 0;
    for id in ids {
        if !h.app.available(&id) || needs_a_person(&h.app, &id) {
            continue;
        }
        (h.app.doc, h.app.selected_toolpath) = start.clone();
        h.app.history = History::default();
        (h.app.form, h.app.view_tool) = (None, None);
        h.app.status.set_message("");
        h.app.invoke(&h.ctx, &id);
        // a tool with a form runs with what the form starts with
        if let Some(form) = h.app.form.take() {
            let outcome = h.app.apply(form.id, &form.params);
            if let Err(why) = outcome {
                assert!(!why.contains("not implemented"), "{id}: {why}");
            }
        }
        h.frame(vec![]);
        let said = &h.app.status.message;
        assert!(!said.contains("not implemented"), "{id}: {said}");
        ran += 1;
    }
    assert!(ran >= 92, "only {ran} controls ran");
}

#[test]
fn preview_all_leaves_hidden_toolpaths_out() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    for id in ["drilling_toolpath", "profile_toolpath"] {
        let params = h.app.registry.get(id).unwrap().initial_params(&h.app.doc);
        h.app.apply(id, &params).unwrap();
    }
    let profile = *h.app.doc.toolpaths.keys().last().unwrap();
    h.app.selected_toolpath = Some(profile);
    h.app.invoke(&h.ctx, "preview_toolpath");
    let only_profile = h.app.simulation.clone();
    let drilling = *h.app.doc.toolpaths.keys().next().unwrap();
    h.app.doc.toolpaths.get_mut(&drilling).unwrap().visible = false;
    h.app.invoke(&h.ctx, "preview_all_toolpaths");
    assert_eq!(h.app.simulation, only_profile);
}

#[test]
fn deleting_the_selected_toolpath_is_undoable() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    h.app.invoke(&h.ctx, "drilling_toolpath");
    let params = h.app.form.as_ref().unwrap().params.clone();
    h.app.apply("drilling_toolpath", &params).unwrap();
    h.app.selected_toolpath = h.app.doc.toolpaths.keys().next().copied();
    h.app.invoke(&h.ctx, "delete_toolpath");
    assert!(h.app.doc.toolpaths.is_empty());
    h.app.invoke(&h.ctx, "undo");
    assert_eq!(h.app.doc.toolpaths.len(), 1);
}

#[test]
fn unsaved_work_is_kept_for_recovery_and_opens_after_a_crash() {
    let dir = std::env::temp_dir().join(format!("cut-craft-recovery-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let recovery = dir.join("recovery.ccjob");
    let autosaved = |h: &mut Harness| {
        h.app.recovery_path = Some(recovery.clone());
        h.rectangle(10.0, 10.0, 20.0, 20.0);
        h.frame(vec![]);
        assert!(!recovery.exists(), "not at once");
        h.time += AUTOSAVE_SECONDS;
        h.frame(vec![]);
        let writing = h.app.autosave.take().expect("a copy is being written");
        writing.join().unwrap().unwrap();
        assert!(recovery.exists());
    };
    // closing the job on purpose leaves nothing to recover
    let mut h = Harness::new();
    autosaved(&mut h);
    h.app.session.discard_changes(&h.app.history);
    h.app.invoke(&h.ctx, "close");
    assert!(!recovery.exists());
    // a run that ends badly leaves the copy, and the next run opens it
    let mut h = Harness::new();
    autosaved(&mut h);
    drop(h);
    let mut next = Harness::at_start();
    next.app.recovery_path = Some(recovery.clone());
    assert!(next.app.open_recovery());
    assert_eq!(next.app.screen, Screen::Job);
    assert_eq!(next.app.doc.vectors.len(), 1);
    assert!(
        next.app.session.is_dirty(&next.app.history),
        "unsaved until saved"
    );
    assert!(next.app.status.message.contains("Recovered"));
    next.app.save_to(&dir.join("kept.ccjob"));
    assert!(!recovery.exists(), "saved: nothing to recover");
    assert!(!next.app.open_recovery());
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn ctrl_x_c_and_v_arrive_as_clipboard_events_and_work_across_windows() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 20.0, 20.0);
    h.frame(vec![Event::Copy]);
    assert_eq!(h.app.clipboard.len(), 1);
    assert!(
        h.copied.starts_with('['),
        "the system clipboard gets the vectors: {}",
        h.copied
    );
    // what another window copied pastes here
    let circle = vec![Path::circle(Point::new(50.0, 50.0), 5.0)];
    h.frame(vec![Event::Paste(serde_json::to_string(&circle).unwrap())]);
    assert_eq!(h.app.doc.vectors.len(), 2);
    assert_eq!(h.app.clipboard, circle);
    // text from elsewhere pastes nothing
    h.frame(vec![Event::Paste("hello".into())]);
    assert_eq!(h.app.doc.vectors.len(), 2);
    // the pasted circle is selected: Ctrl+X takes it
    h.frame(vec![Event::Cut]);
    assert_eq!(h.app.doc.vectors.len(), 1);
    assert_eq!(h.app.clipboard, circle);
}

#[test]
fn crossing_vectors_go_to_the_vector_validator_or_on_as_options_say() {
    use crate::options::IntersectionCheck;
    use cut_craft_core::geometry::{Path, Point};
    let mut h = Harness::new();
    let bow_tie = Path::polyline(
        &[
            Point::new(10.0, 10.0),
            Point::new(50.0, 50.0),
            Point::new(50.0, 10.0),
            Point::new(10.0, 50.0),
        ],
        true,
    );
    let mut c = Change::builder(&h.app.doc);
    let id = c.add(bow_tie);
    c.select(vec![id]);
    let change = c.finish();
    h.app.history.apply(&mut h.app.doc, change).unwrap();
    let params = h
        .app
        .registry
        .get("pocket_toolpath")
        .unwrap()
        .initial_params(&h.app.doc);
    h.app.options.confirmations.toolpath_intersections = IntersectionCheck::OpenVectorValidator;
    let why = h.app.apply("pocket_toolpath", &params).unwrap_err();
    assert!(why.contains("cross"), "{why}");
    assert!(h.app.doc.toolpaths.is_empty());
    assert_eq!(h.app.form.as_ref().map(|f| f.id), Some("vector_validator"));
    assert!(h.app.doc.selection.contains(&id));
    h.app.options.confirmations.toolpath_intersections = IntersectionCheck::DontCheck;
    h.app.apply("pocket_toolpath", &params).unwrap();
    assert_eq!(h.app.doc.toolpaths.len(), 1);
}

#[test]
fn tabs_are_placed_and_removed_by_clicking_the_vector() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    h.click(h.at(60.0, 35.0), Modifiers::NONE);
    h.app.invoke(&h.ctx, "profile_toolpath");
    {
        let params = &mut h.app.form.as_mut().unwrap().params;
        params["tabs"] = "flat".into();
        params["tab"]["spacing"] = "placed".into();
    }
    h.frame(vec![]);
    h.click(h.at(60.0, 30.0), Modifiers::NONE);
    let tabs = |h: &Harness| -> Vec<Point> {
        serde_json::from_value(h.app.form.as_ref().unwrap().params["tab_points"].clone())
            .unwrap_or_default()
    };
    let placed = tabs(&h);
    assert_eq!(placed.len(), 1, "{}", h.app.status.message);
    assert!(near(placed[0], Point::new(60.0, 30.0)), "{placed:?}");
    assert_eq!(
        h.app.doc.selection.len(),
        1,
        "the click placed a tab, not a selection"
    );
    let params = h.app.form.as_ref().unwrap().params.clone();
    h.app.apply("profile_toolpath", &params).unwrap();
    assert_eq!(h.app.doc.toolpaths.len(), 1);
    h.click(h.at(60.0, 30.0), Modifiers::NONE);
    assert!(tabs(&h).is_empty(), "a second click takes it off");
}

/// A new job with a shape on it, in the 3D view.
fn shape_in_3d() -> Harness {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    let shape = h.app.registry.get("create_shape").unwrap().default_params();
    h.app.apply("create_shape", &shape).unwrap();
    h.app.view = View::ThreeD;
    h.frame(vec![]);
    h.frame(vec![]);
    h
}

#[test]
fn the_view_cube_looks_from_the_face_edge_or_corner_clicked() {
    use cut_craft_core::glam::DVec3;
    let mut h = shape_in_3d();
    let looks_from = |h: &Harness, from: DVec3| {
        let towards = h.app.orbit.towards_viewer();
        assert!(
            towards.abs_diff_eq(from.normalize(), 1e-9),
            "{towards} not {from}"
        );
    };
    let cube = h.app.view_cube.expect("the 3D view draws its view cube");
    let framing = h.app.framing;
    // the right face's middle, with its name on it
    let right = cube.screen(h.app.orbit, DVec3::X);
    assert!(
        h.texts_at("Right")
            .iter()
            .any(|at| at.distance(right) < 1.0),
        "named"
    );
    h.click(right, Modifiers::NONE);
    looks_from(&h, DVec3::X);
    assert_eq!(h.app.framing, framing, "turned, not refitted");
    // face on now: its top corner at the back, then its top edge
    let corner = cube.screen(h.app.orbit, DVec3::new(1.0, 0.8, 0.8));
    h.click(corner, Modifiers::NONE);
    looks_from(&h, DVec3::new(1.0, 1.0, 1.0));
    let edge = cube.screen(h.app.orbit, DVec3::new(0.8, 0.0, 1.0));
    h.click(edge, Modifiers::NONE);
    looks_from(&h, DVec3::new(1.0, 0.0, 1.0));
    // a double-click looks from the other side, and the view behind takes no clicks
    let selected = h.app.doc.selected_components.clone();
    h.double_click(cube.screen(h.app.orbit, DVec3::new(1.0, 0.0, 1.0)));
    looks_from(&h, DVec3::new(-1.0, 0.0, -1.0));
    assert_eq!(h.app.doc.selected_components, selected);
    assert_eq!(h.app.framing, framing);
}

#[test]
fn the_3d_view_turns_to_perspective_and_tilts_underneath() {
    let mut h = shape_in_3d();
    let start = h.app.orbit;
    let button = h.text_at("Perspective").expect("the perspective toggle");
    h.click(button, Modifiers::NONE);
    assert!(h.app.framing.perspective);
    h.click(h.text_at("Fit").unwrap(), Modifiers::NONE);
    assert!(h.app.framing.perspective, "Fit keeps the projection");
    // a long drag down, below the cube, tilts the view past side on to under the job
    let cube = h.app.view_cube.unwrap().rect();
    let from = cube.left_bottom() + vec2(-cube.width(), cube.height());
    h.drag(from, from + vec2(0.0, 300.0));
    let pitch = h.app.orbit.pitch;
    assert!(
        pitch > std::f64::consts::FRAC_PI_2 && pitch > start.pitch,
        "{pitch}"
    );
    assert!(h.app.orbit.towards_viewer().z < 0.0, "from underneath");
    h.click(button, Modifiers::NONE);
    assert!(!h.app.framing.perspective, "back to orthographic");
}

#[test]
fn a_vector_behind_the_model_is_not_drawn_over_it() {
    // a tall block on the right half; an outline on the floor left of it
    use cut_craft_core::component::{Combine, Component};
    let mut h = Harness::new();
    let job = h.app.doc.job;
    let o = job.area().min;
    let wall = cut_craft_core::geometry::Rect::new(
        o + Point::new(job.width / 2.0, 0.0),
        o + Point::new(job.width, job.height),
    );
    let mut c = Change::builder(&h.app.doc);
    c.add_component(Component::from_fn(
        "wall",
        Combine::Add,
        wall,
        job.model_cell(),
        |_| Some(job.width / 2.0),
    ));
    h.app.record(Ok(c.finish()), "added");
    h.rectangle(
        job.width / 8.0,
        job.height / 4.0,
        job.width / 8.0,
        job.height / 2.0,
    );
    h.app.view = View::ThreeD;
    // lines drawn in the vectors' colour
    let lines = |h: &mut Harness, orbit, vectors| {
        h.app.orbit = orbit;
        h.app.display.vectors_3d = vectors;
        h.frame(vec![]);
        let stroke = h
            .ctx
            .global_style()
            .visuals
            .widgets
            .noninteractive
            .fg_stroke;
        let colour = egui::epaint::ColorMode::Solid(stroke.color);
        let drawn = h
            .shapes
            .iter()
            .filter(|c| matches!(&c.shape, egui::Shape::Path(p) if p.stroke.color == colour));
        drawn.count()
    };
    let top = view3d::Orbit::TOP;
    assert!(
        lines(&mut h, top, true) > lines(&mut h, top, false),
        "from above it shows"
    );
    let low_right = view3d::Orbit::facing(cut_craft_core::glam::DVec3::new(1.0, 0.0, 0.1));
    assert_eq!(
        lines(&mut h, low_right, true),
        lines(&mut h, low_right, false),
        "from low on the right the block hides it"
    );
}

#[test]
fn set_as_default_in_job_setup_is_what_new_jobs_start_with() {
    let mut h = Harness::at_start();
    h.app.invoke(&h.ctx, "new");
    h.frame(vec![]);
    let form = h.app.form.as_mut().expect("the Job Setup form");
    form.params["size"]["width"] = 300.0.into();
    form.params["size"]["thickness"] = 12.0.into();
    let button = h
        .text_at("Set as Default")
        .expect("a Set as Default button");
    h.click(button, Modifiers::NONE);
    h.frame(vec![]);
    let saved = h.app.options.default_job.expect("kept in the options");
    assert!((saved.width - 300.0).abs() < 1e-9);
    h.app.job_setup_cancel();
    h.app.invoke(&h.ctx, "new");
    h.frame(vec![]);
    let form = h.app.form.as_ref().expect("Job Setup again");
    assert_eq!(form.params["size"]["width"], 300.0);
    assert_eq!(form.params["size"]["thickness"], 12.0);
}
