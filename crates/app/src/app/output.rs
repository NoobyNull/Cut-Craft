//! Toolpath output: from the job, the machine and Save Toolpaths' form to the G-code files
//! to write, and whether the checks before cutting let them be written. Save Toolpaths,
//! Send to Machine, View G-code, the Web API and the screenshots all go through it, so a
//! toolpath is saved the same way whichever of them asks.
//!
//! It never sees the window: [`Output`] is the job and machine as they are, gathered once
//! ([`super::CutCraftApp::output`]); everything here is worked out from that.

use super::{EntityId, SaveForm, files};
use cut_craft_core::document::Document;
use cut_craft_core::preflight::{Check, Finding, Severity};
use cut_craft_machines::{Library, Machine};
use cut_craft_post::Post;
use std::path::{Path, PathBuf};

/// The files a plan makes: each toolpath's id, file by file, in the order they are run.
pub(super) type Plan = Vec<Vec<EntityId>>;

/// The job and machine toolpaths are saved for.
pub(super) struct Output<'a> {
    /// The job as it is (its toolpaths not yet thinned to their tolerances).
    pub doc: &'a Document,
    /// The active machine; `None` before one is set up (Cut Craft's own post is used).
    pub machine: Option<&'a Machine>,
    pub posts: &'a Library,
    /// The toolpath and group picked in the toolpath list, for Save Selected.
    pub selected_toolpath: Option<EntityId>,
    pub selected_group: Option<&'a str>,
    /// The job's file; `None` before it is saved.
    pub job_path: Option<&'a Path>,
    /// Options ▸ Toolpaths: the tolerances the moves are thinned to as they are saved.
    pub tolerances: &'a crate::options::ToolpathSettings,
}

/// What the checks before cutting allow with what they found.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Verdict {
    /// Not written: the errors, each with its fix.
    Refused(String),
    /// Asked first (Save Anyway): every problem found, the question shows them all.
    Ask(Vec<String>),
    /// Written.
    Go,
}

