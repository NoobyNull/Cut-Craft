# What a tool says when it cannot run, keyed by its English words (`error-`, its first words and a
# check of the whole text). The app starts each with a capital and ends it with a full stop,
# so write them as the English is. Each line says which tool says it.

## border-tile/src/lib.rs
error-the-depth-map-is-too-eab97d = the depth map is too big

## commands/3d-finishing-toolpath/src/lib.rs
error-choose-at-least-one-bit-a1ac27 = choose at least one bit
error-the-bit-s-finishing-stepover-217283 = the bit's finishing stepover must be more than 0% and at most 100%: set it in the Bit Library

## commands/3d-roughing-toolpath/src/lib.rs
error-set-the-bit-s-clearing-13c7c3 = set the bit's clearing stepover to more than 0% and at most 100% in the Bit Library
error-set-the-allowance-to-0-dcc459 = set the allowance to 0 or more

## commands/add-draft/src/lib.rs
error-there-is-no-visible-model-48ad9f = there is no visible model to add draft to. Show a component, or create one with a modelling tool
error-set-the-draft-angle-above-cbb20b = set the draft angle above 0° and at most 45°

## commands/array-copy-toolpath/src/lib.rs
error-show-the-toolpaths-to-copy-46f456 = show the toolpaths to copy in the toolpath list
error-the-toolpaths-to-copy-have-bbf7aa = the toolpaths to copy have no cuts. Edit them and press Calculate first

## commands/array-copy/src/lib.rs
error-select-vectors-to-copy-10add3 = select vectors to copy
error-set-rows-and-columns-to-cd6d0a = set rows and columns to 1 or more
error-one-row-of-one-column-2d41dd = one row of one column makes no copies. Set rows or columns to 2 or more

## commands/auto-layout-text/src/lib.rs
error-select-a-closed-vector-to-9a26d4 = select a closed vector to fit the text in
error-set-the-border-from-0-9636b9 = set the border from 0 to under 50 %
error-enter-some-text-4d3e12 = enter some text
error-enter-some-text-the-font-ef7b92 = enter some text the font can draw

## commands/bake-components/src/lib.rs
error-show-at-least-two-components-0bdcf2 = show at least two components to bake together

## commands/bitmap-properties/src/lib.rs
error-there-are-no-pictures-in-dfcb60 = there are no pictures in the job. Import a picture first

## commands/celtic-weave/src/lib.rs
error-give-the-knot-grid-at-587f45 = give the knot grid at least one cell, and a cell size above 0

## commands/chamfer-toolpath/src/lib.rs
error-select-the-closed-vectors-to-8ec819 = select the closed vectors to chamfer
error-the-chamfer-does-not-fit-bac422 = The chamfer does not fit inside the selected shapes. Make the width smaller, or chamfer Outside instead.
error-pick-a-v-bit-or-aae50c = pick a V-bit or a ball nose: a chamfer needs a sloped tip
error-set-the-angle-between-0-88df37 = set the angle between 0° and 90°
error-set-start-depth-and-overcut-010ff8 = set start depth and overcut to 0 or more

## commands/circular-copy/src/lib.rs
error-set-items-to-2-or-6a2708 = set items to 2 or more: the original and at least one copy
error-set-the-angle-to-something-c22429 = set the angle to something other than 0
error-set-the-total-angle-to-524a51 = set the total angle to 360° or less

## commands/close-extend-vector/src/lib.rs
error-select-open-vectors-to-close-ffdbe0 = select open vectors to close
error-no-selected-vector-s-ends-8930ce = no selected vector's ends are within the max gap. Raise the max gap, or move the ends closer
error-set-max-gap-to-0-eb0718 = set max gap to 0 or more

## commands/close-with-line/src/lib.rs
error-that-vector-is-already-closed-ce668c = that vector is already closed. Select an open vector to close

## commands/component-boundary/src/lib.rs
error-select-components-to-draw-a-76f3a1 = select components to draw a boundary round
error-the-components-have-no-shape-22e869 = the components have no shape. Select components with some height

## commands/component-properties/src/lib.rs
error-set-shape-height-to-0-03822c = set shape height to 0 or more

## commands/construction-lines/src/lib.rs
error-the-selected-vectors-are-construction-8e3909 = the selected vectors are construction lines already. Select a solid line to make it one
error-the-selected-vectors-are-real-01e0a3 = the selected vectors are real lines already. Select a dashed construction line to make it real

## commands/convert-text-to-curves/src/lib.rs
error-select-the-text-to-convert-860efd = select the text to convert

