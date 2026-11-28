//! One check before unsigned code runs: gadgets from the My Gadgets folder or an unsigned
//! add-on, and the programs of unsigned tool add-ons.
//!
//! Signed code (from PowerWhittle's own builds and official add-ons) always runs. Any other
//! code runs only once the user has been told it is unsigned and said yes: "Run and
//! remember" until the code changes (an update, an edit, another file in its place), or "Run
//! this time" until PowerWhittle closes. Every way of running code asks here, so the menus,
//! forms and the Web API all follow the same rule; the Web API never asks, it refuses until
//! the user has said yes in the window.

use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};

/// The file in the settings folder that keeps the "Run and remember" answers.
pub const FILE: &str = "unsigned-code-allowed.json";

/// What kind of code is asked about: the warning says what it can do.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// A Lua gadget: it can only change the open job.
    Gadget,
    /// A tool add-on's program: it runs on this computer with the user's permissions.
    AddOn,
}

/// Some code about to run.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Code {
    /// What it is remembered by: a gadget's command id, or `addon:` and an add-on's id.
    pub key: String,
    /// Its name as the user knows it.
    pub name: String,
    pub kind: Kind,
    /// Signed with PowerWhittle's release key: it runs without asking.
    pub signed: bool,
    /// The code's own fingerprint: when it changes, the user is asked again.
    pub fingerprint: String,
}

impl Code {
    /// The unsigned gadget `id` called `name`, whose file holds `source`.
    pub fn gadget(id: &str, name: &str, source: &[u8]) -> Self {
        Self {
            key: id.to_owned(),
            name: name.to_owned(),
            kind: Kind::Gadget,
            signed: false,
            fingerprint: fingerprint(source),
        }
    }

    /// The add-on `id` called `name`, whose program is `program`. An unreadable program has
    /// no fingerprint any answer was given for, so it is asked about.
    pub fn add_on(id: &str, name: &str, signed: bool, program: &Path) -> Self {
        Self {
            key: format!("addon:{id}"),
            name: name.to_owned(),
            kind: Kind::AddOn,
            signed,
            fingerprint: if signed {
                String::new()
            } else {
                std::fs::read(program).map_or_default(|bytes| fingerprint(&bytes))
            },
        }
    }
}

/// `bytes`' SHA-256, in hex.
pub fn fingerprint(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    Sha256::digest(bytes)
        .iter()
        .fold(String::new(), |mut hex, b| {
            let _ = write!(hex, "{b:02x}");
            hex
        })
}

/// The user's answer to [`warning`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Answer {
    /// Run it, and don't ask again until the code changes.
    RunAndRemember,
    /// Run it, and ask again once PowerWhittle has closed.
    RunThisTime,
    /// Don't run it; ask again next time. A "no" is never remembered, so nothing is left
    /// that silently does nothing.
    DontRun,
}

/// The buttons of the question, in order.
pub const RUN_AND_REMEMBER: &str = "Run and Remember";
pub const RUN_THIS_TIME: &str = "Run This Time";
pub const DONT_RUN: &str = "Don't Run";

/// The answers the user gave, and how to ask.
pub struct Trust {
    /// Where "Run and remember" answers are kept; `None` keeps them only until PowerWhittle
    /// closes (tests, and no settings folder).
    path: Option<PathBuf>,
    /// Remembered: key → the fingerprint of the code the user said yes to.
    remembered: BTreeMap<String, String>,
    /// Said yes to until PowerWhittle closes: key and fingerprint.
    this_time: HashSet<(String, String)>,
    /// Asks the user (a dialog; tests answer).
    pub ask: fn(&Code) -> Answer,
}

impl Trust {
    /// The answers kept at `path`, if any.
    pub fn new(path: Option<PathBuf>) -> Self {
        let remembered = path
            .as_deref()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default();
        Self {
            path,
            remembered,
            this_time: HashSet::new(),
            ask,
        }
    }

    /// Kept in the settings folder.
    pub fn in_settings_folder() -> Self {
        Self::new(crate::files::config_dir().map(|dir| dir.join(FILE)))
    }