impl Output<'_> {
    /// The machine has a tool changer (slots in Machine Configuration); with no machine set
    /// up there is none.
    pub(super) fn has_tool_changer(&self) -> bool {
        self.machine.is_some_and(|m| !m.slots.is_empty())
    }

    /// The tool changer's slots, slot 1 first; none without one.
    pub(super) fn slots(&self) -> Vec<Option<String>> {
        self.machine.map_or_default(|m| m.slots.clone())
    }

    /// The post the form saves through: the one picked, else the machine's default, with
    /// its bit changes made pauses when the form says to pause, or tools are changed by
    /// hand between files on a machine without a tool changer.
    pub(super) fn post(&self, form: &SaveForm) -> Result<Post, String> {
        let post = cut_craft_machines::post_for(
            self.machine,
            &self.doc.job,
            self.posts,
            form.post.as_deref(),
        )?;
        let changer = self.has_tool_changer();
        // where the machine's pauses take the head, and the spindle's time to speed
        let pause = self.machine.map_or_else(Default::default, Machine::pause);
        Ok(match form.changes {
            files::ToolChanges::Pause => post.with_pauses(pause),
            // changed by hand between files: no tool changer call (M6) for the first tool
            files::ToolChanges::SeparateFiles if !changer => post.with_pauses(pause),
            _ => post,
        })
    }

    /// The form's toolpaths, file by file: a file for each tool in turn unless the tool
    /// changer or pauses change them (and the post can).
    pub(super) fn plan(&self, form: &SaveForm, post: &Post) -> Result<Plan, String> {
        let plan = files::save_plan(
            self.doc,
            form.mode,
            self.selected_toolpath,
            self.selected_group,
            form.group,
        )?;
        // Group by bit: each bit's toolpaths cut together, before the files are split
        let plan = if self.machine.is_some_and(|m| m.group_by_bit) {
            let merge = form.group && form.mode == files::SaveMode::VisibleToMultipleFiles;
            files::grouped_by_bit(self.doc, &plan, merge)
        } else {
            plan
        };
        Ok(if by_tool(form, post) {
            files::by_tool(self.doc, plan)
        } else {
            plan
        })
    }

    /// The project's name ("Untitled" before it is saved).
    pub(super) fn project_name(&self) -> String {
        self.job_path
            .and_then(Path::file_stem)
            .map_or_else(|| "Untitled".into(), |s| s.to_string_lossy().into_owned())
    }

    /// The name the form's file is saved under (before its extension): a group's own name,
    /// the selected toolpath's when saving only it, else the project's.
    pub(super) fn output_name(&self, form: &SaveForm) -> String {
        let selected = (form.mode == files::SaveMode::Selected)
            .then(|| {
                self.selected_group.map(str::to_owned).or_else(|| {
                    let id = self.selected_toolpath?;
                    Some(self.doc.toolpaths.get(&id)?.name.clone())
                })
            })
            .flatten();
        selected.unwrap_or_else(|| self.project_name())
    }

    /// The form's file name, made safe to save under and with the post's extension.
    pub(super) fn file_name(&self, form: &SaveForm, post: &Post) -> String {
        let named = cut_craft_machines::send::safe_name(&self.output_name(form), false);
        format!("{named}.{}", post.extension)
    }

    /// The warning that the tool changer is chosen in `changes` but the machine has none
    /// set up, if it is.
    pub(super) fn no_tool_changer_for(&self, changes: files::ToolChanges) -> Option<Finding> {
        (changes == files::ToolChanges::ToolChanger && !self.has_tool_changer())
            .then(no_tool_changer)
    }

    /// The toolpaths of `plan` in `doc` that pass over a keep-out zone, by name.
    pub(super) fn not_kept_out(&self, doc: &Document, plan: &[Vec<EntityId>]) -> Vec<String> {
        plan.iter()
            .flatten()
            .filter_map(|id| doc.toolpaths.get(id))
            .filter(|t| !zones_entered(self.doc, t).is_empty())
            .map(|t| t.name.clone())
            .collect()
    }

    /// The job with its toolpaths thinned to their tolerances, as they are saved; refused
    /// when a toolpath of `plan`, as saved, would cut into a keep-out zone.
    pub(super) fn thinned(&self, plan: &[Vec<EntityId>]) -> Result<Document, String> {
        use super::toolpath_output::{CLIP_SETTING, can_clip};
        let thinned = self.tolerances.thinned(self.doc);
        // a toolpath that cannot abide by the keep-out zones is not saved; checked on the
        // moves saved, which the tolerance may have straightened
        let crossing = self.not_kept_out(&thinned, plan);
        if !crossing.is_empty() {
            let clippable = plan
                .iter()
                .flatten()
                .filter_map(|id| thinned.toolpaths.get(id))
                .any(|t| crossing.contains(&t.name) && can_clip(t));
            let way = if clippable {
                format!(", or turn on clipping ({CLIP_SETTING}) and recalculate")
            } else {
                " (laser and drill bank toolpaths are not clipped: move the part or the zone)"
                    .to_owned()
            };
            return Err(format!(
                "Not saved: {} would cut into a keep-out zone. Check the keep-out zones and \
                 their clearance are right{way}",
                crossing.join(", ")
            ));
        }
        Ok(thinned)
    }

    /// The files the form's plan makes through `post` at `path` (and beside it), with the
    /// G-code each holds as made at `made`, with the job's notes filled in from
    /// `note_variables`, in the order they are run: what Save Toolpaths writes and View
    /// G-code shows.
    pub(super) fn texts(
        &self,
        form: &SaveForm,
        post: &Post,
        path: &Path,
        made: chrono::NaiveDateTime,
        note_variables: &[cut_craft_core::notes::Variable],
    ) -> Result<Vec<(PathBuf, String)>, String> {
        let plan = self.plan(form, post)?;
        let thinned = self.thinned(&plan)?;
        let side = (form.add_side && self.doc.job.double_sided.is_some()).then_some(
            if self.doc.bottom_side {
                "Bottom"
            } else {
                "Top"
            },
        );
        let slots = self.slots();
        let per_tool = split_for(form, post);
        let parts = files::save_files(
            post,
            &thinned,
            &plan,
            path,
            side,
            per_tool,
            self.tolerances.file_names(),
        )?;
        // the notes' variables are the whole job's, not one file's; the post is the one
        // these files are made through, whichever the machine has first
        let mut vars = note_variables.to_vec();
        if let Some(v) = vars.iter_mut().find(|v| v.key == "machine.post") {
            v.value.clone_from(&post.name);
        }
        let notes = cut_craft_core::notes::gcode_lines(&self.doc.notes, &vars);
        let job = self.job_path;
        let mut texts = vec![];
        for (path, part) in &parts {
            // one file per tile, each tile moved to the job's corner
            if let Some(tiling) = self.doc.tiling {
                let tiles = tiling.tiles(self.doc.job.area());
                for (n, &tile) in tiles.iter().enumerate() {
                    let stem = path.file_stem().unwrap_or_default().to_string_lossy();
                    let extension = path.extension().unwrap_or_default().to_string_lossy();
                    let named = path.with_file_name(format!("{stem}_tile{}.{extension}", n + 1));
                    let in_tile = tiling.tiled(part, tile);
                    // nothing to cut there: no file to change tools and start the spindle for
                    if in_tile.toolpaths.is_empty() {
                        continue;
                    }
                    let text =
                        files::render_through(post, &slots, &in_tile, &named, job, made, &notes)?;
                    texts.push((named, text));
                }
            } else {
                let text = files::render_through(post, &slots, part, path, job, made, &notes)?;
                texts.push((path.clone(), text));
            }
        }
        Ok(texts)
    }

    /// What saving says once `written` are saved beside `path`.
    pub(super) fn saved_message(&self, written: &[PathBuf], path: &Path) -> String {
        saved_message(written, path, self.doc.tiling.is_some())
    }
}

