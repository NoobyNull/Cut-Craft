//! Help ▸ Messages from the Cut Craft Team: a short message, or an opinion poll with one button
//! per choice, that the team can show inside Cut Craft without a new version.
//!
//! At startup (when Edit ▸ Options ▸ Updates says yes) Cut Craft asks the team's service, in
//! the background, whether there is a message. A new one opens once in a small window; a
//! message already seen is only shown again from the Help menu. Nothing is shown, and nothing
//! said, when the computer is offline. A vote sends the choice, the version and a random
//! number for this install (so each computer votes once and can change its mind); nothing
//! about the person. The service is `services/crash-reports` (`/v1/announcement`, `/v1/vote`).
//!
//! Built in for three versions only: from [`ENDS_IN`] on, Cut Craft never asks.

use cut_craft_updater::Version;
use eframe::egui;
use serde::Deserialize;
use std::sync::mpsc::{Receiver, channel};

/// The team's service: the same Cloudflare Worker as the crash reports.
pub const SERVICE: &str = "https://cutcraft-crash-reports.mattyax.workers.dev";
/// Points messages and votes elsewhere, such as the service run locally for a test
/// (`http://localhost:8787`), never the live one.
pub const SERVICE_VARIABLE: &str = "CUTCRAFT_MESSAGES_URL";
/// The first version that no longer asks for messages: they are built in for 0.6.19, 0.6.20
/// and 0.6.21 only, and the code is to be taken out after that (issue #604).
pub const ENDS_IN: (u64, u64, u64) = (0, 6, 22);
/// The most choices a poll shows: the service allows no more.
pub const MOST_CHOICES: usize = 6;

/// Whether version `v` still asks for messages.
pub fn in_season(v: &Version) -> bool {
    (v.major, v.minor, v.patch) < ENDS_IN
}

/// A message from the team, or a poll when it has choices.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct Message {
    /// Its short name: what "already seen" and "voted" are kept against.
    pub id: String,
    pub title: String,
    pub body: String,
    /// A button each; none for a plain message.
    #[serde(default)]
    pub choices: Vec<String>,
    /// After voting, show how the vote is going.
    #[serde(default)]
    pub show_results: bool,
}

impl Message {
    pub fn is_poll(&self) -> bool {
        !self.choices.is_empty()
    }
}

/// The service's answer to "is there a message?": 204 is none; 200 is one. Anything odd is
/// an error, and so is a message too long to show tidily.
pub fn read_message(status: u16, body: &str) -> Result<Option<Message>, String> {
    match status {
        204 => Ok(None),
        200 => {
            let m: Message = serde_json::from_str(body)
                .map_err(|e| format!("the message could not be read: {e}"))?;
            let fits = !m.id.is_empty()
                && m.id.len() <= 40
                && !m.title.trim().is_empty()
                && m.title.chars().count() <= 80
                && m.body.chars().count() <= 1200
                && m.choices.len() != 1
                && m.choices.len() <= MOST_CHOICES
                && m.choices.iter().all(|c| c.chars().count() <= 40);
            if fits {
                Ok(Some(m))
            } else {
                Err("the message is not one Cut Craft can show".into())
            }
        }
        other => Err(format!("the message service answered {other}")),
    }
}

/// What a vote sends: the poll, the choice (counting from 0), this install's random number
/// and Cut Craft's version. Nothing else.
pub fn vote_body(poll: &str, choice: usize, install: &str, version: &str) -> String {
    serde_json::json!({
        "poll": poll,
        "choice": choice,
        "install_id": install,
        "version": version,
    })
    .to_string()
}

/// How the vote is going, when the service says (the poll shows its results), one count per
/// choice.
pub fn read_totals(body: &str, choices: usize) -> Option<Vec<u64>> {
    let v: serde_json::Value = serde_json::from_str(body).ok()?;
    let totals: Vec<u64> = v
        .get("totals")?
        .as_array()?
        .iter()
        .map(serde_json::Value::as_u64)
        .collect::<Option<_>>()?;
    (totals.len() == choices).then_some(totals)
}

/// A choice's share as a whole percentage.
pub fn percent(count: u64, of: u64) -> u64 {
    (count * 100 + of / 2).checked_div(of).unwrap_or(0)
}

