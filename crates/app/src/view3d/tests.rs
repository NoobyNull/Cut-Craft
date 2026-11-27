use super::*;

#[test]
fn spinning_a_rotary_job_turns_it_round_its_own_axis() {
    // a cylinder along x: its axis stays put on the screen while its round side comes past
    let start = Orbit::default();
    let spun = start.spun_about(DVec3::X, 0.5, 90.0);
    let (axis_before, _) = start.project(DVec3::X);
    let (axis_after, _) = spun.project(DVec3::X);
    assert!(
        (axis_after - axis_before).length() < 1e-9,
        "the axis keeps its place"
    );
    // half a second at 90 degrees a second: 45 degrees round x
    let turned = start.towards_viewer().angle_between(spun.towards_viewer());
    assert!(turned > 1e-3, "the side facing the viewer moves");
    let round_x = |v: DVec3| DVec3::new(0.0, v.y, v.z).normalize();
    let a = round_x(start.towards_viewer()).angle_between(round_x(spun.towards_viewer()));
    assert!((a - 45f64.to_radians()).abs() < 1e-9, "{}", a.to_degrees());
    // along y the same, about y
    let spun = start.spun_about(DVec3::Y, 0.5, 90.0);
    assert!((spun.project(DVec3::Y).0 - start.project(DVec3::Y).0).length() < 1e-9);
    // about the vertical it is the flat job's spin
    assert_eq!(start.spun_about(DVec3::Z, 0.5, 90.0), start.spun(0.5, 90.0));
}

#[test]
fn spinning_turns_the_job_about_its_vertical_and_keeps_the_tilt() {
    let start = Orbit::default();
    let spun = start.spun(0.2, 30.0);
    assert!((spun.yaw - start.yaw - 6f64.to_radians()).abs() < 1e-9);
    assert!((spun.pitch - start.pitch).abs() < 1e-12, "the tilt is kept");
    // a long pause (the window hidden, a slow frame) is not a jump
    let jumped = start.spun(30.0, 30.0);
    assert!((jumped.yaw - start.yaw).abs() <= SPIN_LONGEST_STEP * 30f64.to_radians() + 1e-9);
    // round and round without growing without end
    let mut o = start;
    for _ in 0..10_000 {
        o = o.spun(0.1, 90.0);
    }
    assert!(o.yaw.abs() <= std::f64::consts::TAU, "{}", o.yaw);
}

#[test]
fn a_lithophane_is_white_where_nothing_is_left_and_the_material_where_it_is_whole() {
    use cut_craft_core::document::Job;
    let job = Job {
        thickness: 10.0,
        ..Job::default()
    };
    let mut m = cut_craft_core::simulation::Material::new(&job, 4);
    m.heights[0] = 0.0;
    m.heights[1] = m.bottom;
    m.heights[2] = m.bottom / 2.0;
    let colour = Color32::from_rgb(200, 100, 0);
    let lit = lit_through(m.clone(), colour, 0.5);
    assert_eq!(paint_of(&lit, 0), Some(colour));
    assert_eq!(paint_of(&lit, 1), Some(Color32::WHITE));
    let mid = paint_of(&lit, 2).unwrap();
    assert!(
        mid.g() > 100 && mid.b() > 0 && mid.b() < 255,
        "between: {mid:?}"
    );
    // more light: the half-thick cell is lighter still
    let lighter = paint_of(&lit_through(m, colour, 1.0), 2).unwrap();
    assert!(lighter.b() > mid.b(), "{lighter:?} vs {mid:?}");
}

#[test]
fn a_preview_is_painted_by_who_cut_each_cell_and_coarse_keeps_the_deepest_cells_paint() {
    let mut m = Material::new(
        &cut_craft_core::document::Job {
            width: 4.0,
            height: 4.0,
            ..cut_craft_core::document::Job::default()
        },
        4,
    );
    // cell 0 cut deepest by toolpath 3, cell 1 by toolpath 9 (unpainted)
    (m.heights[0], m.cut_by[0]) = (-2.0, 3);
    (m.heights[1], m.cut_by[1]) = (-1.0, 9);
    let red = Color32::from_rgb(255, 0, 0);
    let p = painted(m.clone(), |id| (id == 3).then_some(red));
    assert_eq!(paint_of(&p, 0), Some(red));
    assert_eq!(paint_of(&p, 1), None, "no fill");
    assert_eq!(paint_of(&p, 5), None, "uncut");
    assert!(
        painted(m.clone(), |_| None).paint.is_empty(),
        "nothing to paint"
    );
    let c = coarse(&p, 2);
    assert_eq!(
        (c.cut_by[0], paint_of(&c, 0)),
        (3, Some(red)),
        "the deepest cell's"
    );
}
use cut_craft_core::document::Job;
use cut_craft_core::geometry::Point;
use eframe::egui::pos2;

fn plain() -> Look {
    Look {
        colour: Color32::WHITE,
        sides: true,
        light: LIGHT,
        wrap: None,
        grain: None,
        shadows: false,
    }
}

fn area_of(t: &[DVec3; 3]) -> f64 {
    (t[1] - t[0]).cross(t[2] - t[0]).length() / 2.0
}

#[test]
fn a_see_through_look_draws_see_through_faces() {
    let face = [DVec3::ZERO, DVec3::X, DVec3::Y];
    let look = Look {
        colour: Color32::from_rgba_unmultiplied(255, 0, 0, 13),
        ..plain()
    };
    let orbit = Orbit::default();
    let mesh = triangles(
        &[(face, false)],
        |p| pos2(p.as_vec3().x, p.as_vec3().y),
        orbit,
        &look,
        Color32::RED,
    );
    assert_eq!(mesh.vertices.len(), 3);
    let c = mesh.vertices[0].color;
    assert_eq!(c.a(), 13, "{c:?}");
    assert!(c.r() > 0 && c.g() == 0, "red: {c:?}");
}

