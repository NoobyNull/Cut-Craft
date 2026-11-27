---
reviewed: 2026-11-27
see-also: preview_detail, rotary_machining_toolpath, rotary_locked_angles_toolpath, rotary_firmware, job_setup
---

# Toolpaths

The Toolpaths page, down the right, turns the selected vectors and the 3D model into
toolpaths, lists them, previews them cutting the material and saves them for the machine.
Pick a toolpath's tool to open its form; choose the cutter from the bit library there. Until you
choose one, a new form starts with a common 6 mm two-flute end mill (6 mm shank, 22 mm of
flute), so it can already say how deep the bit reaches; pick your own bit with Select… so
the sizes and speeds are really yours.

## The bit comes first

Every toolpath form opens with its **Bit**: what cuts decides the rest, as its depth per pass
sets how many passes a depth takes and a V-bit's angle sets how deep it carves. Clearing
bits (Pocket, V-Carve) and 3D Finishing's bits come right under it, then the Cutting Depths
and which side to cut. Rest Machining and the vector selector stay under the depths.

Beside the bit's name is a small drawing of it, to scale: the orange part is what cuts, the
grey part is the shank the collet holds. An end mill shows a flat bottom, a ball nose a round
one, a V-bit its point. Point at the drawing to see it bigger with its flute length and
shank. Dashed lines are sizes the Bit Library has not been given yet; **Edit…** sets them.

A toolpath's form shows its lengths in the job's units, whatever the bit's own units are.
In an inch job, picking a metric bit (a 6 mm end mill, say) leaves Cut Depth, Allowance
Offset, lead lengths and the rest in inches; only the bit's own sizes show as the Bit
Library keeps them. Change the job's units in [Job Setup](help:job_setup).

Reordering the toolpath list (a drag, Move Up or Down, Group), or undoing or redoing that,
leaves each toolpath what it was: a toolpath open in its form stays the one its Calculate
makes again, notes being written in the Toolpath Summary stay with their toolpath, and
what the preview has cut keeps each toolpath's colour. A change made in the list (a
reorder, group, delete, or a tick showing or hiding a toolpath) while a tool's form is open
beside it is kept when that form is cancelled: Cancel takes back only what the form did.

## Pressing Calculate again

A toolpath's form stays open after Calculate, so it is easy to press Calculate a second
time without noticing the toolpath is already made. That never makes a second copy:

1. **Nothing changed** (the same settings, the same vectors picked, and the toolpath not
   edited or out of date since): the toolpath is kept as it is, and the form says
   *Already up to date* under its settings. Nothing is added to Undo.
2. **A setting changed** (a deeper cut, another bit, a new name), or the vectors or the
   material changed so the toolpath is marked ⚠ out of date: Calculate works the same
   toolpath out again **in its place**. It keeps its spot in the cut order and its group in
   the list, and one Undo takes the update back.
3. **Other vectors picked**: Calculate makes a new toolpath for them and keeps the first
   one. With nothing picked, it uses the vectors the toolpath was made from.

To add another toolpath with the same settings on purpose (a second pass, say), press
**Calculate as New**, just above Calculate once a toolpath is made. The form then follows the
new one. A toolpath opened with Edit Toolpath (double-click it in the list) works the same
way: Calculate with nothing changed says *Already up to date*. This is about toolpaths in
the job; what happens to G-code files saved before is set when you save them
([Save Toolpaths](help:save_toolpaths)).

![The Profile form after Calculate was pressed again with nothing changed: Already up to date, and Calculate as New above Calculate](pictures/calculate-again.png)