/// Where the window has got to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Phase {
    Idle,
    Asking,
    /// No message right now (only shown when asked for from the Help menu).
    NoMessage,
    Showing(Message),
    Sending(Message, usize),
    /// The vote went in: with how it is going, when the poll shows that.
    Voted(Message, usize, Option<Vec<u64>>),
    Failed(String),
}

/// What the window asks the app to keep.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Asked {
    Nothing,
    /// The message was shown: not opened by itself again.
    Seen(String),
    /// This install voted for this choice of this poll.
    Voted(String, usize),
    /// Don't show messages like this: switches the startup check off.
    TurnOff,
}

/// What the background thread says.
enum Msg {
    Asked(Result<Option<Message>, String>),
    Voted(Result<Option<Vec<u64>>, String>),
}

pub struct Announcements {
    pub phase: Phase,
    pub open: bool,
    /// Opened from the Help menu: every answer is shown, a message already seen included.
    manual: bool,
    /// The service's address, or none (tests): nothing is sent.
    service: Option<String>,
    current: Version,
    /// This install's choice in each poll, to show which button it pressed.
    voted: Vec<(String, usize)>,
    /// The messages already shown once, which the startup check keeps quiet about.
    seen: Vec<String>,
    /// The window was closed while the question was on its way: its answer is not shown.
    dismissed: bool,
    rx: Option<Receiver<Msg>>,
}

impl Default for Announcements {
    fn default() -> Self {
        // tests never reach the live service
        let service = (!cfg!(test))
            .then(|| std::env::var(SERVICE_VARIABLE).unwrap_or_else(|_| SERVICE.to_owned()));
        Self::new(service, cut_craft_updater::current_version())
    }
}

/// `GET url`, its status and body; why it failed, if it did.
fn get(url: &str) -> Result<(u16, String), String> {
    let mut r = agent()
        .get(url)
        .header("User-Agent", user_agent())
        .call()
        .map_err(|e| e.to_string())?;
    let status = r.status().as_u16();
    let body = r.body_mut().read_to_string().map_err(|e| e.to_string())?;
    Ok((status, body))
}

fn post(url: &str, body: &str) -> Result<String, String> {
    agent()
        .post(url)
        .header("Content-Type", "application/json")
        .header("User-Agent", user_agent())
        .send(body)
        .map_err(|e| match e {
            ureq::Error::StatusCode(410) => "this vote has ended".to_owned(),
            ureq::Error::StatusCode(429) => {
                "too many votes from this network today; try again tomorrow".to_owned()
            }
            other => other.to_string(),
        })?
        .body_mut()
        .read_to_string()
        .map_err(|e| e.to_string())
}

/// Short time-outs: a slow service must never hold anything up.
fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(std::time::Duration::from_secs(10)))
        .http_status_as_error(true)
        .build()
        .into()
}

fn user_agent() -> String {
    format!("CutCraft/{}", env!("CARGO_PKG_VERSION"))
}

impl Announcements {
    pub fn new(service: Option<String>, current: Version) -> Self {
        Self {
            phase: Phase::Idle,
            open: false,
            manual: false,
            service,
            current,
            voted: vec![],
            seen: vec![],
            dismissed: false,
            rx: None,
        }
    }

    /// Opens the window at `phase` (the help's pictures).
    pub fn show(&mut self, phase: Phase) {
        (self.phase, self.open) = (phase, true);
    }

    /// Whether this version still has messages at all (the Help menu item, the startup check).
    pub fn in_season(&self) -> bool {
        in_season(&self.current)
    }

    fn busy(&self) -> bool {
        matches!(self.phase, Phase::Asking | Phase::Sending(..))
    }

