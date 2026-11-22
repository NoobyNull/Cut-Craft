#[test]
fn the_samples_show_each_way_round_and_turned_ones_stay_in_the_band() {
    let params = |style: Style| {
        serde_json::to_value(Params {
            style,
            ..Params::default()
        })
        .unwrap()
    };
    let spread = |rings: &[Vec<Point>]| {
        rings
            .iter()
            .flatten()
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), q| {
                (lo.min(q.y), hi.max(q.y))
            })
    };
    for style in [Style::Chevrons, Style::FolkMix, Style::Feathers] {
        let p = params(style);
        // a space is empty (the vine runs straight through it)
        let (lo, hi) = spread(&sample(&p, "0"));
        assert!(hi - lo < 4.0 || lo > hi, "{style:?}: a space is empty");
        let one = sample(&p, "1");
        let two = sample(&p, "2");
        assert!(!one.is_empty() && !two.is_empty(), "{style:?}");
        assert_ne!(one, two, "{style:?}: 2 differs from 1");
        let (lo, hi) = spread(&sample(&p, "1/4"));
        assert!(
            lo >= -10.0 - 1e-6 && hi <= 10.0 + 1e-6,
            "{style:?}: {lo} {hi}"
        );
    }
    assert!(
        !sample(&params(Style::Twist), "1").is_empty(),
        "a plait draws whole"
    );
    assert!(matches!(
        sample(&serde_json::json!("nonsense"), "1").as_slice(),
        []
    ));
}

use super::*;
use cut_craft_core::Change;
use cut_craft_core::testing::{document_with, square};
use std::f64::consts::TAU;

fn framed() -> Document {
    document_with(vec![square(0.0, 0.0, 200.0)])
}

cut_craft_core::contract_tests!(Embellish, framed);

fn polygon(n: u32, r: f64) -> Path {
    let pts: Vec<Point> = (0..n)
        .map(|k| {
            Point::new(150.0, 150.0) + Point::from_angle(TAU * f64::from(k) / f64::from(n)) * r
        })
        .collect();
    Path::polyline(&pts, true)
}

fn lopsided() -> Path {
    let p = Point::new;
    Path::polyline(
        &[
            p(0.0, 0.0),
            p(240.0, 20.0),
            p(200.0, 120.0),
            p(120.0, 200.0),
            p(60.0, 90.0),
        ],
        true,
    )
}

fn made(doc: &Document, p: &Params) -> Document {
    let mut after = doc.clone();
    after.apply(&Embellish::run(doc, p).unwrap()).unwrap();
    after
}

fn selecting_components(doc: &Document) -> Document {
    let mut d = doc.clone();
    d.selection = cut_craft_core::selection::Selection::default();
    d.selected_components = d.components.keys().copied().collect();
    d
}

fn layer_vectors(doc: &Document, layer: cut_craft_core::document::LayerId) -> usize {
    doc.vectors
        .keys()
        .filter(|id| doc.layer_of(**id) == layer)
        .count()
}

#[test]
fn a_selected_embellishment_opens_with_its_own_settings_and_is_remade_in_place() {
    let doc = document_with(vec![polygon(6, 110.0)]);
    let p = Params {
        name: "Lid".into(),
        style: Style::Scrolls,
        objects: 3,
        ..Params::default()
    };
    let once = selecting_components(&made(&doc, &p));
    assert_eq!(
        Embellish::initial_params(&doc).editing,
        None,
        "nothing to edit yet"
    );
    let opened = Embellish::initial_params(&once);
    assert!(
        opened.editing.is_some(),
        "the component opens Embellish on itself"
    );
    assert_eq!(opened.style, Style::Scrolls);
    assert_eq!(opened.objects, 3);
    assert_eq!(opened.name, "Lid");
    let changed = Params {
        name: "Top".into(),
        objects: 9,
        ..opened
    };
    let twice = made(&once, &changed);
    assert_eq!(twice.layers.len(), once.layers.len(), "the same layer");
    assert_eq!(twice.components.len(), 1, "remade, not added");
    let layer = twice.layers.last().unwrap();
    assert_eq!(layer.name, "Top");
    let fresh = made(
        &doc,
        &Params {
            editing: None,
            ..changed.clone()
        },
    );
    assert_eq!(
        layer_vectors(&twice, layer.id),
        layer_vectors(&fresh, fresh.layers.last().unwrap().id),
        "new pieces replace the old"
    );
    assert_eq!(twice.components.values().next().unwrap().name, "Top");
    assert_eq!(
        Embellish::initial_params(&selecting_components(&twice)).objects,
        9,
        "and it remembers the new settings"
    );
    // undo brings back the first one exactly
    let mut undone = twice.clone();
    undone
        .apply(&Embellish::run(&once, &changed).unwrap().inverse())
        .unwrap();
    assert_eq!(undone.vectors.len(), once.vectors.len());
    assert_eq!(undone.layers, once.layers);
}