## commands/copy-along-vectors/src/lib.rs
error-select-the-vectors-to-copy-faea24 = select the vectors to copy circles along
error-select-the-object-to-copy-932e59 = select the object to copy first, then the vectors to copy it along
error-a-vector-to-copy-along-e2e22d = a vector to copy along has no length. Draw it as a line or curve
error-set-copies-to-1-or-6f49bc = set copies to 1 or more

## commands/copy-to-other-side/src/lib.rs
error-only-a-double-sided-job-79ab7f = only a double-sided job has another side. Set the job to double-sided in Job Setup first

## commands/copy-toolpath-to-sheet/src/lib.rs
error-pick-another-sheet-to-copy-467757 = pick another sheet to copy the toolpath to

## commands/create-component-from-bitmap/src/lib.rs
error-import-the-picture-first-555987 = import the picture first

## commands/create-component-from-visible-model/src/lib.rs
error-there-is-no-visible-model-e61039 = there is no visible model. Show a component, or create one with a modelling tool

## commands/create-fillets/src/lib.rs
error-select-vectors-whose-corners-to-f332bc = select vectors whose corners to fillet
error-the-selected-vectors-have-no-aa723b = the selected vectors have no sharp corners of that kind to fillet. Pick another corner type, or select other vectors
error-the-radius-is-too-large-4ea300 = the radius is too large for every corner: try a smaller one
error-no-fillet-fits-that-corner-854c0e = no fillet fits that corner. Try a smaller radius
error-only-a-fillet-between-two-970051 = only a fillet between two straight sides can be taken out. Click a fillet between two straight sides
error-click-on-a-sharp-corner-e64550 = click on a sharp corner to fillet it, or on a fillet to take it out

## commands/create-shape/src/lib.rs
error-select-an-open-vector-for-97abab = select an open vector for the custom profile with the closed ones
error-select-the-closed-vectors-to-d6185a = select the closed vectors to shape
error-set-the-angle-between-1-670798 = set the angle between 1° and 89°
error-a-flat-shape-needs-a-e79168 = a flat shape needs a height. Set the height to more than 0

## commands/crop-bitmap/src/lib.rs
error-select-the-picture-to-crop-15cc13 = select the picture to crop first
error-select-closed-vectors-to-crop-a83e9c = select closed vectors to crop the picture to
error-the-vectors-do-not-cover-f77de5 = the vectors do not cover the picture. Move a closed vector over the part to keep

## commands/cross-section/src/lib.rs
error-there-is-no-visible-model-d7d297 = there is no visible model to cut a section through. Show a component, or create one with a modelling tool
error-drag-a-line-across-the-6714ed = drag a line across the model

## commands/cut-vector/src/lib.rs
error-cut-between-the-ends-of-29adc5 = cut between the ends of an open vector

## commands/decorative-relief/src/lib.rs
error-the-area-is-too-small-476bc9 = the area is too small for the pattern. Make the pattern size smaller, or draw a bigger area

## commands/delete-items/src/lib.rs
error-only-layers-can-be-empty-a5ea40 = only layers can be empty. Pick Layers to delete empty ones

## commands/delete-layer/src/lib.rs
error-a-job-keeps-at-least-af5f60 = a job keeps at least one layer. Add another layer before deleting this one

## commands/delete-level/src/lib.rs
error-a-job-keeps-at-least-0df7c2 = a job keeps at least one level. Add another level before deleting this one

## commands/delete-point/src/lib.rs
error-the-vector-needs-the-nodes-1a5aa0 = the vector needs the nodes it has left. Delete the whole vector instead

## commands/delete-sheet/src/lib.rs
error-a-job-keeps-at-least-caf928 = a job keeps at least one sheet. Add another sheet before deleting this one

## commands/dimensions/src/lib.rs
error-drag-from-an-arc-or-41a29d = drag from an arc or circle to where its radius goes
error-drag-between-two-different-points-4a070f = drag between two different points
error-drag-from-the-corner-to-11cec8 = drag from the corner to where the arc and its text go

## commands/distort-object/src/lib.rs
error-pick-the-envelope-last-a-ae96bd = pick the envelope last: a closed vector with a node at each corner
error-pick-the-curve-to-distort-fe73bb = pick the curve to distort along last: an open vector
error-vectors-and-components-cannot-be-97153d = vectors and components cannot be distorted together. Select only vectors, or only components
error-the-objects-have-no-width-b2d7c1 = the objects have no width or height. Select objects that are not flat lines

