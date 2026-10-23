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

pub fn runs_on_the_sample_document<C: Command>() -> Change {
    C::run(&sample_document(), &C::Params::default())
        .unwrap_or_else(|e| panic!("{} failed on the sample document: {e}", C::ID))
}

pub fn inverse_restores_the_document<C: Command>() {
    let original = sample_document();
    let change = runs_on_the_sample_document::<C>();
    let mut doc = original.clone();
    doc.apply(&change)
        .expect("a command's change applies to the document it was built from");
    doc.apply(&change.inverse()).expect("the inverse applies");
    assert_eq!(
        doc,
        original,
        "{}: undo did not restore the document",
        C::ID
    );
}

/// Generates the shared contract tests for a command type.
#[macro_export]
macro_rules! contract_tests {
    ($cmd:ty) => {
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
            fn runs_on_the_sample_document() {
                $crate::testing::runs_on_the_sample_document::<$cmd>();
            }

            #[test]
            fn inverse_restores_the_document() {
                $crate::testing::inverse_restores_the_document::<$cmd>();
            }
        }
    };
}
