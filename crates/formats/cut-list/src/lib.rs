//! Cut lists, the Advanced Machining Module's cabinet workflow made plain:
//!
//! - [`parts`] finds the job's parts on every sheet: a group, or a closed vector, with the
//!   vectors inside its box (holes, pockets, an engraved name) belonging to it. Each has a
//!   number in reading order (sheet by sheet, top row first, left to right), the name Name
//!   Part or Import Cut List gave it, and its size.
//! - [`CutList`] (File ▸ Export ▸ Cut List) writes them as a CSV table, identical parts on
//!   one row with how many there are and which sheets they are on.
//! - [`ImportCutList`] (File ▸ Import ▸ Import Cut List) reads a CSV cut list (part name,
//!   length, width, quantity, and a material that picks the layer) and draws each part as a
//!   named rectangle, ready for Nest Parts.

use cut_craft_core::document::{Document, EntityId, LayerId, Units};
use cut_craft_core::geometry::{Path, Point, Rect};
use cut_craft_core::{Change, CommandError, Exporter, FileFormat, Importer};
use std::collections::BTreeMap;
use std::fmt::Write as _;

/// The gap between parts Import Cut List lays out, mm: they are only waiting for Nest Parts.
const IMPORT_GAP: f64 = 10.0;
/// The most parts one cut list may make, so a typo in a quantity cannot hang the program.
const MOST_PARTS: u64 = 2000;

/// A part of the job, as the cut list and the labels see it.
#[derive(Clone, Debug, PartialEq)]
pub struct Part {
    /// From 1, in reading order through the job.
    pub number: usize,
    /// The name Name Part or Import Cut List gave it; `None` when it has none.
    pub name: Option<String>,
    /// The longer side of its box, mm.
    pub length: f64,
    /// The shorter side of its box, mm.
    pub width: f64,
    /// The sheet's material thickness, mm.
    pub thickness: f64,
    /// The sheet's name.
    pub sheet: String,
    /// The sheet's place in the sheet list.
    pub sheet_index: usize,
    /// Where it is on its sheet.
    pub bounds: Rect,
    /// Its vectors (on its sheet).
    pub vectors: Vec<EntityId>,
}

impl Part {
    /// Its name, or "Part 3" for part 3 when it has none.
    pub fn title(&self) -> String {
        self.name
            .clone()
            .unwrap_or_else(|| format!("Part {}", self.number))
    }
}

/// Every part of the job, sheet by sheet, numbered in reading order. Vectors on hidden layers
/// are left out, so construction lines can be kept off the list by hiding their layer.
pub fn parts(doc: &Document) -> Vec<Part> {
    let mut out = Vec::new();
    for (sheet_index, (sheet, on)) in sheets(doc).into_iter().enumerate() {
        let ids: Vec<EntityId> = on
            .vectors
            .keys()
            .copied()
            .filter(|&id| on.layer(on.layer_of(id)).is_none_or(|l| l.visible))
            .collect();
        let mut found = clusters(&on, &ids);
        // reading order: the highest top first, then the leftmost; tops within a millimetre
        // are one row
        found.sort_by(|(_, a), (_, b)| {
            let (ta, tb) = ((a.max.y).round(), (b.max.y).round());
            tb.total_cmp(&ta).then(a.min.x.total_cmp(&b.min.x))
        });
        for (vectors, bounds) in found {
            let size = bounds.size();
            let name = vectors
                .iter()
                .filter_map(|id| on.vector_groups.get(id))
                .find_map(|g| on.part_names.get(g))
                .cloned();
            out.push(Part {
                number: out.len() + 1,
                name,
                length: size.x.max(size.y),
                width: size.x.min(size.y),
                thickness: on.job.thickness,
                sheet: sheet.clone(),
                sheet_index,
                bounds,
                vectors,
            });
        }
    }
    out
}

