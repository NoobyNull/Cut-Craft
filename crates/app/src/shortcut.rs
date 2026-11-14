//! Shortcuts written the way the menus show them: `Ctrl+A`, `Del`, `F9`, and the mouse's
//! extra buttons, `Mouse Back`, `Ctrl+Wheel Click`.

use eframe::egui::{self, Key, KeyboardShortcut, Modifiers, PointerButton};
use std::collections::BTreeMap;

/// The mouse buttons a shortcut can use, as they are written. The left and right buttons are
/// left out: they pick things and open menus everywhere, so a shortcut on them would take
/// those away. On Windows the thumb buttons are X1 and X2; on Linux buttons 8 and 9 (X11) or
/// `BTN_SIDE` and `BTN_EXTRA` (Wayland), all reported as Back and Forward.
pub const MOUSE_BUTTONS: [(PointerButton, &str); 3] = [
    (PointerButton::Middle, "Wheel Click"),
    (PointerButton::Extra1, "Mouse Back"),
    (PointerButton::Extra2, "Mouse Forward"),
];

/// The id of holding a button and dragging to pan, in either view. Not a command: it is
/// listed with the shortcuts so its buttons are set the same way and never clash with one.
pub const PAN_DRAG: &str = "pan_drag";
/// What [`PAN_DRAG`] starts as: dragging with the wheel pressed, as in most CAD programs.
pub const PAN_DRAG_DEFAULT: &str = "Wheel Click";

/// What sets a shortcut off: a key with the modifiers held, or a mouse button.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Trigger {
    Key(KeyboardShortcut),
    Mouse(Modifiers, PointerButton),
}

impl Trigger {
    /// The shortcut as the menus and the settings write it (and as it is saved).
    pub fn text(self) -> String {
        let (modifiers, last) = match self {
            Self::Key(k) => (k.modifiers, key_text(k.logical_key).to_owned()),
            Self::Mouse(m, b) => (m, button_text(b).unwrap_or("Mouse").to_owned()),
        };
        let mut out = String::new();
        if modifiers.command || modifiers.ctrl || modifiers.mac_cmd {
            out.push_str("Ctrl+");
        }
        if modifiers.alt {
            out.push_str("Alt+");
        }
        if modifiers.shift {
            out.push_str("Shift+");
        }
        out + &last
    }

    /// The mouse button, for a mouse shortcut.
    pub fn button(self) -> Option<PointerButton> {
        match self {
            Self::Mouse(_, b) => Some(b),
            Self::Key(_) => None,
        }
    }
}

/// `key` as a shortcut writes it: the menus' short names and punctuation as itself.
fn key_text(key: Key) -> &'static str {
    match key {
        Key::Delete => "Del",
        Key::Insert => "Ins",
        Key::PageUp => "PgUp",
        Key::PageDown => "PgDn",
        Key::Escape => "Esc",
        Key::Comma => ",",
        Key::Period => ".",
        Key::Minus => "-",
        Key::Equals => "=",
        Key::Slash => "/",
        Key::Semicolon => ";",
        Key::Backslash => "\\",
        Key::OpenBracket => "[",
        Key::CloseBracket => "]",
        Key::Backtick => "`",
        Key::Quote => "'",
        k => k.name(),
    }
}

fn button_text(button: PointerButton) -> Option<&'static str> {
    MOUSE_BUTTONS
        .iter()
        .find(|(b, _)| *b == button)
        .map(|(_, name)| *name)
}

/// The modifiers written before the last `+`; `None` for one not known.
fn modifiers_of(held: &[&str]) -> Option<Modifiers> {
    let mut modifiers = Modifiers::NONE;
    for m in held {
        modifiers |= match *m {
            "Ctrl" => Modifiers::COMMAND,
            "Alt" => Modifiers::ALT,
            "Shift" => Modifiers::SHIFT,
            _ => return None,
        };
    }
    Some(modifiers)
}

/// A key or mouse shortcut; `None` for an empty or unrecognised one.
pub fn parse_trigger(text: &str) -> Option<Trigger> {
    let parts: Vec<&str> = text.split('+').map(str::trim).collect();
    let (last, held) = parts.split_last()?;
    if let Some((button, _)) = MOUSE_BUTTONS.iter().find(|(_, name)| name == last) {
        return Some(Trigger::Mouse(modifiers_of(held)?, *button));
    }
    parse(text).map(Trigger::Key)
}

/// What a key or button pressed while a shortcut is being set gives.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Captured {
    /// The shortcut to set.
    Trigger(Trigger),
    /// Esc: stop without changing anything.
    Cancel,
    /// A button that can't be a shortcut, and why.
    Refused(&'static str),
}

/// Why the right button can't be a shortcut.
pub const RIGHT_REFUSED: &str =
    "The right button opens menus, so it can't be a shortcut. Press another key or button.";

