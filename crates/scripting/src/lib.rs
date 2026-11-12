//! Lua gadgets: small Lua 5.4 scripts that drive CutCraft's own commands.
//!
//! A gadget defines `function run(settings)` and calls `cutcraft.run(id, params)` for each
//! command it wants, exactly as the UI would run it. Everything it does comes back as one
//! [`Change`], so a whole gadget run is one Undo step, and a gadget that fails part way
//! changes nothing.
//!
//! The sandbox: only Lua's `string`, `table`, `math` and `utf8` libraries and the `cutcraft`
//! table are there. No files, programs, network, other Lua code or compiled Lua. A gadget that
//! runs too long or uses too much memory is stopped.

use cut_craft_core::document::Document;
use cut_craft_core::{Change, Registry};
use mlua::chunk::ChunkMode;
use mlua::{HookTriggers, Lua, LuaOptions, LuaSerdeExt, StdLib, VmState};
use serde_json::Value;
use std::cell::{Cell, RefCell};
use std::fmt;
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

mod gadget;
pub use gadget::{GadgetCommand, load_gadget};

/// How much a gadget may do before it is stopped.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    /// Lua instructions, counted in steps of [`HOOK_EVERY`]. About a second's work by default.
    pub instructions: u64,
    /// Bytes of Lua memory.
    pub memory: usize,
    /// Commands one run may run. Commands work outside Lua's memory, so this bounds what a
    /// loop of them can build up.
    pub commands: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            instructions: 200_000_000,
            memory: 64 * 1024 * 1024,
            commands: 10_000,
        }
    }
}

/// The ranges Options offers for each limit; values outside are kept to them.
pub const INSTRUCTIONS_RANGE: std::ops::RangeInclusive<u64> = 10_000_000..=5_000_000_000;
pub const MEMORY_RANGE: std::ops::RangeInclusive<usize> = (8 << 20)..=(1024 << 20);
pub const COMMANDS_RANGE: std::ops::RangeInclusive<usize> = 100..=1_000_000;

/// `limits` with each one kept to the range Options offers.
pub fn kept_to_ranges(limits: Limits) -> Limits {
    let keep =
        |v, r: &std::ops::RangeInclusive<usize>| std::cmp::Ord::clamp(v, *r.start(), *r.end());
    Limits {
        instructions: limits
            .instructions
            .clamp(*INSTRUCTIONS_RANGE.start(), *INSTRUCTIONS_RANGE.end()),
        memory: keep(limits.memory, &MEMORY_RANGE),
        commands: keep(limits.commands, &COMMANDS_RANGE),
    }
}

/// What Options ▸ Advanced sets for gadgets, read whenever one runs; the app sets it when
/// Options loads or changes.
static INSTRUCTIONS: AtomicU64 = AtomicU64::new(200_000_000);
static MEMORY: AtomicUsize = AtomicUsize::new(64 * 1024 * 1024);
static COMMANDS: AtomicUsize = AtomicUsize::new(10_000);

/// Sets the limits gadgets run with, each kept to its range.
pub fn set_gadget_limits(limits: Limits) {
    let limits = kept_to_ranges(limits);
    INSTRUCTIONS.store(limits.instructions, Ordering::Relaxed);
    MEMORY.store(limits.memory, Ordering::Relaxed);
    COMMANDS.store(limits.commands, Ordering::Relaxed);
}

/// The limits gadgets run with.
pub fn gadget_limits() -> Limits {
    Limits {
        instructions: INSTRUCTIONS.load(Ordering::Relaxed),
        memory: MEMORY.load(Ordering::Relaxed),
        commands: COMMANDS.load(Ordering::Relaxed),
    }
}

/// How often, in Lua instructions, the run checks its instruction budget.
const HOOK_EVERY: u32 = 10_000;

/// Why a gadget did not finish. Each one is worded for the person who ran the gadget.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScriptError {
    /// The gadget's text is not Lua CutCraft can run (a typo, or compiled Lua).
    Syntax(String),
    /// The gadget stopped with an error, or a command it ran failed.
    Failed(String),
    /// It ran past its instruction budget: most likely a loop that never ends.
    TooLong,
    /// It used more memory than gadgets may.
    OutOfMemory,
    /// It ran more commands than one run may (the limit is given).
    TooManyCommands(usize),
}

