# Machine packages

Hobby CNC routers to add to Cut Craft in one go: **Machine ▸ Install Offline Package...**
and pick one of these files (download it from here first). **Machine ▸ Search for Machine
Online...** opens this folder.

Each file is a list of machines, written the same way as Cut Craft's own `machines.json`:
the name, maker, model, controller, working area (`width` × `height`, in mm) and the posts
it saves toolpaths through. Installing adds every machine you don't already have one of the
same name as; the rest are left alone.

Each machine comes with four posts, **Cut Craft GRBL Arcs** and **GRBL Straight** (lines
only), in mm and in inches, with Arcs in the machine's own units as its default. They suit
grbl 1.1 and grblHAL: a tool change stops the spindle and pauses (M0) for you to change and
zero the tool, then resume from your sender.

Cut Craft also has, for GRBL, Mach3 and FluidNC (with a tool changer), in mm and in inches:
**Arcs**, **Straight**, and rotary **Wrap Y to A** / **Wrap X to A**, each with feeds in
units a minute or in inverse time (**G93**). Add one to a machine in Machine Configuration,
or customise and edit a copy in Post-Processor Management.

**Check the working area against your own machine before cutting.** These are the
makers' published sizes as the packages were written; extensions, kits and revisions
differ. Fix one in Machine Configuration after installing, or send a correction here.

To add machines, copy an entry in the maker's file (or start a new file the same way) and
open a pull request.