    /// Whether `code` may run now without asking: signed, or the user said yes to this very
    /// code.
    pub fn allowed(&self, code: &Code) -> bool {
        code.signed
            || self.remembered.get(&code.key) == Some(&code.fingerprint)
            || self
                .this_time
                .contains(&(code.key.clone(), code.fingerprint.clone()))
    }

    /// Whether `code` may run, asking the user first when it is unsigned and they have not
    /// said yes to it.
    pub fn may_run(&mut self, code: &Code) -> bool {
        if self.allowed(code) {
            return true;
        }
        match (self.ask)(code) {
            Answer::RunAndRemember => {
                self.remembered
                    .insert(code.key.clone(), code.fingerprint.clone());
                self.save();
                true
            }
            Answer::RunThisTime => {
                self.this_time
                    .insert((code.key.clone(), code.fingerprint.clone()));
                true
            }
            Answer::DontRun => false,
        }
    }

    /// How many answers are remembered.
    pub fn remembered(&self) -> usize {
        self.remembered.len()
    }

    /// Forgets every answer: unsigned code is asked about again before it next runs.
    pub fn forget(&mut self) {
        self.remembered.clear();
        self.this_time.clear();
        self.save();
    }

    fn save(&self) {
        let Some(path) = &self.path else { return };
        let text = serde_json::to_string_pretty(&self.remembered).expect("strings serialise");
        let saved = path
            .parent()
            .map_or(Ok(()), std::fs::create_dir_all)
            .and_then(|()| std::fs::write(path, text));
        if let Err(e) = saved {
            // still allowed until PowerWhittle closes; asked again next time
            log::warn!("could not keep the answer in {}: {e}", path.display());
        }
    }
}

/// What every unsigned-code question says about who is responsible.
pub const NOT_RESPONSIBLE: &str = "PowerWhittle and its author are not responsible for what \
    unsigned code does or can do, including anything harmful or malicious.";

/// The question asked before unsigned `code` runs.
pub fn warning(code: &Code) -> String {
    let (what, can) = match code.kind {
        Kind::Gadget => (
            "gadget",
            "A gadget can only change the job that is open, and Undo takes back everything \
             it did in one step. Check its result before you cut.",
        ),
        Kind::AddOn => (
            "add-on",
            "A tool add-on is a program that runs on this computer with your permissions.",
        ),
    };
    format!(
        "\u{201c}{}\u{201d} is not an official PowerWhittle {what}: it is not signed, so nobody \
         has checked what it does.\n\n{NOT_RESPONSIBLE} {can}\n\n\
         {RUN_AND_REMEMBER} runs it now and from now on, until the {what} changes. \
         {RUN_THIS_TIME} asks again after PowerWhittle closes.\n\n\
         Do you want to run this unsigned code?",
        code.name
    )
}

/// Why a script may not run unsigned `code` yet, and how to let it.
pub fn refusal(code: &Code) -> String {
    format!(
        "\u{201c}{}\u{201d} is unsigned code that has not been allowed to run. Run it once \
         in the PowerWhittle window and answer {RUN_AND_REMEMBER} (or {RUN_THIS_TIME}); \
         scripts can use it after that.",
        code.name
    )
}