#[test]
fn its_pieces_or_the_outline_it_was_made_for_open_it_too() {
    let shape = polygon(6, 110.0);
    let doc = document_with(vec![shape.clone()]);
    let p = Params {
        objects: 2,
        ..Params::default()
    };
    let after = made(&doc, &p);
    assert!(
        Embellish::initial_params(&after).editing.is_some(),
        "the outline is still selected: Embellish edits its embellishment"
    );
    let layer = after.layers.last().unwrap().id;
    let piece = *after
        .vectors
        .keys()
        .find(|id| after.layer_of(**id) == layer)
        .unwrap();
    let mut picked = after.clone();
    picked.selection = [piece].into();
    let opened = Embellish::initial_params(&picked);
    assert!(opened.editing.is_some(), "a piece of it");
    assert_eq!(opened.objects, 2);
    // another outline is a new embellishment
    let mut other = after.clone();
    other
        .apply(&Change::swap(&other, [], vec![square(400.0, 0.0, 100.0)]))
        .unwrap();
    assert_eq!(Embellish::initial_params(&other).editing, None);
}

#[test]
fn vectors_only_keeps_its_settings_on_its_layer_and_can_become_a_carving() {
    let doc = document_with(vec![polygon(6, 110.0)]);
    let p = Params {
        make: Make::Vectors,
        ..Params::default()
    };
    let once = made(&doc, &p);
    assert!(once.components.is_empty());
    let opened = Embellish::initial_params(&once);
    assert!(opened.editing.is_some());
    let both = made(
        &once,
        &Params {
            make: Make::Both,
            ..opened
        },
    );
    assert_eq!(both.components.len(), 1, "a carving added to it");
    assert_eq!(both.layers.len(), once.layers.len());
    assert!(
        both.layers.last().unwrap().made.is_none(),
        "the component keeps the settings now"
    );
    let opened = Embellish::initial_params(&selecting_components(&both));
    let carving = made(
        &both,
        &Params {
            make: Make::Carving,
            ..opened
        },
    );
    assert_eq!(
        carving.layers.len(),
        doc.layers.len(),
        "its vectors and layer go"
    );
    assert_eq!(carving.vectors.len(), doc.vectors.len());
    assert_eq!(carving.components.len(), 1);
    let opened = Embellish::initial_params(&selecting_components(&carving));
    assert_eq!(opened.make, Make::Carving);
    let back = made(
        &carving,
        &Params {
            make: Make::Vectors,
            ..opened
        },
    );
    assert!(back.components.is_empty(), "the carving goes");
    assert_eq!(back.layers.len(), doc.layers.len() + 1);
    assert!(Embellish::initial_params(&back).editing.is_some());
}

#[test]
fn each_embellishment_goes_on_a_layer_of_its_own_named_for_it() {
    let shape = polygon(6, 110.0);
    let doc = document_with(vec![shape.clone()]);
    let p = Params {
        name: "Lid".into(),
        ..Params::default()
    };
    let once = made(&doc, &p);
    let twice = made(&once, &p);
    assert_eq!(
        twice.layers.len(),
        doc.layers.len() + 2,
        "a new layer each time"
    );
    assert_eq!(
        twice.current_layer, doc.current_layer,
        "drawing stays where it was"
    );
    for after in [&once, &twice] {
        let layer = after.layers.last().unwrap();
        assert_eq!(layer.name, "Lid");
        let on: Vec<_> = after
            .vectors
            .iter()
            .filter(|(id, _)| after.layer_of(**id) == layer.id)
            .collect();
        assert!(on.len() > 40, "{} pieces on it", on.len());
        assert!(
            on.iter().all(|(_, v)| **v != shape),
            "the outline stays put"
        );
    }
}

fn range_of(doc: &Document) -> (f64, f64) {
    let part = doc.components.values().next().unwrap();
    part.heights
        .iter()
        .filter(|h| !h.is_nan())
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), &h| {
            (lo.min(h), hi.max(h))
        })
}

