//! Advanced settings: how hard PowerWhittle works, sealed so a hand edit is noticed.

use super::{Deserialize, JsonSchema, Options, Serialize};

/// Settings that change how hard PowerWhittle works the computer, or how exactly it cuts: listed
/// only once Show advanced settings is ticked, and put back to their defaults when the file
/// was changed outside PowerWhittle.
// ponytail: wayland_native moved here from General; one saved there before starts off again
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Advanced {
    /// On a Wayland desktop, open the window there directly rather than through `XWayland`;
    /// files cannot be dragged into it then (from the next start). Listed only on Wayland
    #[serde(default)]
    #[schemars(title = "Use Wayland directly (no file drag and drop)")]
    pub wayland_native: bool,
    /// Replaced by Options ▸ Performance's calculation limit, which carries it over once
    /// (0 was every core). Not shown; still saved, so options sealed before stay sealed
    #[serde(default)]
    #[schemars(skip)]
    pub cores: u32,
    /// How many points each stretch of a curved DXF spline becomes when it is imported. Fewer
    /// gives flat-sided curves; more gives smoother curves but bigger, slower jobs
    // left out of the file (and so the check) at its default, so options sealed before it
    // existed still pass; every setting added here later wants the same
    #[serde(
        default = "dxf_spline_detail",
        skip_serializing_if = "is_dxf_spline_detail"
    )]
    #[schemars(
        title = "DXF curve detail (points per span)",
        range(min = 4, max = 256)
    )]
    pub dxf_spline_detail: u32,
    /// How often the Library reads its folders again to find files added, moved or deleted.
    /// Sooner shows changes quicker, but keeps a network drive or a slow disk busy
    #[serde(
        default = "library_rescan_seconds",
        skip_serializing_if = "is_library_rescan_seconds"
    )]
    #[schemars(
        title = "Library: read the folders again every",
        extend("x-unit" = "s"),
        range(min = 1, max = 3600)
    )]
    pub library_rescan_seconds: u32,
    /// The Library stops listing after this many files, so adding a whole drive can't stall
    /// it. More lists bigger collections, but uses more memory and time on every read
    #[serde(
        default = "library_most_files",
        skip_serializing_if = "is_library_most_files"
    )]
    #[schemars(
        title = "Library: most files listed",
        range(min = 1000, max = 10_000_000)
    )]
    pub library_most_files: u32,
    /// Saved Library previews not used for this long are deleted when PowerWhittle starts. Longer
    /// keeps more of them on disk, so folders open faster (from the next start)
    #[serde(
        default = "preview_cache_days",
        skip_serializing_if = "is_preview_cache_days"
    )]
    #[schemars(
        title = "Library: keep unused previews for",
        extend("x-unit" = "days"),
        range(min = 1, max = 3650)
    )]
    pub preview_cache_days: u32,
    /// How many Library previews are kept on the graphics card at once. More scrolls back
    /// faster, but uses more video memory, which can run out on built-in graphics (from the
    /// next start)
    #[serde(
        default = "previews_on_card",
        skip_serializing_if = "is_previews_on_card"
    )]
    #[schemars(
        title = "Library: previews kept on the graphics card",
        range(min = 50, max = 5000)
    )]
    pub previews_on_card: u32,
    /// How closely curves are followed by the straight pieces offsets, pockets and profiles
    /// are cut from, and by the arcs written to G-code. Coarser shows as flats on curved cuts;
    /// finer makes toolpaths slower to work out and G-code files bigger
    #[serde(default = "curve_accuracy", skip_serializing_if = "is_curve_accuracy")]
    #[schemars(
        title = "Curve accuracy for toolpaths",
        extend("x-unit" = "µm"),
        range(min = 1, max = 100)
    )]
    pub curve_accuracy: u32,
    /// How far a 3D roughing move may dip below the machining allowance before it is split.
    /// Coarser eats into the allowance left for finishing; finer makes bigger files
    #[serde(
        default = "roughing_3d_accuracy",
        skip_serializing_if = "is_roughing_3d_accuracy"
    )]
    #[schemars(
        title = "3D roughing accuracy",
        extend("x-unit" = "µm"),
        range(min = 1, max = 100)
    )]
    pub roughing_3d_accuracy: u32,
    /// ⚠ How far a 3D finishing move may dip below the model before it is split. Raising this
    /// lets the tool cut into the finished part (gouging); lowering it makes much bigger files
    #[serde(
        default = "finishing_3d_accuracy",
        skip_serializing_if = "is_finishing_3d_accuracy"
    )]
    #[schemars(
        title = "3D finishing accuracy",
        extend("x-unit" = "µm"),
        range(min = 1, max = 20)
    )]
    pub finishing_3d_accuracy: u32,
    /// How much material must be left above the model for rest machining to go back for it.
    /// Lower finds smaller leftovers but may chase marks the last tool left; higher skips them
    #[serde(default = "rest_threshold", skip_serializing_if = "is_rest_threshold")]
    #[schemars(
        title = "Rest machining threshold",
        extend("x-unit" = "µm"),
        range(min = 5, max = 1000)
    )]
    pub rest_threshold: u32,
    /// The most sheets Nest Parts' Add sheets as needed makes in one go. More nests bigger
    /// batches, but a part that can never fit makes it run much longer before it gives up
    #[serde(
        default = "nest_most_sheets",
        skip_serializing_if = "is_nest_most_sheets"
    )]
    #[schemars(title = "Nesting: most sheets added", range(min = 1, max = 1000))]
    pub nest_most_sheets: u32,
    /// The most filler parts one nesting places. More fills big sheets with small parts, but
    /// slows nesting down
    #[serde(
        default = "nest_most_fillers",
        skip_serializing_if = "is_nest_most_fillers"
    )]
    #[schemars(title = "Nesting: most filler parts", range(min = 1, max = 100_000))]
    pub nest_most_fillers: u32,
    /// How much work a gadget may do, in millions of Lua steps, before it is stopped as
    /// stuck. Higher lets big gadgets finish; a gadget caught in a loop then freezes PowerWhittle
    /// for longer before it is stopped
    #[serde(
        default = "gadget_million_instructions",
        skip_serializing_if = "is_gadget_million_instructions"
    )]
    #[schemars(
        title = "Gadgets: most work (million steps)",
        range(min = 10, max = 5000)
    )]
    pub gadget_million_instructions: u32,
    /// How much memory a gadget may use before it is stopped. Higher lets big gadgets finish,
    /// but a runaway one can take that much from the computer
    #[serde(
        default = "gadget_memory_mb",
        skip_serializing_if = "is_gadget_memory_mb"
    )]
    #[schemars(
        title = "Gadgets: most memory",
        extend("x-unit" = "MB"),
        range(min = 8, max = 1024)
    )]
    pub gadget_memory_mb: u32,
    /// How many commands one gadget run may do. Higher lets a gadget make more shapes in one
    /// go, but each one adds to the job's memory and to the time Undo takes
    #[serde(
        default = "gadget_most_commands",
        skip_serializing_if = "is_gadget_most_commands"
    )]
    #[schemars(
        title = "Gadgets: most commands in one run",
        range(min = 100, max = 1_000_000)
    )]
    pub gadget_most_commands: u32,
}

