//! Import vectors from SVG: every subpath becomes a vector, in millimetres, y up, on the
//! layer it was on in the program that made it, with that layer's name and colour. An SVG
//! sized in pixels (or not sized) is read at 96 pixels per inch, and a note says the size it
//! came in at, since programs that use 72 per inch make it come in 25% small.

use cut_craft_core::document::{Document, Units};
use cut_craft_core::geometry::{Path, Point, Segment};
use cut_craft_core::import_layers::{FileLayer, ImportLayers};
use cut_craft_core::{Change, CommandError, FileFormat, Importer};
use std::collections::BTreeMap;
use usvg::tiny_skia_path::{self as skia, PathSegment};
use usvg::{NodeKind, TreeParsing};

/// usvg resolves every unit to CSS pixels at 96 per inch.
const MM_PER_PX: f64 = 25.4 / 96.0;

pub struct ImportSvg;

impl Importer for ImportSvg {
    const ID: &'static str = "import_svg";
    const FORMAT: FileFormat = FileFormat {
        label: "SVG",
        extensions: &["svg"],
    };

    fn import(doc: &Document, bytes: &[u8]) -> Result<Change, CommandError> {
        let text = String::from_utf8_lossy(bytes);
        let (shown, found) = svg_layers(&text);
        let mut tree = usvg::Tree::from_data(shown.as_bytes(), &usvg::Options::default())
            .map_err(|e| CommandError::Failed(format!("not a readable SVG: {e}")))?;
        tree.calculate_abs_transforms();
        let to_px =
            usvg::utils::view_box_to_transform(tree.view_box.rect, tree.view_box.aspect, tree.size);
        let height_mm = f64::from(tree.size.height()) * MM_PER_PX;

        let mut c = Change::builder(doc);
        let mut ids = vec![];
        // each vector with the index of its layer in `found`, if it is on one
        let mut placed: Vec<(Path, Option<usize>)> = vec![];
        // per layer, how many of its paths are drawn in each colour
        let mut colours: Vec<BTreeMap<[u8; 3], usize>> = vec![BTreeMap::new(); found.len()];
        for node in tree.root.descendants() {
            let NodeKind::Path(ref path) = *node.borrow() else {
                continue;
            };
            let layer = node.ancestors().find_map(|a| match *a.borrow() {
                NodeKind::Group(ref g) if !g.id.is_empty() => {
                    found.iter().position(|l| l.id == g.id)
                }
                _ => None,
            });
            if let (Some(at), Some(colour)) = (layer, drawn_in(path)) {
                *colours[at].entry(colour).or_default() += 1;
            }
            // a path's points are in its parent group's space
            let group = node.parent().and_then(|g| match *g.borrow() {
                NodeKind::Group(ref g) => Some(g.abs_transform),
                _ => None,
            });
            let t = to_px.pre_concat(group.unwrap_or_default());
            let to_mm = |p: skia::Point| {
                let mut q = p;
                t.map_point(&mut q);
                Point::new(
                    f64::from(q.x) * MM_PER_PX,
                    height_mm - f64::from(q.y) * MM_PER_PX,
                )
            };
            placed.extend(subpaths(&path.data, to_mm).into_iter().map(|p| (p, layer)));
        }
        if placed.is_empty() {
            return Err(CommandError::Failed(
                "the SVG has no vector paths. Export it again with its shapes as paths".into(),
            ));
        }
        // the top of the source program's layer list is the last layer drawn
        let top_down: Vec<FileLayer> = found
            .iter()
            .zip(&colours)
            .rev()
            .map(|(l, counted)| FileLayer {
                name: l.name.clone(),
                colour: most_used(counted),
                visible: l.visible,
            })
            .collect();
        let mut layers = ImportLayers::make(&mut c, &top_down);
        for (p, layer) in placed {
            let id = c.add(p);
            let shown = match layer {
                Some(at) => {
                    let (layer, shown) = layers.of(&mut c, &found[at].name);
                    c.move_to_layer(id, layer);
                    shown
                }
                None => true,
            };
            // hidden vectors cannot be picked, so they are not selected either
            if shown {
                ids.push(id);
            }
        }
        if !real_size(bytes) {
            let (w, h) = (
                f64::from(tree.size.width()) * MM_PER_PX,
                f64::from(tree.size.height()) * MM_PER_PX,
            );
            let size = match doc.job.units {
                Units::Mm => format!("{w:.1} x {h:.1} mm"),
                Units::Inch => format!("{:.2} x {:.2} in", w / 25.4, h / 25.4),
            };
            c.note(format!(
                "The SVG gives its size in pixels, so it was read at 96 pixels per inch and came \
                 in at {size}. If it came from a program that uses 72 per inch, it is 25% too \
                 small: put it right with Set Size."
            ));
        }
        c.select(ids);
        Ok(c.finish())
    }
}

