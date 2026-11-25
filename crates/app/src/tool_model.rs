//! The tools every window layout shows, defined once: which groups there are and in what
//! order, which kind of tool each group is, the sub-tabs that gather them, each tool's
//! right-click menu, and the favourites picked from them. A layout only decides where these
//! sit on screen; what they are and what a right-click offers is the same in every layout
//! (`docs/superpowers/specs/2026-11-24-shared-tool-model-design.md`).
//!
//! There are two kinds of tools, named by [`Page`]: the Create/Edit tools
//! ([`Page::Design`]: drawing, editing, arranging, modeling, importing) and the Process tools
//! ([`Page::Toolpaths`]: making toolpaths and the files the machine runs).

use crate::gui::{self, MenuItem, SubTabSet, ToolGroup, Toolbar};
use crate::options::Favourites;
use crate::pages::Page;

/// Where a tool's button is shown, which decides what its right-click menu offers after
/// Add to (or Remove from) Favourites.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Place {
    /// An icon in a grid of its group's tools: the menu can fold tool groups.
    Grid,
    /// A row in a list of tools by name (a rail's list, a folded group's list).
    Row,
    /// Among the favourites: the menu moves it along them.
    Favourites,
}

/// The tool groups of a [`Toolbar`], read the same way by every layout.
#[derive(Clone, Copy)]
pub struct ToolModel<'a> {
    toolbar: &'a Toolbar,
}

impl<'a> ToolModel<'a> {
    pub fn new(toolbar: &'a Toolbar) -> Self {
        Self { toolbar }
    }

    /// Every group: the Create/Edit groups (drawing, then modeling), then the Process ones.
    pub fn all_groups(self) -> Vec<ToolGroup> {
        [Page::Design, Page::Toolpaths]
            .into_iter()
            .flat_map(|kind| self.groups(kind))
            .collect()
    }

    /// One kind's groups, in order.
    pub fn groups(self, kind: Page) -> Vec<ToolGroup> {
        let t = self.toolbar;
        match kind {
            Page::Design => t.groups.iter().chain(&t.modeling_groups).copied().collect(),
            Page::Toolpaths => t.toolpath_groups.clone(),
        }
    }

    /// One kind's sub-tabs and the groups shown over them whichever is open.
    pub fn sections(kind: Page) -> SubTabSet {
        match kind {
            Page::Design => gui::DESIGN,
            Page::Toolpaths => gui::TOOLPATHS,
        }
    }

    /// The kind of tool `id` is: Process when a Process group holds it, else Create/Edit
    /// (gadgets and add-on tools are drawn with the Create/Edit tools).
    pub fn kind_of(self, id: &str) -> Page {
        if self
            .groups(Page::Toolpaths)
            .iter()
            .any(|g| g.tools.contains(&id))
        {
            Page::Toolpaths
        } else {
            Page::Design
        }
    }

    /// `kind`'s favourites in the order picked, leaving out any no longer one of its tools.
    pub fn favourites(self, kind: Page, favourites: &Favourites) -> Vec<&'static str> {
        let groups = self.groups(kind);
        favourites
            .of(kind)
            .0
            .iter()
            .filter_map(|f| {
                groups
                    .iter()
                    .flat_map(|g| g.tools.iter().copied())
                    .find(|id| id == f)
            })
            .collect()
    }

    /// Both kinds' favourites, the Create/Edit ones first.
    pub fn every_favourite(self, favourites: &Favourites) -> Vec<&'static str> {
        let mut tools = self.favourites(Page::Design, favourites);
        tools.extend(self.favourites(Page::Toolpaths, favourites));
        tools
    }

    /// Whether `id` is on its own kind's favourites.
    pub fn is_favourite(self, id: &str, favourites: &Favourites) -> bool {
        favourites.of(self.kind_of(id)).0.iter().any(|f| f == id)
    }

    /// A tool's right-click menu shown at `place`: Add to Favourites or Remove from
    /// Favourites first in every layout, then what that place can do with it. `folded` says
    /// whether tool groups are folded into one button (Options ▸ General).
    pub fn menu(
        self,
        id: &str,
        place: Place,
        favourites: &Favourites,
        folded: bool,
    ) -> Vec<MenuItem> {
        let kind = self.kind_of(id);
        let favourite = self.is_favourite(id, favourites);
        match place {
            Place::Row => vec![gui::favourite_item(kind, id, favourite)],
            Place::Grid => gui::tool_menu(kind, id, favourite, false, folded),
            Place::Favourites => gui::tool_menu(kind, id, favourite, true, folded),
        }
    }
}

#[cfg(test)]
mod tests;