#[test]
fn the_pieces_stand_up_sink_in_or_both_by_their_own_distances() {
    let doc = document_with(vec![polygon(6, 110.0)]);
    let with = |direction| Params {
        direction,
        height: 3.0,
        depth: 1.5,
        ..Params::default()
    };
    let (lo, hi) = range_of(&made(&doc, &with(Direction::Raised)));
    assert!((hi - 3.0).abs() < 0.15 && lo > -0.5, "raised {lo} {hi}");
    let (lo, hi) = range_of(&made(&doc, &with(Direction::Sunk)));
    assert!(hi <= 0.0 && (lo + 1.5).abs() < 0.3, "sunk {lo} {hi}");
    let (lo, hi) = range_of(&made(&doc, &with(Direction::Mixed)));
    assert!((hi - 3.0).abs() < 0.15 && lo < -1.0, "mixed {lo} {hi}");
    for style in [Style::FolkMix, Style::Scrolls] {
        let p = Params {
            style,
            sequence: "12".into(),
            ..with(Direction::Mixed)
        };
        let (pieces, n) = {
            let rings: Vec<Vec<Point>> = vec![polygon(6, 110.0).flatten(0.05)];
            let r = regions(&rings);
            design(
                &r[0].field,
                p.settings(4, p.gap()),
                p.pattern().as_ref(),
                false,
            )
        };
        let _ = n;
        let small = small_ones(&pieces, 0);
        let raised = small
            .iter()
            .zip(&pieces)
            .filter(|(s, it)| !**s && matches!(it, Item::Shape { sink: false, .. }));
        assert!(raised.count() >= 12, "the main pieces stay raised");
        assert!(small.iter().filter(|s| **s).count() >= 12, "fillers sink");
    }
}

#[test]
fn a_direction_needs_its_own_distance() {
    let doc = document_with(vec![polygon(6, 110.0)]);
    let sunk = Params {
        direction: Direction::Sunk,
        height: 0.0,
        ..Params::default()
    };
    assert!(Embellish::run(&doc, &sunk).is_ok(), "no height wanted");
    let flat = Params { depth: 0.0, ..sunk };
    assert!(Embellish::run(&doc, &flat).is_err());
}

#[test]
fn makes_grouped_vectors_and_one_carving_inside_the_shape() {
    for shape in [polygon(6, 110.0), lopsided()] {
        let doc = document_with(vec![shape.clone()]);
        let after = made(&doc, &Params::default());
        let new: Vec<&Path> = after.vectors.values().filter(|v| **v != shape).collect();
        assert!(new.len() > 20, "{} pieces", new.len());
        let bounds = shape.bounds();
        for v in &new {
            let b = v.bounds();
            assert!(
                b.min.x >= bounds.min.x - 0.5 && b.max.x <= bounds.max.x + 0.5,
                "{b:?}"
            );
        }
        assert_eq!(after.components.len(), 1);
        let part = after.components.values().next().unwrap();
        let top = part
            .heights
            .iter()
            .copied()
            .filter(|h| !h.is_nan())
            .fold(0.0, f64::max);
        assert!((top - 3.0).abs() < 0.3, "highest {top}");
    }
}

#[test]
fn a_seed_makes_the_same_design_and_another_seed_another() {
    let doc = document_with(vec![lopsided()]);
    let p = Params {
        make: Make::Vectors,
        ..Params::default()
    };
    let shapes = |d: &Document| {
        let mut v: Vec<String> = d
            .vectors
            .values()
            .map(|v| format!("{:?}", v.bounds()))
            .collect();
        v.sort();
        v
    };
    assert_eq!(shapes(&made(&doc, &p)), shapes(&made(&doc, &p)));
    // another number, another shape of piece (each repeat the same, see the vine's tests)
    let other = Params {
        seed: 5,
        ..p.clone()
    };
    assert_ne!(shapes(&made(&doc, &p)), shapes(&made(&doc, &other)));
    let random = |seed| Params {
        seed,
        random_size: true,
        ..p.clone()
    };
    assert_eq!(
        shapes(&made(&doc, &random(4))),
        shapes(&made(&doc, &random(4)))
    );
    assert_ne!(
        shapes(&made(&doc, &random(4))),
        shapes(&made(&doc, &random(5)))
    );
}

#[test]
fn unticked_pieces_are_left_out() {
    let doc = document_with(vec![lopsided()]);
    let count = |dots, leaves| {
        let p = Params {
            dots,
            leaves,
            make: Make::Vectors,
            ..Params::default()
        };
        made(&doc, &p).vectors.len()
    };
    assert!(count(true, true) > count(false, true));
    assert!(count(false, true) > count(false, false));
}

