//! Colour mode's palette: every colour of the picture as a swatch (up to [`MOST_SWATCHES`],
//! so flat artwork shows each of its colours and a photo a fair spread of them), joined into
//! groups by how alike they look and by the swatches the user merged. Each group is traced
//! as one colour.

use std::collections::BTreeMap;

/// The most swatches a picture is shown as.
pub const MOST_SWATCHES: usize = 48;

/// Groups covering less of the picture than this share join the nearest larger group: the
/// in-between colours along anti-aliased edges, which would otherwise trace as slivers.
// ponytail: 0.2 %, small enough to keep a logo's small details as colours of their own
pub const SMALLEST_SHARE: f64 = 0.002;

/// Colours are binned five bits a channel before they are counted.
const BINS: usize = 1 << 15;

fn bin(c: [u8; 3]) -> usize {
    (usize::from(c[0] >> 3) << 10) | (usize::from(c[1] >> 3) << 5) | usize::from(c[2] >> 3)
}

/// One colour of the picture and how many of its pixels are (nearest) that colour.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Swatch {
    pub colour: [u8; 3],
    pub pixels: usize,
}

/// A picture's swatches, most used first, and which swatch each binned colour is.
#[derive(Clone, Debug)]
pub struct Swatches {
    pub list: Vec<Swatch>,
    of_bin: Vec<u8>,
}

/// A colour as `#rrggbb`.
#[must_use]
pub fn hex([r, g, b]: [u8; 3]) -> String {
    format!("#{r:02x}{g:02x}{b:02x}")
}

/// How light a colour looks, 0 to 255.
#[must_use]
pub fn lightness(c: [u8; 3]) -> f64 {
    0.299 * f64::from(c[0]) + 0.587 * f64::from(c[1]) + 0.114 * f64::from(c[2])
}

/// A colour in CIE L*a*b*, where distance follows how different colours look.
#[must_use]
#[expect(
    clippy::many_single_char_names,
    reason = "the colour space's own names"
)]
pub fn lab(c: [u8; 3]) -> [f64; 3] {
    let linear = |v: u8| {
        let v = f64::from(v) / 255.0;
        if v <= 0.040_45 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };
    let (r, g, b) = (linear(c[0]), linear(c[1]), linear(c[2]));
    // D65 white
    let x = (0.412_456_4 * r + 0.357_576_1 * g + 0.180_437_5 * b) / 0.950_47;
    let y = 0.212_672_9 * r + 0.715_152_2 * g + 0.072_175 * b;
    let z = (0.019_333_9 * r + 0.119_192 * g + 0.950_304_1 * b) / 1.088_83;
    let f = |t: f64| {
        if t > 0.008_856 {
            t.cbrt()
        } else {
            7.787 * t + 16.0 / 116.0
        }
    };
    let (fx, fy, fz) = (f(x), f(y), f(z));
    [116.0 * fy - 16.0, 500.0 * (fx - fy), 200.0 * (fy - fz)]
}

/// How different two colours look (CIE76 ΔE: about 2 is just noticeable, 100 is black from
/// white).
#[must_use]
pub fn difference(a: [u8; 3], b: [u8; 3]) -> f64 {
    let (a, b) = (lab(a), lab(b));
    (0..3).map(|i| (a[i] - b[i]).powi(2)).sum::<f64>().sqrt()
}

/// A run of binned colours: each bin, its pixels and the sum of their channels.
type Entry = (usize, usize, [u64; 3]);

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "a mean of channel values"
)]
fn mean(entries: &[Entry]) -> ([u8; 3], usize) {
    let pixels: usize = entries.iter().map(|e| e.1).sum();
    let mut sum = [0u64; 3];
    for e in entries {
        for (total, v) in sum.iter_mut().zip(e.2) {
            *total += v;
        }
    }
    let n = pixels.max(1) as f64;
    (sum.map(|s| (s as f64 / n).round() as u8), pixels)
}

