//! Extend Vectors: two selected open vectors extended in straight lines from their facing
//! ends to where those ends' lines meet.

use cut_craft_core::document::Document;
use cut_craft_core::geometry::{Path, Point, Segment};
use cut_craft_core::{Change, Command, CommandError, Meta, NoParams};

pub struct CloseExtendVector;

impl Command for CloseExtendVector {
    const ID: &'static str = "close_extend_vector";
    type Params = NoParams;

    fn meta() -> Meta {
        Meta {
            label: "Extend Vectors",
            group: "Edit Objects",
            icon: "drawing/create_vectors/close_extend_vector.svg",
            shortcut: "",
        }
    }

    fn run(doc: &Document, _: &NoParams) -> Result<Change, CommandError> {
        let (ids, paths) = doc.selected_paths(|p| !p.closed);
        let ([a_id, b_id], [a, b]) = (ids.as_slice(), paths.as_slice()) else {
            return Err(CommandError::Failed(
                "select two open vectors to extend".into(),
            ));
        };
        // ponytail: Aspire picks the ends by clicking; here the nearest pair of ends faces
        let ends = |p: &Path| [false, true].map(|at_end| (at_end, end(p, at_end)));
        let (a_end, b_end) = ends(a)
            .into_iter()
            .flat_map(|x| ends(b).into_iter().map(move |y| (x, y)))
            .min_by(|(x, y), (u, v)| x.1.0.distance(y.1.0).total_cmp(&u.1.0.distance(v.1.0)))
            .expect("four pairs");
        let Some(meet) = meeting(a_end.1, b_end.1) else {
            return Err(CommandError::Failed(
                "the ends are parallel: they never meet".into(),
            ));
        };
        let mut c = Change::builder(doc);
        c.replace(*a_id, extended(a, a_end.0, meet));
        c.replace(*b_id, extended(b, b_end.0, meet));
        Ok(c.finish())
    }
}

/// An end of `p` and the direction it leaves in.
fn end(p: &Path, at_end: bool) -> (Point, Point) {
    let pts = p.flatten(cut_craft_core::polygon::TOLERANCE);
    let (tip, before) = if at_end {
        (pts[pts.len() - 1], pts[pts.len() - 2])
    } else {
        (pts[0], pts[1])
    };
    (tip, (tip - before).normalize_or_zero())
}

/// Where two rays' lines cross, if they are not parallel and it lies ahead of both.
fn meeting((from, dir): (Point, Point), (other, other_dir): (Point, Point)) -> Option<Point> {
    let det = dir.perp_dot(other_dir);
    if det.abs() < 1e-12 {
        return None;
    }
    let gap = other - from;
    let ahead = gap.perp_dot(other_dir) / det;
    let other_ahead = gap.perp_dot(dir) / det;
    (ahead >= -1e-9 && other_ahead >= -1e-9).then(|| from + dir * ahead)
}

fn extended(p: &Path, at_end: bool, to: Point) -> Path {
    let mut out = p.clone();
    if at_end {
        out.segments.push(Segment::Line { to });
    } else {
        out = out.reversed();
        out.segments.push(Segment::Line { to });
        out = out.reversed();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::testing::{document_with, run_on};

    fn two_lines() -> Document {
        document_with(vec![
            Path::polyline(&[Point::new(0.0, 0.0), Point::new(5.0, 0.0)], false),
            Path::polyline(&[Point::new(10.0, 10.0), Point::new(10.0, 4.0)], false),
        ])
    }

    cut_craft_core::contract_tests!(CloseExtendVector, two_lines);

    #[test]
    fn both_lines_reach_their_crossing() {
        let mut doc = two_lines();
        run_on::<CloseExtendVector>(&mut doc, &NoParams {});
        for v in doc.vectors.values() {
            let pts = v.flatten(0.01);
            assert!(
                pts.iter().any(|q| q.distance(Point::new(10.0, 0.0)) < 1e-9),
                "{v:?}"
            );
        }
    }

    #[test]
    fn parallel_or_wrong_selections_are_refused() {
        let doc = document_with(vec![
            Path::polyline(&[Point::new(0.0, 0.0), Point::new(5.0, 0.0)], false),
            Path::polyline(&[Point::new(0.0, 5.0), Point::new(5.0, 5.0)], false),
        ]);
        assert!(CloseExtendVector::run(&doc, &NoParams {}).is_err());
        assert!(CloseExtendVector::run(&Document::default(), &NoParams {}).is_err());
    }
}
