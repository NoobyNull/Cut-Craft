//! Previewing toolpaths: playing them on the material, and the material as it is cut.

use super::super::*;

impl Harness {
    /// Options ▸ Animate toolpath preview, on or off.
    pub(in super::super) fn given_animated_preview(&mut self, on: bool) {
        self.app.display.animate_preview = on;
    }

    /// The Preview Control's speed.
    pub(in super::super) fn set_playback_speed(&mut self, speed: f64) {
        self.app.player.speed = speed;
    }

    /// The preview playing, if one is.
    pub(in super::super) fn playback(&self) -> Option<&cut_craft_core::playback::Playback> {
        self.app.player.playback.as_ref()
    }

    /// A rotary job's file played turning the material, if one is shown.
    pub(in super::super) fn turning(&self) -> Option<&cut_craft_rotary::Replay> {
        self.app.turning.as_ref().map(|t| &t.replay)
    }

    /// Runs the playing preview to its end at once.
    pub(in super::super) fn finish_playback(&mut self) {
        self.app.finish_playback();
    }

    /// Preview on toolpath `id` alone.
    pub(in super::super) fn preview_toolpath(&mut self, id: EntityId) {
        self.app.preview_toolpath(id);
    }

    /// Preview on the toolpaths `ids`, as a group's Preview does.
    pub(in super::super) fn preview_toolpaths(&mut self, ids: &[EntityId]) {
        self.app.preview_ids(ids);
    }

    /// The previewed material's height at its first cell.
    pub(in super::super) fn previewed_height(&self) -> f64 {
        self.app
            .preview
            .material
            .as_ref()
            .expect("a preview was played")
            .heights[0]
    }

    /// An earlier preview's cut left in the material: its first cell at `z`.
    pub(in super::super) fn given_previewed_height(&mut self, z: f64) {
        self.app
            .preview
            .material
            .as_mut()
            .expect("a preview was played")
            .heights[0] = z;
    }

    // ---- the material in the 3D view ----

    /// The material's surface the 3D view shows: uncut, the model on it, or cut by a preview.
    pub(in super::super) fn surface_3d(&mut self) -> Option<Material> {
        self.app.surface_3d().map(std::borrow::Cow::into_owned)
    }

    /// Whether the 3D view shows the uncut block (no model yet).
    pub(in super::super) fn showing_uncut_material(&self) -> bool {
        self.app.showing_uncut_material()
    }

    /// Whether the model is shown seated in the material block.
    pub(in super::super) fn model_on_material(&self) -> bool {
        self.app.model_on_material()
    }

    /// Whether the model being imported is shown whole (under Options ▸ Model preview faces).
    pub(in super::super) fn importing_whole(&self) -> bool {
        self.app.importing_whole()
    }

    /// The height the 3D view's origin arrows are drawn at.
    pub(in super::super) fn origin_height_3d(&self) -> f64 {
        self.app.origin_height_3d()
    }

    /// The part of the model being imported that is past the material's edge, if measured.
    pub(in super::super) fn importing_beyond(&self) -> Option<&[cmd_orientate_3d_model::Seen]> {
        self.app
            .form
            .as_ref()
            .and_then(|f| f.beyond.as_deref())
            .map(Vec::as_slice)
    }

    /// Whether the open import form draws its model whole, not as its component.
    pub(in super::super) fn import_drawn_whole(&self) -> bool {
        self.app.form.as_ref().expect("a form is open").model_whole
    }

    /// Material Setup's measure of the model's height, mm.
    pub(in super::super) fn model_height(&self) -> f64 {
        self.app.placement_room().tall
    }

    /// Asks for Material Setup on the first click on the Toolpaths page, as a new job does.
    pub(in super::super) fn given_material_not_asked(&mut self) {
        self.app.ask_material = true;
    }

    /// Preview All on every toolpath (or only the shown ones).
    pub(in super::super) fn preview_all(&mut self, visible_only: bool) {
        self.app.preview_all(visible_only);
    }

    /// The Preview Control's Pause.
    pub(in super::super) fn pause_playback(&mut self) {
        self.app.player.playing = false;
    }