/// The first key or mouse button pressed in `events` while a shortcut is being set: Esc
/// cancels, the right button is refused, and the left button is left to click the window's
/// own buttons (such as Cancel). Ctrl+C, Ctrl+X and Ctrl+V reach egui as Copy, Cut and Paste,
/// so they are turned back into their keys.
pub fn capture(events: &[egui::Event]) -> Option<Captured> {
    events.iter().find_map(|e| match e {
        egui::Event::Key {
            key: Key::Escape,
            pressed: true,
            modifiers,
            ..
        } if modifiers.is_none() => Some(Captured::Cancel),
        egui::Event::Key {
            key,
            pressed: true,
            repeat: false,
            modifiers,
            ..
        } => Some(Captured::Trigger(Trigger::Key(KeyboardShortcut::new(
            logical(*modifiers),
            *key,
        )))),
        egui::Event::Copy => Some(ctrl(Key::C)),
        egui::Event::Cut => Some(ctrl(Key::X)),
        egui::Event::Paste(_) => Some(ctrl(Key::V)),
        egui::Event::PointerButton {
            button: PointerButton::Secondary,
            pressed: true,
            ..
        } => Some(Captured::Refused(RIGHT_REFUSED)),
        egui::Event::PointerButton {
            button,
            pressed: true,
            modifiers,
            ..
        } if button_text(*button).is_some() => Some(Captured::Trigger(Trigger::Mouse(
            logical(*modifiers),
            *button,
        ))),
        _ => None,
    })
}

fn ctrl(key: Key) -> Captured {
    Captured::Trigger(Trigger::Key(KeyboardShortcut::new(Modifiers::COMMAND, key)))
}

/// The modifiers as a shortcut keeps them: Ctrl (or ⌘ on a Mac) as egui's command.
fn logical(m: Modifiers) -> Modifiers {
    let mut out = Modifiers::NONE;
    if m.command || m.ctrl || m.mac_cmd {
        out |= Modifiers::COMMAND;
    }
    if m.alt {
        out |= Modifiers::ALT;
    }
    if m.shift {
        out |= Modifiers::SHIFT;
    }
    out
}

/// The shortcuts in use: each action's own unless the person set others for it in Options,
/// where an empty list takes its shortcuts away. Those that can't be read are left out.
pub fn effective(
    defaults: Vec<(String, Trigger)>,
    overrides: &BTreeMap<String, Vec<String>>,
) -> Vec<(String, Trigger)> {
    let mut out: Vec<(String, Trigger)> = defaults
        .into_iter()
        .filter(|(id, _)| !overrides.contains_key(id))
        .collect();
    for (id, texts) in overrides {
        out.extend(
            texts
                .iter()
                .filter_map(|t| Some((id.clone(), parse_trigger(t)?))),
        );
    }
    out
}

/// The action other than `except` that `trigger` already runs, if any.
pub fn clash<'a>(
    bindings: &'a [(String, Trigger)],
    trigger: Trigger,
    except: &str,
) -> Option<&'a str> {
    bindings
        .iter()
        .find(|(id, t)| id != except && *t == trigger)
        .map(|(id, _)| id.as_str())
}

