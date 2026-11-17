---
reviewed: 2026-11-17
see-also: cutting_time, create_job_sheet, save_toolpaths
---

## Steps

![The steps, one after another](pictures/toolpath_summary/tour.gif)

1. Once the toolpaths are made, open the **Analyse** tab on the Toolpaths page and press **Toolpath Summary**.

   ![Step 1](pictures/toolpath_summary/step-1.png)

2. Press **Toolpath Summary** (under Reports).

   ![Step 2](pictures/toolpath_summary/step-2.png)

3. It lists each toolpath with its bit and how long it takes to cut, and the whole job's time.

   ![Step 3](pictures/toolpath_summary/step-3.png)

## What the times count

Each toolpath's time counts every move it makes: cutting, plunging, rapids, getting to it
from where the toolpath before it ended (or from home, for the first), and its tool change
when it uses another bit. The **Total machining time** adds the trip back home at the end.

The machine speeds up, slows down and slows for corners as its **Limits** in Machine
Configuration say: **Max rate**, **Acceleration** and **Cornering**. **Machine acceleration**
is ticked to begin with. Untick it to leave out speeding up, slowing down and slowing for
corners. Each axis's Max rate still holds.

- **Rapid rate**: how fast rapids go when Machine Configuration gives no Max rate.
- **Scale factor**: multiplies every time, for a machine that runs slower than planned.

How it all works, and how to make the times match your machine, is in
[How long the job takes to cut](help:cutting_time).