    /// Asks the service in the background. `manual` (the Help menu) opens the window and
    /// shows every answer; the startup check only opens it for a message not seen before.
    /// `voted` is this install's earlier votes, `seen` the messages already shown.
    pub fn ask(
        &mut self,
        ctx: &egui::Context,
        manual: bool,
        seen: &[String],
        voted: Vec<(String, usize)>,
    ) {
        self.voted = voted;
        self.seen = seen.to_vec();
        if manual {
            (self.open, self.dismissed) = (true, false);
        }
        if !self.in_season() {
            if manual {
                self.phase = Phase::NoMessage;
            }
            return;
        }
        if self.busy() {
            self.manual |= manual;
            return;
        }
        self.manual = manual;
        self.phase = Phase::Asking;
        let Some(service) = self.service.clone() else {
            return;
        };
        let (tx, rx) = channel();
        self.rx = Some(rx);
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let answer = get(&format!("{service}/v1/announcement"))
                .and_then(|(status, body)| read_message(status, &body));
            let _ = tx.send(Msg::Asked(answer));
            ctx.request_repaint();
        });
    }

    /// The service's answer to "is there a message?".
    pub fn answered(&mut self, answer: Result<Option<Message>, String>) -> Asked {
        // the startup check is quiet about one already shown; the Help menu shows it, even
        // when it was chosen while the startup check was still asking
        let answer = answer.map(|m| m.filter(|m| self.manual || !self.seen.contains(&m.id)));
        let (phase, asked) = match answer {
            Ok(Some(m)) => {
                let id = m.id.clone();
                (Phase::Showing(m), Asked::Seen(id))
            }
            Ok(None) if self.manual => (Phase::NoMessage, Asked::Nothing),
            Err(e) if self.manual => (Phase::Failed(e), Asked::Nothing),
            Err(e) => {
                log::info!("no message from the Cut Craft team: {e}");
                (Phase::Idle, Asked::Nothing)
            }
            Ok(None) => (Phase::Idle, Asked::Nothing),
        };
        if std::mem::take(&mut self.dismissed) {
            // closed while asking: keep it closed, and the message not marked as seen
            self.phase = Phase::Idle;
            return Asked::Nothing;
        }
        self.open |= !matches!(phase, Phase::Idle);
        self.phase = phase;
        asked
    }

    /// A choice's button was pressed: sends the vote in the background.
    fn vote(&mut self, ctx: &egui::Context, message: &Message, choice: usize, install: &str) {
        self.phase = Phase::Sending(message.clone(), choice);
        let Some(service) = self.service.clone() else {
            return;
        };
        let (tx, rx) = channel();
        self.rx = Some(rx);
        let body = vote_body(&message.id, choice, install, &self.current.to_string());
        let (ctx, choices) = (ctx.clone(), message.choices.len());
        std::thread::spawn(move || {
            let sent = post(&format!("{service}/v1/vote"), &body).map(|b| read_totals(&b, choices));
            let _ = tx.send(Msg::Voted(sent));
            ctx.request_repaint();
        });
    }

    /// The service's answer to a vote.
    pub fn vote_answered(&mut self, sent: Result<Option<Vec<u64>>, String>) -> Asked {
        let Phase::Sending(message, choice) = self.phase.clone() else {
            return Asked::Nothing;
        };
        match sent {
            Ok(totals) => {
                self.voted.retain(|(id, _)| *id != message.id);
                self.voted.push((message.id.clone(), choice));
                let id = message.id.clone();
                self.phase = Phase::Voted(message, choice, totals);
                Asked::Voted(id, choice)
            }
            Err(e) => {
                self.phase = Phase::Failed(format!("Your vote was not sent: {e}."));
                Asked::Nothing
            }
        }
    }

    fn poll(&mut self) -> Asked {
        let Some(rx) = &self.rx else {
            return Asked::Nothing;
        };
        let msgs: Vec<Msg> = rx.try_iter().collect();
        let mut asked = Asked::Nothing;
        for msg in msgs {
            let a = match msg {
                Msg::Asked(answer) => self.answered(answer),
                Msg::Voted(sent) => self.vote_answered(sent),
            };
            if a != Asked::Nothing {
                asked = a;
            }
        }
        asked
    }

    /// This install's earlier choice in poll `id`.
    fn my_vote(&self, id: &str) -> Option<usize> {
        self.voted.iter().find(|(p, _)| p == id).map(|(_, c)| *c)
    }

    /// The window, when open; what it asks the app to keep. `install` makes this install's
    /// random number, the first time it is needed.
    pub fn window(&mut self, ctx: &egui::Context, install: &mut dyn FnMut() -> String) -> Asked {
        let polled = self.poll();
        if !self.open {
            return polled;
        }
        let mut open = true;
        let id = egui::Id::unique("team_messages_window");
        let size = egui::vec2(440.0, 300.0);
        let pressed = cut_craft_bug_report::dialog(
            ctx,
            id,
            "Messages from the Cut Craft Team",
            size,
            &mut open,
            |ui| {
                ui.set_max_width(420.0);
                self.body(ui)
            },
        )
        .unwrap_or(Press::Nothing);
        let mut asked = polled;
        let close = match pressed {
            Press::Nothing => false,
            Press::Close => true,
            Press::Choose(choice) => {
                if let Phase::Showing(m) | Phase::Voted(m, ..) = self.phase.clone() {
                    let me = install();
                    self.vote(ctx, &m, choice, &me);
                }
                false
            }
            Press::ChangeVote => {
                if let Phase::Voted(m, ..) = self.phase.clone() {
                    self.phase = Phase::Showing(m);
                }
                false
            }
            Press::TurnOff => {
                asked = Asked::TurnOff;
                true
            }
        };
        if close || !open {
            self.open = false;
            self.dismissed = matches!(self.phase, Phase::Asking);
            if !self.busy() {
                self.phase = Phase::Idle;
            }
        }
        asked
    }

    fn body(&self, ui: &mut egui::Ui) -> Press {
        match &self.phase {
            Phase::Idle => Press::Close,
            Phase::Asking | Phase::Sending(..) => {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(if matches!(self.phase, Phase::Asking) {
                        "Asking for messages from the Cut Craft team..."
                    } else {
                        "Sending your vote..."
                    });
                });
                Press::Nothing
            }
            Phase::NoMessage => {
                ui.heading("No messages right now");
                ui.label(
                    "When the Cut Craft team has news or a question for you, it shows here, \
                     and once when Cut Craft starts.",
                );
                ui.add_space(8.0);
                pressed(ui.button("Close").clicked(), Press::Close)
            }
            Phase::Failed(e) => {
                ui.heading("Could not reach the Cut Craft team");
                ui.label(e);
                ui.label("Check this computer is online, then try again later from the Help menu.");
                ui.add_space(8.0);
                pressed(ui.button("Close").clicked(), Press::Close)
            }
            Phase::Showing(m) => self.showing(ui, m),
            Phase::Voted(m, choice, totals) => voted(ui, m, *choice, totals.as_deref()),
        }
    }

    fn showing(&self, ui: &mut egui::Ui, m: &Message) -> Press {
        ui.heading(&m.title);
        if m.is_poll() {
            ui.weak("An opinion poll: we'd like to know what you think. Nothing is decided by it.");
        }
        ui.add_space(4.0);
        ui.label(&m.body);
        ui.add_space(8.0);
        let mut press = Press::Nothing;
        if m.is_poll() {
            let mine = self.my_vote(&m.id);
            ui.horizontal_wrapped(|ui| {
                for (n, choice) in m.choices.iter().enumerate() {
                    let text = if mine == Some(n) {
                        egui::RichText::new(format!("✔ {choice}")).strong()
                    } else {
                        egui::RichText::new(choice).strong()
                    };
                    let hover = if mine == Some(n) {
                        "Your vote. Press another choice to change it".to_owned()
                    } else {
                        format!("Vote for {choice}")
                    };
                    if ui.button(text).on_hover_text(hover).clicked() {
                        press = Press::Choose(n);
                    }
                }
            });
            ui.add_space(4.0);
            ui.weak(
                "Your vote goes with a random number for this computer, so each computer votes \
                 once and can change its vote. Nothing about you is sent.",
            );
            ui.add_space(8.0);
        }
        ui.horizontal(|ui| {
            let label = if m.is_poll() && self.my_vote(&m.id).is_none() {
                "Not Now"
            } else {
                "Close"
            };
            if ui.button(label).clicked() {
                press = Press::Close;
            }
            if ui
                .link("Don't show me messages like this")
                .on_hover_text(
                    "Turns off Edit ▸ Options ▸ Updates ▸ Show messages from the Cut Craft team. \
                     Help ▸ Messages from the Cut Craft Team still shows them",
                )
                .clicked()
            {
                press = Press::TurnOff;
            }
        });
        press
    }
}

