//! How Cut Craft looks and behaves on a Mac, so it feels like a Mac app: the system font, the
//! system's colours, rounded controls and soft shadows, the window's own traffic-light buttons
//! beside the menus, shortcuts written ⌘Z rather than Ctrl+Z, and the Mac's own keys (the
//! delete key, ⇧⌘Z, ⌘,). Nothing here changes Windows or Linux: each use checks
//! [`is_mac`], which is egui's idea of the system (a test can pretend to be a Mac).

use super::egui;
use crate::shortcut;
use egui::{Color32, CornerRadius, KeyboardShortcut, Shadow, Stroke, Theme, Vec2, Visuals};

/// Whether Cut Craft is running on a Mac.
pub(super) fn is_mac(ctx: &egui::Context) -> bool {
    ctx.os() == egui::os::OperatingSystem::Mac
}

/// The Mac's UI fonts, the first installed used: San Francisco under the names it goes by, then
/// Helvetica Neue, the system font before it.
// ponytail: unconfirmed which of the San Francisco names fontdb finds on each macOS release;
// Helvetica Neue is on every Mac Cut Craft supports, so the worst case is the older system font
pub(super) const UI_FONTS: [&str; 6] = [
    "SF Pro Text",
    "SF Pro",
    "System Font",
    ".SF NS",
    ".SF NS Text",
    "Helvetica Neue",
];

/// Room left of the menus for the window's close, minimise and zoom buttons, in points.
pub(super) const TRAFFIC_LIGHTS: f32 = 76.0;
/// The height of a Mac title bar, in points: the menus' row is this tall so the traffic lights
/// sit centred in it.
pub(super) const TITLE_BAR_HEIGHT: f32 = 28.0;

/// The main window: on a Mac it keeps the system's traffic-light buttons. Without the desktop's
/// own title bar (Options ▸ Appearance), the window's content runs up under a see-through title
/// bar, so the menus sit beside the buttons in one row as Mac apps' toolbars do.
pub(super) fn viewport(
    builder: egui::ViewportBuilder,
    system_title_bar: bool,
) -> egui::ViewportBuilder {
    let builder = builder.with_decorations(true);
    if system_title_bar {
        builder
    } else {
        builder
            .with_fullsize_content_view(true)
            .with_titlebar_shown(false)
            .with_title_shown(false)
    }
}

/// A menu's shortcut written as a Mac's menus write it: `Ctrl+Shift+V` is ⇧⌘V (Cut Craft's Ctrl
/// is the Mac's ⌘), with the modifiers in the Mac's order and its symbols for named keys.
pub(super) fn shortcut_text(text: &str) -> String {
    let text = MAC_KEYS
        .iter()
        .find(|(windows, _)| *windows == text)
        .map_or(text, |(_, mac)| mac);
    let parts: Vec<&str> = text.split('+').map(str::trim).collect();
    let Some((key, held)) = parts.split_last() else {
        return text.to_owned();
    };
    let mut out = String::new();
    // the Mac's order: ⌃ ⌥ ⇧ ⌘
    for (name, symbol) in [("Alt", '⌥'), ("Shift", '⇧'), ("Ctrl", '⌘')] {
        if held.contains(&name) {
            out.push(symbol);
        }
    }
    out.push_str(match *key {
        "Del" | "Delete" => "⌫",
        "Esc" | "Escape" => "⎋",
        "Enter" => "↩",
        "Tab" => "⇥",
        "PgUp" => "⇞",
        "PgDn" => "⇟",
        "Home" => "↖",
        "End" => "↘",
        "Up" => "↑",
        "Down" => "↓",
        "Left" => "←",
        "Right" => "→",
        k => k,
    });
    out
}

/// Shortcuts whose Mac spelling differs from Windows': Redo is ⇧⌘Z on a Mac.
const MAC_KEYS: [(&str, &str); 1] = [("Ctrl+Y", "Ctrl+Shift+Z")];

/// The Mac's own keys for what `bindings` already binds, to go with them: the delete key (which
/// is egui's Backspace) for Del, ⇧⌘Z for Redo, and ⌘, for Options as every Mac app's settings.
pub(super) fn extra_bindings(
    bindings: &[(String, KeyboardShortcut)],
) -> Vec<(String, KeyboardShortcut)> {
    let mut extra: Vec<(String, KeyboardShortcut)> = bindings
        .iter()
        .filter_map(|(id, keys)| {
            let mac = if *keys == KeyboardShortcut::new(egui::Modifiers::NONE, egui::Key::Delete) {
                KeyboardShortcut::new(egui::Modifiers::NONE, egui::Key::Backspace)
            } else {
                let (_, mac) = MAC_KEYS
                    .iter()
                    .find(|(windows, _)| shortcut::parse(windows) == Some(*keys))?;
                shortcut::parse(mac)?
            };
            Some((id.clone(), mac))
        })
        .collect();
    if let Some(options) = shortcut::parse("Ctrl+,") {
        extra.push(("options".to_owned(), options));
    }
    extra
}

