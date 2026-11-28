---
reviewed: 2026-11-20
see-also: document_variables, setup_material, getting_started, keyboard_shortcuts
icon: diagrams/size_wh.svg
---

# Number fields

Every size, depth, speed and count in PowerWhittle is typed into a number field: a box with
the value in it and small up and down arrows on its right. You can type a number, nudge it
with the arrows, drag it, or type a sum and let PowerWhittle work it out.

## Nudging with the arrows

![A number field's arrows, and Shift for steps ten times as big](pictures/basics/number_arrows.png)

1. Click the **up** or **down** arrow on the right of the field. The value moves one step.
2. Hold the arrow down to keep going.
3. Hold **Shift** while you click for steps **ten times as big**.

The step suits the field: 0.1 mm or 0.01 in for a length, 1 for a percentage, an angle or
a count, 100 mm/min (5 in/min) for a feed rate, 1000 rpm for a spindle speed. A field with
a small range takes a smaller step. The arrows stop at the field's lowest and highest
values, so you can't nudge a setting somewhere it doesn't work.

A value between steps goes to the next step first. The 5 mm clearance shows as 0.19685 in
an inch job; one click up makes it exactly 0.2 in, so the G-code says Z0.2000.

## Dragging for fine changes

Point at the value: the pointer shows a two-way arrow. Hold the left button down and drag
**sideways**. Each point you move changes it by a tenth of a step: 0.01 mm or 0.001 in for
a length. Drag right to raise it, left to lower it.

## Typing a sum

![W/2 typed in a number field becomes half the material's width](pictures/basics/number_sums.png)

A number field takes a sum as well as a number. Type it, then press **Enter** (or click
elsewhere) and the answer takes its place.

- `+ - * /` add, take away, times and divide; `^` raises to a power; brackets work too.
- **W** or **X**: the material's width. **H** or **Y**: its height. **T** or **Z**: its
  thickness. Type `W/2` for half the width, or `T-3` for 3 below the full thickness.
- **P**: π (about 3.1416). A circle's distance round is 2 × π × its radius, so for a 50 mm
  radius type `2*P*50`.
- **M** turns inches into millimetres, for a millimetre job: type `2*M` for two inches
  (50.8 mm).
- **I** turns millimetres into inches, for an inch job: type `6*I` for 6 mm (0.236 in).
- `'` after a number: feet, for an inch job. Type `3'` for three feet (36 in), or `2'+10`
  for two feet ten.
- **{name}**: a [document variable](help:document_variables) you set up, such as
  `{depth}`.

If a sum can't be worked out, the field keeps what it had, so a slip never sets a
setting to nonsense.

## Millimetres or inches

A number field shows the job's units: **mm** or **in** beside the value. Change the units
for the whole job with **Edit ▸ Units**, or in [Setup Material](help:setup_material).
Every measurement in the job converts exactly, so nothing changes size. See
[Your first job](help:getting_started).

Not sure what a setting means? Point at its name for a short note about it.
