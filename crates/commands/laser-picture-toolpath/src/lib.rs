//! Laser Picture: a laser burns a picture in lines across it, its power following how dark
//! the picture is, or (dithered) switching full on and off to give the tones.

use cmd_laser_cut_fill_toolpath::laser;
use cut_craft_core::bitmap::Bitmap;
use cut_craft_core::document::{Document, EntityId};
use cut_craft_core::geometry::{Path, Point, Rect};
use cut_craft_core::toolpath::{Move, MoveKind, Tool, cut_points, hatch_lines, made_by};
use cut_craft_core::{Change, Command, CommandError, Meta, ensure_positive};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct LaserPictureToolpath;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Strategy {
    /// One set of parallel lines
    #[default]
    Raster,
    /// Two sets at right angles
    Hatch,
    /// Along the selected vectors
    Selected,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    pub name: String,
    /// Of the laser's maximum, where the picture is black
    #[schemars(extend("x-unit" = "%"), range(min = 0, max = 100))]
    #[schemars(extend("x-group" = "Laser"))]
    pub power: f64,
    #[schemars(range(min = 1))]
    #[schemars(extend("x-group" = "Laser"))]
    pub passes: u32,
    #[schemars(extend("x-group" = "Picture"))]
    pub strategy: Strategy,
    /// Full power or off, spread to give the tones, for materials that only burn or don't
    #[schemars(extend("x-group" = "Picture"))]
    pub dither_image: bool,
    /// Between lines, in kerf widths (1 densest to 30 sparsest)
    #[schemars(range(min = 1, max = 30))]
    #[schemars(extend("x-group" = "Picture"))]
    pub line_spacing: f64,
    #[schemars(extend("x-unit" = "°"))]
    #[schemars(extend("x-group" = "Picture"))]
    pub line_angle: f64,
    /// The laser: its diameter is the kerf, its feed rate the move speed
    pub tool: Tool,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            name: "Laser Picture".into(),
            power: 60.0,
            passes: 1,
            strategy: Strategy::Raster,
            dither_image: false,
            line_spacing: 1.0,
            line_angle: 0.0,
            tool: laser(),
        }
    }
}

impl Params {
    /// The lines to burn along (and the vectors they come from): across `rect` `spacing`
    /// apart, or the selected vectors.
    fn lines(
        &self,
        doc: &Document,
        rect: Rect,
        spacing: f64,
    ) -> Result<(Vec<EntityId>, Vec<Vec<Point>>), CommandError> {
        Ok(if self.strategy == Strategy::Selected {
            let (ids, paths) = doc.selected_paths(|_| true);
            if ids.is_empty() {
                return Err(CommandError::Failed(
                    "select the vectors to burn along".into(),
                ));
            }
            let lines = paths
                .iter()
                .map(|path| {
                    let mut points = cut_points(path);
                    if path.closed {
                        points.push(points[0]);
                    }
                    points
                })
                .collect();
            (ids, lines)
        } else {
            let r = rect;
            let outline = Path::polyline(
                &[
                    r.min,
                    Point::new(r.max.x, r.min.y),
                    r.max,
                    Point::new(r.min.x, r.max.y),
                ],
                true,
            );
            let mut angles = vec![self.line_angle];
            if self.strategy == Strategy::Hatch {
                angles.push(self.line_angle + 90.0);
            }
            let lines = angles
                .into_iter()
                .flat_map(|a| hatch_lines(std::slice::from_ref(&outline), spacing, a))
                .flatten()
                .enumerate()
                .map(|(i, (a, b))| if i % 2 == 0 { vec![a, b] } else { vec![b, a] })
                .collect();
            (vec![], lines)
        })
    }

    /// Moves along each line sampled every `step` over the picture, and the power of each.
    fn burn(
        &self,
        lines: &[Vec<Point>],
        bitmap: &Bitmap,
        step: f64,
        safe: f64,
    ) -> (Vec<Move>, Vec<f64>) {
        let (mut moves, mut power) = (vec![], vec![]);
        for line in lines {
            // the tone still owed, carried along the line when dithering
            let mut owed = 0.0;
            let mut started = false;
            for w in line.windows(2) {
                #[expect(
                    clippy::cast_possible_truncation,
                    clippy::cast_sign_loss,
                    reason = "a count"
                )]
                let n = (w[0].distance(w[1]) / step).ceil().max(1.0) as u32;
                for k in 0..=n {
                    let at = w[0].lerp(w[1], f64::from(k) / f64::from(n));
                    let Some(dark) = bitmap.darkness(at) else {
                        continue;
                    };
                    let level = if self.dither_image {
                        owed += dark;
                        let on = owed >= 0.5;
                        if on {
                            owed -= 1.0;
                        }
                        if on { 1.0 } else { 0.0 }
                    } else {
                        dark
                    };
                    if !started {
                        moves.push(Move {
                            kind: MoveKind::Rapid,
                            to: at.extend(safe),
                        });
                        power.push(0.0);
                        moves.push(Move {
                            kind: MoveKind::Plunge,
                            to: at.extend(0.0),
                        });
                        power.push(0.0);
                        started = true;
                    }
                    // each move burns at the tone of the point it reaches
                    moves.push(Move {
                        kind: MoveKind::Cut,
                        to: at.extend(0.0),
                    });
                    power.push(self.power * level);
                }
            }
            if let Some(last) = moves.last().map(|m: &Move| m.to).filter(|_| started) {
                moves.push(Move {
                    kind: MoveKind::Rapid,
                    to: last.truncate().extend(safe),
                });
                power.push(0.0);
            }
        }
        (moves, power)
    }
}

