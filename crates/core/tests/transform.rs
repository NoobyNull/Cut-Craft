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
