//! The keyboard and clipboard: nudging and deleting nodes, shortcuts and their bindings, and copy,
//! cut and paste.

use super::{
    Change, CutCraftApp, EntityId, Key, KeyboardShortcut, Modifiers, Page, Point, Screen, Shown,
    egui, files, on_the_desktop, shortcut,
};

impl CutCraftApp {
    /// Lets go of picked nodes that no longer stand for a node to move: all of them once
    /// Node Edit is put away, and any whose vector is no longer selected or has no node
    /// where it was picked any more (after an undo, redo or delete).
    pub(super) fn prune_picked_nodes(&mut self) {
        if self.picked_nodes.is_empty() {
            return;
        }
        if self.view_tool != Some(super::ViewTool::NodeEdit) {
            self.picked_nodes.clear();
            return;
        }
        let doc = &self.doc;
        self.picked_nodes.retain(|n| {
            doc.selection.contains(&n.vector)
                && doc.vectors.get(&n.vector).is_some_and(|path| {
                    cut_craft_core::nodes::node_near(path, n.at.into(), n.reach.max(1e-3)).is_some()
                })
        });
    }

    /// Moves the picked nodes by `by`; across a mirror line (H: up and down through the
    /// selection's middle; V: across it) the nodes on the far side move the other way, so
    /// mirrored pairs stay mirrored.
    pub(super) fn nudge_nodes(&mut self, by: Point) {
        use cut_craft_core::nodes::{move_node, node_near};
        let Some(middle) = self
            .doc
            .selection_bounds()
            .map(cut_craft_core::geometry::Rect::center)
        else {
            return;
        };
        let [mirror_h, mirror_v] = self.node_mirror;
        let delta = |at: Point| {
            Point::new(
                if mirror_h && at.x > middle.x {
                    -by.x
                } else {
                    by.x
                },
                if mirror_v && at.y > middle.y {
                    -by.y
                } else {
                    by.y
                },
            )
        };
        let mut c = Change::builder(&self.doc);
        let vectors: std::collections::BTreeSet<EntityId> =
            self.picked_nodes.iter().map(|n| n.vector).collect();
        for vector in vectors {
            let Some(original) = self.doc.vectors.get(&vector) else {
                continue;
            };
            let mut path = original.clone();
            for n in self.picked_nodes.iter().filter(|n| n.vector == vector) {
                let at = Point::from(n.at);
                if let Some(k) = node_near(original, at, n.reach.max(1e-3)) {
                    path = move_node(&path, k, at + delta(at));
                }
            }
            c.replace(vector, path);
        }
        let change = c.finish();
        match self.history.apply(&mut self.doc, change) {
            Ok(()) => {
                if let Some(note) = self.history.last_notes().last() {
                    self.status.set_message(note);
                }
                for n in &mut self.picked_nodes {
                    let at = Point::from(n.at) + delta(Point::from(n.at));
                    n.at = cut_craft_core::params::Xy::new(at.x, at.y);
                }
            }
            Err(e) => self.status.set_message(&e.to_string()),
        }
    }

    /// Del in Node Edit: the picked nodes go, each shape's spans either side joined by a
    /// line, as Delete Point does, in one step to undo. A shape keeps the nodes it needs
    /// (two, or three when closed); the status bar says when some were kept for that.
    pub(super) fn delete_picked_nodes(&mut self) {
        use cut_craft_core::nodes::{delete_node, node_near};
        let mut c = Change::builder(&self.doc);
        let vectors: std::collections::BTreeSet<EntityId> =
            self.picked_nodes.iter().map(|n| n.vector).collect();
        let (mut deleted, mut kept) = (0, 0);
        for vector in vectors {
            let Some(original) = self.doc.vectors.get(&vector) else {
                continue;
            };
            let mut picked: Vec<usize> = (self.picked_nodes.iter())
                .filter(|n| n.vector == vector)
                .filter_map(|n| node_near(original, n.at.into(), n.reach.max(1e-3)))
                .collect();
            // the last first, so the nodes still to go keep their places
            picked.sort_unstable_by(|a, b| b.cmp(a));
            picked.dedup();
            let mut path = original.clone();
            for k in picked {
                match delete_node(&path, k) {
                    Some(fewer) => {
                        path = fewer;
                        deleted += 1;
                    }
                    None => kept += 1,
                }
            }
            if path != *original {
                c.replace(vector, path);
            }
        }
        let change = c.finish();
        self.picked_nodes.clear();
        if deleted > 0
            && let Err(e) = self.history.apply(&mut self.doc, change)
        {
            self.status.set_message(&e.to_string());
            return;
        }
        let s = |n: usize| if n == 1 { "" } else { "s" };
        let mut said = format!("Deleted {deleted} node{}", s(deleted));
        if kept > 0 {
            said = format!(
                "{said}. {kept} node{} kept: a shape needs at least two (three when \
                 closed). Delete the whole shape instead",
                s(kept)
            );
        }
        self.status.set_message(&said);
    }