While a preview plays, its controls sit over the 2D and 3D views: Pause or Run, Single
Step, Run to Retract, Stop, the speed, Progress to drag the bit along the whole job, and
Close up to drag it back and forth over the few minutes either side of where it is. At
Speed ×1 the preview plays as fast as the machine would cut: each move at its bit's feed
or plunge rate, rapids at the Toolpath Summary's rapid rate. A preview starts at ×10, ten
times faster than the machine; the slider goes from ×0.25 to ×1000, and Options sets where
it starts. The line under the controls gives the machine time left, and how long that
takes to watch at the chosen speed. A finishing toolpath with a small bit can take hours,
so speed it up, drag Progress on, or press Stop (or Esc) to keep what is cut so far. Once
stopped or played to the end the controls stay, so you can scrub back over it, until
another preview, Reset Preview, or Close (Undo Last Preview brings back a preview Reset
Preview cleared). New, Open and Close on the File menu clear the
preview, its controls and Undo Last Preview, so nothing of the old job shows in the next. Any
change to the job (an edit, a recalculation, Undo) stops a playing preview where it is and
closes the controls, as its moves are out of date: preview again to play the new ones. A
change in Job Setup to the material's size, thickness or place clears the preview, so the
3D view shows the new block and the next Preview cuts into that. In the 3D view,
double-click a piece the preview has cut right free (a part, or the waste round it) to take
it away and see what is left; Undo Last Preview brings it back. Where nothing is cut free
(uncut material, a pocket, a part held by tabs), even after another piece was taken, a
double-click selects a component or fits the view as usual. Going back starts from the nearest point saved
while it played, so it is quick once played through. A jump is cut across the
cores [Options ▸ Performance](help:performance) allows, and the 3D view keeps its full detail while the preview plays (it is
redrawn a few times a second; the bit moves smoothly in between). If a bit has no shank
diameter set in the Bit Library, the preview draws a common one (1/4" for a V-bit, up
to 1/2" for others) and says so under the controls. The preview cuts a grid of small
squares, as finely as the [Preview detail](help:preview_detail) slider on the Preview tab
says, and only where the toolpaths cut; a bit narrower than two of them (a 1/64" bit with
the slider toward Faster, say)
still cuts every square its tip passes through, but its thin cuts can look broken or
dotted, so a line under the controls says so for as long as the preview plays (or the
status bar after how many were previewed, with Animate Preview off). The line gives the bit's size and the squares', says the toolpath and its G-code are right, and names the slider setting that draws the bit: slide it there and the preview is worked out again when you let go. The Vectors button above the 3D view
(or View ▸ Draw Vectors in 3D View) shows or hides the vectors over the model, and is
remembered next time. Turn off Animate Preview (the
Toolpaths menu, Toolpath Drawing) to cut the whole preview at once, with a progress window
you can Cancel (or press Esc); the status bar then names the toolpath previewed, or says
how many. Either way, Preview All and Preview Visible start again
from uncut material, so old cuts never stay in the result; previewing one toolpath cuts
into what is already previewed. In a double-sided job the preview turns over with Swap
Sides, and back again with its Undo (and over with Redo); a side not previewed yet comes
up uncut, with the side that was up kept underneath.

When you save toolpaths, the post processor writes them as G-code for your machine. A post
with arcs (the Cut Craft GRBL Arcs, Mach3 Arcs and FluidNC ATC Arcs posts) turns runs of
short lines that follow a curve into single G2 (clockwise) and G3 (anticlockwise) arc
moves, so the file is smaller and the machine keeps its speed round curves. Each arc's two
ends are kept the same distance from its centre to well within 0.005 mm, once rounded to
the places the post prints, inch posts included: controllers such as GRBL stop the job
(error 33, invalid arc target) when they are not. Pick a post without arcs (GRBL Straight,
say) only if your controller cannot cut arcs.

Saving again where an earlier save left G-code (the file itself, or the numbered, tile and
side files saved beside it) asks first. Replace deletes all of them, so an old file for a
toolpath you have since removed is not left to run by mistake. Archive moves them into a
dated folder under Archive beside them (a second one saved in the same second gets its own
folder), then adds the new ones. If the new files cannot be saved (no toolpath
shown, say), the earlier ones are put back as they were. Tick Do this every time to
stop being asked; Edit ▸ Options ▸ Confirmation Choices ▸ G-code from an earlier save changes it.
Send to Machine asks the same of an earlier send's files in the project's folder on the
machine (old tile files from another layout too): Replace sends the new files and deletes
the old ones left over; Archive moves them into a dated folder under Archive there first.
A name typed with a dot of its own (sign v1.5) keeps it: the file is sign v1.5.nc.

A drill brought in from a tool library (.vtdb) is kept as a drill: it cuts only straight
down. Its Drill only tick sits beside its Bit Type in the Bit Library, to see or change.
Picked for any toolpath but Drilling, the form says so in red at its foot, and the checks
before saving warn of it too.

A laser is a bit type of its own in the Bit Library (Laser, under Metric Bits ▸
Lasers): its Kerf is the width it burns and its Speed how fast it moves; it has no pass
depth, flutes or spindle. The Laser Cut / Fill and Laser Picture forms start with it and
read it as "Laser, 0.15 mm kerf, 1500 mm/min"; Select… there picks another laser. A router
bit picked on a laser form, or a laser on a router's toolpath, is said in red at the
form's foot. A bit library saved before lasers were a bit type gets the starter laser.

When one file changes tools (a pause to change the bit by hand, or a tool changer), the
first move after the change lifts the new bit straight up to the safe height, and only
then moves across to the next cut. You set Z zero for the new bit during the pause, so
the machine never travels sideways at the height the bit was left at.
If the next toolpath uses the same bit at another spindle speed (another material, say),
the file sets the new speed there, without stopping, so each toolpath cuts at its own rpm.
A bit renamed in the Bit Library keeps its tool changer slot: this job's toolpaths take
the new name at once, and a job saved before the rename takes it when it opens (Undo puts
the old name back), so the file still gives the bit its slot's bit number.
If the Bit Library's or Materials Library's file (tools.json, materials.json) cannot be
read when Cut Craft starts, it is left as it is: changes are kept until Cut Craft closes but
not saved, and each OK says "not saved". Fix or rename that file, then restart. The bits'
feeds edited in the Materials Library live in tools.json, so they are still saved when only
materials.json could not be read.
The Materials Library can stay open while you use the Bit Library: its OK adds only its
own feeds and materials, so bits added, deleted, changed or renamed there meanwhile stay as
they are. In its Add Bit picker, Select keeps the picker's changes at once (a catalogue bit
copied into My Bits, say); the library's own feeds still wait for its OK.
Deleting a whole group in the Bit Library asks first, under the tree, counting its bits,
those Hide Unset Bits hides, those loaded in the tool changer and the job's toolpaths that
cut with them; Cancel still brings them back until OK.
Its Materials Library… button keeps your changes first, as Apply does; when they delete bits
or materials it asks first, as Cancel could not bring those back afterwards (a bit only
renamed is not counted).