/// A layer of the SVG as the program that made it saved it: the `id` of its group (one is
/// given to a group without), the name shown in that program and whether it was shown.
#[derive(Clone, Debug, PartialEq)]
struct SvgLayer {
    id: String,
    name: String,
    visible: bool,
}

const INKSCAPE: &str = "http://www.inkscape.org/namespaces/inkscape";
const SERIF: &str = "http://www.serif.com/";

/// The SVG's layers, in drawing order, and its text with every layer group given an id and
/// its hidden layers shown, so their vectors come in too (on a hidden layer).
///
/// Layers are the groups marked as layers (`inkscape:groupmode="layer"`, from Inkscape and
/// programs that follow it), else the named groups straight under the `<svg>` (how
/// Illustrator, Affinity, `CorelDRAW` and others save layers). Named by the layer's label
/// (`inkscape:label`, `data-name`, `serif:id`), else its id.
fn svg_layers(text: &str) -> (std::borrow::Cow<'_, str>, Vec<SvgLayer>) {
    let options = roxmltree::ParsingOptions {
        allow_dtd: true,
        ..roxmltree::ParsingOptions::default()
    };
    let Ok(xml) = roxmltree::Document::parse_with_options(text, options) else {
        return (text.into(), vec![]);
    };
    let label = |g: &roxmltree::Node| {
        [
            g.attribute((INKSCAPE, "label")),
            g.attribute("data-name"),
            g.attribute((SERIF, "id")),
            g.attribute("id"),
        ]
        .into_iter()
        .flatten()
        .map(str::trim)
        .find(|n| !n.is_empty())
        .map(str::to_owned)
    };
    let root = xml.root_element();
    let marked: Vec<roxmltree::Node> = root
        .descendants()
        .filter(|n| n.has_tag_name("g") && n.attribute((INKSCAPE, "groupmode")) == Some("layer"))
        .collect();
    let groups = if marked.is_empty() {
        root.children()
            .filter(|n| n.has_tag_name("g") && label(n).is_some())
            .collect()
    } else {
        marked
    };
    let mut layers = vec![];
    // (where a start tag is, how long, what replaces it), in the text's order
    let mut edits: Vec<(usize, usize, String)> = vec![];
    let mut seen_ids: Vec<&str> = vec![];
    for (i, g) in groups.iter().enumerate() {
        let start = g.range().start;
        let tag = start_tag(&text[start..]);
        let mut new_tag = tag.to_owned();
        let given = g
            .attribute("id")
            .filter(|id| !id.is_empty() && !seen_ids.contains(id));
        let id = if let Some(id) = given {
            seen_ids.push(id);
            id.to_owned()
        } else {
            let id = format!("cut-craft-layer-{i}");
            // after the tag's name: "<g" or "<svg:g"
            let name_end = tag.find(char::is_whitespace).unwrap_or(tag.len() - 1);
            new_tag.insert_str(name_end, &format!(" id=\"{id}\""));
            id
        };
        let hidden = hides(tag);
        if hidden {
            new_tag = shown_tag(&new_tag);
        }
        if new_tag != tag {
            edits.push((start, tag.len(), new_tag));
        }
        layers.push(SvgLayer {
            name: label(g).unwrap_or_else(|| format!("Layer {}", i + 1)),
            id,
            visible: !hidden,
        });
    }
    if edits.is_empty() {
        return (text.into(), layers);
    }
    let mut out = text.to_owned();
    for (at, len, with) in edits.into_iter().rev() {
        out.replace_range(at..at + len, &with);
    }
    (out.into(), layers)
}