#[test]
fn a_model_is_split_at_the_material_edges_into_what_is_over_it_and_what_is_past() {
    use cut_craft_core::geometry::Rect;
    let area = Rect::new(Point::new(0.0, 0.0), Point::new(10.0, 10.0));
    let inside = [
        DVec3::new(1.0, 1.0, 0.0),
        DVec3::new(4.0, 1.0, 1.0),
        DVec3::new(1.0, 4.0, 2.0),
    ];
    let outside = [
        DVec3::new(11.0, 1.0, 0.0),
        DVec3::new(14.0, 1.0, 0.0),
        DVec3::new(11.0, 4.0, 0.0),
    ];
    // across the right edge, and across a corner
    let across = [
        DVec3::new(6.0, 2.0, 0.0),
        DVec3::new(14.0, 2.0, 0.0),
        DVec3::new(6.0, 6.0, 0.0),
    ];
    let corner = [
        DVec3::new(8.0, 8.0, 3.0),
        DVec3::new(12.0, 8.0, 3.0),
        DVec3::new(8.0, 12.0, 3.0),
    ];
    let faces = [
        (inside, true),
        (outside, false),
        (across, false),
        (corner, true),
    ];
    let (over, past) = split_at_edges(&faces, area);
    assert_eq!(
        over[0],
        (inside, true),
        "a face over the material is kept as it is"
    );
    assert!(past.contains(&outside), "a face past it goes whole");
    let within =
        |v: &DVec3| (-1e-9..=10.0 + 1e-9).contains(&v.x) && (-1e-9..=10.0 + 1e-9).contains(&v.y);
    assert!(over.iter().all(|(t, _)| t.iter().all(within)));
    assert!(
        past.iter().all(|t| !within(&((t[0] + t[1] + t[2]) / 3.0))),
        "every piece past is outside"
    );
    // nothing is lost or doubled, and the pieces face the way their face did
    let total: f64 = faces.iter().map(|(t, _)| area_of(t)).sum();
    let split: f64 =
        over.iter().map(|(t, _)| area_of(t)).sum::<f64>() + past.iter().map(area_of).sum::<f64>();
    assert!((total - split).abs() < 1e-9, "{total} vs {split}");
    let up = |t: &[DVec3; 3]| (t[1] - t[0]).cross(t[2] - t[0]).z > 0.0;
    assert!(over.iter().all(|(t, _)| up(t)) && past.iter().all(up));
    // the corner's piece over the material stays marked
    assert!(over.iter().filter(|(_, m)| *m).count() >= 2);
}

#[test]
fn grain_darkens_lines_along_x_that_shift_going_down_into_the_board() {
    let g = Grain {
        spacing: 4.0,
        depth: 0.3,
    };
    let on = g.at(DVec3::ZERO);
    assert!((on - 0.7).abs() < 1e-6, "a line is `depth` darker: {on}");
    let between = (1..40)
        .map(|i| g.at(DVec3::new(0.0, f64::from(i) * 0.1, 0.0)))
        .fold(0.0, f64::max);
    assert!(between > 0.99, "the wood between lines is not: {between}");
    let deeper = g.at(DVec3::new(0.0, 0.0, -2.0 / GRAIN_DEPTH_SHIFT));
    assert!(
        deeper > on + 0.2,
        "going down, the line moves off: {deeper}"
    );
    let coarse = g.on_cells(g.spacing).depth;
    assert!(
        coarse < g.depth / 2.0,
        "fades on cells too coarse for it: {coarse}"
    );
}

#[test]
fn a_vector_is_flat_at_the_top_of_what_it_outlines() {
    // the left half of the job raised 5: an outline along its edge, and a line from the
    // floor onto it, lie in one plane on top
    let job = Job::default();
    let mut m = Material::new(&job, 40);
    let edge = m.origin.x + 20.0 * m.cell;
    for row in 0..m.rows {
        for column in 0..20 {
            m.heights[row * m.columns + column] += 5.0;
        }
    }
    let (top, y) = (m.heights[0], m.origin.y + 3.3 * m.cell);
    for nudge in [-0.4, -1e-9, 0.0, 1e-9, 0.4] {
        let along = [
            Point::new(edge + nudge * m.cell, y),
            Point::new(edge, y + 5.0),
        ];
        assert_eq!(
            vector_plane(&m, &along),
            top,
            "{nudge} of a cell off the edge"
        );
    }
    let across = [
        Point::new(edge + 10.0 * m.cell, y),
        Point::new(edge - 2.0, y),
    ];
    assert_eq!(vector_plane(&m, &across), top);
    let floor = [Point::new(edge + 3.0 * m.cell, y)];
    assert_eq!(vector_plane(&m, &floor), m.at(floor[0]).unwrap());
    assert_eq!(
        vector_plane(&m, &[Point::new(-1e6, 0.0)]),
        0.0,
        "off the material"
    );
}

#[test]
fn to_world_undoes_project() {
    let orbit = Orbit {
        yaw: 0.7,
        pitch: 0.4,
        roll: 0.0,
    };
    let p = DVec3::new(3.0, -2.0, 5.0);
    let (screen, depth) = orbit.project(p);
    assert!(orbit.to_world(screen.extend(depth)).abs_diff_eq(p, 1e-9));
}

#[test]
fn hiding_the_sides_leaves_only_the_surface() {
    let area = Rect::from_min_max(pos2(0.0, 0.0), pos2(400.0, 300.0));
    let with = mesh(
        &slotted(),
        Orbit::default(),
        Framing::default(),
        &plain(),
        area,
    );
    let without = mesh(
        &slotted(),
        Orbit::default(),
        Framing::default(),
        &Look {
            sides: false,
            ..plain()
        },
        area,
    );
    assert!(without.indices.len() < with.indices.len());
}

#[test]
#[expect(clippy::cast_possible_truncation, reason = "small test coordinates")]
fn the_tool_is_an_upright_cylinder_on_its_tip() {
    // looking straight down: a circle; from the side: a rectangle as tall as the tool
    let down = |p: DVec3| Pos2::new(p.x as f32, -p.y as f32);
    let ring = turned_outline(down, DVec3::ZERO, &[[3.0, 0.0], [3.0, 12.0]]);
    assert!(
        ring.len() >= 12
            && ring
                .iter()
                .all(|p| (p.to_vec2().length() - 3.0).abs() < 1e-3)
    );
    let side = |p: DVec3| Pos2::new(p.x as f32, -p.z as f32);
    let outline = turned_outline(side, DVec3::ZERO, &[[3.0, 0.0], [3.0, 12.0]]);
    let (lo, hi) = outline.iter().fold((f32::MAX, f32::MIN), |(lo, hi), p| {
        (lo.min(p.y), hi.max(p.y))
    });
    assert!((hi - lo - 12.0).abs() < 1e-3, "{lo}..{hi}");
}

/// How wide and tall `parts` stand seen from the side, mm.
#[expect(clippy::cast_possible_truncation, reason = "small test coordinates")]
fn side_size(parts: &[Vec<[f64; 2]>]) -> Vec<(f32, f32)> {
    let side = |p: DVec3| Pos2::new(p.x as f32, -p.z as f32);
    parts
        .iter()
        .map(|rings| {
            let o = turned_outline(side, DVec3::ZERO, rings);
            let (x, y) = (o.iter().map(|p| p.x), o.iter().map(|p| p.y));
            let span = |v: Vec<f32>| {
                v.iter().copied().fold(f32::MIN, f32::max)
                    - v.iter().copied().fold(f32::MAX, f32::min)
            };
            (span(x.collect()), span(y.collect()))
        })
        .collect()
}

