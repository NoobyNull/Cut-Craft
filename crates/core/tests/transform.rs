use cut_craft_core::document::{Document, EntityId};
use cut_craft_core::geometry::{Path, Point, Rect};
use cut_craft_core::glam::DAffine2;
use cut_craft_core::params::Anchor;
use cut_craft_core::{Change, CommandError};

fn doc_with_squares() -> Document {
    let mut doc = Document::default();
    let mut c = Change::builder(&doc);
    let a = c.add(Path::polyline(
        &[
            Point::new(0.0, 0.0),
            Point::new(10.0, 0.0),
            Point::new(10.0, 10.0),
        ],
        true,
    ));
    c.add(Path::polyline(
        &[
            Point::new(50.0, 50.0),
            Point::new(60.0, 50.0),
            Point::new(60.0, 60.0),
        ],
        true,
    ));
    c.select([a]);
    doc.apply(&c.finish()).unwrap();
    doc
}

#[test]
fn transforms_only_the_selected_paths() {
    let mut doc = doc_with_squares();
    let change =
        Change::transform_selected(&doc, |_| DAffine2::from_translation(Point::new(5.0, 0.0)))
            .unwrap();
    doc.apply(&change).unwrap();
    assert_eq!(
        doc.vectors[&EntityId(1)].bounds(),
        Rect::new(Point::new(5.0, 0.0), Point::new(15.0, 10.0))
    );
    assert_eq!(
        doc.vectors[&EntityId(2)].bounds().min,
        Point::new(50.0, 50.0)
    );
}

#[test]
fn nothing_selected_is_an_error() {
    let mut doc = doc_with_squares();
    let mut c = Change::builder(&doc);
    c.select([]);
    doc.apply(&c.finish()).unwrap();
    assert_eq!(
        Change::transform_selected(&doc, |_| DAffine2::IDENTITY),
        Err(CommandError::NothingSelected)
    );
}

