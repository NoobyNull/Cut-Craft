//! The Sequence box: entries saying how each repeat of a pattern is drawn, in turn.
//!
//! Each entry is a sample digit: `0` leaves a space, `1` draws the repeat as designed, `2`
//! flips it across the line (top to bottom), `3` turns it back to front along the line and
//! `4` does both. `/n` after it turns the repeat by 180° ÷ n (`1/4` is 45°), and `R` before
//! it (or after it, `1/4r`) turns it the other way (or, with no turn, back to front). `S`
//! before an entry gives it a random size and `T` (or `O`) a random lean (tilt); before
//! brackets they reach every entry inside (`ST(1,2)`). Entries may be split by commas or spaces;
//! plain digits need none (`1110`). They run along the line one repeat each and start again
//! when they run out, with nothing added between.

use crate::rng::Rng;

/// The most entries a sequence keeps.
pub const MOST: usize = 64;

/// One entry: its sample, its turn's divisor (0 for none), whether it is reversed, and
/// whether its size and lean are random.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Entry {
    pub code: u8,
    pub div: u16,
    pub rev: bool,
    pub size: bool,
    pub lean: bool,
}

/// A parsed sequence.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Seq {
    entries: [Entry; MOST],
    len: u8,
}

/// How one repeat is drawn.
#[derive(Clone, Copy, Debug, PartialEq)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "one for each letter of an entry"
)]
pub struct Turn {
    /// Mirrored top to bottom, across the line.
    pub across: bool,
    /// Mirrored front to back, along the line.
    pub back: bool,
    /// Turned about its middle, radians, anticlockwise.
    pub angle: f64,
    /// A random size (`S`).
    pub size: bool,
    /// A random lean (`T`).
    pub lean: bool,
}

impl Default for Seq {
    fn default() -> Self {
        Self::EMPTY
    }
}

impl Seq {
    pub const EMPTY: Self = Self {
        entries: [Entry {
            code: 0,
            div: 0,
            rev: false,
            size: false,
            lean: false,
        }; MOST],
        len: 0,
    };