/// The default of an Advanced setting, and whether a value is it: settings at their default
/// are left out of the file, so the check written before they existed still matches.
macro_rules! advanced_default {
    ($default:ident, $is:ident, $value:expr) => {
        fn $default() -> u32 {
            $value
        }

        fn $is(value: &u32) -> bool {
            *value == $default()
        }
    };
}

advanced_default!(
    dxf_spline_detail,
    is_dxf_spline_detail,
    cut_craft_core::import_settings::SPLINE_SAMPLES
);
advanced_default!(
    library_rescan_seconds,
    is_library_rescan_seconds,
    u32::try_from(crate::clipart::RELIST.as_secs()).expect("a few seconds")
);
advanced_default!(
    library_most_files,
    is_library_most_files,
    u32::try_from(crate::clipart::MOST).expect("a count that fits")
);
advanced_default!(
    preview_cache_days,
    is_preview_cache_days,
    u32::try_from(crate::clipart::preview::UNUSED_DAYS).expect("a count of days")
);
advanced_default!(
    previews_on_card,
    is_previews_on_card,
    u32::try_from(crate::clipart::preview::KEEP).expect("a count that fits")
);
advanced_default!(
    curve_accuracy,
    is_curve_accuracy,
    micrometres(cut_craft_core::accuracy::CURVE)
);
advanced_default!(
    roughing_3d_accuracy,
    is_roughing_3d_accuracy,
    micrometres(cut_craft_core::accuracy::ROUGHING_3D)
);
advanced_default!(
    finishing_3d_accuracy,
    is_finishing_3d_accuracy,
    micrometres(cut_craft_core::accuracy::FINISHING_3D)
);
advanced_default!(
    rest_threshold,
    is_rest_threshold,
    micrometres(cut_craft_core::accuracy::REST)
);
advanced_default!(
    nest_most_sheets,
    is_nest_most_sheets,
    u32::try_from(cut_craft_core::accuracy::NEST_SHEETS).expect("a small count")
);
advanced_default!(
    nest_most_fillers,
    is_nest_most_fillers,
    u32::try_from(cut_craft_core::accuracy::NEST_FILLERS).expect("a small count")
);

