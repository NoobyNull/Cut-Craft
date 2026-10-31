# Switching a job between inches and millimetres

A user asked for an easy way to change a job from SAE to metric and back, with everything
following: the job size, the material thickness, cut depths, tab heights, every length.
The one exception Matthew named: a tool's specifications stay as they are, in the tool's
own units, unless the tool itself is edited.

## What is already true

The document stores every length in millimetres (`Job.units` is a display setting). Forms
divide by 25.4 when they show a `mm` or `mm/min` field in an inch job (`form::shown_in`);
the rulers, the measure and inspect readouts, the Sheets list, Dimensions, the Toolpath
Summary and the job sheet all read `Job.units` too. So a switch is a change of that one
flag, and the physical part stays the same: a pocket 1 inch deep reads 25.4 mm afterwards.

## The switch

- **Edit ▸ Units ▸ Millimetres / Inches**: radio items (`units_mm`, `units_inch`), ticked
  for the job's units. Picking one sets `Job.units` through a `Change` (undoable, saved
  with the job), and the status bar says what changed.
- **The status bar**: the pointer's X and Y show in the job's units (today they say `mm`
  whatever the job's units are). The unit at their end is a button: one click swaps the
  units, the same as the menu.
- Job Setup's own Units choice keeps working as it does.

## Tools keep their own units

- `Tool.units: Units` (`mm` by default; a tool database or job saved without it loads as
  millimetres). It says how the tool's lengths show, never how they are stored (still mm).
- **Tool Database**: a tool's Diameter, Tip Radius, tooth sizes, Pass Depth and chip load
  show in the tool's units, with a Units drop-down (mm / inches) beside the Tool Type, as
  Aspire's tool form has. Changing it changes only the display: a 6 mm end mill switched to
  inches reads 0.2362 in; the tool is the same tool. The job's units no longer reach the
  Tool Database (`Look::inch` goes).
- **Toolpath forms**: the Tool block's diameter, tip radius and pass depth show in the
  tool's units; its feed and plunge rates already show in the tool's rate units.
- **Imports**: a vendor `.vtdb` tool keeps its file's units (`units` 1 is inches); a Fusion
  `.tools` tool keeps its file's (inch files give inch tools), beside the rate units they
  already keep.

## Tests

- Edit ▸ Units: `units_inch` sets `Job.units`, ticks that item only, and Undo restores mm.
- The status bar's coordinates read `in` in an inch job and `mm` in a mm job.
- A tool with inch units shows its diameter in inches in a toolpath form of a mm job.
- `Tool` without `units` deserialises as mm; a Fusion inch tool imports with inch units.

## Out of scope

Converting a tool's stored figures when its units change (the figures are the tool's; the
units are how they read), the job sheet's and summary's tool columns (they stay in the
job's units, as a report about the job), and a units choice per field.
