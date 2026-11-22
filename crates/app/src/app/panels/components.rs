//! The Components list: its levels, rows and the arrows that open them.

use super::{CutCraftApp, EntityId, HoverHint, egui, gui, tr};

impl CutCraftApp {
    /// The arrows over the tree: the picked component up or down its level, as
    /// (component, direction, the id it moves to) when one is clicked.
    pub(in super::super) fn tree_arrows(
        &self,
        ui: &mut egui::Ui,
    ) -> Option<(EntityId, cmd_reorder_component::Direction, EntityId)> {
        use cmd_reorder_component::{Direction, neighbour};
        let mut moved = None;
        let last = self.doc.selected_components.last();
        let arrows = ui.horizontal(|ui| {
            for (label, direction) in [
                (tr!("panels-move-up"), Direction::Up),
                (tr!("panels-move-down"), Direction::Down),
            ] {
                let to = last.and_then(|id| neighbour(&self.doc, id, direction));
                if ui
                    .add_enabled(to.is_some(), egui::Button::new(label))
                    .clicked()
                {
                    moved = last.zip(to).map(|(id, to)| (id, direction, to));
                }
            }
        });
        crate::marks::note(ui, "tool:reorder_component", arrows.response.rect);
        moved
    }

    /// One component's row in the tree: whether a click picked it (Shift: adds or takes
    /// away; a right-click on an unpicked one picks it) and any action from its menu.
    pub(in super::super) fn component_row(
        &self,
        ui: &mut egui::Ui,
        id: EntityId,
        component: &cut_craft_core::component::Component,
    ) -> (Option<(EntityId, bool)>, Option<String>) {
        let text = format!("{} ({:?})", component.name, component.combine);
        let on = self.doc.selected_components.contains(&id);
        let row = ui.selectable_label(on, text);
        crate::marks::note(ui, &format!("component:{}", component.name), row.rect);
        let mut picked = None;
        if row.clicked() {
            picked = Some((id, ui.input(|i| i.modifiers.shift)));
        }
        if row.secondary_clicked() && !on {
            picked = Some((id, false));
        }
        let action = self.popup(&row, || gui::component_menu(&self.doc, id, component));
        (picked, action)
    }

    /// The component tree, top first: each level (the last on top) with its components
    /// under it, the model being built from the bottom up. the level and component
    /// menus on a right-click; right-clicking an unpicked component picks it first.
    /// A level's row in the component tree: Expand or Collapse, then its name and combine
    /// mode; the name's response (for its menu) and whether the icon was clicked.
    pub(in super::super) fn level_header(
        &self,
        ui: &mut egui::Ui,
        level: &cut_craft_core::component::Level,
        collapsed: bool,
    ) -> (egui::Response, bool) {
        let shown = if level.visible { "" } else { " (hidden)" };
        ui.horizontal(|ui| {
            let (icon, tip) = if collapsed {
                ("view/expand.png", "Expand")
            } else {
                ("view/collapse.png", "Collapse")
            };
            let edge = ui.spacing().interact_size.y;
            let toggled = ui
                .add(self.icon_button(icon, "", tip, edge))
                .on_hover_hint(tip)
                .clicked();
            let name = format!("{}{shown} ({:?})", level.name, level.combine);
            (ui.label(egui::RichText::new(name).strong()), toggled)
        })
        .inner
    }

    #[expect(clippy::too_many_lines, reason = "the tree, its rows and their menus")]
    pub(in super::super) fn component_list(&mut self, ui: &mut egui::Ui) {
        // the tree is the active sheet's
        if let Some(sheet) = self.doc.sheets.get(self.doc.active_sheet) {
            ui.strong(tr!(
                "panels-components-of-sheet",
                sheet = sheet.name.as_str()
            ));
        }
        if self.doc.components.is_empty() {
            ui.weak(tr!("panels-no-components-yet"));
        }
        let components = &self.doc.components;
        self.doc
            .selected_components
            .retain(|id| components.contains_key(id));
        let first = self.doc.levels.keys().next().copied().unwrap_or(0);
        let on_level = |k: &cut_craft_core::component::Component, level: u64| {
            let at = if self.doc.levels.contains_key(&k.level) {
                k.level
            } else {
                first
            };
            at == level
        };
        let (mut chosen, mut picked) = (None, None);
        let moved = self.tree_arrows(ui);
        let mut toggle = None;
        for (&level_id, level) in self.doc.levels.iter().rev() {
            let collapsed = self.collapsed_levels.contains(&level_id);
            let (header, toggled) = self.level_header(ui, level, collapsed);
            crate::marks::note(ui, &format!("level:{}", level.name), header.rect);
            if toggled {
                toggle = Some(level_id);
            }
            if let Some(action) =
                self.popup(&header, || gui::level_menu(&self.doc, level_id, level))
            {
                chosen = Some(action);
            }
            if collapsed {
                continue;
            }
            ui.indent(("level", level_id), |ui| {
                let on_this: Vec<(EntityId, &cut_craft_core::component::Component)> = self
                    .doc
                    .components
                    .iter()
                    .rev()
                    .filter(|(_, k)| on_level(k, level_id))
                    .map(|(&id, k)| (id, k))
                    .collect();
                let mut shown_groups = vec![];
                for &(id, component) in &on_this {
                    let Some(group) = &component.group else {
                        let (pick, action) = self.component_row(ui, id, component);
                        picked = pick.or(picked);
                        chosen = action.or(chosen.take());
                        continue;
                    };
                    // a group shows once, where its top member is, its members under it
                    if shown_groups.contains(&group.id) {
                        continue;
                    }
                    shown_groups.push(group.id);
                    let header = ui.label(format!("{} ({:?})", group.name, group.combine));
                    crate::marks::note(ui, &format!("group:{}", group.name), header.rect);
                    if let Some(action) = self.popup(&header, || gui::group_menu(group)) {
                        chosen = Some(action);
                    }
                    ui.indent(("group", group.id), |ui| {
                        for &(member, k) in &on_this {
                            if k.group.as_ref().is_some_and(|g| g.id == group.id) {
                                let (pick, action) = self.component_row(ui, member, k);
                                picked = pick.or(picked);
                                chosen = action.or(chosen.take());
                            }
                        }
                    });
                }
            });
        }
        // click picks one; Shift-click adds or takes away
        match picked {
            Some((id, true)) => {
                if !self.doc.selected_components.remove(&id) {
                    self.doc.selected_components.insert(id);
                }
            }
            Some((id, false)) => self.doc.selected_components = [id].into(),
            None => {}
        }
        if let Some(level) = toggle
            && !self.collapsed_levels.remove(&level)
        {
            self.collapsed_levels.insert(level);
        }
        if let Some((component, direction, to)) = moved {
            let params = cmd_reorder_component::Params {
                component,
                direction,
            };
            let params = serde_json::to_value(params).expect("params serialize");
            if self.apply("reorder_component", &params).is_ok() {
                // the moved component took its neighbour's id; keep it picked
                self.doc.selected_components = [to].into();
            }
        }
        if let Some(action) = chosen {
            self.invoke(ui.ctx(), &action);
        }
    }
}
