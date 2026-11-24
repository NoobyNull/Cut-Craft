---
reviewed: 2026-11-24
see-also: job_setup, material_setup, new, settings
---

## Steps

![The steps, one after another](pictures/setup_material/tour.gif)

1. Cut Craft opens with a blank job, made with your Job Defaults. To change this job's material, press **Setup Material** (under File Operations, at the top of the Design page).

   ![Step 1](pictures/setup_material/step-1.png)

2. Pick the kind of job: **Single Sided** (cut from the top), **Double Sided** (turned over to cut the back) or **Rotary** (a cylinder on a rotary axis).

   ![Step 2](pictures/setup_material/step-2.png)

3. Measure your board and type its **Width**, **Height** and **Thickness**. Get the thickness right: it decides how deep a cut must go to come through.

   ![Step 3](pictures/setup_material/step-3.png)

4. The line under them says where **Z zero** and **X0 Y0** are. **More settings for this job…** changes them for this job only; **Change my defaults…** changes them for every new job.

   ![Step 4](pictures/setup_material/step-4.png)

5. Press **Apply**. The sheet on screen is now your board, ready to draw on.

   ![Step 5](pictures/setup_material/step-5.png)

## What it shows

Setup Material asks only what changes from one job to the next: what kind of job it is and
how big the material is. A **Double Sided** job also asks how the board is turned over, and a
**Rotary** job asks its shape, which way it lies and where its Z zero is, because getting
those wrong spoils the part.

Everything else (Z zero, X0 Y0, the rapid heights, home, the material and its colour) comes
from **Edit ▸ Options ▸ Job Defaults**, so you set it once for your machine. The line under
the size says what this job uses, for example *600 × 400 × 20 mm, single sided, Z zero on
top, X0 Y0 bottom left*.

- **More settings for this job…** shows the rest of [Job Setup](help:job_setup) on the same
  form. What you change there is for this job only.
- **Change my defaults…** opens the Job Defaults page in [Options](help:settings). What
  you change there is for every new job, not the one open.

Before your first toolpath, [Material Setup](help:material_setup) opens by itself, so you
look at the thickness and where to zero the bit once more for this job. If you never do, the
[checks before cutting](help:check_zeroing) remind you where to zero it.

## Settings

- **Job type**: Single Sided, Double Sided or Rotary.
- **Width (X)**, **Height (Y)**, **Thickness (Z)**: the board's size. Rotary jobs give the
  blank's **Length** and its **Shape** instead: Round with its **Diameter**, Square with its
  **Side**, or Board with its **Thickness** and **Board Width**. The 3D view shows that shape.
- **Units**: millimetres or inches, for this job.
- Double Sided: how the board is **turned over** (along X or Y).
- Rotary: which way the **material lies**, and where **Z zero** is (**Material Top** or
  **Turning Axis**).
