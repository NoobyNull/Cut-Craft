//! Plate Production: badges or nameplates from a template and a data file. The template is the
//! selected vectors; text in it written with Draw
//! Text may hold variables, `!!name!!`, each filled from a column of the data file or from a
//! counter. Plates fill the material inside the margins, one per row of data; when they need
//! more than one sheet of material each sheet's plates go on a layer of their own. The
//! template's toolpaths are made again for each sheet, shifted onto its plates.

use cut_craft_core::document::{Document, EntityId};
use cut_craft_core::geometry::{Point, Rect};
use cut_craft_core::glam::{DAffine2, DVec3};
use cut_craft_core::toolpath::{Move, Toolpath};
use cut_craft_core::{Change, Command, CommandError, Edit, Meta};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct PlateProduction;

/// One step of laying out the plates, run on the document as the steps before left it:
/// which plate it lays, and the template vectors its copies stand for: one for one, in
/// order, or (`text`) a whole template text made again with the plate's values.
struct PlateStep {
    plate: usize,
    template: Vec<EntityId>,
    text: bool,
    run: StepFn,
}

/// What a step made: its plate, the template vectors it stands for, whether it is a text
/// made again, and the vectors it added.
struct Made {
    plate: usize,
    template: Vec<EntityId>,
    text: bool,
    vectors: Vec<EntityId>,
}

/// A row of the data file: its row number in the spreadsheet and its fields.
#[derive(Clone, Debug, Default)]
struct Row {
    number: usize,
    fields: Vec<String>,
}

type StepFn = Box<dyn Fn(&Document) -> Result<Change, CommandError>>;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Separator {
    #[default]
    Comma,
    Tab,
    Semicolon,
    Space,
}

impl Separator {
    /// The file's records, quoted fields keeping their separators (spaces: split at runs of
    /// them, a line a record), each field trimmed.
    fn records(self, text: &str) -> Vec<Vec<String>> {
        let records = match self {
            Self::Comma => cut_craft_core::csv::records(text, ','),
            Self::Tab => cut_craft_core::csv::records(text, '\t'),
            Self::Semicolon => cut_craft_core::csv::records(text, ';'),
            Self::Space => text
                .lines()
                .map(|l| l.split_whitespace().map(str::to_owned).collect())
                .collect(),
        };
        records
            .into_iter()
            .map(|r| r.into_iter().map(|f| f.trim().to_owned()).collect())
            .collect()
    }
}

/// How the plates are laid out on the material.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Layout {
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub top_margin: f64,
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub bottom_margin: f64,
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub left_margin: f64,
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub right_margin: f64,
    /// Between plates side by side
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub spacing_x: f64,
    /// Between plates one above another
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub spacing_y: f64,
}

/// Where a variable's text comes from.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Assignment {
    pub variable: String,
    /// The data file's column, from 1; 0 for the counter
    pub column: usize,
}

/// The counter variables set to column 0 take.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Counter {
    pub start: i64,
    pub step: i64,
    /// Padded with zeros to at least this many digits
    #[schemars(range(min = 0, max = 12))]
    pub digits: usize,
}

/// The data merged into the template.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Data {
    /// A text file of rows; empty to number plates with the counter alone
    pub file: String,
    #[schemars(extend("x-widget" = "row"))]
    pub separator: Separator,
    #[schemars(title = "First row is column names")]
    pub first_row_names: bool,
    /// How many plates when there is no data file
    #[schemars(range(min = 1))]
    pub plates: usize,
    pub variables: Vec<Assignment>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    #[schemars(title = "Layout", extend("x-section" = true))]
    pub layout: Layout,
    #[schemars(title = "Data", extend("x-section" = true))]
    pub data: Data,
    #[schemars(title = "Number Format", extend("x-section" = true))]
    pub counter: Counter,
    /// The template's toolpaths made again for each sheet of plates, each named "S1 - …"
    /// after its sheet and the template's toolpath
    #[serde(default = "yes")]
    #[schemars(title = "Create Toolpaths")]
    pub create_toolpaths: bool,
}

