//! Which post a job's toolpaths are saved through on a machine: the one picked, else the
//! machine's default when it suits the job, else its first post that does (a flat post for
//! a flat job, one wrapping the side that goes round for a rotary job), told the machine's
//! rotary settings. Without a machine (or one with no posts), Cut Craft's own post for this
//! kind of job.

use crate::{Library, Machine, NO_A_AXIS};
use cut_craft_core::document::{Along, Job};
use cut_craft_post::Post;

/// The axis a rotary job wraps round onto A: Y for a cylinder along X, X for one along Y;
/// `None` for a flat job.
pub fn wraps_for(job: &Job) -> Option<&'static str> {
    job.rotary.map(|r| match r.along {
        Along::X => "Y",
        Along::Y => "X",
    })
}

/// The post `job` is saved through on `machine`: `picked` when one was (by name), else the
/// machine's own choice ([`post_for`]'s rules in the module docs). An error says why there
/// is none and how to put it right.
pub fn post_for(
    machine: Option<&Machine>,
    job: &Job,
    library: &Library,
    picked: Option<&str>,
) -> Result<Post, String> {
    if let Some(why) = no_a_axis(machine, job) {
        return Err(why);
    }
    let post = match picked {
        Some(name) => library.load(name)?,
        None => as_written(machine, job, library)?,
    };
    Ok(on_machine(machine, post))
}

/// The machine's posts that suit `job` (flat, or wrapping its side), for Save Toolpaths to
/// choose from; none without a machine (Cut Craft's own is used).
pub fn post_choices(machine: Option<&Machine>, job: &Job, library: &Library) -> Vec<String> {
    let wraps = wraps_for(job);
    machine.map_or_else(Vec::new, |machine| {
        machine
            .posts
            .iter()
            .filter(|name| {
                library
                    .load(name)
                    .is_ok_and(|p| p.wraps() == wraps && (wraps.is_none() || machine.has_rotary()))
            })
            .cloned()
            .collect()
    })
}

/// Why a rotary job can't be saved for `machine`: its firmware has no A axis (standard
/// GRBL, also what unknown firmware is taken for), whatever its posts and 4th axis tick.
/// `None` for a flat job, no machine, or firmware with an A axis.
fn no_a_axis(machine: Option<&Machine>, job: &Job) -> Option<String> {
    let machine = machine?;
    (job.rotary.is_some() && !machine.firmware().has_a_axis()).then(|| {
        format!(
            "{} cannot cut this rotary job. {NO_A_AXIS} If its controller runs other firmware, \
             pick it under Firmware in Machine Configuration",
            machine.name,
        )
    })
}

/// `post` writing for `machine`: a rotary post winds A as its rotary allows (Machine
/// Configuration ▸ Rotary), with its backlash and top speed.
fn on_machine(machine: Option<&Machine>, post: Post) -> Post {
    match machine {
        Some(m) if m.has_rotary() && post.wraps().is_some() => post.with_rotary(m.rotary_axis),
        _ => post,
    }
}