#[test]
fn the_ornament_reaches_the_middle() {
    let shape = polygon(4, 110.0);
    let doc = document_with(vec![shape]);
    let centre = Point::new(150.0, 150.0);
    let near_middle = |style| {
        let p = Params {
            style,
            make: Make::Vectors,
            ..Params::default()
        };
        made(&doc, &p)
            .vectors
            .values()
            .filter(|v| {
                let b = v.bounds();
                b.size().x < 60.0 && b.center().distance(centre) < 25.0
            })
            .count()
    };
    assert!(near_middle(Style::FolkMix) > 0);
}

#[test]
fn every_patterned_style_fills_a_closed_shape_row_after_row() {
    let shape = polygon(6, 110.0);
    let doc = document_with(vec![shape.clone()]);
    let centre = Point::new(150.0, 150.0);
    for style in [
        Style::BandBorder,
        Style::Rope,
        Style::Chain,
        Style::BeadAndReel,
        Style::Laurel,
        Style::Feathers,
        Style::Chevrons,
    ] {
        let p = Params {
            style,
            make: Make::Vectors,
            ..Params::default()
        };
        let rows = made(&doc, &p);
        let near_middle = rows
            .vectors
            .values()
            .filter(|v| v.bounds().center().distance(centre) < 40.0)
            .count();
        assert!(near_middle > 0, "{style:?}: rows reach the middle");
    }
}

#[test]
fn no_style_is_named_a_border() {
    let schema = serde_json::to_string(&schemars::schema_for!(Params)).unwrap();
    assert!(
        !schema.contains(" border\""),
        "a style title ends in border"
    );
    assert!(!schema.contains("Fill the shape"));
}

/// A band `w` mm wide round a 200 mm square.
fn band(w: f64) -> Document {
    document_with(vec![square(0.0, 0.0, 200.0), square(w, w, 200.0 - 2.0 * w)])
}

#[test]
fn a_border_too_wide_for_a_narrow_band_is_made_narrower_to_fit() {
    let doc = band(12.0);
    for style in [Style::Rope, Style::Chain, Style::Chevrons, Style::Laurel] {
        let p = Params {
            style,
            border_width: 30.0,
            make: Make::Vectors,
            ..Params::default()
        };
        let after = match Embellish::run(&doc, &p) {
            Ok(c) => {
                let mut a = doc.clone();
                a.apply(&c).unwrap();
                a
            }
            Err(e) => panic!("{style:?}: {e}"),
        };
        let pieces = after.vectors.len() - doc.vectors.len();
        assert!(pieces > 40, "{style:?}: {pieces}");
        // every piece stays inside the band
        for v in after.vectors.values() {
            let b = v.bounds();
            let inner = Rect::new(Point::splat(12.0), Point::splat(188.0));
            let across = b.min.x.max(b.min.y) >= -0.01 && b.max.x.max(b.max.y) <= 200.01;
            let clear = !(b.min.x > inner.min.x + 0.01
                && b.min.y > inner.min.y + 0.01
                && b.max.x < inner.max.x - 0.01
                && b.max.y < inner.max.y - 0.01);
            assert!(across && clear, "{style:?}: {b:?}");
        }
    }
}

#[test]
fn too_narrow_a_shape_is_still_drawn_with_a_note_that_it_is_too_fine_to_carve() {
    let doc = band(4.0);
    for style in [Style::Rope, Style::Chain, Style::FolkMix, Style::Laurel] {
        let p = Params {
            style,
            ..Params::default()
        };
        let change = Embellish::run(&doc, &p).unwrap_or_else(|e| panic!("{style:?}: {e}"));
        let mut after = doc.clone();
        after.apply(&change).unwrap();
        assert!(
            after.vectors.len() > doc.vectors.len() + 10,
            "{style:?}: {}",
            after.vectors.len() - doc.vectors.len()
        );
        let note = change.notes.join(" ");
        assert!(
            note.contains("smaller than it will carve"),
            "{style:?}: {note}"
        );
    }
    // a shape it fits makes no such note
    let fits = Embellish::run(&band(40.0), &Params::default()).unwrap();
    assert!(fits.notes.is_empty(), "{:?}", fits.notes);
}

