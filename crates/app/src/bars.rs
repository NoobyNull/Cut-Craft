//! Where a bar of tools sits, the same for every bar: Favourites, your own toolbars and the
//! Canvas palette. A bar is in the layout (a tab, a Rail list, a Classic section), snapped
//! to a side of the drawing, or floating over it. A floating bar starts near the top left,
//! goes where its grip is dragged and keeps its distance from the window corner it was left
//! nearest, so it stays put as the window changes size; locked, a drag leaves it where it
//! is. Snapped, it stays on its side, and a drag never pulls it off: only its menu does.
//! Kept in the options; changed only through [`apply`], so every bar changes the same way.

use crate::gui::MenuItem;
use egui::{Pos2, Rect, Vec2, pos2, vec2};
use serde::{Deserialize, Serialize};

/// A side of the drawing a bar snaps to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Side {
    Top,
    Bottom,
    Left,
    Right,
}

impl Side {
    pub const ALL: [Self; 4] = [Self::Top, Self::Bottom, Self::Left, Self::Right];

    /// The side as an action names it.
    pub fn key(self) -> &'static str {
        match self {
            Self::Top => "top",
            Self::Bottom => "bottom",
            Self::Left => "left",
            Self::Right => "right",
        }
    }

    fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|side| side.key() == key)
    }

    /// The menu item that snaps a bar here.
    fn menu_name(self) -> &'static str {
        match self {
            Self::Top => "Snap to Top",
            Self::Bottom => "Snap to Bottom",
            Self::Left => "Snap to Left",
            Self::Right => "Snap to Right",
        }
    }
}

/// A corner of the window a floating bar keeps its distance from.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Corner {
    #[default]
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

/// A bar floating over the drawing.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Floating {
    /// The corner it keeps its distance from.
    #[serde(default)]
    pub corner: Corner,
    /// How far it is from that corner, across and down (or up), in points; none until it
    /// is first moved, when it sits where its bar starts.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub offset: Option<[f32; 2]>,
    /// A drag on its grip leaves it where it is.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub locked: bool,
    /// Turned to run down instead of across.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub down: bool,
}

impl Floating {
    /// Its top-left corner in `window`, being `size` big; `start` until it has been moved.
    /// Always inside the window, however small it has become.
    pub fn top_left(&self, size: Vec2, window: Rect, start: Pos2) -> Pos2 {
        let at = match self.offset {
            None => start,
            Some([x, y]) => {
                let left = match self.corner {
                    Corner::TopLeft | Corner::BottomLeft => window.left() + x,
                    Corner::TopRight | Corner::BottomRight => window.right() - x - size.x,
                };
                let top = match self.corner {
                    Corner::TopLeft | Corner::TopRight => window.top() + y,
                    Corner::BottomLeft | Corner::BottomRight => window.bottom() - y - size.y,
                };
                pos2(left, top)
            }
        };
        pos2(
            at.x.min(window.right() - size.x).max(window.left()),
            at.y.min(window.bottom() - size.y).max(window.top()),
        )
    }

    /// Left with its top-left corner at `at`: kept from the corner of `window` its middle
    /// is nearest. Whether it moved (a locked bar does not).
    pub fn drop_at(&mut self, at: Pos2, size: Vec2, window: Rect) -> bool {
        if self.locked {
            return false;
        }
        let middle = at + size / 2.0;
        let left = middle.x < window.center().x;
        let top = middle.y < window.center().y;
        self.corner = match (top, left) {
            (true, true) => Corner::TopLeft,
            (true, false) => Corner::TopRight,
            (false, true) => Corner::BottomLeft,
            (false, false) => Corner::BottomRight,
        };
        let x = if left {
            at.x - window.left()
        } else {
            window.right() - at.x - size.x
        };
        let y = if top {
            at.y - window.top()
        } else {
            window.bottom() - at.y - size.y
        };
        self.offset = Some([x.max(0.0), y.max(0.0)]);
        true
    }
}

/// Where a bar sits.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "place", rename_all = "snake_case")]
pub enum Place {
    /// In the layout: a Ribbon or Canvas tab, a Rail list, a Classic section.
    Layout,
    /// Snapped to a side of the drawing.
    Snapped { side: Side },
    /// Floating over the drawing.
    Floating(Floating),
}

impl Default for Place {
    /// Floating, where a new bar starts.
    fn default() -> Self {
        Self::Floating(Floating::default())
    }
}