fn yes() -> bool {
    true
}

impl Default for Params {
    fn default() -> Self {
        Self {
            create_toolpaths: true,
            layout: Layout {
                top_margin: 5.0,
                bottom_margin: 5.0,
                left_margin: 5.0,
                right_margin: 5.0,
                spacing_x: 5.0,
                spacing_y: 5.0,
            },
            data: Data {
                file: String::new(),
                separator: Separator::Comma,
                first_row_names: true,
                plates: 1,
                variables: vec![],
            },
            counter: Counter {
                start: 1,
                step: 1,
                digits: 0,
            },
        }
    }
}

/// The `!!name!!` variables in `text`, in order, once each.
fn variables_in(text: &str) -> Vec<String> {
    let mut found = vec![];
    for (i, part) in text.split("!!").enumerate() {
        let name = part.trim();
        let named = !name.is_empty() && name.chars().all(|c| c.is_alphanumeric() || c == '_');
        if i % 2 == 1 && named && !found.iter().any(|f: &String| f == name) {
            found.push(name.to_owned());
        }
    }
    found
}

/// The selected text groups' settings, by group.
fn template_texts(doc: &Document) -> Vec<(u64, &serde_json::Value)> {
    let mut groups: Vec<u64> = doc
        .selection
        .iter()
        .filter_map(|id| doc.vector_groups.get(id).copied())
        .filter(|g| doc.texts.contains_key(g))
        .collect();
    groups.sort_unstable();
    groups.dedup();
    groups.into_iter().map(|g| (g, &doc.texts[&g])).collect()
}

impl Command for PlateProduction {
    const ID: &'static str = "plate_production";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Plate Production",
            group: "Offset and Layout",
            icon: "",
            shortcut: "",
        }
    }

    /// The template's variables, each given the next column.
    fn initial_params(doc: &Document) -> Params {
        let mut names = vec![];
        for (_, spec) in template_texts(doc) {
            for v in variables_in(spec["text"].as_str().unwrap_or_default()) {
                if !names.contains(&v) {
                    names.push(v);
                }
            }
        }
        let mut p = Params::default();
        p.data.variables = names
            .into_iter()
            .enumerate()
            .map(|(i, variable)| Assignment {
                variable,
                column: i + 1,
            })
            .collect();
        p
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        let template: Vec<EntityId> = doc.selection.iter().copied().collect();
        let Some(bounds) = template
            .iter()
            .map(|id| doc.vectors[id].bounds())
            .reduce(Rect::union)
        else {
            return Err(CommandError::Failed(
                "select the vectors of the master template".into(),
            ));
        };
        let rows = rows(&p.data)?;
        let spots = layout(doc.job.area(), bounds, &p.layout, rows.len())?;
        let texts = template_texts(doc);
        let in_text = |id: &EntityId| {
            doc.vector_groups
                .get(id)
                .is_some_and(|g| texts.iter().any(|(t, _)| t == g))
        };
        let plain: Vec<EntityId> = template.iter().copied().filter(|id| !in_text(id)).collect();
        let sheets = spots.iter().map(|s| s.0).max().map_or(1, |s| s + 1);
        // one step lays out a plate's plain vectors (and makes its sheet's layer), then one
        // step per text re-makes that text with the plate's values
        let mut steps: Vec<PlateStep> = vec![];
        for (k, &(sheet, at)) in spots.iter().enumerate() {
            let shift = DAffine2::from_translation(at - bounds.min);
            let copied = plain.clone();
            let new_layer = sheets > 1 && (k == 0 || spots[k - 1].0 != sheet);
            steps.push(PlateStep {
                plate: k,
                template: plain.clone(),
                text: false,
                run: Box::new(move |d: &Document| {
                    let mut c = Change::builder(d);
                    if new_layer {
                        let layer = c.add_layer(&format!("Sheet {}", sheet + 1));
                        c.set_current_layer(layer);
                    }
                    for id in &copied {
                        c.add(d.vectors[id].transformed(&shift));
                    }
                    Ok(c.finish())
                }),
            });
            for (group, spec) in &texts {
                let mut params: cmd_draw_text::Params = serde_json::from_value((*spec).clone())
                    .map_err(|e| CommandError::Failed(format!("a template text: {e}")))?;
                params.text = fill(
                    &params.text,
                    &p.data.variables,
                    &rows[k].fields,
                    &p.counter,
                    k,
                );
                params.anchor.x += at.x - bounds.min.x;
                params.anchor.y += at.y - bounds.min.y;
                params.replaces = None;
                steps.push(PlateStep {
                    plate: k,
                    template: doc.group_members_of(*group),
                    text: true,
                    run: Box::new(move |d: &Document| {
                        <cmd_draw_text::DrawText as Command>::run(d, &params)
                    }),
                });
            }
        }
        // what each step made, as the steps make them
        let made: std::cell::RefCell<Vec<Made>> = std::cell::RefCell::new(vec![]);
        let mut laid = Change::chain_over(doc, &steps, |d, step| {
            let change = (step.run)(d)?;
            made.borrow_mut().push(Made {
                plate: step.plate,
                template: step.template.clone(),
                text: step.text,
                vectors: change
                    .edits
                    .iter()
                    .filter_map(|e| match e {
                        Edit::Add { id, .. } => Some(*id),
                        _ => None,
                    })
                    .collect(),
            });
            Ok(change)
        })?;
        let made = made.into_inner();
        let mut after = doc.clone();
        after
            .apply(&laid)
            .map_err(|e| CommandError::Failed(e.to_string()))?;
        if let Some(note) = overflowing(&after, &made, &spots, bounds, &rows, &p.data) {
            laid.notes.push(note);
        }
        if !p.create_toolpaths {
            return Ok(laid);
        }
        let toolpaths = sheet_toolpaths(doc, &after, &template, &spots, bounds, &made);
        Ok(laid.then(toolpaths))
    }
}