## commands/drag-knife-toolpath/src/lib.rs
error-select-the-vectors-to-cut-d72baf = select the vectors to cut
error-set-the-pressure-between-1-48d2a2 = set the pressure between 1 and 100% of the spring travel
error-set-the-swivel-pressure-less-22bab7 = set the swivel pressure less than the cutting pressure
error-set-passes-to-1-or-165a48 = set passes to 1 or more

## commands/draw-arc/src/lib.rs
error-start-and-end-angle-must-5cbd4e = start and end angle must differ (use Draw Circle for a full circle)

## commands/draw-curve/src/lib.rs
error-a-curve-needs-at-least-cb3b88 = a curve needs at least 2 points (3 to close)

## commands/draw-freehand/src/lib.rs
error-drag-to-draw-5dd4fa = drag to draw

## commands/draw-polygon/src/lib.rs
error-set-sides-to-3-or-0cf077 = set sides to 3 or more

## commands/draw-polyline/src/lib.rs
error-click-at-least-2-points-b2eb0e = click at least 2 points to draw a line
error-click-at-least-3-points-bf69be = click at least 3 points to draw a closed shape

## commands/draw-star/src/lib.rs
error-set-points-to-3-or-79da98 = set points to 3 or more
error-set-inner-radius-smaller-than-9c2249 = set inner radius smaller than outer radius

## commands/draw-text/src/lib.rs
error-select-a-block-of-text-063370 = select a block of text to break into lines
error-text-in-a-box-is-b28b90 = Text in a Box is one block: it has no lines of its own. Select text drawn as lines
error-the-text-is-one-line-7d07f0 = the text is one line already. Select text with more than one line

## commands/drill-bank-toolpath/src/lib.rs
error-select-the-circles-to-drill-aef85e = select the circles to drill: each is drilled at its centre by a drill its size
error-set-the-dwell-and-retract-f8e99b = set the dwell and retract gap to 0 or more

## commands/drilling-toolpath/src/lib.rs
error-select-closed-vectors-to-drill-0aaa3a = select closed vectors to drill at their centres

## commands/edit-picture/src/lib.rs
error-set-gamma-above-0-7b48e3 = set gamma above 0

## commands/embellish/src/lib.rs
error-select-a-vector-to-embellish-5fa8ce = select a vector to embellish: a closed shape gets the ornament designed to fit it, an open line gets it along the line

## commands/emboss/src/lib.rs
error-select-a-component-to-emboss-c0e89d = select a component to emboss
error-set-scale-height-between-1-ead544 = set scale height between 1 and 50 %

## commands/engrave-part-names/src/lib.rs
error-there-are-no-parts-on-ebb394 = There are no parts on this sheet to name. Draw or import closed shapes first.

## commands/extrude-and-weave/src/lib.rs
error-a-drive-rail-is-a-55cfeb = A drive rail is a single point. Draw each drive rail as a line or curve.
error-the-cross-section-makes-no-736b5a = The cross-section makes no shape. Draw it as an open curve that rises away from the line between its ends.
error-select-the-component-to-extrude-c9b530 = select the component to extrude along the rails
error-select-the-drive-rail-s-5ac698 = select the drive rail(s), then one or more open cross-sections
error-the-cross-sections-must-be-85aa97 = the cross-sections must be open
error-the-cross-section-starts-and-0858e9 = the cross-section starts and ends at the same point. Draw it so its ends are apart

## commands/fit-curves-to-vectors/src/lib.rs
error-select-vectors-to-fit-curves-ea3da4 = select vectors to fit curves to

## commands/fluting-toolpath/src/lib.rs
error-select-the-vectors-to-flute-a28ab5 = select the vectors to flute
error-set-start-depth-to-0-1f86ed = set start depth to 0 or more
error-set-ramp-above-0-and-4435c0 = set ramp % above 0 and below 100

## commands/generate-border/src/lib.rs
error-describe-the-border-under-details-f250bd = describe the border under Details, or pick a style
error-set-the-depth-curve-between-5813f0 = set the depth curve between 0.2 and 5

## commands/group-components/src/lib.rs
error-select-two-or-more-components-a3c21b = select two or more components to group
error-components-on-different-levels-cannot-b38d6c = components on different levels cannot be grouped. Move them to one level first

## commands/group/src/lib.rs
error-select-at-least-two-objects-697425 = select at least two objects to group

## commands/import-toolpaths/src/lib.rs
error-pick-the-g-code-file-f0d13d = pick the G-code file to import (File ▸ Import ▸ Import Toolpaths…)
error-the-file-has-no-moves-33d53c = the file has no moves in it: is it G-code?

## commands/inlay-toolpath/src/lib.rs
error-select-the-closed-vectors-to-57c730 = select the closed vectors to inlay
error-set-the-step-depth-less-fdcedd = set the step depth less than the cut depth

