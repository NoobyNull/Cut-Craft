# Separate rotary carving toolpaths over one shared engine

Carving a part on a rotary job is split into separate toolpaths, one bit each: Rotary Roughing,
Rotary Finishing, Locked Angles and Rotary Details, alongside Rotary Lathe for turned parts. The
list order is the cutting order, and each toolpath cuts only what the toolpaths above it leave
(unless Settings' Advanced rotary mode is on). They all sit on one engine, the
`cut-craft-rotary-carving` crate: the model round the axis, the drop, what is left of the
material, and the holder, chuck and tailstock kept clear.

We chose this over one toolpath with many strategies (the old 4-Axis Rotary Machining with
"Sides, then details") because each step wants its own bit, its own place in the list, and its
own recalculation, as flat 3D roughing and finishing already work; saving can still group the
toolpaths by bit. Matthew decided it on 2026-11-25. 4-Axis Rotary Machining stays for old jobs
until its strategies are covered by the new toolpaths.