/// After voting: thanks, and how the vote is going when the poll shows that.
fn voted(ui: &mut egui::Ui, m: &Message, choice: usize, totals: Option<&[u64]>) -> Press {
    ui.heading(&m.title);
    let picked = m.choices.get(choice).map_or("", String::as_str);
    ui.label(format!("Thanks! Your vote for {picked} is in."));
    if let Some(totals) = totals {
        let all: u64 = totals.iter().sum();
        ui.add_space(6.0);
        ui.strong(format!(
            "How it is going: {all} {}",
            if all == 1 { "vote" } else { "votes" }
        ));
        for (name, &n) in m.choices.iter().zip(totals) {
            ui.horizontal(|ui| {
                ui.add_sized([110.0, 18.0], egui::Label::new(name).truncate());
                let share = f32::from(u8::try_from(percent(n, all)).unwrap_or(100)) / 100.0;
                ui.add(
                    egui::ProgressBar::new(share)
                        .desired_width(180.0)
                        .text(format!("{n} ({}%)", percent(n, all))),
                );
            });
        }
    }
    ui.add_space(8.0);
    ui.horizontal(|ui| {
        let change = ui.button("Change My Vote").clicked();
        let close = ui.button("Close").clicked();
        if change {
            Press::ChangeVote
        } else {
            pressed(close, Press::Close)
        }
    })
    .inner
}

