# Depth add-on

`cutcraft-depth` estimates a photo's depth for **Model ▸ Relief from Photo**. It is a separate download so the app and its updates stay small. It needs no Python or GPU.

- **Model:** Depth Anything V2 **Small** (Apache 2.0). Base, Large and Giant are CC-BY-NC-4.0 and must not be shipped.
- **Runtime:** [candle](https://github.com/huggingface/candle) (MIT / Apache 2.0) on the CPU. The release program is about 4 MB.
- **Weights:** take `depth_anything_v2_vits.pth` from `huggingface.co/depth-anything/Depth-Anything-V2-Small`, then run `cutcraft-depth convert depth_anything_v2_vits.pth depth_anything_v2_vits.safetensors` once. The program also reads the `.pth` file directly, as data only, so none of its code runs.
- **Install by hand:** put `cutcraft-depth` (`cutcraft-depth.exe` on Windows) and the `.safetensors` file in the add-ons folder's `depth/` subfolder. Relief from Photo's **Open Add-ons Folder** button opens that subfolder. The paths are `~/.config/cut-craft/addons/depth/` on Linux and `%APPDATA%\cut-craft\addons\depth\` on Windows.
- **Run by hand:** `cutcraft-depth WEIGHTS PHOTO OUT.png` writes a 16-bit grey depth map at the photo's size, with white nearest.

The network sees the photo at 518 × 518. Relief from Photo adds back fine detail from the photo's own shading, so the depth map does not need more resolution than that.
