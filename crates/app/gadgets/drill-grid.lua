-- Drill Grid: rows and columns of holes, centred on the job.
-- Handy for pegboards, shelf-pin holes and speaker grilles.

gadget = {
  name = "Drill Grid",
  description = "Draws a grid of circles in the middle of the job.",
}

settings = {
  { key = "columns", label = "Columns (across)", type = "integer", default = 5, min = 1 },
  { key = "rows", label = "Rows (up)", type = "integer", default = 4, min = 1 },
  { key = "spacing_x", label = "Spacing across", type = "length", default = 25, min = 0,
    help = "From one hole's centre to the next one's, left to right." },
  { key = "spacing_y", label = "Spacing up", type = "length", default = 25, min = 0,
    help = "From one hole's centre to the next one's, bottom to top." },
  { key = "hole_diameter", label = "Hole diameter", type = "length", default = 6, min = 0.1 },
}

function run(s)
  local centre = cutcraft.job().centre
  local x0 = centre.x - (s.columns - 1) * s.spacing_x / 2
  local y0 = centre.y - (s.rows - 1) * s.spacing_y / 2
  for row = 0, s.rows - 1 do
    for col = 0, s.columns - 1 do
      cutcraft.run("draw_circle", {
        center = { x = x0 + col * s.spacing_x, y = y0 + row * s.spacing_y },
        radius = s.hole_diameter / 2,
      })
    end
  end
end
