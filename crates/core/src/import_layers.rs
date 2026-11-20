//! The layers an imported drawing brings: each of the file's layers made in the job with its
//! name, colour and whether it is shown, so the job's layer list matches the program that
//! made the file. Shared by the importers that read layers (DXF, SVG).

use crate::ChangeBuilder;
use crate::document::LayerId;

/// A layer of an imported file, as the file has it.
#[derive(Clone, Debug, PartialEq)]
pub struct FileLayer {
    pub name: String,
    /// The colour its vectors are drawn in; none for the usual vector colour.
    pub colour: Option<[u8; 3]>,
    /// Shown; a layer the file hides comes in hidden.
    pub visible: bool,
}

impl FileLayer {
    /// A shown layer in the usual colour.
    pub fn named(name: &str) -> Self {
        Self {
            name: name.to_owned(),
            colour: None,
            visible: true,
        }
    }
}

/// The job's layer for each of the file's, by the file's name: one of the same name the job
/// already has (whatever its case, as the programs that make these files treat names), else
/// a new one.
pub struct ImportLayers(Vec<(String, LayerId, bool)>);

impl ImportLayers {
    /// Every one of `top_down`'s layers made in the job, even those with nothing on them, so
    /// the layer list reads from the top as the list in the program that made the file. (The
    /// list shows the newest on top, so they are added bottom first.)
    pub fn make(c: &mut ChangeBuilder, top_down: &[FileLayer]) -> Self {
        let mut made = Self(vec![]);
        for l in top_down.iter().rev() {
            if made.find(&l.name).is_none() {
                made.add(c, l);
            }
        }
        made
    }

    fn find(&self, name: &str) -> Option<(LayerId, bool)> {
        self.0
            .iter()
            .find(|(n, ..)| n.eq_ignore_ascii_case(name))
            .map(|&(_, id, shown)| (id, shown))
    }

    fn add(&mut self, c: &mut ChangeBuilder, l: &FileLayer) -> (LayerId, bool) {
        let existing = c
            .layers()
            .iter()
            .find(|have| have.name.eq_ignore_ascii_case(&l.name))
            .map(|have| (have.id, have.visible));
        let (id, shown) = existing.unwrap_or_else(|| {
            // a layer with no name (some programs allow it) still gets one to show
            let name = if l.name.trim().is_empty() {
                "Unnamed layer"
            } else {
                l.name.as_str()
            };
            let id = c.add_layer(name);
            if l.colour.is_some() || !l.visible {
                let mut layer = c
                    .layers()
                    .iter()
                    .find(|have| have.id == id)
                    .expect("just added")
                    .clone();
                layer.colour = l.colour;
                layer.visible = l.visible;
                c.replace_layer(layer);
            }
            (id, l.visible)
        });
        self.0.push((l.name.clone(), id, shown));
        (id, shown)
    }

    /// The job layer the file's layer `name` became, and whether it is shown; made, shown
    /// and in the usual colour, if the file's own list of layers lacked it.
    pub fn of(&mut self, c: &mut ChangeBuilder, name: &str) -> (LayerId, bool) {
        self.find(name)
            .unwrap_or_else(|| self.add(c, &FileLayer::named(name)))
    }
}
