# Language packs

Each folder here is one language pack: its `addon.toml` plus `.ftl` files with the same
names and keys as English (`crates/i18n/locales/en/`). Packs are **not** part of Cut Craft.
The [Language packs workflow](../.github/workflows/language-packs.yml) zips and signs each
folder as `language-<code>.zip` and publishes them on
[NoobyNull/CutCraft-Translations](https://github.com/NoobyNull/CutCraft-Translations).
Each pack is downloaded on its own from Gadgets ▸ Add-ons ▸ Languages.

How to make or update a pack: [docs/translating.md](../docs/translating.md).
