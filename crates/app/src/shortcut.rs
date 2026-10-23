//! Keyboard shortcuts written the way Aspire's menus show them: `Ctrl+A`, `Del`, `F9`.

use eframe::egui::{Key, KeyboardShortcut, Modifiers};

/// `None` for an empty or unrecognised shortcut.
pub fn parse(text: &str) -> Option<KeyboardShortcut> {
    let parts: Vec<&str> = text.split('+').map(str::trim).collect();
    let (key, held) = parts.split_last()?;
    let mut modifiers = Modifiers::NONE;
    for m in held {
        modifiers = modifiers
            | match *m {
                "Ctrl" => Modifiers::COMMAND,
                "Alt" => Modifiers::ALT,
                "Shift" => Modifiers::SHIFT,
                _ => return None,
            };
    }
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
    fn aspire_key_names() {
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
}