/// The machine's choice as its file is written, not yet told the machine's rotary.
fn as_written(machine: Option<&Machine>, job: &Job, library: &Library) -> Result<Post, String> {
    let Some(machine) = machine.filter(|m| !m.posts.is_empty()) else {
        return Ok(Post::default_for(job));
    };
    let wraps = wraps_for(job);
    if wraps.is_some() && !machine.has_rotary() {
        return Err(format!(
            "{} has no 4th axis for this rotary job: tick its 4th axis (A) in Machine \
             Configuration",
            machine.name
        ));
    }
    // the default first: one that will not load says so rather than being passed over
    let default = machine.post().map(|name| library.load(name)).transpose()?;
    if let Some(post) = default.filter(|p| p.wraps() == wraps) {
        return Ok(post);
    }
    machine
        .posts
        .iter()
        .filter_map(|name| library.load(name).ok())
        .find(|p| p.wraps() == wraps)
        .ok_or_else(|| match wraps {
            Some(axis) => format!(
                "{} has no post for this rotary job: add a Rotary A, cylinder along {} post \
                 in Machine Configuration",
                machine.name,
                if axis == "Y" { "X" } else { "Y" }
            ),
            None => format!(
                "{} has only rotary posts: add a flat one in Machine Configuration",
                machine.name
            ),
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Firmware;
    use cut_craft_core::document::Rotary;

    const FLAT: &str = "Cut Craft GRBL Arcs (mm) (*.nc)";
    const ROUND_X: &str = "Cut Craft grblHAL Rotary A, cylinder along X (mm) (*.nc)";
    const ROUND_Y: &str = "Cut Craft grblHAL Rotary A, cylinder along Y (mm) (*.nc)";

    fn library() -> Library {
        Library::new(None)
    }

    fn machine(posts: &[&str], firmware: Firmware, rotary: bool) -> Machine {
        Machine {
            name: "Shop router".into(),
            posts: posts.iter().map(|p| (*p).to_owned()).collect(),
            firmware: Some(firmware),
            rotary,
            ..Machine::default()
        }
    }

    fn flat() -> Job {
        Job {
            rotary: None,
            ..Job::default()
        }
    }

    fn rotary(along: Along) -> Job {
        Job {
            rotary: Some(Rotary {
                diameter: 50.0,
                along,
                support: cut_craft_core::document::Support::default(),
                blank: cut_craft_core::document::Blank::default(),
            }),
            ..Job::default()
        }
    }

    fn name(post: Result<Post, String>) -> String {
        post.expect("a post").name
    }

    #[test]
    fn no_machine_uses_cut_crafts_own_post_for_the_kind_of_job() {
        let lib = library();
        assert_eq!(
            name(post_for(None, &flat(), &lib, None)),
            Post::default_for(&flat()).name
        );
        let round = rotary(Along::Y);
        let post = post_for(None, &round, &lib, None).unwrap();
        assert_eq!(post.wraps(), Some("X"));
    }

    #[test]
    fn a_machine_without_posts_uses_cut_crafts_own_post() {
        let m = machine(&[], Firmware::FluidNc, false);
        assert_eq!(
            name(post_for(Some(&m), &flat(), &library(), None)),
            Post::default_for(&flat()).name
        );
    }

    #[test]
    fn the_default_post_is_used_when_it_suits_the_job() {
        let mut m = machine(&[FLAT, ROUND_X], Firmware::FluidNc, true);
        assert_eq!(name(post_for(Some(&m), &flat(), &library(), None)), FLAT);
        // a rotary default is passed over for a flat job: the first flat post is used
        m.default_post = Some(ROUND_X.into());
        assert_eq!(name(post_for(Some(&m), &flat(), &library(), None)), FLAT);
    }

    #[test]
    fn a_rotary_job_uses_the_post_wrapping_its_side() {
        let m = machine(&[FLAT, ROUND_Y, ROUND_X], Firmware::GrblHal, true);
        assert_eq!(
            name(post_for(Some(&m), &rotary(Along::X), &library(), None)),
            ROUND_X
        );
        assert_eq!(
            name(post_for(Some(&m), &rotary(Along::Y), &library(), None)),
            ROUND_Y
        );
    }

    #[test]
    fn standard_grbl_cannot_cut_a_rotary_job_even_with_a_post_picked() {
        let m = machine(&[FLAT, ROUND_X], Firmware::Grbl, true);
        for picked in [None, Some(ROUND_X)] {
            let why = post_for(Some(&m), &rotary(Along::X), &library(), picked).unwrap_err();
            assert!(
                why.contains("Shop router cannot cut this rotary job"),
                "{why}"
            );
            assert!(why.contains(NO_A_AXIS), "{why}");
        }
        // a flat job on the same machine is fine
        assert!(post_for(Some(&m), &flat(), &library(), None).is_ok());
    }

    #[test]
    fn a_rotary_job_needs_the_4th_axis_ticked() {
        let m = machine(&[FLAT, ROUND_X], Firmware::FluidNc, false);
        let why = post_for(Some(&m), &rotary(Along::X), &library(), None).unwrap_err();
        assert!(why.contains("has no 4th axis"), "{why}");
    }

    #[test]
    fn missing_posts_say_which_to_add() {
        let m = machine(&[FLAT], Firmware::FluidNc, true);
        let why = post_for(Some(&m), &rotary(Along::X), &library(), None).unwrap_err();
        assert!(why.contains("cylinder along X"), "{why}");
        let m = machine(&[ROUND_X], Firmware::FluidNc, true);
        let why = post_for(Some(&m), &flat(), &library(), None).unwrap_err();
        assert!(why.contains("has only rotary posts"), "{why}");
    }

    #[test]
    fn a_default_post_that_will_not_load_says_so() {
        let mut m = machine(&[FLAT], Firmware::FluidNc, false);
        m.posts.insert(0, "Gone".into());
        let why = post_for(Some(&m), &flat(), &library(), None).unwrap_err();
        assert!(why.contains("no post-processor named Gone"), "{why}");
    }

    #[test]
    fn a_picked_post_is_used_and_told_the_machines_rotary() {
        let m = machine(&[FLAT, ROUND_X], Firmware::FluidNc, true);
        let picked = post_for(Some(&m), &rotary(Along::X), &library(), Some(ROUND_X)).unwrap();
        assert_eq!(picked.name, ROUND_X);
        assert!(
            format!("{picked:?}").contains("rotary: Some("),
            "{picked:?}"
        );
        // without the 4th axis ticked the post is left as written
        let m = machine(&[FLAT, ROUND_X], Firmware::FluidNc, false);
        let picked = post_for(Some(&m), &flat(), &library(), Some(ROUND_X)).unwrap();
        assert!(format!("{picked:?}").contains("rotary: None"), "{picked:?}");
    }

    #[test]
    fn choices_are_the_machines_posts_that_suit_the_job() {
        let m = machine(&[FLAT, ROUND_X, ROUND_Y, "Gone"], Firmware::FluidNc, true);
        let lib = library();
        assert_eq!(post_choices(Some(&m), &flat(), &lib), vec![FLAT.to_owned()]);
        assert_eq!(
            post_choices(Some(&m), &rotary(Along::X), &lib),
            vec![ROUND_X.to_owned()]
        );
        // rotary posts are offered only with the 4th axis
        let m = machine(&[FLAT, ROUND_X], Firmware::FluidNc, false);
        assert_eq!(
            post_choices(Some(&m), &rotary(Along::X), &lib),
            Vec::<String>::new()
        );
        assert_eq!(post_choices(None, &flat(), &lib), Vec::<String>::new());
    }
}
