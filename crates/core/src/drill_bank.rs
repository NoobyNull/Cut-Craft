//! Drill banks: a head of several drills on a horizontal (X) and a vertical (Y) arm, an even
//! pitch apart, each dropped (extended) or kept up (retracted) on its own, so one plunge
//! drills every hole of a row that lines up with the drills. Shelf-pin and dowel rows on
//! the 32 mm system are what they are for (`re/SPEC.md`, Drill Bank Toolpath).
//!
//! The arms cross at the central drill, which is on both: it has a number on each arm, for
//! controllers that number the arms apart. The reference drill is where the head is: the
//! G-code's X and Y are its position.

use crate::geometry::Point;
use serde::{Deserialize, Serialize};

/// Which arm a drill is on.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Arm {
    /// Along X
    #[default]
    Horizontal,
    /// Along Y
    Vertical,
}

/// One drill of a bank: its arm and place along it, counted from 0 (left, or front).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DrillId {
    pub arm: Arm,
    pub index: usize,
}

/// A drill's own settings.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct BankDrill {
    /// The number the controller selects it by (often a bit of a bitmask: 1, 2, 4, 8...)
    pub number: u32,
    /// Its diameter, mm: the holes it drills
    pub diameter: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct DrillBank {
    /// The horizontal arm's drills, left to right (+X).
    pub horizontal: Vec<BankDrill>,
    /// The vertical arm's drills, front to back (+Y); empty for a bank of one row.
    pub vertical: Vec<BankDrill>,
    /// Where the arms cross: the central drill's place on the horizontal arm, then on the
    /// vertical one.
    pub central: [usize; 2],
    /// The distance between neighbouring drills, mm.
    pub pitch: f64,
    /// The drill the head is positioned by: the G-code's X and Y are where it is.
    pub reference: DrillId,
}

impl Default for DrillBank {
    /// No drills: a machine without a bank.
    fn default() -> Self {
        Self {
            horizontal: vec![],
            vertical: vec![],
            central: [0, 0],
            pitch: 32.0,
            reference: DrillId::default(),
        }
    }
}

/// A hole to drill: its centre and diameter.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hole {
    pub at: Point,
    pub diameter: f64,
}

/// How near a hole must be to a drill to be drilled by it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tolerances {
    /// How far a hole's centre may be from a drill's, mm.
    pub pitch: f64,
    /// How much its diameter may differ from the drill's, mm.
    pub diameter: f64,
}

/// One plunge of the head: where its reference drill is, the drills dropped, and the hole
/// each drills (`holes[k]` by `drills[k]`, indices into the holes planned).
#[derive(Clone, Debug, PartialEq)]
pub struct Hit {
    pub head: Point,
    pub drills: Vec<DrillId>,
    pub holes: Vec<usize>,
}

/// What a drill bank toolpath keeps for the post: the bank, and each plunge of the head.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct BankToolpath {
    pub bank: DrillBank,
    pub hits: Vec<BankHit>,
}

/// One plunge of the head in a toolpath: its moves (`start..end`), which drill each of its
/// holes one after another (for the preview and simulation), where the reference drill is,
/// and the drills dropped.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct BankHit {
    pub start: usize,
    pub end: usize,
    pub head: Point,
    pub drills: Vec<DrillId>,
}

impl BankToolpath {
    /// The plunges of copies of the toolpath laid end to end: for each copy, how far it is
    /// moved and where its moves start among the copies' moves.
    #[must_use]
    pub fn copied(&self, copies: &[(Point, usize)]) -> Self {
        let hits = copies
            .iter()
            .flat_map(|&(by, from)| {
                self.hits.iter().map(move |hit| BankHit {
                    start: hit.start + from,
                    end: hit.end + from,
                    head: hit.head + by,
                    drills: hit.drills.clone(),
                })
            })
            .collect();
        Self {
            bank: self.bank.clone(),
            hits,
        }
    }
}

