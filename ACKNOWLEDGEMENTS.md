# Acknowledgements

## Vectric Aspire

Thank you to [Vectric Ltd](https://www.vectric.com) for Aspire. Cut Craft was inspired by
Vectric Aspire: its tools, the way it groups them, and the order it takes you through a job
(set up the material, design, model, make toolpaths, save G-code) showed what a hobby CNC
program can be. Cut Craft follows that workflow on purpose, so people who already know Aspire
feel at home, and changes it where a clearer design helps someone new to CNC.

Cut Craft is an independent program:

- It is written from scratch in Rust. Its tools are built to behave the way Aspire's do, not
  copied from it, and none of Vectric's code is used.
- It is not made by, affiliated with, or endorsed by Vectric Ltd.
- Vectric and Aspire are trademarks of their respective owners, used here only to say where
  the inspiration came from.

### Icons

Cut Craft 0.1.0 to 0.2.1 used icons traced from Aspire's, and a few of Aspire's own dialog
pictures, as stand-ins while the tools were being written. From 0.2.2 on, every icon Cut Craft
ships was drawn for it:

- `scripts/make_icons.py` draws the flat icons from a few shared shapes in Cut Craft's own
  style, and `scripts/make_3d_icons.py` the shaded solids of the 3D tools.
- `scripts/model_pictures.py` renders the 3D import pictures from STL models.
- `scripts/draw_icons.py` draws a handful more in the same style.

The GitHub and Thingiverse logos belong to their owners and are credited, with the other
third-party assets and libraries, under **Help ▸ Third Party Licenses**.