/// `None` for an empty or unrecognised shortcut.
pub fn parse(text: &str) -> Option<KeyboardShortcut> {
    let parts: Vec<&str> = text.split('+').map(str::trim).collect();
    let (key, held) = parts.split_last()?;
    let modifiers = modifiers_of(held)?;
    let name = match *key {
        "Del" => "Delete",
        "Ins" => "Insert",
        "PgUp" => "PageUp",
        "PgDn" => "PageDown",
        "Esc" => "Escape",
        k => k,
    };
    Key::from_name(name).map(|k| KeyboardShortcut::new(modifiers, k))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sc(mods: Modifiers, key: Key) -> KeyboardShortcut {
        KeyboardShortcut::new(mods, key)
    }

    #[test]
    fn ctrl_letter() {
        assert_eq!(parse("Ctrl+A"), Some(sc(Modifiers::COMMAND, Key::A)));
    }

    #[test]
    fn plain_keys_and_function_keys() {
        assert_eq!(parse("M"), Some(sc(Modifiers::NONE, Key::M)));
        assert_eq!(parse("F9"), Some(sc(Modifiers::NONE, Key::F9)));
    }

    #[test]
    fn short_key_names() {
        assert_eq!(parse("Del"), Some(sc(Modifiers::NONE, Key::Delete)));
        assert_eq!(parse("PgUp"), Some(sc(Modifiers::NONE, Key::PageUp)));
    }

    #[test]
    fn several_modifiers() {
        assert_eq!(
            parse("Ctrl+Alt+S"),
            Some(sc(Modifiers::COMMAND | Modifiers::ALT, Key::S))
        );
        assert_eq!(parse("Shift+F6"), Some(sc(Modifiers::SHIFT, Key::F6)));
    }

    #[test]
    fn empty_and_unknown_give_none() {
        assert_eq!(parse(""), None);
        assert_eq!(parse("Ctrl+Nonsense"), None);
    }

    #[test]
    fn every_key_reads_back_as_it_is_written() {
        for &key in Key::ALL {
            for mods in [Modifiers::NONE, Modifiers::COMMAND | Modifiers::SHIFT] {
                let t = Trigger::Key(sc(mods, key));
                assert_eq!(parse_trigger(&t.text()), Some(t), "{}", t.text());
            }
        }
    }

    #[test]
    fn mouse_buttons_are_written_and_read() {
        let back = Trigger::Mouse(Modifiers::NONE, PointerButton::Extra1);
        assert_eq!(back.text(), "Mouse Back");
        assert_eq!(parse_trigger("Mouse Back"), Some(back));
        let forward = Trigger::Mouse(Modifiers::COMMAND, PointerButton::Extra2);
        assert_eq!(forward.text(), "Ctrl+Mouse Forward");
        assert_eq!(parse_trigger("Ctrl+Mouse Forward"), Some(forward));
        assert_eq!(
            parse_trigger("Wheel Click"),
            Some(Trigger::Mouse(Modifiers::NONE, PointerButton::Middle))
        );
        assert_eq!(parse_trigger("Ctrl+F9"), parse("Ctrl+F9").map(Trigger::Key));
        assert_eq!(parse_trigger("Mouse Left"), None);
    }

    fn key_event(key: Key, modifiers: Modifiers) -> egui::Event {
        egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers,
        }
    }

    fn button(button: PointerButton, modifiers: Modifiers) -> egui::Event {
        egui::Event::PointerButton {
            pos: egui::Pos2::ZERO,
            button,
            pressed: true,
            modifiers,
        }
    }

    #[test]
    fn capturing_takes_the_first_key_or_extra_button() {
        // a key with Ctrl held on Linux comes with both ctrl and command set
        let held = Modifiers {
            ctrl: true,
            command: true,
            ..Modifiers::NONE
        };
        assert_eq!(
            capture(&[key_event(Key::K, held)]),
            Some(Captured::Trigger(Trigger::Key(sc(
                Modifiers::COMMAND,
                Key::K
            ))))
        );
        assert_eq!(
            capture(&[button(PointerButton::Extra1, Modifiers::NONE)]),
            Some(Captured::Trigger(Trigger::Mouse(
                Modifiers::NONE,
                PointerButton::Extra1
            )))
        );
        assert_eq!(
            capture(&[button(PointerButton::Middle, Modifiers::SHIFT)]),
            Some(Captured::Trigger(Trigger::Mouse(
                Modifiers::SHIFT,
                PointerButton::Middle
            )))
        );
        assert_eq!(capture(&[egui::Event::Copy]), Some(ctrl(Key::C)));
    }

    #[test]
    fn capturing_cancels_on_escape_and_refuses_the_right_button() {
        assert_eq!(
            capture(&[key_event(Key::Escape, Modifiers::NONE)]),
            Some(Captured::Cancel)
        );
        assert_eq!(
            capture(&[button(PointerButton::Secondary, Modifiers::NONE)]),
            Some(Captured::Refused(RIGHT_REFUSED))
        );
        // the left button is the window's own, to press Cancel with
        assert_eq!(
            capture(&[button(PointerButton::Primary, Modifiers::NONE)]),
            None
        );
        assert_eq!(capture(&[]), None);
    }

    #[test]
    fn set_shortcuts_replace_an_actions_own() {
        let k = |t: &str| parse_trigger(t).unwrap();
        let defaults = vec![
            ("undo".to_owned(), k("Ctrl+Z")),
            ("pan".to_owned(), k("P")),
            (PAN_DRAG.to_owned(), k(PAN_DRAG_DEFAULT)),
        ];
        let overrides = BTreeMap::from([
            (
                "pan".to_owned(),
                vec!["Mouse Back".to_owned(), "P".to_owned()],
            ),
            (PAN_DRAG.to_owned(), vec![]),
            (
                "zoom_to_drawing".to_owned(),
                vec!["nonsense+key".to_owned()],
            ),
        ]);
        let mut got = effective(defaults, &overrides);
        got.sort_by_key(|(id, t)| (id.clone(), t.text()));
        assert_eq!(
            got,
            vec![
                ("pan".to_owned(), k("Mouse Back")),
                ("pan".to_owned(), k("P")),
                ("undo".to_owned(), k("Ctrl+Z")),
            ]
        );
        assert_eq!(clash(&got, k("Mouse Back"), "undo"), Some("pan"));
        assert_eq!(clash(&got, k("Mouse Back"), "pan"), None);
        assert_eq!(clash(&got, k("Ctrl+Y"), "undo"), None);
    }
}
