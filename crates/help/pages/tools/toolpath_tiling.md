---
reviewed: 2026-11-23
see-also: save_toolpaths, material_setup
---

## Steps

![The steps, one after another](pictures/toolpath_tiling/tour.gif)

1. Tiling cuts a job bigger than your machine in pieces: the board is moved along between tiles. Open the **Output** tab on the Toolpaths page and press **Toolpath Tiling** (under Big Jobs).

   ![Step 1](pictures/toolpath_tiling/step-1.png)

2. Tick it on, and set how the job is split into tiles: their size and how much they overlap.

   ![Step 2](pictures/toolpath_tiling/step-2.png)

3. Press **Apply**. The tiles are drawn on the job, and **Save Toolpaths** writes one file for each tile.

   ![Step 3](pictures/toolpath_tiling/step-3.png)


libEGL warning: DRI3 error: Could not get DRI3 device
libEGL warning: Ensure your X server supports DRI3 to get accelerated rendering
18:57:51.505 WARN  egui_wgpu: Software rasterizer detected - loss of performance expected. backend: Gl, device_type: Cpu, name: "llvmpipe (LLVM 20.1.2, 256 bits)", driver_info: "4.5 (Core Profile) Mesa 25.2.8-0ubuntu0.24.04.4", vendor: Mesa (0x10005), subgroup_size: 4..=128, transient_saves_memory: None, limit_bucket: None
18:57:51.520 INFO  cut_craft::app: GPU: llvmpipe (LLVM 20.1.2, 256 bits) (Gl, driver: ), buffers up to 134 MB
18:57:51.520 WARN  cut_craft::app: GPU can't run the 3D view's shaders: drawing it without them, grid work on the processor
18:57:51.609 INFO  cut_craft::trace: Tool: New (new)
18:57:53.700 INFO  cut_craft::trace: Tool: Laser Picture (laser_picture_toolpath)
18:57:56.749 INFO  cut_craft::trace: Started: Calculating Laser Picture
18:57:56.749 INFO  cut_craft::app::background: Laser Picture: settings {"name":"Laser Picture","power":60.0,"passes":1,"strategy":"raster","dither_image":false,"line_spacing":10.0,"line_angle":0.0,"tool":{"name":"Laser (0.15 mm kerf)","shape":"end_mill","tip_radius":1.0,"angle":90.0,"diameter":0.15,"pass_depth":3.0,"stepover":0.4,"feed_rate":1500.0,"plunge_rate":1500.0,"spindle_speed":0.0,"taper":0.0,"shank_diameter":0.0,"flute_length":0.0,"flutes":2,"rate_units":"mm…
18:57:56.753 INFO  cut_craft::app::background: Laser Picture: done, changed 1 toolpath, 32452 moves
18:57:56.753 INFO  cut_craft::trace: Finished: Calculating Laser Picture in 0.00 s
