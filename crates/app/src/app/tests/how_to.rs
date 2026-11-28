//! The How-to suite: every tool's tour (`help_tours`) played in the app headless, step by step,
//! as `--help-pictures` plays it, without taking pictures. Each step must still find what it
//! rings, so a moved or renamed button, field or tab shows up here before the help pictures go
//! stale. Split in shards so the tours play side by side.

use super::*;
use crate::app::help_pictures::{Mark, where_is};
use crate::app::help_tours::every_tour;

/// How many shards the tours are split into: one test each.
const SHARDS: usize = 8;

/// Tours that take minutes (a full 3D calculation), played in a test of their own so no shard
/// waits on one.
const SLOW: &[&str] = &["3d_finishing_toolpath"];

/// Plays every quick tour whose place in the list leaves `shard` over [`SHARDS`].
fn play(shard: usize) {
    let quick = every_tour().filter(|t| !SLOW.contains(&t.tool));
    play_tours(quick.skip(shard).step_by(SHARDS));
}

/// Plays the tours, and fails naming every step that found nothing to ring.
fn play_tours<'a>(tours: impl Iterator<Item = &'a crate::app::help_pictures::Tour>) {
    let mut h = Harness::at_start();
    crate::marks::switch_on(&h.ctx);
    // dialogs drawn inside the main window, as the tours take them
    h.ctx.set_embed_viewports(true);
    let mut lost = vec![];
    for tour in tours {
        for (n, step) in tour.steps.iter().enumerate() {
            if let Mark::At(name) = step.mark {
                // found only if this step draws it, not from an earlier step
                crate::marks::forget(&h.ctx, name);
            }
            (step.setup)(&mut h.app, &h.ctx);
            if let Mark::At(name) = step.mark {
                crate::marks::want(&h.ctx, name);
            }
            // forms open and settle; calculations finish in place (the harness waits for them)
            for _ in 0..4 {
                h.frame(vec![]);
            }
            if !matches!(step.mark, Mark::None) && where_is(&h.app, &h.ctx, step.mark).is_none() {
                lost.push(format!("{} step {}: {:?}", tour.tool, n + 1, step.mark));
            }
        }
    }
    assert!(
        lost.is_empty(),
        "these tour steps find nothing to ring (crates/app/src/help_tours/):\n{}",
        lost.join("\n")
    );
}

#[test]
fn tours_shard_1_ring_what_they_point_at() {
    play(0);
}

#[test]
fn tours_shard_2_ring_what_they_point_at() {
    play(1);
}

#[test]
fn tours_shard_3_ring_what_they_point_at() {
    play(2);
}

#[test]
fn tours_shard_4_ring_what_they_point_at() {
    play(3);
}

#[test]
fn tours_shard_5_ring_what_they_point_at() {
    play(4);
}

#[test]
fn tours_shard_6_ring_what_they_point_at() {
    play(5);
}

#[test]
fn tours_shard_7_ring_what_they_point_at() {
    play(6);
}

#[test]
fn tours_shard_8_ring_what_they_point_at() {
    play(7);
}

#[test]
fn the_slow_tours_ring_what_they_point_at() {
    for slow in SLOW {
        assert!(every_tour().any(|t| t.tool == *slow), "{slow} has no tour");
    }
    play_tours(every_tour().filter(|t| SLOW.contains(&t.tool)));
}

/// A tour step ringing a setting off a toolpath form's first tab says which tab it is on, in
/// bold as the tab reads, so a beginner following the help finds it.
#[test]
fn a_tour_step_on_a_later_tab_names_the_tab() {
    use crate::form::form_tabs::{self, FormTab};
    let mut unnamed = vec![];
    let registry = crate::commands::registry();
    for tour in every_tour() {
        let Some(command) = registry.get(tour.tool) else {
            continue;
        };
        let fields = crate::form::arrange(crate::form::fields(&command.schema()));
        if !form_tabs::tabbed(&fields) {
            continue;
        }
        let own = form_tabs::own_name(tour.tool, "");
        for (n, step) in tour.steps.iter().enumerate() {
            let Mark::At(mark) = step.mark else { continue };
            let Some(tab) = form_tabs::tab_of_mark(&fields, mark) else {
                continue;
            };
            let named = format!("**{}**", form_tabs::name(tab, &own));
            if tab != FormTab::Cutting && !step.caption.contains(&named) {
                unnamed.push(format!("{} step {}: {named}", tour.tool, n + 1));
            }
        }
    }
    assert!(unnamed.is_empty(), "say the tab:\n{}", unnamed.join("\n"));
}
