//! What the 3D view keeps of what it draws, so a frame that changes nothing makes nothing
//! again: the meshes and surfaces made for the CPU and the GPU, each with what it was made
//! from.

use super::{FineKey, Material, ModelGpu, ModelKey, RoughKey, SurfaceGpu, TurnedModels, view3d};
use std::sync::Arc;

/// A value kept with what it was made from: it holds while asked for with the same key.
pub(super) struct Kept<K, V>(Option<(K, V)>);

impl<K, V> Default for Kept<K, V> {
    fn default() -> Self {
        Self(None)
    }
}

impl<K: PartialEq, V> Kept<K, V> {
    /// The value kept for `key`, if it was made from that.
    pub(super) fn get(&self, key: &K) -> Option<&V> {
        self.0
            .as_ref()
            .filter(|(kept, _)| kept == key)
            .map(|(_, v)| v)
    }

    /// `value`, made from `key`, kept in place of what was.
    pub(super) fn keep(&mut self, key: K, value: V) -> &V {
        &self.0.insert((key, value)).1
    }

    /// What the kept value was made from.
    #[cfg(test)]
    pub(super) fn key(&self) -> Option<&K> {
        self.0.as_ref().map(|(key, _)| key)
    }

    /// The value kept, whatever it was made from.
    #[cfg(test)]
    pub(super) fn value(&self) -> Option<&V> {
        self.0.as_ref().map(|(_, value)| value)
    }

    /// Nothing kept: the next ask makes it again.
    pub(super) fn clear(&mut self) {
        self.0 = None;
    }
}

#[derive(Default)]
pub(crate) struct Meshes3d {
    /// The model alone at the 3D view's moving detail (picking, handles, the quick mesh):
    /// made once a change, not every frame.
    pub(super) rough: Kept<RoughKey, Material>,
    /// A rotary job's full 3D models as the view draws them round the axis, kept by the
    /// change and the import they show.
    pub(super) turned: Option<TurnedModels>,
    /// The 3D view's fine mesh once it is still, with what it was made for.
    pub(super) fine: Kept<FineKey, egui::Mesh>,
    /// The model at the fine mesh's detail, for drawing in the 3D view to hit what is drawn,
    /// kept by the history revision, job and detail when still (mesh cells) it was made for.
    pub(super) drawn_on: Kept<(u64, cut_craft_core::document::Job, usize), Arc<Material>>,
    /// The imported model's triangles as last drawn, kept while nothing about the view or
    /// the model changes: sorting and lighting a big model every frame is what made the
    /// window drag while a model was being imported.
    pub(super) model: Kept<ModelKey, Arc<egui::Mesh>>,
    /// The 3D view's surface as the GPU draws it, with what it was built from.
    pub(super) surface_gpu: Option<SurfaceGpu>,
    /// The model's walls and bottom, made with the surface they hang from (no preview).
    pub(super) solid_gpu: Option<Arc<view3d::gpu::Geometry>>,
    /// The imported model as the GPU draws it.
    pub(super) model_gpu: Option<ModelGpu>,
    /// The same for the imported model's parts past the material's edges.
    pub(super) ghost_gpu: Option<ModelGpu>,
}

#[cfg(test)]
mod tests;