impl fmt::Display for ScriptError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Syntax(why) => write!(f, "the gadget has a mistake in it: {why}"),
            Self::Failed(why) => write!(f, "the gadget stopped: {why}"),
            Self::TooLong => f.write_str(
                "the gadget took too long and was stopped (it may be stuck in a loop); \
                 nothing in the job was changed",
            ),
            Self::OutOfMemory => f.write_str(
                "the gadget used too much memory and was stopped; nothing in the job was changed",
            ),
            Self::TooManyCommands(limit) => write!(
                f,
                "the gadget tried to do more than {limit} things in one go and was stopped; \
                 nothing in the job was changed"
            ),
        }
    }
}

impl std::error::Error for ScriptError {}

/// Lua wrapper so a failed command is reported at the gadget's own line (`error(.., 2)`).
const PRELUDE: &str = r##"
local run_command = ...
cutcraft.run = function(id, params)
  local ok, why = run_command(id, params or {})
  if not ok then error(why, 2) end
end
print = function(...)
  local parts = {}
  for i = 1, select("#", ...) do parts[i] = tostring((select(i, ...))) end
  cutcraft.message(table.concat(parts, "\t"))
end
"##;

/// Runs the gadget `source` (named `name` in error messages) on `doc` with the user's
/// `settings`, and returns everything it did as one change. The document is not touched.
pub fn run_gadget(
    name: &str,
    source: &str,
    doc: &Document,
    registry: &Registry,
    settings: &Value,
    limits: Limits,
) -> Result<Change, ScriptError> {
    let lua = sandbox(limits)?;
    let stopped = Rc::new(Cell::new(false));
    budget(&lua, limits.instructions, Rc::clone(&stopped))?;

    let ran = Cell::new(0_usize);
    let too_many = Cell::new(false);
    let scratch = RefCell::new(doc.clone());
    let total = RefCell::new(Change::default());

    let result = lua.scope(|scope| {
        let cutcraft = lua.create_table()?;
        cutcraft.set(
            "job",
            scope.create_function(|lua, ()| lua.to_value(&job_info(&scratch.borrow().job)))?,
        )?;
        cutcraft.set(
            "selection",
            scope.create_function(|_, ()| {
                Ok(scratch
                    .borrow()
                    .selection
                    .iter()
                    .map(|id| id.0)
                    .collect::<Vec<_>>())
            })?,
        )?;
        cutcraft.set(
            "vector_count",
            scope.create_function(|_, ()| Ok(scratch.borrow().vectors.len()))?,
        )?;
        cutcraft.set(
            "message",
            scope.create_function(|_, text: String| {
                total.borrow_mut().notes.push(text);
                Ok(())
            })?,
        )?;
        lua.globals().set("cutcraft", cutcraft)?;

        let run_command = scope.create_function(|lua, (id, params): (String, mlua::Value)| {
            if ran.get() >= limits.commands {
                too_many.set(true);
                return Err(mlua::Error::runtime("too many commands"));
            }
            ran.set(ran.get() + 1);
            let params = json_params(lua.from_value(params)?);
            Ok(match run_one(registry, &scratch, &id, &params) {
                Ok(change) => {
                    let mut t = total.borrow_mut();
                    *t = std::mem::take(&mut *t).then(change);
                    (true, None)
                }
                Err(why) => (false, Some(why)),
            })
        })?;
        lua.load(PRELUDE)
            .set_name("=cutcraft")
            .call::<()>(run_command)?;

        lua.load(source)
            .set_name(format!("={name}"))
            .set_mode(ChunkMode::Text)
            .exec()?;
        let run: Option<mlua::Function> = lua.globals().get("run")?;
        let Some(run) = run else {
            return Err(mlua::Error::runtime(
                "it has no `function run(settings)`, so there is nothing to do",
            ));
        };
        run.call::<()>(lua.to_value(settings)?)
    });

    match result {
        Ok(()) => Ok(total.into_inner()),
        Err(_) if stopped.get() => Err(ScriptError::TooLong),
        Err(_) if too_many.get() => Err(ScriptError::TooManyCommands(limits.commands)),
        Err(e) => Err(script_error(&e)),
    }
}

/// The job as a gadget reads it: its settings, plus where the material is (`area`, with
/// `min` and `max` corners) and its middle (`centre`), in job coordinates.
fn job_info(job: &cut_craft_core::document::Job) -> Value {
    let mut info = serde_json::to_value(job).unwrap_or_default();
    let area = job.area();
    let point = |x: f64, y: f64| serde_json::json!({ "x": x, "y": y });
    info["area"] = serde_json::json!({
        "min": point(area.min.x, area.min.y),
        "max": point(area.max.x, area.max.y),
    });
    info["centre"] = point(
        f64::midpoint(area.min.x, area.max.x),
        f64::midpoint(area.min.y, area.max.y),
    );
    info
}

