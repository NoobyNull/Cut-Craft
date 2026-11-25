//! Machine Configuration's post-processors: the list, the editor and its fixes.

use super::{
    Capabilities, CapabilityDb, Entry, Family, Filter, HoverHint, Library, Look, Machine,
    MachineOutcome, Moves, PathBuf, Source, change_log, dialog_size, egui, icon_button, text_view,
    view, viewer,
};

/// Associated Post-Processors: the machine's posts (the default marked), Add and Remove, and
/// a right-click for View and Make Default.
pub(super) fn posts_section(
    ui: &mut egui::Ui,
    look: &Look,
    machine: &mut Machine,
    entries: &[Entry],
    (db, filter, was_open): (&CapabilityDb, &mut Filter, &mut bool),
    picked: &mut Option<usize>,
    viewing: &mut Option<(String, String)>,
) -> Option<MachineOutcome> {
    let mut asked = None;
    ui.group(|ui| {
        ui.set_width(ui.available_width());
        ui.strong(cut_craft_i18n::t!("Associated Post-Processors"));
        if machine.posts.is_empty() {
            ui.weak(cut_craft_i18n::t!(
                "None: toolpaths are saved through Cut Craft's own FluidNC post"
            ));
        }
        let default = machine.post().map(str::to_owned);
        let mut make_default = None;
        for (i, name) in machine.posts.iter().enumerate() {
            let missing = !entries.iter().any(|e| &e.name == name);
            let mut text = egui::RichText::new(name);
            if default.as_deref() == Some(name) {
                text = text.strong();
            }
            let row = ui.selectable_label(*picked == Some(i), text);
            let row = if missing {
                row.on_hover_hint(cut_craft_i18n::t!("Not in the post library"))
            } else {
                row
            };
            if row.clicked() {
                *picked = Some(i);
            }
            cut_craft_bug_report::context_menu(&row, |ui| {
                if ui.button(cut_craft_i18n::t!("View")).clicked() {
                    *viewing = entries.iter().find(|e| &e.name == name).and_then(view);
                    ui.close();
                }
                if ui.button(cut_craft_i18n::t!("Make Default")).clicked() {
                    make_default = Some(name.clone());
                    ui.close();
                }
            });
        }
        if let Some(name) = make_default {
            machine.default_post = Some(name);
        }
        ui.horizontal(|ui| {
            let unused: Vec<(Entry, Capabilities)> = entries
                .iter()
                .filter(|e| e.broken.is_none() && !machine.posts.contains(&e.name))
                .map(|e| (e.clone(), db.of(e)))
                .collect();
            let list = egui::ComboBox::from_id_salt("add_post")
                .selected_text("Add a post-processor")
                // stays open while the filters are changed; a post picked closes it
                .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
                .height(480.0)
                .show_ui(ui, |ui| {
                    // each opening starts afresh from the machine
                    if !*was_open {
                        *filter = Filter::starting(Some(machine));
                    }
                    let shown: Vec<(Entry, Capabilities)> = unused
                        .iter()
                        .filter(|(e, caps)| filter.shows(&e.name, caps))
                        .cloned()
                        .collect();
                    filter.ui(ui, shown.len(), unused.len());
                    ui.separator();
                    if shown.is_empty() {
                        ui.weak(NONE_SHOWN);
                    }
                    for (title, group) in groups(&shown) {
                        ui.strong(title);
                        for (e, caps) in group {
                            let row = ui
                                .selectable_label(false, format!("{}   {}", e.name, caps.tags()))
                                .on_hover_hint(&caps.about);
                            if row.clicked() {
                                machine.posts.push(e.name.clone());
                                ui.close();
                            }
                        }
                    }
                });
            *was_open = list.inner.is_some();
            if icon_button(ui, look, "common/delete.png", "Remove", picked.is_some())
                && let Some(i) = picked.take().filter(|&i| i < machine.posts.len())
            {
                machine.posts.remove(i);
            }
            if ui
                .button(cut_craft_i18n::t!("Post-Processor Management…"))
                .clicked()
            {
                asked = Some(MachineOutcome::ManagePosts);
            }
        });
        ui.weak(cut_craft_i18n::t!(
            "The default (bold) saves toolpaths; right-click a post for more"
        ));
    });
    asked
}

