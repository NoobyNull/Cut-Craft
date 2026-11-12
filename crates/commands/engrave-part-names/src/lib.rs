//! Engrave Part Names: each part's number and name drawn as text in the middle of the part,
//! on a "Part Names" layer, so a Quick Engrave or V-Carve toolpath from that layer marks
//! every part with what its label says. The numbers are the cut list's and the labels'.
//! Selected parts get names, or every part on the sheet when nothing is selected; running it
//! again replaces the names it drew before. Text too big for a part is made smaller to fit,
//! and each text stays editable with Draw Text.

use cut_craft_core::document::{Document, EntityId, LayerId};
use cut_craft_core::font::{self, Align};
use cut_craft_core::geometry::{Path, Point, Rect};
use cut_craft_core::{Change, Command, CommandError, Meta, ensure_positive};
use fmt_cut_list::Part;
use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};
use serde::{Deserialize, Serialize};

pub struct EngravePartNames;

/// The layer the names go on.
pub const LAYER: &str = "Part Names";
/// The share of a part's width and height its text may take.
const ROOM: f64 = 0.8;
/// Text smaller than this cannot be engraved legibly, mm.
const SMALLEST: f64 = 2.0;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Words {
    /// "3 Side": the number matches the part's label
    #[default]
    #[schemars(title = "Number and name")]
    NumberAndName,
    /// Just the part's name, such as "Side"
    #[schemars(title = "Name only")]
    Name,
    /// Just the part's number, such as "3"
    #[schemars(title = "Number only")]
    Number,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    /// What each part is marked with
    #[schemars(title = "Engrave")]
    pub words: Words,
    /// Single-line fonts (Vectric font `.vfont` files in Cut Craft's fonts folder) engrave
    /// fastest; TrueType fonts work too
    #[schemars(schema_with = "font_choices")]
    pub font: String,
    /// The height of a capital letter; smaller parts get smaller text
    #[schemars(title = "Text height", extend("x-unit" = "mm"), range(min = 0))]
    pub height: f64,
}

fn font_choices(_: &mut SchemaGenerator) -> Schema {
    json_schema!({ "type": "string", "enum": font::installed(), "x-widget": "drop-down" })
}

impl Default for Params {
    fn default() -> Self {
        Self {
            words: Words::NumberAndName,
            font: font::installed().into_iter().next().unwrap_or_default(),
            // ponytail: 10 mm [inf], readable on a cabinet part from arm's length
            height: 10.0,
        }
    }
}

fn words(part: &Part, w: Words) -> String {
    let name = part.name.clone().unwrap_or_default();
    match w {
        Words::Number => part.number.to_string(),
        Words::Name if !name.is_empty() => name,
        _ if name.is_empty() => part.number.to_string(),
        _ => format!("{} {name}", part.number),
    }
}

/// The parts on the active sheet: the selected ones, or all when nothing is selected.
fn chosen(doc: &Document) -> Vec<Part> {
    fmt_cut_list::parts(doc)
        .into_iter()
        .filter(|p| p.sheet_index == doc.active_sheet || doc.sheets.len() <= 1)
        .filter(|p| p.vectors.iter().all(|v| doc.vectors.contains_key(v)))
        .filter(|p| doc.selection.is_empty() || p.vectors.iter().any(|v| doc.selection.contains(v)))
        .collect()
}

impl Command for EngravePartNames {
    const ID: &'static str = "engrave_part_names";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Engrave Part Names",
            group: "Offset and Layout",
            icon: "drawing/weld_and_offset/engrave_part_names.png",
            shortcut: "",
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        ensure_positive("text height", p.height)?;
        let parts = chosen(doc);
        if parts.is_empty() {
            return Err(CommandError::Failed(
                "There are no parts on this sheet to name. Draw or import closed shapes first."
                    .into(),
            ));
        }
        let texts: Vec<String> = parts.iter().map(|part| words(part, p.words)).collect();
        let font =
            font::load(&p.font, false, false, &texts.concat()).map_err(CommandError::Failed)?;
        let mut c = Change::builder(doc);
        let layer: LayerId = doc
            .layers
            .iter()
            .find(|l| l.name == LAYER)
            .map_or_else(|| c.add_layer(LAYER), |l| l.id);
        // the names drawn before inside these parts go
        let old: Vec<EntityId> = parts
            .iter()
            .flat_map(|part| part.vectors.iter().copied())
            .filter(|&v| doc.layer_of(v) == layer)
            .collect();
        for &v in &old {
            if let Some(&g) = doc.vector_groups.get(&v) {
                c.set_text(g, None);
            }
            c.remove(v);
        }
        let mut group = doc.next_group();
        let (mut made, mut small) = (Vec::new(), Vec::new());
        for (part, text) in parts.iter().zip(&texts) {
            let Some((height, anchor, paths)) = fitted(&font, text, p.height, part.bounds) else {
                small.push(part.number.to_string());
                continue;
            };
            for path in paths {
                let id = c.add(path);
                c.set_group(id, Some(group));
                c.move_to_layer(id, layer);
                made.push(id);
            }
            let spec = serde_json::json!({
                "mode": "text", "text": text, "font": p.font, "bold": false, "italic": false,
                "align": "center", "height": height, "anchor": {"x": anchor.x, "y": anchor.y},
            });
            c.set_text(group, Some(spec));
            group += 1;
        }
        if made.is_empty() {
            return Err(CommandError::Failed(
                "The parts are too small for text. Set a smaller text height.".into(),
            ));
        }
        c.note(format!(
            "The names are on the {LAYER} layer. To cut them, select that layer's vectors and \
             make a Quick Engrave or V-Carve toolpath."
        ));
        if !small.is_empty() {
            c.note(format!(
                "Too small for text, so not named: part {}.",
                small.join(", ")
            ));
        }
        c.select(made);
        Ok(c.finish())
    }
}