/// macOS's look for both themes: its window and control colours, its blue accent, rounded
/// controls, thin outlines and soft shadows.
pub(super) fn style(ctx: &egui::Context) {
    for theme in [Theme::Light, Theme::Dark] {
        ctx.style_mut_of(theme, |style| {
            visuals(&mut style.visuals, theme == Theme::Dark);
            // a little more room inside buttons, as the Mac's rounded ones have
            style.spacing.button_padding = Vec2::new(7.0, 2.0);
            style.spacing.menu_margin = egui::Margin::same(5);
        });
    }
}

/// One of macOS's colours, light and dark.
struct Mac {
    light: [u8; 3],
    dark: [u8; 3],
}

impl Mac {
    fn pick(&self, dark: bool) -> Color32 {
        let [r, g, b] = if dark { self.dark } else { self.light };
        Color32::from_rgb(r, g, b)
    }
}

/// A window's background (`windowBackgroundColor`).
const WINDOW: Mac = Mac {
    light: [236, 236, 236],
    dark: [43, 43, 43],
};
/// A floating window or dialog's background, a shade apart from the window behind.
const DIALOG: Mac = Mac {
    light: [246, 246, 246],
    dark: [50, 50, 50],
};
/// Text fields' and lists' background (`controlBackgroundColor`).
const FIELD: Mac = Mac {
    light: [255, 255, 255],
    dark: [30, 30, 30],
};
/// Every other row of a list.
const STRIPE: Mac = Mac {
    light: [244, 245, 245],
    dark: [36, 36, 36],
};
/// A push button, at rest, under the pointer, and pressed.
const BUTTON: Mac = Mac {
    light: [255, 255, 255],
    dark: [80, 80, 80],
};
const BUTTON_HOVERED: Mac = Mac {
    light: [248, 248, 248],
    dark: [90, 90, 90],
};
const BUTTON_PRESSED: Mac = Mac {
    light: [226, 226, 226],
    dark: [106, 106, 106],
};
/// A control's outline, and a separator's.
const OUTLINE: Mac = Mac {
    light: [200, 200, 200],
    dark: [96, 96, 96],
};
const SEPARATOR: Mac = Mac {
    light: [216, 216, 216],
    dark: [64, 64, 64],
};
/// Text (`labelColor`).
const TEXT: Mac = Mac {
    light: [38, 38, 38],
    dark: [223, 223, 223],
};
/// The system blue (`controlAccentColor`), and selected text's background.
const ACCENT: Mac = Mac {
    light: [0, 122, 255],
    dark: [10, 132, 255],
};
const SELECTED: Mac = Mac {
    light: [179, 215, 255],
    dark: [63, 99, 139],
};
/// Text on [`SELECTED`]: a deep blue on light, a pale one on dark.
const SELECTED_TEXT: Mac = Mac {
    light: [0, 80, 180],
    dark: [200, 226, 255],
};
const LINK: Mac = Mac {
    light: [0, 104, 218],
    dark: [65, 156, 255],
};

