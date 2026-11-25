//! Part of the rotary carving engine.

use crate::{APPROACH, Outcome, Pass, Solid, Work, takes_solid};
use cut_craft_core::CommandError;
use cut_craft_core::component::placed_model;
use cut_craft_core::document::{Along, Document, Joined};
use cut_craft_core::glam::DVec3;
use cut_craft_core::rotary_axis;
use cut_craft_core::rotary_axis::Surroundings;
use cut_craft_core::simulation::Material;
use cut_craft_core::toolpath::{Move, MoveKind, Tool, follow};
use cut_craft_rotary::{Cutter, Head, Stock};

/// The model in two parts: the rest as heights placed in the material where Material
/// Setup puts them (`None` when there is none), and the full 3D models cut round exactly
/// where they really are in space: Material Setup's model position is a flat job's, and
/// never moves them.
pub fn parts(
    doc: &Document,
    tool: &Tool,
    allowance: f64,
) -> Result<(Option<Material>, Option<Solid>), CommandError> {
    let corner = doc.job.area().min;
    let along_from = match doc.job.rotary.map(|r| r.along) {
        Some(Along::Y) => corner.y,
        _ => corner.x,
    };
    let Some(solid) = Solid::of(doc, tool, allowance, 0.0, along_from) else {
        return Ok((Some(placed_model(doc, tool)?), None));
    };
    let rest = Document {
        components: doc
            .components
            .iter()
            .filter(|(_, k)| !takes_solid(k))
            .map(|(id, k)| (*id, k.clone()))
            .collect(),
        ..doc.clone()
    };
    let rest = if rest.components.is_empty() {
        None
    } else {
        Some(placed_model(&rest, tool)?)
    };
    Ok((rest, Some(solid)))
}

/// How far into the material the bit must go for a point to count as cutting, mm.
pub const GRAZE: f64 = 0.05;

/// `passes` with every stretch that cuts only air taken out, each broken where the bit
/// would be clear of the `stock` (the blank as Job Setup has it, turned as A says): the bit
/// lifts and moves over instead. One point of air is kept at each end of what is left, so
/// the bit comes onto the material and leaves it at its cutting height, and short gaps (less
/// than two bit widths) are cut through rather than lifted over.
pub fn in_material(
    passes: Vec<Pass>,
    stock: &Stock,
    cutter: &Cutter,
    along_x: bool,
    radius: f64,
) -> Vec<Pass> {
    let short = 2.0 * cutter.tool().diameter;
    let cuts = |&(point, across): &(DVec3, f64)| {
        let (along, round) = if along_x {
            (point.x, point.y)
        } else {
            (point.y, point.x)
        };
        stock
            .drop(cutter, along, across, round / radius)
            .is_some_and(|top| radius + point.z < top - GRAZE)
    };
    passes
        .into_iter()
        .flat_map(|pass| {
            let cutting: Vec<bool> = pass.iter().map(cuts).collect();
            runs(&pass, &cutting, &[], short)
                .into_iter()
                .map(|run| pass[run].to_vec())
                .collect::<Vec<_>>()
        })
        .collect()
}