impl Swatches {
    /// The swatches of `rgb` (three bytes a pixel): every colour when there are few, else
    /// [`MOST_SWATCHES`] cut by median cut, each the mean of the pixels it stands for.
    #[must_use]
    #[expect(
        clippy::cast_possible_truncation,
        reason = "fewer than MOST_SWATCHES swatches"
    )]
    pub fn of(rgb: &[u8]) -> Self {
        let mut counts = vec![(0usize, [0u64; 3]); BINS];
        for &c in rgb.as_chunks::<3>().0 {
            let slot = &mut counts[bin(c)];
            slot.0 += 1;
            for (total, v) in slot.1.iter_mut().zip(c) {
                *total += u64::from(v);
            }
        }
        let entries: Vec<Entry> = counts
            .iter()
            .enumerate()
            .filter(|(_, (n, _))| *n > 0)
            .map(|(b, &(n, sum))| (b, n, sum))
            .collect();
        let mut boxes: Vec<Vec<Entry>> = if entries.is_empty() {
            vec![]
        } else {
            vec![entries]
        };
        // median cut: split the box with the most pixels times its widest spread, at its
        // pixels' median along that channel, until there are enough
        while boxes.len() < MOST_SWATCHES {
            let spread = |b: &Vec<Entry>| {
                let channel = |i: usize| {
                    let values = b.iter().map(|e| (e.0 >> (10 - 5 * i)) & 31);
                    values.clone().max().unwrap_or(0) - values.min().unwrap_or(0)
                };
                let (widest, by) = (0..3).map(|i| (channel(i), i)).max().unwrap_or((0, 0));
                (widest, by)
            };
            let Some((at, by)) = boxes
                .iter()
                .enumerate()
                .filter(|(_, b)| b.len() > 1)
                .max_by_key(|(_, b)| spread(b).0 * b.iter().map(|e| e.1).sum::<usize>())
                .map(|(i, b)| (i, spread(b).1))
            else {
                break;
            };
            let mut b = boxes.swap_remove(at);
            b.sort_by_key(|e| (e.0 >> (10 - 5 * by)) & 31);
            let half = b.iter().map(|e| e.1).sum::<usize>() / 2;
            let mut seen = 0;
            let mut cut = b.len() - 1;
            for (i, e) in b.iter().enumerate() {
                seen += e.1;
                if seen >= half {
                    cut = i + 1;
                    break;
                }
            }
            let cut = cut.clamp(1, b.len() - 1);
            let rest = b.split_off(cut);
            boxes.push(b);
            boxes.push(rest);
        }
        let mut made: Vec<(Swatch, Vec<usize>)> = boxes
            .iter()
            .map(|b| {
                let (colour, pixels) = mean(b);
                (Swatch { colour, pixels }, b.iter().map(|e| e.0).collect())
            })
            .collect();
        made.sort_by(|a, b| {
            b.0.pixels
                .cmp(&a.0.pixels)
                .then(a.0.colour.cmp(&b.0.colour))
        });
        let mut of_bin = vec![u8::MAX; BINS];
        for (k, (_, bins)) in made.iter().enumerate() {
            for &b in bins {
                of_bin[b] = k as u8;
            }
        }
        Self {
            list: made.into_iter().map(|(s, _)| s).collect(),
            of_bin,
        }
    }

    /// Which swatch colour `c` is: its own bin's, else the nearest.
    #[must_use]
    pub fn index(&self, c: [u8; 3]) -> usize {
        match self.of_bin[bin(c)] {
            u8::MAX => (0..self.list.len())
                .min_by(|&a, &b| {
                    difference(self.list[a].colour, c)
                        .total_cmp(&difference(self.list[b].colour, c))
                })
                .unwrap_or(0),
            k => usize::from(k),
        }
    }
}

/// Swatches traced as one colour.
#[derive(Clone, Debug, PartialEq)]
pub struct Group {
    /// The pixel-weighted mean of its swatches.
    pub colour: [u8; 3],
    pub pixels: usize,
    /// Its swatches, by number in [`Swatches::list`], most used first.
    pub swatches: Vec<usize>,
}

impl Group {
    /// The colour that names the group: its most used swatch's.
    #[must_use]
    pub fn key(&self, swatches: &Swatches) -> String {
        hex(swatches.list[self.swatches[0]].colour)
    }
}

/// The swatches in groups, darkest first, and each swatch's group.
#[derive(Clone, Debug, PartialEq)]
pub struct Grouping {
    pub groups: Vec<Group>,
    pub of_swatch: Vec<usize>,
}

