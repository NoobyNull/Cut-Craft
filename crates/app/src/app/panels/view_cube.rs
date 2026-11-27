//! The view cube in the 3D view's corner and the buttons round it: a click on a face, edge or
//! corner looks from there, Home goes to the Home view, the two turns turn the view a quarter
//! turn about the line of sight and the nudges bring the next face round, each turning the
//! view smoothly (Options ▸ 3D View Settings ▸ Turn smoothly to new views).

use super::{CutCraftApp, egui, tr, view3d};
use view3d::{CubeButton, Orbit, ViewCube};

impl CutCraftApp {
    /// Where the view is going: the end of a smooth turn under way, else where it is.
    pub(in super::super) fn orbit_goal(&mut self) -> Orbit {
        self.motion_3d.turning().map_or(self.orbit, |t| t.to)
    }

    /// Turns the view to `to`, smoothly unless Options says to jump, stopping Auto-rotate.
    pub(in super::super) fn look_3d(&mut self, to: Orbit, now: f64) {
        self.motion_3d.stop_spin();
        if self.options.view_3d.smooth_turns {
            self.motion_3d.turn(self.orbit, to, now);
        } else {
            self.motion_3d.stop_turning();
            self.orbit = to;
        }
    }

    /// A turning drag of `turn` across a view `size`, as `rotation` takes it. A turntable keeps
    /// the job's up up, so a view the cube turned (or is turning) on its side turns back
    /// upright first, smoothly from where it is on screen, the drag carrying on from there; a
    /// free drag stops a smooth turn where it
    /// is and turns on from there.
    pub(in super::super) fn drag_view_3d(
        &mut self,
        turn: egui::Vec2,
        size: egui::Vec2,
        rotation: view3d::Rotation,
        now: f64,
    ) {
        let turntable = rotation == view3d::Rotation::Turntable;
        // a turn under way to a view on its side starts again from what is on screen now,
        // heading upright, so the picture doesn't jump; once upright it isn't restarted
        let goal = self.orbit_goal();
        if turntable && goal.roll.abs() > 1e-9 {
            self.look_3d(goal.upright(rotation), now);
        }
        match self.motion_3d.turning() {
            Some(turning) if turntable => turning.to = turning.to.dragged_by(turn, size, rotation),
            _ => {
                self.motion_3d.stop_turning();
                self.orbit = self.orbit.dragged_by(turn, size, rotation);
            }
        }
    }

    /// A drag or click on the cube itself: a drag turns the view, a click looks from the
    /// face, edge or corner clicked and a double-click from the other side. `size` is the
    /// view's, for how far a drag turns.
    pub(in super::super) fn view_cube_input(
        &mut self,
        on_cube: &egui::Response,
        cube: ViewCube,
        (size, rotation, now): (egui::Vec2, view3d::Rotation, f64),
    ) {
        if on_cube.dragged() {
            let turn = self.options.controls.turn(on_cube.drag_delta());
            self.drag_view_3d(turn, size, rotation, now);
        } else if on_cube.double_clicked() {
            let back = Orbit::facing(-self.orbit_goal().towards_viewer());
            self.look_3d(back, now);
        } else if let Some(from) = on_cube
            .interact_pointer_pos()
            .filter(|_| on_cube.clicked())
            .and_then(|at| cube.direction_at(self.orbit, at))
        {
            self.look_3d(Orbit::facing(from), now);
        }
    }

    /// Draws the cube, the part under the pointer (`hovered`) lit, its names in `font`, and
    /// its buttons as [`CutCraftApp::view_cube_buttons`] found them.
    pub(in super::super) fn paint_view_cube(
        &self,
        ui: &egui::Ui,
        painter: &egui::Painter,
        (cube, hovered, font): (ViewCube, Option<cut_craft_core::glam::DVec3>, &egui::FontId),
        buttons: &[(CubeButton, bool, bool)],
    ) {
        let look = self.options.view_3d.cube_style.look();
        let names = view3d::FACES.map(|f| tr!(f.3));
        cube.paint(
            painter,
            self.orbit,
            hovered,
            ui.visuals(),
            look,
            &names,
            font,
        );
        self.paint_cube_buttons(ui, cube, buttons);
    }

    /// Moves a smooth turn under way on to this frame's view, asking for the next frame.
    pub(in super::super) fn turn_3d(&mut self, ctx: &egui::Context) {
        if let Some(orbit) = self.motion_3d.turned(ctx.input(|i| i.time)) {
            self.orbit = orbit;
            ctx.request_repaint();
        }
    }

