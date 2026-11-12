//! Create Shape: a 3D component raised over the selected closed vectors, rising from their
//! edges as a round dome, an angled slope, an S, a flat top, or (Custom) the profile of an
//! open vector selected with them. A ring between two vectors rises from both edges. Scale
//! sizes the profile to a height, Total Height the whole shape with its base.

use cut_craft_core::component::{Combine, Component};
use cut_craft_core::document::{Document, EntityId};
use cut_craft_core::geometry::{Path, Point, Rect, inside, polyline_distance};
use cut_craft_core::{Change, Command, CommandError, Meta, ensure_named};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct CreateShape;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Profile {
    /// Rises at the angle from the edge and rounds over the top
    #[default]
    Round,
    /// Rises at the angle all the way
    Angular,
    /// Rises in an S, curving smoothly out of the edge and over the top
    Smooth,
    /// One height all over
    Flat,
    /// The open vector selected with the closed ones, drawn upright: its left end at the
    /// edge, its right end at the middle
    Custom,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    /// What the new component is called in the Components list
    pub name: String,
    /// The shape of the sides as they rise from the edge
    pub profile: Profile,
    /// How steeply round and angular sides rise from the edge
    #[schemars(extend("x-unit" = "°"), range(min = 1, max = 89))]
    pub angle: f64,
    /// The flat top's height; round and angular shapes stop rising here (0: no limit)
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub height: f64,
    /// Smooth shapes: how pronounced the S is, from a straight slope (0) to steep in the middle
    #[serde(default = "half")]
    #[schemars(range(min = 0, max = 1), extend("x-show-when" = {"profile": "smooth"}))]
    pub smoothing: f64,
    /// Added everywhere inside the shape
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub base: f64,
    /// The profile scaled up or down to this height, its shape kept (0: as it rises)
    #[serde(default)]
    #[schemars(title = "Scale", extend("x-unit" = "mm"), range(min = 0))]
    pub scale: f64,
    /// The whole shape, base and all, scaled to this height (0: as made)
    #[serde(default)]
    #[schemars(title = "Total height", extend("x-unit" = "mm"), range(min = 0))]
    pub total_height: f64,
    /// Inside corners rise as straight, mitred slopes to a sharp crease, not curved in
    #[serde(default)]
    pub preserve_internal_corners: bool,
    /// Vectors inside the outer ones are the top, not holes: the profile rises from the
    /// outer edge to them
    #[serde(default)]
    pub blend_to_inner_vectors: bool,
    pub combine: Combine,
    /// The shape being edited (E on it): it is made again from the outlines it keeps
    #[schemars(skip)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub replaces: Option<EntityId>,
}

fn half() -> f64 {
    0.5
}

impl Default for Params {
    fn default() -> Self {
        Self {
            name: "Shape".into(),
            profile: Profile::Round,
            angle: 45.0,
            height: 0.0,
            smoothing: 0.5,
            base: 0.0,
            scale: 0.0,
            total_height: 0.0,
            preserve_internal_corners: false,
            blend_to_inner_vectors: false,
            combine: Combine::Add,
            replaces: None,
        }
    }
}

/// `component`'s profile scaled as `p` asks (Scale sizes the profile, Total Height the
/// profile and base together), then the base added under it.
fn scaled(component: &mut Component, p: &Params) {
    let top = component
        .heights
        .iter()
        .copied()
        .filter(|h| !h.is_nan())
        .fold(0.0, f64::max);
    let factor = if p.total_height > 0.0 && top > 0.0 {
        ((p.total_height - p.base) / top).max(0.0)
    } else if p.scale > 0.0 && top > 0.0 {
        p.scale / top
    } else {
        1.0
    };
    for height in component.heights.iter_mut().filter(|h| !h.is_nan()) {
        *height = *height * factor + p.base;
    }
}

