//! The selection: a set of entities that remembers the order they were picked in, so tools
//! can use "the last selected" (align to it, space between first and last, profiles).

use crate::document::EntityId;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Picked entities, each once, first picked first. Saved as a list, as the set it replaced was.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Selection(Vec<EntityId>);

impl Selection {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn contains(&self, id: &EntityId) -> bool {
        self.0.contains(id)
    }

    /// In the order they were picked.
    pub fn iter(&self) -> std::slice::Iter<'_, EntityId> {
        self.0.iter()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// The one picked last.
    pub fn last(&self) -> Option<EntityId> {
        self.0.last().copied()
    }

    /// Adds `id` as the last picked; false if it was already there.
    pub fn insert(&mut self, id: EntityId) -> bool {
        if self.contains(&id) {
            return false;
        }
        self.0.push(id);
        true
    }

    /// Takes `id` out; false if it was not there.
    pub fn remove(&mut self, id: &EntityId) -> bool {
        let before = self.0.len();
        self.0.retain(|i| i != id);
        self.0.len() != before
    }

    pub fn retain(&mut self, keep: impl FnMut(&EntityId) -> bool) {
        self.0.retain(keep);
    }

    pub fn clear(&mut self) {
        self.0.clear();
    }
}

impl FromIterator<EntityId> for Selection {
    /// Each id once, in the order given.
    fn from_iter<I: IntoIterator<Item = EntityId>>(ids: I) -> Self {
        let mut s = Self::new();
        for id in ids {
            s.insert(id);
        }
        s
    }
}

impl<const N: usize> From<[EntityId; N]> for Selection {
    fn from(ids: [EntityId; N]) -> Self {
        ids.into_iter().collect()
    }
}

impl Extend<EntityId> for Selection {
    fn extend<I: IntoIterator<Item = EntityId>>(&mut self, ids: I) {
        for id in ids {
            self.insert(id);
        }
    }
}

impl IntoIterator for Selection {
    type Item = EntityId;
    type IntoIter = std::vec::IntoIter<EntityId>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a> IntoIterator for &'a Selection {
    type Item = &'a EntityId;
    type IntoIter = std::slice::Iter<'a, EntityId>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl Serialize for Selection {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        self.0.serialize(s)
    }
}

impl<'de> Deserialize<'de> for Selection {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        Ok(Vec::<EntityId>::deserialize(d)?.into_iter().collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_pick_order_each_id_once() {
        let mut s: Selection = [EntityId(3), EntityId(1), EntityId(3)].into();
        assert_eq!(
            s.iter().copied().collect::<Vec<_>>(),
            [EntityId(3), EntityId(1)]
        );
        assert!(s.insert(EntityId(2)));
        assert_eq!(s.last(), Some(EntityId(2)));
        assert!(s.remove(&EntityId(3)));
        assert!(!s.contains(&EntityId(3)));
    }

    #[test]
    fn saves_as_a_list_and_reads_the_old_sets() {
        let s: Selection = [EntityId(5), EntityId(2)].into();
        let json = serde_json::to_string(&s).unwrap();
        assert_eq!(json, "[5,2]");
        let back: Selection = serde_json::from_str("[1,2,2]").unwrap();
        assert_eq!(back.len(), 2);
    }
}