fn bit(
    shape: cut_craft_core::toolpath::ToolShape,
    diameter: f64,
) -> cut_craft_core::toolpath::Tool {
    cut_craft_core::toolpath::Tool {
        shape,
        diameter,
        ..cut_craft_core::toolpath::Tool::default()
    }
}

#[test]
fn the_previewed_tool_is_as_wide_as_the_bit_not_bigger() {
    // a 1/4" end mill on a 1/4" shank: 6.35 mm across all the way up
    let tool = cut_craft_core::toolpath::Tool {
        shank_diameter: 6.35,
        flute_length: 19.05,
        shank_length: 25.4,
        ..bit(cut_craft_core::toolpath::ToolShape::EndMill, 6.35)
    };
    let sizes = side_size(&tool_parts(&tool));
    assert_eq!(sizes.len(), 2, "cutting end and shank");
    for (wide, _) in &sizes {
        assert!((wide - 6.35).abs() < 0.01, "{sizes:?}");
    }
    let tall: f32 = sizes.iter().map(|s| s.1).sum();
    assert!((tall - 44.45).abs() < 0.01, "flutes and shank, {sizes:?}");
}

#[test]
#[expect(clippy::cast_possible_truncation, reason = "small test coordinates")]
fn a_v_bit_is_drawn_as_a_point_under_a_narrower_shank() {
    // 3/4" across its cone, 90°, on a 1/4" shank
    let tool = cut_craft_core::toolpath::Tool {
        angle: 90.0,
        shank_diameter: 6.35,
        ..bit(cut_craft_core::toolpath::ToolShape::VBit, 19.05)
    };
    let parts = tool_parts(&tool);
    let sizes = side_size(&parts);
    let (cone, shank) = (sizes[0], sizes[1]);
    assert!(
        (cone.0 - 19.05).abs() < 0.01 && (cone.1 - 9.525).abs() < 0.01,
        "{sizes:?}"
    );
    assert!((shank.0 - 6.35).abs() < 0.01, "{sizes:?}");
    // a point on the axis at the tip, not a flat bottom
    let side = |p: DVec3| Pos2::new(p.x as f32, -p.z as f32);
    let o = turned_outline(side, DVec3::ZERO, &parts[0]);
    let bottom: Vec<&Pos2> = o.iter().filter(|p| p.y.abs() < 1e-3).collect();
    assert!(bottom.iter().all(|p| p.x.abs() < 1e-3), "{bottom:?}");
}

#[test]
fn a_tool_with_no_size_draws_nothing() {
    assert_eq!(
        tool_parts(&bit(cut_craft_core::toolpath::ToolShape::EndMill, 0.0)),
        Vec::<Vec<[f64; 2]>>::new()
    );
}

#[test]
fn nothing_is_drawn_where_there_is_no_model() {
    let area = Rect::from_min_max(pos2(0.0, 0.0), pos2(400.0, 300.0));
    let full = mesh(
        &slotted(),
        Orbit::default(),
        Framing::default(),
        &plain(),
        area,
    );
    let mut half = slotted();
    let columns = half.columns;
    for (i, h) in half.heights.iter_mut().enumerate() {
        if i % columns < columns / 2 {
            *h = f64::NAN;
        }
    }
    let shown = mesh(&half, Orbit::default(), Framing::default(), &plain(), area);
    assert!(shown.indices.len() < full.indices.len() * 2 / 3);
    assert!(
        shown
            .vertices
            .iter()
            .all(|v| v.pos.x.is_finite() && v.pos.y.is_finite()),
        "no empty cell reaches the screen"
    );
    assert!(
        brightness(&half, columns / 2, 3, LIGHT).is_finite(),
        "an edge cell is lit"
    );
}

#[test]
fn the_model_is_solid_down_to_the_modeling_plane() {
    // 10 x 10 cells of 1 mm, the model a 4 x 4 block of them 5 high, nothing round it
    let job = Job {
        width: 10.0,
        height: 10.0,
        ..Job::default()
    };
    let mut m = Material::new(&job, 10);
    for (i, h) in m.heights.iter_mut().enumerate() {
        let (c, r) = (i % 10, i / 10);
        *h = if (3..7).contains(&c) && (3..7).contains(&r) {
            5.0
        } else {
            f64::NAN
        };
    }
    let tris = solid(&m, None);
    // its surface spans 3 x 3 quads: four walls of 3 quads, 2 triangles each, and the
    // bottom as 3 strips of 2
    assert_eq!(tris.len(), 4 * 3 * 2 + 3 * 2, "{}", tris.len());
    let middle = DVec3::new(5.0, 5.0, 2.5);
    for (t, _) in &tris {
        let normal = (t[1] - t[0]).cross(t[2] - t[0]);
        let centre = (t[0] + t[1] + t[2]) / 3.0;
        assert!(normal.dot(centre - middle) > 0.0, "faces out: {t:?}");
        assert!(
            t.iter().all(|p| (0.0..=5.0).contains(&p.z)),
            "plane to surface"
        );
    }
    assert!(
        tris.iter()
            .any(|(t, _)| t.iter().all(|p| p.z.abs() < 1e-12)),
        "a bottom"
    );
}

/// 100 x 100 mm in 1 mm cells, with a 5 mm deep slot from x = 40 to 60 running along y.
fn slotted() -> Material {
    let job = Job {
        width: 100.0,
        height: 100.0,
        ..Job::default()
    };
    let mut m = Material::new(&job, 100);
    for row in 0..m.rows {
        for column in 40..60 {
            m.heights[row * m.columns + column] = -5.0;
        }
    }
    m
}

#[test]
fn a_screen_point_goes_back_to_the_top_plane_under_it() {
    let m = slotted();
    let area = Rect::from_min_size(pos2(10.0, 20.0), Vec2::new(400.0, 300.0));
    for perspective in [false, true] {
        let framing = Framing {
            zoom: 1.5,
            pan: Vec2::new(7.0, -3.0),
            perspective,
            ..Framing::default()
        };
        let camera = Camera::new(&m, Orbit::default(), framing, area, None);
        let p = DVec3::new(30.0, 70.0, 0.0);
        let back = camera.on_plane(camera.screen(p), 0.0).unwrap();
        // screen positions are f32
        assert!(
            (back - p.truncate()).length() < 1e-3,
            "perspective {perspective}: {back}"
        );
    }
    let edge_on = Camera::new(&m, Orbit::FRONT, Framing::default(), area, None);
    assert_eq!(
        edge_on.on_plane(area.center(), 0.0),
        None,
        "the plane seen edge-on"
    );
    // nearly side on in perspective, the top of the view looks over the horizon
    let low = Orbit {
        pitch: 80f64.to_radians(),
        ..Orbit::FRONT
    };
    let framing = Framing {
        perspective: true,
        ..Framing::default()
    };
    let camera = Camera::new(&m, low, framing, area, None);
    assert_eq!(
        camera.on_plane(area.center_top(), 0.0),
        None,
        "above the horizon, not a point behind the eye"
    );
    assert!(camera.on_plane(area.center_bottom(), 0.0).is_some());
    // a raised plane: the point drawn at that height goes back to it
    let camera = Camera::new(&m, Orbit::default(), framing, area, None);
    let p = DVec3::new(30.0, 70.0, 6.0);
    let back = camera.on_plane(camera.screen(p), 6.0).unwrap();
    assert!((back - p.truncate()).length() < 1e-3, "{back}");
}

