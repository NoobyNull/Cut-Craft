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
    /// The Preview tab's Performance ◀ ▶ Detail slider, percent: the preview's detail now.
    /// Starts at Options ▸ 3D View's default, and goes back to it on Reset Preview.
    pub detail: u8,
    pub origin: bool,
    pub modeling_plane: bool,
    pub material_block: bool,
    pub vectors_3d: bool,
    /// The keep-out zones outlined in red over the 3D view.
    pub keep_out_3d: bool,
    /// Each keep-out zone's name written on it ("Keep-out: Clamp 1"), in both views.
    pub keep_out_labels: bool,
    /// Parts of the surface the light cannot reach (behind a wall or a raised part) are
    /// drawn in shadow.
    pub shadows: bool,
    pub light_follows: bool,
    pub colour_shaded: bool,
    pub shaded_background: bool,
    /// Rotary jobs: the 3D view shows the model and preview wrapped round the axis.
    pub auto_wrapping: bool,
    /// Guide lines drawn in the 2D view.
    pub guides: bool,
    /// A circle on each end of every open vector in the 2D view, so gaps that stop a shape
    /// being pocketed or carved can be seen.
    pub open_ends: bool,
    /// Construction lines drawn (dashed) in the 2D view; hidden, they cannot be picked
    /// either.
    pub construction: bool,
    /// Previews play the tool cutting (Animate preview) rather than appearing cut.
    pub animate_preview: bool,
    /// What solid 2D previews are drawn in ([r, g, b]).
    pub solid_colour: [u8; 3],
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
            detail: crate::preview_detail::DEFAULT_PERCENT,
            origin: true,
            modeling_plane: false,
            material_block: true,
            vectors_3d: true,
            keep_out_3d: true,
            keep_out_labels: true,
            shadows: false,
            light_follows: false,
            colour_shaded: true,
            shaded_background: true,
            auto_wrapping: true,
            guides: true,
            open_ends: false,
            construction: true,
            animate_preview: true,
            // the theme's warning colour, as the line previews are
            solid_colour: {
                let c = eframe::egui::Visuals::light().warn_fg_color;
                [c.r(), c.g(), c.b()]
            },
        }
    }
}

use crate::preview_detail::QUICK_PICKS;

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
            "guides_visible" => &mut self.guides,
            "show_open_vector_ends" => &mut self.open_ends,
            "show_construction_lines" => &mut self.construction,
            "animate_preview" => &mut self.animate_preview,
            "draw_modeling_plane" => &mut self.modeling_plane,
            "draw_material_block" => &mut self.material_block,
            "draw_vectors_in_3d_view" => &mut self.vectors_3d,
            "draw_keep_out_zones_in_3d_view" => &mut self.keep_out_3d,
            "show_keep_out_zone_labels" => &mut self.keep_out_labels,
            "use_shadow_shading" => &mut self.shadows,
            "light_follows_viewer" => &mut self.light_follows,
            "color_shaded_view" => &mut self.colour_shaded,
            "use_shaded_background" => &mut self.shaded_background,
            "auto_wrapping_view" => &mut self.auto_wrapping,
            _ => return None,
        })
    }

    /// Whether `id` is one of these settings shown ticked; `None` when it is not one.
    pub fn checked(&self, id: &str) -> Option<bool> {
        // the menu's tick follows the slider: none when it is between them
        if let Some(&(_, q)) = QUICK_PICKS.iter().find(|(a, _)| *a == id) {
            return Some(self.detail == q);
        }
        let mut copy = *self;
        copy.flag(id).map(|on| *on)
    }

    /// Turns setting `id` over (or moves the detail slider to that quick pick); whether it
    /// was one.
    pub fn toggle(&mut self, id: &str) -> bool {
        if let Some(&(_, q)) = QUICK_PICKS.iter().find(|(a, _)| *a == id) {
            self.detail = q;
            return true;
        }
        self.flag(id).map(|on| *on = !*on).is_some()
    }

    /// Whether `id` is a quick pick on the preview detail slider (the preview is worked
    /// out again at it).
    pub fn is_quick_pick(id: &str) -> bool {
        QUICK_PICKS.iter().any(|(a, _)| *a == id)
    }
}