/// `swatches` in groups: each swatch with the first more-used one that looks within
/// `similarity` (ΔE) of it, then the sets the user merged (by `#rrggbb`) together, then
/// groups under [`SMALLEST_SHARE`] of the picture into the nearest larger one.
#[must_use]
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "means of channel values and pixel counts"
)]
pub fn grouped(swatches: &Swatches, similarity: f64, merged: &[Vec<String>]) -> Grouping {
    let list = &swatches.list;
    // leaders: the most used swatch of each cluster
    let mut leader_of: Vec<usize> = Vec::with_capacity(list.len());
    let mut leaders: Vec<usize> = vec![];
    for (k, s) in list.iter().enumerate() {
        if let Some(&l) = leaders
            .iter()
            .find(|&&l| difference(list[l].colour, s.colour) <= similarity)
        {
            leader_of.push(l);
        } else {
            leaders.push(k);
            leader_of.push(k);
        }
    }
    // merged by hand: union of the clusters each set's swatches are in
    let mut parent: Vec<usize> = (0..list.len()).collect();
    let by_hex: BTreeMap<String, usize> = list
        .iter()
        .enumerate()
        .map(|(k, s)| (hex(s.colour), k))
        .collect();
    for set in merged {
        let members: Vec<usize> = set.iter().filter_map(|h| by_hex.get(h)).copied().collect();
        for pair in members.windows(2) {
            let (a, b) = (
                root(&mut parent, leader_of[pair[0]]),
                root(&mut parent, leader_of[pair[1]]),
            );
            // the more used (lower number) leads
            let (keep, join) = (a.min(b), a.max(b));
            parent[join] = keep;
        }
    }
    let mut members: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for (k, &leader) in leader_of.iter().enumerate() {
        let r = root(&mut parent, leader);
        members.entry(r).or_default().push(k);
    }
    let mut groups: Vec<Vec<usize>> = members.into_values().collect();
    // the smallest groups into the nearest larger one
    let total: usize = list.iter().map(|s| s.pixels).sum();
    let pixels = |g: &[usize]| g.iter().map(|&k| list[k].pixels).sum::<usize>();
    let colour = |g: &[usize]| {
        let n = pixels(g).max(1) as f64;
        let mut sum = [0.0; 3];
        for &k in g {
            for (i, s) in sum.iter_mut().enumerate() {
                *s += f64::from(list[k].colour[i]) * list[k].pixels as f64;
            }
        }
        sum.map(|s| (s / n).round() as u8)
    };
    let smallest = (total as f64 * SMALLEST_SHARE).ceil() as usize;
    let (big, small): (Vec<_>, Vec<_>) = groups.drain(..).partition(|g| pixels(g) >= smallest);
    let mut groups = if big.is_empty() {
        // nothing big enough: all of them stay
        small
    } else {
        let mut big = big;
        let colours: Vec<[u8; 3]> = big.iter().map(|g| colour(g)).collect();
        for g in small {
            let c = colour(&g);
            let nearest = (0..big.len())
                .min_by(|&a, &b| difference(colours[a], c).total_cmp(&difference(colours[b], c)))
                .unwrap_or(0);
            big[nearest].extend(g);
        }
        big
    };
    for g in &mut groups {
        g.sort_unstable();
    }
    let mut groups: Vec<Group> = groups
        .into_iter()
        .map(|g| Group {
            colour: colour(&g),
            pixels: pixels(&g),
            swatches: g,
        })
        .collect();
    groups.sort_by(|a, b| {
        lightness(a.colour)
            .total_cmp(&lightness(b.colour))
            .then(a.swatches[0].cmp(&b.swatches[0]))
    });
    let mut of_swatch = vec![0; list.len()];
    for (n, g) in groups.iter().enumerate() {
        for &k in &g.swatches {
            of_swatch[k] = n;
        }
    }
    Grouping { groups, of_swatch }
}

/// The set `k` is in: followed up its parents, the path shortened on the way.
fn root(parent: &mut [usize], mut k: usize) -> usize {
    while parent[k] != k {
        parent[k] = parent[parent[k]];
        k = parent[k];
    }
    k
}

/// The merged sets with the swatches `chosen` (by `#rrggbb`) merged too: one set holding
/// them and every set any of them was in.
#[must_use]
pub fn merge(merged: &[Vec<String>], chosen: &[String]) -> Vec<Vec<String>> {
    let (touching, mut kept): (Vec<_>, Vec<_>) = merged
        .iter()
        .cloned()
        .partition(|set| set.iter().any(|h| chosen.contains(h)));
    let mut joined: Vec<String> = touching.into_iter().flatten().collect();
    joined.extend(chosen.iter().cloned());
    joined.sort();
    joined.dedup();
    if joined.len() > 1 {
        kept.push(joined);
    }
    kept
}

