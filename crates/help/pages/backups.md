---
reviewed: 2026-11-25
see-also: settings, updates, first_time_setup, interface_layout
---

# Backing up your settings

Everything you set up in PowerWhittle (your settings and window layout, your machines and post
processors, the Bit Library, templates and the rest) can be saved in one **backup file**. Keep
it somewhere safe, or take it to a new computer.

PowerWhittle also makes backups by itself, so you have one from just before anything big
changes.

## Saving a backup yourself

1. Choose **Help ▸ Back Up Settings...**.
2. Every part is ticked to start with. **Untick** any you don't want in this backup.
   Point at a part to see what it holds.
3. Press **Save Backup...** and choose where to save it. The file ends in `.ccbackup`.

The status bar says where the backup was saved.

### What goes in, and what doesn't

| Part | What it holds |
|---|---|
| Settings and window layout | Everything in **Edit ▸ Options**, the layout, your favourite tools and Job Defaults |
| Machines and post processors | Your machines from **Machine Configuration**, and the post processors you added |
| Bit Library | Your bits, their groups and their cutting data |
| Materials Library | The materials and their settings |
| Templates | Your job, toolpath and form settings templates |
| Brushes, textures and fonts | Sculpting brushes, textures and fonts you added |
| My Gadgets | The gadgets you added |
| Library folders and tags | The folders the Library looks in, and your tags |
| Spelling words | The words you added to the spelling list |
| Add-ons list | Which add-ons are installed. The add-ons themselves are downloaded again, so a backup stays small |
| Unsigned code you allowed | The unsigned add-ons and gadgets you said may run |

Never in a backup: your **recent files list**, unsaved work kept in case PowerWhittle closes
unexpectedly, logs, and pictures PowerWhittle can make again.

**The Web API access token** is left out unless you tick **Include the Web API access token**.
Anyone with a backup that holds it could drive PowerWhittle over your network, so keep that
backup private. The key for the AI border tiles stays in your computer's password store and
is never put in a backup.

## The backups PowerWhittle makes by itself

A backup is made just before:

- an **update** is installed,
- the **window layout** changes (**File ▸ Window Layout**, or the layout question),
- **first-time setup** is run again, which changes My Machine,
- PowerWhittle changes a library itself, as when it joined each bit's copies for different
  materials into one.

It also makes one **every week**, when PowerWhittle starts. If nothing has changed since a
backup it already has, no new one is made.

They are kept in a `backups` folder inside PowerWhittle's settings folder:

| Computer | Folder |
|---|---|
| Windows | `%APPDATA%\cut-craft\backups` |
| Mac | `~/Library/Application Support/cut-craft/backups` |
| Linux | `~/.config/cut-craft/backups` |

## Options ▸ Backups

**Edit ▸ Options ▸ Backups** lists the backups kept, newest first, with when each was made
and why. Its settings:

- **Back up every week**: on to start with.
- **Backups to keep**: 10 to start with. When there are more, the oldest automatic backup is
  removed. Backups you save yourself, even into that folder, are never removed.

**Back Up Now...** opens **Help ▸ Back Up Settings**, and **Open Backups Folder** shows
the folder.

## Restoring a backup

Restoring a backup from inside PowerWhittle, with a look at what would change before anything
does, is coming in a later version. Until then a backup is an ordinary zip file: its
`settings` folder holds the files as they were, and they can be copied back into the settings
folder by hand while PowerWhittle is closed.

See also: [Changing settings](help:settings), [Updates](help:updates).