/// `text` centred in `room`: as tall as asked, or smaller to fit; `None` when it would be
/// too small to read. Gives the height, the anchor (the baseline's centre) and the paths.
fn fitted(
    font: &font::Font,
    text: &str,
    height: f64,
    room: Rect,
) -> Option<(f64, Point, Vec<Path>)> {
    let lay = |h: f64| {
        let paths = font.layout(text, h, Align::Center, Point::ZERO);
        let b = paths.iter().map(Path::bounds).reduce(Rect::union)?;
        Some((paths, b))
    };
    let (_, b) = lay(height)?;
    let size = room.size();
    let scale = (size.x * ROOM / b.size().x)
        .min(size.y * ROOM / b.size().y)
        .min(1.0);
    let h = height * scale;
    if h < SMALLEST {
        return None;
    }
    let (paths, b) = lay(h)?;
    // move the text's own box to the middle of the part
    let shift = room.center() - b.center();
    let paths = paths
        .iter()
        .map(|path| path.transformed(&cut_craft_core::glam::DAffine2::from_translation(shift)))
        .collect();
    Some((h, shift, paths))
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::testing::{document_with, run_on, square};

    fn boxes() -> Params {
        Params {
            font: font::installed()[0].clone(),
            ..Params::default()
        }
    }

    #[test]
    fn every_part_gets_its_number_and_name_in_its_middle_on_a_layer() {
        let mut doc = document_with(vec![square(0.0, 0.0, 200.0), square(300.0, 0.0, 200.0)]);
        doc.selection.clear();
        run_on::<EngravePartNames>(&mut doc, &boxes());
        let layer = doc.layers.iter().find(|l| l.name == LAYER).unwrap().id;
        let on_layer: Vec<_> = doc
            .vectors
            .keys()
            .filter(|&&v| doc.layer_of(v) == layer)
            .collect();
        assert_ne!(on_layer.len(), 0);
        assert_eq!(doc.texts.len(), 2, "each text editable with Draw Text");
        // still two parts: the names belong to the parts they are in
        let parts = fmt_cut_list::parts(&doc);
        assert_eq!(parts.len(), 2);
        for part in &parts {
            let text: Vec<_> = part
                .vectors
                .iter()
                .filter(|v| doc.layer_of(**v) == layer)
                .collect();
            assert!(!text.is_empty(), "part {} has its name", part.number);
            let b = text
                .iter()
                .map(|v| doc.vectors[v].bounds())
                .reduce(Rect::union)
                .unwrap();
            assert!(
                (b.center() - part.bounds.center()).length() < 1e-6,
                "centred"
            );
        }
    }

    #[test]
    fn running_again_replaces_the_names_and_small_parts_get_small_text() {
        let mut doc = document_with(vec![square(0.0, 0.0, 12.0)]);
        run_on::<EngravePartNames>(&mut doc, &boxes());
        let first = doc.vectors.len();
        run_on::<EngravePartNames>(&mut doc, &boxes());
        assert_eq!(doc.vectors.len(), first, "replaced, not doubled");
        let text = doc.texts.values().next().unwrap();
        assert!(
            text["height"].as_f64().unwrap() < 10.0,
            "shrunk to fit 12 mm"
        );
    }

    #[test]
    fn parts_too_small_for_text_say_so() {
        let doc = document_with(vec![square(0.0, 0.0, 1.0)]);
        let e = EngravePartNames::run(&doc, &boxes())
            .unwrap_err()
            .to_string();
        assert!(e.contains("too small"), "{e}");
        let e = EngravePartNames::run(&Document::default(), &boxes())
            .unwrap_err()
            .to_string();
        assert!(e.contains("no parts"), "{e}");
    }

    #[test]
    fn the_words_follow_the_choice() {
        let mut part = fmt_cut_list::parts(&document_with(vec![square(0.0, 0.0, 9.0)]))[0].clone();
        assert_eq!(words(&part, Words::NumberAndName), "1");
        part.name = Some("Side".into());
        assert_eq!(words(&part, Words::NumberAndName), "1 Side");
        assert_eq!(words(&part, Words::Name), "Side");
        assert_eq!(words(&part, Words::Number), "1");
    }

    cut_craft_core::contract_tests!(EngravePartNames);
}
