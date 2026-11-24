//! The Bit Library: the shared tools, in their groups, that toolpaths copy.

use super::{CuttingData, Deserialize, Serialize, Tool, ToolShape};

/// The shared tools. A toolpath copies its tool, so editing the database never changes
/// toolpaths already made.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ToolDatabase {
    pub tools: Vec<Tool>,
    /// The groups the tools sit in, with their notes, in tree order; a group any tool names
    /// is in the tree whether listed here or not (added with tool groups)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub groups: Vec<ToolGroup>,
    /// The materials tools can keep cutting data for, the Material list (added with
    /// materials)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub materials: Vec<String>,
    /// The active material: tools picked from here cut with its data. Empty for each
    /// tool's default data.
    // ponytail: active for the whole database; not kept per job
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub material: String,
    /// Each tool type's default name format, by type (`end_mill`), for tools made new
    /// (added with name formats)
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub name_formats: std::collections::BTreeMap<String, String>,
    /// The groups that are read-only catalogues: bit libraries imported from a maker (a
    /// `.vtdb` or Fusion library), each under [`CATALOGUES`]. Their tools are copied into My Bits to be
    /// edited or loaded in a tool changer. Empty in databases saved before catalogues,
    /// whose tools are all the user's own (added with My Bits)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub catalogues: Vec<String>,
    /// Tools joined into another, by their old name: one bit listed once per material
    /// came in as copies, now one tool with cutting data per material; toolpaths and tool
    /// changers that name a copy find the bit ([`Self::index_of`]). See
    /// [`crate::tool_copies`] (added with one shape per bit)
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub joined: std::collections::BTreeMap<String, crate::tool_copies::JoinedCopy>,
    /// Tools renamed, by their old name: the name each has now. A job saved before the
    /// rename still names the old one; it takes the new name when it opens
    /// ([`Self::renamed_now`]), so the tool changer slot the tool is loaded in is found
    /// (added with renaming in jobs)
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub renamed: std::collections::BTreeMap<String, String>,
}

/// The top-level group the catalogues are imported into, apart from My Bits.
pub const CATALOGUES: &str = "Catalogues";

/// A Bit Library group: `/` between levels, as [`Tool::group`].
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ToolGroup {
    pub path: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub notes: String,
}

impl ToolDatabase {
    /// What a first run starts with: common end mills, ball noses and V-bits.
    pub fn starter() -> Self {
        let end_mill =
            |name: &str, diameter: f64, pass_depth: f64, feed_rate: f64, plunge_rate: f64| Tool {
                name: name.into(),
                diameter,
                pass_depth,
                feed_rate,
                plunge_rate,
                ..Tool::default()
            };
        // a stepover to clear with and a smaller one to finish with, fractions of the
        // diameter: a ball nose roughs wide and finishes close for a smooth surface
        let shaped = |name: &str,
                      shape: ToolShape,
                      diameter: f64,
                      angle: f64,
                      (stepover, finish_stepover): (f64, f64)| Tool {
            shape,
            angle,
            stepover,
            finish_stepover,
            ..end_mill(name, diameter, 3.0, 1500.0, 500.0)
        };
        let mut tools = vec![
            end_mill("3 mm End Mill", 3.0, 1.5, 800.0, 250.0),
            Tool::default(),
            end_mill("12 mm End Mill", 12.0, 4.0, 2000.0, 500.0),
            end_mill("1/8 in End Mill", 3.175, 1.5, 800.0, 250.0),
            end_mill("1/4 in End Mill", 6.35, 3.0, 1000.0, 300.0),
            shaped("3 mm Ball Nose", ToolShape::BallNose, 3.0, 90.0, (0.4, 0.1)),
            shaped("6 mm Ball Nose", ToolShape::BallNose, 6.0, 90.0, (0.4, 0.1)),
            shaped(
                "6 mm Radiused End Mill",
                ToolShape::RadiusedEndMill,
                6.0,
                90.0,
                (0.4, 0.2),
            ),
            // a V-bit's stepover spaces V-carving passes, so it is small
            shaped("90° V-Bit", ToolShape::VBit, 12.7, 90.0, (0.02, 0.02)),
            shaped("60° V-Bit", ToolShape::VBit, 12.7, 60.0, (0.02, 0.02)),
            // 60° for ISO threads; a 1 mm tooth suits pitches up to about 1.5 mm
            Tool {
                tooth_size: 1.0,
                tooth_offset: 2.0,
                ..shaped(
                    "10 mm Thread Mill",
                    ToolShape::ThreadMill,
                    10.0,
                    60.0,
                    (0.1, 0.1),
                )
            },
            // flattens the spoilboard: wide, and stepping over most of its width
            Tool::surfacing_bit(),
            // burns rather than cuts: for the laser toolpaths
            Tool::laser(),
        ];
        for tool in &mut tools {
            let set = if tool.name.contains(" in ") {
                "Imperial Tools"
            } else {
                "Metric Tools"
            };
            tool.group = format!("{set}/{}", shape_group(tool.shape));
        }
        let group = |path: &str, notes: &str| ToolGroup {
            path: path.into(),
            notes: notes.into(),
        };
        Self {
            tools,
            groups: vec![
                group("Metric Tools", "Starter tools sized in millimetres."),
                group("Imperial Tools", "Starter tools sized in inches."),
            ],
            ..Self::default()
        }
    }