/// The template's toolpaths once per sheet of plates: each toolpath whose vectors are in
/// `template`, its moves shifted onto every plate of the sheet (`spots`: each plate's sheet
/// and place, `bounds` the template's box), its vectors the plates' copies (`made`), named
/// after the sheet and the toolpath. One cutting a template text is left stale: each plate's
/// text differs from the template's, so the shifted moves would cut the placeholders; the app
/// recalculates it straight away.
fn sheet_toolpaths(
    doc: &Document,
    after: &Document,
    template: &[EntityId],
    spots: &[(usize, Point)],
    bounds: Rect,
    made: &[Made],
) -> Change {
    let sheets = spots.iter().map(|s| s.0).max().map_or(1, |s| s + 1);
    let mut c = Change::builder(after);
    let of_template = |t: &&Toolpath| t.vectors.iter().any(|v| template.contains(v));
    for t in doc.toolpaths.values().filter(of_template) {
        let cuts_text = made
            .iter()
            .any(|m| m.text && m.template.iter().any(|v| t.vectors.contains(v)));
        for sheet in 0..sheets {
            let (mut moves, mut power, mut vectors) = (vec![], vec![], vec![]);
            // where each copy is, for a drill bank's plunges
            let mut placed = vec![];
            for (k, &(_, at)) in spots.iter().enumerate().filter(|(_, s)| s.0 == sheet) {
                let shift = at - bounds.min;
                placed.push((shift, moves.len()));
                moves.extend(t.moves.iter().map(|m| Move {
                    kind: m.kind,
                    to: m.to + DVec3::new(shift.x, shift.y, 0.0),
                }));
                power.extend(t.power.iter().copied());
                vectors.extend(plate_vectors(made, k, &t.vectors));
            }
            let name = format!("S{} - {}", sheet + 1, t.name);
            // its settings carry the name too, so a recalculation keeps it
            let mut params = t.params.clone();
            if params.get("name").is_some_and(serde_json::Value::is_string) {
                params["name"] = name.clone().into();
            }
            c.add_toolpath(Toolpath {
                name,
                params,
                vectors,
                moves,
                power,
                stale: cuts_text,
                drill_bank: t.drill_bank.as_ref().map(|b| b.copied(&placed)),
                ..t.clone()
            });
        }
    }
    c.finish()
}

