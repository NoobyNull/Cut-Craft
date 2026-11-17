---
reviewed: 2026-11-17
see-also: create_component_from_bitmap, emboss, photo_vcarve_toolpath
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

If it says the add-on **is not available to download yet**, nothing is wrong on your
computer: it simply has not been published for your kind of computer yet. Try again after
the next Cut Craft update.

You do not need the add-on if your picture is already a depth map (a grey picture where
white is nearest). Set **Depth from** to **Picture**, and the picture is used as the heights
directly. Tick **Invert** if its black is nearest instead. A ready-made depth map can also
go straight into [Create Component from Bitmap](help:create_component_from_bitmap).