## commands/insert-point/src/lib.rs
error-there-is-no-span-there-fa0e89 = there is no span there. Click on the vector's line

## commands/interactive-vector-trim/src/lib.rs
error-the-drag-crosses-no-vector-3934cf = the drag crosses no vector to trim
error-click-on-the-piece-of-a9c7cb = click on the piece of a vector to trim
error-nothing-crosses-that-vector-click-372aea = nothing crosses that vector. Click a piece between two crossings

## commands/job-setup/src/lib.rs
error-set-plunge-z2-between-0-b43db3 = set Plunge (Z2) between 0 and the clearance (Z1)
error-set-centre-stock-diameter-less-11121b = set Centre Stock Diameter less than the material's diameter, or 0 to leave no centre stock

## commands/join-close-vectors/src/lib.rs
error-select-one-open-vector-to-5934a3 = select one open vector to close, or two to join

## commands/join-open-ends/src/lib.rs
error-click-the-end-of-an-e8b961 = click the end of an open vector (each has a circle on it), then the end to join it to
error-these-ends-never-meet-if-8d3ca1 = these ends never meet if they run on: they point apart or side by side. Join them with Straight Line or Arc instead

## commands/join-vectors/src/lib.rs
error-no-open-vector-ends-are-20fc85 = no open vector ends are within the tolerance. Raise the tolerance, or move the ends closer together

## commands/keep-out-zones/src/lib.rs
error-select-closed-vectors-to-make-16862e = select closed vectors to make the zones from

## commands/keyhole-toolpath/src/lib.rs
error-select-circles-where-the-slots-a2bb48 = select circles where the slots go in

## commands/laser-cut-fill-toolpath/src/lib.rs
error-select-the-closed-vectors-to-9255ad = select the closed vectors to cut or fill
error-the-kerf-leaves-nothing-to-7e6dda = the kerf leaves nothing to cut inside the shapes. Make the kerf smaller
error-set-power-to-more-than-2d5d46 = set power to more than 0 and at most 100 %

## commands/laser-picture-toolpath/src/lib.rs
error-select-the-vectors-to-burn-cbeec9 = select the vectors to burn along
error-import-the-picture-to-burn-9e83dd = import the picture to burn first
error-the-selected-vectors-don-t-d0b602 = the selected vectors don't cross the picture: move them over the picture, or pick Raster or Hatch

## commands/level-clipping/src/lib.rs
error-select-the-closed-vectors-to-2cb8e3 = select the closed vectors to clip the level to

## commands/level-properties/src/lib.rs
error-wrapping-is-for-rotary-jobs-b5933c = wrapping is for rotary jobs only. Set the job to rotary in Job Setup first

## commands/make-start-point/src/lib.rs
error-only-a-closed-vector-s-db4238 = only a closed vector's start can move

## commands/merge-visible-layers/src/lib.rs
error-there-is-no-other-visible-5259f7 = there is no other visible layer to merge. Show the layers to merge

## commands/merged-toolpath/src/lib.rs
error-show-the-toolpaths-to-merge-ed1e3d = show the toolpaths to merge in the toolpath list

## commands/mirror/src/lib.rs
error-the-vector-picked-last-has-829062 = the vector picked last has its two ends together. Pick a straight line to mirror about
error-pick-the-objects-then-last-d63646 = pick the objects, then last the line to mirror them about

## commands/moulding-toolpath/src/lib.rs
error-select-the-drive-rails-and-702bd8 = select the drive rails and an open cross-section profile
error-select-a-drive-rail-as-68d510 = select a drive rail as well as the profile

## commands/move-node/src/lib.rs
error-there-is-no-handle-there-bf9c70 = there is no handle there. Make the span a curve first: right-click it and pick Convert Span to Bezier

## commands/move-nodes/src/lib.rs
error-pick-the-nodes-to-move-1cfce4 = pick the nodes to move

## commands/name-part/src/lib.rs
error-select-a-closed-shape-to-31d51f = Select a closed shape to name. Open lines are not parts.

## commands/nest-parts/src/lib.rs
error-no-part-fits-on-the-ce1f55 = No part fits on the material. Make the material bigger in Job Setup, or make the parts or the border gap smaller.

## commands/offset-model/src/lib.rs
error-there-is-no-visible-model-cb169d = there is no visible model to offset. Show a component, or create one with a modelling tool
error-set-distance-to-something-other-3dba34 = set distance to something other than 0

## commands/offset-vectors/src/lib.rs
error-select-closed-vectors-to-offset-aa8f97 = select closed vectors to offset
error-the-inward-offset-left-nothing-e85bd1 = the inward offset left nothing. Use a smaller distance