#[test]
fn a_screen_point_goes_to_the_surface_it_shows_not_the_plane_behind_it() {
    let m = slotted();
    let area = Rect::from_min_size(pos2(10.0, 20.0), Vec2::new(400.0, 300.0));
    for perspective in [false, true] {
        let framing = Framing {
            zoom: 1.5,
            pan: Vec2::new(7.0, -3.0),
            perspective,
            ..Framing::default()
        };
        let camera = Camera::new(&m, Orbit::default(), framing, area, None);
        // the slot's floor, 5 mm down, seen at a slant: the top plane under the same
        // pixel is millimetres off it
        for p in [
            DVec3::new(50.0, 50.0, -5.0),
            DVec3::new(20.0, 30.0, 0.0),
            DVec3::new(75.0, 10.0, 0.0),
        ] {
            let at = camera.screen(p);
            let back = camera.on_surface(&m, at).unwrap();
            assert!(
                (back - p.truncate()).length() < 0.01,
                "perspective {perspective}: {p} came back as {back}"
            );
            if p.z < 0.0 {
                let plane = camera.on_plane(at, 0.0).unwrap();
                assert!((plane - p.truncate()).length() > 1.0, "{plane}");
            }
        }
        // off the material, the top plane carries on
        let off = DVec3::new(130.0, 50.0, 0.0);
        let back = camera.on_surface(&m, camera.screen(off)).unwrap();
        assert!((back - off.truncate()).length() < 0.01, "{back}");
    }
}

#[test]
fn shadow_shading_darkens_the_floor_behind_a_wall_from_the_light() {
    let m = slotted();
    let top = shadow_top(&m);
    assert_eq!(top, 0.0, "the uncut top");
    // the light comes from -x: the slot's floor by its -x wall is behind that wall
    assert!(in_shadow(&m, (41, 50), LIGHT, top), "just behind the wall");
    assert!(
        !in_shadow(&m, (55, 50), LIGHT, top),
        "far enough out from it"
    );
    assert!(!in_shadow(&m, (10, 50), LIGHT, top), "the flat top");
    assert!(
        !in_shadow(&m, (41, 50), DVec3::Z, top),
        "a light straight above"
    );
    let map = shadow_map(&m, LIGHT);
    let at = |c: usize| map[50 * m.columns + c];
    assert!(at(41) && !at(55) && !at(10), "the map agrees, cell by cell");
    assert!(
        shadowed_brightness(&m, (41, 50), LIGHT, at(41)) < brightness(&m, 41, 50, LIGHT),
        "drawn darker"
    );
    assert!(
        (shadowed_brightness(&m, (55, 50), LIGHT, at(55)) - brightness(&m, 55, 50, LIGHT)).abs()
            < 1e-12,
        "the lit floor unchanged"
    );
}

#[test]
fn walls_facing_the_light_are_brighter_than_flat_and_the_far_ones_darker() {
    let m = slotted();
    let flat = brightness(&m, 10, 50, LIGHT);
    assert!(
        brightness(&m, 60, 50, LIGHT) > flat,
        "the wall rising towards +x faces the light"
    );
    assert!(
        brightness(&m, 39, 50, LIGHT) < flat,
        "the wall dropping towards +x faces away"
    );
    assert!(
        brightness(&m, 50, 50, LIGHT) < flat,
        "the slot floor is deeper so darker"
    );
}

#[test]
#[ignore = "timing only: cargo test --release -- --ignored mesh_timing --nocapture"]
fn mesh_timing() {
    use cut_craft_core::component::{Combine, Component};
    let mut doc = cut_craft_core::document::Document::default();
    let o = doc.job.area().min;
    let rect = cut_craft_core::geometry::Rect::new(
        o + cut_craft_core::geometry::Point::new(150.0, 100.0),
        o + cut_craft_core::geometry::Point::new(450.0, 250.0),
    );
    let mut c = cut_craft_core::Change::builder(&doc);
    c.add_component(Component::from_fn(
        "m",
        Combine::Add,
        rect,
        doc.job.model_cell(),
        |p| Some((p.x * 0.3).sin() + (p.y * 0.2).cos() + 3.0),
    ));
    doc.apply(&c.finish()).unwrap();
    let area = Rect::from_min_size(Pos2::ZERO, eframe::egui::vec2(1400.0, 900.0));
    let look = Look {
        colour: Color32::GRAY,
        light: DVec3::new(-1.0, 1.0, 1.0),
        sides: true,
        wrap: None,
        grain: None,
        shadows: false,
    };
    for cells in [250, 500, 1000, 1500] {
        let t = std::time::Instant::now();
        let m = cut_craft_core::component::model_shape(&doc, cells);
        let built = t.elapsed();
        let t = std::time::Instant::now();
        let mesh = mesh(&m, Orbit::default(), Framing::default(), &look, area);
        eprintln!(
            "{cells} cells: model {built:?}, mesh {:?}, {} triangles",
            t.elapsed(),
            mesh.indices.len() / 3
        );
    }
}

#[test]
fn coarse_cells_of_taken_away_material_stay_empty() {
    let mut m = slotted();
    m.heights
        .iter_mut()
        .take(m.columns * 8)
        .for_each(|h| *h = f64::NAN);
    let c = coarse(&m, 30);
    assert!(c.heights[0].is_nan(), "a block all gone is gone");
    assert!(c.heights[c.columns].is_nan(), "rows 0..8 are 2 coarse rows");
    assert!(c.heights[2 * c.columns].is_finite(), "the rest stays");
}

#[test]
fn coarse_cells_keep_the_deepest_point() {
    let mut m = slotted();
    m.heights[5 * m.columns + 7] = -9.0;
    let c = coarse(&m, 30);
    assert_eq!(
        (c.columns, c.rows),
        (25, 25),
        "4 mm cells: the step that fits 30"
    );
    assert!((c.cell - 4.0).abs() < 1e-12);
    assert_eq!(c.heights[c.columns + 1], -9.0);
    assert_eq!(
        c.heights[12 * c.columns + 12],
        -5.0,
        "x 48 to 52 is in the slot"
    );
    assert_eq!(c.bottom, m.bottom);
}

