---
reviewed: 2026-11-24
see-also: material_setup, profile_toolpath, collisions
---

## Steps

![The steps, one after another](pictures/keep_out_zones/tour.gif)

1. A keep-out zone is an area the bit must never cross, such as a clamp. Draw round each clamp and select the shapes.

   ![Step 1](pictures/keep_out_zones/step-1.png)

2. On the Toolpaths page, press **Keep-Out Zones** (under Job and Tools).

   ![Step 2](pictures/keep_out_zones/step-2.png)

3. Set the **Clearance**: how far beyond the shapes the zone reaches, so the side of the bit stays clear too.

   ![Step 3](pictures/keep_out_zones/step-3.png)

4. Set the **Clamp Height**: how tall the clamps stand above the top of the material (measure to the highest point; a screw head counts). The bit already keeps out of the zones, but the wider collet nut and spindle can come down over a clamp beside a deep cut; with the height set, the checks warn of it. See [Collet and Holder Collisions](help:collisions).

5. Give the zones a **Name** (they are labelled Clamp 1, Clamp 2…), then press **OK**. A toolpath that would cut into a zone is not made.

   ![Naming the zones](pictures/keep_out_zones/step-4.png)
