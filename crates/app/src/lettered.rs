//! Pictures whose letters stay at text size: a drawing (`assets/icons/diagrams`) marks each of
//! its callout letters as a `<path class="label" data-text="D" data-align="left center">`, so
//! a browser or a help page still shows it as drawn, while PowerWhittle takes the letters out
//! and writes them over the picture in the form's own text, the same size however small or
//! large the picture is drawn (Options ▸ Appearance ▸ Text size moves them with the rest).
//!
//! [`show`] is the one way PowerWhittle draws such a picture: forms, choice pictures and tips
//! all go through it. A picture without labels is drawn as it is.

use eframe::egui;
use resvg::usvg;
use std::sync::Arc;

/// A picture's callout letter.
#[derive(Clone, Debug, PartialEq)]
pub struct Label {
    pub text: String,
    /// The point of the letter that stays put, in the picture's own units: its left edge for
    /// a letter right of a dimension line, the middle of its foot for one above it, ….
    pub at: egui::Pos2,
    /// Which point of the letter `at` is.
    pub align: egui::Align2,
    pub colour: egui::Color32,
}

/// A picture read for its letters (see [`read`]).
#[derive(Clone, Debug)]
pub struct Lettering {
    /// The picture's `viewBox`.
    pub view_box: egui::Rect,
    /// What is drawn, letters and card left out: where a small picture is cut to.
    pub drawing: egui::Rect,
    pub labels: Vec<Label>,
    /// The light card the picture is drawn on, filled and edged, if it has one.
    pub card: Option<(egui::Color32, egui::Color32)>,
    /// The thinnest line drawn, in the picture's units.
    pub thinnest: f32,
    /// The picture as SVG with its letters and card taken out.
    pub bare: String,
}

/// The picture `svg` read for its letters; `None` when it has none (or is not SVG), so it is
/// drawn as it is.
pub fn read(svg: &str) -> Option<Lettering> {
    let doc = roxmltree::Document::parse(svg).ok()?;
    let root = doc.root_element();
    let view_box = rect(root.attribute("viewBox")?)?;
    let (w, h) = (view_box.width(), view_box.height());
    let mut labels = vec![];
    let mut cut = vec![];
    let mut card = None;
    for node in root.descendants().filter(roxmltree::Node::is_element) {
        let name = node.tag_name().name();
        if name == "path" && node.attribute("class") == Some("label") {
            let (Some(text), Some(outline)) = (node.attribute("data-text"), node.attribute("d"))
            else {
                continue;
            };
            let bounds = path_bounds(outline)?;
            let align = align(node.attribute("data-align").unwrap_or_default());
            labels.push(Label {
                text: text.to_owned(),
                at: align.pos_in_rect(&bounds),
                align,
                colour: colour(node.attribute("fill")).unwrap_or(egui::Color32::BLACK),
            });
            cut.push(node.range());
        } else if name == "rect" && card.is_none() {
            let size = |a| node.attribute(a).and_then(|v: &str| v.parse::<f32>().ok());
            if let (Some(rw), Some(rh)) = (size("width"), size("height"))
                && rw >= 0.95 * w
                && rh >= 0.95 * h
            {
                card = Some((
                    colour(node.attribute("fill")).unwrap_or(egui::Color32::WHITE),
                    colour(node.attribute("stroke")).unwrap_or(egui::Color32::GRAY),
                ));
                cut.push(node.range());
            }
        }
    }
    if labels.is_empty() {
        return None;
    }
    cut.sort_by_key(|r| std::cmp::Reverse(r.start));
    let mut bare = svg.to_owned();
    for range in cut {
        bare.replace_range(range, "");
    }
    let drawing = drawn_bounds(&bare).unwrap_or(view_box);
    let thinnest = doc
        .descendants()
        .filter_map(|n| n.attribute("stroke-width"))
        .filter_map(|v| v.parse::<f32>().ok())
        .filter(|w| *w > 0.0)
        .fold(f32::INFINITY, f32::min);
    Some(Lettering {
        view_box,
        drawing,
        labels,
        card,
        thinnest: if thinnest.is_finite() { thinnest } else { 1.0 },
        bare,
    })
}