/// Apply on its own line (greyed out until something changes, with whether changes wait
/// beside it), then OK and Cancel.
pub(super) fn buttons(ui: &mut egui::Ui, can_keep: bool, dirty: bool) -> Option<MachineOutcome> {
    let mut outcome = None;
    // in a row of its own: a right-to-left layout on its own takes all the height left, and
    // the window grew down the screen with it
    ui.horizontal(|ui| {
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if cut_craft_bug_report::unapplied::apply_button(ui, can_keep && dirty) {
                outcome = Some(MachineOutcome::Apply);
            }
            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                cut_craft_bug_report::unapplied::note(ui, dirty);
            });
        });
    });
    ui.separator();
    ui.horizontal(|ui| {
        if ui
            .add_enabled(can_keep, egui::Button::new(cut_craft_i18n::t!("OK")))
            .clicked()
        {
            outcome = Some(MachineOutcome::Ok);
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button(cut_craft_i18n::t!("Cancel")).clicked() {
                outcome = Some(MachineOutcome::Cancel);
            }
        });
    });
    outcome
}

/// What Post-Processor Management asks of the app.
#[derive(Debug, PartialEq)]
pub enum PostOutcome {
    /// Pick a `.pp` file to install.
    Install,
    /// Make this post the active machine's default, used for the jobs it suits (flat, or
    /// rotary wrapping its side).
    Select(String),
    /// Show the custom posts' folder.
    OpenFolder(PathBuf),
    /// Something to say in the status bar.
    Message(String),
    Close,
}

/// The open Post-Processor Management window.
#[derive(Default)]
pub struct PostManager {
    pub(super) picked: Option<String>,
    /// A custom post asked to be deleted, waiting for the user to say so again.
    pub(super) deleting: Option<Entry>,
    pub(super) viewing: Option<(String, String)>,
    /// A custom post open in Edit.
    pub(super) editing: Option<Editing>,
    /// The filters over the list: each opening starts from the machine ([`Self::for_machine`]).
    pub(super) filter: Filter,
    /// The posts' capabilities, read when the window first draws.
    pub(super) capabilities: Option<CapabilityDb>,
    /// A post whose capabilities are being corrected (Edit Capabilities), and the correction.
    pub(super) correcting: Option<(Entry, Capabilities)>,
    /// The groups folded away, by their heading.
    pub(super) folded: std::collections::HashSet<&'static str>,
}

/// The posts shown, in their groups: the ones installed by you, then Cut Craft's own by
/// controller. Groups with nothing shown are left out.
pub(super) fn groups(
    shown: &[(Entry, Capabilities)],
) -> Vec<(&'static str, Vec<&(Entry, Capabilities)>)> {
    let mut out = vec![(
        "Installed by you",
        shown.iter().filter(|(e, _)| e.is_custom()).collect(),
    )];
    for family in Family::ALL {
        out.push((
            family.label(),
            shown
                .iter()
                .filter(|(e, caps)| !e.is_custom() && caps.family == family)
                .collect(),
        ));
    }
    out.retain(|(_, group): &(_, Vec<_>)| !group.is_empty());
    out
}

