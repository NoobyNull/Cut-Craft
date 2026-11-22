//! Node editing in the 2D view: the nodes' overlay, picking, dragging and dropping nodes.

use super::{CutCraftApp, Drag, Gesture, Overlay, Path, Point, Rect, Value};

impl CutCraftApp {
    /// Node handles on the selected vectors, their Bézier handles, and the dragged node's
    /// (or handle's) vector as it would be.
    pub(super) fn node_overlay(&self, gesture: Option<Gesture>, reach: f64) -> Overlay {
        // the dragged node's vector as it would be, drawn over the old one
        let mut moving: Vec<Path> = match gesture {
            Some(Gesture::Dragging(drag)) if self.handle_move(drag, reach).is_some() => self
                .handle_move(drag, reach)
                .and_then(|params| {
                    let cmd = self.registry.get("move_handle")?;
                    cmd.run_json(&self.doc, &params).ok()
                })
                .map_or_default(|change| change.added().cloned().collect()),
            Some(Gesture::Dragging(drag)) if self.picked_drag(drag, reach).is_some() => self
                .picked_drag(drag, reach)
                .and_then(|params| {
                    let cmd = self.registry.get("move_nodes")?;
                    cmd.run_json(&self.doc, &params).ok()
                })
                .map_or_default(|change| change.added().cloned().collect()),
            Some(Gesture::Dragging(drag)) => self
                .node_target(drag.start, reach)
                .and_then(|t| {
                    let path = &self.doc.vectors[&t.vector];
                    let k = cut_craft_core::nodes::node_near(path, drag.start, reach)?;
                    Some(cut_craft_core::nodes::move_node(path, k, drag.end))
                })
                .into_iter()
                .collect(),
            _ => vec![],
        };
        let at = self
            .doc
            .selection
            .iter()
            .flat_map(|id| cut_craft_core::nodes::nodes(&self.doc.vectors[id]))
            .collect();
        // the mirror lines, through the selection's middle and past its box
        if let Some(b) = self.doc.selection_bounds() {
            let (m, out) = (b.center(), b.size().max_element() / 4.0 + reach);
            if self.node_mirror[0] {
                let ends = [
                    Point::new(m.x, b.min.y - out),
                    Point::new(m.x, b.max.y + out),
                ];
                moving.push(Path::polyline(&ends, false));
            }
            if self.node_mirror[1] {
                let ends = [
                    Point::new(b.min.x - out, m.y),
                    Point::new(b.max.x + out, m.y),
                ];
                moving.push(Path::polyline(&ends, false));
            }
        }
        let picked = self.picked_nodes.iter().map(|n| n.at.into()).collect();
        let handles = self
            .doc
            .selection
            .iter()
            .filter(|id| self.doc.editable(**id))
            .flat_map(|id| cut_craft_core::nodes::handles(&self.doc.vectors[id]))
            .map(|h| (h.node, h.at))
            .collect();
        Overlay::Nodes {
            at,
            picked,
            moving,
            handles,
        }
    }

    /// How far `at` is from the nearest Bézier handle of the selected, editable vectors,
    /// when that handle is nearer than every node of them (a tie goes to the node).
    pub(super) fn nearest_handle(&self, at: Point) -> Option<f64> {
        let editable = || {
            self.doc
                .selection
                .iter()
                .filter(|id| self.doc.editable(**id))
                .map(|id| &self.doc.vectors[id])
        };
        let d = editable()
            .flat_map(cut_craft_core::nodes::handles)
            .map(|h| h.at.distance(at))
            .min_by(f64::total_cmp)?;
        let node_nearer = editable()
            .flat_map(cut_craft_core::nodes::nodes)
            .any(|q| q.distance(at) <= d);
        (!node_nearer).then_some(d)
    }

    /// Move Handle's params when `drag` starts on a selected vector's Bézier handle (nearer
    /// it than to any node: a tie goes to the node, so a handle lying on its node leaves the node draggable).
    pub(super) fn handle_move(&self, drag: Drag, reach: f64) -> Option<Value> {
        use cut_craft_core::nodes::{handle_near, nodes};
        let (vector, d) = self
            .doc
            .selection
            .iter()
            .filter(|id| self.doc.editable(**id))
            .filter_map(|&id| {
                let h = handle_near(&self.doc.vectors[&id], drag.start, reach)?;
                Some((id, h.at.distance(drag.start)))
            })
            .min_by(|a, b| a.1.total_cmp(&b.1))?;
        let node_nearer = self.doc.selection.iter().any(|id| {
            nodes(&self.doc.vectors[id])
                .iter()
                .any(|q| q.distance(drag.start) <= d)
        });
        if node_nearer {
            return None;
        }
        let from = cut_craft_core::nodes::NodeAt {
            vector,
            at: cut_craft_core::params::Xy::new(drag.start.x, drag.start.y),
            reach,
        };
        let to = cut_craft_core::params::Xy::new(drag.end.x, drag.end.y);
        Some(serde_json::json!({ "from": from, "to": to }))
    }

