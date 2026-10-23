//! The contract every command implements, and the registry the app runs them through.

use crate::Change;
use crate::document::Document;
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

/// One operation: typed parameters in, an invertible [`Change`] out. It never mutates the
/// document itself, so undo, scripting, macros and tests all go through the same path.
pub trait Command: 'static {
    /// Stable identifier: the slug of the UI label (`"draw_circle"`).
    const ID: &'static str;
    /// Parameters; their JSON schema drives the generated settings form.
    type Params: Serialize + DeserializeOwned + JsonSchema + Default + Clone;

    fn meta() -> Meta;
    fn run(doc: &Document, params: &Self::Params) -> Result<Change, CommandError>;
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
    fn run_json(&self, doc: &Document, params: &Value) -> Result<Change, CommandError>;
}

struct Erased<C>(PhantomData<fn() -> C>);

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

    fn run_json(&self, doc: &Document, params: &Value) -> Result<Change, CommandError> {
        let params = C::Params::deserialize(params)
            .map_err(|e| CommandError::InvalidParams(e.to_string()))?;
        C::run(doc, &params)
    }
}

/// Every command the app knows, looked up by id.
#[derive(Default)]
pub struct Registry {
    commands: Vec<Box<dyn DynCommand>>,
}

impl Registry {
    pub fn add<C: Command>(&mut self) -> &mut Self {
        assert!(
            self.get(C::ID).is_none(),
            "command `{}` registered twice",
            C::ID
        );
        self.commands.push(Box::new(Erased::<C>(PhantomData)));
        self
    }

    pub fn get(&self, id: &str) -> Option<&dyn DynCommand> {
        self.commands
            .iter()
            .find(|c| c.id() == id)
            .map(AsRef::as_ref)
    }

    pub fn iter(&self) -> impl Iterator<Item = &dyn DynCommand> {
        self.commands.iter().map(AsRef::as_ref)
    }
}