    /// A database from before lasers were a tool type, with no laser in My Bits: the
    /// starter laser added (in `Metric Tools/Lasers`), so the laser toolpaths have one to
    /// pick. Whether it was added.
    pub fn add_laser_if_none(&mut self) -> bool {
        if self.my_tools().any(|t| t.shape == ToolShape::Laser) {
            return false;
        }
        let mut laser = Tool::laser();
        while self.tools.iter().any(|t| t.name == laser.name) {
            laser.name.push_str(" 2");
        }
        laser.group = format!("Metric Tools/{}", shape_group(ToolShape::Laser));
        self.tools.push(laser);
        true
    }

    /// Whether `tool` sits in a read-only catalogue (its group is in one of
    /// [`Self::catalogues`]).
    pub fn is_catalogue(&self, tool: &Tool) -> bool {
        self.is_catalogue_group(&tool.group)
    }

    /// Whether the group `path` is a catalogue or inside one.
    pub fn is_catalogue_group(&self, path: &str) -> bool {
        self.catalogues.iter().any(|c| {
            path == c
                || path
                    .strip_prefix(c.as_str())
                    .is_some_and(|rest| rest.starts_with('/'))
        })
    }

    /// My Bits: the user's own tools, the ones toolpaths, tool changers and suggestions
    /// take; every tool outside the catalogues.
    pub fn my_tools(&self) -> impl Iterator<Item = &Tool> {
        self.tools.iter().filter(|t| !self.is_catalogue(t))
    }

    pub fn get(&self, name: &str) -> Option<&Tool> {
        self.index_of(name).map(|i| &self.tools[i])
    }

    /// Keeps that the tool `from` is now called `to`: an earlier rename to `from` now
    /// leads to `to`, and renaming back forgets it.
    pub fn note_renamed(&mut self, from: &str, to: &str) {
        if from == to {
            return;
        }
        for now in self.renamed.values_mut() {
            if now == from {
                to.clone_into(now);
            }
        }
        self.renamed.insert(from.to_owned(), to.to_owned());
        self.renamed.retain(|old, now| old != now);
    }