    /// The entries in `text`, in order; anything that is not an entry (commas, spaces,
    /// digits over 4) is passed over, and no more than [`MOST`] are kept.
    pub fn parse(text: &str) -> Self {
        let mut s = Self::EMPTY;
        // R, S and T written for the next entry or bracket
        let (mut rev, mut size, mut lean) = (false, false, false);
        // the S and T of each bracket open round here
        let mut groups: Vec<(bool, bool)> = vec![];
        let inherited = |groups: &[(bool, bool)]| {
            groups
                .iter()
                .fold((false, false), |(a, b), (c, d)| (a || *c, b || *d))
        };
        // whether the last entry is still being written (nothing has split it off)
        let mut open = false;
        let mut chars = text.chars().peekable();
        while let Some(c) = chars.next() {
            match c {
                // an R written after its entry (4r) is that entry's, else the next one's
                'R' | 'r' => match s.len.checked_sub(1) {
                    Some(last) if open => s.entries[usize::from(last)].rev = true,
                    _ => rev = true,
                },
                'S' | 's' => (size, open) = (true, false),
                'O' | 'o' | 'T' | 't' => (lean, open) = (true, false),
                '(' => {
                    groups.push((size, lean));
                    (size, lean, open) = (false, false, false);
                }
                ')' => {
                    groups.pop();
                    open = false;
                }
                '/' => {
                    let mut n: u32 = 0;
                    while let Some(d) = chars.peek().and_then(|c| c.to_digit(10)) {
                        n = (n * 10 + d).min(999);
                        chars.next();
                    }
                    if let (Some(last), true) = (s.len.checked_sub(1), open) {
                        s.entries[usize::from(last)].div = n as u16;
                    }
                }
                '0'..='4' if usize::from(s.len) < MOST => {
                    let (gs, gl) = inherited(&groups);
                    s.entries[usize::from(s.len)] = Entry {
                        code: c as u8 - b'0',
                        div: 0,
                        rev,
                        size: size || gs,
                        lean: lean || gl,
                    };
                    s.len += 1;
                    (rev, size, lean, open) = (false, false, false, true);
                }
                _ => open = false,
            }
        }
        s
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn len(&self) -> usize {
        usize::from(self.len)
    }

    /// The entry for repeat `k`, the sequence starting again when it runs out (a plain 1
    /// when it is empty).
    pub fn entry(&self, k: usize) -> Entry {
        if self.is_empty() {
            Entry {
                code: 1,
                ..Entry::default()
            }
        } else {
            self.entries[k % self.len()]
        }
    }

    /// The sample digit for repeat `k`.
    #[cfg(test)]
    pub fn code(&self, k: usize) -> u8 {
        self.entry(k).code
    }

    /// How repeat `k` is drawn; `None` for a space.
    pub fn turn(&self, k: usize) -> Option<Turn> {
        let e = self.entry(k);
        let mut t = turn(e.code)?;
        t.size = e.size;
        t.lean = e.lean;
        if e.div > 0 {
            let a = std::f64::consts::PI / f64::from(e.div);
            t.angle = if e.rev { a } else { -a };
        } else if e.rev {
            t.back = !t.back;
        }
        Some(t)
    }

    /// The entries, as typed back (brackets opened out): plain digits when none has a
    /// turn, an R, an S or a T, else separated by commas.
    #[cfg(test)]
    pub fn text(&self) -> String {
        let es = &self.entries[..self.len()];
        let plain = es
            .iter()
            .all(|e| e.div == 0 && !e.rev && !e.size && !e.lean);
        let words: Vec<String> = es
            .iter()
            .map(|e| {
                let mut w = String::new();
                if e.size {
                    w.push('S');
                }
                if e.lean {
                    w.push('T');
                }
                if e.rev {
                    w.push('R');
                }
                w.push(char::from(b'0' + e.code));
                if e.div > 0 {
                    w = format!("{w}/{}", e.div);
                }
                w
            })
            .collect();
        words.join(if plain { "" } else { "," })
    }
}

/// How a repeat with the sample digit `code` is drawn, unturned; `None` for a space (0).
pub fn turn(code: u8) -> Option<Turn> {
    let t = |across, back| {
        Some(Turn {
            across,
            back,
            angle: 0.0,
            size: false,
            lean: false,
        })
    };
    match code {
        0 => None,
        2 => t(true, false),
        3 => t(false, true),
        4 => t(true, true),
        _ => t(false, false),
    }
}

/// A sequence made up from `seed`: a short run of digits repeated, so it still reads as a
/// pattern, sometimes with a space.
pub fn random(seed: u32) -> String {
    let mut r = Rng::new(seed);
    let unit = 2 + (r.range(0.0, 4.0) as usize).min(3);
    let mut digits: Vec<u8> = (0..unit)
        .map(|_| 1 + (r.range(0.0, 4.0) as u8).min(3))
        .collect();
    if r.chance(0.3) {
        digits.push(0);
    }
    // never all spaces, and never one plain digit alone
    if digits.iter().all(|d| *d == digits[0]) {
        digits[0] = if digits[0] == 1 { 2 } else { 1 };
    }
    let times = (8 / digits.len()).max(1);
    digits
        .iter()
        .cycle()
        .take(digits.len() * times)
        .map(|d| char::from(b'0' + d))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_digits_0_to_4_and_passes_over_the_rest() {
        let s = Seq::parse("111 000, 2229x5");
        assert_eq!(s.text(), "111000222");
        assert_eq!(Seq::parse("").text(), "");
        assert_eq!(Seq::parse(&"1".repeat(100)).len(), MOST);
    }

    #[test]
    fn starts_again_when_it_runs_out_with_no_space_added() {
        let s = Seq::parse("1110");
        let codes: Vec<u8> = (0..9).map(|k| s.code(k)).collect();
        assert_eq!(codes, [1, 1, 1, 0, 1, 1, 1, 0, 1]);
        let s = Seq::parse("111000111000222000222");
        assert_eq!(s.code(21), 1, "straight back to the start");
        assert_eq!(s.code(20), 2);
    }

    #[test]
    fn each_digit_is_one_way_round() {
        assert_eq!(turn(0), None);
        let t = |a, b| {
            Some(Turn {
                across: a,
                back: b,
                angle: 0.0,
                size: false,
                lean: false,
            })
        };
        assert_eq!(turn(1), t(false, false));
        assert_eq!(turn(2), t(true, false));
        assert_eq!(turn(3), t(false, true));
        assert_eq!(turn(4), t(true, true));
        assert_eq!(
            Seq::EMPTY.turn(5),
            t(false, false),
            "empty draws as designed"
        );
    }

    #[test]
    fn a_slash_turns_by_180_over_it_and_r_turns_the_other_way() {
        let s = Seq::parse("1,1/4,R1/4,R1,2/12,0");
        assert_eq!(s.len(), 6);
        let deg = |k| s.turn(k).unwrap().angle.to_degrees();
        assert!((deg(0)).abs() < 1e-9);
        assert!((deg(1) + 45.0).abs() < 1e-9, "clockwise");
        assert!((deg(2) - 45.0).abs() < 1e-9, "the other way");
        assert!(s.turn(3).unwrap().back, "R alone is back to front");
        assert!((deg(4) + 15.0).abs() < 1e-9 && s.turn(4).unwrap().across);
        assert_eq!(s.turn(5), None);
        assert_eq!(s.text(), "1,1/4,R1/4,R1,2/12,0");
        assert_eq!(Seq::parse("r1/8").text(), "R1/8");
        assert_eq!(Seq::parse("1 1 0").text(), "110");
        // an r after its digit is that entry's
        assert_eq!(
            Seq::parse("1,2,3,4,4r,3r,2r,1r").text(),
            "1,2,3,4,R4,R3,R2,R1"
        );
        let s = Seq::parse("4r,3r,2r,1r");
        let codes: Vec<(bool, bool)> = (0..4)
            .map(|k| s.turn(k).map(|t| (t.across, t.back)).unwrap())
            .collect();
        assert_eq!(
            codes,
            [(true, false), (false, false), (true, true), (false, true)]
        );
        assert_eq!(Seq::parse("1/4r").text(), "R1/4");
    }

    #[test]
    fn s_and_o_make_an_entry_or_a_bracket_random_in_size_and_lean() {
        let s = Seq::parse("O(R1,1,S1)");
        assert_eq!(s.text(), "TR1,T1,ST1");
        let t = |k| s.turn(k).unwrap();
        assert!((0..3).all(|k| t(k).lean));
        assert!(!t(0).size && !t(1).size && t(2).size);
        assert!(t(0).back, "the R still turns it back to front");
        assert_eq!(Seq::parse("SO(1)").text(), "ST1");
        assert_eq!(
            Seq::parse("S(1,1,1,2,2,2,1,1,1,3,3,3)").text(),
            "S1,S1,S1,S2,S2,S2,S1,S1,S1,S3,S3,S3"
        );
        // a bracket's S stops at its end; O is another way to write T
        assert_eq!(Seq::parse("S(1,2),1,O3").text(), "S1,S2,1,T3");
        assert_eq!(Seq::parse("(S(1),2)1").text(), "S1,2,1");
    }

    #[test]
    fn a_random_sequence_is_a_repeating_pattern_of_valid_digits() {
        for seed in 0..40 {
            let text = random(seed);
            assert!(text.len() >= 4, "{text}");
            assert!(text.chars().all(|c| ('0'..='4').contains(&c)), "{text}");
            assert!(text.chars().any(|c| c != '0'), "{text}");
            assert!(
                text.chars().any(|c| c != text.chars().next().unwrap()),
                "{text}"
            );
        }
        let kinds: std::collections::HashSet<String> = (0..40).map(random).collect();
        assert!(kinds.len() > 10, "varied: {kinds:?}");
    }
}
