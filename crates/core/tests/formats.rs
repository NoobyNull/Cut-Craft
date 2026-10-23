use aspire_core::document::Document;
use aspire_core::geometry::{Path, Point};
use aspire_core::{Change, CommandError, Exporter, FileFormat, Importer, Registry};
use std::fmt::Write as _;

/// Test format: one line per path, "x y" of its start point.
struct Starts;

impl Exporter for Starts {
    const ID: &'static str = "export_starts";
    const FORMAT: FileFormat = FileFormat {
        label: "Start points",
        extensions: &["starts", "st"],
    };

    fn export(doc: &Document) -> Result<Vec<u8>, CommandError> {
        let mut out = String::new();
        for p in doc.vectors.values() {
            let _ = writeln!(out, "{} {}", p.start.x, p.start.y);
        }
        Ok(out.into_bytes())
    }
}

impl Importer for Starts {
    const ID: &'static str = "import_starts";
    const FORMAT: FileFormat = FileFormat {
        label: "Start points",
        extensions: &["starts", "st"],
    };

    fn import(doc: &Document, bytes: &[u8]) -> Result<Change, CommandError> {
        let text = std::str::from_utf8(bytes).map_err(|e| CommandError::Failed(e.to_string()))?;
        let mut c = Change::builder(doc);
        for line in text.lines() {
            let mut n = line.split(' ').map(|v| {
                v.parse::<f64>()
                    .map_err(|e| CommandError::Failed(e.to_string()))
            });
            let start = Point::new(n.next().unwrap()?, n.next().unwrap()?);
            c.add(Path::polyline(&[start, start + Point::X], false));
        }
        Ok(c.finish())
    }
}

#[test]
fn exporters_and_importers_round_trip_through_the_registry() {
    let mut registry = Registry::default();
    registry.add_exporter::<Starts>().add_importer::<Starts>();
    let doc = aspire_core::testing::sample_document();
    let bytes = (registry.exporter("export_starts").unwrap().export)(&doc).unwrap();
    let importer = registry
        .importer_for("ST")
        .expect("extensions match case-insensitively");
    let mut fresh = Document::default();
    fresh
        .apply(&(importer.import)(&fresh, &bytes).unwrap())
        .unwrap();
    assert_eq!(fresh.vectors.len(), doc.vectors.len());
}

#[test]
fn unknown_extensions_have_no_importer() {
    let mut registry = Registry::default();
    registry.add_importer::<Starts>();
    assert!(registry.importer_for("dxf").is_none());
    assert_eq!(registry.importers().count(), 1);
}
