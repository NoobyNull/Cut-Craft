//! The contract every command implements, and the registry the app runs them through.

use crate::document::{Document, EntityId, Layer, LayerId};
use crate::formats::{ExportEntry, Exporter, ImportEntry, Importer};
use crate::geometry::{Point, Rect};
use crate::{Change, Edit};
use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use serde::{Deserialize as _, Serialize};
use serde_json::Value;
use std::fmt;
use std::marker::PhantomData;

/// How a command presents itself in the UI.
#[derive(Clone, Copy, Debug)]
pub struct Meta {
    pub label: &'static str,
    /// Drawing-tab group or menu it belongs to.
    pub group: &'static str,
    /// Icon path relative to the icon directory, e.g. `drawing/create_vectors/draw_circle.png`.
    pub icon: &'static str,
    pub shortcut: &'static str,
    /// How its form behaves: what its button says, and whether what it made follows it.
    pub form: Form,
}

/// How a command's form behaves. The command says it, so the app needs no list of names to
/// know which button a form shows; an add-on's command says it the same way.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Form {
    /// Changes what is already there (the selection, a layer, the job): **Apply**.
    Apply,
    /// Makes something new from its settings, another with each press: **Create**.
    Create,
    /// Works out a toolpath from the selected vectors: **Calculate**. A toolpath never cuts
    /// along a construction line, however it was picked. `needs`: what the machine must have
    /// for it to be offered.
    Toolpath { needs: Needs },
    /// Asks an add-on to make something new for the Library: **Generate**.
    Generate,
    /// Used only on the canvas (a stroke, a click, a drag): the form holds settings for the
    /// next one and has nothing to press.
    Gesture,
    /// Makes its piece or model and then follows its settings as they change, with nothing to
    /// press. `create_first`: it waits for **Create** to make it once (it needs something
    /// picked first, such as a photo with its depth).
    Live { create_first: bool },
    /// Makes many pieces once with **Create**, then remakes them as its settings change.
    /// `keeps_apply`: it still offers **Apply** once made, for changes to be seen to go on
    /// the one there.
    Remake { keeps_apply: bool },
}

impl Form {
    /// A toolpath strategy's form, needing nothing of the machine past a spindle.
    pub const TOOLPATH: Self = Self::Toolpath {
        needs: Needs::Nothing,
    };

    /// Whether it works out a toolpath ([`Self::Toolpath`]).
    pub fn is_toolpath(self) -> bool {
        matches!(self, Self::Toolpath { .. })
    }

    /// What the machine must have for the command to be offered.
    pub fn needs(self) -> Needs {
        match self {
            Self::Toolpath { needs } => needs,
            _ => Needs::Nothing,
        }
    }
}

/// What a toolpath needs the machine to have, past a spindle. Without it, the toolpath is
/// not offered (as the tool changer's settings are not without one); one already in a job
/// stays, with a red warning.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Needs {
    Nothing,
    /// A laser on the head (Machine Configuration's Laser fitted).
    Laser,
    /// A 4th axis turning the material (Machine Configuration's 4th axis).
    Rotary,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CommandError {
    InvalidParams(String),
    NothingSelected,
    Failed(String),
}

/// What a tool that works on a selection asks for when nothing is selected.
pub const SELECT_FIRST: &str =
    "Select what to work on: click a vector in the 2D view, Shift-click to add more";

/// What a tool that reads a 3D model file says when the file holds nothing to read.
pub const EMPTY_MODEL: &str =
    "This file has no triangles in it. Export it again from the program that made it.";

/// Every message reads as a sentence, whatever case it was written in: a capital first and a
/// full stop last. A message should also say what to do about it ("Pick a smaller bit"),
/// not only what is wrong; the app's tests run every command to keep that so.
impl fmt::Display for CommandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidParams(why) | Self::Failed(why) => f.write_str(&sentence(why)),
            Self::NothingSelected => f.write_str(&sentence(SELECT_FIRST)),
        }
    }
}

impl std::error::Error for CommandError {}