/// The parts among `ids` on `doc`'s active sheet: a group stays together, and anything whose
/// box lies inside another's belongs to that one. Clusters with no closed vector (a guide
/// line, an open curve) are not parts.
pub fn clusters(doc: &Document, ids: &[EntityId]) -> Vec<(Vec<EntityId>, Rect)> {
    let mut found: Vec<(Vec<EntityId>, Rect)> = Vec::new();
    for &id in ids {
        if found.iter().any(|(members, _)| members.contains(&id)) {
            continue;
        }
        let members: Vec<EntityId> = doc
            .group_members(id)
            .into_iter()
            .filter(|m| ids.contains(m))
            .collect();
        let Some(bounds) = members
            .iter()
            .map(|m| doc.vectors[m].bounds())
            .reduce(Rect::union)
        else {
            continue;
        };
        found.push((members, bounds));
    }
    // the biggest first, so each piece joins the outermost box around it
    found.sort_by(|(_, a), (_, b)| area(*b).total_cmp(&area(*a)));
    let mut out: Vec<(Vec<EntityId>, Rect)> = Vec::new();
    for (members, bounds) in found {
        let closed = members.iter().any(|m| doc.vectors[m].closed);
        match out.iter_mut().find(|(_, b)| b.contains_rect(bounds)) {
            Some(owner) => owner.0.extend(members),
            None if closed => out.push((members, bounds)),
            None => {}
        }
    }
    out
}

fn area(r: Rect) -> f64 {
    let s = r.size();
    s.x * s.y
}

/// Each sheet's name with the job as it is with that sheet active.
fn sheets(doc: &Document) -> Vec<(String, Document)> {
    if doc.sheets.len() <= 1 {
        let name = doc
            .sheets
            .first()
            .map_or_else(|| "Sheet 1".into(), |s| s.name.clone());
        return vec![(name, doc.clone())];
    }
    (0..doc.sheets.len())
        .filter_map(|i| {
            let mut on = doc.clone();
            if i != doc.active_sheet {
                let mut c = Change::builder(&on);
                c.activate_sheet(i);
                on.apply(&c.finish()).ok()?;
            }
            Some((doc.sheets[i].name.clone(), on))
        })
        .collect()
}

/// A row of the cut list: one kind of part.
#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    pub name: String,
    pub quantity: usize,
    pub length: f64,
    pub width: f64,
    pub thickness: f64,
    /// The sheets they are on, in job order.
    pub sheets: Vec<String>,
}

/// The parts as cut list rows: parts with the same name, size and thickness (to a hundredth
/// of a millimetre) are one row. Unnamed parts are "Unnamed part".
pub fn rows(parts: &[Part]) -> Vec<Row> {
    let near = |a: f64, b: f64| (a - b).abs() < 0.005;
    let mut out: Vec<Row> = Vec::new();
    for p in parts {
        let name = p.name.clone().unwrap_or_else(|| "Unnamed part".into());
        let same = |r: &&mut Row| {
            r.name == name
                && near(r.length, p.length)
                && near(r.width, p.width)
                && near(r.thickness, p.thickness)
        };
        if let Some(row) = out.iter_mut().find(same) {
            row.quantity += 1;
            if !row.sheets.contains(&p.sheet) {
                row.sheets.push(p.sheet.clone());
            }
        } else {
            out.push(Row {
                name,
                quantity: 1,
                length: p.length,
                width: p.width,
                thickness: p.thickness,
                sheets: vec![p.sheet.clone()],
            });
        }
    }
    out
}

/// A length in the job's units, as a plain number for a spreadsheet: up to two decimals in
/// millimetres, three in inches, without trailing zeros.
pub fn number(mm: f64, units: Units) -> String {
    let text = match units {
        Units::Mm => format!("{mm:.2}"),
        Units::Inch => format!("{:.3}", mm / 25.4),
    };
    text.trim_end_matches('0').trim_end_matches('.').to_owned()
}

/// The unit's short name for column headings.
pub fn unit(units: Units) -> &'static str {
    match units {
        Units::Mm => "mm",
        Units::Inch => "in",
    }
}

