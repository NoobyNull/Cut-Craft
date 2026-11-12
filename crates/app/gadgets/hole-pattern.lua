-- Hole Pattern: holes spaced evenly round a circle, centred on the job.
-- Handy for bolt circles, clock faces and lazy-susan bearings.

gadget = {
  name = "Hole Pattern",
  description = "Draws circles spaced evenly round a circle in the middle of the job.",
}

settings = {
  { key = "count", label = "Number of holes", type = "integer", default = 6, min = 1, max = 360,
    help = "How many holes, spaced evenly round the circle." },
  { key = "pattern_diameter", label = "Pattern diameter", type = "length", default = 100, min = 0,
    help = "The size of the circle the holes' centres sit on." },
  { key = "hole_diameter", label = "Hole diameter", type = "length", default = 8, min = 0.1,
    help = "How wide each hole is drawn." },
  { key = "start_angle", label = "First hole at (degrees)", type = "number", default = 90,
    help = "0 is to the right of the middle, 90 is straight up." },
}

function run(s)
  local centre = cutcraft.job().centre
  local r = s.pattern_diameter / 2
  for i = 0, s.count - 1 do
    local a = math.rad(s.start_angle) + i * 2 * math.pi / s.count
    cutcraft.run("draw_circle", {
      center = { x = centre.x + r * math.cos(a), y = centre.y + r * math.sin(a) },
      radius = s.hole_diameter / 2,
    })
  end
end