impl Lettering {
    /// The bare picture (see [`Lettering::bare`]) seen through `window` (its `viewBox`), its
    /// lines and arrowheads `thicker` times as thick, so they stay a pixel wide drawn small.
    pub fn svg(&self, window: egui::Rect, thicker: f32) -> String {
        let mut svg = self.bare.clone();
        if (thicker - 1.0).abs() > f32::EPSILON {
            for name in [
                "stroke-width",
                "markerWidth",
                "markerHeight",
                "stroke-dasharray",
            ] {
                svg = scale_attribute(&svg, name, thicker);
            }
        }
        let start = svg.find("<svg").unwrap_or(0);
        let end = svg[start..].find('>').map_or(svg.len(), |e| start + e);
        let tag = &svg[start..end];
        let mut tag = strip_attribute(tag, "width");
        tag = strip_attribute(&tag, "height");
        tag = strip_attribute(&tag, "viewBox");
        let (x, y, w, h) = (window.min.x, window.min.y, window.width(), window.height());
        let tag = format!("{tag} viewBox=\"{x} {y} {w} {h}\" width=\"{w}\" height=\"{h}\"");
        format!("{}{tag}{}", &svg[..start], &svg[end..])
    }
}

/// Where the picture and its letters go in `space`: the picture's `window` (its units) as
/// big as fits with every letter (`sizes`, in [`Lettering::labels`]' order) inside `space` too.
pub fn place(
    lettering: &Lettering,
    window: egui::Rect,
    space: egui::Rect,
    sizes: &[egui::Vec2],
) -> (egui::Rect, Vec<egui::Rect>) {
    let mut room = space;
    let mut placed = (space, vec![]);
    // the letters' overhang depends on the scale and the scale on the overhang: a few rounds
    for _ in 0..6 {
        let scale = (room.width() / window.width())
            .min(room.height() / window.height())
            .max(0.0);
        let picture = egui::Rect::from_center_size(room.center(), window.size() * scale);
        let at = |p: egui::Pos2| picture.min + (p - window.min) * scale;
        let letters: Vec<egui::Rect> = lettering
            .labels
            .iter()
            .zip(sizes)
            .map(|(l, s)| l.align.anchor_size(at(l.at), *s))
            .collect();
        let all = letters.iter().fold(picture, |a, r| a.union(*r));
        placed = (picture, letters);
        if space.expand(0.5).contains_rect(all) {
            break;
        }
        // take what sticks out off that side of the room
        room = egui::Rect::from_min_max(
            room.min + (space.min - all.min).max(egui::Vec2::ZERO),
            room.max - (all.max - space.max).max(egui::Vec2::ZERO),
        );
    }
    placed
}

/// How a picture is fitted: as drawn, or cut to its drawing (a small picture beside fields).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fit {
    Whole,
    Cut,
}

/// Draws `image` in a rect of `size`: an SVG with labels (see [`read`]) without its own
/// letters, with them written over it in the theme's body text, on its card; anything else
/// as it is, fitted to `size`. The picture's width over height, as it will be drawn, is
/// [`aspect`].
pub fn show(
    ui: &mut egui::Ui,
    image: &egui::Image<'static>,
    size: egui::Vec2,
    fit: Fit,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::hover());
    paint(ui, image, rect, fit);
    response
}

