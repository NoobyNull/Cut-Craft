//! The settings for the services Cut Craft talks to: updates, the web API and bug reports.

use super::{Deserialize, JsonSchema, Serialize, on, yes};

/// Help ▸ Check for Updates, and the check at startup.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Updates {
    /// Every time Cut Craft starts, ask GitHub whether there is a new version,
    /// and say so if there is. Nothing is downloaded or installed until you choose Install Now
    #[serde(default = "on")]
    #[schemars(title = "Check for updates when Cut Craft starts")]
    pub check_at_startup: bool,
    /// When GitHub was last asked, in seconds since 1970.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(skip)]
    pub last_check: Option<u64>,
    /// The version Skip This Version was pressed for: not offered again at startup.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(skip)]
    pub skipped_version: Option<String>,
}

impl Updates {
    /// Whether this start asks GitHub: every start while the setting is on, however
    /// recently the last check was, so a release made since is offered straight away.
    pub fn check_this_start(&self) -> bool {
        self.check_at_startup
    }
}

impl Default for Updates {
    fn default() -> Self {
        Self {
            check_at_startup: on(),
            last_check: None,
            skipped_version: None,
        }
    }
}

/// Options ▸ Web API: lets a script, a test or Claude drive Cut Craft over HTTP, the way you
/// do with the mouse. Off until switched on.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct WebApi {
    /// Let programs on this computer drive Cut Craft: select vectors, run tools, press any
    /// button. Calls from this computer need nothing more; other computers need the access
    /// token. The status bar shows WEB API Running in green while it is listening, or in red if
    /// it could not start and why
    #[serde(default)]
    #[schemars(title = "Turn on the Web API")]
    pub enabled: bool,
    /// Also answer other computers on your network, not only this one (listens on 0.0.0.0).
    /// Every call then needs the access token shown below. Calls travel unencrypted, so only
    /// use this on a home or workshop network you trust
    #[serde(default)]
    #[schemars(title = "Allow other computers on my network")]
    pub allow_network: bool,
    /// The number programs call it on, the 7340 in localhost:7340. Pick another if a different
    /// program already uses this one
    #[serde(default = "web_api_port")]
    #[schemars(title = "Port", range(min = 1024, max = 65535))]
    pub port: u16,
    /// The access token calls must send while other computers are allowed; made the first
    /// time the Web API is switched on.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    #[schemars(skip)]
    pub token: String,
}

pub(super) fn web_api_port() -> u16 {
    cut_craft_web_api::DEFAULT_PORT
}

impl Default for WebApi {
    fn default() -> Self {
        Self {
            enabled: false,
            allow_network: false,
            port: web_api_port(),
            token: String::new(),
        }
    }
}

impl WebApi {
    /// What the server runs with, or `None` while it is switched off.
    pub fn server_settings(&self) -> Option<cut_craft_web_api::Settings> {
        self.enabled.then(|| cut_craft_web_api::Settings {
            allow_network: self.allow_network,
            port: self.port,
            // this computer's own programs need no token; open to the network, every call does
            token: self.allow_network.then(|| self.token.clone()),
        })
    }
}

/// Bug reports, and what happens after Cut Craft closes because of an error.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct BugReports {
    /// If Cut Craft closes because of an error, it keeps a crash log saying what it was
    /// doing. The next time it starts it can open a bug report with that log, for you to
    /// read and send. Your user name and home folder are left out of it, and nothing is sent
    /// until you press Submit.
    #[serde(default)]
    #[schemars(title = "If Cut Craft crashes")]
    pub crash_logs: CrashLogs,
    /// Send which system and version this computer runs (Windows 11 23H2, macOS 14 Sonoma,
    /// Ubuntu 24.04...), its processor (Intel/AMD or ARM) and graphics card with crash and
    /// bug reports: some bugs happen only on some computers. Never the computer's name or
    /// your user name. Off: only the report and its log are sent
    #[serde(default = "yes")]
    #[schemars(title = "Send details of my computer")]
    pub computer_details: bool,
    /// A random number for this install, made when a crash report is first sent, so the
    /// crash-report service can tell one install from another. Nothing about the person.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(skip)]
    pub install_id: Option<String>,
}

impl Default for BugReports {
    fn default() -> Self {
        Self {
            crash_logs: CrashLogs::default(),
            computer_details: true,
            install_id: None,
        }
    }
}

/// Whether a crash's log is offered as a bug report at the next start.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CrashLogs {
    /// Send the crash log to Cut Craft's crash-report service by itself (your user name,
    /// folders and file names left out). Where that can't be done, open a bug report with it ticked
    #[schemars(title = "Send the crash log")]
    Send,
    /// Open a bug report, and ask each time whether the crash log goes with it
    #[default]
    #[schemars(title = "Ask me each time")]
    Ask,
    /// Don't open a report: the crash log is kept in Cut Craft's settings folder
    #[schemars(title = "Don't send crash logs")]
    DontSend,
}

#[cfg(test)]
mod tests {
    use super::Updates;

    #[test]
    fn every_start_checks_even_just_after_a_check() {
        let just_checked = Updates {
            last_check: Some(u64::MAX),
            ..Updates::default()
        };
        assert!(just_checked.check_this_start());
        let off = Updates {
            check_at_startup: false,
            ..Updates::default()
        };
        assert!(!off.check_this_start());
    }
}
