//! A rotary file read back: every straight move it makes, as the bit's [`Pose`], so the
//! simulator cuts exactly what the machine will.

use crate::frame::Pose;
use cut_craft_core::document::{Along, Job};
use cut_craft_core::glam::DVec3;
use cut_craft_core::rotary_axis::RotaryAxis;

/// How a file's numbers are read on the machine.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Reading {
    /// The letter of the axis along the turning axis (`X` for a rotary along X).
    pub along: char,
    /// The letter of the machine's axis across the turning axis (`Y` for a rotary along X).
    pub across: char,
    /// The letter A is written as.
    pub turn: char,
    /// Where the turning axis is on the across axis, mm (Machine Configuration's centre line).
    pub centre_across: f64,
    /// +1, or -1 when +A turns the other way from the part's frame (anticlockwise), which
    /// mirrors the across axis to match.
    pub across_sign: f64,
    /// How far above the turning axis Z0 is, mm: 0 with Z zeroed on the axis, the
    /// material's radius with it zeroed on top.
    pub zero_height: f64,
    /// Where the material starts along the axis in the file's numbers (the chuck end), mm:
    /// along is measured from there.
    pub along_start: f64,
}

impl Reading {
    /// How the file a rotary `job` is saved as reads on a machine whose rotary is `axis`:
    /// the axis along the cylinder and the one across it, A, the turning axis's centre
    /// line, and where Z0 and the material's start are. `None` for a flat job.
    #[must_use]
    pub fn of(job: &Job, axis: &RotaryAxis) -> Option<Self> {
        let rotary = job.rotary?;
        let start = job.area().min;
        let (along, across, along_start) = match rotary.along {
            Along::X => ('X', 'Y', start.x),
            Along::Y => ('Y', 'X', start.y),
        };
        Some(Self {
            along,
            across,
            turn: 'A',
            centre_across: axis.centre_across,
            across_sign: axis.across_sign(rotary.along),
            zero_height: rotary.diameter / 2.0 - job.machine_z(0.0),
            along_start,
        })
    }
}

/// One straight move of the file, every axis moving in step.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Motion {
    /// At the rapid rate (G0) rather than cutting.
    pub rapid: bool,
    /// Where the move ends.
    pub to: Pose,
    /// How long a cutting move takes as the file sets its feed, minutes: in inverse time
    /// (G93) one over its F, else its length over F (in degrees when only A moves). `None`
    /// for a rapid, which goes as fast as the machine goes, and a move with no feed set.
    pub minutes: Option<f64>,
}

/// The straight moves (G0, G1) of `text`, read as `reading` says, from the first that sets
/// the bit's height: until then where the bit is is not known. Lengths in inches (G20) are
/// read in millimetres; A in degrees. Comments, `( … )` and after `;`, are skipped.
#[must_use]
#[expect(
    clippy::cast_possible_truncation,
    reason = "G numbers are small whole numbers"
)]
pub fn read(text: &str, reading: &Reading) -> Vec<Motion> {
    let mut unit = 1.0;
    let (mut rapid, mut inverse) = (true, false);
    let mut feed: Option<f64> = None;
    let (mut along, mut across, mut turn) = (0.0, 0.0, 0.0);
    let mut height: Option<f64> = None;
    let mut moves = vec![];
    for line in text.lines() {
        let mut words = String::new();
        let mut depth = 0;
        for c in line.split(';').next().unwrap_or("").chars() {
            match c {
                '(' => depth += 1,
                ')' => depth = (depth - 1).max(0),
                c if depth == 0 => words.push(c),
                _ => {}
            }
        }
        let was = (along, across, height, turn);
        let mut motion = false;
        // in inverse time, F counts only on its own line
        let mut f_here = None;
        for (letter, value) in split(&words) {
            match letter {
                'G' => match value.round() as i64 {
                    0 => (rapid, motion) = (true, true),
                    1 => (rapid, motion) = (false, true),
                    20 => unit = 25.4,
                    21 => unit = 1.0,
                    93 => inverse = true,
                    94 => inverse = false,
                    _ => {}
                },
                'F' => f_here = Some(value),
                l if l == reading.along => {
                    along = value * unit - reading.along_start;
                    motion = true;
                }
                l if l == reading.across => {
                    across = (value * unit - reading.centre_across) * reading.across_sign;
                    motion = true;
                }
                l if l == reading.turn => (turn, motion) = (value.to_radians(), true),
                'Z' => (height, motion) = (Some(value * unit + reading.zero_height), true),
                _ => {}
            }
        }
        if let Some(f) = f_here {
            feed = Some(if inverse { f } else { f * unit });
        }
        if let (true, Some(h)) = (motion, height) {
            let minutes = if rapid {
                None
            } else if inverse {
                f_here.filter(|f| *f > 0.0).map(|f| 1.0 / f)
            } else {
                let length =
                    DVec3::new(along - was.0, across - was.1, h - was.2.unwrap_or(h)).length();
                let length = if length > 0.0 {
                    length
                } else {
                    (turn - was.3).abs().to_degrees()
                };
                feed.filter(|f| *f > 0.0).map(|f| length / f)
            };
            moves.push(Motion {
                rapid,
                to: Pose::new(along, across, h, turn),
                minutes,
            });
        }
    }
    moves
}