## commands/orientate-3d-model/src/lib.rs
error-nothing-of-the-model-is-d299a5 = nothing of the model is above the zero plane. Move the model up, or the zero plane down
error-this-file-has-no-triangles-d9e47e = This file has no triangles in it. Export it again from the program that made it.
error-the-model-has-no-area-8b9bf7 = the model has no area seen from above. Turn it so a wide side faces up
error-the-model-has-no-length-52a2fb = the model has no length or width. Turn it so a wide side faces up
error-choose-an-stl-file-3fb278 = choose an STL file

## commands/overlap-intersect/src/lib.rs
error-select-at-least-two-closed-abcd9b = select at least two closed vectors to overlap
error-the-selected-vectors-do-not-252152 = the selected vectors do not overlap. Move them so they overlap, then try again

## commands/photo-vcarve-toolpath/src/lib.rs
error-import-the-picture-to-carve-a8660f = import the picture to carve first
error-select-the-vectors-to-carve-3dfdd9 = select the vectors to carve along
error-pick-a-v-bit-photo-e57f19 = pick a V-bit: photo V-carving needs one

## commands/plate-production/src/lib.rs
error-select-the-vectors-of-the-860690 = select the vectors of the master template
error-a-plate-does-not-fit-21092c = a plate does not fit on the material inside the margins. Make the plate or the margins smaller
error-set-plates-to-1-or-7b1884 = set plates to 1 or more

## commands/pocket-toolpath/src/lib.rs
error-select-the-closed-vectors-to-a95053 = select the closed vectors to pocket
error-the-pocket-could-not-be-79a780 = the pocket could not be timed

## commands/prism-toolpath/src/lib.rs
error-select-the-closed-vectors-to-0b80a1 = select the closed vectors to prism carve
error-pick-a-v-bit-prism-440f7f = pick a V-bit: prism carving needs one

## commands/profile-toolpath/src/lib.rs
error-select-the-vectors-to-profile-ec4f8a = select the vectors to profile
error-click-on-the-vectors-in-cba6b9 = click on the vectors in the 2D view to place the tabs
error-cut-outside-instead-190532 = cut Outside instead

## commands/quick-engrave-toolpath/src/lib.rs
error-select-the-closed-vectors-to-b7f734 = select the closed vectors to fill
error-select-the-vectors-to-engrave-cedd97 = select the vectors to engrave

## commands/quick-trace/src/lib.rs
error-import-a-bitmap-to-trace-e1f9ce = import a bitmap to trace

## commands/relief-from-photo/src/lib.rs
error-import-the-photo-first-539d35 = import the photo first
error-the-picture-is-empty-pick-827719 = the picture is empty. Pick another picture
error-this-photo-has-no-depth-4a83cc = this photo has no depth yet: use Estimate Depth on it first, or choose 'Picture' under Depth from if the picture is already a depth map
error-set-the-lift-to-0-0d2e4c = set the lift to 0 or more and below the max height
error-set-the-soft-edge-and-058804 = set the soft edge and base to 0 or more
error-pick-the-bit-you-will-ab2118 = pick the bit you will carve with: Select… under Tool, or untick Only What the Bit Can Carve

## commands/reorder-component/src/lib.rs
error-it-is-already-at-the-ff9a66 = it is already at the top of its level
error-it-is-already-at-the-7083ca = it is already at the bottom of its level

## commands/reorder-layer/src/lib.rs
error-it-is-already-at-the-da6de7 = it is already at the top of the list
error-it-is-already-at-the-096963 = it is already at the bottom of the list

## commands/reorder-level/src/lib.rs
error-it-is-already-at-the-472725 = it is already at the end of the list

## commands/reorder-toolpath/src/lib.rs
error-the-new-order-must-name-493f4f = the new order must name every toolpath once

## commands/replace-below/src/lib.rs
error-select-a-component-cd9375 = select a component
error-that-would-remove-the-whole-6153f1 = that would remove the whole component. Set a lower height to replace below

## commands/reset-inside-selected-vectors/src/lib.rs
error-select-the-closed-vectors-to-a33d38 = select the closed vectors to reset inside

## commands/reset-outside-selected-vectors/src/lib.rs
error-select-the-closed-vectors-to-7c823b = select the closed vectors to reset outside

## commands/rotary-details-toolpath/src/lib.rs
error-this-toolpath-turns-the-material-f20ce9 = this toolpath turns the material: set the job up as rotary in Job Setup

