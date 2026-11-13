# Toolpaths

The Toolpaths page, down the right, turns the selected vectors and the 3D model into
toolpaths, lists them, previews them cutting the material and saves them for the machine.
Pick a toolpath's tool to open its form; choose the cutter from the tool database there.

While a preview plays, its controls sit over the 2D and 3D views: Pause or Run, Single
Step, Run to Retract, Stop, the speed, Progress to drag the tool along the whole job, and
Close up to drag it back and forth over the few minutes either side of where it is. At
Speed ×1 the preview plays as fast as the machine would cut: each move at its tool's feed
or plunge rate, rapids at the Toolpath Summary's rapid rate. A preview starts at ×10, ten
times faster than the machine; the slider goes from ×0.25 to ×1000, and Options sets where
it starts. The line under the controls gives the machine time left, and how long that
takes to watch at the chosen speed. A finishing toolpath with a small bit can take hours,
so speed it up, drag Progress on, or press Stop (or Esc) to keep what is cut so far. Once
stopped or played to the end the controls stay, so you can scrub back over it, until
another preview, Reset Preview, or Close. Going back starts from the nearest point saved
while it played, so it is quick once played through. A jump is cut across all the
computer's cores, and the 3D view keeps its full detail while the preview plays (it is
redrawn a few times a second; the tool moves smoothly in between). If a bit has no shank
diameter set in the Tool Database, the preview draws a common one (1/4" for a V-bit, up
to 1/2" for others) and says so under the controls. The Vectors button above the 3D view
(or View ▸ Draw Vectors in 3D View) shows or hides the vectors over the model, and is
remembered next time. Turn off Animate Preview (the
Toolpaths menu, Toolpath Drawing) to cut the whole preview at once, with a progress window
you can Cancel (or press Esc).
