# Add-ons: how they plug into Cut Craft

An add-on is something too big, too specialised or too new to ship inside Cut Craft: a
helper program and its model weights (Depth), a set of Lua gadgets, or a content pack.
This page is the contract between an add-on and Cut Craft.

**The one rule:** an add-on never shows its own interface. Cut Craft does the talking.
Users see ordinary Cut Craft tools: the same menus and toolbars, Cut Craft's forms and
wording, one Undo step per action, the shared progress display, and errors that say how
to fix the problem. They never see an add-on's command line, Python UI, parameter names or
file formats. An add-on describes itself to Cut Craft, and Cut Craft builds the UI and
translates. A design that needs the user to learn the add-on's own language is rejected.

Code: the installer is `crates/updater/src/addon.rs`, the Add-ons window is
`crates/app/src/addons.rs`, and gadget loading is `crates/app/src/gadgets.rs` with the
engine in `crates/scripting`.

## Kinds

| `kind`    | What it is | How it appears in Cut Craft | Installs today? |
|-----------|------------|-----------------------------|-----------------|
| `tool`    | A helper program (and its data) that a Cut Craft tool runs behind the scenes | Through a Cut Craft tool built for it, such as Relief from Photo's Estimate Depth | Only add-ons Cut Craft has a tool for (`addons::TOOLS`; today only `depth`) |
| `gadget`  | One or more Lua gadgets | Items in the Gadgets menu, each with a Cut Craft settings form | Yes |
| `content` | A content pack: clipart, models, textures and so on | Files in the Library | Not yet. See [content-packs.md](content-packs.md) (PR #200); `addons::usable` gains a `content` arm when the Library side lands |

Before installing, Cut Craft reads the add-on's manifest and refuses anything it cannot
use, saying why ("a tool this version of Cut Craft has no place for", "a kind of add-on
this version does not know: update Cut Craft"). Nothing is installed half-usable.

## `addon.toml`

Every add-on's zip has `addon.toml` at its top.

```toml
id = "depth"                         # its folder name: a-z, 0-9, - and _ only
name = "Depth (Relief from Photo)"   # shown to users
kind = "tool"                        # tool, gadget or content
version = "0.1.0"                    # the add-on's own version
needs_cutcraft = ">=0.5.0"           # optional: semver range of Cut Craft versions it works with
program = "cutcraft-depth"           # tool only: its program; .exe is added on Windows
downloads = ["depth_anything_v2_vits.safetensors"]  # optional: large files beside the zip
license = "LicenseRef-CutCraft AND Apache-2.0"      # informational
```

- `program` and every name in `downloads` must be plain file names, with no folders,
  `..` or absolute paths. A manifest that breaks this is refused.
- Unknown keys (`license`, `platforms` and so on) are ignored, so a manifest can carry
  notes for people.
- A manifest is read only up to 64 KB.
- `needs_cutcraft` is checked before anything is unpacked. A failure tells the user to
  update Cut Craft.

## Releases: how an add-on is published

An add-on lives in its own GitHub repository, and Cut Craft installs that repository's
**newest release** (GitHub's "latest": not a draft, not a pre-release, with a version tag
such as `v0.2.0`). A release carries:

- **The zip.** Either one per computer, named `<anything>-<os>-<arch>.zip` (`os` and
  `arch` as Rust spells them: `linux-x86_64`, `windows-x86_64`, `macos-aarch64`), or one
  `<anything>.zip` for every computer (gadgets and content packs). It holds `addon.toml`
  and the add-on's files, which may be in folders. It may unpack to 1 GB at most.
- **Downloads.** Each large file named in `downloads`, attached once beside the zips, so
  the same 100 MB of weights isn't repeated per platform.
- **Signatures** (optional). `<file>.minisig` beside each file, made by
  `scripts/sign-release` with Cut Craft's release key.

GitHub's SHA-256 checksum of every file must match, or the install is refused.

`.github/workflows/depth-addon.yml` is a worked example. It builds `cutcraft-depth` for
Linux and Windows, zips each with `addon.toml`, converts the Small model's weights and
publishes the release on `NoobyNull/CutCraft-Depth`. It does not sign (see Signing), so
it needs only the `ADDON_RELEASE_TOKEN` secret.

## Install, update and remove

**Gadgets ▸ Add-ons**:

- **Official add-ons** are listed with an Install button, their repository built in
  (`addon::OFFICIAL`). To add one, add an entry there: id, name, a one-sentence `about`,
  and its repository.
- **Install from GitHub** takes any repository, typed as `owner/name` or pasted as a link.
- Opening the window checks each installed add-on's repository. When there is a newer
  release, it offers **Update to x.y.z**. **Remove** deletes the add-on's folder.

Tools may also start an install themselves. Relief from Photo has **Install Depth
Add-on**, which installs the official Depth entry and shows progress in its own form.

Install details:

- An add-on is unpacked into `<settings folder>/addons/<id>/` (`~/.config/cut-craft/addons/`
  on Linux, `%APPDATA%\cut-craft\addons\` on Windows), with `installed.json` recording
  the repository, tag, version and whether it was signed. Updates use that record.
- Everything is first unpacked beside the old copy, then swapped in. If any step fails
  (checksum, signature, size, missing file), the old copy stays as it was and nothing
  half-made is left behind.
- A tool's `program` is made runnable on Linux.

## Signing

Matthew's rule: signing is optional and nothing is blocked for lacking it. Unsigned
add-ons install and run after a warning. Matthew chose not to sign add-ons, the official
Depth add-on included, so in practice every add-on today takes the unsigned path below.
A release can still carry signatures, and Cut Craft checks them when present.

- **Signed** means every file in the release has a `.minisig` that checks out against
  the release key built into Cut Craft (`crates/updater/release-key.pub`, the updater's
  key). The signature's trusted comment must name the file (`file:<name>`), so an old
  signed file can't be passed off as a new one. Signed add-ons install and run straight
  away.
- **Unsigned** add-ons are allowed. Every warning about one says the same thing
  (`addons::NOT_RESPONSIBLE`): nobody has checked what it does, and Cut Craft and its
  author are not responsible for what it does or can do, including anything harmful or
  malicious. A tool add-on is a program that runs on the computer with the user's
  permissions. The warning appears:
  - before it installs, and again before each update (an update is new, unchecked code);
  - the first time its program runs in a session, for a tool add-on (`Addons::may_run`).
    Saying yes covers the rest of the session; reinstalling or updating asks again;
  - before its gadgets first run each session, for a gadget add-on: they sit under My
    Gadgets and use the unsigned-gadget warning.
- **A signature that is present but does not match** refuses the install outright.
  That isn't "unsigned", it means something was changed.
- A tool add-on placed in the folder by hand, with no `installed.json`, counts as unsigned.
  Gadget add-ons are loaded only with that record; loose gadgets belong in My Gadgets.
- A signed add-on's gadgets sit beside the built-in ones.

## Tool add-ons

A tool add-on is a program Cut Craft runs, never something the user runs. For each tool
add-on there is a Cut Craft tool that:

- has its own settings form, generated from its `Params` like every other command, and
  so in Cut Craft's words;
- checks whether the add-on is installed, and if not offers an Install button rather than
  file paths;
- runs the program on a background thread, with no console window on Windows, passing
  files in a temporary folder and reading the answer back;
- turns the answer into an ordinary undoable change, refusing it if the job changed in
  the meantime.

The program's interface is therefore private to the pair. For Depth it is
`cutcraft-depth WEIGHTS PHOTO OUT.png`: a 16-bit grey depth map, white nearest (see
`crates/app/src/depth.rs` and `crates/depth-addon`). Errors go to standard error in plain
words, and Cut Craft passes them on.

**Declaring tools from the manifest (not built yet).** Today each tool add-on needs a tool
written into Cut Craft for it, which is why only ids in `addons::TOOLS` install. The
intended next step is for `addon.toml` to declare its tools so Cut Craft can draw them
without new code. Each tool would give a label, a menu or page placement, an icon, a
settings list (the same setting types gadgets use, below), and how to call the program:
arguments built from the settings, plus which document input it takes (chosen picture,
selected vectors, components) and what it returns (a depth map, vectors, a component).
Cut Craft would build the form, run the program in the background, and apply the result
as one Undo step. Until that exists, a new tool add-on starts with a Cut Craft PR that
adds its tool.

## Gadget add-ons

A gadget add-on's zip holds `addon.toml` (`kind = "gadget"`) and one or more `.lua`
files at its top. Each file is one gadget, loaded at startup into the Gadgets menu. A
newly installed gadget appears after Cut Craft restarts.

A gadget file declares itself and its settings, and does its work with Cut Craft's own
commands:

```lua
gadget = {
  name = "Hole Pattern",
  description = "Draws circles spaced evenly round a circle in the middle of the job.",
}

settings = {
  { key = "count", label = "Number of holes", type = "integer", default = 6, min = 1, max = 360 },
  { key = "pattern_diameter", label = "Pattern diameter", type = "length", default = 100, min = 0,
    help = "Shown as the setting's hint." },
  { key = "hole_diameter", label = "Hole diameter", type = "length", default = 8, min = 0.1 },
}

function run(s)
  local centre = cutcraft.job().centre
  local r = s.pattern_diameter / 2
  for i = 0, s.count - 1 do
    local a = i * 2 * math.pi / s.count
    cutcraft.run("draw_circle", {
      center = { x = centre.x + r * math.cos(a), y = centre.y + r * math.sin(a) },
      radius = s.hole_diameter / 2,
    })
  end
end
```

- **Setting types:** `integer`, `number`, `length` (in mm, shown in the job's units),
  `boolean`, `text`, and `choice` (with `choices = { "a", "b" }`). Cut Craft draws the
  form, so a gadget never draws UI.
- **The `cutcraft` table:**
  - `cutcraft.run(id, params)` runs any Cut Craft command by its id with the same
    parameters its form takes. The ids are the ones in `crates/app/src/commands.rs`.
  - `cutcraft.job()` gives the job's size, units, `area` and `centre`.
  - `cutcraft.selection()` gives the selected objects' ids.
  - `cutcraft.vector_count()` counts the vectors.
  - `cutcraft.message(text)`, or `print`, adds a note to the status bar.
- **One Undo step.** Everything a run does is one change. A gadget that fails part way
  changes nothing.
- **The sandbox:** only Lua's `string`, `table`, `math` and `utf8` libraries. There are
  no files, programs, network, `require` or compiled Lua. A run is stopped after about a
  second of work, 64 MB of memory, or 10,000 commands, and the user is told in plain
  words.

## Content packs

See [content-packs.md](content-packs.md) (PR #200). A pack's zip carries `addon.toml`
(`kind = "content"`, the same id as the pack) beside its `pack.json`. It installs through
the same window into `addons/<id>/`. The Library lists `addons/*/` folders whose kind is
`content`, checks each with `Pack::parse` and `Pack::verify`, and shows any that fail as
broken.
