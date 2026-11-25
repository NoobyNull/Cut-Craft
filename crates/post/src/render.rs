//! Writing a job's toolpaths with a post: its blocks, moves, arcs and values.

use super::{
    CLOSE_ENOUGH, DVec3, Document, File, Format, HashMap, Job, LARGEST_ARC, LENGTHS, MM_PER_INCH,
    MoveKind, Output, Path, Post, Step, Toolpath, Values, arcs, dwell, notes_lines, parts,
    tool_numbers, tools_used, winding, xy_origin, z_origin,
};

impl Post {
    #[expect(
        clippy::too_many_lines,
        clippy::cognitive_complexity,
        reason = "one pass writing every toolpath in order"
    )]
    pub fn render(&self, doc: &Document, file: &File) -> Result<String, String> {
        let toolpaths: Vec<_> = doc.toolpaths.values().collect();
        let Some(first) = toolpaths.first() else {
            return Err("there are no toolpaths to save".into());
        };
        let tools = tools_used(&toolpaths);
        self.can_write(doc, &toolpaths)?;
        let number = tool_numbers(&tools, file.slots);
        let mut out = Output {
            post: self,
            text: String::new(),
            last: HashMap::new(),
            line_number: self.numbering.start,
        };
        let mut winding = self.winding(&doc.job);
        let wrapped = |label: &str, flat: f64, winding: &mut Option<winding::Winding>| match (
            winding.as_mut(),
            &self.wrap,
        ) {
            (Some(w), Some((axis, _))) if *axis == label => w.degrees(flat),
            _ => self.wrapped(&doc.job, label, flat),
        };
        let mut v = self.job_values(doc, file);
        v.lists.insert(
            "TOOLS_USED",
            tools.iter().map(|&(t, _)| t.to_owned()).collect(),
        );
        v.lists.insert(
            "TOOLPATHS_OUTPUT",
            toolpaths.iter().map(|t| t.name.clone()).collect(),
        );
        v.texts.insert("TOOLPATH_NAME", first.name.clone());
        v.texts.insert("TOOLNAME", first.tool.name.clone());
        v.tool(&first.tool, number(&first.tool.name));
        let header_at = out.text.len();
        out.block("HEADER", &v);
        if doc.notes_in_gcode {
            out.notes(header_at, &notes_lines(doc, file));
        }

        let mut previous_tool: Option<&str> = None;
        let mut previous_speed: Option<f64> = None;
        let mut first_rapid = true;
        // the rate of the feed moves since the last rapid: a new one starts a FIRST_ block,
        // which is where most posts print it
        let mut feeding: Option<f64> = None;
        let home = doc.job.home();
        let mut at = DVec3::new(home.x, home.y, home.z);
        // how far off the turning axis the bit is held, and where that is on the machine's
        // across axis (Y for a rotary along X): from the axis's centre, the way A turns
        let mut held = 0.0;
        // only when a toolpath holds the bit off the axis: otherwise the file leaves the
        // across axis where the machine was set up, as it always has
        let holds_off = toolpaths
            .iter()
            .any(|t| t.across.iter().any(|a| a.abs() > 1e-9));
        let across_at = doc
            .job
            .rotary
            .filter(|_| self.wrap.is_some() && holds_off)
            .map(|r| {
                let axis = self.rotary.unwrap_or_default();
                let sign = axis.across_sign(r.along);
                move |across: f64| axis.centre_across + sign * across
            });
        for toolpath in &toolpaths {
            v.texts.insert("TOOLPATH_NAME", toolpath.name.clone());
            v.texts.insert("TOOLNAME", toolpath.tool.name.clone());
            v.tool(&toolpath.tool, number(&toolpath.tool.name));
            match previous_tool {
                Some(t) if t != toolpath.tool.name => {
                    out.block("TOOLCHANGE", &v);
                    out.block("SPINDLE_ON", &v);
                    // a pause whose tool change leaves starting the spindle to SPINDLE_ON
                    // waits for it here, once it is turning
                    out.block(super::AFTER_SPINDLE_ON, &v);
                    // the new bit has just been zeroed by hand, so the machine's height is
                    // unknown: print every value again, Z above all, rather than leave the
                    // first rapid to travel sideways at whatever height the bit was left
                    out.last.clear();
                    // lift straight up to the height the last toolpath left at before any
                    // sideways move: only Z changes, so print X and Y only if the post
                    // always prints them
                    v.numbers.insert("X", wrapped("X", at.x, &mut winding));
                    v.numbers.insert("Y", wrapped("Y", at.y, &mut winding));
                    v.numbers.insert("Z", doc.job.machine_z(at.z));
                    if let Some(across_at) = &across_at {
                        v.numbers.insert("ACROSS", across_at(held));
                    }
                    out.value("X", &v);
                    out.value("Y", &v);
                    out.block("RAPID_MOVE", &v);
                    // the pause took the head to the change position, not where the last
                    // toolpath ended: the next move prints X and Y whatever they were
                    if self.park.is_some() {
                        out.last.remove("X");
                        out.last.remove("Y");
                    }
                }
                Some(_) => {
                    out.block("NEW_SEGMENT", &v);
                    // the same bit at another speed (another material, or the speed edited
                    // between toolpaths): its feeds print on every move, but the spindle
                    // speed only where the post starts the spindle, so start it again
                    if previous_speed != Some(toolpath.tool.spindle_speed)
                        && toolpath.power.is_empty()
                    {
                        if out.post.blocks.contains_key("SPINDLE_ON") {
                            out.block("SPINDLE_ON", &v);
                        } else {
                            out.line("[S] M3", &v);
                        }
                    }
                }
                None => {}
            }
            previous_tool = Some(&toolpath.tool.name);
            previous_speed = Some(toolpath.tool.spindle_speed);
            let laser = !toolpath.power.is_empty();
            // the laser's power while it is on
            let mut burning: Option<f64> = None;
            // a drill bank: which drills are down, and how the post chooses them
            let bank = toolpath.drill_bank.as_ref();
            let by_pattern = bank.is_some() && self.bank_mode()?;
            let mut down = vec![];
            if bank.is_some() {
                out.block("SETUP_DRILL_BANK", &v);
            }
            for (hit, part) in parts(toolpath) {
                if let (Some(b), Some(hit)) = (bank, hit) {
                    out.drop_drills(&mut v, &b.bank, &hit.drills, by_pattern, &mut down);
                }
                let mut steps = self.steps(&part, &doc.job, at, held);
                if let Some(w) = winding.as_mut() {
                    steps = w.plan(steps, at);
                }
                for step in steps {
                    let (kind, to, across) = match step {
                        Step::Line(kind, to, _, across) => (kind, to, across),
                        Step::Arc(kind, arc) => (kind, arc.to, 0.0),
                    };
                    let from = std::mem::replace(&mut at, to);
                    let moved_across = across - std::mem::replace(&mut held, across);
                    // the bit moving only across the axis still moves
                    let length = (to.distance_squared(from) + moved_across * moved_across).sqrt();
                    if self.inverse_time && kind != MoveKind::Rapid && length == 0.0 {
                        // a move that goes nowhere has no time to invert
                        continue;
                    }
                    v.numbers.insert("X", wrapped("X", to.x, &mut winding));
                    v.numbers.insert("Y", wrapped("Y", to.y, &mut winding));
                    v.numbers.insert("Z", doc.job.machine_z(to.z));
                    if let Some(across_at) = &across_at {
                        v.numbers.insert("ACROSS", across_at(across));
                    }
                    if let (true, Step::Line(_, _, power, _)) = (laser, step) {
                        v.numbers.insert("P", power);
                        out.jet(&v, kind, power, &mut burning);
                    }
                    let block = if kind == MoveKind::Rapid {
                        feeding = None;
                        let initial = std::mem::replace(&mut first_rapid, false);
                        if initial && self.blocks.contains_key("INITIAL_RAPID_MOVE") {
                            "INITIAL_RAPID_MOVE"
                        } else {
                            "RAPID_MOVE"
                        }
                    } else {
                        let rate = if kind == MoveKind::Plunge {
                            toolpath.tool.plunge_rate
                        } else {
                            toolpath.tool.feed_rate
                        };
                        v.numbers.insert("F", rate);
                        v.numbers.insert(
                            "FI",
                            winding.as_ref().map_or(rate / length, |w| {
                                w.inverse_time(from, to, moved_across, rate)
                            }),
                        );
                        let start = feeding.replace(rate) != Some(rate);
                        if let Step::Arc(_, arc) = step {
                            let (printed_from, arc) = self.as_printed(from, arc);
                            v.arc(printed_from, &arc);
                            self.arc_block(&arc, from, start)
                        } else {
                            self.feed_block(kind, start)
                        }
                    };
                    out.block(block, &v);
                    // a drill waits at the bottom of each peck when its toolpath asks
                    let line = matches!(step, Step::Line(..));
                    if line && kind == MoveKind::Plunge && dwell(toolpath) > 0.0 {
                        v.numbers.insert("DWELL", dwell(toolpath));
                        out.block("DWELL_MOVE", &v);
                    }
                }
            }
            if let Some(b) = bank {
                out.drop_drills(&mut v, &b.bank, &[], by_pattern, &mut down);
                out.block("FINAL_DRILL_BANK_ACTION", &v);
            }
            if burning.is_some() {
                out.block("JET_TOOL_OFF", &v);
            }
        }
        if let (Some(w), Some((axis, _))) = (winding.as_mut(), &self.wrap) {
            // home round the material: unwound only after a spin back
            let (flat, now) = if *axis == "X" {
                (home.x, at.x)
            } else {
                (home.y, at.y)
            };
            let label = if *axis == "X" { "XH" } else { "YH" };
            let w_home = w.home(flat, now);
            v.numbers.insert(label, w.degrees(w_home));
        }
        out.block("FOOTER", &v);
        Ok(out.text)
    }

    /// Refuses what this post cannot write: a flat job for a rotary post, lasers without
    /// laser support.
    pub(super) fn can_write(
        &self,
        doc: &Document,
        toolpaths: &[&cut_craft_core::toolpath::Toolpath],
    ) -> Result<(), String> {
        if let Some((axis, letter)) = &self.wrap
            && doc.job.rotary.is_none()
        {
            return Err(format!(
                "this post wraps {axis} onto {letter}: set the job up as rotary in Job Setup"
            ));
        }
        // how far the bit is held off the axis goes move by move: a list that has lost step
        // with the moves would write each move after the first gap at another move's place
        if let Some(t) = toolpaths
            .iter()
            .find(|t| !t.across.is_empty() && t.across.len() != t.moves.len())
        {
            return Err(format!(
                "\"{}\" holds the bit off the turning axis, but its moves and its places across \
                 the axis no longer match ({} moves, {} places): calculate it again before saving",
                t.name,
                t.moves.len(),
                t.across.len()
            ));
        }
        if let Some(t) = toolpaths
            .iter()
            .find(|t| t.across.iter().any(|a| a.abs() > 1e-9))
            && (self.wrap.is_none() || !self.formats.contains_key("ACROSS"))
        {
            return Err(format!(
                "\"{}\" holds the bit off the turning axis, and this post can't move the bit \
                 across: pick one of Cut Craft's Rotary A posts",
                t.name
            ));
        }
        if let Some(t) = toolpaths
            .iter()
            .find(|t| dwell(t) > 0.0 && !self.blocks.contains_key("DWELL_MOVE"))
        {
            return Err(format!(
                "{} dwells at the bottom of each peck, but this post has no DWELL_MOVE block",
                t.name
            ));
        }
        if let Some(t) = toolpaths.iter().find(|t| t.drill_bank.is_some()) {
            self.bank_mode()
                .map_err(|why| format!("{} uses a drill bank, but this post {why}", t.name))?;
        }
        if let Some(t) = toolpaths
            .iter()
            .find(|t| !t.power.is_empty() && !self.laser)
        {
            return Err(format!(
                "{} is a laser toolpath, but this post does not support lasers (LASER_SUPPORT)",
                t.name
            ));
        }
        Ok(())
    }

    /// How this post drives a drill bank: by pattern (`true`: one block choosing every drill
    /// at once) or drill by drill (`false`), as the format's documentation says the blocks choose; why
    /// it cannot when it cannot.
    pub(super) fn bank_mode(&self) -> Result<bool, String> {
        let has = |name: &str| self.blocks.contains_key(name);
        if !has("SETUP_DRILL_BANK") || !has("FINAL_DRILL_BANK_ACTION") {
            return Err("cannot drive one: it needs SETUP_DRILL_BANK and \
                        FINAL_DRILL_BANK_ACTION blocks (ask the machine's maker for a post with \
                        drill bank support)"
                .into());
        }
        let per_drill = (
            has("DRILL_BANK_EXTEND_DRILL"),
            has("DRILL_BANK_RETRACT_DRILL"),
        );
        match (has("DRILL_BANK_PATTERN"), per_drill) {
            (true, (false, false)) => Ok(true),
            (false, (true, true)) => Ok(false),
            (true, _) => Err(
                "has both DRILL_BANK_PATTERN and the extend and retract blocks: \
                              it should have one or the other, not both"
                    .into(),
            ),
            (false, _) => Err("needs a DRILL_BANK_PATTERN block, or both \
                               DRILL_BANK_EXTEND_DRILL and DRILL_BANK_RETRACT_DRILL"
                .into()),
        }
    }

    /// The block for a feed move: plunges have their own if the post does, and a new rate
    /// (`start`) begins with the FIRST_ one.
    pub(super) fn feed_block(&self, kind: MoveKind, start: bool) -> &'static str {
        let (first, then) = if kind == MoveKind::Plunge && self.blocks.contains_key("PLUNGE_MOVE") {
            ("FIRST_PLUNGE_MOVE", "PLUNGE_MOVE")
        } else {
            ("FIRST_FEED_MOVE", "FEED_MOVE")
        };
        if start && self.blocks.contains_key(first) {
            first
        } else {
            then
        }
    }

    /// How many millimetres `mm` is once printed as `label`: rounded to the places the post
    /// prints it to, in its units.
    pub(super) fn printed(&self, label: &str, mm: f64) -> f64 {
        let step = self.printed_step(label);
        (mm / step).round() * step
    }

    /// The smallest step, in millimetres, `label` prints in.
    pub(super) fn printed_step(&self, label: &str) -> f64 {
        let format = self
            .formats
            .get(label)
            .cloned()
            .unwrap_or_else(|| Format::default_for(label));
        let unit = if self.inches && LENGTHS.contains(&label) {
            MM_PER_INCH
        } else {
            1.0
        };
        let places = i32::try_from(format.decimals).unwrap_or(i32::MAX);
        let multiplier = if format.multiplier.abs() > f64::EPSILON {
            format.multiplier.abs()
        } else {
            1.0
        };
        unit / multiplier / 10_f64.powi(places)
    }

    /// `arc` from `from` as the machine reads it: from where the last move was printed to,
    /// with its centre moved onto the line halfway between its printed ends and to the
    /// printed step nearest that keeps both ends the same distance from it. Controllers
    /// check this (GRBL stops with error 33 past 0.005 mm), and rounding to the places an
    /// inch post prints could otherwise put a small arc's ends that far apart.
    pub(super) fn as_printed(&self, from: DVec3, arc: arcs::Arc) -> (DVec3, arcs::Arc) {
        let at = |p: DVec3| {
            cut_craft_core::glam::DVec2::new(self.printed("X", p.x), self.printed("Y", p.y))
        };
        let (start, end) = (at(from), at(arc.to));
        let chord = end - start;
        let mut centre = arc.centre;
        if chord.length_squared() > f64::EPSILON {
            let middle = (start + end) / 2.0;
            centre -= chord * (centre - middle).dot(chord) / chord.length_squared();
        }
        let (si, sj) = (self.printed_step("I"), self.printed_step("J"));
        let offset = centre - start;
        let (i, j) = ((offset.x / si).round(), (offset.y / sj).round());
        let mismatch = |c: cut_craft_core::glam::DVec2| (c.distance(start) - c.distance(end)).abs();
        let nearest = start + cut_craft_core::glam::DVec2::new(i * si, j * sj);
        // the nearest rounding unless a step either way matches the ends noticeably better
        if mismatch(nearest) <= CLOSE_ENOUGH {
            return (
                start.extend(from.z),
                arcs::Arc {
                    centre: nearest,
                    ..arc
                },
            );
        }
        let best = (-1..=1)
            .flat_map(|a| (-1..=1).map(move |b| (f64::from(a), f64::from(b))))
            .map(|(a, b)| start + cut_craft_core::glam::DVec2::new((i + a) * si, (j + b) * sj))
            .min_by(|a, b| mismatch(*a).total_cmp(&mismatch(*b)))
            .unwrap_or(centre);
        (
            start.extend(from.z),
            arcs::Arc {
                centre: best,
                ..arc
            },
        )
    }

    /// The block for an arc starting at `from`, by its direction and whether it changes
    /// height; a new rate (`start`) begins with the FIRST_ one, as for feed moves.
    pub(super) fn arc_block(&self, arc: &arcs::Arc, from: DVec3, start: bool) -> &'static str {
        let helical = (arc.to.z - from.z).abs() > 1e-9;
        let [first, then] = match (arc.sweep < 0.0, helical) {
            (true, false) => ["FIRST_CW_ARC_MOVE", "CW_ARC_MOVE"],
            (false, false) => ["FIRST_CCW_ARC_MOVE", "CCW_ARC_MOVE"],
            (true, true) => ["FIRST_CW_HELICAL_ARC_MOVE", "CW_HELICAL_ARC_MOVE"],
            (false, true) => ["FIRST_CCW_HELICAL_ARC_MOVE", "CCW_HELICAL_ARC_MOVE"],
        };
        if start && self.blocks.contains_key(first) {
            first
        } else {
            then
        }
    }

    /// Which arcs may replace runs of cuts, if any: the post needs both arc blocks (and both
    /// helical ones for arcs that change height), and a flat job.
    pub(super) fn arc_limits(&self, laser: bool) -> Option<arcs::Limits> {
        let has = |names: [&str; 2]| names.iter().all(|n| self.blocks.contains_key(*n));
        // a wrapped axis is round, so arcs in the plane are not arcs on the machine
        // ponytail: lasers keep their lines, whose powers may differ
        if !has(["CW_ARC_MOVE", "CCW_ARC_MOVE"]) || self.wrap.is_some() || laser {
            return None;
        }
        let unit = if self.inches { MM_PER_INCH } else { 1.0 };
        Some(arcs::Limits {
            // the tolerance curves were flattened to, Options ▸ Advanced's curve accuracy
            tolerance: cut_craft_core::accuracy::curve(),
            min_radius: self.arc_radius.0.map_or(0.0, |r| r * unit),
            max_radius: self.arc_radius.1.map_or(LARGEST_ARC, |r| r * unit),
            helical: has(["CW_HELICAL_ARC_MOVE", "CCW_HELICAL_ARC_MOVE"]),
        })
    }

    /// The moves of `toolpath`, starting from `at` with the bit held `held_across` off the
    /// turning axis, as they are written: a plunge from above the job's Plunge (Z2) gap goes
    /// down to it at the rapid rate first, and runs of cuts become arcs where the post can
    /// write them.
    pub(super) fn steps(
        &self,
        toolpath: &Toolpath,
        job: &Job,
        mut at: DVec3,
        mut held_across: f64,
    ) -> Vec<Step> {
        let gap = job.plunge_gap();
        // a bit held off the turning axis: its moves are where its tip cuts, written as where A
        // turns to and how high the bit is
        let held = match (&self.wrap, job.rotary) {
            (Some((axis, _)), Some(r)) if !toolpath.across.is_empty() => {
                Some((usize::from(*axis == "Y"), r.diameter / 2.0))
            }
            _ => None,
        };
        // a square's corners or a board's edges swing out past the material's top: a rapid
        // that turns A lower than that lifts clear first, turns, and comes back down
        let corners = match (&self.wrap, job.rotary) {
            (Some((axis, _)), Some(_)) if job.turning_gap() > job.clearance + 1e-9 => {
                Some((usize::from(*axis == "Y"), job.turning_gap()))
            }
            _ => None,
        };
        // (kind, to, laser power, across)
        let mut lines = Vec::with_capacity(toolpath.moves.len());
        let mut from = at;
        for (i, m) in toolpath.moves.iter().enumerate() {
            let across = toolpath.across.get(i).copied().unwrap_or(0.0);
            let mut to = m.to;
            if let Some((round, radius)) = held {
                let (w, z) = cut_craft_core::rotary_axis::held_at(radius, to[round], to.z, across);
                to[round] = w;
                to.z = z;
            }
            if m.kind != MoveKind::Rapid && from.z > gap + 1e-9 && to.z < gap {
                let down = from.lerp(to, (from.z - gap) / (from.z - to.z));
                lines.push((MoveKind::Rapid, down, 0.0, across));
            }
            if let Some((round, high)) = corners
                && m.kind == MoveKind::Rapid
                && (to[round] - from[round]).abs() > 1e-9
                && from.z.min(to.z) < high - 1e-9
            {
                // straight up where it is, held as it is; across and round only once clear
                lines.push((
                    MoveKind::Rapid,
                    from.with_z(from.z.max(high)),
                    0.0,
                    held_across,
                ));
                lines.push((MoveKind::Rapid, to.with_z(to.z.max(high)), 0.0, across));
            }
            let power = toolpath.power.get(i).copied().unwrap_or(0.0);
            lines.push((m.kind, to, power, across));
            from = to;
            held_across = across;
        }
        let line = |&(kind, to, power, across): &(MoveKind, DVec3, f64, f64)| {
            Step::Line(kind, to, power, across)
        };
        let Some(limits) = self.arc_limits(!toolpath.power.is_empty()) else {
            return lines.iter().map(line).collect();
        };
        // runs of cuts, and of plunges (a helix goes down in arcs at the plunge rate)
        let mut steps = Vec::with_capacity(lines.len());
        for run in lines.chunk_by(|a, b| a.0 == b.0) {
            let kind = run[0].0;
            if kind == MoveKind::Rapid {
                steps.extend(run.iter().map(line));
            } else {
                let cuts: Vec<DVec3> = run.iter().map(|l| l.1).collect();
                steps.extend(arcs::fit(at, &cuts, &limits).into_iter().map(|p| match p {
                    arcs::Piece::Line(to) => Step::Line(kind, to, 0.0, 0.0),
                    arcs::Piece::Arc(arc) => Step::Arc(kind, arc),
                }));
            }
            at = run[run.len() - 1].1;
        }
        steps
    }

    /// A drill bank's drill numbers as `[DL]` prints them.
    pub(super) fn drill_list_text(&self, numbers: &[u32]) -> String {
        let (prefix, separator, first) = &self.drill_list;
        let mut text = String::new();
        for (i, n) in numbers.iter().enumerate() {
            match i {
                0 => {}
                1 => text.push_str(first),
                _ => text.push_str(separator),
            }
            text.push_str(prefix);
            text.push_str(&n.to_string());
        }
        text
    }

    /// How A winds through the file, for a wrapping post told the machine's rotary.
    pub(super) fn winding(&self, job: &Job) -> Option<winding::Winding> {
        let (axis, _) = self.wrap.as_ref()?;
        let rotary = job.rotary?;
        let round = usize::from(*axis == "Y");
        Some(winding::Winding::new(
            self.rotary?,
            round,
            rotary.diameter,
            // clear of a square's corners or a board's edges as they swing round
            job.turning_gap(),
        ))
    }

    /// A flat X or Y as it prints: a wrapped axis in degrees round the cylinder.
    pub(super) fn wrapped(&self, job: &Job, label: &str, flat: f64) -> f64 {
        match (&self.wrap, job.rotary) {
            (Some((axis, _)), Some(r)) if *axis == label => {
                flat / (std::f64::consts::PI * r.diameter) * 360.0
            }
            _ => flat,
        }
    }

    /// What stays the same through the file: the material, the file, the time and the
    /// starting position.
    pub(super) fn job_values(&self, doc: &Document, file: &File) -> Values {
        let job = &doc.job;
        let safe = job.machine_z(job.clearance);
        let home = job.home();
        let (x, y, z) = (
            self.wrapped(job, "X", home.x),
            self.wrapped(job, "Y", home.y),
            job.machine_z(home.z),
        );
        let area = job.area();
        let mut v = Values::default();
        for (label, value) in [
            ("XH", x),
            ("YH", y),
            ("ZH", z),
            ("SAFEZ", safe),
            ("X", x),
            ("Y", y),
            ("Z", z),
            ("XLENGTH", job.width),
            ("YLENGTH", job.height),
            ("ZLENGTH", job.thickness),
            ("XMIN", area.min.x),
            ("XMAX", area.max.x),
            ("YMIN", area.min.y),
            ("YMAX", area.max.y),
            ("ZMIN", job.machine_z(-job.thickness)),
            ("ZMAX", job.machine_z(0.0)),
            ("WRAP_DIA", job.rotary.map_or(0.0, |r| r.diameter)),
        ] {
            v.numbers.insert(label, value);
        }
        // a bit change's position: never below the safe Z, so going there clears the work
        if let Some([x, y, z]) = self.park {
            v.numbers.insert("PARKX", x);
            v.numbers.insert("PARKY", y);
            v.numbers.insert("PARKZ", job.machine_z(z).max(safe));
        }
        v.numbers.insert("SPINUP", self.spin_up);
        v.texts
            .insert("SPINUP_SECONDS", super::seconds(self.spin_up));
        let text = |s: Option<&std::ffi::OsStr>| s.map(|s| s.to_string_lossy().into_owned());
        let shown = |p: Option<&Path>| p.map(|p| p.display().to_string());
        for (label, value) in [
            ("TP_FILENAME", text(file.path.file_stem())),
            ("TP_DIR", shown(file.path.parent())),
            ("TP_EXT", text(file.path.extension())),
            ("PATHNAME", shown(Some(file.path))),
            ("FILE_NAME", text(file.job.and_then(Path::file_stem))),
            ("FILE_PATHNAME", shown(file.job)),
            ("DATE", Some(file.made.format("%A %B %d %Y").to_string())),
            ("TIME", Some(file.made.format("%I:%M %p").to_string())),
            ("Z_ORIGIN", Some(z_origin(job).into())),
            ("XY_ORIGIN", Some(xy_origin(job.datum).into())),
            (
                "UNITS",
                Some(if self.inches { "INCHES" } else { "MM" }.into()),
            ),
            (
                "PRODUCT",
                Some(concat!("Cut Craft ", env!("CARGO_PKG_VERSION")).into()),
            ),
            ("POST_NAME", Some(self.name.clone())),
            // one line, so a note can never break out of the comment it is printed in
            ("FILE_NOTES", Some(notes_lines(doc, file).join(" / "))),
            ("TOOLPATH_NOTES", None),
            ("TOOL_NOTES", None),
        ] {
            v.texts.insert(label, value.unwrap_or_default());
        }
        v
    }
}
