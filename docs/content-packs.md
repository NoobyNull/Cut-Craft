# Content packs

Optional sets of models, pictures and drawings that install into the Library. A pack is
a GitHub release of its own repository, installed and updated by the shared
install-from-repo installer (Add-ons). Once installed, its files sit in a folder of their
own that is added to the Library, so they show up there like any other files.

A pack's release zip carries `addon.toml` (kind `content`, the pack's id) beside
`pack.json`, so the add-on installer (Gadgets ▸ Add-ons) installs and updates it like any
add-on, into `<config>/addons/<id>/`. The Library then adds each installed content add-on
whose `pack.json` passes `Pack::parse` and `Pack::verify`.

## Licences

Packs may only hold items under **CC0**, **public domain** or **CC BY** (3.0 or 4.0), so a
pack's own licence never stops anyone selling what they make with it. (Whether someone may
use Cut Craft itself to make things for sale is set by Cut Craft's LICENSE, not by the pack.) Non-commercial (NC) and no-derivatives (ND)
licences are refused when a pack is read (`cut-craft-content-pack`). CC BY items need
their author and source in the manifest, and they are listed by `Pack::credits` for the
Credits page.

## `pack.json`

At the top of the pack, listing every file in it:

```json
{
  "format": "cut-craft-pack",
  "version": 1,
  "id": "staunton-chess",
  "name": "Staunton Chess Set",
  "description": "…",
  "pack_version": "1.0.0",
  "licence": "CC0-1.0",
  "author": "Cut Craft",
  "homepage": "https://github.com/NoobyNull/Cut-Craft",
  "items": [
    {
      "file": "Pieces/Pawn half.stl",
      "title": "Pawn (half)",
      "description": "…",
      "tags": ["chess", "pawn"],
      "sha256": "…64 lower-case hex digits…"
    }
  ]
}
```

- `id`: lower-case letters, digits and hyphens; the folder the pack installs into.
- `file`: a path inside the pack with `/` between folders on every system. Absolute
  paths, `..`, `\` and `:` are refused.
- `licence`, `author`, `source` may be set per item, overriding the pack's.
  Licence names: `CC0-1.0`, `public-domain`, `CC-BY-4.0`, `CC-BY-3.0`.
- `sha256`: `Pack::verify` checks every unpacked file against it.

## Staunton chess set (`crates/packs/staunton-chess`)

A full set modelled from scratch on the 1849 Staunton pattern and released as CC0.
`cargo run --release -p pack-staunton-chess -- <folder>` writes the pack:

- `Pieces/<Piece> half.stl` for all six pieces: half a piece lying flat, at most 18 mm
  thick so it fits 3/4″ wood; cut two and glue them face to face. King 95 mm tall.
- `Pieces/Knight two-sided.stl`: the whole knight, 31 mm thick, for 1½″ wood.
- `Tutorial - Two-sided knight.ccjob`: a double-sided job with toolpaths for both sides
  (1/4″ end mill roughing, 1/8″ ball nose finishing), a frame and two tabs that hold the
  knight, and two 1/4″ dowel pins on the flip line. Its steps are job notes starting with
  `.`, so they open with the job.

The pieces are made so a cutter from above reaches all of them (no overhangs), which is
what lets each half, and each side of the knight, be cut on a 3-axis router.
