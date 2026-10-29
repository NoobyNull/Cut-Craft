//! Align Objects: the selected vectors lined up on an edge or the centre of the selection, or
//! centred in the material. Ten commands that differ only in where each path moves to.

use cut_craft_core::document::Document;
use cut_craft_core::geometry::Point;
use cut_craft_core::{Change, Command, CommandError, Meta, NoParams};

fn meta(label: &'static str, icon: &'static str, shortcut: &'static str) -> Meta {
    Meta {
        label,
        group: "Align",
        icon,
        shortcut,
    }
}

pub struct AlignLeft;

impl Command for AlignLeft {
    const ID: &'static str = "align_left";
    type Params = NoParams;

    fn meta() -> Meta {
        meta("Align Left", "drawing/align/left.png", "")
    }

    fn run(doc: &Document, _: &NoParams) -> Result<Change, CommandError> {
        Change::align_selected(doc, |path, sel| Point::new(sel.min.x - path.min.x, 0.0))
    }
}

pub struct AlignRight;

impl Command for AlignRight {
    const ID: &'static str = "align_right";
    type Params = NoParams;

    fn meta() -> Meta {
        meta("Align Right", "drawing/align/right.png", "")
    }

    fn run(doc: &Document, _: &NoParams) -> Result<Change, CommandError> {
        Change::align_selected(doc, |path, sel| Point::new(sel.max.x - path.max.x, 0.0))
    }
}

pub struct AlignTop;

impl Command for AlignTop {
    const ID: &'static str = "align_top";
    type Params = NoParams;

    fn meta() -> Meta {
        meta("Align Top", "drawing/align/top.png", "")
    }

    fn run(doc: &Document, _: &NoParams) -> Result<Change, CommandError> {
        Change::align_selected(doc, |path, sel| Point::new(0.0, sel.max.y - path.max.y))
    }
}

pub struct AlignBottom;

impl Command for AlignBottom {
    const ID: &'static str = "align_bottom";
    type Params = NoParams;

    fn meta() -> Meta {
        meta("Align Bottom", "drawing/align/bottom.png", "")
    }

    fn run(doc: &Document, _: &NoParams) -> Result<Change, CommandError> {
        Change::align_selected(doc, |path, sel| Point::new(0.0, sel.min.y - path.min.y))
    }
}

pub struct CenterHorizontally;

impl Command for CenterHorizontally {
    const ID: &'static str = "center_horizontally";
    type Params = NoParams;

    fn meta() -> Meta {
        meta(
            "Center Horizontally",
            "drawing/align/center_horizontally.png",
            "",
        )
    }

    fn run(doc: &Document, _: &NoParams) -> Result<Change, CommandError> {
        Change::align_selected(doc, |path, sel| {
            Point::new(sel.center().x - path.center().x, 0.0)
        })
    }
}

pub struct CenterVertically;

impl Command for CenterVertically {
    const ID: &'static str = "center_vertically";
    type Params = NoParams;

    fn meta() -> Meta {
        meta(
            "Center Vertically",
            "drawing/align/center_vertically.png",
            "",
        )
    }

    fn run(doc: &Document, _: &NoParams) -> Result<Change, CommandError> {
        Change::align_selected(doc, |path, sel| {
            Point::new(0.0, sel.center().y - path.center().y)
        })
    }
}

pub struct Center;

impl Command for Center {
    const ID: &'static str = "center";
    type Params = NoParams;

    fn meta() -> Meta {
        meta("Center", "drawing/align/center.png", "")
    }

    fn run(doc: &Document, _: &NoParams) -> Result<Change, CommandError> {
        // cascade: centring is centring horizontally, then vertically
        Change::chain(
            doc,
            &[&|d| CenterHorizontally::run(d, &NoParams {}), &|d| {
                CenterVertically::run(d, &NoParams {})
            }],
        )
    }
}

pub struct CenterHorizontallyInMaterial;

impl Command for CenterHorizontallyInMaterial {
    const ID: &'static str = "center_horizontally_in_material";
    type Params = NoParams;

    fn meta() -> Meta {
        meta(
            "Center Horizontally In Material",
            "drawing/align/center_horizontally_in_material.png",
            "",
        )
    }

    fn run(doc: &Document, _: &NoParams) -> Result<Change, CommandError> {
        let job = doc.job;
        Change::align_selected(doc, |_, sel| {
            Point::new(job.area().center().x - sel.center().x, 0.0)
        })
    }
}

pub struct CenterVerticallyInMaterial;

impl Command for CenterVerticallyInMaterial {
    const ID: &'static str = "center_vertically_in_material";
    type Params = NoParams;

    fn meta() -> Meta {
        meta(
            "Center Vertically In Material",
            "drawing/align/center_vertically_in_material.png",
            "",
        )
    }

    fn run(doc: &Document, _: &NoParams) -> Result<Change, CommandError> {
        let job = doc.job;
        Change::align_selected(doc, |_, sel| {
            Point::new(0.0, job.area().center().y - sel.center().y)
        })
    }
}

pub struct CenterInMaterial;

impl Command for CenterInMaterial {
    const ID: &'static str = "center_in_material";
    type Params = NoParams;

    fn meta() -> Meta {
        meta(
            "Center In Material",
            "drawing/align/center_in_material.png",
            "F9",
        )
    }

