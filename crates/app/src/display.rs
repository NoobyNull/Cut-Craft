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
            quality: 1,
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
            animate_preview: true,
            // the theme's warning colour, as the line previews are
            solid_colour: {
                let c = eframe::egui::Visuals::light().warn_fg_color;
                [c.r(), c.g(), c.b()]
            },
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
            "guides_visible" => &mut self.guides,
            "show_open_vector_ends" => &mut self.open_ends,
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

/// The preview quality menu's names, as Toolpaths ▸ Preview Simulation Quality lists them.
const QUALITY_NAMES: [(u32, &str); 5] = [
    (1, "Standard"),
    (2, "High (2x Slower)"),
    (4, "Very High (4x Slower)"),
    (8, "Extremely High (8x Slower)"),
    (25, "Maximum (25x Slower)"),
];

/// A word under the preview when a bit is narrower than two of its `cell`-wide squares
/// (made at `quality`): its thin cuts can show broken, dotted or not at all, though the
/// toolpath is right. It names the quality that draws them, or the finest there is. A tiny
/// pencil or detail bit on a big job is where this comes up.
pub fn fine_bit_note<'a>(
    tools: impl IntoIterator<Item = &'a cut_craft_core::toolpath::Tool>,
    cell: f64,
    quality: u32,
) -> Option<String> {
    let tool = tools
        .into_iter()
        .filter(|t| t.diameter > 0.0 && t.diameter < 2.0 * cell)
        .min_by(|a, b| a.diameter.total_cmp(&b.diameter))?;
    // a square's size at quality q: the cells across grow with the square root of it
    let at = |q: u32| cell * (f64::from(quality.max(1)) / f64::from(q)).sqrt();
    let fits = QUALITY_NAMES
        .iter()
        .find(|&&(q, _)| q > quality && 2.0 * at(q) <= tool.diameter);
    let fix = match fits {
        Some(&(_, name)) => format!(
            "To see them, pick Toolpaths ▸ Preview Simulation Quality ▸ {name} and preview again"
        ),
        None if quality < 25 => "Toolpaths ▸ Preview Simulation Quality ▸ Maximum (25x Slower) \
             shows them best; preview again"
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
        assert!(note.contains("Preview Simulation Quality"), "{note}");
        assert!(note.contains("toolpath and its G-code are right"), "{note}");
        assert_eq!(
            fine_bit_note([&fine], 0.5, 1),
            None,
            "finer squares draw it"
        );
    }

    #[test]
    fn a_fine_bit_note_names_the_quality_that_draws_the_bit() {
        use cut_craft_core::toolpath::Tool;
        // a 1/64 in pencil bit on a 600 mm job at Standard: 0.6 mm squares
        let pencil = Tool {
            name: "1/64 TBN".into(),
            diameter: 25.4 / 64.0,
            ..Tool::default()
        };
        let note = fine_bit_note([&pencil], 0.6, 1).unwrap();
        assert!(note.contains("1/64 TBN (0.40 mm)"), "{note}");
        assert!(note.contains("0.60 mm squares"), "{note}");
        // 0.6 / sqrt(q) at most 0.198 mm wants q >= 9.2: Maximum
        assert!(
            note.contains("▸ Maximum (25x Slower) and preview again"),
            "{note}"
        );
        // a 1.6 mm bit with 1 mm squares: High makes them 0.71 mm
        let sixteenth = Tool {
            name: "1/16 Ball".into(),
            diameter: 1.6,
            ..Tool::default()
        };
        let note = fine_bit_note([&sixteenth], 1.0, 1).unwrap();
        assert!(
            note.contains("▸ High (2x Slower) and preview again"),
            "{note}"
        );
        // already at Maximum and still too coarse
        let note = fine_bit_note([&pencil], 0.5, 25).unwrap();
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
    fn quality_is_one_of_the_levels_and_sets_the_cells() {
        let mut d = Display::default();
        assert_eq!(d.preview_cells(), 1000);
        assert!(d.toggle("preview_quality_4"));
        assert_eq!(d.checked("preview_quality_4"), Some(true));
        assert_eq!(d.checked("preview_quality_1"), Some(false));
        assert_eq!(d.preview_cells(), 2000, "4 times the work");
    }
}