#[test]
fn a_tiny_shapes_pieces_are_made_small_enough_to_stay_inside_it() {
    let doc = document_with(vec![square(10.0, 10.0, 3.0)]);
    let change = Embellish::run(&doc, &Params::default()).unwrap();
    let mut after = doc.clone();
    after.apply(&change).unwrap();
    // (to the field's half cell: 0.25 mm)
    let inside = Rect::new(Point::splat(9.75), Point::splat(13.25));
    for v in after.vectors.values() {
        for q in v.flatten(0.01) {
            assert!(inside.contains(q), "{q:?}");
        }
    }
    assert!(
        change
            .notes
            .join(" ")
            .contains("smaller than it will carve")
    );
}

#[test]
fn tightness_sets_the_gap() {
    let p = |tightness| Params {
        tightness,
        ..Params::default()
    };
    assert_eq!(p(1).gap(), 3.0);
    assert_eq!(p(10).gap(), 0.75);
}

#[test]
fn needs_a_vector() {
    let err = Embellish::run(&Document::default(), &Params::default()).unwrap_err();
    assert!(
        err.to_string().to_lowercase().contains("select a vector"),
        "{err}"
    );
}

/// An open S-curve from (20, 100) to (320, 100).
fn wave() -> Path {
    let pts: Vec<Point> = (0..=150)
        .map(|i| {
            let x = f64::from(i) * 2.0;
            Point::new(20.0 + x, 100.0 + 30.0 * (x / 300.0 * TAU).sin())
        })
        .collect();
    Path::polyline(&pts, false)
}

fn pieces_of(after: &Document, line: &Path) -> Vec<Path> {
    after
        .vectors
        .values()
        .filter(|v| *v != line)
        .cloned()
        .collect()
}

#[test]
fn every_patterned_style_runs_along_an_open_line_from_end_to_end() {
    let line = wave();
    let flat = line.flatten(0.05);
    let doc = document_with(vec![line.clone()]);
    for style in [
        Style::FolkMix,
        Style::Scrolls,
        Style::Flowers,
        Style::BandBorder,
        Style::Rope,
        Style::Chain,
        Style::Twist,
        Style::Braid,
        Style::BeadAndReel,
        Style::Laurel,
        Style::Feathers,
        Style::Chevrons,
    ] {
        let p = Params {
            style,
            make: Make::Vectors,
            ..Params::default()
        };
        let after = made(&doc, &p);
        let pieces = pieces_of(&after, &line);
        assert!(pieces.len() > 8, "{style:?}: {}", pieces.len());
        let (mut first, mut last) = (f64::INFINITY, f64::INFINITY);
        for v in &pieces {
            for q in v.flatten(0.1) {
                let d = cut_craft_core::geometry::polyline_distance(&flat, q);
                assert!(
                    d < p.border_width / 2.0 + 0.6,
                    "{style:?}: {d} off the line"
                );
                first = first.min(q.distance(flat[0]));
                last = last.min(q.distance(*flat.last().unwrap()));
            }
        }
        assert!(
            first < 20.0 && last < 20.0,
            "{style:?}: ends {first} {last}"
        );
    }
}

#[test]
fn an_open_lines_carving_lies_along_it_and_selecting_the_line_reopens_it() {
    let line = wave();
    let doc = document_with(vec![line.clone()]);
    let p = Params {
        style: Style::Rope,
        objects: 3,
        ..Params::default()
    };
    let after = made(&doc, &p);
    assert_eq!(after.components.len(), 1);
    let part = after.components.values().next().unwrap();
    let top = part
        .heights
        .iter()
        .copied()
        .filter(|h| !h.is_nan())
        .fold(0.0, f64::max);
    assert!((top - p.height).abs() < 0.4, "highest {top}");
    // the line is still selected: Embellish opens on its embellishment
    let opened = Embellish::initial_params(&after);
    assert!(opened.editing.is_some());
    assert_eq!(opened.style, Style::Rope);
    let again = made(
        &after,
        &Params {
            style: Style::Chain,
            ..opened
        },
    );
    assert_eq!(again.components.len(), 1, "remade, not added");
    assert_eq!(again.layers.len(), after.layers.len());
}

#[test]
fn a_closed_shape_and_an_open_line_are_embellished_together() {
    let line = wave();
    let shape = square(400.0, 0.0, 200.0);
    let doc = document_with(vec![line.clone(), shape.clone()]);
    let p = Params {
        style: Style::Laurel,
        make: Make::Vectors,
        ..Params::default()
    };
    let after = made(&doc, &p);
    let pieces = pieces_of(&after, &line);
    let on_line = pieces.iter().filter(|v| v.bounds().max.x < 360.0).count();
    let in_square = pieces.iter().filter(|v| v.bounds().min.x > 395.0).count();
    assert!(on_line > 8 && in_square > 8, "{on_line} {in_square}");
}
