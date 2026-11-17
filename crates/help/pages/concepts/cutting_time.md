---
reviewed: 2026-11-17
see-also: toolpath_summary, create_job_sheet, progress, toolpaths, web_api
---

# How long the job takes to cut

The **Toolpath Summary**, the job sheet, the notes' **Run time** variables and the Web API
all tell you how long the machine will take to cut the job. This is the time at the machine,
not the time Cut Craft takes to work the toolpaths out. That one shows in the progress
window: see [Long jobs and the time left](help:progress).

![The Toolpath Summary: each toolpath's time and the whole job's](pictures/toolpath_summary/step-3.png)

## What the time counts

Every move the saved G-code makes:

- **Cutting and plunging** at each bit's feed rate and plunge rate.
- **Rapids** inside each toolpath, lifting clear and moving to the next cut.
- **Getting to each toolpath** from where the one before it ended, and from the job's home
  position for the first one.
- **The trip back home** after the last toolpath. It is in the job's total, not in a
  toolpath's own time.
- **Tool changes**: each time the next toolpath uses another bit. A change by hand counts
  2 minutes and a change by the tool changer counts 15 seconds. Change these with **Bit
  change by hand** and **Bit change by tool changer** in **Edit ▸ Options ▸ Toolpaths**. A
  bit loaded in the machine's tool changer counts as the quick change, and any other bit
  counts as a change by hand.
- **Drill dwells**: the wait at the bottom of each peck when a Drilling toolpath asks for one.

A toolpath's own time includes getting to it and its tool change. That way, the toolpath
times add up to the job's total apart from the trip home.

## Speeding up, slowing down and corners

A real machine cannot jump straight to its feed rate. It speeds up at the start of each
move, slows down at the end, and slows for corners. A job with lots of short moves, such as
a 3D finish or a V-carve, can take far longer than its feed rates suggest.

Cut Craft plans the moves the way the machine's controller does:

1. Each move's top speed is its feed rate, held to the **Max rate** of every axis it uses.
   Rapids run at the axes' Max rates.
2. The speed through each corner comes from **Cornering**. A gentle bend keeps nearly full
   speed, and a sharp corner or a turn back slows right down.
3. The machine then speeds up and slows down between those speeds as fast as its
   **Acceleration** allows. A move too short to reach its feed never gets there.

These figures come from the active machine's **Limits** in **Machine Configuration**:

| Setting | What it is | Where to find it on the controller |
|---|---|---|
| Max rate (X, Y, Z) | The fastest each axis moves | GRBL $110 to $112 |
| Acceleration (X, Y, Z) | How fast each axis speeds up | GRBL $120 to $122 |
| Cornering | How far the controller lets the bit stray from a sharp corner to keep its speed (junction deviation). Leave it at 0 for the usual 0.01 mm | GRBL $11 |

Without an active machine, or with no Max rate given, rapids run at the Toolpath Summary's
**Rapid rate**. Without accelerations, moves start and stop at once, which gives a time that
is too short.

## Making the time match your machine

- Fill in the machine's **Limits** in Machine Configuration. They make the biggest difference.
- **Machine acceleration** in the Toolpath Summary is ticked to begin with. Untick it to see
  the time at the feed rates alone.
- If your machine still runs slower than the estimate (a feed override, a slow controller),
  set the Toolpath Summary's **Scale factor**. For example, 1.2 makes every time a fifth longer.
- Time a real job once and compare it with the estimate to choose the scale factor.

Rotary jobs are timed as if the material were flat, so their times are rougher.