/// Edit Capabilities: a post's controller, units and moves as the filters see them, kept in
/// the capabilities file (the post itself is not changed).
pub(super) fn corrector(
    ctx: &egui::Context,
    correcting: &mut Option<(Entry, Capabilities)>,
    db: &mut CapabilityDb,
) -> Option<PostOutcome> {
    let (entry, caps) = correcting.as_mut()?;
    let mut open = true;
    let mut outcome = None;
    let mut done = false;
    let id = egui::Id::unique("post_processor_capabilities");
    let title = format!("Edit Capabilities: {}", entry.name);
    cut_craft_bug_report::dialog(ctx, id, &title, egui::vec2(460.0, 420.0), &mut open, |ui| {
        cut_craft_bug_report::mark(ui, "Edit Capabilities");
        ui.weak(cut_craft_i18n::t!(
            "How the filters in the post lists see this post. The post itself is not changed."
        ));
        egui::Grid::new("post_capabilities")
            .num_columns(2)
            .show(ui, |ui| {
                ui.label(cut_craft_i18n::t!("Controller"));
                egui::ComboBox::from_id_salt("post_capability_family")
                    .selected_text(caps.family.label())
                    .show_ui(ui, |ui| {
                        for f in Family::ALL {
                            ui.selectable_value(&mut caps.family, f, f.label());
                        }
                    });
                ui.end_row();
                ui.label(cut_craft_i18n::t!("Units"));
                ui.horizontal(|ui| {
                    ui.radio_value(&mut caps.inches, false, cut_craft_i18n::t!("mm"));
                    ui.radio_value(&mut caps.inches, true, cut_craft_i18n::t!("inch"));
                });
                ui.end_row();
                ui.label(cut_craft_i18n::t!("Moves"));
                ui.vertical(|ui| {
                    for m in Moves::ALL {
                        ui.radio_value(&mut caps.moves, m, m.label())
                            .on_hover_hint(m.hint());
                    }
                });
                ui.end_row();
                ui.label("");
                ui.add_enabled(
                    caps.moves == Moves::Rotary,
                    egui::Checkbox::new(
                        &mut caps.inverse_time,
                        cut_craft_i18n::t!("Inverse-time feeds (G93)"),
                    ),
                )
                .on_hover_hint(cut_craft_i18n::t!(
                    "Rotary feeds written as one over each move's time in minutes"
                ));
                ui.end_row();
            });
        ui.label(cut_craft_i18n::t!("What it is for (shown on hover)"));
        ui.add(
            egui::TextEdit::multiline(&mut caps.about)
                .desired_rows(3)
                .desired_width(f32::INFINITY),
        );
        ui.horizontal(|ui| {
            if ui.button(cut_craft_i18n::t!("Save")).clicked() {
                let kept = caps.clone();
                outcome = Some(PostOutcome::Message(match db.set(entry, Some(kept)) {
                    Ok(()) => format!("Capabilities kept for {}", entry.name),
                    Err(e) => e,
                }));
                done = true;
            }
            if ui
                .add_enabled(
                    db.corrected(entry),
                    egui::Button::new(cut_craft_i18n::t!("Read From the Post")),
                )
                .on_hover_hint(cut_craft_i18n::t!(
                    "Forget the correction and go by what the post's lines say"
                ))
                .on_disabled_hover_text(cut_craft_i18n::t!(
                    "Nothing corrected yet: this is what its lines say"
                ))
                .clicked()
            {
                outcome = Some(PostOutcome::Message(match db.set(entry, None) {
                    Ok(()) => format!("{} goes by its own lines again", entry.name),
                    Err(e) => e,
                }));
                done = true;
            }
            if ui.button(cut_craft_i18n::t!("Cancel")).clicked() {
                done = true;
            }
        });
    });
    if done || !open {
        *correcting = None;
    }
    outcome
}

/// What the post lists say when the filters leave nothing.
pub(super) const NONE_SHOWN: &str =
    "No post-processor matches these filters: tick more of them, or press Show All";

/// A custom post being edited: its file, the text as last saved, and as it is now.
#[derive(Clone, Debug, Default)]
pub(super) struct Editing {
    pub(super) name: String,
    pub(super) path: PathBuf,
    pub(super) saved: String,
    pub(super) text: String,
}

impl Editing {
    pub(super) fn of(entry: &Entry) -> Option<Self> {
        let Source::Custom(path) = &entry.source else {
            return None;
        };
        let text = Library::text(entry).ok()?;
        Some(Self {
            name: entry.name.clone(),
            path: path.clone(),
            saved: text.clone(),
            text,
        })
    }

    pub(super) fn save(&mut self) -> Result<(), String> {
        Library::save(&self.path, &self.text)?;
        self.saved.clone_from(&self.text);
        Ok(())
    }
}

