---
reviewed: 2026-11-20
see-also: create_component_from_bitmap, emboss, photo_vcarve_toolpath, addons
---

## Steps

![The steps, one after another](pictures/relief_from_photo/tour.gif)

1. Relief from Photo shapes a photo into a carved relief, with the background cut away. It needs the Depth add-on to work out the depth.

   ![Step 1](pictures/relief_from_photo/step-1.png)

2. On the Design page, under **Model**, press **Relief from Photo**.

   ![Step 2](pictures/relief_from_photo/step-2.png)

3. Choose how high the subject stands and how much detail to keep, then press **Create**.

   ![Step 3](pictures/relief_from_photo/step-3.png)

## The Depth add-on

The first time, the form shows **Install Depth Add-on**. Press it once: it downloads the
free add-on (about 100 MB) from GitHub and keeps it for next time. After that, choose the
photo and press **Estimate Depth**.

Once the add-on is installed, Relief from Photo is also in **Gadgets ▸ My Gadgets** (as
**Depth (Relief from Photo)**) and in the **Gadgets** section of the tool buttons. To update
or remove the add-on, use [Add-ons](help:addons).

If it says the add-on **is not available to download yet**, nothing is wrong on your
computer: it simply has not been published for your kind of computer yet. Try again after
the next Cut Craft update.

### High detail

Once the add-on is installed, the form offers **Get High Detail Model**. It is a second,
bigger model (about 670 MB, downloaded once) that sees finer detail, such as hair, folds in
cloth and lettering. After it arrives a **High detail** tick box appears, ticked. Untick it
to go back to the standard model.

Two things to know before you get it:

- It is slower: an estimate takes several times longer, and needs about 4 GB of free
  memory.
- Its licence allows **personal, non-commercial use only**. Do not use it for pieces you
  sell. The standard model has no such limit.

You do not need the add-on if your picture is already a depth map (a grey picture where
white is nearest). Set **Depth from** to **Picture**, and the picture is used as the heights
directly. Tick **Invert** if its black is nearest instead. A ready-made depth map can also
go straight into [Create Component from Bitmap](help:create_component_from_bitmap).
