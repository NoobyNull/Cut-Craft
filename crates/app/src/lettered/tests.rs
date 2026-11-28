use super::*;

/// A small drawing: a card, a line, and a letter D right of a dimension line.
const SMALL: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 200 100" width="200" height="100">
  <rect x="0.5" y="0.5" width="199" height="99" rx="10" fill="#FBFAF7" stroke="#D9D4CC"/>
  <path d="M40,20 L120,20 L120,80 L40,80 Z" fill="#EBCB98" stroke="#A9814F" stroke-width="2"/>
  <path d="M130,20 L130,80" stroke="#B0187E" stroke-width="1.2" stroke-dasharray="5 3"/>
  <path class="label" data-text="D" data-align="left center" d="M140 44H150V56H140Z" fill="#B0187E"/>
</svg>"##;

fn diagrams() -> Vec<(String, String)> {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/icons/diagrams");
    let mut all: Vec<(String, String)> = std::fs::read_dir(dir)
        .unwrap()
        .flatten()
        .filter(|e| e.path().extension().is_some_and(|x| x == "svg"))
        .map(|e| {
            (
                e.file_name().to_string_lossy().into_owned(),
                std::fs::read_to_string(e.path()).unwrap(),
            )
        })
        .collect();
    all.sort();
    all
}

#[test]
fn a_picture_names_its_letters_and_the_point_of_each_that_stays_put() {
    let l = read(SMALL).expect("has letters");
    assert_eq!(
        l.view_box,
        egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(200.0, 100.0))
    );
    assert_eq!(l.labels.len(), 1);
    let d = &l.labels[0];
    assert_eq!(d.text, "D");
    // right of its line, it keeps its left edge, half way up
    assert_eq!(d.align, egui::Align2::LEFT_CENTER);
    assert!(
        (d.at - egui::pos2(140.0, 50.0)).length() < 0.01,
        "{:?}",
        d.at
    );
    assert_eq!(d.colour, egui::Color32::from_rgb(0xb0, 0x18, 0x7e));
    assert_eq!(l.thinnest, 1.2);
}

#[test]
fn the_bare_picture_has_neither_its_letters_nor_its_card() {
    let l = read(SMALL).unwrap();
    assert!(!l.bare.contains("label"));
    assert!(!l.bare.contains("<rect"));
    assert!(l.bare.contains("#EBCB98"), "the drawing stays");
    // what is drawn reaches from the part to the dashed line, not the card's edges
    assert!(
        l.drawing.min.x > 30.0 && l.drawing.max.x < 135.0,
        "{:?}",
        l.drawing
    );
    assert!(
        l.drawing.min.y > 10.0 && l.drawing.max.y < 90.0,
        "{:?}",
        l.drawing
    );
    assert!(usvg::Tree::from_str(&l.bare, &usvg::Options::default()).is_ok());
}

#[test]
fn a_picture_without_letters_is_drawn_as_it_is() {
    assert!(read(&SMALL.replace(r#"class="label" "#, "")).is_none());
    assert!(read("not a picture").is_none());
}

#[test]
fn cut_to_its_drawing_and_made_thicker_a_picture_still_draws() {
    let l = read(SMALL).unwrap();
    let svg = l.svg(l.drawing, 2.5);
    assert!(svg.contains(r#"stroke-width="3""#), "1.2 × 2.5");
    assert!(svg.contains(r#"stroke-dasharray="12.5 7.5""#));
    let tree = usvg::Tree::from_str(&svg, &usvg::Options::default()).expect("parses");
    assert!((tree.size().width() - l.drawing.width()).abs() < 0.01);
}

#[test]
fn lines_are_drawn_thicker_only_when_they_would_come_out_under_a_pixel() {
    assert_eq!(thicker(1.5), 1.0);
    assert_eq!(thicker(0.5), 2.0);
    assert_eq!(thicker(0.3), 3.5);
    assert_eq!(thicker(0.0), 1.0);
}

#[test]
fn every_letter_stays_inside_the_room_given_however_small_the_picture() {
    let l = read(SMALL).unwrap();
    let letter = [egui::vec2(12.0, 16.0)];
    for (w, h) in [(400.0, 200.0), (120.0, 60.0), (60.0, 40.0)] {
        let space = egui::Rect::from_min_size(egui::pos2(10.0, 10.0), egui::vec2(w, h));
        for window in [l.view_box, l.drawing] {
            let (picture, letters) = place(&l, window, space, &letter);
            assert!(
                space.expand(0.6).contains_rect(picture),
                "{w}x{h}: {picture:?}"
            );
            assert!(
                space.expand(0.6).contains_rect(letters[0]),
                "{w}x{h}: {letters:?}"
            );
            assert_eq!(letters[0].size(), letter[0], "letters are never scaled");
        }
    }
}

#[test]
fn every_letter_in_a_measurement_picture_is_marked_as_one() {
    let mut lettered = 0;
    for (name, svg) in diagrams() {
        // the V-Carve order pictures number the corners the bit visits: eight numbers a few
        // points apart, which at text size would cover the shape they number
        if name.starts_with("vcarve_order_") || name == "vcarve_stay_down.svg" {
            continue;
        }
        let doc = roxmltree::Document::parse(&svg).expect(&name);
        // a letter is a filled magenta outline of its own, not a line or an arrowhead
        let unmarked = doc.descendants().filter(|n| {
            n.has_tag_name("path")
                && n.attribute("fill") == Some("#B0187E")
                && n.attribute("stroke").is_none()
                && !n.ancestors().any(|a| a.has_tag_name("marker"))
                && n.attribute("class") != Some("label")
        });
        assert_eq!(unmarked.count(), 0, "{name} has a letter not marked as one");
        let Some(l) = read(&svg) else { continue };
        lettered += 1;
        assert!(l.labels.iter().all(|x| !x.text.is_empty()), "{name}");
        assert!(l.card.is_some(), "{name} is drawn on a card");
        assert!(
            l.view_box.contains_rect(l.drawing.shrink(1.0)),
            "{name}: {:?}",
            l.drawing
        );
    }
    assert!(lettered >= 39, "only {lettered} pictures with letters");
}

/// The text shapes `show` paints for `SMALL` drawn `size` big.
fn letters_drawn(size: egui::Vec2) -> Vec<f32> {
    let ctx = egui::Context::default();
    egui_extras::install_image_loaders(&ctx);
    ctx.include_bytes("bytes://test/small.svg", SMALL.as_bytes().to_vec());
    let mut sizes = vec![];
    for _ in 0..2 {
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
            show(
                ui,
                &egui::Image::new("bytes://test/small.svg"),
                size,
                Fit::Cut,
            );
        });
        sizes = out
            .shapes
            .iter()
            .filter_map(|c| match &c.shape {
                egui::Shape::Text(t) => {
                    t.galley.job.sections.first().map(|s| s.format.font_id.size)
                }
                _ => None,
            })
            .collect();
        out.textures_delta.clear();
    }
    sizes
}

#[test]
fn letters_are_the_body_text_size_however_big_the_picture_is_drawn() {
    let body = egui::TextStyle::Body.resolve(&egui::Style::default()).size;
    let small = letters_drawn(egui::vec2(80.0, 50.0));
    let large = letters_drawn(egui::vec2(400.0, 200.0));
    assert_eq!(small, vec![body]);
    assert_eq!(large, vec![body]);
}
