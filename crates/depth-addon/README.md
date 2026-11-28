# Depth add-on

How add-ons plug into PowerWhittle (manifest, releases, signing, kinds): [docs/addons.md](../../docs/addons.md).

`cutcraft-depth` estimates a photo's depth for **Model ▸ Relief from Photo**. It is a separate download so the app and its updates stay small. It needs no Python or GPU.

- **Model:** Depth Anything V2 **Small** (Apache 2.0), which everyone gets. **Large** (CC-BY-NC-4.0: personal, non-commercial use only) is an optional download, Relief from Photo's **Get High Detail Model**, and the app says what its licence allows wherever it is offered or used. It is stored at half precision (about 670 MB). Base and Giant are not offered.
- **Credit:** Depth Anything V2, by Lihe Yang, Bingyi Kang, Zilong Huang, Zhen Zhao, Xiaogang Xu, Jiashi Feng and Hengshuang Zhao (2024), <https://github.com/DepthAnything/Depth-Anything-V2>. The Large weights are used under [CC BY-NC 4.0](https://creativecommons.org/licenses/by-nc/4.0/), changed only by conversion to half-precision safetensors.
- **Runtime:** [candle](https://github.com/huggingface/candle) (MIT / Apache 2.0) on the CPU. The release program is about 4 MB.
- **Weights:** take `depth_anything_v2_vits.pth` from `huggingface.co/depth-anything/Depth-Anything-V2-Small`, then run `cutcraft-depth convert depth_anything_v2_vits.pth depth_anything_v2_vits.safetensors` once (Large: the same with `_vitl` from `Depth-Anything-V2-Large`). The program reads which model a file holds from its weights. The program also reads the `.pth` file directly, as data only, so none of its code runs.
- **Install:** Relief from Photo's **Install Depth Add-on** button, or **Gadgets ▸ Add-ons**, installs the newest release of `NoobyNull/CutCraft-Depth` and offers its updates there. Those releases are made by `.github/workflows/depth-addon.yml` (bump `version` in `addon.toml`, then push a `depth-v<version>` tag or use Run workflow).
- **Install by hand:** put `cutcraft-depth` (`cutcraft-depth.exe` on Windows) and the `.safetensors` file in the add-ons folder's `depth/` subfolder. The paths are `~/.config/powerwhittle/addons/depth/` on Linux (`$XDG_CONFIG_HOME/powerwhittle/addons/depth/` when that is set) and `%APPDATA%\powerwhittle\addons\depth\` on Windows.
- **Run by hand:** `cutcraft-depth WEIGHTS PHOTO OUT.png` writes a 16-bit grey depth map at the photo's size, with white nearest.

The network sees the photo at 518 × 518. Relief from Photo adds back fine detail from the photo's own shading, so the depth map does not need more resolution than that.