    fn run(doc: &Document, _: &NoParams) -> Result<Change, CommandError> {
        // cascade: centre horizontally in the material, then vertically
        Change::chain(
            doc,
            &[
                &|d| CenterHorizontallyInMaterial::run(d, &NoParams {}),
                &|d| CenterVerticallyInMaterial::run(d, &NoParams {}),
            ],
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::testing::{after, run_on, sample_document};

    /// Whether every selected vector's bounds satisfy `holds` after `C` runs on the sample.
    fn every_bounds<C: Command<Params = NoParams>>(
        holds: impl Fn(cut_craft_core::geometry::Rect) -> bool,
    ) -> bool {
        after::<C>(&NoParams {})
            .vectors
            .values()
            .all(|p| holds(p.bounds()))
    }

    mod align_left {
        use super::*;
        cut_craft_core::contract_tests!(AlignLeft);

        #[test]
        fn every_left_edge_meets_the_selection_s() {
            assert!(every_bounds::<AlignLeft>(|b| (b.min.x - 10.0).abs() < 1e-9));
        }
    }

    mod align_right {
        use super::*;
        cut_craft_core::contract_tests!(AlignRight);

        #[test]
        fn every_right_edge_meets_the_selection_s() {
            assert!(every_bounds::<AlignRight>(
                |b| (b.max.x - 225.0).abs() < 1e-9
            ));
        }
    }

    mod align_top {
        use super::*;
        cut_craft_core::contract_tests!(AlignTop);

        #[test]
        fn every_top_edge_meets_the_selection_s() {
            assert!(every_bounds::<AlignTop>(|b| (b.max.y - 125.0).abs() < 1e-9));
        }
    }

    mod align_bottom {
        use super::*;
        cut_craft_core::contract_tests!(AlignBottom);

        #[test]
        fn every_bottom_edge_meets_the_selection_s() {
            assert!(every_bounds::<AlignBottom>(
                |b| (b.min.y - 10.0).abs() < 1e-9
            ));
        }
    }

    mod center_horizontally {
        use super::*;
        cut_craft_core::contract_tests!(CenterHorizontally);

        #[test]
        fn every_centre_lines_up_vertically() {
            assert!(every_bounds::<CenterHorizontally>(|b| {
                (b.center().x - 117.5).abs() < 1e-9
            }));
        }
    }

    mod center_vertically {
        use super::*;
        cut_craft_core::contract_tests!(CenterVertically);

        #[test]
        fn every_centre_lines_up_horizontally() {
            assert!(every_bounds::<CenterVertically>(|b| {
                (b.center().y - 67.5).abs() < 1e-9
            }));
        }
    }

    mod center {
        use super::*;
        cut_craft_core::contract_tests!(Center);

        #[test]
        fn every_centre_meets_the_selection_s() {
            assert!(every_bounds::<Center>(|b| {
                (b.center() - Point::new(117.5, 67.5)).length() < 1e-9
            }));
        }
    }

    mod center_horizontally_in_material {
        use super::*;
        cut_craft_core::contract_tests!(CenterHorizontallyInMaterial);

        #[test]
        fn selection_centre_is_on_the_job_s_vertical_centre_line() {
            let doc = after::<CenterHorizontallyInMaterial>(&NoParams {});
            let c = doc.selection_bounds().unwrap().center();
            assert!((c.x - 300.0).abs() < 1e-9, "{c:?}");
        }

        #[test]
        fn keeps_the_selection_s_layout() {
            let before = sample_document();
            let doc = after::<CenterHorizontallyInMaterial>(&NoParams {});
            let size = |d: &Document| d.selection_bounds().unwrap().size();
            assert!((size(&doc) - size(&before)).length() < 1e-9);
        }
    }

    mod center_vertically_in_material {
        use super::*;
        cut_craft_core::contract_tests!(CenterVerticallyInMaterial);

        #[test]
        fn selection_centre_is_on_the_job_s_horizontal_centre_line() {
            let doc = after::<CenterVerticallyInMaterial>(&NoParams {});
            let c = doc.selection_bounds().unwrap().center();
            assert!((c.y - 200.0).abs() < 1e-9, "{c:?}");
        }

        #[test]
        fn keeps_the_selection_s_layout() {
            let before = sample_document();
            let doc = after::<CenterVerticallyInMaterial>(&NoParams {});
            let size = |d: &Document| d.selection_bounds().unwrap().size();
            assert!((size(&doc) - size(&before)).length() < 1e-9);
        }
    }

    mod center_in_material {
        use super::*;
        cut_craft_core::contract_tests!(CenterInMaterial);

        #[test]
        fn the_material_centre_is_wherever_the_datum_puts_it() {
            let mut doc = sample_document();
            doc.job.datum = cut_craft_core::document::Datum::TopRight;
            run_on::<CenterInMaterial>(&mut doc, &NoParams {});
            let c = doc.selection_bounds().unwrap().center();
            assert!(c.distance(doc.job.area().center()) < 1e-9, "{c:?}");
        }

        #[test]
        fn selection_centre_is_the_job_centre() {
            let c = after::<CenterInMaterial>(&NoParams {})
                .selection_bounds()
                .unwrap()
                .center();
            assert!((c - Point::new(300.0, 200.0)).length() < 1e-9, "{c:?}");
        }
    }
}