## commands/rotary-finishing-toolpath/src/lib.rs
error-no-room-for-the-registration-29c30f = No room for the registration mark clear of the chuck and tailstock, so it is left out: pencil a line along the material at A=0 before you start, or bring End in from the tailstock.

## commands/rotary-lathe-toolpath/src/lib.rs
error-the-lathe-spins-the-material-5ba114 = the lathe spins the material: set the job up as rotary in Job Setup
error-the-bit-s-stepover-must-7f947e = the bit's stepover must be more than 0% and at most 100%: set it in the Bit Library
error-part-off-s-start-is-e8e2fe = Part off's Start is outside the machining limit: move Start to where the limit reaches along the part, or pick Material as the limit.

## commands/rounding-toolpath/src/lib.rs
error-rounding-needs-a-rotary-job-d51669 = rounding needs a rotary job: set one up in Job Setup
error-the-blank-is-no-bigger-86dd90 = the blank is no bigger than the finished cylinder. Make the blank bigger, or the allowance smaller
error-the-optimized-raster-is-for-42e540 = the optimized raster is for square blanks. Pick another strategy

## commands/scale-model-z-height/src/lib.rs
error-the-model-is-flat-so-cc4772 = the model is flat, so there is no height to scale. Give it height with a modelling tool first

## commands/sculpt/src/lib.rs
error-there-is-no-component-to-a07711 = there is no component to sculpt: create one first
error-the-brush-has-no-height-a8d16c = the brush has no height to press with
error-drag-over-the-component-to-2db6af = drag over the component to sculpt

## commands/segment-3d-model/src/lib.rs
error-the-plane-misses-the-model-e8c990 = the plane misses the model: move it inside
error-set-the-plane-between-0-fbf26f = set the plane between 0 and 100 %

## commands/select/src/lib.rs
error-select-a-closed-shape-first-19fdd7 = Select a closed shape first, then choose Select Vectors Inside to add everything inside it (or just click inside the shape).

## commands/send-to-sheet/src/lib.rs
error-the-vectors-are-on-that-3cb610 = the vectors are on that sheet already. Pick another sheet

## commands/set-size/src/lib.rs
error-the-selection-has-no-area-5845c1 = the selection has no area to resize. Select objects that are not flat lines

## commands/sketch-carving-toolpath/src/lib.rs
error-import-the-picture-to-sketch-d3e3ec = import the picture to sketch first
error-there-is-no-3d-model-a9e397 = there is no 3D model to sketch. Create one with a modelling tool first
error-the-sketch-has-no-lines-7d6256 = the sketch has no lines. Use a thicker line
error-select-the-closed-boundary-vectors-024190 = select the closed boundary vectors

## commands/slice/src/lib.rs
error-there-is-no-visible-model-15fb46 = there is no visible model to slice. Show a component, or create one with a modelling tool
error-the-model-has-no-thickness-7e455e = the model has no thickness above the zero plane. Show a component with some height
error-make-at-least-one-slice-39eb5b = make at least one slice

## commands/smooth-point/src/lib.rs
error-the-end-of-an-open-b80809 = the end of an open vector has nothing to smooth against

## commands/smooth-selected-components/src/lib.rs
error-there-are-no-components-to-4f721b = there are no components to smooth. Create one with a modelling tool first

## commands/split/src/lib.rs
error-select-a-component-and-one-e5c734 = select a component and one vector to split it along
error-select-a-closed-vector-to-446226 = select a closed vector to split with
error-select-a-component-to-split-e12616 = select a component to split
error-the-vector-does-not-cross-405c08 = the vector does not cross the component. Move it so it crosses

## commands/subtract-vectors/src/lib.rs
error-select-at-least-two-closed-8a37fc = select at least two closed vectors to subtract

## commands/surfacing-toolpath/src/lib.rs
error-set-the-bit-s-stepover-b4f207 = set the bit's stepover between 1% and 100% of its width (Edit… under Bit)
error-there-is-nothing-to-flatten-a592d9 = there is nothing to flatten
error-enter-the-spoilboard-s-width-f2c7df = enter the spoilboard's width and height (set your machine's bed size in Machine ▸ Machines and they fill in)
error-select-the-closed-outline-of-8132c8 = select the closed outline of what to flatten, or flatten the whole material

## commands/text-on-a-curve/src/lib.rs
error-select-a-single-line-of-a3b429 = select a single line of text and the curve to wrap it onto

## commands/texture-area/src/lib.rs
error-select-the-component-to-tile-46a104 = select the component to tile in the component tree last, after the closed vectors to fill (none: the whole job), as clicking a vector lets go of the component; or pick the tile, then open this tool
error-the-component-used-as-the-09170d = The component used as the tile has been deleted. Select another component to tile.
error-check-the-tile-settings-the-c887c4 = check the tile settings: the tile needs a shape, the spacing must leave each tile some room, and there must be four reflections