    /// The name `tool` (a toolpath's copy) has now, when its tool was renamed: only while
    /// no tool has the old name again and the renamed one is still the same size and
    /// shape, so a new bit given the old name is never mistaken for it.
    pub fn renamed_now(&self, tool: &Tool) -> Option<&str> {
        let now = self.renamed.get(&tool.name)?;
        if self.tools.iter().any(|t| t.name == tool.name) {
            return None;
        }
        let renamed = self.tools.iter().find(|t| t.name == *now)?;
        let same = renamed.shape == tool.shape
            && (renamed.diameter - tool.diameter).abs() < 1e-6
            && (renamed.angle - tool.angle).abs() < 1e-6;
        same.then_some(now.as_str())
    }

    /// Where the tool `name` is: in My Bits when it is there, else in a catalogue (a tool
    /// copied in from a catalogue keeps its name, and the copy is the one used).
    pub fn index_of(&self, name: &str) -> Option<usize> {
        let named = |i: &usize| self.tools[*i].name == name;
        let all = 0..self.tools.len();
        all.clone()
            .filter(named)
            .find(|&i| !self.is_catalogue(&self.tools[i]))
            .or_else(|| all.clone().find(named))
            .or_else(|| {
                // a copy joined into its bit: the bit
                let bit = &self.joined.get(name).filter(|j| j.bit != name)?.bit;
                let named = |i: &usize| self.tools[*i].name == *bit;
                all.clone().find(named)
            })
    }

    /// The tool `name` as a toolpath takes it: cutting with the active material's data.
    /// Gives each tool loaded in a tool changer its slot (counted from 1) as its tool
    /// number, so the number shown is the holder it sits in. A tool not loaded keeps its
    /// number unless a loaded tool now has it (it was displaced, or taken out and the slot
    /// refilled): then it has none (0), so Load by Bit Number is never ambiguous.
    /// Whether any number changed.
    pub fn number_by_slots(&mut self, slots: &[Option<String>]) -> bool {
        let slot_of = |name: &str| {
            slots
                .iter()
                .position(|s| s.as_deref() == Some(name))
                .and_then(|i| u32::try_from(i + 1).ok())
        };
        // only My Bits go in a tool changer: a catalogue's tool of the same name is not
        // the one loaded
        let catalogue: Vec<bool> = self.tools.iter().map(|t| self.is_catalogue(t)).collect();
        let taken: Vec<u32> = self
            .tools
            .iter()
            .zip(&catalogue)
            .filter(|(_, c)| !**c)
            .filter_map(|(t, _)| slot_of(&t.name))
            .collect();
        let mut changed = false;
        for (tool, _) in self.tools.iter_mut().zip(&catalogue).filter(|(_, c)| !**c) {
            let number = match slot_of(&tool.name) {
                Some(slot) => slot,
                None if taken.contains(&tool.number) => 0,
                None => tool.number,
            };
            changed |= tool.number != number;
            tool.number = number;
        }
        changed
    }

    pub fn pick(&self, name: &str) -> Option<Tool> {
        // a toolpath naming a bit's per-material copy keeps cutting with that copy's data,
        // whatever the active material, under the name it has
        if let Some(copy) = self.joined_copy(name) {
            let bit = self.get(name)?;
            let mut tool = bit.for_material(&copy.material);
            name.clone_into(&mut tool.name);
            return Some(tool);
        }
        self.get(name).map(|t| t.for_material(&self.material))
    }

    /// As [`Self::pick`], cutting `material` (the active material when empty) with what
    /// the bit has for it: its own feeds, else its class's range placed by the material's
    /// hardness in `library`.
    pub fn pick_in(
        &self,
        name: &str,
        material: &str,
        library: &crate::material_library::MaterialLibrary,
    ) -> Option<Tool> {
        if self.joined_copy(name).is_some() {
            return self.pick(name);
        }
        let material = if material.is_empty() {
            self.material.as_str()
        } else {
            material
        };
        self.get(name).map(|t| t.for_material_in(material, library))
    }

    /// The copy `name` was, when it is a copy joined into a bit and no tool has the name now.
    pub(super) fn joined_copy(&self, name: &str) -> Option<&crate::tool_copies::JoinedCopy> {
        let copy = self.joined.get(name)?;
        (!self.tools.iter().any(|t| t.name == name)).then_some(copy)
    }