/// The cut list as CSV, in the job's units.
pub fn csv(doc: &Document) -> String {
    let units = doc.job.units;
    let u = unit(units);
    let mut out = format!("Part,Quantity,Length ({u}),Width ({u}),Thickness ({u}),Sheet\r\n");
    for r in rows(&parts(doc)) {
        let _ = write!(
            out,
            "{},{},{},{},{},{}\r\n",
            field(&r.name),
            r.quantity,
            number(r.length, units),
            number(r.width, units),
            number(r.thickness, units),
            field(&r.sheets.join("; ")),
        );
    }
    out
}

/// `s` as one CSV field: quoted when it holds a comma, a quote or a line break, and a name
/// starting like a formula (`=`, `+`, `-`, `@`) led by an apostrophe, so a spreadsheet shows
/// it as text instead of working it out.
fn field(s: &str) -> String {
    let guarded;
    let s = if s.starts_with(['=', '+', '-', '@', '\t', '\r']) {
        guarded = format!("'{s}");
        guarded.as_str()
    } else {
        s
    };
    if s.contains([',', '"', '\n', '\r', ';']) {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_owned()
    }
}

/// File ▸ Export ▸ Cut List.
pub struct CutList;

impl Exporter for CutList {
    const ID: &'static str = "cut_list";
    const FORMAT: FileFormat = FileFormat {
        label: "Cut list (CSV, opens in a spreadsheet)",
        extensions: &["csv"],
    };

    fn export(doc: &Document) -> Result<Vec<u8>, CommandError> {
        if parts(doc).is_empty() {
            return Err(CommandError::Failed(
                "The job has no parts to list. Draw or import closed shapes first.".into(),
            ));
        }
        Ok(csv(doc).into_bytes())
    }
}

/// A line of an imported cut list, in millimetres.
#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    pub name: String,
    pub length: f64,
    pub width: f64,
    pub quantity: u32,
    /// Empty when the list has no material column.
    pub material: String,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Column {
    Name,
    Length,
    Width,
    Quantity,
    Material,
    Other,
}

fn column(heading: &str) -> Column {
    let h = heading.trim().to_lowercase();
    let h = h.split(['(', '[']).next().unwrap_or_default().trim();
    match h {
        "name" | "part" | "part name" | "description" | "label" | "item" => Column::Name,
        "length" | "len" | "l" | "height" | "h" | "long" => Column::Length,
        "width" | "w" | "wide" => Column::Width,
        "quantity" | "qty" | "count" | "copies" | "pcs" | "pieces" | "number" | "no" | "#" => {
            Column::Quantity
        }
        "material" | "board" | "stock" => Column::Material,
        _ => Column::Other,
    }
}

