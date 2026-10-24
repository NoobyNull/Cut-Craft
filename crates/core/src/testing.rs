//! Shared tests every command must pass (`contract_tests!`) and a sample document to run
//! them on. Enabled with the `testing` feature; command crates use it as a dev-dependency.

use crate::document::Document;
use crate::geometry::{Path, Point};
use crate::{Change, Command, Registry};

/// Two squares, an open polyline and a circle, all selected.
pub fn sample_document() -> Document {
    let p = Point::new;
    let mut doc = Document::default();
    let mut c = Change::builder(&doc);
    let ids = [
        c.add(Path::polyline(
            &[p(10.0, 10.0), p(30.0, 10.0), p(30.0, 30.0), p(10.0, 30.0)],
            true,
        )),
        c.add(Path::polyline(
            &[p(50.0, 20.0), p(80.0, 20.0), p(80.0, 40.0), p(50.0, 40.0)],
            true,
        )),
        c.add(Path::polyline(
            &[p(100.0, 10.0), p(120.0, 50.0), p(140.0, 10.0)],
            false,
        )),
        c.add(Path::circle(p(200.0, 100.0), 25.0)),
    ];
    c.select(ids);
    doc.apply(&c.finish()).expect("sample document is valid");
    doc
}

pub fn id_is_a_slug_and_meta_is_labelled<C: Command>() {
    assert!(
        !C::ID.is_empty()
            && C::ID
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_'),
        "id `{}` must be snake_case",
        C::ID
    );
    assert!(!C::meta().label.is_empty(), "{} has no label", C::ID);
}

pub fn default_params_round_trip_through_json<C: Command>() {
    let json = serde_json::to_value(C::Params::default()).expect("params serialize");
    let back: C::Params = serde_json::from_value(json.clone()).expect("params deserialize");
    assert_eq!(serde_json::to_value(back).unwrap(), json);
}

pub fn schema_describes_an_object<C: Command>() {
    let mut registry = Registry::default();
    registry.add::<C>();
    let schema = registry.get(C::ID).unwrap().schema();
    assert_eq!(
        schema["type"],
        "object",
        "{} params must be a struct",
        C::ID
    );
}

pub fn runs_on<C: Command>(doc: &Document) -> Change {
    C::run(doc, &C::Params::default())
        .unwrap_or_else(|e| panic!("{} failed on its contract document: {e}", C::ID))
}

pub fn inverse_restores<C: Command>(original: &Document) {
    let change = runs_on::<C>(original);
    let mut doc = original.clone();
    doc.apply(&change)
        .expect("a command's change applies to the document it was built from");
    doc.apply(&change.inverse()).expect("the inverse applies");
    assert_eq!(
        &doc,
        original,
        "{}: undo did not restore the document",
        C::ID
    );
}

/// Generates the shared contract tests for a command type, run on the sample document or on
/// `$doc()` for commands whose preconditions the sample does not meet.
#[macro_export]
macro_rules! contract_tests {
    ($cmd:ty) => {
        $crate::contract_tests!($cmd, $crate::testing::sample_document);
    };
    ($cmd:ty, $doc:expr) => {
        mod contract {
            #[allow(unused_imports)]
            use super::*;

            #[test]
            fn id_is_a_slug_and_meta_is_labelled() {
                $crate::testing::id_is_a_slug_and_meta_is_labelled::<$cmd>();
            }

            #[test]
            fn default_params_round_trip_through_json() {
                $crate::testing::default_params_round_trip_through_json::<$cmd>();
            }

            #[test]
            fn schema_describes_an_object() {
                $crate::testing::schema_describes_an_object::<$cmd>();
            }

            #[test]
            fn runs_on_its_contract_document() {
                $crate::testing::runs_on::<$cmd>(&$doc());
            }

            #[test]
            fn inverse_restores_the_document() {
                $crate::testing::inverse_restores::<$cmd>(&$doc());
            }
        }
    };
}

/// A document holding `paths`, all selected.
pub fn document_with(paths: Vec<Path>) -> Document {
    let mut doc = Document::default();
    doc.apply(&Change::swap(&doc, [], paths))
        .expect("new paths always apply");
    doc
}

/// An axis-aligned square with its lower-left corner at (`x`, `y`).
pub fn square(x: f64, y: f64, side: f64) -> Path {
    let p = Point::new;
    Path::polyline(
        &[
            p(x, y),
            p(x + side, y),
            p(x + side, y + side),
            p(x, y + side),
        ],
        true,
    )
}

/// Runs a drawing command on an empty document; asserts it added exactly one path and
/// selected it; returns that path.
pub fn drawn_path<C: Command>(params: &C::Params) -> Path {
    let empty = Document::default();
    let change = C::run(&empty, params).unwrap_or_else(|e| panic!("{} failed: {e}", C::ID));
    let mut doc = empty.clone();
    doc.apply(&change).expect("change applies");
    assert_eq!(doc.vectors.len(), 1, "{} must add exactly one path", C::ID);
    assert_eq!(doc.selection.len(), 1, "{} must select what it drew", C::ID);
    doc.vectors.into_values().next().unwrap()
}

/// The error message a command gives for `params` on an empty document.
pub fn rejection<C: Command>(params: &C::Params) -> String {
    match C::run(&Document::default(), params) {
        Ok(_) => panic!("{} accepted parameters it should reject", C::ID),
        Err(e) => e.to_string(),
    }
}

