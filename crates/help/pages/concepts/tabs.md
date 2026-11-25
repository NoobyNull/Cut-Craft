---
reviewed: 2026-11-22
see-also: profile_toolpath, nest_parts, tool_database
---

# Tabs

Tabs (also called bridges or hold-downs) are small bits of material a
[Profile](help:profile_toolpath) leaves uncut round a part. When the bit cuts all the way
through, they keep the part joined to the sheet, so it can't come loose, spin into the bit
or fly off. Once the job is done, snap or cut the part free and sand the tabs flush.

Use tabs whenever a profile cuts **through** the material and the part isn't held down
another way (screws, tape or a vacuum table).

## Adding tabs

![Add tabs to toolpath, on the Profile form](pictures/profile_toolpath/step-5.png)

1. Open the [Profile Toolpath](help:profile_toolpath) form and tick **Add tabs to
   toolpath**.
2. Press **Edit Tabs…**. The Edit Tabs window opens over the form.
3. Pick how many and where:
   - **Automatic** (a safe start): about one every **Distance apart**, 3 or more on a shape,
     but never more than a third of it in tabs, so a small shape gets 1 or 2.
   - **Number**: this many on every shape, spread along it.
   - **Distance apart**: one about every this far along each shape, with **At least** and
     **At most** per shape.
   - **Where clicked**: press **Add Tabs**, then click on the vectors in the 2D view to add
     a tab, click one to take it off, or drag one to move it.
4. Leave **Not on corners** ticked so tabs sit on straight or gently curved parts, where
   they are easier to clean up.
5. Press **Calculate** in the Profile form. The preview shows each tab as a bump left
   standing in the cut.

## Size

- **Size from the bit** (on to start): each tab is as thick as one pass of the bit (at most
  half the cut) and twice as long as it is thick. A 1/4" bit taking 1/4" passes leaves tabs
  1/2" long and 1/4" thick.
- Turn it off to type a **Length** (along the path) and **Thickness** (up from the bottom
  of the cut) yourself. Thin tabs are easier to break; thick ones hold small parts better.
- **2D Tabs** are flat-topped blocks. **3D Tabs** ramp up and down, which is gentler on the
  bit and leaves less to clean up.

Tabs that are too few or too thin for a heavy part can let it shift on the last pass. If in
doubt, use more tabs rather than thicker ones.