/// `text` as a sentence: trimmed, its first letter upper case, and a full stop unless it
/// already ends with one (or `!`, `?`).
pub fn sentence(text: &str) -> String {
    let text = text.trim();
    let mut chars = text.chars();
    let mut out: String = match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => return String::new(),
    };
    if !out.ends_with(['.', '!', '?']) {
        out.push('.');
    }
    out
}

/// Uniform validation for lengths, radii and counts: fails unless `value > 0` (NaN fails).
/// `name` is the setting as its form names it ("pass depth").
pub fn ensure_positive(name: &str, value: f64) -> Result<(), CommandError> {
    if value > 0.0 {
        Ok(())
    } else {
        Err(CommandError::InvalidParams(format!(
            "set {name} to more than 0"
        )))
    }
}

/// The one message for a tool too big to cut inside the selected shapes, used by every
/// toolpath that cuts inside: it names the tool and says what to do. `instead` is the other
/// way out where there is one ("cut Outside instead"); without it, making the shapes bigger.
pub fn tool_does_not_fit(tool: &crate::toolpath::Tool, instead: Option<&str>) -> CommandError {
    let name = tool.name.trim();
    let tool = if name.is_empty() {
        format!("{} mm bit", tool.diameter)
    } else {
        name.to_owned()
    };
    let instead = instead.unwrap_or("make the shapes bigger");
    CommandError::Failed(format!(
        "The {tool} does not fit inside the selected shapes. Pick a smaller bit, or {instead}."
    ))
}

/// The message for an item a command names that is no longer in the job (deleted, or undone):
/// `what` is its kind ("toolpath").
pub fn gone(what: &str) -> CommandError {
    CommandError::InvalidParams(format!(
        "That {what} is not in the job any more. Pick another {what}."
    ))
}

/// The layer a command names, or the error the user sees when it is gone.
pub fn existing_layer(doc: &Document, id: LayerId) -> Result<&Layer, CommandError> {
    doc.layer(id).ok_or_else(|| gone("layer"))
}

/// The item a command names by id (a toolpath, a component...), or the error the user sees
/// when it is gone; `what` names the kind of item.
pub fn existing<'a, T>(
    items: &'a std::collections::BTreeMap<EntityId, T>,
    id: EntityId,
    what: &str,
) -> Result<&'a T, CommandError> {
    items.get(&id).ok_or_else(|| gone(what))
}

/// Names (of layers, toolpaths...) must have something in them; returns the trimmed name.
pub fn ensure_named(name: &str) -> Result<String, CommandError> {
    let name = name.trim();
    if name.is_empty() {
        Err(CommandError::InvalidParams("Type a name.".into()))
    } else {
        Ok(name.to_owned())
    }
}

/// One operation: typed parameters in, an invertible [`Change`] out. It never mutates the
/// document itself, so undo, scripting, macros and tests all go through the same path.
pub trait Command: 'static {
    /// Stable identifier: the slug of the UI label (`"draw_circle"`).
    const ID: &'static str;
    /// Parameters; their JSON schema drives the generated settings form.
    type Params: Serialize + DeserializeOwned + JsonSchema + Default + Clone;

    fn meta() -> Meta;

    /// What the settings form starts with: by default `Params::default()`; commands that edit
    /// something existing (job setup, set size) start from the document instead.
    fn initial_params(_doc: &Document) -> Self::Params {
        Self::Params::default()
    }

    /// The settings that draw `path` again, for a command that draws shapes like it (so its
    /// form can edit a shape drawn earlier); `current` gives what the shape does not say.
    fn params_from(_path: &crate::geometry::Path, _current: &Self::Params) -> Option<Self::Params> {
        None
    }

    /// Whether the form, opened now, works on something it made earlier (the selection is
    /// one it made), so pressing it changes that one rather than making another.
    fn reopens(_doc: &Document) -> bool {
        false
    }

    fn run(doc: &Document, params: &Self::Params) -> Result<Change, CommandError>;
}