## commands/texture-toolpath/src/lib.rs
error-select-the-vectors-to-use-f4f59d = select the vectors to use as the pattern
error-the-boundary-offset-leaves-nothing-c8cf1e = the boundary offset leaves nothing to texture. Make the boundary offset smaller
error-not-a-texture-settings-file-ddc043 = not a texture settings file

## commands/thread-milling-toolpath/src/lib.rs
error-select-the-vectors-to-thread-d37c59 = select the vectors to thread round their centres
error-set-the-thread-mill-s-42c34f = set the thread mill's tooth offset to at least half the tooth size
error-set-the-max-depth-at-5d8a17 = set the max depth at least a pitch below the tooth offset
error-the-thread-mill-is-too-9e0153 = the thread mill is too big for this thread. Pick a smaller thread mill

## commands/trace-bitmap/src/lib.rs
error-nothing-to-trace-tick-more-427fe0 = nothing to trace. Tick more colours, or lower the noise filter
error-nothing-to-trace-set-another-f93ed0 = nothing to trace. Set another threshold or trace colour
error-this-picture-has-no-colours-fd9e95 = this picture has no colours. Set the trace to black and white

## commands/trim-objects/src/lib.rs
error-the-object-picked-last-the-37dae3 = the object picked last, the boundary, must be closed. Pick a closed vector last
error-pick-the-objects-to-trim-06a68c = pick the objects to trim, then last the boundary

## commands/turn-and-spin/src/lib.rs
error-select-one-open-vector-as-eb0964 = select one open vector as the profile
error-the-profile-makes-no-shape-466e07 = The profile makes no shape. Draw it as an open curve that bends away from a straight line, like half the outline of a vase.
error-the-profile-starts-and-ends-d5e78e = The profile starts and ends at the same point. Draw it so its ends are apart: it turns round the line between them.
error-the-profile-must-run-to-9ced10 = the profile must run to the right of its left end

## commands/two-rail-sweep/src/lib.rs
error-a-rail-is-a-single-b4d73b = A rail is a single point. Draw each rail as a line or curve.
error-the-cross-sections-need-width-ef7866 = the cross-sections need width: draw them upright, across from end to end
error-select-two-rails-then-one-e4aecc = select two rails, then one or more open cross-sections

## commands/ungroup/src/lib.rs
error-nothing-selected-is-in-a-d6b845 = nothing selected is in a group. Select a group to ungroup

## commands/vcarve-engrave-toolpath/src/lib.rs
error-select-the-closed-vectors-to-6fa22c = select the closed vectors to carve
error-clearance-bits-clear-a-fill-e94c6e = clearance bits clear a Fill's flat bottom: set Carve to Fill, or take them off
error-clearance-bits-clear-the-flat-eda53f = clearance bits clear the flat bottom: give a flat depth
error-pick-a-v-bit-v-d942c5 = pick a V-bit: V-carving needs one

## commands/vector-boundary/src/lib.rs
error-select-closed-vectors-or-tick-4c14d6 = select closed vectors, or tick Rubber Band Boundary for open ones

## commands/vector-selector/src/lib.rs
error-no-vectors-pass-those-filters-a3fb79 = no vectors pass those filters. Turn some filters off, or loosen them

## commands/vector-texture/src/lib.rs
error-the-area-is-too-small-e8fad0 = the area is too small for the texture. Make the texture smaller, or draw a bigger area
error-the-pattern-is-too-fine-956c94 = the pattern is too fine for an area this big. Make the pattern size or the smallest gap bigger

## commands/vector-unwrapper/src/lib.rs
error-the-vector-unwrapper-is-for-5771ec = the Vector Unwrapper is for rotary jobs only. Set the job to rotary in Job Setup first
error-select-one-closed-vector-the-11b94d = select one closed vector: the cross-section to unwrap

## commands/vector-validator/src/lib.rs
error-no-zero-length-spans-to-395a5d = no zero-length spans to fix in the vectors checked

## commands/weld-vectors/src/lib.rs
error-select-at-least-two-closed-abd570 = select at least two closed vectors to weld

## commands/wrapped-fluting-layout/src/lib.rs
error-this-gadget-lays-out-a-5f7842 = this gadget lays out a rotary job. Set the job to rotary in Job Setup first
error-the-offsets-leave-nothing-of-bee874 = the offsets leave nothing of the work piece. Make the offsets smaller
error-set-flutes-to-1-or-f57712 = set flutes to 1 or more