/// Every vertex of a path made only of lines (start plus each segment end).
pub fn vertices(path: &Path) -> Vec<Point> {
    std::iter::once(path.start)
        .chain(path.segments.iter().map(crate::geometry::Segment::end))
        .collect()
}

/// Runs a command with `params` on `doc` and applies what it made.
pub fn run_on<C: Command>(doc: &mut Document, params: &C::Params) {
    let change = C::run(doc, params).unwrap_or_else(|e| panic!("{} failed: {e}", C::ID));
    doc.apply(&change).expect("change applies");
}

/// Runs a command with `params` on the sample document and returns the result.
pub fn after<C: Command>(params: &C::Params) -> Document {
    let mut doc = sample_document();
    run_on::<C>(&mut doc, params);
    doc
}

/// The sample document with an empty "Layer 2" on top; layer 1 stays current.
pub fn two_layers() -> Document {
    let mut doc = sample_document();
    let mut c = Change::builder(&doc);
    c.add_layer("Layer 2");
    let change = c.finish();
    doc.apply(&change).expect("adding a layer applies");
    doc
}

/// `doc` with layer `id` replaced by `edit` of it.
pub fn edit_layer(
    doc: &mut Document,
    id: crate::document::LayerId,
    edit: impl FnOnce(&mut crate::document::Layer),
) {
    let mut layer = doc.layer(id).expect("layer exists").clone();
    edit(&mut layer);
    let mut c = Change::builder(doc);
    c.replace_layer(layer);
    let change = c.finish();
    doc.apply(&change).expect("replacing a layer applies");
}

/// The error message a command gives for `params` on the sample document.
pub fn after_err<C: Command>(params: &C::Params) -> String {
    match C::run(&sample_document(), params) {
        Ok(_) => panic!("{} accepted parameters it should reject", C::ID),
        Err(e) => e.to_string(),
    }
}

/// Runs a toolpath strategy on a document holding `paths` (all selected); the toolpath it made.
pub fn made_toolpath<C: Command>(
    paths: Vec<Path>,
    params: &C::Params,
) -> Result<crate::toolpath::Toolpath, String> {
    let mut doc = document_with(paths);
    let change = C::run(&doc, params).map_err(|e| e.to_string())?;
    doc.apply(&change).expect("change applies");
    assert_eq!(
        doc.toolpaths.len(),
        1,
        "{} must add exactly one toolpath",
        C::ID
    );
    Ok(doc.toolpaths.into_values().next().unwrap())
}

/// The XY extent of a toolpath's cutting moves.
pub fn cut_bounds(toolpath: &crate::toolpath::Toolpath) -> crate::geometry::Rect {
    toolpath
        .moves
        .iter()
        .filter(|m| m.kind == crate::toolpath::MoveKind::Cut)
        .map(|m| crate::geometry::Rect::point(m.to.truncate()))
        .reduce(crate::geometry::Rect::union)
        .expect("the toolpath cuts something")
}

/// A 40 x 40 x 10 mm job holding an 8 mm high hemisphere in the middle, for 3D toolpaths.
pub fn domed() -> Document {
    use crate::component::{Combine, Component};
    use crate::document::Job;
    use crate::geometry::Rect;
    let mut doc = Document {
        job: Job {
            width: 40.0,
            height: 40.0,
            thickness: 10.0,
            ..Job::default()
        },
        ..Document::default()
    };
    let middle = Point::new(20.0, 20.0);
    let area = Rect::new(Point::new(12.0, 12.0), Point::new(28.0, 28.0));
    let dome = Component::from_fn("dome", Combine::Add, area, 0.2, |p| {
        let r = p.distance(middle);
        (r < 8.0).then(|| (64.0 - r * r).sqrt())
    });
    let mut c = Change::builder(&doc);
    c.add_component(dome);
    let change = c.finish();
    doc.apply(&change).expect("a component applies");
    doc
}

/// A document whose first entity is a toolpath (id 1), for toolpath commands' contract tests.
pub fn with_toolpath() -> Document {
    use crate::toolpath::{Tool, Toolpath};
    let mut doc = Document::default();
    let mut c = Change::builder(&doc);
    c.add_toolpath(Toolpath {
        name: "Profile".into(),
        strategy: "profile_toolpath".into(),
        params: serde_json::Value::Null,
        tool: Tool::default(),
        vectors: vec![],
        moves: vec![],
        visible: true,
        power: vec![],
    });
    let change = c.finish();
    doc.apply(&change).expect("a toolpath applies");
    doc
}

/// `with_toolpath` plus two more toolpaths: ids 1, 2, 3.
pub fn three_toolpaths() -> Document {
    let mut doc = with_toolpath();
    let first = doc.toolpaths[&crate::document::EntityId(1)].clone();
    let mut c = Change::builder(&doc);
    c.add_toolpath(first.clone());
    c.add_toolpath(first);
    let change = c.finish();
    doc.apply(&change).expect("toolpaths apply");
    doc
}

/// `domed` plus a second component: ids 1, 2.
pub fn two_components() -> Document {
    let mut doc = domed();
    let first = doc.components[&crate::document::EntityId(1)].clone();
    let mut c = Change::builder(&doc);
    c.add_component(first);
    let change = c.finish();
    doc.apply(&change).expect("a component applies");
    doc
}