    /// The arrow keys nudge the selection by the snapping grid's squares: a whole one, half
    /// with Shift, a tenth with Ctrl + Shift (Matthew's steps); with Ctrl alone, by Snapping Options' Fixed Nudge Distance.
    pub(super) fn nudge(&mut self, ctx: &egui::Context) {
        if self.screen != Screen::Job || ctx.egui_wants_keyboard_input() {
            return;
        }
        let (shift, ctrl) = ctx.input(|i| (i.modifiers.shift, i.modifiers.command));
        let g = self.options.snapping.grid_spacing;
        let step = match (shift, ctrl) {
            (true, true) => 0.1 * g,
            (true, false) => 0.5 * g,
            // Snapping Options' Fixed Nudge Distance
            (false, true) => self.options.snapping.fixed_nudge,
            (false, false) => g,
        };
        let by = ctx.input(|i| {
            [
                (Key::ArrowLeft, -1.0, 0.0),
                (Key::ArrowRight, 1.0, 0.0),
                (Key::ArrowDown, 0.0, -1.0),
                (Key::ArrowUp, 0.0, 1.0),
            ]
            .iter()
            .filter(|(key, ..)| i.key_pressed(*key))
            .fold(Point::ZERO, |sum, &(_, x, y)| sum + Point::new(x, y) * step)
        });
        if by == Point::ZERO {
            return;
        }
        // node editing with nodes picked: the arrows move those, mirrored across the lines
        self.prune_picked_nodes();
        if self.view_tool == Some(super::ViewTool::NodeEdit) && !self.picked_nodes.is_empty() {
            self.nudge_nodes(by);
            return;
        }
        if self.doc.selection_bounds().is_none() {
            return;
        }
        let params =
            serde_json::json!({ "relative": true, "x": by.x, "y": by.y, "anchor": "bottom_left" });
        if let Err(e) = self.apply("move", &params) {
            self.status.set_message(&e);
        }
    }

    /// Ctrl+X, Ctrl+C and Ctrl+V, which egui gives as clipboard events rather than keys.
    fn clipboard_keys(&mut self, ctx: &egui::Context) {
        // Ctrl+X, Ctrl+C and Ctrl+V reach egui as clipboard events, never as keys; the system
        // clipboard carries what Copy took, so another Cut Craft window pastes it too
        let (cut, copy, pasted) = ctx.input(|i| {
            i.raw
                .events
                .iter()
                .fold((false, false, None), |(x, c, p), e| match e {
                    egui::Event::Cut => (true, c, p),
                    egui::Event::Copy => (x, true, p),
                    egui::Event::Paste(text) => (x, c, Some(text.clone())),
                    _ => (x, c, p),
                })
        });
        // each only while Options still gives the action its key (taken away, it does nothing)
        let bindings = self.bindings(ctx);
        let bound = |id: &str, key: Key| {
            let keys = KeyboardShortcut::new(Modifiers::COMMAND, key);
            bindings.contains(&(id.to_owned(), shortcut::Trigger::Key(keys)))
        };
        let (cut, copy) = (cut && bound("cut", Key::X), copy && bound("copy", Key::C));
        let pasted = pasted.filter(|_| bound("paste", Key::V));
        if !ctx.egui_wants_keyboard_input() && self.screen == Screen::Job {
            if cut && self.available("cut") {
                self.invoke(ctx, "cut");
            } else if copy && self.available("copy") {
                self.invoke(ctx, "copy");
            }
            if let Some(paths) = pasted.and_then(|text| serde_json::from_str(&text).ok()) {
                // vectors another window copied: what they belonged to there is not known,
                // and this window's own copied components are not part of them
                if paths != self.clipboard {
                    self.clipboard_from = super::Copied::default();
                    self.clipboard_components.clear();
                }
                self.clipboard = paths;
                self.invoke(ctx, "paste");
            }
        }
    }