    /// The view cube's buttons from `cube`: each pressed one turns the view, Home's right-click
    /// menu sets or resets the Home view. Each button shown, whether it can be pressed and
    /// whether the pointer is over it, for [`CutCraftApp::paint_cube_buttons`].
    pub(in super::super) fn view_cube_buttons(
        &mut self,
        ui: &egui::Ui,
        cube: ViewCube,
    ) -> Vec<(CubeButton, bool, bool)> {
        let now = ui.input(|i| i.time);
        let goal = self.orbit_goal();
        let mut shown = vec![];
        for (button, on) in ViewCube::buttons(goal) {
            let id = ui.scope_id().with(("view_cube_button", icon_name(button)));
            let sense = if on {
                egui::Sense::click()
            } else {
                egui::Sense::hover()
            };
            let response = ui.interact(cube.button_rect(button), id, sense);
            shown.push((button, on, response.hovered()));
            let response = response.on_hover_text(tip(goal, button, on));
            if button == CubeButton::Home {
                self.home_menu(&response);
            }
            if response.clicked() {
                self.press_cube_button(button, goal, now);
            }
        }
        shown
    }

    /// Draws the view cube's buttons as [`CutCraftApp::view_cube_buttons`] found them, each
    /// with its picture from `assets/icons/view` or its own drawing.
    fn paint_cube_buttons(
        &self,
        ui: &egui::Ui,
        cube: ViewCube,
        shown: &[(CubeButton, bool, bool)],
    ) {
        for &(button, on, hovered) in shown {
            let icon = self
                .icons
                .find(&format!("view/{}.png", icon_name(button)), "")
                .map(|key| self.icons.image(key));
            cube.paint_button(ui, button, (on, hovered), ui.visuals(), icon.as_ref());
        }
    }

    /// A view cube button pressed, the view on its way to `goal`.
    pub(in super::super) fn press_cube_button(
        &mut self,
        button: CubeButton,
        goal: Orbit,
        now: f64,
    ) {
        let to = match button {
            CubeButton::Home => {
                self.framing = self.framing.fitted();
                self.options.view_3d.home()
            }
            CubeButton::TurnLeft => goal.quarter_turned(false),
            CubeButton::TurnRight => goal.quarter_turned(true),
            CubeButton::Nudge(side) => goal.nudged(side),
        };
        self.look_3d(to, now);
    }

    /// Home's right-click menu: this view becomes Home, or Home goes back to the starting view.
    fn home_menu(&mut self, response: &egui::Response) {
        cut_craft_bug_report::context_menu(response, |ui| {
            if ui.button(tr!("view-cube-make-home")).clicked() {
                let Orbit { yaw, pitch, roll } = self.orbit_goal();
                self.options.view_3d.home_view = Some([yaw, pitch, roll]);
                self.save_options();
                ui.close();
            }
            let set = self.options.view_3d.home_view.is_some();
            if ui
                .add_enabled(set, egui::Button::new(tr!("view-cube-reset-home")))
                .clicked()
            {
                self.options.view_3d.home_view = None;
                self.save_options();
                ui.close();
            }
        });
    }
}

/// The file a button's picture is read from (`assets/icons/view/`), when there is one; the
/// button draws its own otherwise.
fn icon_name(button: CubeButton) -> &'static str {
    match button {
        CubeButton::Home => "view_home",
        CubeButton::TurnLeft => "view_turn_left",
        CubeButton::TurnRight => "view_turn_right",
        CubeButton::Nudge(view3d::Nudge::Up) => "view_nudge_up",
        CubeButton::Nudge(view3d::Nudge::Down) => "view_nudge_down",
        CubeButton::Nudge(view3d::Nudge::Left) => "view_nudge_left",
        CubeButton::Nudge(view3d::Nudge::Right) => "view_nudge_right",
    }
}

/// A button's tooltip: what it does, a nudge naming the face it brings round, a turn that
/// can't be pressed how to make it work.
fn tip(goal: Orbit, button: CubeButton, on: bool) -> String {
    match button {
        CubeButton::Home => tr!("view-cube-home-tip"),
        CubeButton::TurnLeft | CubeButton::TurnRight if !on => tr!("view-cube-turn-off-tip"),
        CubeButton::TurnLeft => tr!("view-cube-turn-left-tip"),
        CubeButton::TurnRight => tr!("view-cube-turn-right-tip"),
        CubeButton::Nudge(side) => {
            tr!("view-cube-nudge-tip", face = tr!(goal.face_beyond(side)))
        }
    }
}
