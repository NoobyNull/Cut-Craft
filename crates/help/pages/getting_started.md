---
reviewed: 2026-11-24
see-also: design, toolpaths, setup_material, finding_help, first_time_setup
icon: file/new.png
---

# Your first job

New to Cut Craft? This page shows the whole journey once, from a blank sheet to a file
your machine can cut.

## Try the sample project first

![The blank sheet Cut Craft opens with, and Try a sample project in its middle](pictures/basics/empty-sheet.png)

Cut Craft opens on a blank sheet of material. In its middle, **Try a sample project**
opens a small finished sign: the word HELLO with a rounded border, its toolpaths already
made. Press **Preview All Toolpaths** on the Toolpaths page and watch it cut, before you
start your own.

![The sample sign, its V-Carve and Profile toolpaths made](pictures/basics/sample-project.png)

The sample is a new, unsaved job: change anything you like, it harms nothing.

## Two steps: Design, then Toolpaths

Cut Craft works in two steps:

1. **Design** is where you draw **what** you want: shapes, text and 3D models. See
   [Design page](help:design).
2. **Toolpaths** is where you tell the machine **how** to cut it: which bit, how deep, and
   which side of the line. See [Toolpaths page](help:toolpaths).

In the [Ribbon layout](help:layout_ribbon) Cut Craft starts in, these are the **Design** and
**Toolpaths** tabs across the top, with **Model** between them for 3D parts. The
[Rail](help:layout_rail) and [Classic](help:layout_classic) layouts group the same tools
differently, but the two steps are the same.

![In the Classic layout: Design down the left to draw, Toolpaths down the right to say how to cut it](pictures/basics/design_then_toolpaths.png)

## Your own first job, step by step

1. **Set up the material.** Press [Setup Material](help:setup_material), measure your
   board and type its width, height and thickness.
2. **Draw.** For a simple sign, [Draw Rectangle](help:draw_rectangle) for the outline and
   [Draw Text](help:draw_text) for the words.
3. **Make the toolpaths.** Pick the text and make a
   [V-Carve / Engrave](help:vcarve_engrave_toolpath) toolpath; pick the outline and make a
   [Profile Toolpath](help:profile_toolpath) with [tabs](help:tabs) to cut it free.
4. **Preview.** Watch it cut on screen, and fix anything that looks wrong.
5. **Save the toolpaths** for your machine with [Save Toolpaths](help:save_toolpaths).
   Cut Craft runs its [checks before cutting](help:checks_before_cutting) first.
6. **Save the job** too, so you can change it later: see
   [Saving and opening jobs](help:saving_jobs).

## Millimetres or inches

![Switching units converts every measurement, so nothing changes size](pictures/basics/units.png)

Work in whichever your rulers and bits use. **Edit ▸ Units ▸ Millimetres** or **Inches**
switches the open job, and converts every measurement in it exactly, so nothing changes
size. New jobs start in the units set in **Edit ▸ Options ▸ Job Defaults**. Number fields
also take the other units as you type: see [Number fields](help:number_fields).

## Made a mistake?

![Ctrl+Z undoes the last change](pictures/basics/undo.png)

**Ctrl+Z** (**Edit ▸ Undo**) undoes the last change, as many times as you need.
**Ctrl+Y** (**Edit ▸ Redo**) puts it back. Undo covers drawing, moving, deleting and
toolpath changes alike, so try things: nothing is lost.

Next: [Finding help](help:finding_help) shows the quickest way to an answer, and
[Keyboard shortcuts](help:keyboard_shortcuts) the keys worth learning first.
