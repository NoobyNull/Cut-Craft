
## What it cuts

A round bit can't get right into a valley or an inside corner that is tighter than itself:
after 3D Finishing with a 6 mm ball, every place two curves meet, or a wall meets the floor,
is left rounded (red below). The 3D Pencil toolpath runs a smaller ball nose, the pencil
bit, along just those valleys, so they come out crisp. It cuts nothing anywhere else, so it
takes minutes, not the hours a second full finish with a small bit would.

![Left: after 3D Finishing with a 6 mm ball, the valleys left rounded in red. Middle: the 3D Pencil toolpath along them. Right: after the pencil pass, only a thin line left](bytes://help/pencil-before-after.png)

![The pencil passes cutting the valleys the finishing bit left, then the job after them](bytes://help/pencil-pass.gif)

## Step by step

1. Make your [3D Finishing Toolpath](help:3d_finishing_toolpath) first, with the bigger
   ball nose.
2. Open **3D Pencil Toolpath**. Under **Tool**, press **Select…** and pick a ball nose
   about half the size of the finishing bit (a 3 mm ball after a 6 mm one, or 1/16" after
   1/8").
3. Under **Bits that finished before**, make sure the finishing bit is listed (the
   starter 6 mm ball is there to begin with: **Select…** to change it). The pencil bit then
   cuts only where that bit left material.
4. Leave **Valleys sharper than** at 20° and **Extra passes each side** at 3 to start
   with, and press **Calculate**.
5. Preview it after the finishing toolpath: the valleys should come out sharp.

## Good to know

- **Extra passes** run beside each valley line, a stepover apart (the pencil bit's own
  finishing stepover from the Tool Database), and stop where the finishing bit already
  cut. More passes clean a wider strip; 0 cuts the valley line alone.
- With nothing under **Bits that finished before**, the pencil bit runs along every valley
  of the model sharper than the angle, even where nothing was left. That is slower, but
  fine on its own as a quick corner tidy.
- A warning in red at the foot of the form, never a refusal, says when something looks
  wrong: a pencil bit no smaller than the finishing bit, a bit that is not a ball nose, or
  no valleys found (lower the angle, or pick a smaller pencil bit).
- **Previewing a tiny pencil bit:** the preview carves a grid of small squares, about half
  a millimetre across on a big job at Standard quality. A 1/64" (0.4 mm) bit is finer than
  that, so its cuts can look broken or not show at all, and a ⚠ line under the preview
  says so. The toolpath and its G-code are right. The line names the Toolpaths ▸ Preview
  Simulation Quality to pick to see them; preview again after.
- Walls that stand straight up keep a little fillet the size of the pencil bit's own
  radius. Pick a smaller pencil bit for a sharper corner.
- Programs can make it through the [Web API](help:web_api), as `3d_pencil_toolpath`.

Pencil passes are a feature of high-end CAM programs; Cut Craft finds the valleys with
its own method.