Which material a toolpath's feeds are for: name the material on each layer (Layer Properties
▸ Material) and in [Job Setup](help:job_setup) ▸ Material. A toolpath cuts with its bit's
feeds for the hardest material its vectors are on. A vector on a layer that names no material
counts as the hardest class (Hard Metals), even when Job Setup names one, so a forgotten
layer cuts slowly rather than too fast. A toolpath over no vectors (over the 3D model) uses
Job Setup's material, or the hardest class if that is empty. A job where no layer or sheet
names a material keeps cutting with the Bit Library's active material, as before.

Toolpaths ▸ Clearing Settings (or Clearing Settings… in a Pocket or Profile form's Cutting
window) sets how the light-cut clearing styles cut, for the whole job. It is saved with the
job, so the job cuts the same on any computer, whatever that computer's own defaults are;
Use as My Default makes new jobs start with them, and Reset to My Defaults puts this job
back. Light Step is how wide a bite Adaptive clearing takes and how far each trochoidal
loop moves on, as a share of the bit's diameter: the same share gives every bit the same
gentle bite B (10% keeps about 37° of the edge in the wood at once; 5 to 15% suits most
work). The window draws the bite, the loops at your bit's size, and the rows. On the Way
Back chooses Keep climb (every raster row cuts climb, the bit coming back through the row
it just cut, and loops on a pass back keep turning the same way, so they cut climb too) or
Alternate (rows back and forth, climb then conventional, and loops turning the other way
on a pass back: quicker, with a rougher wall). Changing them is an edit you can undo, and
marks the toolpaths they shape as needing to be calculated again.