/// The numbers a post prints for the drills dropped: `[DB]`, `[DBX]`, `[DBY]`, and the
/// list `[DL]` (one number a drill).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Selection {
    pub all: u32,
    pub horizontal: u32,
    pub vertical: u32,
    pub list: Vec<u32>,
}

impl DrillBank {
    /// A bank of `horizontal` drills along X and `vertical` along Y crossing at their middles,
    /// all `diameter` across, numbered 1, 2, 4, 8... (a bitmask) along X then Y. The
    /// central drill is the reference.
    pub fn new(horizontal: usize, vertical: usize, pitch: f64, diameter: f64) -> Self {
        let central = [
            horizontal.saturating_sub(1) / 2,
            vertical.saturating_sub(1) / 2,
        ];
        let mut bit = 0u32;
        let mut next = || {
            let n = 1u32.checked_shl(bit).unwrap_or(0);
            bit += 1;
            n
        };
        let horizontal: Vec<BankDrill> = (0..horizontal)
            .map(|_| BankDrill {
                number: next(),
                diameter,
            })
            .collect();
        let vertical = (0..vertical)
            .map(|j| BankDrill {
                // the crossing drill's vertical number is its own bit too
                number: next(),
                diameter: if j == central[1] {
                    horizontal.get(central[0]).map_or(diameter, |d| d.diameter)
                } else {
                    diameter
                },
            })
            .collect();
        let reference = if horizontal.is_empty() {
            DrillId {
                arm: Arm::Vertical,
                index: central[1],
            }
        } else {
            DrillId {
                arm: Arm::Horizontal,
                index: central[0],
            }
        };
        Self {
            horizontal,
            vertical,
            central,
            pitch,
            reference,
        }
    }

    /// Gives the arms `horizontal` and `vertical` drills, keeping the drills that remain.
    /// New drills take the last drill's diameter and the next bits of the bitmask; the
    /// central and reference drills stay on the bank.
    pub fn set_counts(&mut self, horizontal: usize, vertical: usize) {
        let diameter = self.drills().last().map_or(5.0, |(_, d)| d.diameter);
        let mut highest = self
            .horizontal
            .iter()
            .chain(&self.vertical)
            .map(|d| d.number)
            .max()
            .unwrap_or(0);
        let mut next = || {
            highest = highest
                .saturating_add(1)
                .checked_next_power_of_two()
                .unwrap_or(0);
            BankDrill {
                number: highest,
                diameter,
            }
        };
        for (arm, count) in [
            (&mut self.horizontal, horizontal),
            (&mut self.vertical, vertical),
        ] {
            arm.truncate(count);
            while arm.len() < count {
                arm.push(next());
            }
        }
        self.central = [
            self.central[0].min(horizontal.saturating_sub(1)),
            self.central[1].min(vertical.saturating_sub(1)),
        ];
        let len = match self.reference.arm {
            Arm::Horizontal => horizontal,
            Arm::Vertical => vertical,
        };
        if self.reference.index >= len {
            self.reference = if horizontal > 0 {
                DrillId {
                    arm: Arm::Horizontal,
                    index: self.central[0],
                }
            } else {
                DrillId {
                    arm: Arm::Vertical,
                    index: self.central[1],
                }
            };
        }
    }

    /// The drill as it is known, and whether it is the crossing drill.
    pub fn is_crossing(&self, id: DrillId) -> bool {
        self.crosses()
            && self.canonical(id)
                == DrillId {
                    arm: Arm::Horizontal,
                    index: self.central[0],
                }
    }

    /// Where `id` is from the crossing of the arms, mm (for drawing the bank).
    pub fn position(&self, id: DrillId) -> Point {
        self.place(id)
    }

    /// Whether `a` and `b` are the same drill (the crossing drill has a place on each arm).
    pub fn same(&self, a: DrillId, b: DrillId) -> bool {
        self.canonical(a) == self.canonical(b)
    }