/// Edit: a custom post's instructions, saved to its file, with what Cut Craft makes of them
/// (a post that does not read shows why, and is marked ⚠ in the list until it does).
pub(super) fn editor(ctx: &egui::Context, editing: &mut Option<Editing>) -> Option<PostOutcome> {
    let edit = editing.as_mut()?;
    let mut open = true;
    let mut outcome = None;
    let (id, title) = (
        egui::Id::unique("post_processor_edit"),
        format!("Edit Post-Processor: {}", edit.name),
    );
    cut_craft_bug_report::dialog_with_panels(ctx, id, &title, dialog_size(ctx), &mut open, |ui| {
        cut_craft_bug_report::mark(ui, "Edit Post-Processor");
        match cut_craft_post::Post::parse(&edit.text) {
            Ok(post) if post.name == edit.name => {
                ui.weak(cut_craft_i18n::t!("Reads as a post-processor"));
            }
            // machines know a post by its name, so a new one leaves them looking for the old
            Ok(post) => {
                ui.colored_label(
                    ui.visuals().warn_fg_color,
                    cut_craft_i18n::tr!(
                        "machines-post-renamed",
                        name = post.name.clone(),
                        old = edit.name.clone()
                    ),
                );
            }
            Err(why) => {
                ui.colored_label(
                    ui.visuals().error_fg_color,
                    cut_craft_i18n::tr!("machines-post-will-not-load", why = why.clone()),
                );
            }
        }
        let changed = edit.text != edit.saved;
        ui.horizontal(|ui| {
            if ui
                .add_enabled(changed, egui::Button::new(cut_craft_i18n::t!("Save")))
                .clicked()
            {
                outcome = Some(PostOutcome::Message(match edit.save() {
                    Ok(()) => format!("Saved {}", edit.name),
                    Err(e) => e,
                }));
            }
            if ui
                .add_enabled(changed, egui::Button::new(cut_craft_i18n::t!("Revert")))
                .clicked()
            {
                edit.text.clone_from(&edit.saved);
            }
            if changed {
                ui.weak(cut_craft_i18n::t!(
                    "Unsaved changes are lost when this closes"
                ));
            }
        });
        text_view(ui, &mut edit.text);
    });
    if !open {
        *editing = None;
    }
    outcome
}

impl PostManager {
    /// Post-Processor Management as it opens: its filters set for `machine` (its units, and
    /// rotary posts only when it has a 4th axis).
    pub fn for_machine(machine: Option<&Machine>) -> Self {
        Self {
            filter: Filter::starting(machine),
            ..Self::default()
        }
    }

    /// The list's filters, to change from outside (the help's pictures).
    pub fn filter_mut(&mut self) -> &mut Filter {
        &mut self.filter
    }

    /// Opens Edit Capabilities on the post named `name`, as its Inspect menu does.
    pub fn edit_capabilities(&mut self, library: &Library, name: &str) {
        let db = self
            .capabilities
            .get_or_insert_with(|| CapabilityDb::load(library));
        self.correcting = library.find(name).map(|e| {
            let caps = db.of(&e);
            (e, caps)
        });
    }

    pub fn show(
        &mut self,
        ctx: &egui::Context,
        library: &Library,
        look: &Look,
    ) -> Option<PostOutcome> {
        let mut open = true;
        let mut outcome = None;
        let (id, title) = (
            egui::Id::unique("post_processor_management"),
            "Post-Processor Management",
        );
        cut_craft_bug_report::dialog_with_panels(
            ctx,
            id,
            title,
            dialog_size(ctx),
            &mut open,
            |ui| {
                cut_craft_bug_report::mark(ui, "Post-Processor Management");
                outcome = self.contents(ui, library, look).1;
            },
        );
        viewer(ctx, &mut self.viewing);
        if let Some(said) = editor(ctx, &mut self.editing) {
            outcome = Some(said);
        }
        let db = self
            .capabilities
            .get_or_insert_with(|| CapabilityDb::load(library));
        if let Some(said) = corrector(ctx, &mut self.correcting, db) {
            outcome = Some(said);
        }
        if !open {
            outcome = Some(PostOutcome::Close);
        }
        outcome
    }