/// Whether each tool gets files of its own: chosen, or the post cannot change bits.
fn by_tool(form: &SaveForm, post: &Post) -> bool {
    form.changes == files::ToolChanges::SeparateFiles || !post.changes_tools()
}

/// Whether files are split per tool (not by the form's Multiple Files), and so named after
/// their tool.
pub(super) fn split_for(form: &SaveForm, post: &Post) -> bool {
    by_tool(form, post) && form.mode != files::SaveMode::VisibleToMultipleFiles
}

/// The names of the keep-out zones of `doc` that `toolpath` cuts into (its cutter's edge
/// reaching one) or rapids over.
pub(super) fn zones_entered(
    doc: &Document,
    toolpath: &cut_craft_core::toolpath::Toolpath,
) -> Vec<String> {
    let radius = toolpath.tool.diameter / 2.0;
    cut_craft_core::keep_out::entered(doc.named_keep_out(), &toolpath.moves, radius)
        .into_iter()
        .map(|z| format!("\"{}\"", z.name))
        .collect()
}

/// The warning when the tool changer is chosen but the machine has none set up.
pub(super) fn no_tool_changer() -> Finding {
    Finding {
        severity: Severity::Warning,
        message: "Tool changer (ATC) is chosen, but the machine has no tool changer set up: \
                  the file changes bits with M6, which a machine without one stops on or \
                  ignores, cutting on with the wrong bit. Choose Separate file for each bit, \
                  or add the tool changer's slots in Machine Configuration"
            .into(),
        check: Check::ToolChanger,
    }
}

/// Why toolpaths with `found` are not saved: the errors, each with its fix; `None` when
/// there are none.
pub(super) fn refused(found: &[Finding]) -> Option<String> {
    let errors: Vec<&str> = found
        .iter()
        .filter(|f| f.severity == Severity::Error)
        .map(|f| f.message.as_str())
        .collect();
    (!errors.is_empty()).then(|| format!("Not saved: {}", errors.join("; ")))
}

/// Whether every warning in `found` was ticked as read (errors are never ticked).
pub(super) fn all_acknowledged(
    found: &[Finding],
    acknowledged: &std::collections::BTreeSet<String>,
) -> bool {
    found
        .iter()
        .filter(|f| f.severity != Severity::Error)
        .all(|f| acknowledged.contains(&f.message))
}