/// The stretches of `pass` where `cutting` says the bit cuts something, each with one
/// point of air kept at either end, so the bit comes onto the material and leaves it at its
/// cutting height; gaps shorter than `short` are cut through rather than lifted over. A point
/// `blocked` (the holder would touch something there; none when it is empty) is never kept.
pub fn runs(
    pass: &[(DVec3, f64)],
    cutting: &[bool],
    blocked: &[bool],
    short: f64,
) -> Vec<std::ops::Range<usize>> {
    let free = |i: usize| !blocked.get(i).copied().unwrap_or(false);
    let mut keep = cutting.to_vec();
    for (i, &c) in cutting.iter().enumerate() {
        if c {
            if i > 0 && free(i - 1) {
                keep[i - 1] = true;
            }
            if i + 1 < keep.len() && free(i + 1) {
                keep[i + 1] = true;
            }
        }
    }
    let mut i = 0;
    while i < keep.len() {
        if keep[i] {
            i += 1;
            continue;
        }
        let from = i;
        while i < keep.len() && !keep[i] {
            i += 1;
        }
        if from > 0 && i < keep.len() && (from..i).all(free) {
            let length: f64 = pass[from - 1..=i]
                .windows(2)
                .map(|w| w[0].0.distance(w[1].0))
                .sum();
            if length < short {
                keep[from..i].fill(true);
            }
        }
    }
    let mut out = vec![];
    let mut from: Option<usize> = None;
    for (i, &kept) in keep.iter().enumerate() {
        match (kept, from) {
            (true, None) => from = Some(i),
            (false, Some(f)) => {
                out.push(f..i);
                from = None;
            }
            _ => {}
        }
    }
    if let Some(f) = from {
        out.push(f..keep.len());
    }
    out
}

/// The passes as moves, each from above its start and back up after: with how far across
/// the bit is held at each when it is `held_off` the axis (otherwise none).
pub fn joined(
    passes: &[(Pass, bool)],
    safe: f64,
    held_off: bool,
    radius: f64,
    along_x: bool,
) -> (Vec<Move>, Vec<f64>) {
    let round = usize::from(along_x);
    let mut moves: Vec<Move> = vec![];
    let mut across = vec![];
    for (pass, quick) in passes {
        let points: Vec<DVec3> = pass.iter().map(|&(point, _)| point).collect();
        let mut followed = follow(&points, safe);
        let mut held: Vec<f64> = std::iter::once(pass[0].1)
            .chain(pass.iter().map(|&(_, d)| d))
            .chain(std::iter::once(pass[pass.len() - 1].1))
            .collect();
        if *quick && followed.len() > 1 {
            // nothing above where it starts: down at the rapid rate to just above it, and
            // only the last of the way at the plunge rate
            let mut near = followed[1];
            near.kind = MoveKind::Rapid;
            near.to.z += APPROACH;
            followed.insert(1, near);
            held.insert(1, pass[0].1);
        }
        if held_off {
            // follow adds a rise to above the first point and from the last
            for (m, &d) in followed.iter_mut().zip(&held) {
                // kept where the tip of the bit held d across cuts, on the unrolled material
                let (w, z) = rotary_axis::tip_of(radius, m.to[round], m.to.z, d);
                m.to[round] = w;
                m.to.z = z;
            }
            across.extend(held);
        }
        moves.extend(followed);
    }
    (moves, across)
}

/// The angles typed in Tilt angles, in degrees: none when nothing is typed (they are picked),
/// and, in red, any word that is not an angle (left out; none left, they are picked).
pub fn angles(typed: &str) -> (Option<Vec<f64>>, Option<String>) {
    let mut good = vec![];
    let mut bad = vec![];
    for w in typed
        .split(|c: char| c == ',' || c == ';' || c.is_whitespace())
        .map(|w| w.trim_end_matches(['°', 'º']))
        .filter(|w| !w.is_empty())
    {
        match w.parse::<f64>().ok().filter(|a| a.is_finite()) {
            Some(a) => good.push(a.rem_euclid(360.0)),
            None => bad.push(format!("\"{w}\"")),
        }
    }
    let note = (!bad.is_empty()).then(|| {
        format!(
            "Tilt angles: {} {} not an angle in degrees, so {} left out{}. Type angles such as \
             15, 165, 45, or leave it empty to have them picked.",
            bad.join(", "),
            if bad.len() == 1 { "is" } else { "are" },
            if bad.len() == 1 { "it is" } else { "they are" },
            if good.is_empty() {
                " and the angles are picked"
            } else {
                ""
            },
        )
    });
    ((!good.is_empty()).then_some(good), note)
}

