---
reviewed: 2026-11-23
see-also: clipart_library, orientate_3d_model, decorative_relief, relief_from_photo, addons
---

## Steps

![The steps, one after another](pictures/generate_border/tour.gif)

1. Generate Border makes one repeating length of picture-frame moulding in a style you pick, ready to carve. It needs the free Borders add-on and a Google key of your own.

   ![Step 1](pictures/generate_border/step-1.png)

2. On the Design page, under **Model**, press **Generate Border**.

   ![Step 2](pictures/generate_border/step-2.png)

3. Pick a **Style**, type anything to add under **Details**, and set the border's **Length** and **Depth**.

   ![Step 3](pictures/generate_border/step-3.png)

4. Type your Google key, press **Save Key**, then press **Generate**. It takes about half a minute.

   ![Step 4](pictures/generate_border/step-4.png)

5. The border is saved in your **Library**, under **Generated ▸ Borders**, its photo on its tile. Double-click it, or drag it onto the job, to place it like any 3D model.

   ![Step 5](pictures/generate_border/step-5.png)

## What you get

Each border is saved as a pair with one name: a **depth map** (a grey picture, white the
highest) that is carved, and a **photo** of the same border that the Library shows on its
tile. A small recipe file beside them remembers the style and the size you asked for, so the
border comes into any job at that size, in millimetres or inches, whichever the job uses.

![A border's depth map, left, and its photo, right](pictures/generate_border/depth-and-photo.png)

The border repeats from left to right: its two ends are made to meet, so copies placed end
to end run on without a step. Place it with [Orientate 3D Model](help:orientate_3d_model),
as you would a 3D model from the [Library](help:clipart_library).

## The settings

- **Name**: what the border is called in the Library. If the name is taken, a number is
  added, so nothing you made before is ever replaced.
- **Style**: Classical Baroque, Modern Minimalist, Art Deco, Rustic Vintage, Victorian
  Carved, Contemporary Metallic, Rococo, Mid-Century Modern or Gothic Pewter. **Your Own
  Description** uses only what you type under Details.
- **Details**: anything to add, in your own words, such as "laurel leaves" or "a twisted
  rope".
- **Shape**: how tall the border is for its length: Narrow (21:9), Medium (16:9) or Wide
  (4:3).
- **Length**: one repeat, end to end. Its height follows from the shape.
- **Depth**: how far its highest point stands above its lowest.

Under **Depth**:

- **Depth from**: **Google** turns the picture it drew into a depth map, so the depth
  matches the picture. **Depth add-on** works it out on your own computer instead, free and
  with nothing more sent, using the [Depth add-on](help:relief_from_photo). If your copy of
  the Depth add-on is not official, PowerWhittle asks before running it too, just as it asks
  about the Borders add-on (see [Running unsigned code](help:addons)).
- **Depth curve**: below 1 the middle heights rise, for a fuller carving; above 1 they fall,
  for crisper tops. 1 leaves them as they came.
- **Smoothing**: smooths the small steps between greys so the carving isn't terraced.
- **Seam blend**: how far in from each end the two ends are blended to meet.

## Your Google key

The pictures are drawn by Google's image model, using a key of your own. Get one free from
Google AI Studio (the form's **Get a free Google key** link opens the page), paste it into the
**Google key** box and press **Save Key**.

- The key is kept in your computer's password store (Windows Credential Manager, the macOS
  Keychain, or the Linux keyring). If the computer has none, it goes in a private file in
  PowerWhittle's settings folder that only you can read.
- It is never saved in your jobs, your settings, exports or bug reports.
- **Forget Key** removes it from this computer.
- Generating sends Google only the style and your Details. Google bills what you generate
  to your own account; PowerWhittle charges nothing.

## The Borders add-on

The first time, the form shows **Install Borders Add-on**. Press it once: it downloads the
small add-on (about 5 MB) from GitHub and keeps it for next time. To update or remove it,
use [Add-ons](help:addons).

## When something goes wrong

What went wrong shows in red at the foot of the form. The usual ones:

- **No key**: type your key and press Save Key.
- **Google turned the key down**: check it in Google AI Studio, then save it again.
- **Google would not draw it**: some words are refused. Change your Details and try again.