    /// Whether the arms cross: both have drills.
    fn crosses(&self) -> bool {
        !self.horizontal.is_empty() && !self.vertical.is_empty()
    }

    /// The drill as it is known: the crossing drill by its place on the horizontal arm.
    fn canonical(&self, id: DrillId) -> DrillId {
        if self.crosses() && id.arm == Arm::Vertical && id.index == self.central[1] {
            DrillId {
                arm: Arm::Horizontal,
                index: self.central[0],
            }
        } else {
            id
        }
    }

    /// Each drill once (the crossing drill by its horizontal place), with its settings.
    pub fn drills(&self) -> Vec<(DrillId, BankDrill)> {
        let h = self.horizontal.iter().enumerate().map(|(index, d)| {
            let id = DrillId {
                arm: Arm::Horizontal,
                index,
            };
            (id, *d)
        });
        let v = self
            .vertical
            .iter()
            .enumerate()
            .filter(|&(j, _)| !self.crosses() || j != self.central[1])
            .map(|(index, d)| {
                let id = DrillId {
                    arm: Arm::Vertical,
                    index,
                };
                (id, *d)
            });
        h.chain(v).collect()
    }

    /// Where `id` is from the crossing, mm.
    fn place(&self, id: DrillId) -> Point {
        let along = |index: usize, centre: usize| {
            // places are small counts: exact in f64
            #[expect(clippy::cast_precision_loss, reason = "a drill's place on its arm")]
            let steps = index as f64 - centre as f64;
            steps * self.pitch
        };
        match self.canonical(id) {
            DrillId {
                arm: Arm::Horizontal,
                index,
            } => Point::new(along(index, self.central[0]), 0.0),
            DrillId {
                arm: Arm::Vertical,
                index,
            } => Point::new(0.0, along(index, self.central[1])),
        }
    }

    /// Where `id` is from the reference drill, mm: add it to the head's position to get the
    /// drill's.
    pub fn offset(&self, id: DrillId) -> Point {
        self.place(id) - self.place(self.reference)
    }

    /// What is wrong with the bank, said so it can be put right; `None` when it can drill.
    pub fn problem(&self) -> Option<String> {
        if self.horizontal.is_empty() && self.vertical.is_empty() {
            return Some(
                "the machine has no drill bank: set one up in Machine Configuration".into(),
            );
        }
        if self.pitch.is_nan() || self.pitch <= 0.0 {
            return Some("the drill bank's pitch (distance between drills) must be above 0".into());
        }
        if self.crosses()
            && (self.central[0] >= self.horizontal.len() || self.central[1] >= self.vertical.len())
        {
            return Some("the drill bank's central drill is not on both arms".into());
        }
        let arm = match self.reference.arm {
            Arm::Horizontal => &self.horizontal,
            Arm::Vertical => &self.vertical,
        };
        if self.reference.index >= arm.len() {
            return Some("the drill bank's reference drill is not one of its drills".into());
        }
        if self
            .drills()
            .iter()
            .any(|(_, d)| d.diameter.is_nan() || d.diameter <= 0.0)
        {
            return Some("every drill in the bank needs a diameter above 0".into());
        }
        None
    }

    /// The numbers to print for `drills` dropped. The crossing drill adds its number on each
    /// arm to that arm's (`[DBX]`, `[DBY]`), and its horizontal one to the list.
    pub fn selection(&self, drills: &[DrillId]) -> Selection {
        let mut s = Selection::default();
        for &id in drills {
            let id = self.canonical(id);
            let (number, arm) = match id.arm {
                Arm::Horizontal => (self.horizontal[id.index].number, &mut s.horizontal),
                Arm::Vertical => (self.vertical[id.index].number, &mut s.vertical),
            };
            *arm |= number;
            if self.crosses() && id.arm == Arm::Horizontal && id.index == self.central[0] {
                s.vertical |= self.vertical[self.central[1]].number;
            }
            s.list.push(number);
        }
        s.list.sort_unstable();
        s.list.dedup();
        s.all = s.horizontal | s.vertical;
        s
    }