advanced_default!(
    gadget_million_instructions,
    is_gadget_million_instructions,
    u32::try_from(cut_craft_scripting::Limits::default().instructions / 1_000_000)
        .expect("a count of millions")
);
advanced_default!(
    gadget_memory_mb,
    is_gadget_memory_mb,
    u32::try_from(cut_craft_scripting::Limits::default().memory >> 20).expect("a count of MB")
);
advanced_default!(
    gadget_most_commands,
    is_gadget_most_commands,
    u32::try_from(cut_craft_scripting::Limits::default().commands).expect("a small count")
);

/// A count from the options file as a `usize`.
pub(super) fn count(n: u32) -> usize {
    usize::try_from(n).unwrap_or(usize::MAX)
}

/// `mm` in whole micrometres, as the accuracy settings show it.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "small positive lengths"
)]
pub(super) fn micrometres(mm: f64) -> u32 {
    (mm * 1000.0).round() as u32
}

impl Default for Advanced {
    fn default() -> Self {
        Self {
            wayland_native: false,
            cores: 0,
            dxf_spline_detail: dxf_spline_detail(),
            library_rescan_seconds: library_rescan_seconds(),
            library_most_files: library_most_files(),
            preview_cache_days: preview_cache_days(),
            previews_on_card: previews_on_card(),
            curve_accuracy: curve_accuracy(),
            roughing_3d_accuracy: roughing_3d_accuracy(),
            finishing_3d_accuracy: finishing_3d_accuracy(),
            rest_threshold: rest_threshold(),
            nest_most_sheets: nest_most_sheets(),
            nest_most_fillers: nest_most_fillers(),
            gadget_million_instructions: gadget_million_instructions(),
            gadget_memory_mb: gadget_memory_mb(),
            gadget_most_commands: gadget_most_commands(),
        }
    }
}

impl Advanced {
    /// Whether this desktop is a Wayland one on Linux or a BSD, where `wayland_native` applies.
    pub fn wayland() -> bool {
        cfg!(all(unix, not(target_os = "macos"))) && std::env::var_os("WAYLAND_DISPLAY").is_some()
    }

    /// Hands these settings to the code they steer.
    pub fn apply(&self) {
        cut_craft_core::import_settings::set_spline_samples(self.dxf_spline_detail);
        let (relist, most, unused_days, keep) = self.library_limits();
        crate::clipart::limits::set(relist, most, unused_days, keep);
        cut_craft_core::accuracy::set(self.accuracy());
        cut_craft_scripting::set_gadget_limits(self.gadget_limits());
    }

    /// What a gadget may do before it is stopped, each kept to its range.
    pub fn gadget_limits(&self) -> cut_craft_scripting::Limits {
        cut_craft_scripting::kept_to_ranges(cut_craft_scripting::Limits {
            instructions: u64::from(self.gadget_million_instructions) * 1_000_000,
            memory: count(self.gadget_memory_mb) << 20,
            commands: count(self.gadget_most_commands),
        })
    }

    /// The Library's rescan seconds, most files, days an unused preview is kept and previews
    /// on the graphics card, each kept to its range.
    pub fn library_limits(&self) -> (u64, usize, u64, usize) {
        (
            u64::from(self.library_rescan_seconds.clamp(1, 3600)),
            count(self.library_most_files.clamp(1000, 10_000_000)),
            u64::from(self.preview_cache_days.clamp(1, 3650)),
            count(self.previews_on_card.clamp(50, 5000)),
        )
    }

