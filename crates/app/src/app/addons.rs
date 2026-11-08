//! The app's side of add-ons: Help ▸ Add-ons, and Model ▸ Depth to STL, which runs the
//! Depth to STL add-on on the chosen picture and adds the relief it makes as a component.

use super::CutCraftApp;
use cut_craft_bug_report::hint::HoverHint as _;
use cut_craft_core::component::{Combine, Component};
use cut_craft_core::document::EntityId;
use eframe::egui;

impl CutCraftApp {
    /// Help ▸ Add-ons.
    pub(super) fn add_ons_window(&mut self, ctx: &egui::Context) {
        let folder = self.addons_path.clone();
        if self.add_ons.window(ctx, folder.as_deref()) && self.depth2stl.open {
            // installed or removed while Depth to STL was open: look again
            self.depth2stl.show(folder.as_deref(), ctx);
        }
    }

    /// Model ▸ Depth to STL: the add-on's window when it is installed, else Cut Craft's own
    /// tool for the picture: Relief from Photo for a colour photo, Create Component from
    /// Bitmap for a depth map.
    pub(super) fn depth_to_stl(&mut self, ctx: &egui::Context) {
        let folder = self.addons_path.clone();
        if folder.as_deref().and_then(crate::depth2stl::find).is_some() {
            self.depth2stl.show(folder.as_deref(), ctx);
            return;
        }
        let photo = self
            .depth2stl_picture()
            .is_some_and(|id| !self.doc.bitmaps[&id].rgb.is_empty());
        let (id, name) = if photo {
            ("relief_from_photo", "Relief from Photo")
        } else {
            (
                "create_component_from_bitmap",
                "Create Component from Bitmap",
            )
        };
        self.invoke(ctx, id);
        self.status.set_message(&format!(
            "Depth to STL is not installed, so {name} is used instead. Help ▸ Add-ons installs it."
        ));
    }

    /// The picture Depth to STL works on: the one picked last, else the newest.
    fn depth2stl_picture(&self) -> Option<EntityId> {
        self.doc
            .selected_bitmaps
            .iter()
            .rev()
            .copied()
            .find(|id| self.doc.bitmaps.contains_key(id))
            .or_else(|| self.doc.bitmaps.keys().next_back().copied())
    }

    /// Model ▸ Depth to STL.
    pub(super) fn depth2stl_window(&mut self, ctx: &egui::Context) {
        self.depth2stl.take_description();
        self.take_depth2stl();
        if !self.depth2stl.open {
            return;
        }
        let chosen = self.depth2stl_picture();
        // a new picture: the relief starts as wide as it is
        if chosen != self.depth2stl.sized_for
            && let Some(id) = chosen
            && let Some(settings) = self.depth2stl.settings.as_object_mut()
            && settings.contains_key("width")
        {
            settings.insert("width".into(), self.doc.bitmaps[&id].rect.size().x.into());
            self.depth2stl.sized_for = chosen;
        }
        let units = self.doc.job.units;
        let material = crate::form::material(&self.doc, units);
        let mut open = true;
        let size = super::dialog_size(ctx);
        let w = &mut self.depth2stl;
        let picture = chosen.map(|id| &self.doc.bitmaps[&id]);
        let mut pressed = Pressed::default();
        cut_craft_bug_report::dialog(
            ctx,
            egui::Id::unique("depth_to_stl"),
            "Depth to STL",
            size,
            &mut open,
            |ui| pressed = depth2stl_form(ui, w, picture, units, material),
        );
        let Pressed { start, get, reset } = pressed;
        self.depth2stl.open = open;
        if get {
            self.add_ons.show(ctx, self.addons_path.as_deref());
        }
        if reset && let Some(Ok(d)) = &self.depth2stl.described {
            self.depth2stl.settings = d.defaults();
            self.depth2stl.sized_for = None;
        }
        if start && let (Some(program), Some(id)) = (self.depth2stl.program.clone(), chosen) {
            let w = &mut self.depth2stl;
            w.message = None;
            match crate::depth2stl::Running::start(
                &program,
                id,
                &self.doc.bitmaps[&id],
                &w.settings,
                ctx,
            ) {
                Ok(running) => w.running = Some(running),
                Err(e) => w.message = Some(e),
            }
        }
    }