    /// The plunges that drill `holes`, fewest first-come: from the first hole not yet
    /// drilled, the head goes where one of its drills fits that hole and drops as many others
    /// onto holes as it can there. An error names the holes no drill fits.
    pub fn plan(&self, holes: &[Hole], tol: Tolerances) -> Result<Vec<Hit>, String> {
        if let Some(problem) = self.problem() {
            return Err(problem);
        }
        if tol.pitch.is_nan() || tol.pitch < 0.0 || tol.pitch >= self.pitch / 2.0 {
            return Err(format!(
                "the pitch tolerance must be at least 0 and under half the pitch ({} mm)",
                self.pitch / 2.0
            ));
        }
        let drills = self.drills();
        let fits = |d: &BankDrill, h: &Hole| (d.diameter - h.diameter).abs() <= tol.diameter + 1e-9;
        let unfit: Vec<&Hole> = holes
            .iter()
            .filter(|h| !drills.iter().any(|(_, d)| fits(d, h)))
            .collect();
        if !unfit.is_empty() {
            let mut sizes: Vec<String> = unfit
                .iter()
                .map(|h| format!("{} mm", round(h.diameter)))
                .collect();
            sizes.dedup();
            let mut have: Vec<String> = drills
                .iter()
                .map(|(_, d)| format!("{} mm", round(d.diameter)))
                .collect();
            have.sort();
            have.dedup();
            let n = unfit.len();
            return Err(format!(
                "{n} hole{} ({}) match no drill in the bank ({}): draw them the size of a \
                 drill, or widen the diameter tolerance",
                if n == 1 { "" } else { "s" },
                sizes.join(", "),
                have.join(", ")
            ));
        }
        let mut done = vec![false; holes.len()];
        let mut hits = vec![];
        while let Some(first) = done.iter().position(|d| !d) {
            let mut best: Option<Hit> = None;
            for (id, _) in drills.iter().filter(|(_, d)| fits(d, &holes[first])) {
                let head = holes[first].at - self.offset(*id);
                let mut hit = Hit {
                    head,
                    drills: vec![*id],
                    holes: vec![first],
                };
                for (other, d) in &drills {
                    if other == id {
                        continue;
                    }
                    let spot = head + self.offset(*other);
                    let found = (0..holes.len()).find(|&k| {
                        !done[k]
                            && !hit.holes.contains(&k)
                            && holes[k].at.distance(spot) <= tol.pitch + 1e-9
                            && fits(d, &holes[k])
                    });
                    if let Some(k) = found {
                        hit.drills.push(*other);
                        hit.holes.push(k);
                    }
                }
                if best
                    .as_ref()
                    .is_none_or(|b| hit.holes.len() > b.holes.len())
                {
                    best = Some(hit);
                }
            }
            let hit = best.expect("the first hole fits a drill");
            for &k in &hit.holes {
                done[k] = true;
            }
            hits.push(hit);
        }
        Ok(hits)
    }
}