impl Command for LaserPictureToolpath {
    const ID: &'static str = "laser_picture_toolpath";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Laser Picture",
            group: "Toolpath Operations",
            icon: "toolpaths/laser_picture.png",
            shortcut: "",
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        if !(p.power > 0.0 && p.power <= 100.0) {
            return Err(CommandError::InvalidParams(
                "power must be more than 0 and at most 100 %".into(),
            ));
        }
        if p.passes == 0 {
            return Err(CommandError::InvalidParams(
                "it needs at least 1 pass".into(),
            ));
        }
        ensure_positive("kerf", p.tool.diameter)?;
        ensure_positive("line spacing", p.line_spacing)?;
        // the bitmap picked last, else the newest
        let Some(bitmap) = doc.chosen_bitmap() else {
            return Err(CommandError::Failed(
                "import the picture to burn first".into(),
            ));
        };
        let spacing = p.tool.diameter * p.line_spacing.clamp(1.0, 30.0);
        let (ids, lines) = p.lines(doc, bitmap.rect, spacing)?;
        let size = bitmap.rect.max - bitmap.rect.min;
        #[expect(clippy::cast_precision_loss, reason = "pixel counts")]
        let step = (size.x / bitmap.columns as f64)
            .min(size.y / bitmap.rows as f64)
            .min(spacing);
        let safe = doc.job.clearance;
        let (moves, power) = p.burn(&lines, bitmap, step, safe);
        if moves.is_empty() {
            return Err(CommandError::Failed(
                "nothing to burn on the picture".into(),
            ));
        }
        let count = moves.len();
        let moves: Vec<Move> = (0..p.passes).flat_map(|_| moves.iter().copied()).collect();
        let power: Vec<f64> = power
            .iter()
            .copied()
            .cycle()
            .take(count * p.passes as usize)
            .collect();
        let mut toolpath = made_by::<Self>(p, p.name.clone(), p.tool.clone(), ids, moves);
        toolpath.power = power;
        let mut c = Change::builder(doc);
        c.add_toolpath(toolpath);
        Ok(c.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::toolpath::Toolpath;

    /// A 10 x 2 mm picture: black, then 25 % grey, then white, left to right.
    fn with_picture() -> Document {
        let mut doc = cut_craft_core::testing::sample_document();
        let mut c = Change::builder(&doc);
        c.add_bitmap(Bitmap {
            name: "tones".into(),
            columns: 30,
            rows: 2,
            grey: (0..60).map(|i| [0, 191, 255][(i % 30) / 10]).collect(),
            rect: Rect::new(Point::ZERO, Point::new(10.0, 2.0)),
            depth: Vec::new(),
        });
        let change = c.finish();
        doc.apply(&change).unwrap();
        doc
    }

    cut_craft_core::contract_tests!(LaserPictureToolpath, with_picture);

    fn burn(p: &Params) -> Toolpath {
        let mut doc = with_picture();
        let change = LaserPictureToolpath::run(&doc, p).unwrap();
        doc.apply(&change).unwrap();
        doc.toolpaths.into_values().next().unwrap()
    }

    /// Power along the first line at x.
    fn power_at(t: &Toolpath, x: f64) -> f64 {
        let y = t
            .moves
            .iter()
            .find(|m| m.kind == MoveKind::Cut)
            .unwrap()
            .to
            .y;
        t.moves
            .iter()
            .zip(&t.power)
            .find(|(m, _)| {
                m.kind == MoveKind::Cut && (m.to.y - y).abs() < 1e-9 && (m.to.x - x).abs() < 0.2
            })
            .map(|(_, &p)| p)
            .unwrap()
    }

    #[test]
    fn power_follows_the_darkness() {
        let t = burn(&Params::default());
        assert_eq!(t.power.len(), t.moves.len());
        assert!((power_at(&t, 1.0) - 60.0).abs() < 1e-6);
        assert!(
            (power_at(&t, 5.0) - 15.0).abs() < 1.0,
            "{}",
            power_at(&t, 5.0)
        );
        assert!(power_at(&t, 9.0) < 1e-6);
    }

    #[test]
    fn dithering_is_full_on_or_off_in_the_right_share() {
        let t = burn(&Params {
            dither_image: true,
            ..Params::default()
        });
        assert!(t.power.iter().all(|&p| p == 0.0 || (p - 60.0).abs() < 1e-9));
        let y = t
            .moves
            .iter()
            .find(|m| m.kind == MoveKind::Cut)
            .unwrap()
            .to
            .y;
        let grey: Vec<f64> = t
            .moves
            .iter()
            .zip(&t.power)
            .filter(|(m, _)| {
                m.kind == MoveKind::Cut && (m.to.y - y).abs() < 1e-9 && (4.0..6.0).contains(&m.to.x)
            })
            .map(|(_, &p)| p / 60.0)
            .collect();
        let share = grey.iter().sum::<f64>() / f64::from(u32::try_from(grey.len()).unwrap());
        assert!((share - 0.25).abs() < 0.1, "{share}");
    }

    #[test]
    fn lines_are_the_kerf_times_the_spacing_apart() {
        let t = burn(&Params {
            line_spacing: 4.0,
            ..Params::default()
        });
        let mut rows: Vec<f64> = t
            .moves
            .iter()
            .filter(|m| m.kind == MoveKind::Cut)
            .map(|m| m.to.y)
            .collect();
        rows.sort_by(f64::total_cmp);
        rows.dedup_by(|a, b| (*a - *b).abs() < 1e-9);
        assert!((rows[1] - rows[0] - 0.6).abs() < 1e-9, "{rows:?}");
    }
}