/// Plate `plate`'s copies of the template vectors `of`: a plain vector's own copy, and the
/// whole of each text made again from a template text holding one of them.
fn plate_vectors(made: &[Made], plate: usize, of: &[EntityId]) -> Vec<EntityId> {
    let mut out = vec![];
    for m in made.iter().filter(|m| m.plate == plate) {
        if m.text {
            if m.template.iter().any(|v| of.contains(v)) {
                out.extend(m.vectors.iter().copied());
            }
        } else {
            out.extend(
                m.template
                    .iter()
                    .zip(&m.vectors)
                    .filter(|(t, _)| of.contains(t))
                    .map(|(_, v)| *v),
            );
        }
    }
    out
}

/// A note naming the rows whose text runs past their plate (the template's box, `bounds`,
/// where `spots` put it), with how to fix it; `None` when every plate's text fits.
fn overflowing(
    after: &Document,
    made: &[Made],
    spots: &[(usize, Point)],
    bounds: Rect,
    rows: &[Row],
    data: &Data,
) -> Option<String> {
    // a little give for curve flattening
    const GIVE: f64 = 0.01;
    let mut over = vec![];
    for (k, &(_, at)) in spots.iter().enumerate() {
        let plate = Rect::new(at, at + bounds.size());
        let text = made
            .iter()
            .filter(|m| m.plate == k && m.text)
            .flat_map(|m| &m.vectors)
            .filter_map(|v| after.vectors.get(v))
            .map(cut_craft_core::geometry::Path::bounds)
            .reduce(Rect::union);
        let Some(text) = text else { continue };
        let outside = text.min.x < plate.min.x - GIVE
            || text.min.y < plate.min.y - GIVE
            || text.max.x > plate.max.x + GIVE
            || text.max.y > plate.max.y + GIVE;
        if outside {
            let row = &rows[k];
            // the first column a variable takes names the row
            let name = data
                .variables
                .iter()
                .filter(|a| a.column > 0)
                .find_map(|a| row.fields.get(a.column - 1))
                .filter(|f| !f.is_empty());
            over.push(match name {
                Some(name) => format!("row {} ({name})", row.number),
                None => format!("plate {}", k + 1),
            });
        }
    }
    if over.is_empty() {
        return None;
    }
    let which = over.join(", ");
    Some(format!(
        "The text runs past the plate's edge on {which}. Make the template's text a Text in a \
         Box (Draw Text) so long names shrink to fit the box, or make the text smaller, then \
         undo and run Plate Production again."
    ))
}

/// The data's rows, numbered as the spreadsheet numbers them; with no file, one empty row
/// per plate asked for.
fn rows(data: &Data) -> Result<Vec<Row>, CommandError> {
    if data.file.trim().is_empty() {
        if data.plates == 0 {
            return Err(CommandError::InvalidParams(
                "set plates to 1 or more".into(),
            ));
        }
        return Ok((1..=data.plates)
            .map(|number| Row {
                number,
                fields: vec![],
            })
            .collect());
    }
    let bytes = std::fs::read(&data.file)
        .map_err(|e| CommandError::Failed(format!("{}: {e}", data.file)))?;
    let text = cut_craft_core::csv::decode(&bytes);
    let skip = usize::from(data.first_row_names);
    let rows: Vec<Row> = data
        .separator
        .records(&text)
        .into_iter()
        .enumerate()
        .skip(skip)
        .filter(|(_, fields)| fields.iter().any(|f| !f.is_empty()))
        .map(|(i, fields)| Row {
            number: i + 1,
            fields,
        })
        .collect();
    if rows.is_empty() {
        return Err(CommandError::Failed(format!(
            "{} has no rows of data. Pick a file with a row for each plate, or fewer rows to skip",
            data.file
        )));
    }
    Ok(rows)
}