#[test]
fn identity_transforms_make_no_edits() {
    let doc = doc_with_squares();
    assert!(
        Change::transform_selected(&doc, |_| DAffine2::IDENTITY)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn anchors_pick_points_of_a_rectangle() {
    let r = Rect::new(Point::new(0.0, 0.0), Point::new(10.0, 20.0));
    assert_eq!(Anchor::BottomLeft.point(r), Point::new(0.0, 0.0));
    assert_eq!(Anchor::Center.point(r), Point::new(5.0, 10.0));
    assert_eq!(Anchor::TopRight.point(r), Point::new(10.0, 20.0));
    assert_eq!(Anchor::Left.point(r), Point::new(0.0, 10.0));
    assert_eq!(Anchor::Top.point(r), Point::new(5.0, 20.0));
}

#[test]
fn align_selected_moves_each_path_by_an_offset_from_its_bounds() {
    let mut doc = doc_with_squares();
    let mut c = Change::builder(&doc);
    c.select([EntityId(1), EntityId(2)]);
    doc.apply(&c.finish()).unwrap();
    // left-align: each path's left edge to the selection's left edge
    let change =
        Change::align_selected(&doc, |path, sel| Point::new(sel.min.x - path.min.x, 0.0)).unwrap();
    doc.apply(&change).unwrap();
    assert!(doc.vectors.values().all(|p| p.bounds().min.x == 0.0));
}

mod components {
    use super::*;
    use cut_craft_core::testing::domed;

    fn dome_selected() -> Document {
        let mut doc = domed();
        doc.selected_components = [EntityId(1)].into();
        doc
    }

    fn peak(doc: &Document) -> (Point, f64) {
        let k = &doc.components[&EntityId(1)];
        (0..k.heights.len())
            .filter_map(|i| k.shaped(i).map(|z| (k.centre_of(i), z)))
            .max_by(|a, b| a.1.total_cmp(&b.1))
            .unwrap()
    }

    #[test]
    fn a_selected_component_counts_in_the_selection_bounds() {
        let doc = dome_selected();
        assert_eq!(
            doc.selection_bounds(),
            Some(doc.components[&EntityId(1)].extent())
        );
    }

    #[test]
    fn moving_carries_the_shape_and_its_heights() {
        let mut doc = dome_selected();
        let (at, top) = peak(&doc);
        let shift = Point::new(5.0, -3.0);
        let change =
            Change::transform_selected(&doc, |_| DAffine2::from_translation(shift)).unwrap();
        doc.apply(&change).unwrap();
        let (moved, high) = peak(&doc);
        assert!((moved - (at + shift)).length() < 0.3, "{moved:?}");
        assert!((high - top).abs() < 0.2, "{high} {top}");
    }

    #[test]
    fn scaling_up_makes_it_wider_not_taller() {
        let mut doc = dome_selected();
        let before = doc.components[&EntityId(1)].clone();
        let c = before.extent().center();
        let t = DAffine2::from_translation(c)
            * DAffine2::from_scale(Point::splat(2.0))
            * DAffine2::from_translation(-c);
        doc.apply(&Change::transform_selected(&doc, |_| t).unwrap())
            .unwrap();
        let after = &doc.components[&EntityId(1)];
        assert!((after.extent().size().x - 2.0 * before.extent().size().x).abs() < 0.5);
        let (lo, hi) = (
            before.height_range().unwrap(),
            after.height_range().unwrap(),
        );
        assert!((lo.1 - hi.1).abs() < 0.2, "{lo:?} {hi:?}");
        // the shape's edge moved out: a point just outside the old dome is now inside
        assert!(before.height_at(c + Point::new(9.0, 0.0)).is_none());
        assert!(after.height_at(c + Point::new(9.0, 0.0)).is_some());
    }

    #[test]
    fn rotating_a_ramp_turns_its_slope() {
        use cut_craft_core::component::{Combine, Component};
        let mut doc = Document::default();
        let ramp = Component::from_fn(
            "ramp",
            Combine::Add,
            Rect::new(Point::ZERO, Point::new(10.0, 10.0)),
            0.5,
            |p| Some(p.x),
        );
        let mut c = Change::builder(&doc);
        let id = c.add_component(ramp);
        doc.apply(&c.finish()).unwrap();
        doc.selected_components = [id].into();
        let centre = Point::splat(5.0);
        let t = DAffine2::from_translation(centre)
            * DAffine2::from_angle(90_f64.to_radians())
            * DAffine2::from_translation(-centre);
        doc.apply(&Change::transform_selected(&doc, |_| t).unwrap())
            .unwrap();
        let k = &doc.components[&id];
        let at = |x, y| k.height_at(Point::new(x, y)).unwrap();
        assert!(at(5.0, 9.0) > at(5.0, 1.0) + 6.0, "rises along y now");
        assert!((at(1.0, 5.0) - at(9.0, 5.0)).abs() < 0.6, "flat along x");
    }

    #[test]
    fn fade_and_tilt_anchors_move_with_it() {
        use cut_craft_core::component::Tilt;
        use cut_craft_core::params::Xy;
        let mut doc = dome_selected();
        let mut k = doc.components[&EntityId(1)].clone();
        k.shaping.tilt = Some(Tilt {
            pivot: Xy::new(12.0, 20.0),
            toward: Xy::new(28.0, 20.0),
            angle: 10.0,
        });
        let mut c = Change::builder(&doc);
        c.replace_component(EntityId(1), k);
        doc.apply(&c.finish()).unwrap();
        let shift = Point::new(3.0, 0.0);
        doc.apply(
            &Change::transform_selected(&doc, |_| DAffine2::from_translation(shift)).unwrap(),
        )
        .unwrap();
        let t = doc.components[&EntityId(1)].shaping.tilt.unwrap();
        assert_eq!(
            (t.pivot, t.toward),
            (Xy::new(15.0, 20.0), Xy::new(31.0, 20.0))
        );
    }
}

/// A 100-wide plaque and the two-letter word "II" grouped as text, all selected.
fn plaque_and_grouped_word() -> Document {
    let mut doc = Document::default();
    let mut c = Change::builder(&doc);
    let plaque = c.add(Path::polyline(
        &[
            Point::new(0.0, 0.0),
            Point::new(100.0, 0.0),
            Point::new(100.0, 40.0),
            Point::new(0.0, 40.0),
        ],
        true,
    ));
    let bar = |x: f64| {
        Path::polyline(
            &[
                Point::new(x, 10.0),
                Point::new(x + 2.0, 10.0),
                Point::new(x + 2.0, 30.0),
                Point::new(x, 30.0),
            ],
            true,
        )
    };
    let a = c.add(bar(0.0));
    let b = c.add(bar(10.0));
    c.set_group(a, Some(7));
    c.set_group(b, Some(7));
    c.select([plaque, a, b]);
    doc.apply(&c.finish()).unwrap();
    doc
}

#[test]
fn centring_moves_a_group_as_one_piece() {
    let mut doc = plaque_and_grouped_word();
    let change = Change::align_selected(&doc, |item, sel| {
        Point::new(sel.center().x - item.center().x, 0.0)
    })
    .unwrap();
    doc.apply(&change).unwrap();
    let a = doc.vectors[&EntityId(2)].bounds();
    let b = doc.vectors[&EntityId(3)].bounds();
    assert!(
        (b.min.x - a.min.x - 10.0).abs() < 1e-9,
        "letters kept apart: {a:?} {b:?}"
    );
    assert!((a.min.x - 44.0).abs() < 1e-9, "the word is centred: {a:?}");
}

#[test]
fn spacing_spaces_whole_groups() {
    let mut doc = Document::default();
    let mut c = Change::builder(&doc);
    let mut ids = Vec::new();
    // three two-letter words, each a group
    for (n, x) in [0.0, 30.0, 100.0].into_iter().enumerate() {
        for dx in [0.0, 5.0] {
            let id = c.add(Path::polyline(
                &[
                    Point::new(x + dx, 0.0),
                    Point::new(x + dx + 2.0, 0.0),
                    Point::new(x + dx + 2.0, 5.0),
                ],
                true,
            ));
            c.set_group(id, Some(n as u64 + 1));
            ids.push(id);
        }
    }
    c.select(ids.clone());
    doc.apply(&c.finish()).unwrap();
    let change =
        Change::space_selected(&doc, false, cut_craft_core::Sequence::ByPosition, false).unwrap();
    doc.apply(&change).unwrap();
    let x = |i: usize| doc.vectors[&ids[i]].bounds().min.x;
    assert!(
        (x(1) - x(0) - 5.0).abs() < 1e-9,
        "the first word holds together"
    );
    assert!(
        (x(3) - x(2) - 5.0).abs() < 1e-9,
        "the middle word holds together"
    );
    assert!(
        (x(2) - 50.0).abs() < 1e-9,
        "the middle word is spaced evenly: {}",
        x(2)
    );
}

#[test]
fn copies_of_a_group_are_groups_of_their_own_and_moved_text_stays_text() {
    let mut doc = plaque_and_grouped_word();
    doc.texts.insert(
        7,
        serde_json::json!({"text": "II", "anchor": {"x": 0.0, "y": 10.0}}),
    );
    doc.part_names.insert(7, "Word".into());
    let word = [EntityId(2), EntityId(3)];
    let shifts = [
        DAffine2::from_translation(Point::new(0.0, 50.0)),
        DAffine2::from_translation(Point::new(0.0, 100.0)),
    ];
    doc.apply(&Change::copy_vectors(&doc, &word, &shifts))
        .unwrap();
    let copies: Vec<EntityId> = doc.vectors.keys().copied().filter(|id| id.0 > 3).collect();
    assert_eq!(copies.len(), 4);
    let groups: Vec<u64> = copies.iter().map(|id| doc.vector_groups[id]).collect();
    assert_eq!(groups[0], groups[1], "the first copy is one group");
    assert_eq!(groups[2], groups[3], "so is the second");
    assert!(
        groups[0] != groups[2] && groups[0] != 7,
        "each copy its own group"
    );
    let text = &doc.texts[&groups[2]];
    assert_eq!(text["anchor"]["y"], 110.0, "the text moved with its copy");
    assert_eq!(doc.part_names[&groups[0]], "Word");
}

#[test]
fn a_mirrored_copy_of_text_is_grouped_curves() {
    let mut doc = plaque_and_grouped_word();
    doc.texts.insert(7, serde_json::json!({"text": "II"}));
    let word = [EntityId(2), EntityId(3)];
    let flip = DAffine2::from_scale(Point::new(-1.0, 1.0));
    doc.apply(&Change::copy_vectors(&doc, &word, &[flip]))
        .unwrap();
    let g = doc.vector_groups[&EntityId(4)];
    assert_eq!(doc.vector_groups[&EntityId(5)], g);
    assert!(
        !doc.texts.contains_key(&g),
        "mirrored letters cannot be edited as text"
    );
}

#[test]
fn a_moved_copy_of_a_dimension_is_grouped_curves() {
    let mut doc = plaque_and_grouped_word();
    doc.texts.insert(
        7,
        serde_json::json!({"command": "dimensions", "from": {"x": 0.0, "y": 0.0}, "to": {"x": 50.0, "y": 0.0}}),
    );
    let word = [EntityId(2), EntityId(3)];
    let shift = [DAffine2::from_translation(Point::new(0.0, 50.0))];
    doc.apply(&Change::copy_vectors(&doc, &word, &shift))
        .unwrap();
    let copy = doc.vector_groups[&EntityId(4)];
    assert_ne!(copy, 7, "the copy is a group of its own");
    assert!(
        !doc.texts.contains_key(&copy),
        "a dimension's settings point at what it measured, so its copy does not take them"
    );
}

#[test]
fn a_dimension_copied_onto_itself_is_still_a_dimension() {
    let mut doc = plaque_and_grouped_word();
    let record = serde_json::json!({"command": "dimensions", "from": {"x": 0.0, "y": 0.0}});
    doc.texts.insert(7, record.clone());
    let word = [EntityId(2), EntityId(3)];
    doc.apply(&Change::copy_vectors(&doc, &word, &[DAffine2::IDENTITY]))
        .unwrap();
    let copy = doc.vector_groups[&EntityId(4)];
    assert_eq!(
        doc.texts[&copy], record,
        "a pasted dimension can still be edited"
    );
}

mod rotary_solids {
    use super::*;
    use cut_craft_core::component::{Combine, Component};
    use cut_craft_core::document::{Along, Job};
    use cut_craft_core::glam::DVec3;

    /// A rotary job 40 mm across, its axis along `along`, holding a full 3D model: one
    /// triangle, at A=0 straight up 10 from the axis and one turned a quarter round.
    fn job(along: Along) -> (Document, EntityId) {
        let mut doc = Document {
            job: Job::rotary(60.0, 40.0, along),
            ..Document::default()
        };
        let mut model =
            Component::from_fn("Model", Combine::Add, doc.job.area(), 0.5, |_| Some(20.0));
        model.set_solid(&[[
            DVec3::new(10.0, 0.0, 10.0),
            DVec3::new(20.0, 10.0, 0.0),
            DVec3::new(30.0, 0.0, 10.0),
        ]]);
        let mut c = Change::builder(&doc);
        let id = c.add_component(model);
        doc.apply(&c.finish()).unwrap();
        doc.selected_components = [id].into();
        (doc, id)
    }

    fn close(a: DVec3, b: DVec3) -> bool {
        (a - b).length() < 1e-4
    }

    #[test]
    fn moving_a_full_model_moves_it_along_the_axis_and_turns_it_round() {
        // a quarter of the way round a 40 mm cylinder is 10 pi mm on the unrolled sheet
        let quarter = 10.0 * std::f64::consts::PI;
        for (along, by) in [
            (Along::X, Point::new(5.0, quarter)),
            (Along::Y, Point::new(quarter, 5.0)),
        ] {
            let (mut doc, id) = job(along);
            let change =
                Change::transform_selected(&doc, |_| DAffine2::from_translation(by)).unwrap();
            doc.apply(&change).unwrap();
            let moved = doc.components[&id].solid_triangles();
            assert_eq!(moved.len(), 1, "{along:?}: the model is kept whole");
            let [a, b, _] = moved[0];
            // 5 further along; straight up has turned the way A turns round, to +y
            assert!(close(a, DVec3::new(15.0, 10.0, 0.0)), "{along:?}: {a:?}");
            assert!(close(b, DVec3::new(25.0, 0.0, -10.0)), "{along:?}: {b:?}");
        }
    }

    #[test]
    fn a_full_model_is_not_scaled_turned_or_mirrored_on_the_sheet_and_is_told_why() {
        let (doc, _) = job(Along::X);
        let refused =
            Change::transform_selected(&doc, |_| DAffine2::from_scale(Point::new(2.0, 2.0)));
        match refused {
            Err(CommandError::Failed(why)) => {
                assert!(why.contains("Orientate 3D Model"), "{why}");
            }
            other => panic!("{other:?}"),
        }
    }
}