    pub(super) fn shortcuts(&mut self, ctx: &egui::Context) {
        // while a slow job runs, the window only shows its progress; while a shortcut is
        // being set, the keys and buttons pressed are for it
        if self.busy.is_some() || self.capture.is_some() {
            return;
        }
        self.clipboard_keys(ctx);
        // node editing with nodes picked: H and V toggle the mirror lines, not Mirror
        if !ctx.egui_wants_keyboard_input()
            && self.view_tool == Some(super::ViewTool::NodeEdit)
            && !self.picked_nodes.is_empty()
        {
            for (key, line) in [(Key::H, 0), (Key::V, 1)] {
                let shortcut = KeyboardShortcut::new(Modifiers::NONE, key);
                if ctx.input_mut(|i| i.consume_shortcut(&shortcut)) {
                    self.node_mirror[line] = !self.node_mirror[line];
                }
            }
        }
        self.run_shortcuts(ctx);
        // Escape during a 3D handle drag cancels only the drag, staying in Transform mode
        let escape = KeyboardShortcut::new(Modifiers::NONE, Key::Escape);
        if self.handle_drag.is_held()
            && !ctx.egui_wants_keyboard_input()
            && ctx.input_mut(|i| i.consume_shortcut(&escape))
        {
            self.cancel_drag_3d();
        }
        // Escape with an open end picked lets the end go, keeping Join Open Ends open
        if self.join_ends_first().is_some()
            && !ctx.egui_wants_keyboard_input()
            && ctx.input_mut(|i| i.consume_shortcut(&escape))
        {
            self.join_ends_forget();
            self.status
                .set_message(&cut_craft_i18n::tr!("status-end-let-go"));
        }
        // Escape stops a preview playing (keeping what it has cut), and does nothing else
        if self.player.playback.is_some()
            && !ctx.egui_wants_keyboard_input()
            && ctx.input_mut(|i| i.consume_shortcut(&escape))
        {
            self.finish_playback();
            self.status
                .set_message(&cut_craft_i18n::tr!("status-preview-stopped"));
        }
        // Escape with no form open, stepped inside a group, steps out one level
        if let Some(group) = self.doc.inside_group
            && self.form.is_none()
            && !ctx.egui_wants_keyboard_input()
            && ctx.input_mut(|i| i.consume_shortcut(&escape))
        {
            self.step_out_of_group(group);
        }
        // Escape in a field being typed in only leaves the field
        if !ctx.egui_wants_keyboard_input()
            && ctx.input_mut(|i| {
                i.consume_shortcut(&KeyboardShortcut::new(Modifiers::NONE, Key::Escape))
            })
        {
            // a window in front (Find Tool, the library's Properties) closes alone
            let in_front = self.search.take().is_some() || self.clipart.close_properties();
            if !in_front {
                self.escape_closes();
            }
        }
        self.nudge(ctx);
    }

    /// Whether `id` is a placing tool (Transform Objects: Move, Rotate, Set Size, Mirror,
    /// Distort), which Esc leaves for the form it was opened over.
    pub(super) fn placing(&self, id: &str) -> bool {
        self.registry
            .get(id)
            .is_some_and(|c| c.meta().group == "Transform Objects")
    }