    /// Move Nodes' params when `drag` starts on one of two or more picked nodes.
    pub(super) fn picked_drag(&self, drag: Drag, reach: f64) -> Option<Value> {
        let on_one = self
            .picked_nodes
            .iter()
            .any(|n| Point::from(n.at).distance(drag.start) <= reach);
        (self.picked_nodes.len() > 1 && on_one).then(|| {
            let by = drag.end - drag.start;
            serde_json::json!({
                "nodes": self.picked_nodes,
                "by": cut_craft_core::params::Xy::new(by.x, by.y),
            })
        })
    }

    /// The nodes of the selected, editable vectors inside the box `rect`, for Move Nodes.
    pub(super) fn nodes_in(&self, rect: Rect) -> Vec<cut_craft_core::nodes::NodeAt> {
        self.doc
            .selection
            .iter()
            .filter(|id| self.doc.editable(**id))
            .flat_map(|&vector| {
                cut_craft_core::nodes::nodes(&self.doc.vectors[&vector])
                    .into_iter()
                    .filter(|p| rect.contains(*p))
                    .map(move |p| cut_craft_core::nodes::NodeAt {
                        vector,
                        at: cut_craft_core::params::Xy::new(p.x, p.y),
                        reach: 1e-6,
                    })
            })
            .collect()
    }

    /// A drag from one of several picked nodes moves them all; from a node moves it; a box
    /// round nodes of the selected vectors picks them (Shift adds); any other drag selects,
    /// as with the Select tool.
    pub(super) fn node_drop(&mut self, drag: Drag, shift: bool, reach: f64) {
        // the same order as the preview in `node_overlay`: a handle first
        if let Some(params) = self.handle_move(drag, reach) {
            if let Err(e) = self.apply("move_handle", &params) {
                self.status.set_message(&e);
            }
        } else if let Some(params) = self.picked_drag(drag, reach) {
            let by = drag.end - drag.start;
            match self.apply("move_nodes", &params) {
                Ok(()) => {
                    for n in &mut self.picked_nodes {
                        n.at = cut_craft_core::params::Xy::new(n.at.x + by.x, n.at.y + by.y);
                    }
                }
                Err(e) => self.status.set_message(&e),
            }
        } else if let Some(params) = self.node_move(drag, reach) {
            self.picked_nodes.clear();
            if let Err(e) = self.apply("move_node", &params) {
                self.status.set_message(&e);
            }
        } else if let picked @ [_, ..] = &self.nodes_in(Rect::new(drag.start, drag.end))[..] {
            if !shift {
                self.picked_nodes.clear();
            }
            self.picked_nodes.extend(picked.iter().cloned());
        } else {
            self.picked_nodes.clear();
            let ids = self
                .doc
                .window(Rect::new(drag.start, drag.end), drag.end.x < drag.start.x);
            self.select(ids, shift);
        }
    }

    /// A selected, editable vector with a node or span within `reach` of `at`.
    pub(super) fn node_target(
        &self,
        at: Point,
        reach: f64,
    ) -> Option<cut_craft_core::nodes::NodeAt> {
        self.doc
            .selection
            .iter()
            .find(|id| {
                let path = &self.doc.vectors[id];
                self.doc.editable(**id)
                    && (cut_craft_core::nodes::node_near(path, at, reach).is_some()
                        || cut_craft_core::nodes::span_near(path, at, reach).is_some())
            })
            .map(|&vector| cut_craft_core::nodes::NodeAt {
                vector,
                at: cut_craft_core::params::Xy::new(at.x, at.y),
                reach,
            })
    }

    /// Move Node's params when `drag` starts on a selected vector's node.
    pub(super) fn node_move(&self, drag: Drag, reach: f64) -> Option<Value> {
        let target = self.node_target(drag.start, reach)?;
        cut_craft_core::nodes::node_near(&self.doc.vectors[&target.vector], drag.start, reach)?;
        let to = cut_craft_core::params::Xy::new(drag.end.x, drag.end.y);
        Some(serde_json::json!({ "from": target, "to": to }))
    }
}
