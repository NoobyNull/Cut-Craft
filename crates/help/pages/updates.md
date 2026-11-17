---
reviewed: 2026-11-17
see-also: web_api, contents
---

# Updates

Cut Craft keeps itself up to date in one click. About once a day, when it starts, it asks
GitHub whether there is a new version. If there is, it asks you:

**A new version (0.6.14) is available. Download, install and relaunch?**

![The Updates window offering a new version, with Install Now, Later and Skip This Version](pictures/update-offer.png)

- **Install Now** does the whole update for you (see below).
- **Later** keeps the version you have. You are asked again next time Cut Craft starts.
- **Skip This Version** stops Cut Craft offering that version when it starts. A newer one
  is still offered, and **Help ▸ Check for Updates** still finds it.

**Help ▸ Check for Updates** asks GitHub straight away, whenever you like.

## What Install Now does

![Install Now: downloading, checking the download, then Cut Craft reopening on the same job](pictures/update-steps.gif)

1. **Downloads** the new version from GitHub, with a bar showing how far it has got. You can
   close the window and keep working while it downloads.
2. **Checks** the download is a genuine Cut Craft release, signed by its maker, and that it
   arrived whole. A download that fails either check is thrown away.
3. **Installs** it in place of the version you are running.
4. **Closes and reopens Cut Craft.** If your job has changes that are not saved, you are
   asked whether to save them first, as when you close Cut Craft yourself. The new version
   then opens on the same job.

If you press **Cancel** at the save question, Cut Craft stays open and the Updates window
offers **Relaunch Now** (when you are ready) or **Relaunch Later** (the new version starts
next time you open Cut Craft).

Nothing is downloaded until you press **Install Now**, and your jobs, settings, tools and
machines are never touched by an update.

## If something goes wrong

If the download stops, or it does not pass the checks, the version you have is left exactly
as it was and the window says what happened. Most of the time the fix is one of these:

- **Could not reach GitHub**: check this computer is online, then press **Try Again**.
- **GitHub is not answering for a while**: GitHub limits how often one computer can ask.
  Wait an hour and try again.
- **Not allowed to change its own files**: Cut Craft was installed for every user of the
  computer, in a folder only an administrator can change. Press **Open the Download Page**
  and install the new version the same way you installed this one.
- **The signature or checksum does not match**: the download was damaged on the way.
  Nothing was installed. Try again later, or download it from the release page.

## Copies that update another way

Some copies of Cut Craft are looked after by something else, which owns their files. For
these the window says how to update instead, and each command has a **Copy** button so you
can paste it into a terminal:

![The Updates window for a Flatpak, with the install command and its Copy button](pictures/update-by-hand.png)

- **Installed with apt, dnf or an AUR helper** (a .deb, .rpm or AUR package): update it the
  way you update the rest of the computer, for example `sudo apt update && sudo apt upgrade`
  or `sudo dnf upgrade`. These copies do not check at startup, since the package manager
  offers new versions itself.
- **Flatpak**: download the new `.flatpak` file from the release page, then open it, or run
  `flatpak install --user` with its name in the folder you saved it to.
- **Mac**: download the new `.dmg` and drag Cut Craft to Applications again.
- **Built from the source code**: `git pull`, then `cargo build --release`.

The AppImage, the Linux `.tar.gz` and the Windows `.zip` replace themselves. An AppImage
named for its version (`CutCraft-0.6.13-x86_64.AppImage`) takes the new version's name, and
the shortcuts that started the old one (in `~/.local/bin` and the application menu) start
the new one.

## Turning the check off

Turn off **Edit ▸ Options ▸ Updates ▸ Check for updates when Cut Craft starts** to stop the
startup check. **Help ▸ Check for Updates** still works.

See also: [Help Contents](help:contents)