/// A 20 x 10 mm part of a 600 x 400 sheet at 0.1 mm, from (100, 50), a hole 2 mm deep
/// (marked 4) in its middle.
fn part_of_sheet() -> (Job, Material) {
    let job = Job::default();
    let o = job.area().min;
    let area = cut_craft_core::geometry::Rect {
        min: o + Point::new(100.0, 50.0),
        max: o + Point::new(120.0, 60.0),
    };
    let mut m = Material::over(&job, area, 0.1);
    for r in 40..60 {
        for c in 90..110 {
            m.heights[r * m.columns + c] = -2.0;
            m.cut_by[r * m.columns + c] = 4;
        }
    }
    (job, m)
}

#[test]
fn a_part_of_the_sheet_is_drawn_on_the_whole_sheet_uncut_round_it() {
    let (job, m) = part_of_sheet();
    let o = job.area().min;
    let s = on_sheet(&m, job.area(), 1200);
    assert_eq!((s.columns, s.rows), (1200, 800), "half a millimetre each");
    assert!((s.origin - o).length() < 1e-9);
    // the hole is 2 mm across: four sheet cells, each the deepest under it
    let at = |x: f64, y: f64| s.index_at(o + Point::new(x, y)).unwrap();
    assert_eq!(s.heights[at(110.0, 55.0)], -2.0);
    assert_eq!(s.cut_by[at(110.0, 55.0)], 4);
    assert_eq!(s.heights[at(102.0, 52.0)], 0.0, "uncut in the part");
    assert_eq!(s.heights[at(300.0, 300.0)], 0.0, "uncut round it");
    assert_eq!(s.cut_by[at(300.0, 300.0)], 0);
    // a whole sheet is coarsened as before
    let whole = Material::new(&job, 2400);
    assert_eq!(on_sheet(&whole, job.area(), 1200), coarse(&whole, 1200));
}

#[test]
fn the_sheet_has_a_hole_for_the_preview_with_a_lowered_rim_under_its_edge() {
    let (job, m) = part_of_sheet();
    let o = job.area().min;
    let s = holed(on_sheet(&m, job.area(), 1200), m.area());
    let at = |x: f64, y: f64| s.index_at(o + Point::new(x, y)).unwrap();
    assert!(
        s.heights[at(110.0, 55.0)].is_nan(),
        "the preview is drawn there"
    );
    assert!(s.heights[at(101.7, 55.0)].is_nan(), "past the rim");
    let rim = s.heights[at(100.2, 55.0)];
    assert!(rim < 0.0 && rim > -0.05, "under the preview's edge: {rim}");
    assert_eq!(s.heights[at(99.7, 55.0)], 0.0, "beyond it, as it was");
    // the sheet's own edge stays, so its sides stand
    let (_, edge) = part_of_sheet();
    let mut at_edge = edge;
    at_edge.origin = o;
    let s = holed(on_sheet(&at_edge, job.area(), 1200), at_edge.area());
    assert!(!s.heights[0].is_nan(), "the corner keeps its side");
    assert!(s.heights[2 * s.columns + 5].is_nan(), "inside the rim");
}

#[test]
fn from_straight_above_the_projection_is_the_plan() {
    let top = Orbit {
        yaw: 0.0,
        pitch: 0.0,
        roll: 0.0,
    };
    let (p, depth) = top.project(DVec3::new(30.0, 40.0, -2.0));
    assert!(p.abs_diff_eq(DVec2::new(30.0, 40.0), 1e-12));
    assert!((depth + 2.0).abs() < 1e-12, "deeper is further away");
}

#[test]
fn tilting_raises_the_far_edge_and_puts_it_behind() {
    let tilted = Orbit {
        yaw: 0.0,
        pitch: 60f64.to_radians(),
        roll: 0.0,
    };
    let (front, front_depth) = tilted.project(DVec3::ZERO);
    let (back, back_depth) = tilted.project(DVec3::new(0.0, 100.0, 0.0));
    assert!(back.y > front.y);
    assert!(back_depth < front_depth);
}

#[test]
fn the_mesh_fits_the_area_and_is_drawn_far_to_near() {
    let area = Rect::from_min_max(pos2(0.0, 0.0), pos2(400.0, 300.0));
    let orbit = Orbit {
        yaw: 0.0,
        pitch: 60f64.to_radians(),
        roll: 0.0,
    };
    let mesh = mesh(&slotted(), orbit, Framing::default(), &plain(), area);
    assert_ne!(mesh.indices.len(), 0);
    assert!(
        mesh.vertices
            .iter()
            .all(|v| area.expand(1e-3).contains(v.pos)),
        "inside the area"
    );
    let centre_y = |triangle: &[u32]| -> f32 {
        triangle
            .iter()
            .map(|&i| mesh.vertices[i as usize].pos.y)
            .sum::<f32>()
            / 3.0
    };
    let (first, last) = (
        centre_y(&mesh.indices[..3]),
        centre_y(&mesh.indices[mesh.indices.len() - 3..]),
    );
    assert!(first < last, "far (higher on screen) first, near last");
}

#[test]
fn seen_from_below_a_cut_underside_is_drawn_cell_by_cell() {
    let area = Rect::from_min_max(pos2(0.0, 0.0), pos2(400.0, 300.0));
    let below = Orbit::facing(DVec3::NEG_Z);
    let flat = mesh(&slotted(), below, Framing::default(), &plain(), area);
    let mut cut = slotted();
    cut.under = vec![cut.bottom; cut.heights.len()];
    cut.under[0] = cut.bottom + 2.0;
    let shown = mesh(&cut, below, Framing::default(), &plain(), area);
    assert!(
        shown.indices.len() > flat.indices.len() * 3 / 2,
        "the underside's own grid: {} {}",
        shown.indices.len(),
        flat.indices.len()
    );
    let above = mesh(&cut, Orbit::TOP, Framing::default(), &plain(), area);
    let plain_above = mesh(&slotted(), Orbit::TOP, Framing::default(), &plain(), area);
    assert_eq!(
        above.indices.len(),
        plain_above.indices.len(),
        "hidden from above"
    );
}