## commands/wrapped-spiral-layout/src/lib.rs
error-set-strands-to-1-or-712767 = set strands to 1 or more, and the pitch or spacing above 0

## core/src/change.rs
error-select-the-objects-to-space-5fd609 = select the objects to space, then the vector to space them inside
error-select-at-least-three-objects-9a936a = select at least three objects to space

## core/src/command.rs
error-select-what-to-work-on-e0e677 = Select what to work on: click a vector in the 2D view, Shift-click to add more
error-only-construction-lines-are-selected-bad7b0 = Only construction lines are selected, and toolpaths never cut along those. Select real vectors, or right-click a construction line and pick Make Real Line to cut along it
error-type-a-name-cf6c71 = Type a name.

## core/src/component.rs
error-select-the-closed-vectors-to-100822 = select the closed vectors to machine inside
error-the-machining-boundary-is-empty-5c006d = the machining boundary is empty
error-there-is-no-3d-model-29926e = there is no 3D model to machine: create a shape first
error-the-model-does-not-fit-b5cfb0 = the model does not fit in the material where Material Setup puts it. Change the position in Material Setup, or make the model lower

## core/src/document.rs
error-the-picture-this-toolpath-was-1d4208 = the picture this toolpath was made from has been deleted. Import it again and make a new toolpath from it

## core/src/locked_face.rs
error-this-toolpath-already-turns-a-03cd29 = this toolpath already turns A as it cuts: Flat on a locked face is only for flat toolpaths
error-a-drill-bank-s-holes-bfcbb2 = a drill bank's holes go where its drills are: Flat on a locked face is only for toolpaths the bit cuts itself

## core/src/nodes.rs
error-that-vector-cannot-be-edited-a6e9cd = that vector cannot be edited: its layer is hidden or locked, or it was deleted. Show or unlock its layer, or pick another vector
error-there-is-no-node-there-622bc8 = there is no node there. Click on a node

## core/src/vfont.rs
error-not-a-vectric-font-2925f6 = not a Vectric font
error-the-font-shares-an-object-0fe45e = the font shares an object; not expected in fonts
error-a-glyph-holds-something-other-005b08 = a glyph holds something other than contours

## formats/cut-list/src/lib.rs
error-the-job-has-no-parts-b74f41 = The job has no parts to list. Draw or import closed shapes first.
error-the-cut-list-is-empty-256301 = The cut list is empty.
error-the-cut-list-has-no-a3d745 = The cut list has no parts in it (every quantity is 0, or no rows).

## formats/export-as-3d-clipart/src/lib.rs
error-not-a-clipart-file-this-3f98e8 = not a clipart file this version of PowerWhittle reads
error-the-clipart-file-is-damaged-d6ef42 = the clipart file is damaged
error-there-is-no-component-to-efe33c = there is no component to export: create one first

## formats/export-as-grayscale-bitmap/src/lib.rs
error-there-is-no-3d-model-ffdf55 = there is no 3D model to export
error-the-model-is-too-big-0a64ed = the model is too big for a bitmap. Use a lower resolution

## formats/export-as-stl/src/lib.rs
error-there-is-no-3d-model-698ad7 = there is no 3D model to export: create a shape first
error-closing-with-the-other-side-f46ed4 = closing with the other side needs a double-sided job with a model on its other side. Turn this off, or set the job to double-sided in Job Setup
error-the-job-is-too-small-caffed = the job is too small to export

## formats/export-dxf/src/lib.rs
error-there-are-no-vectors-to-66410f = there are no vectors to export. Draw or import vectors first

## formats/import-crv3d/src/lib.rs
error-not-a-vectric-job-no-869605 = not a Vectric job: no 2D data
error-the-job-holds-no-vectors-2f83fc = the job holds no vectors

## formats/import-dxf/src/lib.rs
error-the-dxf-has-no-lines-0ad344 = the DXF has no lines, arcs, circles, polylines, ellipses or splines. Export it again from the program that made it

## formats/import-stl/src/lib.rs
error-the-model-has-no-area-4a8078 = the model has no area seen from above. Export it again from the program that made it
error-not-an-stl-file-e67a9e = not an STL file
error-the-stl-file-has-a-762c91 = the STL file has a broken triangle

## formats/import-svg/src/lib.rs
error-the-svg-has-no-vector-d67023 = the SVG has no vector paths. Export it again with its shapes as paths

## formats/part-labels/src/lib.rs
error-the-job-has-no-parts-e61b41 = The job has no parts to label. Draw or import closed shapes first.
