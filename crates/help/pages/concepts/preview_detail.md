---
reviewed: 2026-11-25
see-also: toolpaths, view_3d, settings, performance
---

# Preview detail

The toolpath preview works out what your bits cut on a grid of points, seen from above.
The closer together the points are, the truer small corners, holes and thin walls look.
The **Preview detail** slider sets how close they are.

![The same corner and hole at 0.5 mm between points (stepped) and at 0.1 mm (smooth)](pictures/concepts/preview_detail.png)

Only the picture changes. The [toolpaths](help:toolpaths) and the G-code they save are the
same whatever the slider says: a stepped curve in the preview is still cut as a smooth arc.

## Using the slider

1. Open the **Toolpaths** page and its **Preview** tab. The slider is under **Preview All
   Toolpaths** and **Reset Preview**, with **Faster** on the left and **More detail** on
   the right.
2. Drag it. Nothing changes while you drag.
3. Let go. A preview that is showing is worked out again at the new detail, in the
   background, and you can keep working meanwhile.

Hold the pointer over the slider to see what it means for this job: about how far apart
the points are, the area they cover and how much memory they take.

| Slider | Between points | Good for |
| --- | --- | --- |
| 0% (Faster) | about 0.5 mm | a quick look, a slower computer |
| 50% (the default) | about 0.1 mm | most jobs |
| 100% (More detail) | about 0.025 mm | tiny bits, fine carving, checking parts that fit together |

## Where the points go

The points go where your toolpaths cut, with a margin round them, not across the whole
sheet. A small part in the corner of a big sheet gets as much detail as it would on a
sheet of its own size. The rest of the sheet is uncut, so it is drawn flat round the
preview.

If the toolpaths cover a big area, the slider also limits how many points there are, so
the preview stays quick. Then a red note under the slider says the preview is drawn
coarser than the slider asks: slide toward **More detail** if your computer can take it.
If the graphics card itself can take no more, the note says so; that is the most this
computer can show.

A 3D toolpath set to **Material Boundary** works over the whole block, so its preview
covers the whole sheet and its points are spread over all of it. Set the toolpath to
**Model Boundary** to keep the preview, and its detail, to your model.

## Zooming in and out

The preview is kept on the graphics card in tiles, at its full detail and at a half and
a quarter of it. The 3D view draws the copy whose points are about a pixel apart on your
screen, and only the tiles you can see:

- **Zoomed out**, it draws a quarter of the points over the whole preview, so turning
  and moving the view stays smooth. Nothing finer is lost: a fine groove still shows,
  as the deepest point of each group is the one kept.
- **Zoomed in**, it draws the full detail of the part you are looking at.
- Zooming out never throws the fine tiles away. They stay on the card, up to an amount
  of memory the slider sets (more toward **More detail**); past that, the tiles you
  haven't looked at for longest are let go first, and made again when you look back.

All of it is let go when you press **Reset Preview**, make a new preview, or close PowerWhittle.

## Now and the default

- The slider on the Preview tab is the detail **now**. It doesn't change the default.
- **Edit ▸ Options ▸ Performance ▸ Preview detail** holds the **default**: what the slider
  starts at when PowerWhittle opens. Changing it waits for **Apply**, as all
  [settings](help:settings) do.
- **Reset Preview** puts the slider back to the default.
- **Toolpaths ▸ Preview Simulation Quality** has quick picks that move the slider:
  Standard 20%, High 40%, Very High 60%, Extremely High 80% and Maximum 100%. The tick
  shows which one the slider is at, or none when it is in between.

A rotary or double-sided job's preview covers the whole block. Those, and any preview on a
computer whose graphics card can't run the 3D view's shaders, are drawn at the 3D view's
own detail.

The slider changes the toolpath preview only. Your model in the [3D view](help:view_3d)
keeps the detail set in Options ▸ 3D View.

## Small bits

A bit narrower than two of the preview's squares (a 1/64" pencil bit, say) can show
broken or dotted cuts, though the toolpath is right. A line under the preview says so and
names the slider setting that draws it truly.
