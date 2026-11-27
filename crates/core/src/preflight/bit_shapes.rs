//! Bits are known by name in the files, the tool changer and the bit changes, but each
//! toolpath keeps its own copy of its bit's shape: this check finds where a name stands for
//! two shapes, so a toolpath is never cut with a bit it was not made for.

use super::{Check, Finding};
use crate::document::Units;
use crate::toolpath::Toolpath;

/// Each toolpath's bit against the Bit Library's of the same name (a tool changer holder
/// with that name holds the library's), and every two toolpaths whose bits share a name but
/// not a shape (no bit change is written between them).
pub(super) fn check(
    toolpaths: &[&Toolpath],
    limits: &super::Limits,
    units: Units,
    found: &mut Vec<Finding>,
) {
    if let Some(library) = &limits.library {
        for toolpath in toolpaths {
            let tool = &toolpath.tool;
            let Some(listed) = library.get(&tool.name) else {
                continue;
            };
            let differences = tool.shape_differences(listed, units);
            if differences.is_empty() {
                continue;
            }
            let held = (limits.slots.iter())
                .position(|s| s.as_deref() == Some(tool.name.as_str()))
                .map(|i| i + 1);
            let (name, bit) = (&toolpath.name, &tool.name);
            let differ = in_each(&differences, "here", "in the library");
            found.push(match held {
                Some(slot) => Finding::error(
                    Check::BitShapes,
                    format!(
                        "{name} was made for a {bit} that is not the Bit Library's {bit} \
                         ({differ}). The tool changer's T{slot} holds the \
                         library's, so it would cut this toolpath with the wrong bit. Pick \
                         the bit again with the toolpath's Select… and press Calculate, or \
                         give one of them another name"
                    ),
                ),
                None => Finding::warning(
                    Check::BitShapes,
                    format!(
                        "{name} was made for a {bit} that is not the Bit Library's {bit} \
                         ({differ}). Put the bit it was made for in the \
                         machine, or pick the bit again with the toolpath's Select… and \
                         press Calculate"
                    ),
                ),
            });
        }
    }
    // the first toolpath with each name's shape, then any other shape of that name
    let mut seen: Vec<&Toolpath> = vec![];
    for toolpath in toolpaths {
        let tool = &toolpath.tool;
        let Some(first) = seen.iter().find(|t| t.tool.name == tool.name) else {
            seen.push(toolpath);
            continue;
        };
        let differences = first.tool.shape_differences(tool, units);
        if differences.is_empty() {
            continue;
        }
        found.push(Finding::error(
            Check::BitShapes,
            format!(
                "{} and {} cut with two different bits both called {} ({}). Bits \
                 are changed by name, so no bit change is made between them and one would \
                 be cut with the other's bit. Give one of the bits another name in the Bit \
                 Library, then pick it again with that toolpath's Select… and press \
                 Calculate",
                first.name,
                toolpath.name,
                tool.name,
                in_each(
                    &differences,
                    &format!("in {}", first.name),
                    &format!("in {}", toolpath.name)
                ),
            ),
        ));
    }
}

/// `differences` ("diameter 6 mm here, 6.35 mm") said with where each size is: `ours` in
/// place of "here", `theirs` after the other size; a whole different bit type as it is.
fn in_each(differences: &[String], ours: &str, theirs: &str) -> String {
    differences
        .iter()
        .map(|d| match d.split_once(" here, ") {
            Some((mine, other)) => format!("{mine} {ours}, {other} {theirs}"),
            None => d.clone(),
        })
        .collect::<Vec<_>>()
        .join("; ")
}