#[test]
fn turning_never_changes_where_the_block_is_or_its_size() {
    let area = Rect::from_min_max(pos2(0.0, 0.0), pos2(400.0, 300.0));
    let width = |orbit| {
        let mesh = mesh(&slotted(), orbit, Framing::default(), &plain(), area);
        let xs = mesh.vertices.iter().map(|v| v.pos.x);
        xs.clone().fold(f32::INFINITY, f32::min)..xs.fold(f32::NEG_INFINITY, f32::max)
    };
    for orbit in [Orbit::TOP, Orbit::default(), Orbit::FRONT, Orbit::RIGHT] {
        let r = width(orbit);
        assert!(
            r.start >= -1e-3 && r.end <= 400.0 + 1e-3,
            "{orbit:?}: {r:?}"
        );
    }
    let top = width(Orbit::TOP);
    assert!(
        (f32::midpoint(top.start, top.end) - 200.0).abs() < 1.0,
        "centred: {top:?}"
    );
}

#[test]
fn zooming_keeps_the_point_under_the_cursor_still() {
    let at = Vec2::new(50.0, -20.0);
    let f = Framing::default().panned(Vec2::new(10.0, 5.0));
    let z = f.zoomed_about(at, 2.0);
    // a block point at offset o from its centre shows at pan + zoom * o
    let o = (at - f.pan) / f.zoom;
    assert!((z.pan + o * z.zoom - at).length() < 1e-4);
    assert!((z.zoom - 2.0).abs() < 1e-6);
}

#[test]
fn mouse_buttons_pick_rotate_or_pan() {
    assert_eq!(
        drag_mode([true, false, false], false),
        Some(DragMode::Rotate)
    );
    assert_eq!(
        drag_mode([false, true, false], false),
        Some(DragMode::Rotate)
    );
    assert_eq!(drag_mode([false, false, true], false), Some(DragMode::Pan));
    assert_eq!(drag_mode([false, true, false], true), Some(DragMode::Pan));
    assert_eq!(drag_mode([true, true, false], false), Some(DragMode::Pan));
    assert_eq!(drag_mode([false, false, false], false), None);
}

#[test]
fn the_standard_views_look_from_where_they_say() {
    // the side nearest the viewer has the greatest depth
    let nearest = |orbit: Orbit, a: DVec3, b: DVec3| orbit.project(a).1 > orbit.project(b).1;
    assert!(nearest(Orbit::TOP, DVec3::Z, DVec3::NEG_Z));
    assert!(nearest(Orbit::FRONT, DVec3::NEG_Y, DVec3::Y));
    assert!(nearest(Orbit::RIGHT, DVec3::X, DVec3::NEG_X));
}

/// Whether `a` and `b` put job points in the same places on screen.
fn same_view(a: Orbit, b: Orbit) -> bool {
    [DVec3::X, DVec3::Y, DVec3::Z, DVec3::new(1.0, 2.0, 3.0)]
        .into_iter()
        .all(|p| {
            let (pa, da) = a.project(p);
            let (pb, db) = b.project(p);
            (pa - pb).length() < 1e-9 && (da - db).abs() < 1e-9
        })
}

#[test]
fn any_way_of_facing_survives_its_matrix() {
    for (yaw, pitch, roll) in [
        (0.3, 1.1, 0.0),
        (-2.0, 2.5, 0.7),
        (1.0, 0.0, 0.4),
        (0.0, 3.0, -2.9),
    ] {
        let o = Orbit { yaw, pitch, roll };
        let back = Orbit::from_matrix(o.matrix());
        assert!(same_view(o, back), "{o:?} came back as {back:?}");
        // and to_world undoes project
        let p = DVec3::new(0.4, -1.3, 2.2);
        let (on, depth) = o.project(p);
        assert!((o.to_world(DVec3::new(on.x, on.y, depth)) - p).length() < 1e-9);
    }
}

#[test]
fn free_rotation_turns_as_the_turntable_does_side_on_and_goes_past_the_top() {
    let size = Vec2::new(400.0, 300.0);
    for by in [Vec2::new(30.0, 0.0), Vec2::new(0.0, 20.0)] {
        let turntable = Orbit::FRONT.dragged_by(by, size, Rotation::Turntable);
        let free = Orbit::FRONT.dragged_by(by, size, Rotation::Free);
        assert!(
            same_view(turntable, free),
            "{by:?}: {turntable:?} against {free:?}"
        );
    }
    // up past straight down: the turntable stops, free rotation carries on over
    let over = Vec2::new(0.0, -60.0);
    let stopped = Orbit::TOP.dragged_by(over, size, Rotation::Turntable);
    let carried = Orbit::TOP.dragged_by(over, size, Rotation::Free);
    assert!(
        same_view(stopped, Orbit::TOP),
        "the turntable stops at the top"
    );
    assert!(!same_view(carried, Orbit::TOP), "free rotation does not");
    assert!(carried.towards_viewer().z > 0.0 && carried.towards_viewer().z < 1.0);
}

#[test]
fn dragging_turns_and_tilts_from_straight_down_past_side_on_to_straight_up() {
    let size = Vec2::new(400.0, 300.0);
    let o = Orbit::TOP.dragged_by(Vec2::new(200.0, -300.0), size, Rotation::Turntable);
    assert!((o.yaw - std::f64::consts::FRAC_PI_2).abs() < 1e-9);
    assert_eq!(o.pitch, 0.0, "cannot tilt past straight down");
    let under = Orbit::FRONT.dragged_by(Vec2::new(0.0, 50.0), size, Rotation::Turntable);
    assert!(under.pitch > std::f64::consts::FRAC_PI_2, "past side on");
    assert!(under.towards_viewer().z < 0.0, "from underneath");
    assert_eq!(
        Orbit::TOP
            .dragged_by(Vec2::new(0.0, 900.0), size, Rotation::Turntable)
            .pitch,
        std::f64::consts::PI
    );
}

#[test]
fn facing_looks_from_each_face_edge_and_corner() {
    assert_eq!(Orbit::facing(DVec3::Z), Orbit::TOP);
    let same = |a: Orbit, b: Orbit| a.towards_viewer().abs_diff_eq(b.towards_viewer(), 1e-12);
    assert!(same(Orbit::facing(DVec3::NEG_Y), Orbit::FRONT));
    assert!(same(Orbit::facing(DVec3::X * 3.0), Orbit::RIGHT));
    for from in [
        DVec3::NEG_Z,
        DVec3::new(1.0, -1.0, 0.0),
        DVec3::new(-1.0, 1.0, 1.0),
        DVec3::new(1.0, 1.0, -1.0),
    ] {
        let towards = Orbit::facing(from).towards_viewer();
        assert!(
            towards.abs_diff_eq(from.normalize(), 1e-12),
            "{from}: {towards}"
        );
    }
    let below = Orbit::facing(DVec3::NEG_Z);
    let (x, _) = below.project(DVec3::X);
    assert!(x.abs_diff_eq(DVec2::X, 1e-12), "x still to the right");
}

