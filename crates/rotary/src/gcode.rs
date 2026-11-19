//! A rotary file read back: every straight move it makes, as the bit's [`Pose`], so the
//! simulator cuts exactly what the machine will.

use crate::frame::Pose;

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
}

/// One straight move of the file, every axis moving in step.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Motion {
    /// At the rapid rate (G0) rather than cutting.
    pub rapid: bool,
    /// Where the move ends.
    pub to: Pose,
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
    let mut rapid = true;
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
        let mut motion = false;
        for (letter, value) in split(&words) {
            match letter {
                'G' => match value.round() as i64 {
                    0 => (rapid, motion) = (true, true),
                    1 => (rapid, motion) = (false, true),
                    20 => unit = 25.4,
                    21 => unit = 1.0,
                    _ => {}
                },
                l if l == reading.along => (along, motion) = (value * unit, true),
                l if l == reading.across => {
                    across = (value * unit - reading.centre_across) * reading.across_sign;
                    motion = true;
                }
                l if l == reading.turn => (turn, motion) = (value.to_radians(), true),
                'Z' => (height, motion) = (Some(value * unit + reading.zero_height), true),
                _ => {}
            }
        }
        if let (true, Some(h)) = (motion, height) {
            moves.push(Motion {
                rapid,
                to: Pose::new(along, across, h, turn),
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
                to: Pose::new(1.0, -3.0, 25.0, 0.0)
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
}