/// [`show`] in `rect`, without taking room for it: for a picture beside what takes the room.
pub fn paint(ui: &egui::Ui, image: &egui::Image<'static>, rect: egui::Rect, fit: Fit) {
    if !ui.is_rect_visible(rect) {
        return;
    }
    let Some(lettering) = lettering(ui.ctx(), image) else {
        image.paint_at(ui, rect);
        return;
    };
    let font = egui::TextStyle::Body.resolve(ui.style());
    let painter = ui.painter_at(rect.expand(1.0));
    let galleys: Vec<_> = lettering
        .labels
        .iter()
        .map(|l| painter.layout_no_wrap(l.text.clone(), font.clone(), l.colour))
        .collect();
    let window = match fit {
        Fit::Whole => lettering.view_box,
        Fit::Cut => lettering.drawing,
    };
    let pad = font.size * 0.4;
    let space = rect.shrink(pad);
    let sizes: Vec<egui::Vec2> = galleys.iter().map(|g| g.size()).collect();
    let (picture, letters) = place(&lettering, window, space, &sizes);
    if let Some((fill, edge)) = lettering.card {
        let round = ui.visuals().widgets.noninteractive.corner_radius;
        painter.rect(
            rect,
            round,
            fill,
            egui::Stroke::new(1.0, edge),
            egui::StrokeKind::Inside,
        );
    }
    let scale = picture.width() / window.width();
    let thicker = thicker(lettering.thinnest * scale * ui.ctx().pixels_per_point());
    let uri = bare_uri(ui.ctx(), image, &lettering, window, fit, thicker);
    egui::Image::new(uri).paint_at(ui, picture);
    for (galley, at) in galleys.into_iter().zip(letters) {
        painter.galley(at.min, galley, egui::Color32::PLACEHOLDER);
    }
}

/// The width over the height of `image` drawn with `fit`; `None` for a picture without labels.
pub fn aspect(ctx: &egui::Context, image: &egui::Image<'static>, fit: Fit) -> Option<f32> {
    let lettering = lettering(ctx, image)?;
    let window = match fit {
        Fit::Whole => lettering.view_box,
        Fit::Cut => lettering.drawing,
    };
    Some(window.width() / window.height())
}

/// How many times thicker to draw lines that come out `thinnest` pixels wide, so the
/// thinnest is a pixel: in quarters, so a few sizes share one drawing.
fn thicker(thinnest: f32) -> f32 {
    if thinnest >= 1.0 || thinnest <= 0.0 {
        return 1.0;
    }
    ((1.0 / thinnest) * 4.0).ceil().min(16.0) / 4.0
}

/// `image`'s letters, read once and kept with egui's memory; `None` until its bytes are there,
/// and for pictures without letters.
fn lettering(ctx: &egui::Context, image: &egui::Image<'static>) -> Option<Arc<Lettering>> {
    let egui::ImageSource::Uri(uri) = image.source(ctx) else {
        return None;
    };
    if !uri.ends_with(".svg") {
        return None;
    }
    let id = egui::Id::unique(("lettering", uri.as_ref()));
    if let Some(known) = ctx.data(|d| d.get_temp::<Option<Arc<Lettering>>>(id)) {
        return known;
    }
    let Ok(egui::load::BytesPoll::Ready { bytes, .. }) = ctx.try_load_bytes(&uri) else {
        return None;
    };
    let read = std::str::from_utf8(&bytes)
        .ok()
        .and_then(read)
        .map(Arc::new);
    ctx.data_mut(|d| d.insert_temp(id, read.clone()));
    read
}

/// Where the bare picture for `image` seen through `window` is kept in egui, handed over the
/// first time it is asked for.
fn bare_uri(
    ctx: &egui::Context,
    image: &egui::Image<'static>,
    lettering: &Lettering,
    window: egui::Rect,
    fit: Fit,
    thicker: f32,
) -> String {
    let original = match image.source(ctx) {
        egui::ImageSource::Uri(uri) => uri.replace("://", "/"),
        _ => String::new(),
    };
    let uri = format!("bytes://lettered/{fit:?}/{thicker}/{original}");
    let id = egui::Id::unique(("lettered", &uri));
    if !ctx.data(|d| d.get_temp::<bool>(id).unwrap_or(false)) {
        ctx.include_bytes(uri.clone(), lettering.svg(window, thicker).into_bytes());
        ctx.data_mut(|d| d.insert_temp(id, true));
    }
    uri
}