/// `value` to two places at most, for messages.
fn round(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn h(index: usize) -> DrillId {
        DrillId {
            arm: Arm::Horizontal,
            index,
        }
    }

    fn v(index: usize) -> DrillId {
        DrillId {
            arm: Arm::Vertical,
            index,
        }
    }

    const TOL: Tolerances = Tolerances {
        pitch: 0.1,
        diameter: 0.1,
    };

    fn hole(x: f64, y: f64) -> Hole {
        Hole {
            at: Point::new(x, y),
            diameter: 5.0,
        }
    }

    #[test]
    fn a_new_bank_crosses_at_its_middle_numbered_as_a_bitmask() {
        let bank = DrillBank::new(5, 3, 32.0, 5.0);
        assert_eq!(bank.central, [2, 1]);
        assert_eq!(bank.reference, h(2));
        let numbers: Vec<u32> = bank.horizontal.iter().map(|d| d.number).collect();
        assert_eq!(numbers, [1, 2, 4, 8, 16]);
        assert_eq!(bank.vertical[1].number, 64, "the crossing drill's Y number");
        // 5 + 3 drills sharing one
        assert_eq!(bank.drills().len(), 7);
        assert_eq!(bank.offset(h(0)), Point::new(-64.0, 0.0));
        assert_eq!(bank.offset(v(2)), Point::new(0.0, 32.0));
        assert_eq!(
            bank.offset(v(1)),
            Point::ZERO,
            "the crossing drill is the reference"
        );
        assert_eq!(bank.problem(), None);
    }

    #[test]
    fn offsets_are_from_the_reference_drill() {
        let mut bank = DrillBank::new(5, 0, 32.0, 5.0);
        bank.reference = h(0);
        assert_eq!(bank.offset(h(4)), Point::new(128.0, 0.0));
        assert_eq!(bank.offset(h(0)), Point::ZERO);
    }

    #[test]
    fn a_row_of_holes_is_drilled_in_as_few_plunges_as_the_bank_allows() {
        let bank = DrillBank::new(5, 0, 32.0, 5.0);
        // seven shelf-pin holes 32 mm apart: five in one plunge, the last two in another
        let holes: Vec<Hole> = (0..7)
            .map(|i| hole(50.0 + 32.0 * f64::from(i), 40.0))
            .collect();
        let hits = bank.plan(&holes, TOL).unwrap();
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].holes.len(), 5);
        assert_eq!(hits[1].holes.len(), 2);
        for hit in &hits {
            for (drill, &k) in hit.drills.iter().zip(&hit.holes) {
                assert!(
                    (hit.head + bank.offset(*drill)).distance(holes[k].at) < 1e-9,
                    "each drill is over its hole"
                );
            }
        }
        let mut all: Vec<usize> = hits.iter().flat_map(|h| h.holes.clone()).collect();
        all.sort_unstable();
        assert_eq!(all, (0..7).collect::<Vec<_>>(), "every hole once");
    }

    #[test]
    fn holes_off_the_pitch_or_another_size_are_drilled_alone() {
        let mut bank = DrillBank::new(3, 0, 32.0, 5.0);
        bank.horizontal[2].diameter = 8.0;
        let holes = [
            hole(0.0, 0.0),
            hole(32.05, 0.0),
            // 8 mm wide 64 mm along: the 8 mm drill
            Hole {
                at: Point::new(64.0, 0.0),
                diameter: 8.0,
            },
            // off the pitch
            hole(200.0, 0.0),
            hole(216.0, 0.0),
        ];
        let hits = bank.plan(&holes, TOL).unwrap();
        assert_eq!(hits[0].holes, [0, 1, 2]);
        assert_eq!(hits[0].drills, [h(0), h(1), h(2)]);
        assert_eq!(hits.len(), 3, "the two 16 mm apart take a plunge each");
    }

    #[test]
    fn both_arms_drill_at_once() {
        let bank = DrillBank::new(3, 3, 32.0, 5.0);
        // an L of holes: along X and up Y from the corner
        let holes = [hole(0.0, 0.0), hole(32.0, 0.0), hole(0.0, 32.0)];
        let hits = bank.plan(&holes, TOL).unwrap();
        assert_eq!(hits.len(), 1, "{hits:?}");
        let s = bank.selection(&hits[0].drills);
        // the crossing drill (2 on X, 16 on Y), the right one (4) and the back one (32)
        assert_eq!((s.horizontal, s.vertical), (2 | 4, 16 | 32));
        assert_eq!(s.all, 2 | 4 | 16 | 32);
        assert_eq!(s.list, [2, 4, 32]);
    }

    #[test]
    fn holes_no_drill_fits_are_named_with_the_drills_there_are() {
        let bank = DrillBank::new(3, 0, 32.0, 5.0);
        let holes = [
            hole(0.0, 0.0),
            Hole {
                at: Point::new(9.0, 0.0),
                diameter: 6.0,
            },
        ];
        let err = bank.plan(&holes, TOL).unwrap_err();
        assert_eq!(
            err,
            "1 hole (6 mm) match no drill in the bank (5 mm): draw them the size of a drill, \
             or widen the diameter tolerance"
        );
        let wide = Tolerances {
            diameter: 1.0,
            ..TOL
        };
        assert_eq!(bank.plan(&holes, wide).unwrap().len(), 2);
    }

    #[test]
    fn a_bank_says_what_is_wrong_with_it() {
        let none = DrillBank::default();
        assert!(none.problem().unwrap().contains("Machine Configuration"));
        assert!(none.plan(&[hole(0.0, 0.0)], TOL).is_err());
        let mut flat = DrillBank::new(3, 0, 0.0, 5.0);
        assert!(flat.problem().unwrap().contains("pitch"));
        flat.pitch = 32.0;
        flat.reference = h(7);
        assert!(flat.problem().unwrap().contains("reference"));
        let bank = DrillBank::new(3, 0, 32.0, 5.0);
        let loose = Tolerances { pitch: 16.0, ..TOL };
        assert!(
            bank.plan(&[hole(0.0, 0.0)], loose)
                .unwrap_err()
                .contains("half the pitch")
        );
    }

    #[test]
    fn resizing_keeps_the_drills_and_numbers_new_ones_on() {
        let mut bank = DrillBank::new(3, 0, 32.0, 5.0);
        bank.horizontal[2].diameter = 8.0;
        bank.set_counts(5, 2);
        let numbers: Vec<u32> = bank.drills().iter().map(|(_, d)| d.number).collect();
        // the arms cross at the second drill along X and the first along Y, whose vertical
        // number (32) is not a drill of its own
        assert_eq!(numbers, [1, 2, 4, 8, 16, 64]);
        assert_eq!(bank.horizontal[4].diameter, 8.0, "the last drill's size");
        assert_eq!(bank.problem(), None);
        bank.reference = h(4);
        bank.set_counts(2, 2);
        assert_eq!(bank.central, [1, 0]);
        assert_eq!(
            bank.reference,
            h(1),
            "a reference taken away moves to the centre"
        );
        assert!(bank.is_crossing(v(0)) && bank.same(v(0), h(1)));
        bank.set_counts(0, 2);
        assert_eq!(bank.reference, v(0));
        assert_eq!(bank.problem(), None);
    }

    #[test]
    fn copies_of_a_bank_toolpath_move_their_heads_and_moves_along() {
        let one = BankToolpath {
            bank: DrillBank::new(3, 0, 32.0, 5.0),
            hits: vec![BankHit {
                start: 0,
                end: 9,
                head: Point::new(10.0, 10.0),
                drills: vec![h(0), h(1), h(2)],
            }],
        };
        let two = one.copied(&[(Point::ZERO, 0), (Point::new(100.0, 0.0), 9)]);
        assert_eq!(two.hits.len(), 2);
        assert_eq!((two.hits[1].start, two.hits[1].end), (9, 18));
        assert_eq!(two.hits[1].head, Point::new(110.0, 10.0));
    }

    #[test]
    fn a_bank_of_one_column_drills_along_y() {
        let bank = DrillBank::new(0, 3, 32.0, 5.0);
        assert_eq!(bank.reference, v(1), "the reference is on the arm there is");
        assert_eq!(bank.problem(), None);
        assert_eq!(bank.offset(v(2)), Point::new(0.0, 32.0));
        let holes = [hole(0.0, 0.0), hole(0.0, 32.0), hole(0.0, 64.0)];
        assert_eq!(bank.plan(&holes, TOL).unwrap().len(), 1);
    }
}
