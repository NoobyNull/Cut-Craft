//! Performance: how much of the computer calculations may use, and how much detail the
//! previews and the 3D view are drawn with, in one place.

use super::{Deserialize, JsonSchema, Serialize};
use super::{Detail, MovingDetail, preview_detail, target_fps, yes};
use cut_craft_core::work::Limit;

/// How hard PowerWhittle works the computer: how much of it calculations may use, and how much
/// detail the previews and the 3D view are drawn with. Toward speed or toward detail, each.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Performance {
    /// While ticked, calculations (toolpaths, previews, simulations, imports) use only the
    /// share of the processor set below, and give way to your other programs: a video or
    /// the browser stays smooth. Unticked, they use every core as fast as the computer goes,
    /// but other programs may stutter meanwhile. Applies from the next calculation
    #[serde(default = "yes")]
    #[schemars(title = "Limit calculations so other programs stay smooth")]
    pub limit_calculations: bool,
    /// How much of the processor calculations may use: less keeps other programs smooth,
    /// more finishes sooner. 80% leaves a core or more free for everything else
    #[serde(default = "share")]
    #[schemars(
        title = "Share of the computer to use",
        extend(
            "x-unit" = "%",
            "x-widget" = "slider",
            "x-show-when" = {"limit_calculations": "true"}
        ),
        range(min = 1, max = 100)
    )]
    pub share: u8,
    /// How finely a toolpath preview is worked out to start with: toward Performance is
    /// quicker and uses less memory, toward Detail shows small corners, holes and thin
    /// walls truer (about 0.5 mm between points at 0%, 0.1 mm at 50%, 0.025 mm at 100%).
    /// The Preview tab has the same slider for now; Reset Preview goes back to this
    #[serde(default = "preview_detail")]
    #[schemars(
        title = "Preview detail",
        extend("x-unit" = "%", "x-widget" = "slider"),
        range(min = 0, max = 100)
    )]
    pub preview_detail: u8,
    /// How finely the model is drawn once the view is still: higher shows more detail and
    /// takes longer to draw after each change
    #[serde(default)]
    #[schemars(title = "Detail when still")]
    pub still_detail: Detail,
    /// How finely the model is drawn while it or the view moves (turning the view, dragging
    /// a handle): lower keeps it smooth on a slower computer. Never finer than the detail
    /// when still. A playing preview keeps the detail when still
    #[serde(default)]
    #[schemars(
        title = "Detail while moving",
        extend("x-hide-when" = {"adjust_to_frame_rate": "true"})
    )]
    pub moving_detail: MovingDetail,
    /// While the view moves, draw the model as finely as this computer can keep smooth:
    /// coarser when frames come slowly, finer again when they come quickly, never finer
    /// than the detail when still
    #[serde(default = "yes")]
    #[schemars(title = "Adjust detail to keep the view smooth")]
    pub adjust_to_frame_rate: bool,
    /// How many pictures a second the 3D view aims for while it moves: higher is smoother
    /// and coarser, lower sharper and choppier. 30 suits most; films run at 24
    #[serde(default = "target_fps")]
    #[schemars(
        title = "Smoothness to aim for",
        extend("x-unit" = "frames/s", "x-show-when" = {"adjust_to_frame_rate": "true"}),
        range(min = 10, max = 120)
    )]
    pub target_fps: f64,
    /// Show the 3D view's detail and frames a second at the foot of the window, to see what
    /// the setting above does
    #[serde(default)]
    #[schemars(title = "Show detail and frame rate in the status bar")]
    pub show_frame_rate: bool,
}

fn share() -> u8 {
    cut_craft_core::work::DEFAULT_SHARE
}

impl Default for Performance {
    fn default() -> Self {
        Self {
            limit_calculations: true,
            share: share(),
            preview_detail: preview_detail(),
            still_detail: Detail::default(),
            moving_detail: MovingDetail::default(),
            adjust_to_frame_rate: true,
            target_fps: target_fps(),
            show_frame_rate: false,
        }
    }
}

impl Performance {
    /// The calculation limit these settings ask for.
    pub fn limit(&self) -> Limit {
        Limit {
            on: self.limit_calculations,
            share: self.share.clamp(1, 100),
        }
    }

    /// Hands the calculation limit to the code that starts calculations' threads.
    pub fn apply(&self) {
        cut_craft_core::work::set_limit(self.limit());
    }

    /// Frames a second wanted while moving, kept to the setting's range.
    pub fn fps_wanted(&self) -> f64 {
        self.target_fps.clamp(10.0, 120.0)
    }

    /// Mesh cells along the job's longer side once the view is still, at most.
    pub fn still_cells(&self) -> usize {
        self.still_detail.cells()
    }

    /// Mesh cells along the job's longer side while the model or the view moves.
    pub fn moving_cells(&self) -> usize {
        let still = self.still_detail;
        let level = match self.moving_detail {
            MovingDetail::OneStepLower => still.lower(1),
            MovingDetail::TwoStepsLower => still.lower(2),
            MovingDetail::Low => Detail::Low,
            MovingDetail::Medium => Detail::Medium,
            MovingDetail::High => Detail::High,
            MovingDetail::VeryHigh => Detail::VeryHigh,
        };
        level.min(still).cells()
    }
}

/// The settings this page took over from older versions' options file, carried over once
/// when it has no Performance page yet: the 3D View page's detail settings, and the hidden
/// Advanced "Processor cores to use (0 = all)", turned into a share of this computer's cores.
pub(super) fn carry_over(file: &mut serde_json::Value) {
    let Some(sections) = file.as_object_mut() else {
        return;
    };
    if sections.contains_key(PAGE) {
        return;
    }
    let mut page = serde_json::Map::new();
    if let Some(view) = sections
        .get("view_3d")
        .and_then(serde_json::Value::as_object)
    {
        for key in MOVED_FROM_VIEW {
            if let Some(value) = view.get(key) {
                page.insert(key.to_owned(), value.clone());
            }
        }
    }
    let cores = sections
        .get("advanced")
        .and_then(|a| a.get("cores"))
        .and_then(serde_json::Value::as_u64)
        .and_then(|c| usize::try_from(c).ok())
        .unwrap_or(0);
    let limit = Limit::from_old_cores(cores, cut_craft_core::work::all_cores());
    page.insert("limit_calculations".to_owned(), limit.on.into());
    page.insert("share".to_owned(), limit.share.into());
    sections.insert(PAGE.to_owned(), page.into());
}

/// This page's key in the options file.
pub const PAGE: &str = "performance";

/// The settings that were on the 3D View page before this one.
const MOVED_FROM_VIEW: [&str; 6] = [
    "preview_detail",
    "still_detail",
    "moving_detail",
    "adjust_to_frame_rate",
    "target_fps",
    "show_frame_rate",
];
