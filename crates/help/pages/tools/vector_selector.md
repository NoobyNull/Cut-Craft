---
reviewed: 2026-11-23
see-also: select_vectors_inside, select_all_vectors, unselect_all
---

**Vector Selector** (Edit ▸ Selection ▸ Vector Selector...) selects vectors by what they are, not
by clicking them: open ones, closed ones, circles, or circles of one size. Use it to pick every
hole of one size to drill.

## Steps

![The steps, one after another](pictures/vector_selector/tour.gif)

1. Choose **Edit ▸ Selection ▸ Vector Selector...** to select vectors by what they are, not by clicking.

   ![Step 1](pictures/vector_selector/step-1.png)

2. Pick circles only, or circles of one size (**Circle diameter**): every hole of one size, ready to drill.

   ![Step 2](pictures/vector_selector/step-2.png)

3. Press **Apply**. Every vector that passes is selected, on the layers you chose.

   ![Step 3](pictures/vector_selector/step-3.png)

## Settings

- **Open vectors** and **Closed vectors**: pick vectors whose ends don't or do meet.
- **Closed kind**: every closed shape, or circles only.
- **Circles**, **Circle diameter** and **Circle tolerance**: every circle, or only circles of one size, and how
  far off that size one may be.
- **Select vectors on**: every visible layer, the current one, or the layers you name.

Vectors on hidden or locked layers are never selected: the form says in red how many were left
out. Toolpaths can follow the same filters, so new holes join the toolpath when it is made again.