/// A pointer drag in job coordinates, with the usual modifier meanings.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Drag {
    pub start: Point,
    pub end: Point,
    /// Alt held: the start point is the centre.
    pub from_center: bool,
    /// Ctrl held: keep it square.
    pub constrain: bool,
}

impl Drag {
    /// The box the drag describes, honouring both modifiers.
    pub fn rect(&self) -> Rect {
        let mut d = self.end - self.start;
        if self.constrain {
            let side = d.x.abs().max(d.y.abs());
            d = Point::new(side.copysign(d.x), side.copysign(d.y));
        }
        if self.from_center {
            Rect::new(self.start - d, self.start + d)
        } else {
            Rect::new(self.start, self.start + d)
        }
    }
}

/// A command that can also be drawn with the mouse. Lives in the command's own crate, so
/// changing how a tool drags means editing only that crate.
pub trait Interactive: Command {
    /// Parameters for `drag`; anything the drag does not decide comes from `current`.
    fn from_drag(drag: Drag, current: &Self::Params) -> Self::Params;
}

// Parameters for commands that take none. A plain comment: a doc comment would be every
// such command's help page intro.
#[derive(Clone, Copy, Debug, Default, serde::Serialize, serde::Deserialize, JsonSchema)]
pub struct NoParams {}

/// Whether a command with `meta` makes toolpaths, and so never sees construction lines.
pub fn makes_toolpaths(meta: &Meta) -> bool {
    meta.form.is_toolpath()
}

/// What a toolpath says when only construction lines were selected for it: they are left
/// out, so it found nothing to cut. Any other error is as it was.
fn only_construction(doc: &Document, error: CommandError) -> CommandError {
    let picked = !doc.selection.is_empty();
    if picked && doc.selection.iter().all(|id| doc.is_construction(*id)) {
        CommandError::Failed(CONSTRUCTION_ONLY.into())
    } else {
        error
    }
}

/// The message for a toolpath given only construction lines.
pub const CONSTRUCTION_ONLY: &str = "Only construction lines are selected, and toolpaths never \
     cut along those. Select real vectors, or right-click a construction line and pick Make \
     Real Line to cut along it";

/// A command with its types erased, as the app and plugins see it.
pub trait DynCommand: Send + Sync {
    fn id(&self) -> &'static str;
    fn meta(&self) -> Meta;
    fn schema(&self) -> Value;
    fn default_params(&self) -> Value;
    fn initial_params(&self, doc: &Document) -> Value;
    /// [`Command::params_from`], as JSON; `current` is the form's settings.
    fn params_from(&self, path: &crate::geometry::Path, current: &Value) -> Option<Value>;
    fn run_json(&self, doc: &Document, params: &Value) -> Result<Change, CommandError>;
    /// [`Command::reopens`].
    fn reopens(&self, _doc: &Document) -> bool {
        false
    }
    /// Whether the command can be drawn with the mouse.
    fn interactive(&self) -> bool;
    /// The params a drag describes; an error for commands that are not interactive.
    fn drag_params(&self, drag: Drag, current: &Value) -> Result<Value, CommandError>;
}

type DragFn = fn(Drag, &Value) -> Result<Value, CommandError>;

struct Erased<C> {
    drag: Option<DragFn>,
    command: PhantomData<fn() -> C>,
}

fn drag_json<C: Interactive>(drag: Drag, current: &Value) -> Result<Value, CommandError> {
    let current =
        C::Params::deserialize(current).map_err(|e| CommandError::InvalidParams(e.to_string()))?;
    Ok(serde_json::to_value(C::from_drag(drag, &current)).expect("command params must serialize"))
}