/// `data-align`'s words (`left center`) as egui's: across, then up and down.
fn align(words: &str) -> egui::Align2 {
    let mut words = words.split_whitespace();
    let across = match words.next() {
        Some("left") => egui::Align::Min,
        Some("right") => egui::Align::Max,
        _ => egui::Align::Center,
    };
    let down = match words.next() {
        Some("top") => egui::Align::Min,
        Some("bottom") => egui::Align::Max,
        _ => egui::Align::Center,
    };
    egui::Align2([across, down])
}

fn colour(hex: Option<&str>) -> Option<egui::Color32> {
    egui::Color32::from_hex(hex?).ok()
}

/// A `viewBox`'s four numbers as a rect.
fn rect(text: &str) -> Option<egui::Rect> {
    match numbers(text)[..] {
        [left, top, width, height] => Some(egui::Rect::from_min_size(
            egui::pos2(left, top),
            egui::vec2(width, height),
        )),
        _ => None,
    }
}

fn numbers(text: &str) -> Vec<f32> {
    text.split(|c: char| c.is_whitespace() || c == ',')
        .filter_map(|n| n.parse().ok())
        .collect()
}

/// Where a path's outline reaches, in the picture's units.
fn path_bounds(d: &str) -> Option<egui::Rect> {
    let svg = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1 1"><path d="{d}"/></svg>"#
    );
    let tree = usvg::Tree::from_str(&svg, &usvg::Options::default()).ok()?;
    let b = tree.root().abs_bounding_box();
    Some(egui::Rect::from_min_size(
        egui::pos2(b.x(), b.y()),
        egui::vec2(b.width(), b.height()),
    ))
}

/// Where everything drawn in `svg` reaches, lines' widths and all, in its units.
fn drawn_bounds(svg: &str) -> Option<egui::Rect> {
    let tree = usvg::Tree::from_str(svg, &usvg::Options::default()).ok()?;
    let drawn = tree.root().abs_stroke_bounding_box();
    // the tree is laid out in the picture's width and height; back to its viewBox's units
    let doc = roxmltree::Document::parse(svg).ok()?;
    let view_box = rect(doc.root_element().attribute("viewBox")?)?;
    let size = tree.size();
    let scale = egui::vec2(
        view_box.width() / size.width(),
        view_box.height() / size.height(),
    );
    Some(egui::Rect::from_min_size(
        view_box.min + egui::vec2(drawn.x(), drawn.y()) * scale,
        egui::vec2(drawn.width(), drawn.height()) * scale,
    ))
}

/// `svg` with every `name="…"`'s numbers `times` as big.
fn scale_attribute(svg: &str, name: &str, times: f32) -> String {
    let key = format!(" {name}=\"");
    let mut out = String::with_capacity(svg.len());
    let mut rest = svg;
    while let Some(i) = rest.find(&key) {
        let start = i + key.len();
        let Some(len) = rest[start..].find('"') else {
            break;
        };
        out.push_str(&rest[..start]);
        let scaled: Vec<String> = numbers(&rest[start..start + len])
            .iter()
            .map(|n| format!("{}", n * times))
            .collect();
        out.push_str(&scaled.join(" "));
        rest = &rest[start + len..];
    }
    out.push_str(rest);
    out
}

/// An element's start `tag` without its `name` attribute.
fn strip_attribute(tag: &str, name: &str) -> String {
    let key = format!(" {name}=\"");
    let Some(i) = tag.find(&key) else {
        return tag.to_owned();
    };
    let start = i + key.len();
    let end = tag[start..].find('"').map_or(tag.len(), |e| start + e + 1);
    format!("{}{}", &tag[..i], &tag[end..])
}

#[cfg(test)]
mod tests;
