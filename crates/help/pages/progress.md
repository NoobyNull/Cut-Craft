---
reviewed: 2026-11-25
see-also: toolpaths, web_api, contents, performance
---

# Long jobs and the time left

Some jobs take a while: calculating a 3D or Adaptive toolpath, Recalculate All, previewing
with Animate Preview off, the checks before saving toolpaths, opening or importing a big
file. Cut Craft does them in the background and, once one takes longer than a moment,
shows a progress window over the job.

![The progress window: what it is calculating, the step it is on, a bar at 20%, "1 s so far, about 2 minutes left" and Cancel](pictures/progress-window.png)

From the top, the window shows:

1. **What it is doing**, such as "Calculating 3D Finishing…".
2. **The step it is on**, and under it a note when that step is a slow one.
3. **How far it has got**, as a bar. A job that cannot tell how far it is shows a spinner
   and "Working…" instead.
4. **The time so far and the time left**, such as "12 s so far, about 2 minutes left".
5. **Cancel** (or **Esc**) stops the job. Nothing in your job changes.

Meanwhile the status bar says how briskly it is going: **Crawl**, **Walk**, **Run** or
**Sprint**. Click it to choose how much of the computer calculations may use (see
[Keeping the computer responsive](help:performance)).

## How the time left is worked out

The time left is a guess, and it gets better as the job goes on:

- **At first** it says it is **working out how long it will take**, because a job's first
  moments say little about the rest.
- **When you have done this kind of job before**, Cut Craft starts from how long it took last
  time, so you get a guess straight away. Then it follows how fast this job is really going.
- **The guess counts down with the clock** and changes gently. When a step is slower than
  the ones before, the time left goes up a little at a time, not in a sudden jump.
- **Times are rounded** the way you would say them: "about 35 seconds", "about 2 minutes",
  "about 1 hour 20 minutes". Near the end it says **almost done**.
- **A job that cannot tell how far it is** (most 2D toolpaths, which are usually quick)
  says how long it **usually takes**. If it runs well past that, it says it is
  **taking longer than usual**.

When you recalculate or preview several toolpaths, each one gets its own share of the bar.
A toolpath that took longer last time, or a preview with more moves to cut, gets a bigger
share, so the bar moves at an even pace.

Cut Craft remembers how long each kind of job took in a small file, `job-times.json`, in
its settings folder (on Linux `~/.config/cut-craft`, on Windows `%APPDATA%\cut-craft`,
on a Mac `~/Library/Application Support/cut-craft`). Deleting it only means the first
guesses start from scratch.

This is the time Cut Craft takes to work the job out on your computer. How long the
machine will take to cut is a different time: see [How long the job takes to cut](help:cutting_time),
and the line under the preview's controls, which gives the machine time left as it plays.

Programs that use the [Web API](help:web_api) see the same time left in **GET status**.
