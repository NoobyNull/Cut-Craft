---
reviewed: 2026-11-24
see-also: drilling_toolpath, pocket_toolpath
---

## Steps

![The steps, one after another](pictures/thread_milling_toolpath/tour.gif)

1. Thread milling cuts a screw thread inside a hole (a nut) or round a post (a bolt). Draw a circle at each thread and select them.

   ![Step 1](pictures/thread_milling_toolpath/step-1.png)

2. On the Toolpaths page, press **Thread Milling** (under Holes).

   ![Step 2](pictures/thread_milling_toolpath/step-2.png)

3. Choose **Internal** for a thread inside a hole, or **External** for a thread round a post.

   ![Step 3](pictures/thread_milling_toolpath/step-3.png)

4. Set the thread's **Diameter** (its largest size) and **Pitch** (the distance between ridges), or pick a standard thread.

   ![Step 4](pictures/thread_milling_toolpath/step-4.png)

5. Clear the hole first with a Pocket toolpath. Then press **Calculate**: the thread mill spirals round each centre.

   ![Step 5](pictures/thread_milling_toolpath/step-5.png)

For an internal thread, clear the hole first with a [Pocket](help:pocket_toolpath), then thread it. When it looks right in the preview, [save it for your machine](help:save_toolpaths).

The **Bit** box on the form says which bit cuts it: **Select…** picks another from the [Bit Library](help:tool_database), and **Edit…** changes its sizes, feeds and speeds there.