/// An element's start tag, `<` to `>`, from the text starting at it.
fn start_tag(from: &str) -> &str {
    let mut quote = None;
    for (i, ch) in from.char_indices() {
        match (quote, ch) {
            (None, '"' | '\'') => quote = Some(ch),
            (Some(q), _) if ch == q => quote = None,
            (None, '>') => return &from[..=i],
            _ => {}
        }
    }
    from
}

/// The hiding a start tag says, as `display` or `visibility`, in an attribute or its style.
const HIDING: [(&str, &str); 2] = [("display", "none"), ("visibility", "hidden")];

/// Whether a start tag hides its element.
fn hides(tag: &str) -> bool {
    HIDING.iter().any(|&(property, value)| {
        attribute(tag, property).is_some_and(|v| v.trim() == value)
            || attribute(tag, "style").is_some_and(|style| {
                style.split(';').any(|d| {
                    d.split_once(':')
                        .is_some_and(|(k, v)| k.trim() == property && v.trim() == value)
                })
            })
    })
}

/// A start tag with its hiding taken out, so what it holds is read.
fn shown_tag(tag: &str) -> String {
    let mut shown = tag.to_owned();
    for (property, value) in HIDING {
        let open = if property == "display" {
            "inline"
        } else {
            "visible"
        };
        for q in ['"', '\''] {
            shown = shown.replace(
                &format!("{property}={q}{value}{q}"),
                &format!("{property}={q}{open}{q}"),
            );
        }
        if let Some(style) = attribute(&shown, "style").map(str::to_owned) {
            let fixed: Vec<String> = style
                .split(';')
                .map(|d| match d.split_once(':') {
                    Some((k, v)) if k.trim() == property && v.trim() == value => {
                        format!("{property}:{open}")
                    }
                    _ => d.to_owned(),
                })
                .collect();
            shown = shown.replacen(&style, &fixed.join(";"), 1);
        }
    }
    shown
}

/// The colour a path is drawn in: its stroke's, else its fill's, if a plain colour.
fn drawn_in(path: &usvg::Path) -> Option<[u8; 3]> {
    let paint = path
        .stroke
        .as_ref()
        .map(|s| &s.paint)
        .or_else(|| path.fill.as_ref().map(|f| &f.paint))?;
    match paint {
        usvg::Paint::Color(c) => Some([c.red, c.green, c.blue]),
        _ => None,
    }
}

/// A layer's colour: the one most of its paths are drawn in (on a tie, the lowest). Black
/// and white draw in the usual vector colour, so they show on a light and a dark screen.
fn most_used(counted: &BTreeMap<[u8; 3], usize>) -> Option<[u8; 3]> {
    let most = counted.values().copied().max()?;
    counted
        .iter()
        .find(|&(_, &n)| n == most)
        .map(|(&c, _)| c)
        .filter(|c| *c != [0, 0, 0] && *c != [255, 255, 255])
}

/// Whether the SVG's own `width` and `height` are in real-world units (mm, cm, in, pt, pc),
/// so its size does not depend on how many pixels make an inch.
fn real_size(bytes: &[u8]) -> bool {
    let text = String::from_utf8_lossy(bytes);
    let Some(start) = text.find("<svg") else {
        return false;
    };
    let tag = &text[start..];
    let tag = &tag[..tag.find('>').unwrap_or(tag.len())];
    let real = |name: &str| {
        attribute(tag, name).is_some_and(|v| {
            let v = v.trim().to_ascii_lowercase();
            ["mm", "cm", "in", "pt", "pc"]
                .iter()
                .any(|u| v.ends_with(u))
        })
    };
    real("width") && real("height")
}