/// Asks, in a system dialog, whether to run unsigned `code`.
fn ask(code: &Code) -> Answer {
    let answer = crate::app::message_dialog()
        .set_title("Unsigned code")
        .set_description(warning(code))
        .set_level(rfd::MessageLevel::Warning)
        .set_buttons(rfd::MessageButtons::YesNoCancelCustom(
            RUN_AND_REMEMBER.into(),
            RUN_THIS_TIME.into(),
            DONT_RUN.into(),
        ))
        .show();
    match answer {
        rfd::MessageDialogResult::Custom(c) if c == RUN_AND_REMEMBER => Answer::RunAndRemember,
        rfd::MessageDialogResult::Custom(c) if c == RUN_THIS_TIME => Answer::RunThisTime,
        _ => Answer::DontRun,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn star(source: &str) -> Code {
        Code::gadget("gadget_star_burst", "Star Burst", source.as_bytes())
    }

    fn scratch(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("cut-craft-trust-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir.join(FILE)
    }

    #[test]
    fn signed_code_runs_without_asking() {
        let mut trust = Trust::new(None);
        trust.ask = |_| panic!("asked about signed code");
        let mut code = star("a");
        code.signed = true;
        assert!(trust.allowed(&code));
        assert!(trust.may_run(&code));
    }

    #[test]
    fn unsigned_code_is_asked_about_and_no_is_never_remembered() {
        let mut trust = Trust::new(None);
        assert!(!trust.allowed(&star("a")));
        trust.ask = |_| Answer::DontRun;
        assert!(!trust.may_run(&star("a")));
        assert!(!trust.may_run(&star("a")), "asked again, still no");
        assert!(!trust.allowed(&star("a")));
        assert_eq!(trust.remembered(), 0);
    }

    #[test]
    fn run_and_remember_lasts_until_the_code_changes_even_after_closing() {
        let path = scratch("remember");
        let mut trust = Trust::new(Some(path.clone()));
        trust.ask = |code| {
            assert!(warning(code).contains("Do you want to run this unsigned code?"));
            Answer::RunAndRemember
        };
        assert!(trust.may_run(&star("a")));
        // PowerWhittle closes and starts again: still allowed, not asked
        let mut again = Trust::new(Some(path.clone()));
        again.ask = |_| panic!("asked again about the same code");
        assert!(again.allowed(&star("a")));
        assert!(again.may_run(&star("a")));
        // the gadget changed (an update, an edit): asked again
        again.ask = |_| Answer::DontRun;
        assert!(!again.may_run(&star("b")));
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn run_this_time_lasts_until_cut_craft_closes() {
        let path = scratch("this-time");
        let mut trust = Trust::new(Some(path.clone()));
        trust.ask = |_| Answer::RunThisTime;
        assert!(trust.may_run(&star("a")));
        trust.ask = |_| panic!("asked twice in one run");
        assert!(trust.may_run(&star("a")));
        assert!(
            !Trust::new(Some(path.clone())).allowed(&star("a")),
            "closed: forgotten"
        );
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn forgetting_asks_about_everything_again() {
        let path = scratch("forget");
        let mut trust = Trust::new(Some(path.clone()));
        trust.ask = |_| Answer::RunAndRemember;
        assert!(trust.may_run(&star("a")));
        assert_eq!(trust.remembered(), 1);
        trust.forget();
        assert_eq!(trust.remembered(), 0);
        assert!(!trust.allowed(&star("a")));
        assert!(!Trust::new(Some(path.clone())).allowed(&star("a")));
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn an_add_on_is_known_by_its_program() {
        let dir = scratch("addon").parent().unwrap().to_path_buf();
        std::fs::create_dir_all(&dir).unwrap();
        let program = dir.join("program");
        std::fs::write(&program, "one").unwrap();
        let mut trust = Trust::new(None);
        trust.ask = |code| {
            assert!(warning(code).contains("program that runs on this computer"));
            Answer::RunAndRemember
        };
        let code = Code::add_on("depth", "The Depth add-on", false, &program);
        assert!(trust.may_run(&code));
        // a new program in its place is asked about
        std::fs::write(&program, "two").unwrap();
        assert!(!trust.allowed(&Code::add_on("depth", "The Depth add-on", false, &program)));
        // a signed one never is
        assert!(trust.allowed(&Code::add_on("depth", "Depth", true, &program)));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn the_question_and_the_refusal_say_who_is_responsible_and_what_to_do() {
        let text = warning(&star("a"));
        for words in [
            "Star Burst",
            "not responsible",
            "Undo",
            "not signed",
            RUN_AND_REMEMBER,
        ] {
            assert!(text.contains(words), "{words}: {text}");
        }
        let why = refusal(&star("a"));
        assert!(
            why.contains("Star Burst") && why.contains(RUN_AND_REMEMBER),
            "{why}"
        );
    }
}