/// The cut list in `text`, its sizes in `units`. Columns are found by their headings (Name,
/// Length, Width, Quantity, Material, in any order; others are ignored); without a heading
/// row they are Name, Length, Width, Quantity. Commas, semicolons or tabs separate fields.
pub fn read(text: &str, units: Units) -> Result<Vec<Entry>, String> {
    let text = text.trim_start_matches('\u{feff}');
    let first = text
        .lines()
        .find(|l| !l.trim().is_empty())
        .unwrap_or_default();
    let delimiter = [',', ';', '\t']
        .into_iter()
        .max_by_key(|d| first.matches(*d).count())
        .filter(|d| first.contains(*d))
        .unwrap_or(',');
    let records = cut_craft_core::csv::records(text, delimiter);
    // blank lines before the headings are skipped; rows keep their line numbers
    let start = records
        .iter()
        .position(|r| r.iter().any(|f| !f.trim().is_empty()))
        .unwrap_or(records.len());
    let Some(head) = records.get(start) else {
        return Err("The cut list is empty.".into());
    };
    let scale = match units {
        Units::Mm => 1.0,
        Units::Inch => 25.4,
    };
    let headed = head.iter().any(|f| column(f) != Column::Other);
    let columns: Vec<Column> = if headed {
        head.iter().map(|f| column(f)).collect()
    } else {
        vec![
            Column::Name,
            Column::Length,
            Column::Width,
            Column::Quantity,
            Column::Material,
        ]
    };
    for needed in [Column::Length, Column::Width] {
        if !columns.contains(&needed) {
            let which = if needed == Column::Length {
                "Length"
            } else {
                "Width"
            };
            return Err(format!(
                "The cut list has no {which} column. Give the first row headings such as \
                 Name, Length, Width, Quantity."
            ));
        }
    }
    let mut out = Vec::new();
    for (i, record) in records.iter().enumerate().skip(start + usize::from(headed)) {
        if record.iter().all(|f| f.trim().is_empty()) {
            continue;
        }
        let row = i + 1;
        let get = |c: Column| {
            columns
                .iter()
                .position(|&x| x == c)
                .and_then(|at| record.get(at))
                .map_or("", |f| f.trim())
        };
        let size = |c: Column, what: &str| -> Result<f64, String> {
            let raw = get(c);
            let value: f64 = decimal(raw, delimiter).ok_or_else(|| {
                format!(
                    "Row {row}: the {what} \"{raw}\" is not a number. Use digits, like 450 or 17.5."
                )
            })?;
            if value > 0.0 && value.is_finite() {
                Ok(value * scale)
            } else {
                Err(format!("Row {row}: the {what} must be more than zero."))
            }
        };
        let quantity = match get(Column::Quantity) {
            "" => 1,
            raw => raw.parse::<u32>().map_err(|_| {
                format!("Row {row}: the quantity \"{raw}\" is not a whole number. Use 1, 2, 3...")
            })?,
        };
        out.push(Entry {
            name: get(Column::Name).to_owned(),
            length: size(Column::Length, "length")?,
            width: size(Column::Width, "width")?,
            quantity,
            material: get(Column::Material).to_owned(),
        });
    }
    if out.iter().all(|e| e.quantity == 0) {
        return Err("The cut list has no parts in it (every quantity is 0, or no rows).".into());
    }
    Ok(out)
}

/// A number written with a point, or with a comma when commas do not separate fields (as
/// spreadsheets in many European languages write them).
fn decimal(raw: &str, delimiter: char) -> Option<f64> {
    let raw = raw.trim();
    raw.parse()
        .ok()
        .or_else(|| (delimiter != ',').then(|| raw.replace(',', ".").parse().ok())?)
}

/// File ▸ Import ▸ Import Cut List: each part of the list, as many times as its quantity, as
/// a named rectangle (length along X), laid out in rows from the job's bottom-left corner a
/// little apart and selected, so Nest Parts can arrange them next. A material puts its parts
/// on a layer of that name.
pub struct ImportCutList;

impl Importer for ImportCutList {
    const ID: &'static str = "import_cut_list";
    const FORMAT: FileFormat = FileFormat {
        label: "Cut list (CSV)",
        extensions: &["csv"],
    };