    /// Esc with nothing in front: the open form closes (Import 3D Model's as its Cancel), with
    /// a new job's setup, the view tool and the pages it opened. A placing tool's form opened
    /// over another goes back to that one, edits and all; what it placed stays placed (the
    /// toolpaths it changes are out of date, as after any move).
    fn escape_closes(&mut self) {
        let behind = self.form_behind.take();
        if let Some(mut behind) = behind
            && self.form.as_ref().is_some_and(|f| self.placing(f.id))
        {
            // what was placed is kept: Cancel on the form back now leaves it
            behind.undo_from = self.history.undo_depth();
            behind.list_steps.clear();
            if let Some(placing) = self.form.take().filter(|f| f.page != behind.page) {
                self.pages.done_with(placing.page);
            }
            if self.pages.shown(behind.page) == Shown::Hidden {
                self.pages.open(behind.page);
            }
            self.status.current_tool =
                crate::tool_names::form_name(&self.registry, behind.id, behind.label);
            self.status.set_message(&cut_craft_i18n::tr!(
                "status-back-to",
                tool = self.status.current_tool.clone()
            ));
            self.form = Some(behind);
            return;
        }
        if self.screen == Screen::NewJob {
            self.job_setup_cancel();
        }
        // a form whose buttons are Import and Cancel: Esc is its Cancel
        if self
            .form
            .as_ref()
            .is_some_and(|f| f.id == "orientate_3d_model")
        {
            self.cancel_form();
        }
        self.form = None;
        self.view_tool = None;
        self.pages.done_with(Page::Design);
        self.pages.done_with(Page::Toolpaths);
    }

    /// Runs the actions whose shortcut was pressed: a key's, or a mouse button's.
    fn run_shortcuts(&mut self, ctx: &egui::Context) {
        // single-letter shortcuts must not fire while typing in a form field; a mouse
        // button's can, as it types nothing
        let typing = ctx.egui_wants_keyboard_input();
        let job = self.screen == Screen::Job;
        let fired: Vec<String> = shortcut::most_specific_first(self.bindings(ctx))
            .into_iter()
            .filter(|(id, _)| id != shortcut::PAN_DRAG)
            .filter(|(id, _)| job || ["new", "open", "tool_help"].contains(&id.as_str()))
            .filter(|(id, trigger)| match trigger {
                // help and the views work from a field being typed in too, as they always did,
                // while on a function key (which types nothing)
                shortcut::Trigger::Key(keys) => {
                    let name = keys.logical_key.name();
                    let function = name.len() > 1 && name.starts_with('F');
                    let any_time =
                        function && ["tool_help", "view_2d", "view_3d"].contains(&id.as_str());
                    (!typing || any_time) && ctx.input_mut(|i| i.consume_shortcut(keys))
                }
                shortcut::Trigger::Mouse(held, button) => ctx.input(|i| {
                    i.pointer.button_pressed(*button) && i.modifiers.matches_exact(*held)
                }),
            })
            .map(|(id, _)| id)
            .collect();
        for id in fired {
            self.invoke(ctx, &id);
        }
    }

    /// Every action's own shortcuts: each command's, each built-in action's, and dragging with
    /// the wheel pressed to pan.
    pub(super) fn default_bindings(&self) -> Vec<(String, shortcut::Trigger)> {
        let commands = self
            .registry
            .iter()
            .filter_map(|c| Some((c.id().to_owned(), shortcut::parse(c.meta().shortcut)?)));
        let built_ins = (crate::app::built_ins::BUILT_INS.iter())
            .filter_map(|b| Some((b.id.to_owned(), shortcut::parse(b.keys)?)));
        let keys = commands.chain(built_ins);
        let mut bindings: Vec<_> = keys
            .map(|(id, keys)| (id, shortcut::Trigger::Key(keys)))
            .collect();
        bindings.extend(
            shortcut::parse_trigger(shortcut::PAN_DRAG_DEFAULT)
                .map(|t| (shortcut::PAN_DRAG.to_owned(), t)),
        );
        bindings
    }

    /// Every shortcut in use: [`Self::default_bindings`] with those set in Options in their
    /// place, and on a Mac its own keys for some of them ([`super::mac::extra_bindings`]).
    pub(super) fn bindings(&self, ctx: &egui::Context) -> Vec<(String, shortcut::Trigger)> {
        self.bindings_with(ctx, &self.options.controls.shortcuts)
    }

