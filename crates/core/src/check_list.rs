//! The checks before cutting, each by name: what it looks for and what it protects
//! against, in the order Save Toolpaths lists them. Every [`crate::preflight::Finding`]
//! says which check found it, so the list can show each check passed, needing a look, or
//! needing a fix, and a check that could not be made says why ([`crate::preflight::not_checked`]).

/// One check before cutting.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Check {
    /// Toolpaths with moves, a feed rate, a plunge rate and a spindle speed.
    CanCut,
    /// Toolpaths made from the design as it is now.
    UpToDate,
    /// The job's Z zero and X0 Y0 were looked at, not only taken from Job Defaults.
    Zeroing,
    /// The toolpaths fit on the machine's bed.
    Bed,
    /// The toolpaths go no higher and deeper than the Z axis travels.
    ZTravel,
    /// Feed and plunge rates no faster than the machine moves.
    MachineSpeeds,
    /// Spindle speeds the spindle can turn at.
    SpindleRange,
    /// Every bit loaded in the tool changer.
    ToolChanger,
    /// No two bits sharing a tool number (0, changed by hand, may be shared).
    ToolNumbers,
    /// No cutting into the keep-out zones (clamps).
    KeepOut,
    /// A rotary job's rotary set up: its centre line found, lying the job's way.
    RotarySetup,
    /// Rapid moves clear of the material.
    Rapids,
    /// Cutting that takes material off, not air.
    AirCutting,
    /// The shank, collet nut and spindle clear of the material.
    Collisions,
    /// No cutting deeper than the bit's flutes reach.
    Reach,
    /// No cutting well below the material, into the spoilboard.
    Spoilboard,
    /// Going down into the material the way the toolpath asked.
    Plunges,
    /// Drills only drilling, and holes the drill's size.
    Drilling,
    /// The bit fits every corner and detail it is asked to cut.
    MissedSpots,
    /// Parts cut free are held down (tabs).
    LooseParts,
    /// Copies kept on the material.
    OnMaterial,
    /// Bits from a maker's catalogue rather than generic ones.
    GenericTool,
    /// Feeds and speeds near the maker's figures for the bit.
    MakersFigures,
}

/// Which part of the checks finds a check's problems: the checks run in this order, so
/// while they run each check is waiting, running or checked.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Phase {
    /// Each toolpath's own settings and bit, one toolpath after another.
    Settings,
    /// The toolpaths together against the machine and the keep-out zones.
    Machine,
    /// The toolpaths cut in order from uncut material.
    Cutting,
    /// The shank, collet nut and spindle followed down every deep cut.
    Collisions,
}

/// What the progress says while each phase runs: [`Phase::of_stage`] reads it back.
const SETTINGS_STAGE: &str = "Checking the settings and reach of ";
const MACHINE_STAGE: &str = "Checking the toolpaths fit the machine";
const CUTTING_STAGE: &str = "Cutting the toolpaths in order to look for crashes and air cutting";
const COLLISIONS_STAGE: &str = "Looking for the collet or spindle hitting the material";

impl Phase {
    /// What the progress says while this phase runs (the settings' for `toolpath`).
    #[must_use]
    pub fn stage(self, toolpath: &str) -> String {
        match self {
            Self::Settings => format!("{SETTINGS_STAGE}{toolpath}"),
            Self::Machine => MACHINE_STAGE.to_owned(),
            Self::Cutting => CUTTING_STAGE.to_owned(),
            Self::Collisions => COLLISIONS_STAGE.to_owned(),
        }
    }

    /// The phase running when the progress says `stage`; `None` before any has started.
    #[must_use]
    pub fn of_stage(stage: &str) -> Option<Self> {
        if stage.starts_with(SETTINGS_STAGE) {
            Some(Self::Settings)
        } else if stage == MACHINE_STAGE {
            Some(Self::Machine)
        } else if stage == CUTTING_STAGE {
            Some(Self::Cutting)
        } else if stage == COLLISIONS_STAGE {
            Some(Self::Collisions)
        } else {
            None
        }
    }
}

