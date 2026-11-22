//! Laying the moves along the line: the ramps, leads and tabs of each pass.

use super::*;

impl Out<'_> {
    pub(super) fn push(&mut self, kind: MoveKind, p: Point, z: f64) {
        self.moves.push(Move {
            kind,
            to: DVec3::new(p.x, p.y, z),
        });
    }

    pub(super) fn here(&self) -> Point {
        self.moves.last().map_or(Point::ZERO, |m| m.to.truncate())
    }

    pub(super) fn rapid_down(&mut self, p: Point, safe_z: f64, to: f64, kind: MoveKind) {
        self.push(MoveKind::Rapid, p, safe_z);
        self.push(kind, p, to);
    }

    pub(super) fn rapid_up(&mut self, safe_z: f64) {
        let p = self.here();
        self.push(MoveKind::Rapid, p, safe_z);
    }

    /// Down from `above` to `z` along the path from `from`, as the ramp says; returns where
    /// along the path the tool is at the bottom.
    pub(super) fn ramp_down(&mut self, from: f64, above: f64, z: f64) -> f64 {
        let (line, ramping) = (self.line, self.entry.ramping);
        if let Some(settings) = self.entry.enter {
            return self.enter(line, from, above, z, settings);
        }
        if self.entry.no_plunge && self.entry.ramp == Ramp::None {
            // an older toolpath going straight down with a bit that must not
            let settings = EntrySettings {
                mode: EntryMode::Plunge,
                ..EntrySettings::default()
            };
            return self.enter(line, from, above, z, settings);
        }
        let drop = above - z;
        let slope = ramping.angle.to_radians().tan();
        let by_angle = ramping.by == RampBy::Angle && slope > 0.0;
        match self.entry.ramp {
            _ if drop <= 0.0 => {}
            Ramp::None => {
                if self.moves.last().is_none_or(|m| m.to.z > z) {
                    self.push(MoveKind::Plunge, line.point(from), z);
                }
            }
            Ramp::Smooth | Ramp::Spiral => {
                let wanted = if by_angle {
                    drop / slope
                } else {
                    ramping.distance
                };
                let room = if line.closed {
                    line.length
                } else {
                    line.length - from
                };
                let length = wanted.min(room);
                if length <= 0.0 {
                    self.push(MoveKind::Plunge, line.point(from), z);
                    return from;
                }
                self.descend(&line.stations(from, from + length), above, z);
                if line.closed {
                    return from + length;
                }
                // an open path: back to its start along the cut just made
                for s in line.stations(from + length, from).into_iter().skip(1) {
                    self.push(MoveKind::Cut, line.point(s), z);
                }
            }
            Ramp::ZigZag => {
                let room = if line.closed {
                    line.length
                } else {
                    line.length - from
                };
                let leg = ramping.distance.min(room);
                if leg <= 0.0 {
                    self.push(MoveKind::Plunge, line.point(from), z);
                    return from;
                }
                #[expect(
                    clippy::cast_possible_truncation,
                    clippy::cast_sign_loss,
                    reason = "a small positive count"
                )]
                let legs = if by_angle {
                    let n = (drop / (slope * leg)).ceil().max(2.0) as usize;
                    n + n % 2
                } else {
                    2
                };
                let mut stations = vec![];
                for i in 0..legs {
                    let (out, back) = if i % 2 == 0 {
                        (from, from + leg)
                    } else {
                        (from + leg, from)
                    };
                    let leg = line.stations(out, back);
                    stations.extend(if i == 0 { &leg[..] } else { &leg[1..] });
                }
                self.descend(&stations, above, z);
            }
        }
        from
    }

    /// Down from `above` to `z` where `track` is at `from`, by `settings`, fitted to the room
    /// there; a fallback is noted where the mode asked for has no room. Returns where along
    /// `track` the tool is at the bottom.
    pub(super) fn enter(
        &mut self,
        track: &Line,
        from: f64,
        above: f64,
        z: f64,
        settings: EntrySettings,
    ) -> f64 {
        let drop = above - z;
        if drop <= 0.0 {
            return from;
        }
        let wanted = settings.mode;
        let at = track.point(from);
        let (used, end) = match wanted {
            EntryMode::Plunge if self.entry.no_plunge => {
                let distance = settings.ramp_distance.max(1e-6);
                if room_along(track, from) + 1e-9 >= distance {
                    (EntryMode::Ramp, self.slope(track, from, distance, above, z))
                } else {
                    let angle = drop.atan2(distance).to_degrees();
                    self.zig_zag(track, from, above, z, angle, 2)
                }
            }
            EntryMode::Plunge => {
                if self.moves.last().is_none_or(|m| m.to.z > z) {
                    self.push(MoveKind::Plunge, at, z);
                }
                (EntryMode::Plunge, from)
            }
            EntryMode::Helix => {
                if self.helix(track, from, above, z, settings) {
                    (EntryMode::Helix, from)
                } else {
                    self.zig_zag(track, from, above, z, settings.helix_angle, 2)
                }
            }
            EntryMode::ZigZag => self.zig_zag(
                track,
                from,
                above,
                z,
                settings.zigzag_angle,
                settings.zigzag_passes,
            ),
            EntryMode::Ramp => {
                let distance = settings.ramp_distance.max(1e-6);
                if room_along(track, from) + 1e-9 >= distance {
                    (EntryMode::Ramp, self.slope(track, from, distance, above, z))
                } else {
                    let angle = drop.atan2(distance).to_degrees();
                    self.zig_zag(track, from, above, z, angle, 2)
                }
            }
        };
        if used != wanted {
            self.entry
                .fallbacks
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .push(Fallback { at, wanted, used });
        }
        end
    }

    /// Down along `track` from `from` over `distance` in one slope; on a closed track the
    /// cut goes on from the bottom, on an open one the tool comes back to `from` at depth.
    pub(super) fn slope(
        &mut self,
        track: &Line,
        from: f64,
        distance: f64,
        above: f64,
        z: f64,
    ) -> f64 {
        self.descend_on(track, &track.stations(from, from + distance), above, z);
        if track.closed {
            return from + distance;
        }
        for s in track.stations(from + distance, from).into_iter().skip(1) {
            self.push(MoveKind::Cut, track.point(s), z);
        }
        from
    }

    /// Down back and forth along `track` from `from`, in `passes` legs (made even, so it ends
    /// where it began) no steeper than `angle`°: shorter legs, and more, where the track is
    /// short. Straight down where there is no room for a leg.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "a small positive count"
    )]
    pub(super) fn zig_zag(
        &mut self,
        track: &Line,
        from: f64,
        above: f64,
        z: f64,
        angle: f64,
        passes: u32,
    ) -> (EntryMode, f64) {
        let drop = above - z;
        let slope = angle.clamp(0.1, 89.0).to_radians().tan();
        let room = room_along(track, from);
        let shortest = (SHORTEST_LEG * self.entry.tool_diameter).max(SHORTEST_LEG_MM);
        if room < shortest {
            self.push(MoveKind::Plunge, track.point(from), z);
            return (EntryMode::Plunge, from);
        }
        let even = |n: u32| n.max(2) + n % 2;
        let mut legs = even(passes.max(1));
        let mut leg = drop / (f64::from(legs) * slope);
        if leg > room {
            leg = room;
            legs = even((drop / (leg * slope) - 1e-9).ceil().min(1e6) as u32);
        }
        let mut stations = vec![];
        for i in 0..legs {
            let (out, back) = if i % 2 == 0 {
                (from, from + leg)
            } else {
                (from + leg, from)
            };
            let leg = track.stations(out, back);
            stations.extend(if i == 0 { &leg[..] } else { &leg[1..] });
        }
        self.descend_on(track, &stations, above, z);
        (EntryMode::ZigZag, from)
    }

    /// Down in a helix beside where `track` is at `from`, in the room there: as big as asked,
    /// or as big as fits, no steeper than asked; then once round flat at the bottom, and
    /// onto the track. False, with nothing done, where even a small one does not fit.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "a small positive count"
    )]
    pub(super) fn helix(
        &mut self,
        track: &Line,
        from: f64,
        above: f64,
        z: f64,
        settings: EntrySettings,
    ) -> bool {
        let Some(room) = self.entry.room.as_ref() else {
            return false;
        };
        let bit = self.entry.tool_diameter;
        if bit <= 0.0 {
            return false;
        }
        let wanted = bit * settings.helix_diameter / 200.0;
        let least = (bit * SMALLEST_HELIX / 2.0).min(wanted);
        if wanted <= 0.0 {
            return false;
        }
        let p = track.point(from);
        let ahead = track.tangent(from);
        let before = if from > 1e-6 {
            from - 1e-6
        } else if track.closed {
            track.length - 1e-6
        } else {
            from
        };
        let behind = track.tangent(before);
        let square = (behind.perp() + ahead.perp()).normalize_or(ahead.perp());
        let sides: &[f64] = match self.entry.side {
            s if s > 0.0 => &[1.0],
            s if s < 0.0 => &[-1.0],
            _ => &[1.0, -1.0],
        };
        // the centre of a helix `r` round that fits, out from the track on a side
        let place = |r: f64| {
            sides.iter().find_map(|&side| {
                [1.0, 1.1, 1.25, 1.5, 2.0, 3.0].iter().find_map(|&k| {
                    let centre = p + square * side * r * k;
                    let start = centre + (p - centre).normalize_or_zero() * r;
                    (room.fits(centre, r) && room.holds(start, p)).then_some(centre)
                })
            })
        };
        let (radius, centre) = if let Some(c) = place(wanted) {
            (wanted, c)
        } else {
            let Some(mut fit) = place(least).map(|c| (least, c)) else {
                return false;
            };
            let (mut lo, mut hi) = (least, wanted);
            for _ in 0..24 {
                let mid = f64::midpoint(lo, hi);
                if let Some(c) = place(mid) {
                    fit = (mid, c);
                    lo = mid;
                } else {
                    hi = mid;
                }
            }
            fit
        };
        let start = centre + (p - centre).normalize_or(square) * radius;
        let a0 = (start - centre).to_angle();
        // round the way the cut goes on
        let turn = if (start - centre).perp().dot(ahead) >= 0.0 {
            1.0
        } else {
            -1.0
        };
        let drop = above - z;
        let slope = settings.helix_angle.clamp(0.1, 89.0).to_radians().tan();
        let round = std::f64::consts::TAU * radius;
        let turns = (drop / (round * slope) - 1e-9).ceil().max(1.0) as u32;
        let per_turn = PER_TURN.max((round / 1.0).ceil() as u32);
        let on = |k: u32| {
            let a = a0 + turn * std::f64::consts::TAU * f64::from(k) / f64::from(per_turn);
            centre + Point::from_angle(a) * radius
        };
        if start.distance(p) > 1e-9 {
            self.push(MoveKind::Cut, start, above);
        }
        let steps = turns * per_turn;
        for k in 1..=steps {
            let depth = above - drop * f64::from(k) / f64::from(steps);
            self.push(MoveKind::Plunge, on(k), depth);
        }
        for k in 1..=per_turn {
            self.push(MoveKind::Cut, on(k), z);
        }
        if start.distance(p) > 1e-9 {
            self.push(MoveKind::Cut, p, z);
        }
        true
    }

    /// Down from `above` to `z` steadily over `track` through `stations`.
    pub(super) fn descend_on(&mut self, track: &Line, stations: &[f64], above: f64, z: f64) {
        let total: f64 = stations.windows(2).map(|w| (w[1] - w[0]).abs()).sum();
        let mut gone = 0.0;
        for w in stations.windows(2) {
            gone += (w[1] - w[0]).abs();
            let depth = above + (z - above) * gone / total.max(1e-12);
            self.push(MoveKind::Plunge, track.point(w[1]), depth);
        }
    }

    /// Down from `above` to `z` steadily over the path through `stations`.
    pub(super) fn descend(&mut self, stations: &[f64], above: f64, z: f64) {
        let total: f64 = stations.windows(2).map(|w| (w[1] - w[0]).abs()).sum();
        let mut gone = 0.0;
        for w in stations.windows(2) {
            gone += (w[1] - w[0]).abs();
            let depth = above + (z - above) * gone / total;
            self.push(MoveKind::Plunge, self.line.point(w[1]), depth);
        }
    }

    /// Cuts from `a` to `b` along the path at `z` (or from `start` down to `z`, a spiral),
    /// lifting over the tabs.
    pub(super) fn along(&mut self, from: f64, to: f64, z: f64, start: Option<f64>, kind: MoveKind) {
        let lift = self.tab_lift(z);
        let mut stations = self.line.stations(from, to);
        if let Some((half, _, _)) = lift {
            let period = self.line.length;
            for &c in &self.tabs {
                for turn in [0.0, period, 2.0 * period] {
                    for edge in [c - half, c, c + half] {
                        let at = edge + turn;
                        if at > from && at < to {
                            stations.push(at);
                        }
                    }
                }
            }
            stations.sort_by(f64::total_cmp);
        }
        let depth = |at: f64| start.map_or(z, |top| top + (z - top) * (at - from) / (to - from));
        for at in stations.into_iter().skip(1) {
            let point = self.line.point(at);
            let base = depth(at);
            let Some((half, top, flat)) = lift else {
                self.push(kind, point, base);
                continue;
            };
            // how far into the nearest tab, from its middle
            let off = self
                .tabs
                .iter()
                .map(|&c| {
                    let gap = (at - c).rem_euclid(self.line.length);
                    gap.min(self.line.length - gap)
                })
                .fold(f64::INFINITY, f64::min);
            if flat {
                let edge = (off - half).abs() < 1e-9;
                let entering = edge && self.moves.last().is_some_and(|m| m.to.z < top - 1e-9);
                if edge {
                    // straight up or down at the tab's ends
                    let (first, then) = if entering { (base, top) } else { (top, base) };
                    self.push(kind, point, first.max(base));
                    self.push(kind, point, then.max(base));
                } else {
                    self.push(kind, point, if off < half { top.max(base) } else { base });
                }
            } else {
                let rise = (top - base) * (1.0 - off / half);
                self.push(kind, point, base + rise.max(0.0));
            }
        }
    }

    /// Half a tab's lifted length, its top, and whether it is flat, when `z` is below it.
    pub(super) fn tab_lift(&self, z: f64) -> Option<(f64, f64, bool)> {
        let e = self.entry;
        (e.tabs != Tabs::None && !self.tabs.is_empty() && z < e.tab_top - 1e-9).then(|| {
            (
                f64::midpoint(e.tab.length, e.tool_diameter),
                e.tab_top,
                e.tabs == Tabs::Flat,
            )
        })
    }

    /// One pass entered and left along leads: down at the lead's start (ramping along it if
    /// asked), round the path, past its start by the overcut, out, and up.
    pub(super) fn with_leads(&mut self, above: f64, z: f64, safe_z: f64) {
        let (line, leads, side) = (self.line, self.entry.leads, self.entry.side);
        let lead_in = lead(self.entry, line.point(0.0), line.tangent(0.0), side, false);
        let ramping = self.entry.ramp != Ramp::None && above > z;
        if let Some(settings) = self.entry.enter {
            // down where the lead starts, then along it at depth
            let track = Line::new(&lead_in, false);
            self.rapid_down(lead_in[0], safe_z, above.max(z), MoveKind::Plunge);
            self.enter(&track, 0.0, above, z, settings);
            for &p in &lead_in[1..] {
                self.push(MoveKind::Cut, p, z);
            }
        } else {
            self.rapid_down(
                lead_in[0],
                safe_z,
                if ramping { above } else { z },
                MoveKind::Plunge,
            );
            let total: f64 = lead_in.windows(2).map(|w| w[0].distance(w[1])).sum();
            let mut gone = 0.0;
            for w in lead_in.windows(2) {
                gone += w[0].distance(w[1]);
                if ramping {
                    let depth = above + (z - above) * gone / total;
                    self.push(MoveKind::Plunge, w[1], depth);
                } else {
                    self.push(MoveKind::Cut, w[1], z);
                }
            }
        }
        let end = if line.closed {
            line.length + leads.overcut.min(line.length)
        } else {
            line.length
        };
        self.along(0.0, end, z, None, MoveKind::Cut);
        if leads.lead_out {
            let out = lead(self.entry, line.point(end), line.tangent(end), side, true);
            for &p in &out[1..] {
                self.push(MoveKind::Cut, p, z);
            }
        }
        self.rapid_up(safe_z);
    }
}

/// A lead onto the path at `at`, travelling along `tangent` (or off it, `out`), from the
/// `side` of it, as points in the order cut.
pub(super) fn lead(entry: &Entry, at: Point, tangent: Point, side: f64, out: bool) -> Vec<Point> {
    let leads = entry.leads;
    let normal = tangent.perp() * side;
    let away = if out { 1.0 } else { -1.0 };
    let points = match entry.lead {
        Lead::Circular if leads.radius > 0.0 => {
            let sweep = (leads.length / leads.radius).min(std::f64::consts::FRAC_PI_2);
            let centre = at + normal * leads.radius;
            // the arc runs anticlockwise round its centre when it is on the left
            let turn = side * away;
            let steps = 16;
            (0..=steps)
                .map(|i| {
                    let a = turn * sweep * f64::from(i) / f64::from(steps);
                    centre + Point::from_angle(a).rotate(at - centre)
                })
                .collect::<Vec<_>>()
        }
        _ => {
            let a = leads.angle.to_radians();
            let off = at + (tangent * a.cos() * away + normal * a.sin()) * leads.length;
            vec![at, off]
        }
    };
    if out {
        points
    } else {
        points.into_iter().rev().collect()
    }
}
