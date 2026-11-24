---
reviewed: 2026-11-23
see-also: join_vectors, join_open_ends, select_all_open_vectors
---

**Join / Close Vectors** joins two open vectors into one at their closest ends, or closes one
open vector. Right-click the selection ▸ **Join / Close Vectors...**.

## Steps

![The steps, one after another](pictures/join_close_vectors/tour.gif)

1. Select two open vectors to join, or one to close.

   ![Step 1](pictures/join_close_vectors/step-1.png)

2. Right-click and choose **Join / Close Vectors...**, then pick how the gap is filled.

   ![Step 2](pictures/join_close_vectors/step-2.png)

3. Press **Apply**. The two are one vector now, joined at their closest ends.

   ![Step 3](pictures/join_close_vectors/step-3.png)

## Settings

**Method**:

- **Straight line**: a straight line bridges the gap.
- **Smooth curve**: a curve that carries on smoothly from both ends.
- **Move end points**: both ends move to meet halfway, with no new line.

To join many vectors whose ends already touch, use [Join Vectors](help:join_vectors) (J).
