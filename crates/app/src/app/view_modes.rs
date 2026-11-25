//! The 2D view's modes and buttons: tracing a bitmap, placing tabs, filleting corners, joining open
//! ends, measuring and the View menu.

use super::{
    Align, Canvas, CutCraftApp, Drag, Gesture, Layout, Overlay, Path, Point, Rect, TOOL_ICON_ROWS,
    VIEW_BUTTONS, ViewTool, dialog_size, egui, gui, inspected, inspected_rows, measured,
    measured_rows,
};
use cut_craft_bug_report::hint::HoverHint;
use cut_craft_i18n::tr;

impl CutCraftApp {
    /// The view buttons over the top right of the 2D view, below the ruler.
    pub(super) fn view_buttons(&mut self, ui: &mut egui::Ui, area: egui::Rect) {
        let row = ui.spacing().interact_size.y;
        let edge = TOOL_ICON_ROWS * row;
        let gap = ui.spacing().item_spacing;
        let room = egui::Rect::from_min_max(
            area.min + egui::vec2(row, row) + gap,
            egui::pos2(area.max.x - gap.x, area.max.y),
        );
        let in_hand = |id: &str| match self.view_tool {
            Some(ViewTool::Pan) => id == "pan",
            Some(ViewTool::ZoomBox) => id == "zoom_box",
            Some(ViewTool::NodeEdit) => id == "node_edit",
            Some(ViewTool::Measure) => id == "measure_inspect",
            Some(ViewTool::Transform) => id == "transform",
            None => false,
        };
        let mut clicked = None;
        // The toolpath drawing toggles over the top left of the 2D view
        ui.scope_builder(egui::UiBuilder::new().max_rect(room), |ui| {
            ui.with_layout(Layout::left_to_right(Align::Min), |ui| {
                let shown = self.display.previews_2d;
                let drawing = if shown {
                    "view/2d_toolpath_drawing_on.png"
                } else {
                    "view/2d_toolpath_drawing_off.png"
                };
                for (id, icon, tip, on) in [
                    (
                        "draw_2d_previews",
                        drawing,
                        "Draw 2D Toolpath Previews",
                        shown,
                    ),
                    (
                        "make_2d_previews_solid",
                        "view/solid_2d_toolpath_drawing.png",
                        "Make 2D Previews Solid",
                        self.display.solid_previews,
                    ),
                ] {
                    let button = self.icon_button(icon, "", tip, edge).selected(on);
                    if ui.add(button).on_hover_hint(tip).clicked() {
                        clicked = Some(id);
                    }
                }
                // the job's construction lines shown or hidden: the eye says which they are
                if !self.doc.construction.is_empty() {
                    let (icon, tip) = if self.display.construction {
                        (
                            "view/construction_lines_shown.png",
                            "Construction lines shown: click to hide them",
                        )
                    } else {
                        (
                            "view/construction_lines_hidden.png",
                            "Construction lines hidden: click to show them",
                        )
                    };
                    let button = self.icon_button(icon, "", tip, edge);
                    if ui.add(button).on_hover_hint(tip).clicked() {
                        clicked = Some("show_construction_lines");
                    }
                }
            });
        });
        ui.scope_builder(egui::UiBuilder::new().max_rect(room), |ui| {
            // laid out from the right, so the first button is the leftmost
            ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                for id in VIEW_BUTTONS.into_iter().rev() {
                    let label = &self.tool_label(id);
                    let icon = gui::built_in_icon(id).unwrap_or_default();
                    let button = self
                        .icon_button(icon, id, label, edge)
                        .selected(in_hand(id));
                    let response = ui
                        .add_enabled(self.available(id), button)
                        .on_hover_hint(label)
                        .on_disabled_hover_text(tr!("views-select-first", tool = label));
                    if response.clicked() {
                        clicked = Some(id);
                    }
                }
            });
        });
        if let Some(id) = clicked {
            self.invoke(ui.ctx(), id);
        }
    }

    /// Whether the open form is a profile toolpath placing its tabs by clicking.
    /// Trace Bitmap's form is open: the 2D view picks its area and colours.
    pub(super) fn tracing(&self) -> bool {
        self.form.as_ref().is_some_and(|f| f.id == "trace_bitmap")
    }

    /// Trace Bitmap's area: only it is traced (`None`: the whole picture).
    pub(super) fn set_trace_area(&mut self, area: Option<Rect>) {
        if let Some(form) = self.form.as_mut() {
            form.params["area"] = serde_json::to_value(area).expect("a box serialises");
            if area.is_none() {
                form.params.as_object_mut().map(|m| m.remove("area"));
            }
        }
    }

    /// A click with Trace Bitmap open: with an area picked, the whole picture again; else, in
    /// colour mode, the colour under the click is picked to trace or let go.
    pub(super) fn trace_click(&mut self, at: Point) {
        let Some(form) = self.form.as_ref() else {
            return;
        };
        if !form.params["area"].is_null() {
            self.set_trace_area(None);
            self.status
                .set_message(&cut_craft_i18n::tr!("status-tracing-whole-picture"));
            return;
        }
        let Ok(p) = serde_json::from_value::<cmd_trace_bitmap::Params>(form.params.clone()) else {
            return;
        };
        let Some(colours) = cmd_trace_bitmap::toggled_at(&self.doc, &p, at) else {
            return;
        };
        self.status.set_message(&if colours.is_empty() {
            cut_craft_i18n::tr!("status-no-colours-traced")
        } else {
            cut_craft_i18n::tr!("status-colours-to-trace", colours = colours.join(", "))
        });
        if let Some(form) = self.form.as_mut() {
            form.params["trace"] = serde_json::json!(colours);
        }
    }

    /// Trace Bitmap's area, dashed.
    pub(super) fn paint_trace_area(&self, ui: &egui::Ui, canvas: &Canvas) {
        let Some(area) = self
            .form
            .as_ref()
            .filter(|f| f.id == "trace_bitmap")
            .and_then(|f| serde_json::from_value::<Rect>(f.params["area"].clone()).ok())
        else {
            return;
        };
        let (a, b) = (
            canvas.transform.to_screen(area.min),
            canvas.transform.to_screen(area.max),
        );
        let corners = [a, egui::pos2(b.x, a.y), b, egui::pos2(a.x, b.y), a];
        let dash = ui.spacing().interact_size.y / 3.0;
        ui.painter_at(canvas.area).extend(egui::Shape::dashed_line(
            &corners,
            egui::Stroke::new(1.0, ui.visuals().text_color()),
            dash,
            dash,
        ));
    }

    pub(super) fn placing_tabs(&self) -> bool {
        self.form.as_ref().is_some_and(|f| {
            f.id == "profile_toolpath"
                && f.params["tabs"].as_str().is_some_and(|t| t != "none")
                && f.params["tab"]["spacing"] == "placed"
        })
    }

    /// A ring round each clicked tab, the size a click on it must land within.
    pub(super) fn tab_overlay(&self, reach: f64) -> Overlay {
        Overlay::Preview(
            self.tab_points()
                .into_iter()
                .map(|p| Path::circle(p, reach))
                .collect(),
        )
    }

    /// The open form's clicked tabs.
    fn tab_points(&self) -> Vec<Point> {
        self.form
            .as_ref()
            .and_then(|f| serde_json::from_value(f.params["tab_points"].clone()).ok())
            .unwrap_or_default()
    }

    /// The point on a selected vector nearest `at`, within `reach`.
    fn on_selected(&self, at: Point, reach: f64) -> Option<Point> {
        self.doc
            .selection
            .iter()
            .filter_map(|id| self.doc.vectors.get(id))
            .filter_map(|path| {
                let mut flat = path.flatten(cut_craft_core::polygon::TOLERANCE);
                if path.closed
                    && let Some(&first) = flat.first()
                {
                    flat.push(first);
                }
                cut_craft_core::geometry::polyline_closest(&flat, at)
            })
            .min_by(|p, q| p.distance(at).total_cmp(&q.distance(at)))
            .filter(|p| p.distance(at) <= reach)
    }

    /// A drag with no tool: from a placed tab it moves it, else it selects.
    pub(super) fn plain_drop(&mut self, drag: Drag, shift: bool, reach: f64, placing_tabs: bool) {
        if placing_tabs && self.on_a_tab(drag.start, reach) {
            self.move_tab(drag, reach);
        } else {
            self.select_drop(drag, shift, reach);
        }
    }

    /// Whether `at` is on one of the open form's placed tabs.
    fn on_a_tab(&self, at: Point, reach: f64) -> bool {
        self.tab_points().iter().any(|t| t.distance(at) <= reach)
    }

    /// A drag from a placed tab moves it along to the selected vector where it is dropped.
    pub(super) fn move_tab(&mut self, drag: Drag, reach: f64) {
        let mut points = self.tab_points();
        let Some(i) = points.iter().position(|p| p.distance(drag.start) <= reach) else {
            return;
        };
        let Some(on) = self.on_selected(drag.end, reach) else {
            self.status
                .set_message(&cut_craft_i18n::tr!("status-drop-tab-on-vector"));
            return;
        };
        points[i] = on;
        self.status
            .set_message(&cut_craft_i18n::tr!("status-tab-moved"));
        if let Some(form) = self.form.as_mut() {
            form.params["tab_points"] = serde_json::json!(points);
        }
    }

    /// A click while placing tabs: on a tab takes it off, on a selected vector adds one there.
    pub(super) fn place_tab(&mut self, at: Point, reach: f64) {
        let mut points = self.tab_points();
        if let Some(i) = points.iter().position(|p| p.distance(at) <= reach) {
            points.remove(i);
            self.status
                .set_message(&cut_craft_i18n::tr!("status-tab-removed"));
        } else {
            let on = self.on_selected(at, reach);
            let Some(on) = on else {
                self.status
                    .set_message(&cut_craft_i18n::tr!("status-click-to-add-tab"));
                return;
            };
            points.push(on);
            self.status
                .set_message(&cut_craft_i18n::tr!("status-tab-added"));
        }
        if let Some(form) = self.form.as_mut() {
            form.params["tab_points"] = serde_json::json!(points);
        }
    }

    /// Whether Create Fillets is open: a click on a sharp corner fillets it, a click on a
    /// fillet takes it out.
    pub(super) fn filleting(&self) -> bool {
        self.form.as_ref().is_some_and(|f| f.id == "create_fillets")
    }

    /// Create Fillets' settings aimed at `at`, within `reach` of it.
    fn fillet_params(&self, at: Point, reach: f64) -> Option<cmd_create_fillets::Params> {
        let form = self.form.as_ref().filter(|f| f.id == "create_fillets")?;
        let mut params: cmd_create_fillets::Params =
            serde_json::from_value(form.params.clone()).ok()?;
        params.at = Some(at);
        params.within = reach;
        Some(params)
    }

    /// The vector as a click at `at` would leave it: its corner filleted, or its fillet gone.
    pub(super) fn fillet_overlay(&self, at: Option<Point>, reach: f64) -> Overlay {
        let Some(at) = at else { return Overlay::None };
        let Some(params) = self.fillet_params(at, reach) else {
            return Overlay::None;
        };
        match cmd_create_fillets::at_click(&self.doc, &params, at) {
            Ok((_, path, _)) => Overlay::Preview(vec![path]),
            Err(_) => Overlay::None,
        }
    }

    /// A click with Create Fillets open: the corner there filleted, or the fillet there
    /// taken out.
    pub(super) fn fillet_click(&mut self, at: Point, reach: f64) {
        let Some(params) = self.fillet_params(at, reach) else {
            return;
        };
        let params = serde_json::to_value(params).expect("fillet settings serialise");
        if let Err(e) = self.apply("create_fillets", &params) {
            self.status.set_message(&e);
        }
    }

    /// Whether Join Open Ends is open: clicks pick open ends, two at a time, to join.
    pub(super) fn joining_ends(&self) -> bool {
        self.form.as_ref().is_some_and(|f| f.id == "join_open_ends")
    }

    /// Join Open Ends' first end, once one is picked.
    pub(super) fn join_ends_first(&self) -> Option<Point> {
        let form = self.form.as_ref().filter(|_| self.joining_ends())?;
        serde_json::from_value(form.params["first"].clone()).ok()
    }

    /// Join Open Ends' settings with its first end picked, aimed at `second`.
    fn join_ends_params(&self, second: Point, reach: f64) -> Option<cmd_join_open_ends::Params> {
        let form = self.form.as_ref().filter(|_| self.joining_ends())?;
        let mut params: cmd_join_open_ends::Params =
            serde_json::from_value(form.params.clone()).ok()?;
        params.second = Some(second);
        params.within = reach;
        Some(params)
    }

    /// The first end picked, ringed, and the vector a click at `at` would make.
    pub(super) fn join_ends_overlay(&self, at: Option<Point>, reach: f64) -> Overlay {
        let Some(first) = self.join_ends_first() else {
            return Overlay::None;
        };
        let mut paths = vec![Path::circle(first, reach)];
        if let Some(at) = at
            && let Some(params) = self.join_ends_params(at, reach)
            && let Ok((_, _, joined)) = cmd_join_open_ends::joined(&self.doc, &params)
        {
            paths.push(joined);
        }
        Overlay::Preview(paths)
    }

    /// A click with Join Open Ends open: the first end picked, or the second, which joins
    /// the two. A click on the first end again lets it go.
    pub(super) fn join_ends_click(&mut self, at: Point, reach: f64) {
        let first = self.join_ends_first();
        let Some(end) = cmd_join_open_ends::end_near(&self.doc, at, reach) else {
            self.status
                .set_message(&cut_craft_i18n::tr!("status-no-open-end"));
            return;
        };
        let Some(first) = first else {
            if let Some(form) = self.form.as_mut() {
                form.params["first"] = serde_json::json!(end.point);
            }
            self.status
                .set_message(&cut_craft_i18n::tr!("status-end-picked"));
            return;
        };
        let same = cmd_join_open_ends::end_near(&self.doc, first, reach)
            .is_some_and(|f| (f.id, f.at_start) == (end.id, end.at_start))
            && cmd_join_open_ends::open_ends(&self.doc)
                .iter()
                .filter(|e| e.point.distance(at) <= reach)
                .count()
                == 1;
        if same {
            self.join_ends_forget();
            self.status
                .set_message(&cut_craft_i18n::tr!("status-end-let-go"));
            return;
        }
        let Some(params) = self.join_ends_params(at, reach) else {
            return;
        };
        let params = serde_json::to_value(params).expect("join settings serialise");
        match self.apply("join_open_ends", &params) {
            Ok(()) => self.join_ends_forget(),
            Err(e) => self.status.set_message(&e),
        }
    }

    /// Join Open Ends with no end picked.
    pub(super) fn join_ends_forget(&mut self) {
        if let Some(form) = self.form.as_mut().filter(|f| f.id == "join_open_ends") {
            form.params["first"] = serde_json::Value::Null;
            form.params["second"] = serde_json::Value::Null;
        }
    }

    /// The open form's command, if it can be drawn with the mouse.
    pub(super) fn drawing_tool(&self) -> Option<&'static str> {
        let form = self.form.as_ref()?;
        self.registry
            .get(form.id)
            .filter(|c| c.interactive())
            .map(cut_craft_core::DynCommand::id)
    }

    /// Measure / Inspect: a finished drag measures between its ends, a click inspects a vector.
    pub(super) fn measure(&mut self, gesture: Option<Gesture>, reach: f64) {
        let units = self.doc.job.units;
        match gesture {
            Some(Gesture::Dropped { drag, .. }) => {
                self.status
                    .set_message(&measured(drag.start, drag.end, units));
                self.measure_rows = measured_rows(drag.start, drag.end, units);
            }
            Some(Gesture::Click { at, .. }) => {
                if let Some(id) = self.doc.pick(at, reach) {
                    let path = &self.doc.vectors[&id];
                    self.status.set_message(&inspected(path, at, reach, units));
                    self.measure_rows = inspected_rows(path, at, reach, units);
                } else {
                    self.status
                        .set_message(&cut_craft_i18n::tr!("status-click-to-measure"));
                }
            }
            _ => {}
        }
    }

    /// Measure / Inspect's form while the tool is on: its last measurement or inspection.
    pub(super) fn measure_window(&mut self, ctx: &egui::Context) {
        if self.view_tool != Some(ViewTool::Measure) {
            return;
        }
        let mut open = true;
        let (id, size) = (egui::Id::unique("measure_inspect"), dialog_size(ctx));
        let rows = &self.measure_rows;
        cut_craft_bug_report::dialog(ctx, id, "Measure / Inspect", size, &mut open, |ui| {
            if rows.is_empty() {
                ui.label(tr!("views-drag-between-two-points-to"));
            }
            egui::Grid::new("measure_rows")
                .striped(true)
                .show(ui, |ui| {
                    for (name, value) in rows {
                        ui.label(name);
                        ui.monospace(value);
                        ui.end_row();
                    }
                });
        });
        if !open {
            self.view_tool = None;
        }
    }

    /// The 2D view's right-click menu: a node's while node editing, else the selection's.
    pub(super) fn view_menu(&mut self, ui: &egui::Ui, canvas: &Canvas, reach: f64) {
        // a right-click finishes a polyline being drawn
        if !self.polyline.is_empty() && canvas.response().secondary_clicked() {
            self.finish_polyline(false);
            return;
        }
        // where the menu opened: kept while it stays open, since egui forgets the press
        // once the button is up, and used for what it was opened on and for guides
        // inserted from it
        if canvas.response().secondary_clicked() {
            self.menu_at = canvas
                .response()
                .interact_pointer_pos()
                .map(|q| canvas.transform.to_job(q));
        }
        let pressed = self.menu_at;
        let guide =
            pressed.and_then(|at| self.doc.guides.iter().position(|g| g.distance(at) <= reach));
        // the menu acts on what was right-clicked: a vector not yet selected is picked
        // first (with its group), one already selected keeps the selection as it is
        if canvas.response().secondary_clicked()
            && guide.is_none()
            && let Some(id) = pressed.and_then(|at| self.doc.pick(at, reach))
            && !self.doc.selection.contains(&id)
        {
            self.select([id], false);
        }
        let node_target = pressed
            .filter(|_| self.view_tool == Some(ViewTool::NodeEdit))
            .and_then(|at| self.node_target(at, reach));
        // over a keep-out zone: its name and where it is edited, first in the menu
        let zone = pressed.and_then(|at| self.doc.keep_out_at(at));
        let selection_menu = || match (guide, node_target) {
            (Some(i), _) => gui::guide_menu(i, self.doc.guides[i].locked),
            (None, Some(target)) => gui::node_menu(&self.doc, &target),
            (None, None) => {
                let mut items = zone.map_or_else(Vec::new, |(z, machine)| {
                    gui::keep_out_menu(&z.label(), machine, self.display.keep_out_labels)
                });
                items.extend(self.menus.selection_menu(&self.doc));
                items
            }
        };
        if let Some(action) = self.popup(canvas.response(), selection_menu) {
            self.invoke(ui.ctx(), &action);
        }
        self.keep_out_hint(canvas);
    }

    /// Resting the pointer on a keep-out zone in the 2D view: what it is and how to edit it.
    fn keep_out_hint(&self, canvas: &Canvas) {
        let response = canvas.response();
        if response.dragged() || self.drawing_tool().is_some() {
            return;
        }
        if let Some((zone, machine)) = canvas.cursor().and_then(|at| self.doc.keep_out_at(at)) {
            let hint = if machine {
                tr!("views-keep-out-machine-clamp", zone = zone.label())
            } else {
                tr!("views-keep-out-job-zone", zone = zone.label())
            };
            response.clone().on_hover_text(hint);
        }
    }
}
