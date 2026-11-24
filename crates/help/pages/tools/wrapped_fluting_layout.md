---
reviewed: 2026-11-23
see-also: wrapped_spiral_layout, vector_unwrapper, fluting_toolpath
---

**Wrapped Fluting Layout** (Gadgets menu) lays out flutes along a rotary job's cylinder, evenly
spaced round it, with coves round the ends if you like. It draws vectors on the unrolled surface
for a Profile or Fluting toolpath; it makes no toolpaths itself.

## Steps

![The steps, one after another](pictures/wrapped_fluting_layout/tour.gif)

1. In a rotary job, choose **Gadgets ▸ Wrapped Fluting Layout** and set the **Number of flutes** along the piece.

   ![Step 1](pictures/wrapped_fluting_layout/step-1.png)

2. Set the **Start offset** and **End offset**, and tick **Cove at start** or **Cove at end** for a groove round that end.

   ![Step 2](pictures/wrapped_fluting_layout/step-2.png)

3. Press **Create**. The flutes are laid out as vectors on the unrolled surface, for a Profile or Fluting toolpath.

   ![Step 3](pictures/wrapped_fluting_layout/step-3.png)

## Settings

- **Number of flutes**: how many run along the piece.
- **Start offset** and **End offset**: from each end of the piece to where the flutes start and
  end.
- **Cove at start** and **Cove at end**: a groove round the cylinder there.

It needs a rotary job: see [Job Setup](help:job_setup).