/// How far `at` is in from the closed `ring`, inside corners mitred: where its nearest
/// point is a corner (inside the shape, only an inside corner can be), the distance to the
/// farther of the two sides' lines, so the two slopes meet in a straight crease.
fn mitred_distance(ring: &[Point], at: Point) -> f64 {
    if ring.len() < 3 {
        return polyline_distance(ring, at);
    }
    // the ring without a repeated closing point
    let count = if ring[0].distance(ring[ring.len() - 1]) < 1e-12 {
        ring.len() - 1
    } else {
        ring.len()
    };
    let corner = |k: usize| ring[k % count];
    // the nearest distance, and the corner it is to (None: to the middle of a side)
    let mut best = (f64::INFINITY, None);
    for side in 0..count {
        let (from, along) = (corner(side), corner(side + 1) - corner(side));
        let share = if along.length_squared() > 0.0 {
            ((at - from).dot(along) / along.length_squared()).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let far = (from + along * share).distance(at);
        if far < best.0 {
            let end = if share <= 0.0 {
                Some(side)
            } else if share >= 1.0 {
                Some((side + 1) % count)
            } else {
                None
            };
            best = (far, end);
        }
    }
    let Some(k) = best.1 else {
        return best.0;
    };
    let to_line =
        |from: Point, to: Point| (at - from).perp_dot((to - from).normalize_or_zero()).abs();
    let (before, here, after) = (corner(k + count - 1), corner(k), corner(k + 1));
    to_line(before, here).max(to_line(here, after))
}

/// The outlines the shape rises over: the shape's own when `p` edits one (with its id), else
/// the selected closed vectors and the first open one (a custom profile).
fn outlines(doc: &Document, p: &Params) -> (Option<EntityId>, Vec<Path>) {
    let kept = p.replaces.and_then(|id| {
        let shape = doc.components.get(&id)?.shape.as_ref()?;
        Some((id, Vec::<Path>::deserialize(&shape["paths"]).ok()?))
    });
    if let Some((id, paths)) = kept {
        return (Some(id), paths);
    }
    let (_, closed) = doc.selected_paths(|path| path.closed);
    let (_, open) = doc.selected_paths(|path| !path.closed);
    (
        None,
        closed.into_iter().chain(open.into_iter().take(1)).collect(),
    )
}

/// Each cell's distance in from the edge of `rings` (`p` says whether inside corners are
/// mitred). Blending to inner vectors, the rings inside another are the top: a cell's share
/// of the way from the outer edge to them, times the widest gap between the two.
fn inward(name: &str, p: &Params, rings: &[Vec<Point>], bounds: Rect, cell: f64) -> Component {
    let edge = |ring: &[Point], at: Point| {
        if p.preserve_internal_corners {
            mitred_distance(ring, at)
        } else {
            polyline_distance(ring, at)
        }
    };
    let nearest = |rings: &[Vec<Point>], at: Point| {
        rings
            .iter()
            .map(|ring| edge(ring, at))
            .fold(f64::INFINITY, f64::min)
    };
    let within_another = |i: usize| {
        rings
            .iter()
            .enumerate()
            .any(|(j, o)| j != i && inside(std::slice::from_ref(o), rings[i][0]))
    };
    let (mut outer, mut inner) = (vec![], vec![]);
    for (i, ring) in rings.iter().enumerate() {
        if p.blend_to_inner_vectors && within_another(i) {
            inner.push(ring.clone());
        } else {
            outer.push(ring.clone());
        }
    }
    if inner.is_empty() {
        return Component::from_fn(name, p.combine, bounds, cell, |at| {
            inside(rings, at).then(|| nearest(rings, at))
        });
    }
    let gap = outer
        .iter()
        .flatten()
        .map(|&q| nearest(&inner, q))
        .fold(0.0, f64::max);
    Component::from_fn(name, p.combine, bounds, cell, |at| {
        if !inside(&outer, at) {
            return None;
        }
        if inner.iter().any(|r| inside(std::slice::from_ref(r), at)) {
            return Some(gap);
        }
        let (out, inn) = (nearest(&outer, at), nearest(&inner, at));
        Some(gap * out / (out + inn).max(1e-12))
    })
}

impl Command for CreateShape {
    const ID: &'static str = "create_shape";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Create Shape",
            group: "Modeling",
            icon: "",
            shortcut: "",
        }
    }

    /// On a selected shape Create Shape made, its own settings, so the form edits it.
    fn initial_params(doc: &Document) -> Params {
        doc.selected_components
            .iter()
            .rev()
            .find_map(|&id| {
                let kept = doc.components.get(&id)?.shape.as_ref()?;
                let p = Params::deserialize(&kept["params"]).ok()?;
                Some(Params {
                    replaces: Some(id),
                    ..p
                })
            })
            .unwrap_or_default()
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        let name = ensure_named(&p.name)?;
        if !(1.0..=89.0).contains(&p.angle) {
            return Err(CommandError::InvalidParams(
                "set the angle between 1° and 89°".into(),
            ));
        }
        if p.profile == Profile::Flat && p.height + p.base + p.total_height <= 0.0 {
            return Err(CommandError::InvalidParams(
                "a flat shape needs a height. Set the height to more than 0".into(),
            ));
        }
        let (editing, all) = outlines(doc, p);
        let paths: Vec<&Path> = all.iter().filter(|q| q.closed).collect();
        let custom = if p.profile == Profile::Custom {
            let section = all
                .iter()
                .find(|q| !q.closed)
                .and_then(cmd_two_rail_sweep::Section::new);
            Some(section.ok_or_else(|| {
                CommandError::Failed(
                    "select an open vector for the custom profile with the closed ones".into(),
                )
            })?)
        } else {
            None
        };
        let Some(bounds) = paths.iter().map(|q| q.bounds()).reduce(Rect::union) else {
            return Err(CommandError::Failed(
                "select the closed vectors to shape".into(),
            ));
        };
        let cell = doc.job.model_cell();
        let rings: Vec<Vec<Point>> = paths.iter().map(|path| path.flatten(cell / 4.0)).collect();
        let mut component = inward(&name, p, &rings, bounds, cell);
        let deepest = component.heights.iter().fold(0.0, |a: f64, &d| a.max(d));
        let (sin, tan) = (p.angle.to_radians().sin(), p.angle.to_radians().tan());
        let rise = |d: f64| match p.profile {
            Profile::Angular => d * tan,
            // a circular arc meeting the edge at the angle, its top over the deepest point
            Profile::Round => {
                let radius = deepest / sin;
                (radius * radius - (deepest - d).powi(2)).max(0.0).sqrt() - deepest / tan
            }
            // [inf] the angular rise to the deepest point, eased in and out: t^k / (t^k +
            // (1 - t)^k), k from 1 (straight) to 4 as the smoothing goes from 0 to 1
            Profile::Smooth => {
                let along = if deepest > 0.0 { d / deepest } else { 0.0 };
                let ease = 1.0 + 3.0 * p.smoothing.clamp(0.0, 1.0);
                let (rising, left) = (along.powf(ease), (1.0 - along).powf(ease));
                let share = if rising + left > 0.0 {
                    rising / (rising + left)
                } else {
                    0.0
                };
                deepest * tan * share
            }
            Profile::Flat => p.height,
            Profile::Custom => {
                let along = if deepest > 0.0 { d / deepest } else { 0.0 };
                custom.as_ref().map_or(0.0, |section| {
                    cmd_two_rail_sweep::height(
                        &[(0.0, section.clone())],
                        0.0,
                        along,
                        (1.0, false, false),
                    )
                })
            }
        };
        let limit = if p.height > 0.0 {
            p.height
        } else {
            f64::INFINITY
        };
        for height in component.heights.iter_mut().filter(|h| !h.is_nan()) {
            *height = rise(*height).min(limit);
        }
        scaled(&mut component, p);
        let kept = Params {
            replaces: None,
            ..p.clone()
        };
        component.shape = Some(serde_json::json!({ "params": kept, "paths": all }));
        let mut c = Change::builder(doc);
        match editing {
            Some(id) => {
                let old = &doc.components[&id];
                component.level = old.level;
                component.group.clone_from(&old.group);
                component.visible = old.visible;
                component.colour = old.colour;
                component.shaping = old.shaping.clone();
                c.replace_component(id, component);
            }
            None => {
                c.add_component(component);
            }
        }
        Ok(c.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::geometry::{Path, Point};
    use cut_craft_core::testing::{document_with, square};

    cut_craft_core::contract_tests!(CreateShape);

    #[test]
    fn a_custom_profile_scale_and_total_height() {
        // a ramp rising 6 from the edge to the middle
        let ramp = Path::polyline(&[Point::new(0.0, 200.0), Point::new(10.0, 206.0)], false);
        let p = Params {
            profile: Profile::Custom,
            ..Params::default()
        };
        let c = shape(vec![square(0.0, 0.0, 40.0), ramp.clone()], &p).unwrap();
        assert!(
            near(height(&c, 20.0, 20.0), 6.0, 0.3),
            "the middle: {}",
            height(&c, 20.0, 20.0)
        );
        assert!(
            near(height(&c, 10.0, 20.0), 3.0, 0.4),
            "half way in: {}",
            height(&c, 10.0, 20.0)
        );
        let scaled = shape(
            vec![square(0.0, 0.0, 40.0), ramp.clone()],
            &Params {
                scale: 12.0,
                ..p.clone()
            },
        )
        .unwrap();
        assert!(near(height(&scaled, 20.0, 20.0), 12.0, 0.3));
        let total = shape(
            vec![square(0.0, 0.0, 40.0), ramp],
            &Params {
                total_height: 10.0,
                base: 4.0,
                ..p.clone()
            },
        )
        .unwrap();
        assert!(
            near(height(&total, 20.0, 20.0), 10.0, 0.3),
            "base and profile: 10"
        );
        assert!(
            near(height(&total, 0.5, 20.0), 4.0, 0.5),
            "the base at the edge"
        );
        assert!(
            shape(vec![square(0.0, 0.0, 40.0)], &p).is_err(),
            "needs its profile"
        );
    }

    #[test]
    fn a_ring_between_two_vectors_rises_from_both_edges() {
        let p = Params {
            profile: Profile::Angular,
            ..Params::default()
        };
        let ring = shape(vec![square(0.0, 0.0, 40.0), square(10.0, 10.0, 20.0)], &p).unwrap();
        assert!(
            ring.height_at(Point::new(20.0, 20.0))
                .is_none_or(f64::is_nan),
            "the hole"
        );
        // 5 in from both the outer and the inner edge: the ridge
        let ridge = height(&ring, 5.0, 20.0);
        assert!(ridge > height(&ring, 2.0, 20.0) && ridge > height(&ring, 8.0, 20.0));
    }

    /// The component made over `paths` with `params`, or the error.
    fn shape(paths: Vec<Path>, params: &Params) -> Result<Component, String> {
        let mut doc = document_with(paths);
        let change = CreateShape::run(&doc, params).map_err(|e| e.to_string())?;
        doc.apply(&change).unwrap();
        assert_eq!(doc.components.len(), 1);
        Ok(doc.components.into_values().next().unwrap())
    }

    fn height(c: &Component, x: f64, y: f64) -> f64 {
        c.height_at(Point::new(x, y)).expect("inside the shape")
    }

    fn near(a: f64, b: f64, within: f64) -> bool {
        (a - b).abs() <= within
    }

    #[test]
    fn a_smooth_shape_rises_in_an_s() {
        let p = |profile| Params {
            profile,
            ..Params::default()
        };
        let smooth = shape(vec![square(0.0, 0.0, 40.0)], &p(Profile::Smooth)).unwrap();
        let angular = shape(vec![square(0.0, 0.0, 40.0)], &p(Profile::Angular)).unwrap();
        // 20 in from every edge is the deepest point: the same height as the angular shape
        let (top_s, top_a) = (height(&smooth, 20.0, 20.0), height(&angular, 20.0, 20.0));
        assert!(near(top_s, top_a, 1.0), "{top_s} vs {top_a}");
        // near the edge it starts flatter than the straight slope, and rises smoothly
        assert!(height(&smooth, 2.0, 20.0) < height(&angular, 2.0, 20.0));
        let ups: Vec<f64> = (1..20)
            .map(|x| height(&smooth, f64::from(x), 20.0))
            .collect();
        assert!(ups.windows(2).all(|w| w[1] >= w[0] - 1e-9), "{ups:?}");
    }

    #[test]
    fn angular_shapes_rise_at_their_angle_from_the_edge() {
        let params = Params {
            profile: Profile::Angular,
            ..Params::default()
        };
        let c = shape(vec![square(0.0, 0.0, 100.0)], &params).unwrap();
        // half a cell of distance error either way
        assert!(
            near(height(&c, 50.0, 50.0), 50.0, 0.5),
            "{}",
            height(&c, 50.0, 50.0)
        );
        assert!(near(height(&c, 10.0, 50.0), 10.0, 0.5));
        assert_eq!(c.name, "Shape");
    }

    #[test]
    fn round_shapes_rise_at_the_edge_angle_and_round_over_the_top() {
        let params = Params {
            angle: 60.0,
            ..Params::default()
        };
        let c = shape(vec![Path::circle(Point::new(50.0, 50.0), 50.0)], &params).unwrap();
        // a circular arc meeting the edge at 60 degrees peaks at radius x tan(30 degrees)
        let peak = 50.0 * 30f64.to_radians().tan();
        assert!(
            near(height(&c, 50.0, 50.0), peak, 0.5),
            "{}",
            height(&c, 50.0, 50.0)
        );
        assert!(height(&c, 50.0, 50.0) > height(&c, 80.0, 50.0));
    }

    #[test]
    fn the_height_limit_flattens_the_top_and_the_base_raises_it_all() {
        let params = Params {
            profile: Profile::Angular,
            height: 20.0,
            base: 5.0,
            ..Params::default()
        };
        let c = shape(vec![square(0.0, 0.0, 100.0)], &params).unwrap();
        assert!(near(height(&c, 50.0, 50.0), 25.0, 1e-9));
        assert!(near(height(&c, 10.0, 50.0), 15.0, 0.5));
    }

    #[test]
    fn flat_shapes_need_a_height() {
        let flat = Params {
            profile: Profile::Flat,
            ..Params::default()
        };
        let why = shape(vec![square(0.0, 0.0, 100.0)], &flat).unwrap_err();
        assert!(why.contains("height"), "{why}");
        let c = shape(
            vec![square(0.0, 0.0, 100.0)],
            &Params {
                height: 8.0,
                ..flat
            },
        )
        .unwrap();
        assert!(near(height(&c, 50.0, 50.0), 8.0, 1e-9));
    }

    #[test]
    fn holes_stay_empty_and_the_combine_mode_is_kept() {
        let params = Params {
            combine: Combine::Subtract,
            ..Params::default()
        };
        let c = shape(
            vec![square(0.0, 0.0, 100.0), square(40.0, 40.0, 20.0)],
            &params,
        )
        .unwrap();
        assert_eq!(c.height_at(Point::new(50.0, 50.0)), None);
        assert_eq!(c.combine, Combine::Subtract);
    }

    #[test]
    fn shapes_are_as_fine_as_the_job_modeling_resolution() {
        use cut_craft_core::document::Resolution;
        let cell = |resolution| {
            let mut doc = document_with(vec![square(0.0, 0.0, 100.0)]);
            doc.job.resolution = resolution;
            cut_craft_core::testing::run_on::<CreateShape>(&mut doc, &Params::default());
            doc.components.into_values().next().unwrap().cell
        };
        let standard = cell(Resolution::Standard);
        assert!((standard - cut_craft_core::document::Job::default().model_cell()).abs() < 1e-12);
        assert!((cell(Resolution::High) - standard / 3f64.sqrt()).abs() < 1e-12);
    }

    #[test]
    fn needs_closed_vectors() {
        let open = Path::polyline(&[Point::new(0.0, 0.0), Point::new(10.0, 0.0)], false);
        let why = shape(vec![open], &Params::default()).unwrap_err();
        assert!(why.contains("closed"), "{why}");
    }

    /// An L: a 40 square with its top right quarter cut away, the inside corner at (20, 20).
    fn ell() -> Path {
        Path::polyline(
            &[
                Point::new(0.0, 0.0),
                Point::new(40.0, 0.0),
                Point::new(40.0, 20.0),
                Point::new(20.0, 20.0),
                Point::new(20.0, 40.0),
                Point::new(0.0, 40.0),
            ],
            true,
        )
    }

    #[test]
    fn preserved_internal_corners_rise_straight_to_a_crease() {
        let p = |preserve| Params {
            profile: Profile::Angular,
            preserve_internal_corners: preserve,
            ..Params::default()
        };
        let round = shape(vec![ell()], &p(false)).unwrap();
        let mitred = shape(vec![ell()], &p(true)).unwrap();
        // 3 from both sides of the inside corner: round, 3 x root 2 from the corner; mitred, 3
        let (r, m) = (height(&round, 17.0, 17.0), height(&mitred, 17.0, 17.0));
        assert!(near(r, 18f64.sqrt(), 0.5), "round {r}");
        assert!(near(m, 3.0, 0.5), "mitred {m}");
        // away from the corner the two are the same
        assert!(near(
            height(&round, 5.0, 10.0),
            height(&mitred, 5.0, 10.0),
            1e-9
        ));
    }

    #[test]
    fn blending_to_inner_vectors_makes_them_the_top() {
        let p = |blend| Params {
            profile: Profile::Angular,
            blend_to_inner_vectors: blend,
            ..Params::default()
        };
        let both = || vec![square(0.0, 0.0, 40.0), square(15.0, 15.0, 10.0)];
        let ring = shape(both(), &p(false)).unwrap();
        assert!(
            ring.height_at(Point::new(20.0, 20.0))
                .is_none_or(f64::is_nan),
            "a hole"
        );
        let blended = shape(both(), &p(true)).unwrap();
        let top = height(&blended, 20.0, 20.0);
        assert!(near(top, height(&blended, 16.0, 16.0), 1e-9), "flat on top");
        let ups: Vec<f64> = [1.0, 5.0, 10.0, 14.0]
            .iter()
            .map(|&x| height(&blended, x, 20.0))
            .collect();
        assert!(ups.windows(2).all(|w| w[1] > w[0]), "rises inward: {ups:?}");
        assert!(top > ups[3] - 1e-9);
    }

    #[test]
    fn a_selected_shape_opens_its_settings_and_is_remade_in_place() {
        let mut doc = document_with(vec![square(0.0, 0.0, 100.0)]);
        let angular = Params {
            profile: Profile::Angular,
            ..Params::default()
        };
        cut_craft_core::testing::run_on::<CreateShape>(&mut doc, &angular);
        let id = *doc.components.keys().next().unwrap();
        // nothing selected: the form opens on new settings
        assert_eq!(CreateShape::initial_params(&doc).replaces, None);
        doc.selected_components = [id].into();
        doc.selection = cut_craft_core::selection::Selection::default();
        let p = CreateShape::initial_params(&doc);
        assert_eq!((p.replaces, p.profile), (Some(id), Profile::Angular));
        // a lower angle, the vectors no longer selected: made again from its own outline
        cut_craft_core::testing::run_on::<CreateShape>(&mut doc, &Params { angle: 20.0, ..p });
        assert_eq!(doc.components.len(), 1);
        let c = &doc.components[&id];
        let want = 50.0 * 20f64.to_radians().tan();
        assert!(
            near(height(c, 50.0, 50.0), want, 0.5),
            "{}",
            height(c, 50.0, 50.0)
        );
    }
}
