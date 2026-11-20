---
reviewed: 2026-11-20
see-also: settings, materials, view_3d, interface_layout
icon: common/materials_appearance.png
---

# Colors on screen

The 2D view draws your vectors, the toolpaths made from them and the bit's rapid moves each
in its own color. If you find them hard to tell apart (with color blindness, say, or on a
dim screen in the workshop), pick other colors.

![The same profile toolpath in the dark theme, in the standard colors (left) and the color-blind friendly ones (right)](pictures/line-colors-dark.png)

## Changing them

1. Open **Edit ▸ Options ▸ Appearance**.
2. Pick one under **Vector and toolpath colors**:
   - **Standard**: vectors in the text color, toolpaths orange, rapid moves red, as Cut
     Craft has always drawn them.
   - **Color-blind friendly**: vectors black (white in the dark theme), toolpaths deep
     orange, rapid moves pink. They differ in how light or dark they are, not only in hue,
     so they stay apart for every kind of color blindness and in grayscale.
   - **My own colors**: a **Vector color**, **Toolpath color** and **Rapid move color** to
     pick for the light theme and for the dark theme, under **2D view in the light theme**
     and **2D view in the dark theme**. Pick a toolpath color lighter or darker than the
     vector color as well as another hue.
3. Press **Apply** to see them. Before **Apply**, **Cancel** throws your changes away; after
   it, the colors stay until you change them again or press **Reset to defaults** (see
   [Changing settings](help:settings)).

![The same in the light theme](pictures/line-colors-light.png)

## What stays the same

- The 3D view draws vectors in the same vector color.
- What you have selected is always blue, whichever colors you choose.
- Solid previews (**Toolpaths ▸ Toolpath Drawing ▸ Make 2D Previews Solid**) take the
  **2D solid preview color** in **Options ▸ Toolpath Settings** instead.
- The 3D preview's colors (the wood, the cut) are in **Toolpaths ▸ Preview Colors**: see
  [Materials](help:materials).

**Reset to defaults** at the foot of the Appearance page puts every color back.
**Appearance ▸ Theme** picks a **Light** or **Dark** window, or **System** to follow your
desktop.
