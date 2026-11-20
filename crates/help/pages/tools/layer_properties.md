---
reviewed: 2026-11-20
see-also: add_layer, set_current_layer, show_hide, import_vectors
---

Change a layer's **Name**, whether it is **Visible**, and whether it is **Locked** (drawn, but
its vectors can't be picked or changed).

**Material** names what this layer's vectors are cut from, from the Materials Library. A
toolpath picks its bit's feeds for the hardest material its vectors are on. Leave it empty
and the layer counts as the hardest class (the slowest, safest feeds), even when
[Job Setup](help:job_setup) names a material, so name it on every layer you cut. See
[Toolpaths](help:toolpaths).

**Own Colour** draws this layer's vectors in a colour of their own, picked in **Colour**
below, so you can tell the layers apart at a glance in the 2D and 3D views. Untick it and
they go back to the usual vector colour (set in Options under Appearance). The colour only
changes how the vectors look: it never changes how they are cut.

To change it:

1. In the **Layers** tab, click the small coloured square before the layer's name (or
   double-click the name).
2. Tick **Own Colour**.
3. Click **Colour** and pick one.
4. Press **Apply**.

A drawing brought in from a DXF keeps the colours its layers had: see
[Import Vectors](help:import_vectors).
