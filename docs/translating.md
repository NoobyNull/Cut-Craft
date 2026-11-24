# Translating Cut Craft

Cut Craft's words live in [Fluent](https://projectfluent.org) files (`.ftl`), not in its
code. English is built in (`crates/i18n/locales/en/`). Every other language is a
**language pack**: a separate download that someone installs from Gadgets ▸ Add-ons ▸
Languages and picks in Edit ▸ Options ▸ Language. A pack never ships inside Cut Craft.

Anything a pack doesn't translate shows in English, so a pack can be published while it is
still partway done.

## Make a pack

1. Copy the example pack's folder and rename it for your language's code, such as
   `language-packs/de/` for German or `language-packs/pt-BR/` for Brazilian Portuguese.
2. Edit its `addon.toml`:

   ```toml
   id = "language-de"        # "language-" and the code, in lower case (language-pt-br)
   name = "Deutsch"          # the language's name in itself
   kind = "language"
   language = "de"           # the code
   version = "0.1.0"         # raise it each time the pack changes
   needs_cutcraft = ">=0.7.0"
   translators = ["Your Name"] # how you want to be credited; add each helper
   ```

   `translators` shows in Add-ons and Options as **Translated by …**.

3. Copy any English file you want to translate from `crates/i18n/locales/en/`, keeping its
   name, and translate the words after each `=`. Keep everything else exactly as it is:

   ```ftl
   # English (start.ftl)
   start-whats-new = What's new in v{ $version }
   commit-calculate = Calculate
       .tooltip = Work out the toolpath with these settings

   # German
   start-whats-new = Neu in v{ $version }
   commit-calculate = Berechnen
       .tooltip = Werkzeugweg mit diesen Einstellungen berechnen
   ```

   - **Keys** (left of `=`) never change. A key English doesn't have is never shown.
   - **`.tooltip`, `.disabled` and other attribute lines** must be indented, as above, or
     the file won't read.
   - **`{ $name }`** is a value Cut Craft puts in, such as a number or a file name. Keep it,
     with the same name, anywhere in your sentence.
   - You may leave a key out. English shows in its place.

4. **Counts** use plural forms. English has `one` and `other`. Use the forms your language
   has, for example Russian `one`, `few`, `many` and `other`:

   ```ftl
   tabs-placed = { $count ->
       [one] { $count } перемычка
       [few] { $count } перемычки
       [many] { $count } перемычек
      *[other] { $count } перемычки
   }
   ```

   The `*` marks the form used when no other fits. Every count needs one.

5. **Menu and window names inside sentences**, such as "Edit ▸ Options ▸ Job Defaults",
   should use the same words as your translation of those menus, so people can follow them.

## Tool names, groups, tabs and menus

The tools' names and everything that groups them are in four files of their own, so you
can translate them on their own:

| File | What it holds | Key |
| --- | --- | --- |
| `tools.ftl` | Every tool's name: its button, Find Tool (Ctrl+K), the menus and its help page | `tools-` and the tool's id, such as `tools-draw-circle` |
| `tool-groups.ftl` | The names over groups of tools, and the captioned boxes of tools that do one job different ways | `group-` and the English name, such as `group-create-vectors` |
| `subtabs.ftl` | The Design and Toolpaths sub-tabs (with `.hint`, their tooltip), the side tabs, the browser tabs and the Ribbon's tabs | `design-subtab-…`, `toolpaths-subtab-…`, `page-tab-…` |
| `menu-items.ftl` | The menu bar's items and a tool's and a sub-tab's right-click menus | `menu-item-` and the English name |

A menu item named just as the tool it runs (Edit ▸ Draw Circle) isn't in `menu-items.ftl`:
it uses the tool's words from `tools.ftl`, so you translate each tool once.

A tool's key comes from its id, which never changes, so your translation stays when its
English name is reworded. Group and menu keys come from their English names: if one is
reworded, its key changes too, and the new one shows in English until it's translated.

Keep tool names short. A Ribbon or Rail button has room for about as much as the English.

## Words to keep in English

- **G-code, GRBL, Mach, FluidNC, DXF, SVG, STL** and other file and machine names.
- **Cut Craft** itself.
- Never name other CAD/CAM programs. Cut Craft's checks refuse them.

## Test a pack

- `cargo nextest run -p cut-craft-i18n --profile languages` reads every pack in
  `language-packs/`. It fails on any line Fluent can't read, a key given twice, a key English
  doesn't have, or a missing `addon.toml` field.
- To see it in Cut Craft without publishing it, copy your pack's files into the add-ons
  folder, in a folder named as its `id` (such as `language-de`). The add-ons folder is
  `~/.config/cut-craft/addons/` on Linux, `%APPDATA%\cut-craft\addons\` on Windows and
  `~/Library/Application Support/cut-craft/addons/` on a Mac. Beside the `.ftl` files, add an
  `installed.json` that says where it came from:

  ```json
  {"repo": {"owner": "NoobyNull", "name": "CutCraft-Translations"},
   "tag": "v0.0.0", "version": "0.1.0", "signed": false}
  ```

  Start Cut Craft and pick it in Edit ▸ Options ▸ Language. It shows as not official, which
  is right for a pack you made yourself. Words that are too long for their place show
  cut off. Shorten them, as nothing in Cut Craft moves to make room.
- Edit ▸ Options ▸ Language ▸ **Test language** (with Show advanced settings ticked) shows
  English stretched by 40%. It's a quick way to see where long words won't fit.

## Publish

Open a pull request with your folder. Once it's merged, the **Language packs** workflow
(`.github/workflows/language-packs.yml`) zips each pack as `language-<code>.zip`, signs it
with Cut Craft's release key (so it shows as official), and publishes it on
[NoobyNull/CutCraft-Translations](https://github.com/NoobyNull/CutCraft-Translations).
Cut Craft offers an update to anyone who has the pack installed.

### Official, community supported

Every pack that passes the checks (`cargo nextest run --profile languages`) is signed and
published automatically: nobody reads it by hand first. Cut Craft shows such a pack as
**Official, community supported**. "Official" means it was checked and signed, holds only
words and can't run anything. "Community supported" means its wording is written and kept
up by its translators, not by Cut Craft's author. When someone reports a wrong or missing
word (from Edit ▸ Options ▸ Language, or the report form while a pack is in use), it goes to
the Translations repository's Issues, for the translators to answer.

Translations made with a machine or an AI should be checked by a native speaker who knows
woodworking and CNC words before they're published.