    /// Adds the relief once Depth to STL answers (undoable), centred on the picture it was
    /// made from.
    fn take_depth2stl(&mut self) {
        let Some(answer) = self
            .depth2stl
            .running
            .as_mut()
            .and_then(crate::depth2stl::Running::answer)
        else {
            return;
        };
        let sent = self.depth2stl.running.take().expect("answered");
        let result = answer.and_then(|made| {
            // another job opened, or the picture edited or deleted, while the add-on worked:
            // the relief is for that picture alone
            if self.doc.bitmaps.get(&sent.bitmap) != Some(&sent.photo) {
                return Err(
                    "the picture changed while Depth to STL worked on it; try again".to_owned(),
                );
            }
            let centre = sent.photo.rect.center();
            let heights = made.bitmap("Depth to STL", centre);
            let relief = Component::from_fn(
                &format!("{} relief", sent.photo.name),
                Combine::Add,
                heights.rect,
                self.doc.job.model_cell(),
                |at| heights.tone(at).map(|t| t * made.top),
            );
            let mut c = cut_craft_core::Change::builder(&self.doc);
            c.add_component(relief);
            let change = c.finish();
            self.history
                .apply(&mut self.doc, change)
                .map_err(|e| e.to_string())?;
            Ok(made)
        });
        let message = match result {
            Ok(made) => format!(
                "Added a {:.1} × {:.1} × {:.2} mm relief ({} × {} points).{}",
                made.width,
                made.height,
                made.top,
                made.columns,
                made.rows,
                made.bit.map(|b| format!(" Bit: {b}.")).unwrap_or_default()
            ),
            Err(e) => e,
        };
        self.status.set_message(&message);
        self.depth2stl.message = Some(message);
    }
}

/// The Depth to STL window's buttons.
#[derive(Default)]
struct Pressed {
    start: bool,
    get: bool,
    reset: bool,
}

/// The Depth to STL window's contents: how to get the add-on, or its settings for the
/// picture and Make Relief.
fn depth2stl_form(
    ui: &mut egui::Ui,
    w: &mut crate::depth2stl::Window,
    picture: Option<&cut_craft_core::bitmap::Bitmap>,
    units: cut_craft_core::document::Units,
    material: cut_craft_core::calc::Values,
) -> Pressed {
    let mut pressed = Pressed::default();
    if w.program.is_none() {
        ui.label(
            "Depth to STL turns a depth map into a detailed relief sized for your \
             carving bit. It is a free add-on, downloaded once.",
        );
        pressed.get = ui.button("Get Depth to STL…").clicked();
        return pressed;
    }
    let described = match &w.described {
        None => {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label("Starting Depth to STL…");
            });
            return pressed;
        }
        Some(Err(e)) => {
            ui.colored_label(ui.visuals().warn_fg_color, e);
            return pressed;
        }
        Some(Ok(d)) => d,
    };
    ui.weak(format!("Depth to STL {}", described.version));
    let Some(picture) = picture else {
        ui.label(
            "Import a depth map first (File ▸ Import), or a photo and use Estimate Depth \
             in Relief from Photo. White is high.",
        );
        return pressed;
    };
    let deep = picture.depth.len() == picture.grey.len();
    ui.label(format!(
        "Picture: {} ({} × {} pixels{})",
        picture.name,
        picture.columns,
        picture.rows,
        if deep { ", full precision" } else { "" }
    ));
    let fields = crate::form::fields(&described.settings);
    let none = |_: &str| None;
    let look = crate::form::Look {
        units,
        picture: &none,
        material,
        tool: None,
        group_end: None,
    };
    let running = w.running.is_some();
    ui.add_enabled_ui(!running, |ui| {
        crate::form::show(ui, &fields, &mut w.settings, &look);
    });
    ui.separator();
    if let Some(run) = &w.running {
        ui.horizontal(|ui| {
            ui.spinner();
            match &run.step {
                Some((stage, at, of)) => ui.label(format!("{stage} ({at} of {of})")),
                None => ui.label("Starting…"),
            };
        });
        if let Some((_, at, of)) = run.step {
            #[expect(clippy::cast_precision_loss, reason = "step counts")]
            ui.add(egui::ProgressBar::new(at as f32 / of.max(1) as f32));
        }
    } else if let Some(message) = &w.message {
        ui.label(message);
    }
    ui.horizontal(|ui| {
        pressed.start = ui
            .add_enabled(!running, egui::Button::new("Make Relief"))
            .on_hover_hint("Adds the relief as a new component over the picture")
            .clicked();
        pressed.reset = ui
            .add_enabled(!running, egui::Button::new("Reset Settings"))
            .clicked();
    });
    pressed
}