/// The merged sets with the swatches `chosen` taken out of them, so they stand alone again
/// (unless they look alike enough to group by similarity).
#[must_use]
pub fn separate(merged: &[Vec<String>], chosen: &[String]) -> Vec<Vec<String>> {
    merged
        .iter()
        .map(|set| {
            set.iter()
                .filter(|h| !chosen.contains(h))
                .cloned()
                .collect::<Vec<_>>()
        })
        .filter(|set| set.len() > 1)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn picture(colours: &[([u8; 3], usize)]) -> Vec<u8> {
        colours
            .iter()
            .flat_map(|&(c, n)| std::iter::repeat_n(c, n).flatten())
            .collect()
    }

    #[test]
    fn flat_artwork_shows_each_of_its_colours_most_used_first() {
        let rgb = picture(&[
            ([250, 250, 250], 600),
            ([200, 20, 20], 300),
            ([20, 20, 200], 100),
        ]);
        let s = Swatches::of(&rgb);
        let colours: Vec<_> = s.list.iter().map(|w| (w.colour, w.pixels)).collect();
        assert_eq!(
            colours,
            vec![
                ([250, 250, 250], 600),
                ([200, 20, 20], 300),
                ([20, 20, 200], 100)
            ]
        );
        assert_eq!(s.index([200, 20, 20]), 1);
        // a colour not in the picture: the nearest swatch
        assert_eq!(s.index([30, 30, 190]), 2);
    }

    #[test]
    fn a_photo_is_cut_down_to_at_most_48_swatches_that_cover_every_pixel() {
        // a smooth ramp through thousands of colours
        let mut rgb = vec![];
        for r in 0..64u8 {
            for g in 0..64u8 {
                rgb.extend([r * 4, g * 4, 255 - r * 2]);
            }
        }
        let s = Swatches::of(&rgb);
        assert_eq!(s.list.len(), MOST_SWATCHES);
        assert_eq!(s.list.iter().map(|w| w.pixels).sum::<usize>(), 64 * 64);
        assert!(s.list.windows(2).all(|w| w[0].pixels >= w[1].pixels));
    }

    #[test]
    fn near_identical_colours_group_by_similarity_and_apart_at_zero() {
        let rgb = picture(&[
            ([250, 250, 250], 500),
            ([200, 20, 20], 300),
            ([206, 24, 18], 100),
            ([20, 20, 200], 100),
        ]);
        let s = Swatches::of(&rgb);
        let apart = grouped(&s, 0.0, &[]);
        assert_eq!(apart.groups.len(), 4);
        let near = grouped(&s, 5.0, &[]);
        assert_eq!(near.groups.len(), 3, "the two reds are one");
        // darkest first: blue, red, white
        assert_eq!(near.groups[2].colour, [250, 250, 250]);
        let red = &near.groups[near.of_swatch[s.index([206, 24, 18])]];
        assert_eq!(red.pixels, 400);
        assert_eq!(red.key(&s), "#c81414", "named after its most used swatch");
        // averaged by pixels
        assert_eq!(red.colour, [202, 21, 20]);
    }

    #[test]
    fn swatches_merged_by_hand_are_one_group_and_separate_undoes_it() {
        let rgb = picture(&[
            ([250, 250, 250], 500),
            ([200, 20, 20], 300),
            ([20, 20, 200], 200),
        ]);
        let s = Swatches::of(&rgb);
        let sets = merge(&[], &["#c81414".into(), "#1414c8".into()]);
        assert_eq!(sets, vec![vec!["#1414c8".to_string(), "#c81414".into()]]);
        let g = grouped(&s, 0.0, &sets);
        assert_eq!(g.groups.len(), 2);
        assert_eq!(g.groups[0].pixels, 500, "red and blue together");
        // merging one more joins the set it touches
        let more = merge(&sets, &["#c81414".into(), "#fafafa".into()]);
        assert_eq!(more.len(), 1);
        assert_eq!(grouped(&s, 0.0, &more).groups.len(), 1);
        assert_eq!(
            separate(&sets, &["#c81414".into()]),
            Vec::<Vec<String>>::new()
        );
    }

    #[test]
    fn slivers_of_in_between_colour_join_the_nearest_larger_group() {
        let rgb = picture(&[
            ([250, 250, 250], 5000),
            ([20, 20, 20], 4990),
            // anti-aliasing between them: 0.1 % of the picture
            ([135, 135, 135], 10),
        ]);
        let s = Swatches::of(&rgb);
        let g = grouped(&s, 0.0, &[]);
        assert_eq!(g.groups.len(), 2);
        assert_eq!(g.groups.iter().map(|g| g.pixels).sum::<usize>(), 10_000);
    }

    #[test]
    fn differences_follow_how_colours_look() {
        assert!(difference([0, 0, 0], [255, 255, 255]) > 99.0);
        assert!(difference([200, 20, 20], [200, 20, 20]) < 1e-9);
        assert!(difference([200, 20, 20], [206, 24, 18]) < 5.0);
    }
}