/// `text` with each assigned `!!variable!!` replaced by its column of `row`, or by plate
/// `index`'s count.
fn fill(
    text: &str,
    assigned: &[Assignment],
    row: &[String],
    counter: &Counter,
    index: usize,
) -> String {
    let count = counter.start + counter.step * i64::try_from(index).unwrap_or(i64::MAX);
    let mut out = text.to_owned();
    for a in assigned {
        let value = if a.column == 0 {
            let digits = counter.digits;
            format!("{count:0digits$}")
        } else {
            row.get(a.column - 1).cloned().unwrap_or_default()
        };
        out = out.replace(&format!("!!{}!!", a.variable), &value);
    }
    out
}

/// Where each of `count` plates goes: the sheet of material it is on, and its lower-left
/// corner, filling rows from the top left inside the margins.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "plate counts are small"
)]
fn layout(
    material: Rect,
    plate: Rect,
    margins: &Layout,
    count: usize,
) -> Result<Vec<(usize, Point)>, CommandError> {
    let (width, height) = (plate.size().x, plate.size().y);
    let room =
        |size: f64, a: f64, b: f64, gap: f64, of: f64| ((size - a - b + gap) / (of + gap)).floor();
    let across = room(
        material.size().x,
        margins.left_margin,
        margins.right_margin,
        margins.spacing_x,
        width,
    );
    let down = room(
        material.size().y,
        margins.top_margin,
        margins.bottom_margin,
        margins.spacing_y,
        height,
    );
    if across < 1.0 || down < 1.0 {
        return Err(CommandError::Failed(
            "a plate does not fit on the material inside the margins. Make the plate or the margins smaller".into(),
        ));
    }
    let (across, down) = (across as usize, down as usize);
    let per_sheet = across * down;
    Ok((0..count)
        .map(|k| {
            let (sheet, i) = (k / per_sheet, k % per_sheet);
            let (column, row) = ((i % across) as f64, (i / across) as f64);
            let x = material.min.x + margins.left_margin + column * (width + margins.spacing_x);
            let y =
                material.max.y - margins.top_margin - height - row * (height + margins.spacing_y);
            (sheet, Point::new(x, y))
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::testing::{after, run_on};

    /// A job with a selected template: a 40 × 20 border and the text "!!name!!".
    fn template() -> Document {
        let mut doc = after::<cmd_draw_text::DrawText>(&cmd_draw_text::Params {
            text: "!!name!!".into(),
            font: "Test Boxes".into(),
            ..cmd_draw_text::Params::default()
        });
        let mut c = Change::builder(&doc);
        let border = c.add(cut_craft_core::geometry::Path::polyline(
            &[
                Point::new(5.0, 5.0),
                Point::new(45.0, 5.0),
                Point::new(45.0, 25.0),
                Point::new(5.0, 25.0),
            ],
            true,
        ));
        let mut ids: Vec<EntityId> = doc.selection.iter().copied().collect();
        ids.push(border);
        c.select(ids);
        doc.apply(&c.finish()).unwrap();
        doc
    }

    fn data_file(name: &str, text: &str) -> String {
        let path = std::env::temp_dir().join(format!(
            "cut-craft-plates-{}-{name}.csv",
            std::process::id()
        ));
        std::fs::write(&path, text).unwrap();
        path.display().to_string()
    }

    cut_craft_core::contract_tests!(PlateProduction, template);

    #[test]
    fn variables_are_found_in_the_template_text() {
        assert_eq!(
            variables_in("Hi !!first!! !!last!!, !!first!!"),
            ["first", "last"]
        );
        let p = PlateProduction::initial_params(&template());
        assert_eq!(p.data.variables[0].variable, "name");
        assert_eq!(p.data.variables[0].column, 1);
    }

    #[test]
    fn one_plate_per_row_with_its_text() {
        let mut doc = template();
        let before = doc.vectors.len();
        let mut p = PlateProduction::initial_params(&doc);
        p.data.file = data_file("rows", "name\nI\nIH\nH\n");
        run_on::<PlateProduction>(&mut doc, &p);
        let texts: Vec<&str> = doc
            .texts
            .values()
            .filter_map(|t| t["text"].as_str())
            .collect();
        for name in ["I", "IH", "H"] {
            assert!(texts.contains(&name), "{name} in {texts:?}");
        }
        // 3 borders + 1 + 2 + 1 letter boxes
        assert_eq!(doc.vectors.len(), before + 3 + 4);
    }

    #[test]
    fn quoted_fields_keep_their_commas() {
        let mut doc = template();
        let mut p = PlateProduction::initial_params(&doc);
        p.data.file = data_file("quoted", "name\n\"I, H\"\n");
        run_on::<PlateProduction>(&mut doc, &p);
        assert!(doc.texts.values().any(|t| t["text"] == "I, H"));
    }

    #[test]
    fn excel_windows_text_is_read() {
        let path = std::env::temp_dir().join(format!(
            "cut-craft-plates-{}-latin1.csv",
            std::process::id()
        ));
        std::fs::write(&path, b"name\nRen\xe9e\n").unwrap();
        let data = Data {
            file: path.display().to_string(),
            ..Params::default().data
        };
        assert_eq!(rows(&data).unwrap()[0].fields, ["Renée"]);
    }

    #[test]
    fn rows_keep_their_spreadsheet_row_numbers() {
        let data = Data {
            file: data_file("numbers", "name\nI\n\nH\n"),
            ..Params::default().data
        };
        let got: Vec<usize> = rows(&data).unwrap().iter().map(|r| r.number).collect();
        assert_eq!(got, [2, 4]);
    }

    #[test]
    fn text_running_past_the_plate_is_named_in_a_note() {
        let doc = template();
        let mut p = PlateProduction::initial_params(&doc);
        // the border is 40 wide; ten letter boxes are far wider
        p.data.file = data_file("long", "name\nI\nIHIHIHIHIHIHIHIHIHIH\n");
        let change = PlateProduction::run(&doc, &p).unwrap();
        let note = change.notes.join(" ");
        assert!(note.contains("row 3"), "{note}");
        assert!(note.contains("IHIHIHIHIHIHIHIHIHIH"), "{note}");
        assert!(!note.contains("row 2"), "{note}");
        assert!(note.contains("Text in a Box"), "{note}");
    }

    #[test]
    fn toolpaths_on_placeholder_text_are_left_to_recalculate_with_each_plates_text() {
        use cut_craft_core::toolpath::Tool;
        let mut doc = template();
        let border = *doc.selection.iter().last().unwrap();
        let letters: Vec<EntityId> = doc
            .selection
            .iter()
            .copied()
            .filter(|v| *v != border)
            .collect();
        let toolpath = |name: &str, vectors: Vec<EntityId>| Toolpath {
            name: name.into(),
            strategy: "vcarve_toolpath".into(),
            params: serde_json::json!({ "name": name }),
            tool: Tool::default(),
            vectors,
            moves: vec![],
            visible: true,
            power: vec![],
            stale: false,
            group: None,
            drill_bank: None,
        };
        let mut c = Change::builder(&doc);
        c.add_toolpath(toolpath("Carve name", letters));
        c.add_toolpath(toolpath("Cut out", vec![border]));
        doc.apply(&c.finish()).unwrap();
        let mut p = PlateProduction::initial_params(&doc);
        p.data.file = data_file("stale", "name\nI\nIH\n");
        run_on::<PlateProduction>(&mut doc, &p);
        let made = |name: &str| {
            doc.toolpaths
                .values()
                .find(|t| t.name == name)
                .unwrap()
                .clone()
        };
        let carve = made("S1 - Carve name");
        assert!(
            carve.stale,
            "the template's letters are not the plates' letters"
        );
        assert_eq!(
            carve.params["name"], "S1 - Carve name",
            "recalculating keeps the sheet's name"
        );
        // its vectors are every plate's own letters: I, then I and H
        assert_eq!(carve.vectors.len(), 3);
        let filled: Vec<EntityId> = doc
            .texts
            .iter()
            .filter(|(_, t)| t["text"] == "I" || t["text"] == "IH")
            .flat_map(|(g, _)| doc.group_members_of(*g))
            .collect();
        assert!(carve.vectors.iter().all(|v| filled.contains(v)));
        assert!(!made("S1 - Cut out").stale, "plain vectors only move");
    }

    #[test]
    fn counters_number_the_plates_padded() {
        let counter = Counter {
            start: 7,
            step: 2,
            digits: 3,
        };
        let assigned = [Assignment {
            variable: "n".into(),
            column: 0,
        }];
        assert_eq!(fill("No. !!n!!", &assigned, &[], &counter, 2), "No. 011");
    }

    #[test]
    fn plates_fill_the_material_then_go_on_to_another_sheet() {
        let material = Rect::new(Point::ZERO, Point::new(100.0, 50.0));
        let plate = Rect::new(Point::ZERO, Point::new(40.0, 20.0));
        let l = Params::default().layout;
        // 100 − 10 margins fits 2 across (40 + 5 + 40); 50 − 10 fits 1 down (20 + 5 + 20 = 45)
        let spots = layout(material, plate, &l, 3).unwrap();
        assert_eq!(spots[0], (0, Point::new(5.0, 25.0)));
        assert_eq!(spots[1], (0, Point::new(50.0, 25.0)));
        assert_eq!(spots[2].0, 1, "a second sheet");
        let big = Rect::new(Point::ZERO, Point::new(200.0, 200.0));
        assert!(layout(material, big, &l, 1).is_err());
    }

    #[test]
    fn the_templates_toolpaths_are_made_again_for_each_sheet_of_plates() {
        use cut_craft_core::toolpath::{MoveKind, Tool};
        let mut doc = template();
        let border = *doc.selection.iter().last().unwrap();
        let at = |x, y, z| DVec3::new(x, y, z);
        let moves = vec![
            Move {
                kind: MoveKind::Rapid,
                to: at(5.0, 5.0, 5.0),
            },
            Move {
                kind: MoveKind::Plunge,
                to: at(5.0, 5.0, -2.0),
            },
            Move {
                kind: MoveKind::Cut,
                to: at(45.0, 5.0, -2.0),
            },
        ];
        let mut c = Change::builder(&doc);
        c.add_toolpath(Toolpath {
            name: "Profile".into(),
            strategy: "profile_toolpath".into(),
            params: serde_json::Value::Null,
            tool: Tool::default(),
            vectors: vec![border],
            moves: moves.clone(),
            visible: true,
            power: vec![],
            stale: false,
            group: None,
            drill_bank: None,
        });
        doc.apply(&c.finish()).unwrap();
        let without = doc.clone();
        let mut p = PlateProduction::initial_params(&doc);
        p.data.file = data_file("toolpaths", "name\nI\nH\n");
        run_on::<PlateProduction>(&mut doc, &p);
        let made: Vec<&Toolpath> = doc
            .toolpaths
            .values()
            .filter(|t| t.name.starts_with("S1 - "))
            .collect();
        assert_eq!(made.len(), 1, "one per template toolpath per sheet");
        let t = made[0];
        assert_eq!(
            (t.name.as_str(), t.strategy.as_str()),
            ("S1 - Profile", "profile_toolpath")
        );
        assert_eq!(
            t.moves.len(),
            2 * moves.len(),
            "the template's moves on each plate"
        );
        assert_eq!(t.vectors.len(), 2, "the two plates' borders");
        assert!(t.vectors.iter().all(|v| doc.vectors.contains_key(v)));
        // each plate's cut is the template's, shifted where the plate went
        let shift = t.moves[1].to - moves[1].to;
        assert!(shift.z.abs() < 1e-9 && (t.moves[2].to - moves[2].to - shift).length() < 1e-9);
        let second = t.moves[moves.len() + 1].to - moves[1].to;
        assert!(second != shift && second.z.abs() < 1e-9);
        // without the option, only the vectors
        let mut doc = without;
        p.create_toolpaths = false;
        run_on::<PlateProduction>(&mut doc, &p);
        assert!(doc.toolpaths.values().all(|t| !t.name.starts_with("S1 - ")));
    }
}