    /// [`Self::bindings`] with `set` as the shortcuts set in Options: those in the Options
    /// window, not applied yet, when a new one is checked for clashes.
    pub(super) fn bindings_with(
        &self,
        ctx: &egui::Context,
        set: &std::collections::BTreeMap<String, Vec<String>>,
    ) -> Vec<(String, shortcut::Trigger)> {
        let mut bindings = shortcut::effective(self.default_bindings(), set);
        if super::mac::is_mac(ctx) {
            // ⌘Q quits there: Quick Engrave's Ctrl+Q is no shortcut on a Mac
            bindings.retain(
                |(_, t)| !matches!(t, shortcut::Trigger::Key(k) if super::mac::kept_by_the_mac(*k)),
            );
            let keys: Vec<(String, KeyboardShortcut)> = bindings
                .iter()
                .filter_map(|(id, t)| match t {
                    shortcut::Trigger::Key(k) => Some((id.clone(), *k)),
                    shortcut::Trigger::Mouse(..) => None,
                })
                .collect();
            let extra = super::mac::extra_bindings(&keys);
            bindings.extend(
                extra
                    .into_iter()
                    .map(|(id, k)| (id, shortcut::Trigger::Key(k))),
            );
        }
        bindings
    }

    /// The mouse buttons that pan either view while held and dragged.
    pub(super) fn pan_buttons(&self) -> Vec<egui::PointerButton> {
        let defaults = (shortcut::parse_trigger(shortcut::PAN_DRAG_DEFAULT))
            .map(|t| (shortcut::PAN_DRAG.to_owned(), t));
        shortcut::effective(
            defaults.into_iter().collect(),
            &self.options.controls.shortcuts,
        )
        .into_iter()
        .filter(|(id, _)| id == shortcut::PAN_DRAG)
        .filter_map(|(_, t)| t.button())
        .collect()
    }

    /// Whether a pan button ([`Self::pan_buttons`]) is held down.
    pub(super) fn pan_button_down(&self, ctx: &egui::Context) -> bool {
        let buttons = self.pan_buttons();
        ctx.input(|i| buttons.iter().any(|&b| i.pointer.button_down(b)))
    }

    /// Every action a shortcut can be set for, by id, with its name: panning by dragging first,
    /// then the commands and the built-in actions by name.
    pub(super) fn shortcut_actions(&self) -> Vec<(String, String)> {
        let mut actions: Vec<(String, String)> = vec![];
        actions.extend(
            self.registry
                .iter()
                .map(|c| (c.id().to_owned(), self.tool_label(c.id()))),
        );
        for b in crate::app::built_ins::BUILT_INS {
            if !actions.iter().any(|(a, _)| a == b.id) {
                actions.push((b.id.to_owned(), self.tool_label(b.id)));
            }
        }
        actions.sort_by(|a, b| a.1.cmp(&b.1));
        // panning first: the mouse's own, and what most people come here to set
        let pan = (
            shortcut::PAN_DRAG.to_owned(),
            "Pan (hold and drag)".to_owned(),
        );
        actions.insert(0, pan);
        actions
    }

    /// The shortcut a menu shows for `id`: the first one set in Options, else the command's or
    /// the built-in action's own; `None` leaves the menu item's own.
    pub(super) fn shortcut_text(&self, id: &str) -> Option<String> {
        match self.options.controls.shortcuts.get(id) {
            Some(set) => Some(set.first().cloned().unwrap_or_default()),
            None => (self.registry.get(id).map(|c| c.meta().shortcut))
                .or_else(|| crate::app::built_ins::get(id).map(|b| b.keys))
                .map(str::to_owned),
        }
    }

    /// Copy, or Cut (`cut`): the selected vectors and 3D parts onto the clipboard, the
    /// vectors as text too so they paste into another window.
    pub(super) fn copy_selection(&mut self, ctx: &egui::Context, cut: bool) {
        self.clipboard = self
            .doc
            .selection
            .iter()
            .map(|id| self.doc.vectors[id].clone())
            .collect();
        let doc = &self.doc;
        let picked: std::collections::BTreeSet<EntityId> = doc.selection.iter().copied().collect();
        let groups: Vec<Vec<u64>> = doc
            .selection
            .iter()
            .map(|&id| doc.chain_within(id, &picked))
            .collect();
        self.clipboard_from = super::Copied {
            layers: doc.selection.iter().map(|&id| doc.layer_of(id)).collect(),
            job: self.job_epoch,
            texts: records_of(&groups, &doc.texts),
            names: records_of(&groups, &doc.part_names),
            groups,
        };
        self.clipboard_components = self
            .doc
            .selected_components
            .iter()
            .filter_map(|id| self.doc.components.get(id).cloned())
            .collect();
        let text = serde_json::to_string(&self.clipboard).expect("paths serialize");
        ctx.copy_text(text);
        if cut && let Err(e) = self.apply("delete", &serde_json::json!({})) {
            self.status.set_message(&e);
        }
    }