impl Check {
    /// Every check, in the order they are listed.
    pub const ALL: [Self; 23] = [
        Self::CanCut,
        Self::UpToDate,
        Self::Zeroing,
        Self::Bed,
        Self::ZTravel,
        Self::MachineSpeeds,
        Self::SpindleRange,
        Self::ToolChanger,
        Self::ToolNumbers,
        Self::KeepOut,
        Self::RotarySetup,
        Self::Rapids,
        Self::AirCutting,
        Self::Collisions,
        Self::Reach,
        Self::Spoilboard,
        Self::Plunges,
        Self::Drilling,
        Self::MissedSpots,
        Self::LooseParts,
        Self::OnMaterial,
        Self::GenericTool,
        Self::MakersFigures,
    ];

    /// Its name in the list, saying what passing it means.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::CanCut => "Settings that can cut",
            Self::UpToDate => "Toolpaths up to date",
            Self::Zeroing => "Zero set where the job says",
            Self::Bed => "Fits on the machine's bed",
            Self::ZTravel => "Fits the Z axis travel",
            Self::MachineSpeeds => "Feeds the machine can reach",
            Self::SpindleRange => "Spindle speeds the spindle can turn",
            Self::ToolChanger => "Bits loaded in the tool changer",
            Self::ToolNumbers => "Each bit its own tool number",
            Self::KeepOut => "Keeps out of keep-out zones",
            Self::RotarySetup => "Rotary set up",
            Self::Rapids => "Rapid moves clear of the material",
            Self::AirCutting => "Not cutting air",
            Self::Collisions => "Collet and spindle clear of the material",
            Self::Reach => "Bit long enough for the depth",
            Self::Spoilboard => "Not cutting into the spoilboard",
            Self::Plunges => "Going down into the material",
            Self::Drilling => "Drills and holes",
            Self::MissedSpots => "Bit fits every corner",
            Self::LooseParts => "Cut-out parts held down",
            Self::OnMaterial => "Stays on the material",
            Self::GenericTool => "Bits from a maker's catalogue",
            Self::MakersFigures => "Feeds and speeds as the maker gives",
        }
    }

    /// What it looks for and what that protects against, for a beginner: shown when the
    /// check is pointed at, and at the top of its help.
    #[must_use]
    pub fn guards(self) -> &'static str {
        match self {
            Self::CanCut => {
                "Every toolpath has moves, a feed rate, a plunge rate and a spindle speed, \
                 so the machine does not stand still or cut with the spindle off"
            }
            Self::UpToDate => {
                "Each toolpath was made from the design as it is now, so it cuts what you see \
                 and not an older shape"
            }
            Self::Zeroing => {
                "The job says where you zero the bit (Z zero on top of the material or on \
                 the bed, X0 Y0 at a corner or the middle). When those came from Job Defaults \
                 and were never looked at for this job, you are asked to check them, so the \
                 machine does not cut in the air or deep into the bed"
            }
            Self::Bed => {
                "The toolpaths fit inside the machine's bed, so it does not run into its \
                 own end stops part-way through the job"
            }
            Self::ZTravel => {
                "The toolpaths go no higher or deeper than the Z axis can travel, so the \
                 spindle does not hit the top of its travel or fall short of the depth"
            }
            Self::MachineSpeeds => {
                "No feed or plunge rate is faster than the machine moves, so the cut runs \
                 as planned and the job time is right"
            }
            Self::SpindleRange => {
                "Each spindle speed is one the spindle can turn at, so the bit is not \
                 cutting slower or faster than set (which burns the wood or breaks bits)"
            }
            Self::ToolChanger => {
                "Every bit the job uses is loaded in the tool changer, so the machine does \
                 not stop half-way to ask for one"
            }
            Self::ToolNumbers => {
                "No two different bits share a tool number, so a tool changer never picks up \
                 the wrong bit. Tool number 0 means the bit is changed by hand, and any \
                 number of bits may share it"
            }
            Self::KeepOut => {
                "No cut goes into a keep-out zone, so the bit does not hit a clamp or \
                 anything else you marked to stay clear of"
            }
            Self::RotarySetup => {
                "For a rotary job, the rotary's centre line has been found and it lies the way \
                 the job's cylinder does, so the bit cuts round the real turning axis and a \
                 round carving does not come out lumpy or on the wrong side"
            }
            Self::Rapids => {
                "Rapid moves (full speed, not cutting) stay clear of the material that is \
                 still there, so the bit does not crash into the work"
            }
            Self::AirCutting => {
                "The cutting moves take material off rather than cutting where it is \
                 already gone, so the job does not waste time"
            }
            Self::Collisions => {
                "On deep cuts, the plain shank, the collet nut that holds the bit and the \
                 spindle body stay clear of the wood still standing and of clamps, so they do not hit it, \
                 spoil the part or pull the bit out of the collet. Always asked about, \
                 even when ticked"
            }
            Self::Reach => {
                "No toolpath cuts deeper than the bit's flutes reach, so the shank does not \
                 rub and burn, and the bit does not snap"
            }
            Self::Spoilboard => {
                "No cut goes more than a little below the material into the spoilboard \
                 (or the machine's bed), as a wrong thickness or depth would"
            }
            Self::Plunges => {
                "The bit goes down into the material the way the toolpath asked (a ramp or \
                 a zig-zag), not straight down where that is hard on it"
            }
            Self::Drilling => {
                "Drills only drill straight down, and each hole drawn is the size the drill \
                 makes, so a drill is not snapped or a hole cut the wrong size"
            }
            Self::MissedSpots => {
                "The bit fits into every corner and detail it is asked to cut, so nothing is \
                 left behind that you expected cut"
            }
            Self::LooseParts => {
                "Parts cut right through are held by tabs (or you know to hold them), so a \
                 loose part is not thrown or pulled into the bit"
            }
            Self::OnMaterial => {
                "Copies of a toolpath stay on the material, so none is cut in the air or \
                 into the bed"
            }
            Self::GenericTool => {
                "Each bit comes from a maker's catalogue, whose feeds and speeds are made \
                 for that bit. A generic bit's are only starting values"
            }
            Self::MakersFigures => {
                "Each bit's feed rate, plunge rate and spindle speed are near the maker's \
                 figures for it, so a changed number that could break the bit or burn the \
                 wood is not missed"
            }
        }
    }

    /// The help topic that explains it.
    #[must_use]
    pub fn help(self) -> &'static str {
        match self {
            Self::CanCut => "check_can_cut",
            Self::UpToDate => "check_up_to_date",
            Self::Zeroing => "check_zeroing",
            Self::Bed => "check_bed",
            Self::ZTravel => "check_z_travel",
            Self::MachineSpeeds => "check_machine_speeds",
            Self::SpindleRange => "check_spindle_range",
            Self::ToolChanger => "check_tool_changer",
            Self::ToolNumbers => "check_tool_numbers",
            Self::KeepOut => "check_keep_out",
            Self::RotarySetup => "check_rotary_setup",
            Self::Rapids => "check_rapids",
            Self::AirCutting => "check_air_cutting",
            Self::Collisions => "check_collisions",
            Self::Reach => "check_reach",
            Self::Spoilboard => "check_spoilboard",
            Self::Plunges => "check_plunges",
            Self::Drilling => "check_drilling",
            Self::MissedSpots => "check_missed_spots",
            Self::LooseParts => "check_loose_parts",
            Self::OnMaterial => "check_on_material",
            Self::GenericTool => "check_generic_tool",
            Self::MakersFigures => "check_makers_figures",
        }
    }

    /// The part of the checks that finds its problems.
    #[must_use]
    pub fn phase(self) -> Phase {
        match self {
            Self::Zeroing
            | Self::Bed
            | Self::ZTravel
            | Self::KeepOut
            | Self::ToolNumbers
            | Self::RotarySetup => Phase::Machine,
            Self::Rapids | Self::AirCutting => Phase::Cutting,
            Self::Collisions => Phase::Collisions,
            _ => Phase::Settings,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_check_is_listed_once_with_a_name_and_help_of_its_own() {
        let mut names: Vec<&str> = Check::ALL.iter().map(|c| c.name()).collect();
        let mut helps: Vec<&str> = Check::ALL.iter().map(|c| c.help()).collect();
        names.sort_unstable();
        names.dedup();
        helps.sort_unstable();
        helps.dedup();
        assert_eq!(names.len(), Check::ALL.len());
        assert_eq!(helps.len(), Check::ALL.len());
        assert!(Check::ALL.iter().all(|c| c.guards().len() > 40));
    }

    #[test]
    fn the_running_phase_is_read_back_from_the_progress() {
        for phase in [
            Phase::Settings,
            Phase::Machine,
            Phase::Cutting,
            Phase::Collisions,
        ] {
            assert_eq!(Phase::of_stage(&phase.stage("Pocket 1")), Some(phase));
        }
        assert_eq!(Phase::of_stage(""), None);
    }
}
