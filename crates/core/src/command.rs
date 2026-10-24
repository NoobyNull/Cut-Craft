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
    /// Icon path relative to the icon directory, e.g. `drawing/create_vectors/draw_circle.svg`.
    pub icon: &'static str,
    pub shortcut: &'static str,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CommandError {
    InvalidParams(String),
    NothingSelected,
    Failed(String),
}

impl fmt::Display for CommandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidParams(why) => write!(f, "invalid parameters: {why}"),
            Self::NothingSelected => f.write_str("nothing is selected"),
            Self::Failed(why) => f.write_str(why),
        }
    }
}

impl std::error::Error for CommandError {}

/// Uniform validation for lengths, radii and counts: fails unless `value > 0` (NaN fails).
pub fn ensure_positive(name: &str, value: f64) -> Result<(), CommandError> {
    if value > 0.0 {
        Ok(())
    } else {
        Err(CommandError::InvalidParams(format!(
            "{name} must be greater than 0"
        )))
    }
}

/// The layer a command names, or the error the user sees when it is gone.
pub fn existing_layer(doc: &Document, id: LayerId) -> Result<&Layer, CommandError> {
    doc.layer(id)
        .ok_or_else(|| CommandError::InvalidParams(format!("layer {} does not exist", id.0)))
}

/// The item a command names by id (a toolpath, a component...), or the error the user sees
/// when it is gone; `what` names the kind of item.
pub fn existing<'a, T>(
    items: &'a std::collections::BTreeMap<EntityId, T>,
    id: EntityId,
    what: &str,
) -> Result<&'a T, CommandError> {
    items
        .get(&id)
        .ok_or_else(|| CommandError::InvalidParams(format!("{what} {} does not exist", id.0)))
}

/// Names (of layers, toolpaths...) must have something in them; returns the trimmed name.
pub fn ensure_named(name: &str) -> Result<String, CommandError> {
    let name = name.trim();
    if name.is_empty() {
        Err(CommandError::InvalidParams("the name is empty".into()))
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

    fn run(doc: &Document, params: &Self::Params) -> Result<Change, CommandError>;
}

/// A pointer drag in job coordinates, with the modifier meanings Aspire uses.
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

/// Parameters for commands that take none.
#[derive(Clone, Copy, Debug, Default, serde::Serialize, serde::Deserialize, JsonSchema)]
pub struct NoParams {}

/// A command with its types erased, as the app and plugins see it.
pub trait DynCommand {
    fn id(&self) -> &'static str;
    fn meta(&self) -> Meta;
    fn schema(&self) -> Value;
    fn default_params(&self) -> Value;
    fn initial_params(&self, doc: &Document) -> Value;
    fn run_json(&self, doc: &Document, params: &Value) -> Result<Change, CommandError>;
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

    fn run_json(&self, doc: &Document, params: &Value) -> Result<Change, CommandError> {
        let params = C::Params::deserialize(params)
            .map_err(|e| CommandError::InvalidParams(e.to_string()))?;
        C::run(doc, &params)
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
        assert!(
            self.get(C::ID).is_none(),
            "command `{}` registered twice",
            C::ID
        );
        self.commands.push(Box::new(command));
        self
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
        let toolpath = doc
            .toolpaths
            .get(&id)
            .ok_or_else(|| CommandError::Failed(format!("toolpath {} does not exist", id.0)))?;
        let strategy = self.get(&toolpath.strategy).ok_or_else(|| {
            CommandError::Failed(format!(
                "{} was made by `{}`, which is not available",
                toolpath.name, toolpath.strategy
            ))
        })?;
        let mut sources = doc.clone();
        sources.selection = toolpath
            .vectors
            .iter()
            .copied()
            .filter(|v| doc.vectors.contains_key(v))
            .collect();
        if sources.selection.is_empty() {
            return Err(CommandError::Failed(format!(
                "the vectors {} was made from have been deleted",
                toolpath.name
            )));
        }
        let fresh = strategy
            .run_json(&sources, &toolpath.params)?
            .edits
            .into_iter()
            .find_map(|e| match e {
                Edit::AddToolpath { toolpath, .. } => Some(*toolpath),
                _ => None,
            })
            .ok_or_else(|| {
                CommandError::Failed(format!("`{}` did not make a toolpath", toolpath.strategy))
            })?;
        let mut c = Change::builder(doc);
        if &fresh != toolpath {
            c.replace_toolpath(id, fresh);
        }
        Ok(c.finish())
    }

    /// Every toolpath recalculated: one [`Registry::recalculate`] each, chained.
    pub fn recalculate_all(&self, doc: &Document) -> Result<Change, CommandError> {
        let ids: Vec<EntityId> = doc.toolpaths.keys().copied().collect();
        let each: Vec<_> = ids
            .iter()
            .map(|&id| move |d: &Document| self.recalculate(d, id))
            .collect();
        let steps: Vec<crate::Step<'_>> = each.iter().map(|f| f as crate::Step<'_>).collect();
        Change::chain(doc, &steps)
    }

    pub fn iter(&self) -> impl Iterator<Item = &dyn DynCommand> {
        self.commands.iter().map(AsRef::as_ref)
    }
}