#[test]
fn in_perspective_nearer_is_larger_and_the_block_still_fits() {
    let area = Rect::from_min_max(pos2(0.0, 0.0), pos2(400.0, 300.0));
    let m = slotted();
    let ortho = Framing::default();
    let perspective = Framing {
        perspective: true,
        ..ortho
    };
    // from the front, the front (y 0) and back (y 100) edges' lengths
    let edges = |framing: Framing| {
        let screen = screen_of(&m, Orbit::FRONT, framing, area, None);
        let across =
            |y: f64| screen(DVec3::new(0.0, y, 0.0)).distance(screen(DVec3::new(100.0, y, 0.0)));
        (across(0.0), across(100.0))
    };
    let (front, back) = edges(ortho);
    assert!((front - back).abs() < 1e-3, "orthographic: the same");
    let (front, back) = edges(perspective);
    assert!(front > back * 1.1, "perspective: {front} against {back}");
    for orbit in [Orbit::TOP, Orbit::default(), Orbit::FRONT] {
        let drawn = mesh(&m, orbit, perspective, &plain(), area);
        assert!(
            drawn
                .vertices
                .iter()
                .all(|v| area.expand(1e-3).contains(v.pos)),
            "{orbit:?}"
        );
    }
}

#[test]
fn from_below_the_block_shows_its_bottom_last() {
    let area = Rect::from_min_max(pos2(0.0, 0.0), pos2(400.0, 300.0));
    let m = slotted();
    let count = |orbit| {
        mesh(&m, orbit, Framing::default(), &plain(), area)
            .vertices
            .len()
    };
    let below = Orbit::facing(DVec3::new(0.3, -1.0, -0.5));
    let above = Orbit::facing(DVec3::new(0.3, -1.0, 0.5));
    assert_eq!(count(below), count(above) + 4, "one quad more: the bottom");
    let drawn = mesh(&m, below, Framing::default(), &plain(), area);
    let bottom = Rgba::from(drawn.vertices.last().unwrap().color);
    let lit_bottom = lit(DVec3::NEG_Z, LIGHT);
    assert!(
        (f64::from(bottom.r()) - lit_bottom).abs() < 0.01,
        "drawn last, over the surface"
    );
}

/// 100 x 100 mm in 1 mm cells, its top at 0, with a 20 mm high ridge from x = 40 to 60.
fn ridged() -> Material {
    let job = Job {
        width: 100.0,
        height: 100.0,
        ..Job::default()
    };
    let mut m = Material::new(&job, 100);
    for row in 0..m.rows {
        for column in 40..60 {
            m.heights[row * m.columns + column] = 20.0;
        }
    }
    m
}

#[test]
fn the_surface_hides_what_is_behind_it_and_nothing_else() {
    let m = ridged();
    let seen = |orbit: Orbit, framing: Framing, p: DVec3| seen_of(&m, orbit, framing)(p);
    let (ortho, perspective) = (
        Framing::default(),
        Framing {
            perspective: true,
            ..Framing::default()
        },
    );
    let left = DVec3::new(20.0, 50.0, 0.0);
    let ridge_top = DVec3::new(50.0, 50.0, 20.0);
    let (left_view, above_right) = (
        Orbit::facing(DVec3::NEG_X),
        Orbit::facing(DVec3::new(1.0, 0.0, 0.3)),
    );
    for framing in [ortho, perspective] {
        assert!(!seen(Orbit::RIGHT, framing, left), "behind the ridge");
        assert!(seen(Orbit::TOP, framing, left), "from above");
        assert!(seen(left_view, framing, left), "on the near side");
        assert!(seen(above_right, framing, ridge_top), "on top of it");
    }
    // from low on the right the ridge hides the near part of the floor behind it
    let low = Orbit::facing(DVec3::new(1.0, 0.0, 0.5));
    assert!(!seen(low, ortho, DVec3::new(35.0, 50.0, 0.0)));
    assert!(
        seen(low, ortho, DVec3::new(-30.0, 50.0, 60.0)),
        "high off it"
    );
    let below = Orbit::facing(DVec3::NEG_Z);
    assert!(!seen(below, ortho, left), "the block is under the top");
    assert!(
        seen(below, ortho, DVec3::new(-5.0, 50.0, 0.0)),
        "off the block"
    );
}

#[test]
fn a_line_is_cut_where_it_is_hidden() {
    // a line across the ridge's far side, seen from the right: the part behind it goes
    let m = ridged();
    let seen = seen_of(&m, Orbit::RIGHT, Framing::default());
    let line = [DVec3::new(10.0, 20.0, 0.0), DVec3::new(10.0, 80.0, 0.0)];
    assert!(visible_runs(&line, m.cell, &seen).is_empty(), "all behind");
    let across = [DVec3::new(10.0, 50.0, 0.0), DVec3::new(90.0, 50.0, 0.0)];
    let runs = visible_runs(&across, m.cell, &seen);
    assert_eq!(runs.len(), 1, "{runs:?}");
    assert!(
        runs[0].iter().all(|p| p.x >= 59.0),
        "only right of the ridge"
    );
    let all = |_| true;
    assert_eq!(visible_runs(&across, 1.0, all)[0].len(), 81, "a point a mm");
    let gap = |p: DVec3| !(40.0..60.0).contains(&p.x);
    assert_eq!(visible_runs(&across, 1.0, gap).len(), 2, "split in two");
}

#[test]
fn the_view_cube_turns_to_what_is_under_the_pointer() {
    let cube = ViewCube {
        centre: pos2(100.0, 100.0),
        half: 30.0,
    };
    let iso = Orbit::default();
    // the middles of the faces in sight, and a corner and an edge of the top
    for from in [DVec3::Z, DVec3::NEG_Y, DVec3::X] {
        let at = cube.screen(iso, from);
        assert_eq!(cube.direction_at(iso, at), Some(from), "{from}");
    }
    let corner = cube.screen(iso, ViewCube::part_middle(DVec3::new(1.0, -1.0, 1.0)));
    let towards = cube.direction_at(iso, corner).unwrap();
    assert!(towards.abs_diff_eq(DVec3::new(1.0, -1.0, 1.0).normalize(), 1e-12));
    let edge = cube.screen(iso, ViewCube::part_middle(DVec3::new(0.0, -1.0, 1.0)));
    let towards = cube.direction_at(iso, edge).unwrap();
    assert!(towards.abs_diff_eq(DVec3::new(0.0, -1.0, 1.0).normalize(), 1e-12));
    assert_eq!(cube.direction_at(iso, pos2(0.0, 0.0)), None, "off it");
    assert!(cube.rect().contains(cube.screen(iso, DVec3::ONE)));
    // face on, the faces seen edge-on take no clicks
    assert_eq!(
        cube.direction_at(Orbit::RIGHT, cube.screen(Orbit::RIGHT, DVec3::X)),
        Some(DVec3::X)
    );
}