/// The value of attribute `name` in an element's start `tag`.
fn attribute<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
    let mut from = 0;
    while let Some(at) = tag[from..].find(name).map(|i| i + from) {
        from = at + name.len();
        let before = tag[..at].chars().next_back();
        if !before.is_some_and(char::is_whitespace) {
            continue;
        }
        let rest = tag[from..].trim_start();
        let Some(rest) = rest.strip_prefix('=') else {
            continue;
        };
        let rest = rest.trim_start();
        let quote = rest.chars().next().filter(|q| *q == '"' || *q == '\'')?;
        let value = &rest[1..];
        return value.find(quote).map(|end| &value[..end]);
    }
    None
}

/// Splits a path at each move-to; quadratics become cubics (exactly).
fn subpaths(data: &skia::Path, to_mm: impl Fn(skia::Point) -> Point) -> Vec<Path> {
    // ponytail: a drawing segment straight after Close (no MoveTo) continues the closed path;
    // usvg always emits a MoveTo there, so this has not been needed
    let mut done = vec![];
    let mut current: Option<Path> = None;
    let flush = |current: &mut Option<Path>, done: &mut Vec<Path>| {
        if let Some(p) = current.take().filter(|p| !p.segments.is_empty()) {
            done.push(p.closed_where_ends_meet());
        }
    };
    for segment in data.segments() {
        if let PathSegment::MoveTo(p) = segment {
            flush(&mut current, &mut done);
            current = Some(Path {
                start: to_mm(p),
                segments: vec![],
                closed: false,
            });
            continue;
        }
        let Some(path) = current.as_mut() else {
            continue;
        };
        let from = path.segments.last().map_or(path.start, Segment::end);
        match segment {
            PathSegment::LineTo(p) => path.segments.push(Segment::Line { to: to_mm(p) }),
            PathSegment::QuadTo(c, p) => {
                let (c, to) = (to_mm(c), to_mm(p));
                path.segments.push(Segment::Cubic {
                    c1: from + (c - from) * (2.0 / 3.0),
                    c2: to + (c - to) * (2.0 / 3.0),
                    to,
                });
            }
            PathSegment::CubicTo(c1, c2, p) => path.segments.push(Segment::Cubic {
                c1: to_mm(c1),
                c2: to_mm(c2),
                to: to_mm(p),
            }),
            PathSegment::Close => {
                if from.distance(path.start) > 0.0 {
                    path.segments.push(Segment::Line { to: path.start });
                }
                path.closed = true;
            }
            PathSegment::MoveTo(_) => unreachable!("handled above"),
        }
    }
    flush(&mut current, &mut done);
    done
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::Exporter;
    use cut_craft_core::geometry::{Point, Rect};
    use cut_craft_core::testing::sample_document;

    fn import(svg: &str) -> Document {
        let mut doc = Document::default();
        let change = ImportSvg::import(&doc, svg.as_bytes()).unwrap();
        doc.apply(&change).unwrap();
        doc
    }

    fn all_bounds(doc: &Document) -> Rect {
        doc.vectors
            .values()
            .map(cut_craft_core::geometry::Path::bounds)
            .reduce(Rect::union)
            .unwrap()
    }

    #[test]
    fn our_own_export_comes_back_the_same_shape_and_place() {
        let original = sample_document();
        let svg = String::from_utf8(fmt_export_svg::ExportSvg::export(&original).unwrap()).unwrap();
        let doc = import(&svg);
        assert_eq!(doc.vectors.len(), 4);
        let (a, b) = (all_bounds(&doc), original.selection_bounds().unwrap());
        assert!(
            a.min.distance(b.min) < 1e-3 && a.max.distance(b.max) < 1e-3,
            "{a:?} vs {b:?}"
        );
    }

    #[test]
    fn pixel_units_are_converted_to_millimetres_with_y_up() {
        // 96 px = 1 inch; the rect is 30 x 40 px, 20 px below the top of a 100 px tall canvas
        let doc = import(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100"><rect x="10" y="20" width="30" height="40"/></svg>"#,
        );
        let b = all_bounds(&doc);
        let mm = 25.4 / 96.0;
        let want = Rect::new(
            Point::new(10.0 * mm, 40.0 * mm),
            Point::new(40.0 * mm, 80.0 * mm),
        );
        assert!(
            b.min.distance(want.min) < 1e-3 && b.max.distance(want.max) < 1e-3,
            "{b:?} vs {want:?}"
        );
    }

    #[test]
    fn a_size_in_pixels_or_none_says_what_it_was_read_at() {
        let notes = |svg: &str| {
            ImportSvg::import(&Document::default(), svg.as_bytes())
                .unwrap()
                .notes
                .join(" ")
        };
        let rect = r#"<rect x="0" y="0" width="72" height="36"/>"#;
        for size in [
            r#"width="288" height="144""#,
            r#"width="288px" height="144px""#,
            "",
        ] {
            let told = notes(&format!(
                r#"<svg xmlns="http://www.w3.org/2000/svg" {size} viewBox="0 0 288 144">{rect}</svg>"#
            ));
            assert!(
                told.contains("96 pixels per inch") && told.contains("Set Size"),
                "{size}: {told}"
            );
            assert!(told.contains("72"), "names the other common size: {told}");
        }
        // a size in real units needs no note
        let told = notes(&format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="100mm" height="50mm" viewBox="0 0 100 50">{rect}</svg>"#
        ));
        assert!(told.is_empty(), "{told}");
    }

    #[test]
    fn each_subpath_becomes_its_own_vector() {
        let doc = import(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100"><path d="M0 0 L10 0 M20 0 L30 0" stroke="black"/></svg>"#,
        );
        assert_eq!(doc.vectors.len(), 2);
    }

    #[test]
    fn imported_vectors_are_selected() {
        let doc = import(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"><circle cx="5" cy="5" r="4"/></svg>"#,
        );
        assert_eq!(doc.selection.len(), 1);
        assert!(doc.vectors.values().next().unwrap().closed);
    }

    #[test]
    fn a_path_that_ends_where_it_starts_is_closed_without_a_z() {
        // as Vectric exports them: back to the first point with L, no Z
        let doc = import(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100"><path d="M 10,10 L 40,10 L 40,40 L 10,10" stroke="black"/><path d="M 50,10 L 80,10 L 80,40" stroke="black"/></svg>"#,
        );
        let mut closed: Vec<bool> = doc.vectors.values().map(|p| p.closed).collect();
        closed.sort_unstable();
        assert_eq!(
            closed,
            [false, true],
            "the triangle closed, the open line not"
        );
    }

    #[test]
    fn garbage_is_an_error_not_a_panic() {
        assert!(ImportSvg::import(&Document::default(), b"definitely not svg").is_err());
        let empty = r#"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"/>"#;
        assert_eq!(
            ImportSvg::import(&Document::default(), empty.as_bytes())
                .unwrap_err()
                .to_string(),
            "The SVG has no vector paths. Export it again with its shapes as paths."
        );
    }

    /// Three Inkscape layers, bottom to top: Outline (red), Holes (blue, hidden) and Notes
    /// (nothing on it); one circle outside any layer.
    const INKSCAPE_LAYERS: &str = r##"<svg xmlns="http://www.w3.org/2000/svg"
        xmlns:inkscape="http://www.inkscape.org/namespaces/inkscape"
        width="100mm" height="100mm" viewBox="0 0 100 100">
      <g inkscape:groupmode="layer" id="layer1" inkscape:label="Outline">
        <rect x="10" y="10" width="80" height="80" style="fill:none;stroke:#ff0000"/>
        <rect x="20" y="20" width="10" height="10" style="fill:none;stroke:#ff0000"/>
      </g>
      <g inkscape:groupmode="layer" inkscape:label="Holes" style="display:none">
        <circle cx="50" cy="50" r="5" stroke="#0000ff" fill="none"/>
      </g>
      <g inkscape:groupmode="layer" id="layer3" inkscape:label="Notes"/>
      <circle cx="80" cy="80" r="2" stroke="black" fill="none"/>
    </svg>"##;

    fn named<'a>(doc: &'a Document, name: &str) -> &'a cut_craft_core::document::Layer {
        doc.layers
            .iter()
            .find(|l| l.name == name)
            .unwrap_or_else(|| panic!("no layer {name}"))
    }

    fn top_down(doc: &Document) -> Vec<&str> {
        doc.layers.iter().rev().map(|l| l.name.as_str()).collect()
    }

    #[test]
    fn inkscape_layers_come_in_by_name_listed_as_inkscape_lists_them() {
        let doc = import(INKSCAPE_LAYERS);
        assert_eq!(top_down(&doc), ["Notes", "Holes", "Outline", "Layer 1"]);
        assert_eq!(doc.vectors_on(named(&doc, "Outline").id).count(), 2);
        assert_eq!(doc.vectors_on(named(&doc, "Holes").id).count(), 1);
        assert_eq!(doc.vectors_on(named(&doc, "Notes").id).count(), 0, "kept");
        assert_eq!(
            doc.vectors_on(named(&doc, "Layer 1").id).count(),
            1,
            "not on one"
        );
    }

    #[test]
    fn a_layers_colour_is_what_its_paths_are_drawn_in() {
        let doc = import(INKSCAPE_LAYERS);
        assert_eq!(named(&doc, "Outline").colour, Some([255, 0, 0]));
        assert_eq!(named(&doc, "Holes").colour, Some([0, 0, 255]));
        assert_eq!(named(&doc, "Notes").colour, None);
    }

    #[test]
    fn a_hidden_layer_comes_in_hidden_with_its_vectors_unselected() {
        let doc = import(INKSCAPE_LAYERS);
        let holes = named(&doc, "Holes");
        assert!(!holes.visible);
        assert!(named(&doc, "Outline").visible);
        assert_eq!(doc.vectors.len(), 4);
        assert_eq!(doc.selection.len(), 3);
        assert!(doc.selection.iter().all(|&id| doc.layer_of(id) != holes.id));
    }

    #[test]
    fn named_top_level_groups_are_layers_as_illustrator_saves_them() {
        let doc = import(
            r##"<?xml version="1.0" encoding="utf-8"?>
            <!DOCTYPE svg PUBLIC "-//W3C//DTD SVG 1.1//EN" "http://www.w3.org/Graphics/SVG/1.1/DTD/svg11.dtd">
            <svg xmlns="http://www.w3.org/2000/svg" width="50mm" height="50mm" viewBox="0 0 50 50">
              <defs><style>.a{fill:none;stroke:#00a651;}</style></defs>
              <g id="Cut_lines" data-name="Cut lines">
                <rect class="a" x="1" y="1" width="40" height="40"/>
              </g>
              <g id="Engrave"><path d="M5 5 L20 20" stroke="#2e3192"/></g>
              <g transform="translate(1 1)"><path d="M0 0 L5 0" stroke="#000"/></g>
            </svg>"##,
        );
        assert_eq!(top_down(&doc), ["Engrave", "Cut lines", "Layer 1"]);
        assert_eq!(named(&doc, "Cut lines").colour, Some([0, 166, 81]));
        assert_eq!(named(&doc, "Engrave").colour, Some([46, 49, 146]));
        assert_eq!(
            doc.vectors_on(named(&doc, "Layer 1").id).count(),
            1,
            "an unnamed group is not a layer"
        );
    }

    #[test]
    fn a_layer_named_as_one_the_job_has_is_used() {
        let mut doc = Document::default();
        let mut c = Change::builder(&doc);
        let mine = c.add_layer("outline");
        doc.apply(&c.finish()).unwrap();
        doc.apply(&ImportSvg::import(&doc, INKSCAPE_LAYERS.as_bytes()).unwrap())
            .unwrap();
        assert_eq!(doc.vectors_on(mine).count(), 2);
        assert!(!doc.layers.iter().any(|l| l.name == "Outline"));
    }

    #[test]
    fn hiding_inside_a_style_or_attribute_is_found_and_taken_out() {
        let tag = r#"<g id="a" style="opacity:1; display : none ;fill:red" visibility="hidden">"#;
        assert!(hides(tag));
        let shown = shown_tag(tag);
        assert!(!hides(&shown), "{shown}");
        assert!(
            shown.contains("fill:red") && shown.contains(r#"id="a""#),
            "{shown}"
        );
        assert!(!hides(r#"<g id="display" class="none">"#));
    }
}