![Light Step: the bit (Dia across) bites B into the wall as it goes along; B is the Light Step's share of Dia](bytes://icons/diagrams/light_step.png)

A raster pocket's rows step over to the next along the wall without lifting, so the bit
goes down into the material once per pass instead of at the start of every row.

Pocket no longer offers trochoidal loops: for a light bite at full depth, pick Adaptive
under Clear Pocket, which follows the pocket's shape and takes the Light Step. Loops stay
on Profile, for cutting along a line. A pocket made in loops before keeps them, and its
Cutting window still lets you change it back to straight.

Adaptive sweeps a narrow channel (up to about four bit widths wide) in one trip from end to
end, in D-shaped loops: the round front of each cuts from wall to wall, a Light Step further
along than the last, keeping the bit in the wood at the same light bite, then it comes
straight back across (through what the loops before cut, without lifting) to the next.
The loops shrink where the channel narrows. The bit goes down once per channel, and the
round fronts post as arcs instead of many short lines. The rest of the pocket, including
each channel's ends and corners, is cleared the usual adaptive way. A tip narrower than
the bit can't be reached by it: clear it with a smaller bit or a V-bit.

![A narrow channel from above: down in the middle, out in a spiral, then D-shaped loops a Light Step apart from wall to wall](bytes://icons/diagrams/adaptive_channel.png)

Adaptive works out every step the bit takes, so a big pocket can take a little while to
calculate. The progress window says which step it is on (finding the narrow channels,
sweeping each one, clearing the wider areas), how far it has got and about how long is
left ([Long jobs and the time left](help:progress)); big open areas take longer than the
rest. Cancel stops it and leaves the toolpath as it was.

![Leave for Edge Pass: the clearing stops A short of the wall, then the bit goes round once at the full depth D and skims A off](bytes://icons/diagrams/edge_pass.png)

Pocket's Clear Pocket section has two settings for straighter walls. Leave for Edge Pass
(A in the picture) stops the clearing that far short of the walls, so the pass round the edge takes one thin,
even skim instead of whatever the clearing left (0 clears right up to them). Edge Pass at
Full Depth goes round the edge once, at the bottom, after all the clearing passes: one
clean wall with no lines where the passes met. It needs a bit whose flutes reach the full
depth; with shorter flutes (from the Bit Library) the edge is cut at each pass instead.
Raster and Adaptive pockets can also leave the edge pass out (Profile Pass: None); offset
rings always end on the edge. Only Raster offers First, round the edge before the rows:
Adaptive always goes round the edge last, so switching a pocket set to First over to Adaptive
picks Last.

## Flat cuts on a rotary job

On a rotary job, a flat toolpath (Profile, Pocket, Drilling, V-Carve and the others) is
usually wrapped round the part: A turns as the bit moves round, so a line drawn round the sheet
is cut round the material. To cut a flat face instead, such as a pocket on one side of a
square bar or a flat spot on a dowel, tick **Flat on a locked face** at the foot of the
toolpath's form.

1. Draw the vectors on the sheet where the face is: the middle of what they cut sets the angle
   A turns to.
2. Tick **Flat on a locked face** and press **Calculate**.

A turns to that angle and stays there. The bit moves across the face instead of A turning,
and the depth is measured down from the face, so a pocket 3 mm deep is 3 mm deep all across
it. On a square or a board, A turns to the nearest flat face. On a round blank, the face is
the flat touching the top of the material, so it cuts deepest in the middle. Tick **Set the
angle myself** to type the angle in instead.

The bit is held to one side of the turning axis, so this needs a post that moves the bit
across the axis: the **Rotary A** posts do.

## Previewing a rotary job

On a rotary job the animated preview plays your toolpaths the way the machine will run
them. Cut Craft writes each toolpath through the machine's own post processor (the one Save
Toolpaths uses), reads the file back move by move, and cuts the material
[Job Setup](help:job_setup) describes (round, square or a flat board) with exactly those
moves. The 3D view shows the material turning with A while the bit moves along, across and
up and down over it, as on the machine, so what you see cut is what the file cuts.

![The preview of a rotary job paused part way: a star's outline already cut round a dowel that has turned with A, and the bit cutting a pocket further along](pictures/rotary-turning-preview.png)

1. Set the job up as rotary in Job Setup, with the material's shape.
2. Pick a machine with a 4th axis (A) and a Rotary A post in Machine Configuration (see
   [The 4th axis and your firmware](help:rotary_firmware)). With no machine picked, Cut Craft's
   own rotary post is used.
3. Turn on Animate Preview (Toolpaths menu ▸ Toolpath Drawing) and preview the toolpaths.
4. Run, Pause, Single Step and the Progress slider move the turning material too: drag
   back to see what had been cut by then, and on again.

While it plays, the view is always drawn round the axis, even with Auto-Wrapping View off.
Each move takes as long as the file's feed says (most rotary posts write feeds in inverse
time, G93), and the turning material keeps to the same point through the job as the
Progress slider. Previewing one toolpath after others carries on from what those cut. The
material is cut as small cells and drawn smoothed. How small follows the Performance ◀ ▶
Detail slider on the Preview tab: in the middle, about a third of a millimetre on a 300 mm by
80 mm blank, fine enough to show a finish's passes. If the preview looks striped or stepped,
move the slider towards Detail and preview again; towards Performance it plays faster. If the job can't be saved for the active machine (a machine
on standard GRBL has no A axis, say), the preview plays unrolled as before, and Save
Toolpaths says why.

## Checks before cutting (the Analyse tab)

The Analyse tab, and Save Toolpaths before it writes a file, checks the shown toolpaths
before you cut. Every check is listed by name, with how it came out: passed, ⚠ to read
and tick, ✖ must fix, or not checked (and why). Press a check's **?** for its own page.
See [Checks before cutting](help:checks_before_cutting) for every check, what it protects
you from, and how to fix what it finds, and [Collet and Holder Collisions](help:collisions)
for the collet nut, spindle or shank hitting wood beside a deep cut.

The checks cut every toolpath in order, so a big 3D finish (millions of tiny moves with a
small ball nose) takes a little while: a few seconds on a fast computer, longer on a slow
one. They run in the background with a progress window that says which step they are on,
and Cut Craft keeps drawing while they work. Cancel (or Esc) stops them; the tab then says
nothing has been checked yet, with a Check Again button, and Save Toolpaths and Send to
Machine wait until the checks have run. Each ⚠ the checks find is ticked to say you have
read it; Save Toolpaths and Send to Machine wait, greyed, until every one is ticked, and the
save then goes on without asking again. While the save asks
its questions and you name the file, the Save Toolpaths window steps aside so nothing hides
behind it, and comes back afterwards. They run again only when a toolpath, the job or
the machine changes, so going back to the tab is instant. See also
[Save Toolpaths](help:save_toolpaths) and [Toolpath Summary](help:toolpath_summary).

## Looking at the G-code (View G-code)

Sometimes you only want to read the G-code, not save it: to check a feed rate, see where the
bit changes are, or copy a few lines into a forum post. **View G-code…** on the Output tab
(beside Machine Files), and in the Save Toolpaths window, shows the G-code saving would
write, in a window of its own, without saving a file.

![The View G-code window over the Output tab: the post-processor, what the checks before cutting found, the file, its lines numbered, and Copy All, Save Toolpaths… and Refresh](pictures/view-gcode.png)

1. Make your toolpaths and tick the ones you want in the toolpath list.
2. Open the **Output** tab and press **View G-code…**.
3. The window says which post-processor made it (the machine's default, or the one picked
   in Save Toolpaths), the file's name and how many lines it has. Each line is numbered.
4. Above the G-code, the [checks before cutting](help:checks_before_cutting) list what they
   found, the same checks Save Toolpaths runs: **✖ Must fix** lines in red, **⚠** warnings
   in amber, or a green line when they found nothing. While they run, a spinner says so.
   Nothing is stopped here: you decide what to change, and the G-code stays there to read.
5. When saving would make several files (one per bit, or one per tile), pick which to read
   from **File**.
6. **Copy All** copies the file's G-code to paste somewhere else. **Save Toolpaths…** opens
   Save Toolpaths with the same choices, to check the toolpaths and save the files.
   **Refresh** makes the G-code again after you change a toolpath.

Nothing is saved until you save. See also [Save Toolpaths](help:save_toolpaths) and
[Checks before cutting](help:checks_before_cutting).
