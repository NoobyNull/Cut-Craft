---
reviewed: 2026-11-17
see-also: add_layer, set_current_layer, show_hide
---

Change a layer's **Name**, whether it is **Visible**, and whether it is **Locked** (drawn, but
its vectors can't be picked or changed).

**Material** names what this layer's vectors are cut from, from the Materials Library. A
toolpath picks its bit's feeds for the hardest material its vectors are on. Leave it empty
and the layer counts as the hardest class (the slowest, safest feeds), even when
[Job Setup](help:job_setup) names a material, so name it on every layer you cut. See
[Toolpaths](help:toolpaths).