impl<C: Command> DynCommand for Erased<C> {
    fn id(&self) -> &'static str {
        C::ID
    }

    fn meta(&self) -> Meta {
        C::meta()
    }

    fn schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(C::Params))
            .expect("a JSON schema always serializes")
    }

    fn default_params(&self) -> Value {
        serde_json::to_value(C::Params::default()).expect("command params must serialize")
    }

    fn initial_params(&self, doc: &Document) -> Value {
        serde_json::to_value(C::initial_params(doc)).expect("command params must serialize")
    }

    fn params_from(&self, path: &crate::geometry::Path, current: &Value) -> Option<Value> {
        let current = C::Params::deserialize(current).unwrap_or_default();
        let p = C::params_from(path, &current)?;
        Some(serde_json::to_value(p).expect("command params must serialize"))
    }

    fn run_json(&self, doc: &Document, params: &Value) -> Result<Change, CommandError> {
        let params = C::Params::deserialize(params)
            .map_err(|e| CommandError::InvalidParams(format!("Check the tool's settings: {e}")))?;
        if makes_toolpaths(&C::meta()) {
            // a toolpath never cuts along a construction line, however it was picked
            return C::run(&doc.for_cutting(), &params).map_err(|e| only_construction(doc, e));
        }
        C::run(doc, &params)
    }

    fn reopens(&self, doc: &Document) -> bool {
        C::reopens(doc)
    }

    fn interactive(&self) -> bool {
        self.drag.is_some()
    }

    fn drag_params(&self, drag: Drag, current: &Value) -> Result<Value, CommandError> {
        let to_params = self.drag.ok_or_else(|| {
            CommandError::Failed(format!("{} cannot be drawn with the mouse", C::ID))
        })?;
        to_params(drag, current)
    }
}

/// Every command the app knows, looked up by id.
#[derive(Default)]
pub struct Registry {
    commands: Vec<Box<dyn DynCommand>>,
    exporters: Vec<ExportEntry>,
    importers: Vec<ImportEntry>,
}

impl Registry {
    pub fn add<C: Command>(&mut self) -> &mut Self {
        self.insert(Erased::<C> {
            drag: None,
            command: PhantomData,
        })
    }

    /// Registers a command that can also be drawn with the mouse.
    pub fn add_interactive<C: Interactive>(&mut self) -> &mut Self {
        self.insert(Erased::<C> {
            drag: Some(drag_json::<C>),
            command: PhantomData,
        })
    }

    fn insert<C: Command>(&mut self, command: Erased<C>) -> &mut Self {
        self.add_dyn(Box::new(command))
    }

    /// Registers a command made at run time (a Lua gadget) rather than compiled in.
    pub fn add_dyn(&mut self, command: Box<dyn DynCommand>) -> &mut Self {
        assert!(
            self.get(command.id()).is_none(),
            "command `{}` registered twice",
            command.id()
        );
        self.commands.push(command);
        self
    }

    /// The drawing command that draws `path`, with the settings that draw it again (E, and
    /// Shift-click in a drawing form).
    pub fn drawn_by(
        &self,
        doc: &Document,
        path: &crate::geometry::Path,
    ) -> Option<(&'static str, Value)> {
        self.commands.iter().find_map(|c| {
            let p = c.params_from(path, &c.initial_params(doc))?;
            Some((c.id(), p))
        })
    }

    /// Every command, in the order they were added.
    pub fn commands(&self) -> impl Iterator<Item = &dyn DynCommand> {
        self.commands.iter().map(AsRef::as_ref)
    }

    pub fn get(&self, id: &str) -> Option<&dyn DynCommand> {
        self.commands
            .iter()
            .find(|c| c.id() == id)
            .map(AsRef::as_ref)
    }

    pub fn add_exporter<E: Exporter>(&mut self) -> &mut Self {
        assert!(
            self.exporter(E::ID).is_none(),
            "exporter `{}` registered twice",
            E::ID
        );
        self.exporters.push(ExportEntry {
            id: E::ID,
            format: E::FORMAT,
            export: E::export,
        });
        self
    }

    pub fn add_importer<I: Importer>(&mut self) -> &mut Self {
        assert!(
            self.importers.iter().all(|i| i.id != I::ID),
            "importer `{}` registered twice",
            I::ID
        );
        self.importers.push(ImportEntry {
            id: I::ID,
            format: I::FORMAT,
            import: I::import,
        });
        self
    }