#[test]
fn the_view_cube_is_a_dice_with_26_parts() {
    let parts = &*super::cube::PARTS;
    assert_eq!(parts.len(), 26);
    let corners = |n: usize| parts.iter().filter(|p| p.outline.len() == n).count();
    // eight-sided faces, four-sided edge bevels and six-sided corners (touching three faces
    // and three edges)
    assert_eq!((corners(8), corners(4), corners(6)), (6, 12, 8));
    // each edge bevel is 15% of the cube across
    let top = parts.iter().find(|p| p.from == DVec3::Z).unwrap();
    let widest = top.outline.iter().map(|p| p.x).fold(f64::MIN, f64::max);
    assert!((widest - (1.0 - BEVEL)).abs() < 1e-9, "{widest}");
    assert!((BEVEL / 2.0 - 0.15).abs() < 1e-12);
}

#[test]
fn the_view_cube_turns_to_the_bevel_under_the_pointer_seen_straight_on() {
    let cube = ViewCube {
        centre: pos2(100.0, 100.0),
        half: 30.0,
    };
    // looking at the front, its bevels still show round it and take clicks
    let front = Orbit::FRONT;
    let top_edge = cube.screen(front, ViewCube::part_middle(DVec3::new(0.0, -1.0, 1.0)));
    let towards = cube.direction_at(front, top_edge).unwrap();
    assert!(towards.abs_diff_eq(DVec3::new(0.0, -1.0, 1.0).normalize(), 1e-12));
    let corner = cube.screen(front, ViewCube::part_middle(DVec3::new(1.0, -1.0, 1.0)));
    let towards = cube.direction_at(front, corner).unwrap();
    assert!(towards.abs_diff_eq(DVec3::new(1.0, -1.0, 1.0).normalize(), 1e-12));
}

#[test]
fn the_cube_turns_and_nudges_only_straight_on() {
    let cube = ViewCube {
        centre: pos2(100.0, 100.0),
        half: 30.0,
    };
    let pressable = |orbit| -> Vec<CubeButton> {
        ViewCube::buttons(orbit)
            .into_iter()
            .filter_map(|(b, on)| on.then_some(b))
            .collect()
    };
    assert_eq!(pressable(Orbit::default()), vec![CubeButton::Home]);
    assert_eq!(
        pressable(Orbit::FRONT).len(),
        7,
        "Home, two turns, four nudges"
    );
    // the buttons sit apart, round the cube
    let rects: Vec<_> = ViewCube::buttons(Orbit::FRONT)
        .into_iter()
        .map(|(b, _)| cube.button_rect(b))
        .collect();
    for (k, a) in rects.iter().enumerate() {
        assert!(cube.rect().expand(1.0).contains_rect(*a), "{a:?}");
        for b in &rects[k + 1..] {
            assert!(!a.intersects(*b), "{a:?} {b:?}");
        }
    }
    let up = cube.button_rect(CubeButton::Nudge(Nudge::Up)).center();
    assert_eq!(
        cube.button_at(Orbit::FRONT, up),
        Some(CubeButton::Nudge(Nudge::Up))
    );
    assert_eq!(
        cube.button_at(Orbit::default(), up),
        None,
        "hidden off straight on"
    );
}

#[test]
fn a_quarter_turn_turns_the_picture_about_the_line_of_sight() {
    let on_screen = |orbit: Orbit, v: DVec3| orbit.project(v).0;
    // from the top, clockwise: X from pointing right to pointing down the screen
    let turned = Orbit::TOP.quarter_turned(true);
    assert!(turned.head_on());
    assert!(on_screen(turned, DVec3::X).abs_diff_eq(DVec2::NEG_Y, 1e-12));
    assert!(turned.towards_viewer().abs_diff_eq(DVec3::Z, 1e-12));
    let back = turned.quarter_turned(false);
    assert!(on_screen(back, DVec3::X).abs_diff_eq(DVec2::X, 1e-12));
    // from the front it turns about Y, from the right about X
    let front = Orbit::FRONT.quarter_turned(true);
    assert!(front.towards_viewer().abs_diff_eq(DVec3::NEG_Y, 1e-12));
    assert!(on_screen(front, DVec3::Z).abs_diff_eq(DVec2::X, 1e-12));
    let right = Orbit::RIGHT.quarter_turned(false);
    assert!(right.towards_viewer().abs_diff_eq(DVec3::X, 1e-12));
}

#[test]
fn a_nudge_brings_round_the_face_beyond_that_side_of_the_screen() {
    let from = |orbit: Orbit| orbit.towards_viewer();
    let front = Orbit::FRONT;
    assert_eq!(front.face_beyond(Nudge::Up), "view-cube-top");
    assert!(from(front.nudged(Nudge::Up)).abs_diff_eq(DVec3::Z, 1e-12));
    assert!(from(front.nudged(Nudge::Down)).abs_diff_eq(DVec3::NEG_Z, 1e-12));
    assert!(from(front.nudged(Nudge::Left)).abs_diff_eq(DVec3::NEG_X, 1e-12));
    assert!(from(front.nudged(Nudge::Right)).abs_diff_eq(DVec3::X, 1e-12));
    // relative to the screen: turned on its side, up is what is now at the top
    let turned = front.quarter_turned(true);
    assert_eq!(turned.face_beyond(Nudge::Up), "view-cube-left");
    assert!(from(turned.nudged(Nudge::Up)).abs_diff_eq(DVec3::NEG_X, 1e-12));
    assert!(front.nudged(Nudge::Up).head_on());
    assert!(!Orbit::default().head_on());
}

#[test]
fn a_smooth_turn_eases_from_one_view_to_the_next() {
    let turning = Turning::new(Orbit::FRONT, Orbit::TOP, 10.0);
    let (start, done) = turning.at(10.0);
    assert!(!done && start.towards_viewer().abs_diff_eq(DVec3::NEG_Y, 1e-9));
    let (half, _) = turning.at(10.0 + TURN_SECONDS / 2.0);
    let way = DVec3::new(0.0, -1.0, 1.0).normalize();
    assert!(
        half.towards_viewer().abs_diff_eq(way, 1e-9),
        "{}",
        half.towards_viewer()
    );
    let (end, done) = turning.at(10.0 + TURN_SECONDS);
    assert!(done);
    assert_eq!(end, Orbit::TOP);
    // a turntable stands a view turned on its side upright
    let side = Orbit::FRONT.quarter_turned(true);
    assert_eq!(side.upright(Rotation::Turntable).roll, 0.0);
    assert_eq!(side.upright(Rotation::Free), side);
}