impl Place {
    /// Floating, running down: where the Canvas palette starts.
    pub fn column() -> Self {
        Self::Floating(Floating {
            down: true,
            ..Floating::default()
        })
    }

    /// Whether it is where the Canvas palette starts, never moved (so nothing needs
    /// keeping).
    pub fn is_column(&self) -> bool {
        *self == Self::column()
    }

    /// Whether its tools run across (snapped to the top or the foot, or floating unturned)
    /// rather than down.
    pub fn runs_across(&self) -> bool {
        match self {
            Self::Layout => true,
            Self::Snapped { side } => matches!(side, Side::Top | Side::Bottom),
            Self::Floating(floating) => !floating.down,
        }
    }
}

/// The action that opens the Customize Toolbars window, last on every bar's menu.
pub const CUSTOMIZE: &str = "customize_toolbars";

/// What a bar's menu says to put it back in the layout, by the layout it is in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Home {
    /// Ribbon and Canvas: a tab in the header.
    Header,
    /// Rail: a list of the Rail's.
    Rail,
    /// Classic: a section of the Design tab.
    DesignTab,
}

impl Home {
    fn attach_name(self) -> &'static str {
        match self {
            Self::Header => "Attach to Header",
            Self::Rail => "Attach to Rail",
            Self::DesignTab => "Attach to Design Tab",
        }
    }
}

/// A bar's menu, its actions each `prefix` then what to do: the four sides to snap to,
/// Float, then (floating) Lock Position and Turn; then for a bar with a `home` in the
/// layout, Detach out of it or Attach back to it.
pub fn menu(place: &Place, prefix: &str, home: Option<Home>) -> Vec<MenuItem> {
    let action = |what: &str| format!("{prefix}{what}");
    let mut menu: Vec<MenuItem> = Side::ALL
        .into_iter()
        .map(|side| MenuItem {
            is_checked: *place == Place::Snapped { side },
            ..MenuItem::with_action(side.menu_name(), &action(&format!("snap:{}", side.key())))
        })
        .collect();
    menu.push(MenuItem {
        is_checked: matches!(place, Place::Floating(_)),
        ..MenuItem::with_action("Float", &action("float"))
    });
    if let Place::Floating(floating) = place {
        menu.push(MenuItem::separator());
        menu.push(MenuItem {
            is_checked: floating.locked,
            ..MenuItem::with_action("Lock Position", &action("lock"))
        });
        menu.push(MenuItem::with_action("Turn", &action("turn")));
    }
    if let Some(home) = home {
        menu.push(MenuItem::separator());
        menu.push(if *place == Place::Layout {
            MenuItem::with_action("Detach", &action("float"))
        } else {
            MenuItem::with_action(home.attach_name(), &action("attach"))
        });
    }
    menu.push(MenuItem::separator());
    menu.push(MenuItem::with_action("Customize Toolbars...", CUSTOMIZE));
    menu
}

/// Carries out bar action `what` (after its prefix) on `place`: `snap:<side>`, `float`,
/// `lock`, `turn` (floating only) or `attach` (back into the layout, for a bar that can
/// be). Whether it changed anything.
pub fn apply(place: &mut Place, what: &str, can_attach: bool) -> bool {
    let before = *place;
    match what.split_once(':') {
        Some(("snap", side)) => {
            if let Some(side) = Side::from_key(side) {
                *place = Place::Snapped { side };
            }
        }
        _ => match (what, &mut *place) {
            ("float", Place::Floating(_)) => {}
            // off a side it keeps running the way it ran along it
            (
                "float",
                Place::Snapped {
                    side: Side::Left | Side::Right,
                },
            ) => {
                *place = Place::column();
            }
            ("float", _) => *place = Place::default(),
            ("lock", Place::Floating(floating)) => floating.locked = !floating.locked,
            ("turn", Place::Floating(floating)) => floating.down = !floating.down,
            ("attach", _) if can_attach => *place = Place::Layout,
            _ => {}
        },
    }
    *place != before
}

/// Where a floating bar starts, the `n`th of them, so that several made at once do not
/// lie on top of one another: under `top_left`, each a step further down.
pub fn start(top_left: Pos2, n: usize, step: f32) -> Pos2 {
    // a few at most; the float is small
    #[expect(clippy::cast_precision_loss)]
    let n = n as f32;
    top_left + vec2(0.0, n * step)
}

#[cfg(test)]
mod tests;