/// A word under the preview when a bit is narrower than two of its `cell`-wide squares
/// (made with the detail slider at `percent`): its thin cuts can show broken, dotted or not
/// at all, though the toolpath is right. It names the slider setting that draws them, or
/// says this is the finest there is. A tiny pencil or detail bit is where this comes up.
pub fn fine_bit_note<'a>(
    tools: impl IntoIterator<Item = &'a cut_craft_core::toolpath::Tool>,
    cell: f64,
    percent: u8,
) -> Option<String> {
    let tool = tools
        .into_iter()
        .filter(|t| t.diameter > 0.0 && t.diameter < 2.0 * cell)
        .min_by(|a, b| a.diameter.total_cmp(&b.diameter))?;
    let fits = crate::preview_detail::percent_for(tool.diameter / 2.0).filter(|&p| p > percent);
    let fix = match fits {
        Some(p) => format!(
            "To see them, slide Preview detail on the Preview tab to {p}% or more (toward \
             More detail)"
        ),
        None if percent < 100 => "To see them, slide Preview detail on the Preview tab \
             further toward More detail"
            .to_owned(),
        None => "This is the finest preview there is".to_owned(),
    };
    Some(format!(
        "⚠ {} ({:.2} mm) is finer than the preview's {:.2} mm squares, so its cuts may look \
         broken or be missing here. The toolpath and its G-code are right. {fix}.",
        tool.name, tool.diameter, cell
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_bit_under_two_cells_wide_gets_a_note_and_a_wider_one_none() {
        use cut_craft_core::toolpath::Tool;
        let fine = Tool {
            name: "1/16 End Mill".into(),
            diameter: 1.6,
            ..Tool::default()
        };
        let wide = Tool {
            diameter: 6.0,
            ..Tool::default()
        };
        assert_eq!(fine_bit_note([&wide], 2.44, 1), None);
        let note = fine_bit_note([&wide, &fine], 2.44, 1).expect("1.6 mm under 4.88 mm");
        assert!(note.contains("1/16 End Mill"), "{note}");
        assert!(note.contains("Preview detail"), "{note}");
        assert!(note.contains("toolpath and its G-code are right"), "{note}");
        assert_eq!(
            fine_bit_note([&fine], 0.5, 1),
            None,
            "finer squares draw it"
        );
    }

    #[test]
    fn a_fine_bit_note_names_the_slider_setting_that_draws_the_bit() {
        use cut_craft_core::toolpath::Tool;
        // a 1/64 in pencil bit with 0.6 mm squares at 20%
        let pencil = Tool {
            name: "1/64 TBN".into(),
            diameter: 25.4 / 64.0,
            ..Tool::default()
        };
        let note = fine_bit_note([&pencil], 0.6, 20).unwrap();
        assert!(note.contains("1/64 TBN (0.40 mm)"), "{note}");
        assert!(note.contains("0.60 mm squares"), "{note}");
        // squares of 0.198 mm at most: the slider at 29% or more
        assert!(note.contains("to 29% or more"), "{note}");
        // a 1.6 mm bit with 1 mm squares, the slider past where its spacing alone would
        // do: the area kept it coarse, so further right
        let sixteenth = Tool {
            name: "1/16 Ball".into(),
            diameter: 1.6,
            ..Tool::default()
        };
        let note = fine_bit_note([&sixteenth], 1.0, 20).unwrap();
        assert!(note.contains("further toward More detail"), "{note}");
        // already at 100% and still too coarse
        let note = fine_bit_note([&pencil], 0.5, 100).unwrap();
        assert!(note.contains("finest preview there is"), "{note}");
    }

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
    fn the_quality_menu_moves_the_slider_and_its_tick_follows_it() {
        let mut d = Display::default();
        assert_eq!(d.detail, 50);
        assert_eq!(
            d.checked("preview_quality_4"),
            Some(false),
            "50% is none of them"
        );
        assert!(d.toggle("preview_quality_4"));
        assert_eq!(d.detail, 60);
        assert_eq!(d.checked("preview_quality_4"), Some(true));
        assert_eq!(d.checked("preview_quality_1"), Some(false));
        d.detail = 20;
        assert_eq!(d.checked("preview_quality_1"), Some(true));
        assert!(Display::is_quick_pick("preview_quality_25"));
        assert!(!Display::is_quick_pick("draw_rapid_moves"));
    }
}