    /// The window's insides: the buttons (and a delete waiting to be said again) along the
    /// bottom, the post list filling the rest. Where the list shows, and what was asked.
    pub(super) fn contents(
        &mut self,
        ui: &mut egui::Ui,
        library: &Library,
        look: &Look,
    ) -> (egui::Rect, Option<PostOutcome>) {
        let mut outcome = None;
        egui::Panel::bottom("post_processor_buttons").show(ui, |ui| {
            if let Some(asked) = self.confirm_delete(ui) {
                outcome = Some(asked);
            }
            ui.horizontal(|ui| {
                if icon_button(ui, look, "common/open.png", "Install Post-Processor", true) {
                    outcome = Some(PostOutcome::Install);
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button(cut_craft_i18n::t!("Close")).clicked() {
                        outcome = Some(PostOutcome::Close);
                    }
                    let picked = self.picked.clone();
                    let select = ui
                        .add_enabled(picked.is_some(), egui::Button::new(cut_craft_i18n::t!("Select")))
                        .on_hover_hint(cut_craft_i18n::t!("Make it the post the active machine saves toolpaths through, for the jobs it suits: a flat post for flat jobs, a wrapping one for rotary jobs"),
                        );
                    if select.clicked()
                        && let Some(name) = picked
                    {
                        outcome = Some(PostOutcome::Select(name));
                    }
                });
            });
        });
        let db = self
            .capabilities
            .get_or_insert_with(|| CapabilityDb::load(library));
        let entries: Vec<(Entry, Capabilities)> = library
            .entries()
            .into_iter()
            .map(|e| {
                let caps = db.of(&e);
                (e, caps)
            })
            .collect();
        let total = entries.len();
        let shown: Vec<(Entry, Capabilities)> = entries
            .into_iter()
            .filter(|(e, caps)| self.filter.shows(&e.name, caps))
            .collect();
        egui::Panel::top("post_processor_filters").show(ui, |ui| {
            self.filter.ui(ui, shown.len(), total);
        });
        let list = egui::ScrollArea::vertical()
            .auto_shrink(false)
            .show(ui, |ui| {
                if shown.is_empty() {
                    ui.weak(NONE_SHOWN);
                }
                // one grid for every group, so the columns line up down the whole list
                egui::Grid::new("post_rows")
                    .num_columns(4)
                    .striped(true)
                    .show(ui, |ui| {
                        for (title, group) in groups(&shown) {
                            let folded = self.folded.contains(title);
                            let arrow = if folded { "▸" } else { "▾" };
                            let heading =
                                egui::RichText::new(format!("{arrow} {title} ({})", group.len()))
                                    .strong();
                            if ui
                                .selectable_label(false, heading)
                                .on_hover_hint(cut_craft_i18n::t!("Fold or unfold this group"))
                                .clicked()
                                && !self.folded.remove(title)
                            {
                                self.folded.insert(title);
                            }
                            ui.end_row();
                            if folded {
                                continue;
                            }
                            for (entry, caps) in group {
                                if let Some(asked) = self.row(ui, look, library, entry, caps) {
                                    outcome = Some(asked);
                                }
                            }
                        }
                    });
            })
            .inner_rect;
        (list, outcome)
    }

    /// One post, a row of the group's grid: a pencil for a custom one with its name, its
    /// version, what it writes, and its Inspect menu (also on a right-click).
    pub(super) fn row(
        &mut self,
        ui: &mut egui::Ui,
        look: &Look,
        library: &Library,
        entry: &Entry,
        caps: &Capabilities,
    ) -> Option<PostOutcome> {
        let mut asked = None;
        let label = ui
            .horizontal(|ui| {
                let edge = ui.spacing().interact_size.y;
                match (entry.is_custom(), (look.picture)("common/edit.png")) {
                    (true, Some(pencil)) => {
                        ui.add(pencil.fit_to_exact_size(egui::vec2(edge, edge)));
                    }
                    _ => {
                        ui.add_space(edge + ui.spacing().item_spacing.x);
                    }
                }
                let picked = self.picked.as_ref() == Some(&entry.name);
                let label = ui.selectable_label(picked, &entry.name);
                if let Some(why) = &entry.broken {
                    ui.colored_label(ui.visuals().error_fg_color, "⚠")
                        .on_hover_hint(why);
                }
                label
            })
            .inner;
        let label = if caps.about.is_empty() {
            label
        } else {
            label.on_hover_hint(&caps.about)
        };
        if label.clicked() {
            self.picked = Some(entry.name.clone());
        }
        match Library::text(entry).ok().and_then(|t| Library::version(&t)) {
            Some(version) => {
                ui.weak(version)
                    .on_hover_hint(cut_craft_i18n::t!("Version: the date of its last change"));
            }
            None => {
                ui.label("");
            }
        }
        ui.weak(caps.tags()).on_hover_hint(cut_craft_i18n::t!(
            "What it writes: units, moves, and G93 for inverse-time feeds"
        ));
        let button = ui.menu_button(cut_craft_i18n::t!("Inspect ▾"), |ui| {
            self.inspect(ui, library, entry, caps)
        });
        if let Some(Some(a)) = button.inner {
            asked = Some(a);
        }
        cut_craft_bug_report::context_menu(&label, |ui| {
            if let Some(a) = self.inspect(ui, library, entry, caps) {
                asked = Some(a);
            }
        });
        ui.end_row();
        asked
    }

    /// Inspect: View, Changelog and Customize (which opens the copy in Edit) for Cut Craft's posts;
    /// View, Changelog, Edit, Open File Location and Delete for custom ones.
    pub(super) fn inspect(
        &mut self,
        ui: &mut egui::Ui,
        library: &Library,
        entry: &Entry,
        caps: &Capabilities,
    ) -> Option<PostOutcome> {
        let mut asked = None;
        if ui.button(cut_craft_i18n::t!("View")).clicked() {
            self.viewing = view(entry);
            ui.close();
        }
        if ui.button(cut_craft_i18n::t!("Changelog")).clicked() {
            self.viewing = change_log(entry);
            ui.close();
        }
        if ui
            .button(cut_craft_i18n::t!("Edit Capabilities…"))
            .on_hover_hint(cut_craft_i18n::t!(
                "Correct its controller, units or moves, for the filters"
            ))
            .clicked()
        {
            self.correcting = Some((entry.clone(), caps.clone()));
            ui.close();
        }
        match &entry.source {
            Source::BuiltIn(_) => {
                if ui.button(cut_craft_i18n::t!("Customize")).clicked() {
                    asked = Some(match library.customize(entry) {
                        Ok(copy) => {
                            self.picked = Some(copy.name.clone());
                            self.editing = Editing::of(&copy);
                            PostOutcome::Message(format!("Customised as {}", copy.name))
                        }
                        Err(e) => PostOutcome::Message(e),
                    });
                    ui.close();
                }
            }
            Source::Custom(path) => {
                if ui.button(cut_craft_i18n::t!("Edit…")).clicked() {
                    self.editing = Editing::of(entry);
                    ui.close();
                }
                if ui
                    .button(cut_craft_i18n::t!("Open File Location…"))
                    .clicked()
                {
                    asked = path
                        .parent()
                        .map(|d| PostOutcome::OpenFolder(d.to_path_buf()));
                    ui.close();
                }
                if ui.button(cut_craft_i18n::t!("Delete…")).clicked() {
                    self.deleting = Some(entry.clone());
                    ui.close();
                }
            }
        }
        asked
    }

    /// Delete asks again, as it cannot be undone.
    pub(super) fn confirm_delete(&mut self, ui: &mut egui::Ui) -> Option<PostOutcome> {
        let entry = self.deleting.clone()?;
        let mut asked = None;
        ui.group(|ui| {
            ui.colored_label(
                ui.visuals().warn_fg_color,
                cut_craft_i18n::tr!("machines-delete-post", name = entry.name.clone()),
            );
            ui.horizontal(|ui| {
                if ui.button(cut_craft_i18n::t!("Delete")).clicked() {
                    self.deleting = None;
                    asked = Some(PostOutcome::Message(match Library::delete(&entry) {
                        Ok(()) => format!("Deleted {}", entry.name),
                        Err(e) => e,
                    }));
                }
                if ui.button(cut_craft_i18n::t!("Keep")).clicked() {
                    self.deleting = None;
                }
            });
        });
        asked
    }
}