    pub fn exporter(&self, id: &str) -> Option<&ExportEntry> {
        self.exporters.iter().find(|e| e.id == id)
    }

    pub fn importer(&self, id: &str) -> Option<&ImportEntry> {
        self.importers.iter().find(|i| i.id == id)
    }

    pub fn exporters(&self) -> impl Iterator<Item = &ExportEntry> {
        self.exporters.iter()
    }

    pub fn importers(&self) -> impl Iterator<Item = &ImportEntry> {
        self.importers.iter()
    }

    /// The importer for a file extension (without the dot), case-insensitively.
    pub fn importer_for(&self, extension: &str) -> Option<&ImportEntry> {
        self.importers.iter().find(|i| i.format.matches(extension))
    }

    /// Re-runs the strategy that made toolpath `id`, with its saved params, on the vectors it was
    /// made from (whatever is selected now). Unchanged results make an empty change.
    pub fn recalculate(&self, doc: &Document, id: EntityId) -> Result<Change, CommandError> {
        self.edit(doc, id, None)
    }

    /// Edit Toolpath: toolpath `id` made again from its vectors with `params` (its own when
    /// `None`), in its place.
    pub fn edit(
        &self,
        doc: &Document,
        id: EntityId,
        params: Option<&serde_json::Value>,
    ) -> Result<Change, CommandError> {
        let toolpath = doc.toolpaths.get(&id).ok_or_else(|| gone("toolpath"))?;
        let strategy = self.get(&toolpath.strategy).ok_or_else(|| {
            CommandError::Failed(format!(
                "{} was made by `{}`, which is not available. Install the add-on that made it, \
                 or delete this toolpath.",
                toolpath.name, toolpath.strategy
            ))
        })?;
        let mut sources = doc.clone();
        sources.remaking = Some(id);
        let settings = params.unwrap_or(&toolpath.params);
        // a toolpath with a Vector Selector takes whatever vectors pass it now
        if let Some(selector) = crate::selector::Selector::of(settings) {
            sources.selection = selector.picked(doc).into_iter().collect();
            if sources.selection.is_empty() {
                return Err(CommandError::Failed(format!(
                    "no vectors pass {}'s Vector Selector. Edit its filters",
                    toolpath.name
                )));
            }
        } else {
            sources.selection = toolpath
                .vectors
                .iter()
                .copied()
                .filter(|v| doc.vectors.contains_key(v))
                .collect();
            // made from no vectors, now set to work inside Selected Vector(s) (a 3D
            // toolpath's boundary switched in Edit): the vectors selected now
            if toolpath.vectors.is_empty()
                && settings.get("machining_boundary").and_then(|b| b.as_str())
                    == Some("selected_vectors")
            {
                sources.selection.clone_from(&doc.selection);
            }
            // made since into construction lines: never cut
            let kept = sources.selection.len();
            sources.selection = (sources.selection.iter().copied())
                .filter(|v| !doc.is_construction(*v))
                .collect();
            if kept > 0 && sources.selection.is_empty() {
                return Err(CommandError::Failed(format!(
                    "Every vector {0} was made from is a construction line now, and toolpaths \
                     never cut along those. Right-click them and pick Make Real Line, or select \
                     other vectors, then edit {0} and press Calculate.",
                    toolpath.name
                )));
            }
            // a toolpath made from no vectors (3D, rotary, picture) needs none to recalculate
            if sources.selection.is_empty() && !toolpath.vectors.is_empty() {
                return Err(CommandError::Failed(format!(
                    "The vectors {0} was made from have been deleted. Select new vectors, then \
                     edit {0} and press Calculate.",
                    toolpath.name
                )));
            }
        }
        let fresh = strategy
            .run_json(&sources, params.unwrap_or(&toolpath.params))?
            .edits
            .into_iter()
            .find_map(|e| match e {
                Edit::AddToolpath { toolpath, .. } => Some(*toolpath),
                _ => None,
            })
            .ok_or_else(|| {
                CommandError::Failed(format!(
                    "{} could not be made again: what it was made from is no longer there. \
                     Delete it and make it again",
                    toolpath.name
                ))
            })?;
        let mut fresh = fresh;
        crate::rest::apply(&doc.job, &mut fresh, params.unwrap_or(&toolpath.params))?;
        if let Some(selector) = settings.get("selector") {
            fresh.params["selector"] = selector.clone();
        }
        // its preview colour and notes are the toolpath's own, not its settings
        for key in crate::toolpath::OWN_PARAMS {
            if let Some(kept) = toolpath.params.get(key) {
                fresh.params[key] = kept.clone();
            }
        }
        let mut c = Change::builder(doc);
        if &fresh != toolpath {
            c.replace_toolpath(id, fresh);
        }
        Ok(c.finish())
    }