/// What a gadget file says about itself at its top level: its `gadget` table (name,
/// description) and its `settings` list, as JSON (`null` where it has none).
pub(crate) fn describe(name: &str, source: &str) -> Result<(Value, Value), ScriptError> {
    let limits = Limits::default();
    let lua = sandbox(limits)?;
    let stopped = Rc::new(Cell::new(false));
    budget(&lua, limits.instructions, Rc::clone(&stopped))?;
    let read = || -> mlua::Result<(Value, Value)> {
        lua.load(source)
            .set_name(format!("={name}"))
            .set_mode(ChunkMode::Text)
            .exec()?;
        let globals = lua.globals();
        let gadget: mlua::Value = globals.get("gadget")?;
        let settings: mlua::Value = globals.get("settings")?;
        Ok((lua.from_value(gadget)?, lua.from_value(settings)?))
    };
    match read() {
        Ok(found) => Ok(found),
        Err(_) if stopped.get() => Err(ScriptError::TooLong),
        Err(e) => Err(script_error(&e)),
    }
}

/// A Lua with only the safe libraries, and nothing that reaches outside the job.
fn sandbox(limits: Limits) -> Result<Lua, ScriptError> {
    let failed = |e: mlua::Error| ScriptError::Failed(e.to_string());
    let lua = Lua::new_with(
        StdLib::TABLE | StdLib::STRING | StdLib::MATH | StdLib::UTF8,
        LuaOptions::default(),
    )
    .map_err(failed)?;
    lua.set_memory_limit(limits.memory).map_err(failed)?;
    let globals = lua.globals();
    for name in ["dofile", "loadfile", "load", "collectgarbage", "require"] {
        globals.set(name, mlua::Nil).map_err(failed)?;
    }
    let string: mlua::Table = globals.get("string").map_err(failed)?;
    string.set("dump", mlua::Nil).map_err(failed)?;
    Ok(lua)
}

/// Stops the gadget once it has run `instructions` Lua instructions, setting `stopped`.
fn budget(lua: &Lua, instructions: u64, stopped: Rc<Cell<bool>>) -> Result<(), ScriptError> {
    let checks = instructions / u64::from(HOOK_EVERY);
    let used = Cell::new(0_u64);
    lua.set_hook(
        HookTriggers::new().every_nth_instruction(HOOK_EVERY),
        move |_, _| {
            used.set(used.get() + 1);
            if used.get() > checks {
                stopped.set(true);
                Err(mlua::Error::runtime("the gadget took too long"))
            } else {
                Ok(VmState::Continue)
            }
        },
    )
    .map_err(|e| ScriptError::Failed(e.to_string()))
}

/// Runs command `id` on the gadget's working copy of the job, which then shows its result to
/// the commands after it.
fn run_one(
    registry: &Registry,
    scratch: &RefCell<Document>,
    id: &str,
    params: &Value,
) -> Result<Change, String> {
    let command = registry
        .get(id)
        .ok_or_else(|| format!("there is no tool called \"{id}\""))?;
    let mut doc = scratch.borrow_mut();
    let change = command
        .run_json(&doc, params)
        .map_err(|e| format!("{id}: {e}"))?;
    doc.apply(&change).map_err(|e| format!("{id}: {e:?}"))?;
    Ok(change)
}

/// Lua has one kind of table, so `{}` arrives as an empty list; commands want an object.
fn json_params(params: Value) -> Value {
    match params {
        Value::Array(a) if a.is_empty() => Value::Object(serde_json::Map::new()),
        Value::Null => Value::Object(serde_json::Map::new()),
        other => other,
    }
}

/// The error the person sees, without Lua's internal wrapping and stack traceback.
fn script_error(e: &mlua::Error) -> ScriptError {
    match e {
        mlua::Error::SyntaxError { message, .. } => ScriptError::Syntax(message.clone()),
        mlua::Error::MemoryError(_) => ScriptError::OutOfMemory,
        mlua::Error::CallbackError { cause, .. } => script_error(cause),
        mlua::Error::RuntimeError(message) => ScriptError::Failed(
            message
                .split("\nstack traceback:")
                .next()
                .unwrap_or(message)
                .to_owned(),
        ),
        other => ScriptError::Failed(other.to_string()),
    }
}