    /// Paste and Paste in Place: pasted where they were copied from, in this job or another.
    pub(super) fn paste_clipboard(&mut self) {
        let mut c = Change::builder(&self.doc);
        // a copied group pastes as a group of its own, on its layer, text still text
        let from = &self.clipboard_from;
        let ids = if from.groups.len() == self.clipboard.len() {
            let paths = self.clipboard.iter().cloned().zip(from.groups.clone());
            let shift = Some(Point::ZERO);
            c.add_regrouped(paths.collect(), &from.texts, &from.names, shift)
        } else {
            self.clipboard.iter().map(|p| c.add(p.clone())).collect()
        };
        // on the layer it came from in this job; in another, on the current layer
        let layers = if from.job == self.job_epoch {
            from.layers.as_slice()
        } else {
            &[]
        };
        for (&id, layer) in ids.iter().zip(layers) {
            if self.doc.layer(*layer).is_some() {
                c.move_to_layer(id, *layer);
            }
        }
        c.select(ids);
        for k in &self.clipboard_components {
            c.add_component(k.clone());
        }
        self.record(Ok(c.finish()), "Pasted");
    }

    /// Increment and Save: saved under the next free numbered name.
    pub(super) fn increment_and_save(&mut self) {
        match self.session.path.clone() {
            Some(path) => self.save_to(&files::incremented(&path, std::path::Path::exists)),
            None => self.save_as(),
        }
    }

    /// Print: the drawing as a PDF opened in the system viewer, which previews and prints it
    /// (a print window of our own, with scale and tiling, is not done).
    pub(super) fn print_drawing(&mut self) {
        let path = std::env::temp_dir().join("cut-craft-print.pdf");
        let opened = files::export(&self.registry, "export_pdf", &self.doc, &path).and_then(|()| {
            on_the_desktop();
            files::show_folder(&path).map_err(|e| e.to_string())
        });
        if let Err(e) = opened {
            self.status.set_message(&cut_craft_i18n::tr!(
                "status-could-not-print",
                error = e.clone()
            ));
        }
    }

    pub(super) fn open_application_data_folder(&mut self) {
        let opened = files::config_dir()
            .ok_or_else(|| "no home folder".to_owned())
            .and_then(|dir| {
                std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
                on_the_desktop();
                files::show_folder(&dir).map_err(|e| e.to_string())
            });
        if let Err(e) = opened {
            self.status.set_message(&cut_craft_i18n::tr!(
                "status-could-not-open-folder",
                error = e.clone()
            ));
        }
    }

    /// Help ▸ Keyboard Shortcuts' rows: each shortcut in use and what it runs, by name.
    pub(super) fn shortcut_list(&self, ctx: &egui::Context) -> Vec<(String, String)> {
        let mut rows: Vec<(String, String)> = self
            .bindings(ctx)
            .into_iter()
            .map(|(id, trigger)| {
                let label = if id == shortcut::PAN_DRAG {
                    "Pan (hold and drag)".to_owned()
                } else {
                    self.tool_label(&id)
                };
                let keys = match trigger {
                    shortcut::Trigger::Key(keys) => ctx.format_shortcut(&keys),
                    mouse @ shortcut::Trigger::Mouse(..) => mouse.text(),
                };
                (keys, label)
            })
            .collect();
        rows.sort_by(|a, b| a.1.cmp(&b.1));
        rows
    }
}

/// The entries of `records` (text settings or part names, by group) for `groups`.
pub(super) fn records_of<T: Clone>(
    groups: &[Vec<u64>],
    records: &std::collections::BTreeMap<u64, T>,
) -> std::collections::BTreeMap<u64, T> {
    groups
        .iter()
        .flatten()
        .filter_map(|g| Some((*g, records.get(g).cloned()?)))
        .collect()
}