/// A button pressed in the window.
enum Press {
    Nothing,
    Close,
    Choose(usize),
    ChangeVote,
    TurnOff,
}

fn pressed(clicked: bool, press: Press) -> Press {
    if clicked { press } else { Press::Nothing }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn poll() -> Message {
        Message {
            id: "name-poll".into(),
            title: "What should Cut Craft be called?".into(),
            body: "We'd like to know what you think.".into(),
            choices: vec!["CutCraft".into(), "PowerWhittle".into()],
            show_results: true,
        }
    }

    fn at(v: &str) -> Announcements {
        Announcements::new(None, Version::parse(v).unwrap())
    }

    fn draw(a: &mut Announcements) -> Asked {
        let ctx = egui::Context::default();
        let mut asked = Asked::Nothing;
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
            asked = a.window(ui.ctx(), &mut || "install-0001".into());
        });
        out.textures_delta.clear();
        asked
    }

    #[test]
    fn messages_are_built_in_for_three_versions_only() {
        for v in ["0.6.18", "0.6.19", "0.6.20", "0.6.21"] {
            assert!(in_season(&Version::parse(v).unwrap()), "{v}");
        }
        for v in ["0.6.22", "0.6.30", "0.7.0", "1.0.0"] {
            assert!(!in_season(&Version::parse(v).unwrap()), "{v}");
        }
    }

    #[test]
    fn a_later_version_never_asks() {
        let mut a = at("0.6.22");
        a.ask(&egui::Context::default(), false, &[], vec![]);
        assert_eq!(a.phase, Phase::Idle);
        assert!(!a.open);
    }

    #[test]
    fn the_services_answer_is_read() {
        assert_eq!(read_message(204, ""), Ok(None));
        let body = r#"{"id":"name-poll","title":"What should Cut Craft be called?",
            "body":"We'd like to know what you think.","choices":["CutCraft","PowerWhittle"],
            "show_results":true}"#;
        assert_eq!(read_message(200, body), Ok(Some(poll())));
        let plain = r#"{"id":"hello","title":"Hello","body":"Thanks for testing."}"#;
        let m = read_message(200, plain).unwrap().unwrap();
        assert!(!m.is_poll());
        assert!(read_message(500, "").is_err());
        assert!(read_message(200, "not json").is_err());
    }

    #[test]
    fn a_message_too_big_to_show_tidily_is_refused() {
        let mut m = serde_json::json!({"id": "x1", "title": "T", "body": "b"});
        m["choices"] = serde_json::json!(["only one"]);
        assert!(read_message(200, &m.to_string()).is_err());
        m["choices"] = serde_json::json!(["1", "2", "3", "4", "5", "6", "7"]);
        assert!(read_message(200, &m.to_string()).is_err());
        m["choices"] = serde_json::json!([]);
        m["title"] = serde_json::json!("T".repeat(81));
        assert!(read_message(200, &m.to_string()).is_err());
    }

    #[test]
    fn a_vote_sends_only_the_choice_the_version_and_the_install_number() {
        let body: serde_json::Value =
            serde_json::from_str(&vote_body("name-poll", 1, "abc-12345", "0.6.19")).unwrap();
        assert_eq!(
            body,
            serde_json::json!({
                "poll": "name-poll", "choice": 1, "install_id": "abc-12345", "version": "0.6.19"
            })
        );
    }

    #[test]
    fn totals_are_read_when_the_poll_shows_them() {
        assert_eq!(
            read_totals(r#"{"ok":true,"totals":[3,5]}"#, 2),
            Some(vec![3, 5])
        );
        assert_eq!(read_totals(r#"{"ok":true}"#, 2), None);
        assert_eq!(read_totals(r#"{"ok":true,"totals":[3]}"#, 2), None);
        assert_eq!(percent(1, 3), 33);
        assert_eq!(percent(2, 3), 67);
        assert_eq!(percent(0, 0), 0);
    }

    #[test]
    fn a_new_message_opens_by_itself_and_is_kept_as_seen() {
        let mut a = at("0.6.19");
        assert_eq!(
            a.answered(Ok(Some(poll()))),
            Asked::Seen("name-poll".into())
        );
        assert!(a.open);
        assert_eq!(a.phase, Phase::Showing(poll()));
    }

    #[test]
    fn at_startup_nothing_and_offline_say_nothing() {
        let mut a = at("0.6.19");
        a.answered(Ok(None));
        assert!(!a.open);
        a.answered(Err("no network".into()));
        assert!(!a.open);
        assert_eq!(a.phase, Phase::Idle);
    }

    #[test]
    fn from_the_help_menu_every_answer_is_shown() {
        let mut a = at("0.6.19");
        a.ask(&egui::Context::default(), true, &[], vec![]);
        assert!(a.open);
        assert_eq!(a.phase, Phase::Asking);
        a.answered(Ok(None));
        assert_eq!(a.phase, Phase::NoMessage);
        a.ask(&egui::Context::default(), true, &[], vec![]);
        a.answered(Err("no network".into()));
        assert!(matches!(a.phase, Phase::Failed(_)));
    }

    #[test]
    fn at_startup_a_message_already_seen_stays_quiet() {
        let mut a = at("0.6.19");
        a.ask(
            &egui::Context::default(),
            false,
            &["name-poll".into()],
            vec![],
        );
        assert_eq!(a.answered(Ok(Some(poll()))), Asked::Nothing);
        assert!(!a.open);
    }

    #[test]
    fn help_chosen_while_the_startup_check_asks_shows_a_message_already_seen() {
        let mut a = at("0.6.19");
        let seen = ["name-poll".to_owned()];
        a.ask(&egui::Context::default(), false, &seen, vec![]);
        a.ask(&egui::Context::default(), true, &seen, vec![]);
        a.answered(Ok(Some(poll())));
        assert!(a.open);
        assert_eq!(a.phase, Phase::Showing(poll()));
    }

    #[test]
    fn a_window_closed_while_asking_stays_closed() {
        let mut a = at("0.6.19");
        a.ask(&egui::Context::default(), true, &[], vec![]);
        a.open = false;
        a.dismissed = true;
        assert_eq!(a.answered(Ok(Some(poll()))), Asked::Nothing);
        assert!(!a.open);
        assert_eq!(a.phase, Phase::Idle);
        // asking again from Help shows it
        a.ask(&egui::Context::default(), true, &[], vec![]);
        a.answered(Ok(Some(poll())));
        assert!(a.open);
    }

    #[test]
    fn a_vote_is_kept_and_shows_how_it_is_going() {
        let mut a = at("0.6.19");
        a.answered(Ok(Some(poll())));
        a.phase = Phase::Sending(poll(), 1);
        assert_eq!(
            a.vote_answered(Ok(Some(vec![2, 3]))),
            Asked::Voted("name-poll".into(), 1)
        );
        assert_eq!(a.phase, Phase::Voted(poll(), 1, Some(vec![2, 3])));
        assert_eq!(a.my_vote("name-poll"), Some(1));
    }

    #[test]
    fn a_vote_that_was_not_sent_says_so() {
        let mut a = at("0.6.19");
        a.phase = Phase::Sending(poll(), 0);
        assert_eq!(
            a.vote_answered(Err("this vote has ended".into())),
            Asked::Nothing
        );
        assert!(matches!(&a.phase, Phase::Failed(e) if e.contains("not sent")));
        assert_eq!(a.my_vote("name-poll"), None);
    }

    #[test]
    fn every_stage_draws_and_stays_open_until_a_button_is_pressed() {
        let mut a = at("0.6.19");
        a.open = true;
        let plain = Message {
            choices: vec![],
            ..poll()
        };
        for phase in [
            Phase::Asking,
            Phase::NoMessage,
            Phase::Showing(poll()),
            Phase::Showing(plain),
            Phase::Sending(poll(), 0),
            Phase::Voted(poll(), 0, Some(vec![1, 0])),
            Phase::Voted(poll(), 1, None),
            Phase::Failed("no network".into()),
        ] {
            a.phase = phase.clone();
            assert_eq!(draw(&mut a), Asked::Nothing, "{phase:?}");
            assert!(a.open, "{phase:?}");
        }
    }
}