    /// Every toolpath recalculated: one [`Registry::recalculate`] each, chained.
    pub fn recalculate_all(&self, doc: &Document) -> Result<Change, CommandError> {
        let ids: Vec<EntityId> = doc.toolpaths.keys().copied().collect();
        self.recalculate_each(doc, &ids)
    }

    /// Toolpaths `ids` recalculated in turn, chained, each a share of the progress bar
    /// ([`crate::progress`]) named as it goes; stops early once asked to. A toolpath that
    /// copies others (Merged, Array Copy) goes after its sources, and a source that is out
    /// of date is recalculated with it even when not named (sources are hidden), so the
    /// copy takes their new moves.
    pub fn recalculate_each(
        &self,
        doc: &Document,
        ids: &[EntityId],
    ) -> Result<Change, CommandError> {
        let ids = sources_first(doc, ids);
        let steps: Vec<(usize, EntityId)> = ids.iter().copied().enumerate().collect();
        // each toolpath's share of the bar: how long its kind took last time
        let kinds: Vec<Option<String>> = ids
            .iter()
            .map(|id| (doc.toolpaths.get(id)).map(|t| crate::eta::calculating(&t.strategy)))
            .collect();
        let weights = crate::eta::weights(&kinds);
        Change::chain_over(doc, &steps, |d, &(i, id)| {
            crate::progress::stop_if_cancelled()?;
            if let Some(t) = d.toolpaths.get(&id) {
                crate::progress::stage(&format!("{} ({} of {})", t.name, i + 1, steps.len()));
            }
            let started = std::time::Instant::now();
            let made = crate::progress::part_weighted(i, &weights, || self.recalculate(d, id));
            if made.is_ok()
                && !crate::progress::cancelled()
                && let Some(kind) = &kinds[i]
            {
                crate::eta::record(kind, started.elapsed().as_secs_f64());
            }
            made
        })
    }

    pub fn iter(&self) -> impl Iterator<Item = &dyn DynCommand> {
        self.commands.iter().map(AsRef::as_ref)
    }
}

/// `ids` in order, each copy (Merged, Array Copy) moved after its sources and preceded by
/// those of them that are out of date and not named.
fn sources_first(doc: &Document, ids: &[EntityId]) -> Vec<EntityId> {
    fn visit(
        doc: &Document,
        id: EntityId,
        named: &std::collections::BTreeSet<EntityId>,
        seen: &mut std::collections::BTreeSet<EntityId>,
        out: &mut Vec<EntityId>,
    ) {
        if !seen.insert(id) {
            return;
        }
        if let Some(t) = doc.toolpaths.get(&id) {
            for s in crate::change::sources_of(t) {
                if named.contains(&s) || doc.toolpaths.get(&s).is_some_and(|s| s.stale) {
                    visit(doc, s, named, seen, out);
                }
            }
        }
        out.push(id);
    }
    let named: std::collections::BTreeSet<EntityId> = ids.iter().copied().collect();
    let mut seen = std::collections::BTreeSet::new();
    let mut out = Vec::with_capacity(ids.len());
    for &id in ids {
        visit(doc, id, &named, &mut seen, &mut out);
    }
    out
}