/// A line's words: each letter with the number after it.
fn split(words: &str) -> Vec<(char, f64)> {
    let mut out = vec![];
    let mut chars = words.char_indices().peekable();
    while let Some((i, c)) = chars.next() {
        if !c.is_ascii_alphabetic() {
            continue;
        }
        let start = i + c.len_utf8();
        let mut end = start;
        while let Some(&(j, d)) = chars.peek() {
            if d.is_ascii_digit() || d == '.' || d == '-' || d == '+' || d == ' ' {
                end = j + d.len_utf8();
                chars.next();
            } else {
                break;
            }
        }
        if let Ok(v) = words[start..end].trim().parse() {
            out.push((c.to_ascii_uppercase(), v));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALONG_X: Reading = Reading {
        along: 'X',
        across: 'Y',
        turn: 'A',
        centre_across: 5.0,
        across_sign: 1.0,
        zero_height: 20.0,
        along_start: 0.0,
    };

    #[test]
    fn a_file_is_read_as_the_bit_moves_from_its_first_height() {
        let text =
            "(header Z9 X9)\nG21 G90\nG0 X1 Y2\nG0 Z5.0\nG1 X10.5 A90 Z-2 ; cut\nY7\nG20\nG0 Z1\n";
        let moves = read(text, &ALONG_X);
        assert_eq!(moves.len(), 4, "{moves:?}");
        assert_eq!(
            moves[0],
            Motion {
                rapid: true,
                to: Pose::new(1.0, -3.0, 25.0, 0.0),
                minutes: None,
            }
        );
        let cut = moves[1];
        assert!(!cut.rapid && (cut.to.turn - std::f64::consts::FRAC_PI_2).abs() < 1e-12);
        assert!((cut.to.height - 18.0).abs() < 1e-12 && (cut.to.along - 10.5).abs() < 1e-12);
        // modal: still cutting, across moved to 7, 2 past the axis
        assert!(!moves[2].rapid && (moves[2].to.across - 2.0).abs() < 1e-12);
        // inches
        assert!((moves[3].to.height - 45.4).abs() < 1e-9);
        // the other way round mirrors across
        let mirrored = read(
            "G0 Z0 Y7",
            &Reading {
                across_sign: -1.0,
                ..ALONG_X
            },
        );
        assert!((mirrored[0].to.across + 2.0).abs() < 1e-12);
    }

    #[test]
    fn words_written_without_spaces_are_read_too() {
        assert_eq!(
            split("G1X1.5Y-2A+3"),
            vec![('G', 1.0), ('X', 1.5), ('Y', -2.0), ('A', 3.0)]
        );
    }

    #[test]
    fn a_cutting_move_takes_as_long_as_its_feed_says() {
        let text =
            "G0 X0 Y5 Z0 A0\nG94 G1 X30 Y5 Z-40 F500\nG1 A90\nG93 G1 X40 F4\nX50 F0.5\nG0 Z10\n";
        let moves = read(text, &ALONG_X);
        let minutes: Vec<Option<f64>> = moves.iter().map(|m| m.minutes).collect();
        // 30 along and 40 down is 50 mm at 500 mm/min; 90° of A alone at 500°/min; in
        // inverse time a quarter and two minutes; a rapid as fast as the machine goes
        let want = [None, Some(0.1), Some(0.18), Some(0.25), Some(2.0), None];
        assert_eq!(minutes.len(), want.len());
        for (got, want) in minutes.iter().zip(want) {
            match (got, want) {
                (Some(g), Some(w)) => assert!((g - w).abs() < 1e-12, "{minutes:?}"),
                (g, w) => assert_eq!(*g, w, "{minutes:?}"),
            }
        }
        // feed in inches a minute is read in millimetres
        let inch = read("G20 G0 X0 Y0 Z0\nG94 G1 X1 F10", &ALONG_X);
        assert!((inch[1].minutes.unwrap() - 0.1).abs() < 1e-12);
    }

    #[test]
    fn a_job_reads_its_file_from_its_own_start_and_its_axis_letters() {
        use cut_craft_core::document::{Along, Datum, Job, ZZero};
        let axis = RotaryAxis {
            centre_across: 12.0,
            ..RotaryAxis::default()
        };
        let mut job = Job::rotary(100.0, 30.0, Along::X);
        job.datum = Datum::Centre;
        let r = Reading::of(&job, &axis).unwrap();
        assert_eq!((r.along, r.across, r.turn), ('X', 'Y', 'A'));
        assert!((r.along_start + 50.0).abs() < 1e-9, "{r:?}");
        // Z zeroed on the surface: Z0 is the radius up
        job.z_zero = ZZero::MaterialSurface;
        assert!((Reading::of(&job, &axis).unwrap().zero_height - 15.0).abs() < 1e-9);
        // zeroed on the axis
        job.z_zero = ZZero::MachineBed;
        assert!(Reading::of(&job, &axis).unwrap().zero_height.abs() < 1e-9);
        let moves = read("G0 X-50 Y12 Z3", &Reading::of(&job, &axis).unwrap());
        assert_eq!(moves[0].to, Pose::new(0.0, 0.0, 3.0, 0.0));
        let along_y = Reading::of(&Job::rotary(100.0, 30.0, Along::Y), &axis).unwrap();
        assert_eq!((along_y.along, along_y.across), ('Y', 'X'));
        assert_eq!(Reading::of(&Job::default(), &axis), None);
    }
}