    /// How exactly toolpaths are worked out, in mm, each kept to its range.
    pub fn accuracy(&self) -> cut_craft_core::accuracy::Accuracy {
        let mm = |um: u32| f64::from(um) / 1000.0;
        cut_craft_core::accuracy::kept_to_ranges(cut_craft_core::accuracy::Accuracy {
            curve: mm(self.curve_accuracy),
            roughing_3d: mm(self.roughing_3d_accuracy),
            finishing_3d: mm(self.finishing_3d_accuracy),
            rest: mm(self.rest_threshold),
            nest_sheets: count(self.nest_most_sheets),
            nest_fillers: count(self.nest_most_fillers),
        })
    }

    /// The check saved beside these settings: a keyed hash of them, so an edit made outside
    /// PowerWhittle shows. The key ships in the app, so it stops a hand edit, not a determined
    /// one; that is all it is for.
    pub(super) fn seal(&self) -> String {
        seal_of(&serde_json::to_vec(self).expect("settings serialise"))
    }

    /// Whether `seal` is these settings' own. Options saved before the check have none: they
    /// pass when only `wayland_native`, the one setting there was then, is set.
    pub(super) fn sealed_by(&self, seal: Option<&str>) -> bool {
        match seal {
            Some(seal) => seal == self.seal(),
            None => {
                *self
                    == Self {
                        wayland_native: self.wayland_native,
                        ..Self::default()
                    }
            }
        }
    }
}

/// The keyed hash of `bytes` that [`Advanced::seal`] saves, in hex.
pub(super) fn seal_of(bytes: &[u8]) -> String {
    use hmac::Mac;
    let mut mac = hmac::Hmac::<sha2::Sha256>::new_from_slice(SEAL_KEY).expect("any key fits");
    mac.update(bytes);
    mac.finalize()
        .into_bytes()
        .iter()
        .fold(String::new(), |mut hex, b| {
            use std::fmt::Write as _;
            let _ = write!(hex, "{b:02x}");
            hex
        })
}

/// The key of [`Advanced::seal`], and the options file's name for it.
pub(super) const SEAL_KEY: &[u8] = b"Cut Craft advanced settings, v1";
pub(super) const SEAL: &str = "advanced_check";

/// The warning above the advanced settings, and what it goes on to say.
pub const WARNING: &str = "⚠ Changing these can freeze PowerWhittle or spoil a cut.";
pub const WARNING_MORE: &str = "They set how hard PowerWhittle works your computer and how exactly \
    it cuts. Leave them alone unless you know why you are changing one; Reset to defaults puts \
    them all back.";

/// What reading the options file found: the options, whether the advanced ones were
/// changed outside PowerWhittle, and the settings that could not be read.
#[derive(Debug, Default)]
pub struct Read {
    pub options: Options,
    pub tampered: bool,
    /// Each setting left at its default, as `section ▸ setting` (or the section, or
    /// `everything`); empty when the whole file was read.
    pub unread: Vec<String>,
}

/// What the status bar says when some settings could not be read.
pub fn unread_message(unread: &[String]) -> String {
    format!(
        "Some settings could not be read, so they are back to their defaults: {}. Your old \
         settings file is kept beside the new one as options.json.bak. Set them again in \
         Edit ▸ Options.",
        unread.join(", ")
    )
}

/// What the status bar says when the advanced settings were changed outside PowerWhittle.
pub const TAMPERED: &str = "Advanced settings were changed outside PowerWhittle, so they were put \
    back to their safe defaults. Change them in Edit ▸ Options ▸ Advanced.";

/// What the status bar says after the options are read: what could not be read, and that
/// the advanced settings were put back, both when both happened; nothing when all was well.
pub fn load_message(unread: &[String], tampered: bool) -> Option<String> {
    match (unread.is_empty(), tampered) {
        (true, false) => None,
        (true, true) => Some(TAMPERED.to_owned()),
        (false, false) => Some(unread_message(unread)),
        (false, true) => Some(format!("{} {TAMPERED}", unread_message(unread))),
    }
}
