# Language packs

Each folder here is one language pack: its `addon.toml` plus `.ftl` files with the same
names and keys as English (`crates/i18n/locales/en/`). Packs are **not** part of PowerWhittle.
The [Language packs workflow](../.github/workflows/language-packs.yml) zips and signs each
folder as `language-<code>.zip` and publishes them on
[NoobyNull/CutCraft-Translations](https://github.com/NoobyNull/CutCraft-Translations).
Each pack is downloaded on its own from Gadgets ▸ Add-ons ▸ Languages.

Packs are **official, community supported**: one that passes the checks is signed and
published automatically, and its wording is written and kept up by its translators (named
in `translators` in its `addon.toml`). Wording problems go to the Translations repository's
Issues.

How to make or update a pack: [docs/translating.md](../docs/translating.md).