    /// Drags the Preview Control's Progress to `share` of the way through.
    pub(in super::super) fn seek_playback(&mut self, share: f64) {
        let playback = self.app.player.playback.as_mut().expect("a preview plays");
        let at = playback.length() * share;
        playback.seek(at);
    }

    /// Whether a finished preview's cut material is shown.
    pub(in super::super) fn has_previewed_material(&self) -> bool {
        self.app.preview.material.is_some()
    }

    // ---- the preview and its control ----

    /// The material as the last preview cut it, if there is one.
    pub(in super::super) fn previewed(&self) -> Option<&Material> {
        self.app.preview.material.as_ref()
    }

    /// The preview shows `material`, as one cut it.
    pub(in super::super) fn given_previewed(&mut self, material: Option<Material>) {
        self.app.preview.material = material;
    }

    /// Whether Undo Last Preview has an earlier preview to bring back.
    pub(in super::super) fn preview_before_kept(&self) -> bool {
        self.app.preview.before.is_some()
    }

    /// Whether the preview is playing (not paused or stopped).
    pub(in super::super) fn playing(&self) -> bool {
        self.app.player.playing
    }

    /// Whether a stopped preview is kept so its controls stay to scrub it.
    pub(in super::super) fn playback_kept(&self) -> bool {
        self.app.player.kept.is_some()
    }

    /// Where the tool is being moved to a little each frame, if it is.
    pub(in super::super) fn playback_goal(&self) -> Option<f64> {
        self.app.player.goal
    }

    /// How many times the playing preview's view has been made.
    pub(in super::super) fn playback_builds(&self) -> usize {
        self.app.player.builds
    }

    /// The playing preview's view as drawn: its playback revision and detail, if drawn.
    pub(in super::super) fn playback_drawn(&self) -> Option<(u64, usize)> {
        self.app
            .player
            .view
            .as_ref()
            .map(|((revision, cells, _), _)| (*revision, *cells))
    }

    /// The playing preview is redrawn no more often than `every`.
    pub(in super::super) fn given_playback_redraw(&mut self, every: std::time::Duration) {
        self.app.player.redraw = every;
    }

    /// A frame may spend `budget` cutting the playing preview.
    pub(in super::super) fn given_playback_budget(&mut self, budget: std::time::Duration) {
        self.app.player.budget = budget;
    }

    /// A double-click on the 3D view drawn in `fit`, at `at`: takes away the piece cut free
    /// there, whether there was one.
    pub(in super::super) fn remove_preview_piece(&mut self, fit: Rect, at: Pos2) -> bool {
        self.app.remove_preview_piece(fit, at)
    }

    /// Create Component from Toolpath Preview.
    pub(in super::super) fn component_from_preview(&mut self) {
        self.app.component_from_preview();
    }

    /// An uncut block for the job as it is now.
    pub(in super::super) fn fresh_material(&self) -> Material {
        self.app.fresh_material()
    }

    /// The finely previewed material as the 3D view shows it, the whole sheet round it.
    pub(in super::super) fn fine_surface(&mut self) -> Material {
        self.app.fine_surface()
    }

    /// Starts a new job, as File ▸ New does once the old one is dealt with.
    pub(in super::super) fn reset_job(&mut self) {
        self.app.reset_job();
    }

    // ---- Trace Bitmap's window ----

    /// Whether Trace Bitmap's window is open.
    pub(in super::super) fn trace_window_open(&self) -> bool {
        self.app.trace_view.is_some()
    }

    /// How many colours the window's preview traced, once it has.
    pub(in super::super) fn traced_colours(&self) -> Option<usize> {
        self.app
            .trace_view
            .as_ref()
            .expect("the trace window is open")
            .traced_colours()
    }

    /// Whether the window's preview shows the form's settings as they are now.
    pub(in super::super) fn trace_up_to_date(&self) -> bool {
        let view = self
            .app
            .trace_view
            .as_ref()
            .expect("the trace window is open");
        view.shown_params() == self.app.form.as_ref().map(|f| &f.params)
    }

    /// Unticks the window's Live, so a change waits for Update preview.
    pub(in super::super) fn untick_live_trace(&mut self) {
        self.app
            .trace_view
            .as_mut()
            .expect("the trace window is open")
            .live = false;
    }
}
