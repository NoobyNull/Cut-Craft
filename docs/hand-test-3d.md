# Hand test: 3D modeling (work from 2026-10-26)

Tick each line, or note what went wrong next to it (what you did, what you saw, what you
expected). A screenshot path is enough for anything visual.

## Set-up
- [ ] New flat job (e.g. 300 x 200 x 20 mm). Draw a circle and a rectangle; Create Shape on
      each (a dome and a flat-topped one) so there are two components.

## Component Properties (double-click a component in the Components tab)
- [ ] Shape Height % halves / doubles the height; the 3D view updates.
- [ ] Base Height lifts the component.
- [ ] Fade: anchors typed in; the far side drops by the percentage.
- [ ] Tilt: pivot + raised point + angle; the shape leans (expect the side behind the
      pivot to dip too — that's by design, flag it if it looks wrong next to Aspire).
- [ ] Bake: properties reset to defaults, the shape stays as it looked.

## Transforming components (select in 2D view or Components tab)
- [ ] Move, Rotate, Mirror, Set Size each move the 3D shape with the 2D outline.
- [ ] Align / Center tools work on two selected components.

## Picking in the 3D view
- [ ] Double-click a component in the 3D view selects it (tree and 2D view agree).
- [ ] Double-click off the model fits the view.

## Sculpting (Model > Sculpt Visible Model)
- [ ] Smooth, Smudge, Deposit, Remove each change the surface under the drag.
- [ ] Undo Brush pulls back toward the pre-sculpt shape; Ctrl+Z undoes one stroke.
- [ ] Diameter / strength sliders feel sane (too fast, too slow?).

## Distort Object (Drawing tab, or Model > Distort Selected Component)
- [ ] Bounding box mode on a component.
- [ ] Along a curve; between two curves.
- [ ] Create Envelope, Node Edit it (N), Distort to Envelope.

## Levels and groups (Components tab)
- [ ] Insert New Level; move a component onto it (Move to Level).
- [ ] Level combine mode / base height change the result.
- [ ] Level Clipping: Apply with a closed vector, then Update after moving it, then Remove.
- [ ] A mirror mode reflects half the job.
- [ ] Move Up / Down on a component and on a level.
- [ ] Group two components; group combine mode; Ungroup.
- [ ] Duplicate Component.

## Clipart and segmenting
- [ ] Model > Export as 3D Clipart saves a `.ccclip`; Import Component reads it back.
- [ ] Import an STL (Orientate 3D Model form: up face, rotation, size).
- [ ] Model > Segment 3D Model on the STL: two halves laid side by side.

## Rotary (new rotary job in Job Setup)
- [ ] Job Setup form swaps to Length / Diameter / Orientation.
- [ ] Material Setup's model position moves the model.
- [ ] Import an STL as Full 3D model: it wraps round the axis in the 3D view.
- [ ] Auto-Wrapping View on / off (View menu).
- [ ] Vector Unwrapper on a closed cross-section, then Two Rail Sweep straight after.
- [ ] Level Wrapping: a shape run off one edge comes round the other.

## General feel
- [ ] Anything slow (more than a second or so after a change)?
- [ ] Anything that looks different from Aspire 12's equivalent?