    fn import(doc: &Document, bytes: &[u8]) -> Result<Change, CommandError> {
        let entries = read(&cut_craft_core::csv::decode(bytes), doc.job.units)
            .map_err(CommandError::Failed)?;
        let total: u64 = entries.iter().map(|e| u64::from(e.quantity)).sum();
        if total > MOST_PARTS {
            return Err(CommandError::Failed(format!(
                "The cut list asks for {total} parts; Cut Craft imports up to {MOST_PARTS} at \
                 once. Check the quantities, or split the list into smaller files."
            )));
        }
        let area = doc.job.area();
        let mut c = Change::builder(doc);
        let mut layers: BTreeMap<String, LayerId> = BTreeMap::new();
        let mut group = doc.next_group();
        let (mut x, mut y, mut row) = (area.min.x, area.min.y, 0.0_f64);
        let mut made = Vec::new();
        for e in &entries {
            let layer = (!e.material.is_empty()).then(|| {
                *layers.entry(e.material.clone()).or_insert_with(|| {
                    doc.layers
                        .iter()
                        .find(|l| l.name == e.material)
                        .map_or_else(|| c.add_layer(&e.material), |l| l.id)
                })
            });
            for _ in 0..e.quantity {
                if x > area.min.x && x + e.length > area.max.x {
                    (x, y, row) = (area.min.x, y + row + IMPORT_GAP, 0.0);
                }
                let p = Point::new;
                let rect = Path::polyline(
                    &[
                        p(x, y),
                        p(x + e.length, y),
                        p(x + e.length, y + e.width),
                        p(x, y + e.width),
                    ],
                    true,
                );
                let id = c.add(rect);
                c.set_group(id, Some(group));
                if !e.name.is_empty() {
                    c.set_part_name(group, Some(e.name.clone()));
                }
                if let Some(layer) = layer {
                    c.move_to_layer(id, layer);
                }
                made.push(id);
                group += 1;
                x += e.length + IMPORT_GAP;
                row = row.max(e.width);
            }
        }
        c.note(format!(
            "Imported {} part{} from the cut list. Next, Nest Parts arranges them on your \
             material.",
            made.len(),
            if made.len() == 1 { "" } else { "s" }
        ));
        c.select(made);
        Ok(c.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::testing::{document_with, square};

    fn imported(csv: &str) -> Document {
        let mut doc = Document::default();
        let change = ImportCutList::import(&doc, csv.as_bytes()).unwrap();
        doc.apply(&change).unwrap();
        doc
    }

    #[test]
    fn a_cut_list_becomes_named_rectangles_that_list_back_out() {
        let doc =
            imported("Name,Length,Width,Quantity\r\nSide,720,560,2\r\nShelf,\"564\",540,1\r\n");
        assert_eq!(doc.vectors.len(), 3);
        assert_eq!(doc.selection.len(), 3, "selected, ready for Nest Parts");
        let found = parts(&doc);
        assert_eq!(found.len(), 3);
        let names: Vec<_> = found.iter().map(Part::title).collect();
        assert_eq!(names.iter().filter(|n| *n == "Side").count(), 2);
        let side = found.iter().find(|p| p.title() == "Side").unwrap();
        assert!((side.length - 720.0).abs() < 1e-9 && (side.width - 560.0).abs() < 1e-9);
        let table = rows(&found);
        assert_eq!(table.len(), 2);
        assert_eq!(table.iter().find(|r| r.name == "Side").unwrap().quantity, 2);
        let text = csv(&doc);
        assert!(text.starts_with("Part,Quantity,Length (mm),Width (mm),Thickness (mm),Sheet"));
        assert!(text.contains("Side,2,720,560,"), "{text}");
    }

    #[test]
    fn each_part_s_name_survives_undo_and_redo() {
        let doc = Document::default();
        let change = ImportCutList::import(&doc, b"Side,720,560,2").unwrap();
        let mut after = doc.clone();
        after.apply(&change).unwrap();
        assert_eq!(after.part_names.len(), 2, "each copy is its own part");
        after.apply(&change.inverse()).unwrap();
        assert_eq!(after, doc);
    }

    #[test]
    fn headings_in_any_order_semicolons_and_decimal_commas() {
        let entries = read(
            "Qty;Material;Width (mm);Length (mm);Part\n3;Birch 18;300,5;600;Door\n\n",
            Units::Mm,
        )
        .unwrap();
        assert_eq!(
            entries,
            vec![Entry {
                name: "Door".into(),
                length: 600.0,
                width: 300.5,
                quantity: 3,
                material: "Birch 18".into(),
            }]
        );
        // inches in an inch job
        let e = read("Top,24,12", Units::Inch).unwrap();
        assert!((e[0].length - 609.6).abs() < 1e-9 && e[0].quantity == 1);
    }

    #[test]
    fn blank_lines_before_the_headings_and_formula_names_are_harmless() {
        let e = read("\n\nName,Length,Width\nSide,10,10\n", Units::Mm).unwrap();
        assert_eq!(e[0].name, "Side");
        assert_eq!(field("=SUM(A1)"), "'=SUM(A1)");
        assert_eq!(field("-1,5"), "\"'-1,5\"");
        assert_eq!(field("Side"), "Side");
    }

    #[test]
    fn mistakes_say_which_row_and_how_to_fix_it() {
        let e = read("Name,Length,Width\nSide,abc,560\n", Units::Mm).unwrap_err();
        assert!(
            e.contains("Row 2") && e.contains("length") && e.contains("number"),
            "{e}"
        );
        let e = read("Name,Length,Width\nSide,0,560\n", Units::Mm).unwrap_err();
        assert!(e.contains("more than zero"), "{e}");
        let e = read("Name,Size\nSide,600\n", Units::Mm).unwrap_err();
        assert!(e.contains("no Length column"), "{e}");
        let e = read("Name,Length,Width,Qty\nSide,1,1,two\n", Units::Mm).unwrap_err();
        assert!(e.contains("whole number"), "{e}");
        let doc = Document::default();
        let e = ImportCutList::import(&doc, b"Side,10,10,5000").unwrap_err();
        assert!(e.to_string().contains("up to 2000"), "{e}");
    }

    #[test]
    fn a_material_puts_its_parts_on_a_layer_of_that_name() {
        let doc = imported("Name,Length,Width,Material\nA,10,10,MDF\nB,10,10,MDF\nC,10,10,Ply\n");
        let names: Vec<_> = doc.layers.iter().map(|l| l.name.as_str()).collect();
        assert_eq!(names, ["Layer 1", "MDF", "Ply"]);
    }

    #[test]
    fn excel_s_windows_csv_reads() {
        let mut bytes = b"Name,Length,Width\nT\xe9te,10,10\n".to_vec();
        let doc = Document::default();
        let mut after = doc.clone();
        after
            .apply(&ImportCutList::import(&doc, &bytes).unwrap())
            .unwrap();
        assert_eq!(parts(&after)[0].title(), "T\u{e9}te");
        bytes.clear();
    }

    #[test]
    fn holes_and_text_inside_a_part_belong_to_it_and_open_lines_are_not_parts() {
        let p = Point::new;
        let doc = document_with(vec![
            square(0.0, 0.0, 100.0),
            square(10.0, 10.0, 20.0),
            square(200.0, 0.0, 50.0),
            Path::polyline(&[p(300.0, 0.0), p(400.0, 0.0)], false),
        ]);
        let found = parts(&doc);
        assert_eq!(found.len(), 2);
        assert_eq!(
            found[0].vectors.len(),
            2,
            "the hole is part of the big square"
        );
        assert_eq!(found[0].title(), "Part 1");
        assert!((found[1].length - 50.0).abs() < 1e-9);
    }

    #[test]
    fn parts_are_numbered_top_row_first_left_to_right() {
        let doc = document_with(vec![
            square(100.0, 0.0, 10.0),
            square(0.0, 0.0, 10.0),
            square(50.0, 100.0, 10.0),
        ]);
        let order: Vec<f64> = parts(&doc).iter().map(|p| p.bounds.min.x).collect();
        assert_eq!(order, [50.0, 0.0, 100.0]);
    }

    #[test]
    fn hidden_layers_are_left_off_and_every_sheet_is_listed() {
        let mut doc = cut_craft_core::testing::two_sheets();
        let mut c = Change::builder(&doc);
        c.activate_sheet(1);
        doc.apply(&c.finish()).unwrap();
        doc.apply(&Change::swap(&doc, [], vec![square(0.0, 0.0, 40.0)]))
            .unwrap();
        let on_both = parts(&doc);
        let sheets: Vec<_> = on_both.iter().map(|p| p.sheet.clone()).collect();
        assert!(sheets.len() >= 2, "{sheets:?}");
        assert_ne!(sheets.first(), sheets.last(), "parts from both sheets");
        for layer in &mut doc.layers {
            layer.visible = false;
        }
        assert_eq!(parts(&doc).len(), 0);
    }

    #[test]
    fn an_empty_job_says_what_to_do() {
        let e = CutList::export(&Document::default())
            .unwrap_err()
            .to_string();
        assert!(e.contains("Draw or import"), "{e}");
    }

    #[test]
    fn names_with_commas_are_quoted_and_inch_jobs_list_inches() {
        let mut doc = imported("Name,Length,Width\n\"Back, left\",254,127\n");
        doc.job.units = Units::Inch;
        let text = csv(&doc);
        assert!(text.contains("\"Back, left\",1,10,5,"), "{text}");
        assert!(text.contains("Length (in)"));
    }
}