/// Where along the part the model is, from the material's start: its first point to its
/// last.
pub fn model_along(work: &Work) -> Option<(f64, f64)> {
    // the full models are kept in the job's own X and Y: counted from the material's start
    if let Some(solid) = work.solid
        && let Some((lo, hi)) = solid.mesh.bounds()
    {
        return Some((lo.x - solid.along_from, hi.x - solid.along_from));
    }
    let t = work.turned;
    let mut found: Option<(f64, f64)> = None;
    for i in 0..t.along {
        if (0..t.round)
            .any(|j| t.radii[j * t.along + i] > 1e-9 && t.radii[j * t.along + i] < t.radius - 1e-9)
        {
            #[expect(clippy::cast_precision_loss, reason = "cell counts are small")]
            let (a, b) = (i as f64 * t.along_cell, (i + 1) as f64 * t.along_cell);
            found = Some(found.map_or((a, b), |(lo, hi)| (lo.min(a), hi.max(b))));
        }
    }
    found
}

/// In red, when the model is in the chuck or tailstock, or so near them that the collet nut
/// and spindle can't get down beside it: how much room to leave.
pub fn too_near(
    work: &Work,
    head: &Head,
    around: &Surroundings,
    length: f64,
    joined: Joined,
) -> Vec<String> {
    let Some((first, last)) = model_along(work) else {
        return vec![];
    };
    let units = work.units;
    let room = head.widest() + 2.0;
    let (free_from, free_to) = head.free();
    let mut notes = vec![];
    // an end the part stays joined at is not cut beside the model: no room needed there
    let (at_chuck, at_tailstock) = joined.ends();
    if around.chuck > 0.0 && !at_chuck && first < free_from + room {
        notes.push(format!(
            "The model starts {} from the job's start, but the chuck holds the first {} \
             (Machine Configuration's rotary settings), and the collet nut and spindle, {} \
             across, can't get down beside it. Leave at least {} between the chuck and the \
             model: move the model along, or make the material longer.",
            units.length(first, 1),
            units.length(around.chuck, 1),
            units.length(2.0 * head.widest(), 0),
            units.length(room, 0),
        ));
    }
    if around.tailstock > 0.0 && !at_tailstock && last > free_to - room {
        notes.push(format!(
            "The model ends {} from the job's end, but the tailstock takes the last {}, and \
             the collet nut and spindle can't get down beside it. Leave at least {} between \
             the model and the tailstock.",
            units.length((length - last).max(0.0), 1),
            units.length(around.tailstock, 1),
            units.length(room, 0),
        ));
    }
    notes
}

/// In red, when the holder kept the bit from some of the model at every angle tried: where
/// along the part, and what would reach it.
pub fn left_note(
    outcome: &Outcome,
    tool: &Tool,
    around: &Surroundings,
    units: cut_craft_core::document::Units,
) -> Option<String> {
    if outcome.left.is_empty() {
        return None;
    }
    let (from, to) = outcome
        .left
        .iter()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), p| {
            (lo.min(p.along), hi.max(p.along))
        });
    let tilts = if outcome.tilts.is_empty() {
        String::new()
    } else {
        let list: Vec<String> = outcome.tilts.iter().map(|a| format!("{a:.0}°")).collect();
        format!(" (A locked at {} for the details)", list.join(", "))
    };
    // the reach was taken, not given: the likeliest thing to put right
    let (stickout, guessed) = cut_craft_core::collision::stickout(tool);
    let guess = if guessed {
        format!(
            " The Tool Database doesn't say how far this bit sticks out of the collet, so {} \
             was taken: set its stick-out there if it sticks out further.",
            units.length(stickout, 0)
        )
    } else {
        String::new()
    };
    Some(format!(
        "The collet nut ({} across) and spindle can't get down to some of the model at any \
         angle tried{tilts}: it is left rough between {} and {} along the part. A bit that \
         sticks out further, a slimmer holder, or the model sliced into parts would reach it.\
         {guess}",
        units.length(around.holder.nut_diameter, 0),
        units.length(from, 1),
        units.length(to, 1),
    ))
}
