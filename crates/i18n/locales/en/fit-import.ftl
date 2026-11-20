# src/app/fit_import.rs
fit-import-done = Done
fit-import-gap-at-each-end = Gap at each end
fit-import-how-far-in-from-each = How far in from each end of the line the drawing starts and stops. 0 runs it the line's whole length
fit-import-how-the-drawing-takes-the = How the drawing takes the shape's size:
fit-import-keep-the-drawing-as-it = Keep the drawing as it is now
fit-import-margin = Margin
fit-import-take-the-drawing-back-out = Take the drawing back out, as if it was never imported
fit-import-the-drawing-is-bent-to = The drawing is bent to run the line's length, like lettering on an arc.
fit-import-the-gap-kept-between-the = The gap kept between the drawing and the shape's edge, so a cut inside the shape does not run into its border. 0 lets it touch
fit-import-undo-import = Undo Import
fit-import-where-it-sits = Where it sits:

# Import Into Selected Shape, when the selection will not do.
fit-import-select-first = Select the shape to import into first (a circle, rectangle or any closed vector), or an open line to bend the drawing along, then choose Import Into Selected Shape
fit-import-select-a-vector = Select a vector to import into: a 3D component cannot hold a drawing
fit-import-select-just-one = Select just one shape or line to import into
fit-import-crumples = The line bends tighter than the drawing is tall, so the drawing's edge on the inside of the bend folds over itself. Put it on the other side of the line, or use a gentler curve
fit-import-importing = Importing { $name }
fit-import-shape-gone = The shape to import into is gone: nothing was imported
fit-import-imported-into = Imported { $name } into the selected shape
fit-import-imported-along = Imported { $name } along the selected line
fit-import-along-line-title = Import Along Line
fit-import-into-shape-title = Import Into Shape
fit-import-undone = Import undone: nothing changed
# How a drawing takes a closed shape's size.
fit-import-way-scale = Scale to fit
    .tooltip = As big as fits inside the shape's outline, its proportions kept: nothing is squashed
fit-import-way-stretch = Stretch
    .tooltip = Made as wide and tall as the shape, so it fills it: lettering and pictures come out squashed or stretched
fit-import-way-morph = Morph to shape
    .tooltip = Bent so its edges follow the shape's outline all the way round, like a picture on a balloon. Curves become many short lines
# Which side of an open line the drawing sits on.
fit-import-side-above = Above the line
    .tooltip = The drawing stands on the line, as lettering stands on a ruled line. On an arch it sits on the outside of the curve
fit-import-side-on = Centred on the line
    .tooltip = The line runs through the middle of the drawing
fit-import-side-below = Below the line
    .tooltip = The drawing hangs from the line. On an arch it sits on the inside of the curve
