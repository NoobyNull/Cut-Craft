---
reviewed: 2026-11-20
see-also: settings, checks_before_cutting, check_missed_spots, vector_validator, save_toolpaths
---

# Confirmation Choices

**Edit ▸ Options ▸ Confirmation Choices** decides what Cut Craft does when something looks
wrong: ask you, fix it a set way, or let it go. Each starts on the safe choice, so most
people never need to change them.

![Options open at Confirmation Choices](pictures/basics/options-confirmations.png)

## Warn when material left is over

When a bit's taper or round end can't reach right into a 3D model's dips and corners, it
leaves a little wood behind. The [checks before cutting](help:checks_before_cutting) warn
about it under [Bit fits every corner](help:check_missed_spots), but only where more than
this is left: **0.1 mm** to start with.

- **Raise it** (say to 0.3 mm) if the warnings are about bits of wood too small to matter,
  or that sanding will take off.
- **Lower it** for fine detail where every tenth of a millimetre shows.

It goes from 0 to 10 mm. Material a later toolpath cleans up (a smaller bit's finishing
pass after it) is never warned about.

## Crossing vectors

Three choices for when the vectors you pick cross over themselves or each other, which
spoils the cut:

- **Toolpath vector intersections check**: vectors picked for a V-Carve or Pocket toolpath.
- **Radiused offset intersections check**: vectors picked for a round-cornered offset.
- **Sharp offset intersections check**: vectors picked for a sharp-cornered offset.

Each can **Ask** (go on, open the [Vector Validator](help:vector_validator) to see where,
or stop), **Open vector validator** straight away with the crossing vectors picked, or
**Don't check**.

## Ask to recalculate out-of-date toolpaths

After an edit leaves toolpaths out of date, Cut Craft asks whether to recalculate them
all. Untick it and it stops asking. The toolpaths are still marked ⚠ in the Toolpaths list,
and [saving them](help:save_toolpaths) still asks first, so an old toolpath is never cut
by mistake.

## G-code from an earlier save

Saving toolpaths where an earlier save of the same name left G-code:

- **Ask** each time: Replace or Archive.
- **Replace** the earlier files. The older numbered files go too, so none is left to run
  by mistake.
- **Archive**: move the earlier files into a dated folder under **Archive** beside them,
  then add the new ones.

Like every page of Options, changes here wait for **Apply**: see
[Changing settings](help:settings).