fn visuals(v: &mut Visuals, dark: bool) {
    let text = TEXT.pick(dark);
    v.panel_fill = WINDOW.pick(dark);
    v.window_fill = DIALOG.pick(dark);
    v.extreme_bg_color = FIELD.pick(dark);
    v.text_edit_bg_color = Some(FIELD.pick(dark));
    v.faint_bg_color = STRIPE.pick(dark);
    v.hyperlink_color = LINK.pick(dark);
    v.selection.bg_fill = SELECTED.pick(dark);
    v.selection.stroke = Stroke::new(v.selection.stroke.width, SELECTED_TEXT.pick(dark));
    v.text_cursor.stroke.color = ACCENT.pick(dark);
    v.window_corner_radius = CornerRadius::same(10);
    v.menu_corner_radius = CornerRadius::same(6);
    v.window_stroke = Stroke::new(1.0, OUTLINE.pick(dark));
    let alpha = if dark { 110 } else { 45 };
    v.window_shadow = Shadow {
        offset: [0, 8],
        blur: 24,
        spread: 0,
        color: Color32::from_black_alpha(alpha),
    };
    v.popup_shadow = Shadow {
        offset: [0, 4],
        blur: 12,
        spread: 0,
        color: Color32::from_black_alpha(alpha),
    };
    let w = &mut v.widgets;
    w.noninteractive.bg_fill = WINDOW.pick(dark);
    w.noninteractive.weak_bg_fill = WINDOW.pick(dark);
    w.noninteractive.bg_stroke = Stroke::new(1.0, SEPARATOR.pick(dark));
    w.noninteractive.fg_stroke.color = text;
    for (state, fill) in [
        (&mut w.inactive, BUTTON.pick(dark)),
        (&mut w.hovered, BUTTON_HOVERED.pick(dark)),
        (&mut w.active, BUTTON_PRESSED.pick(dark)),
        (&mut w.open, BUTTON_PRESSED.pick(dark)),
    ] {
        state.bg_fill = fill;
        state.weak_bg_fill = fill;
        state.bg_stroke = Stroke::new(1.0, OUTLINE.pick(dark));
        state.fg_stroke.color = text;
        // the Mac's controls don't grow under the pointer
        state.expansion = 0.0;
    }
    for state in [
        &mut w.noninteractive,
        &mut w.inactive,
        &mut w.hovered,
        &mut w.active,
        &mut w.open,
    ] {
        state.corner_radius = CornerRadius::same(5);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{Key, Modifiers};

    #[test]
    fn shortcuts_read_as_a_macs_menus_write_them() {
        assert_eq!(shortcut_text("Ctrl+Z"), "⌘Z");
        assert_eq!(shortcut_text("Ctrl+Shift+V"), "⇧⌘V");
        assert_eq!(shortcut_text("Ctrl+Alt+S"), "⌥⌘S");
        assert_eq!(shortcut_text("Shift+Ctrl+Alt+S"), "⌥⇧⌘S", "the Mac's order");
        assert_eq!(shortcut_text("Del"), "⌫", "the Mac's delete key");
        assert_eq!(shortcut_text("Esc"), "⎋");
        assert_eq!(shortcut_text("F9"), "F9");
        assert_eq!(shortcut_text("E"), "E");
        assert_eq!(shortcut_text("Ctrl+Y"), "⇧⌘Z", "Redo, as Mac apps have it");
        assert_eq!(shortcut_text(""), "");
    }

    fn sc(mods: Modifiers, key: Key) -> KeyboardShortcut {
        KeyboardShortcut::new(mods, key)
    }

    #[test]
    fn the_macs_own_keys_join_the_bindings() {
        let bindings = vec![
            ("delete".to_owned(), sc(Modifiers::NONE, Key::Delete)),
            ("redo".to_owned(), sc(Modifiers::COMMAND, Key::Y)),
            ("undo".to_owned(), sc(Modifiers::COMMAND, Key::Z)),
        ];
        let extra = extra_bindings(&bindings);
        assert!(extra.contains(&("delete".to_owned(), sc(Modifiers::NONE, Key::Backspace))));
        assert!(extra.contains(&(
            "redo".to_owned(),
            sc(Modifiers::COMMAND | Modifiers::SHIFT, Key::Z)
        )));
        assert!(extra.contains(&("options".to_owned(), sc(Modifiers::COMMAND, Key::Comma))));
        assert!(
            !extra.iter().any(|(id, _)| id == "undo"),
            "⌘Z is already the Mac's Undo"
        );
    }

    #[test]
    fn the_mac_look_has_the_systems_colours_and_rounded_controls() {
        let ctx = egui::Context::default();
        style(&ctx);
        for (theme, dark) in [(Theme::Light, false), (Theme::Dark, true)] {
            ctx.set_theme(theme);
            let v = ctx.global_style().visuals.clone();
            assert_eq!(v.dark_mode, dark, "each theme keeps its own");
            assert_eq!(v.panel_fill, WINDOW.pick(dark));
            assert_eq!(v.selection.bg_fill, SELECTED.pick(dark));
            assert_eq!(v.widgets.inactive.corner_radius, CornerRadius::same(5));
            assert!(
                (v.widgets.inactive.bg_stroke.width - 1.0).abs() < f32::EPSILON,
                "thin outlines"
            );
        }
    }

    #[test]
    fn a_mac_window_keeps_its_traffic_lights() {
        let unified = viewport(egui::ViewportBuilder::default(), false);
        assert_eq!(unified.decorations, Some(true));
        assert_eq!(unified.fullsize_content_view, Some(true));
        assert_eq!(unified.titlebar_shown, Some(false));
        let framed = viewport(egui::ViewportBuilder::default(), true);
        assert_eq!(framed.decorations, Some(true));
        assert_eq!(
            framed.fullsize_content_view, None,
            "the desktop's own title bar"
        );
    }

    #[test]
    fn only_a_mac_counts_as_one() {
        let ctx = egui::Context::default();
        ctx.set_os(egui::os::OperatingSystem::Nix);
        assert!(!is_mac(&ctx));
        ctx.set_os(egui::os::OperatingSystem::Mac);
        assert!(is_mac(&ctx));
    }
}