    /// Puts back a tool as [`Self::pick`] gave it: with a material active its cutting data
    /// become that material's, and the stored tool's default stays.
    pub fn put_picked(&mut self, mut tool: Tool) {
        if !self.material.is_empty() {
            let data = CuttingData {
                material: self.material.clone(),
                ..tool.cutting("")
            };
            if let Some(stored) = self.get(&tool.name) {
                tool.set_cutting(stored.cutting(""));
            }
            tool.set_cutting(data);
        }
        self.put(tool);
    }

    /// Adds `name` to the material list, unless it is there already (whatever the case).
    pub fn add_material(&mut self, name: &str) {
        let name = name.trim();
        if !name.is_empty() && !self.materials.iter().any(|m| m.eq_ignore_ascii_case(name)) {
            self.materials.push(name.to_owned());
        }
    }

    /// The Duplicate Material: `from` copied as `name` (its own name and " copy" when
    /// none is given; a number added while the name is taken), with every tool's cutting
    /// data for it when `with_data`. The copy becomes the active material; its name.
    pub fn duplicate_material(&mut self, from: &str, name: &str, with_data: bool) -> String {
        let base = if name.trim().is_empty() {
            format!("{from} copy")
        } else {
            name.trim().to_owned()
        };
        let mut copy = base.clone();
        let mut next = 2;
        while self.materials.iter().any(|m| m.eq_ignore_ascii_case(&copy)) {
            copy = format!("{base} {next}");
            next += 1;
        }
        self.materials.push(copy.clone());
        if with_data {
            for tool in &mut self.tools {
                if tool.material_data(from).is_some() {
                    tool.copy_cutting(from, &copy);
                }
            }
        }
        self.material.clone_from(&copy);
        copy
    }

    /// Takes `name` off the material list with every tool's data for it; the
    /// default is active again if it was.
    pub fn delete_material(&mut self, name: &str) {
        self.materials.retain(|m| !m.eq_ignore_ascii_case(name));
        for tool in &mut self.tools {
            tool.materials
                .retain(|m| !m.material.eq_ignore_ascii_case(name));
        }
        if self.material.eq_ignore_ascii_case(name) {
            self.material.clear();
        }
    }

    /// Replaces the tool with the same name, or adds it at the end.
    pub fn put(&mut self, mut tool: Tool) {
        // a joined copy's data goes back as its bit's for the copy's material
        if let Some(copy) = self.joined_copy(&tool.name).cloned() {
            if let Some(i) = self.index_of(&tool.name) {
                self.tools[i].set_cutting(CuttingData {
                    material: copy.material,
                    ..tool.cutting("")
                });
            }
            return;
        }
        match self.index_of(&tool.name) {
            Some(i) if !self.is_catalogue(&self.tools[i]) => self.tools[i] = tool,
            // a catalogue stays as it came: the changed tool joins My Bits
            Some(_) => {
                tool.group = tool.own_group();
                self.tools.push(tool);
            }
            None => self.tools.push(tool),
        }
    }

    pub fn to_json(&self) -> Vec<u8> {
        serde_json::to_vec_pretty(self).expect("tools always serialize")
    }

    pub fn from_json(bytes: &[u8]) -> Result<Self, serde_json::Error> {
        serde_json::from_slice(bytes)
    }
}

/// The group a starter tool of `shape` goes in.
pub fn shape_group(shape: ToolShape) -> &'static str {
    match shape {
        ToolShape::EndMill => "End Mills",
        ToolShape::BallNose => "Ball Nose",
        ToolShape::VBit => "V-Bits",
        ToolShape::RadiusedEndMill => "Radiused End Mills",
        ToolShape::ThreadMill => "Thread Mills",
        ToolShape::FormTool => "Form Tools",
        ToolShape::SurfacingBit => "Surfacing Bits",
        ToolShape::Laser => "Lasers",
    }
}
