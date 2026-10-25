//! How things are drawn: the Toolpaths ▸ Toolpath Drawing and Preview Simulation Quality
//! menus and the View menu's toggles. View state, not part of the job or its undo history.

/// The display settings, each a menu item that turns it on or off.
// ponytail: kept for the session; saved settings belong with Edit ▸ Options
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[expect(clippy::struct_excessive_bools, reason = "independent menu toggles")]
pub struct Display {
    pub previews_2d: bool,
    pub solid_previews: bool,
    pub auto_open_3d: bool,
    pub rapids: bool,
    pub plunges: bool,
    pub retracts: bool,
    /// Feed moves above the material that link one cut to the next.
    pub joins: bool,
    /// How many times slower than Standard the preview simulation may be: 1, 2, 4, 8 or 25.
    pub quality: u32,
    pub origin: bool,
    pub modeling_plane: bool,
    pub material_block: bool,
    pub vectors_3d: bool,
    pub light_follows: bool,
    pub colour_shaded: bool,
    pub shaded_background: bool,
}

impl Default for Display {
    fn default() -> Self {
        Self {
            previews_2d: true,
            solid_previews: false,
            auto_open_3d: true,
            rapids: false,
            plunges: false,
            retracts: false,
            joins: false,
            quality: 1,
            origin: true,
            modeling_plane: false,
            material_block: true,
            vectors_3d: true,
            light_follows: false,
            colour_shaded: true,
            shaded_background: true,
        }
    }
}

/// The preview quality menu's levels: (action, times slower).
pub const QUALITIES: [(&str, u32); 5] = [
    ("preview_quality_1", 1),
    ("preview_quality_2", 2),
    ("preview_quality_4", 4),
    ("preview_quality_8", 8),
    ("preview_quality_25", 25),
];

/// Simulation cells along the job's longer side at Standard quality.
const STANDARD_CELLS: f64 = 1000.0;

impl Display {
    fn flag(&mut self, id: &str) -> Option<&mut bool> {
        Some(match id {
            "draw_2d_previews" => &mut self.previews_2d,
            "make_2d_previews_solid" => &mut self.solid_previews,
            "auto_open_3d_view" => &mut self.auto_open_3d,
            "draw_rapid_moves" => &mut self.rapids,
            "draw_plunge_moves" => &mut self.plunges,
            "draw_retract_moves" => &mut self.retracts,
            "draw_join_moves" => &mut self.joins,
            "draw_origin" => &mut self.origin,
            "draw_modeling_plane" => &mut self.modeling_plane,
            "draw_material_block" => &mut self.material_block,
            "draw_vectors_in_3d_view" => &mut self.vectors_3d,
            "light_follows_viewer" => &mut self.light_follows,
            "color_shaded_view" => &mut self.colour_shaded,
            "use_shaded_background" => &mut self.shaded_background,
            _ => return None,
        })
    }

    /// Whether `id` is one of these settings shown ticked; `None` when it is not one.
    pub fn checked(&self, id: &str) -> Option<bool> {
        if let Some(&(_, q)) = QUALITIES.iter().find(|(a, _)| *a == id) {
            return Some(self.quality == q);
        }
        let mut copy = *self;
        copy.flag(id).map(|on| *on)
    }

    /// Turns setting `id` over (or picks that quality); whether it was one.
    pub fn toggle(&mut self, id: &str) -> bool {
        if let Some(&(_, q)) = QUALITIES.iter().find(|(a, _)| *a == id) {
            self.quality = q;
            return true;
        }
        self.flag(id).map(|on| *on = !*on).is_some()
    }

    /// Simulation cells along the job's longer side: the work grows with their square, so
    /// N times slower is the square root of N times as many.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "a cell count"
    )]
    pub fn preview_cells(&self) -> usize {
        (STANDARD_CELLS * f64::from(self.quality).sqrt()).round() as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toggles_turn_over_and_report_their_state() {
        let mut d = Display::default();
        assert_eq!(d.checked("draw_rapid_moves"), Some(false));
        assert!(d.toggle("draw_rapid_moves"));
        assert_eq!(d.checked("draw_rapid_moves"), Some(true));
        assert!(!d.toggle("not_a_setting"));
        assert_eq!(d.checked("not_a_setting"), None);
    }

    #[test]
    fn quality_is_one_of_the_levels_and_sets_the_cells() {
        let mut d = Display::default();
        assert_eq!(d.preview_cells(), 1000);
        assert!(d.toggle("preview_quality_4"));
        assert_eq!(d.checked("preview_quality_4"), Some(true));
        assert_eq!(d.checked("preview_quality_1"), Some(false));
        assert_eq!(d.preview_cells(), 2000, "4 times the work");
    }
}