/// What the checks before cutting allow with what they `found`: refused for an error; else
/// asked about (Save Anyway) unless every warning was ticked as read in `acknowledged`, or
/// `accepted` already answers Save Anyway for each of them. The collet or spindle hitting
/// the material ([`Severity::MustAsk`]) is asked about even when ticked.
pub(super) fn verdict(
    found: &[Finding],
    acknowledged: &std::collections::BTreeSet<String>,
    accepted: &[String],
) -> Verdict {
    if let Some(refused) = refused(found) {
        return Verdict::Refused(refused);
    }
    let ticked = all_acknowledged(found, acknowledged);
    let must_ask = found.iter().any(|f| f.severity == Severity::MustAsk);
    let problems: Vec<String> = found.iter().map(|f| f.message.clone()).collect();
    // Save Anyway answered for these problems, every one of them shown then
    let answered = problems.iter().all(|p| accepted.contains(p));
    if !problems.is_empty() && (must_ask || !ticked) && !answered {
        Verdict::Ask(problems)
    } else {
        Verdict::Go
    }
}

/// What the status bar says once `written` are saved, beside `path`; `tiled` while Toolpath
/// Tiling is on, which names the files by tile.
pub(super) fn saved_message(written: &[PathBuf], path: &Path, tiled: bool) -> String {
    let saved = match written {
        [one] => format!("Saved toolpaths to {}", one.display()),
        _ => format!(
            "Saved {} toolpath files beside {}",
            written.len(),
            path.display()
        ),
    };
    if tiled {
        format!(
            "{saved}, one for each tile with something to cut: Toolpath Tiling is on \
             (Toolpaths ▸ Toolpath Tiling to turn it off)"
        )
    } else {
        saved
    }
}

/// Writes each file of `texts`; the files written, in order.
pub(super) fn write(texts: &[(PathBuf, String)]) -> Result<Vec<PathBuf>, String> {
    let mut written = vec![];
    for (path, text) in texts {
        files::write_text(path, text)?;
        written.push(path.clone());
    }
    Ok(written)
}

/// What became of an earlier save's files when new ones were written over them.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct WrittenOver {
    /// The new files, in order.
    pub written: Vec<PathBuf>,
    /// Where the earlier files were archived; `None` when they were replaced.
    pub archived: Option<PathBuf>,
}

/// Writes the files `make` makes (saved at `path`) where an earlier save left `earlier`,
/// archiving them (into a dated folder under Archive, named from `when`) or replacing
/// them. They are moved aside first, so the new files never mix with them, and put back if
/// the new files cannot be made or written: a failed save never leaves the user with no
/// G-code at all. The error says what was not done and where the earlier files are.
pub(super) fn write_over(
    make: impl FnOnce() -> Result<Vec<(PathBuf, String)>, String>,
    path: &Path,
    earlier: &[PathBuf],
    archiving: bool,
    when: chrono::NaiveDateTime,
) -> Result<WrittenOver, String> {
    let moved = if archiving {
        files::archive(earlier, path, when)
    } else {
        files::set_aside(earlier, path)
    };
    let folder = moved.map_err(|e| {
        let doing = if archiving { "archive" } else { "replace" };
        format!("Not saved: could not {doing} {e}")
    })?;
    match make().and_then(|texts| write(&texts)) {
        Ok(written) => {
            if !archiving {
                let _ = std::fs::remove_dir_all(&folder);
            }
            Ok(WrittenOver {
                written,
                archived: archiving.then_some(folder),
            })
        }
        Err(e) => {
            let kept = match files::put_back(earlier, &folder, path) {
                Ok(()) => "; the earlier files were left as they were".to_owned(),
                Err(back) => format!(
                    "; the earlier files are in {} (could not put back {back})",
                    folder.display()
                ),
            };
            Err(format!("{e}{kept}"))
        }
    }
}

#[cfg(test)]
#[path = "output/tests.rs"]
mod output_tests;
