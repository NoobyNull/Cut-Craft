---
reviewed: 2026-11-25
see-also: web_api, contents, settings
---

# Updates

Cut Craft keeps itself up to date in one click. Every time it starts, it asks
GitHub whether there is a new version. If there is, it asks you:

**A new version (0.6.14) is available. Download, install and relaunch?**

![The Updates window offering a new version, with Install Now, Later and Skip This Version](pictures/update-offer.png)

- **Install Now** does the whole update for you (see below).
- **Later** keeps the version you have. You are asked again next time Cut Craft starts.
- **Skip This Version** stops Cut Craft offering that version when it starts. A newer one
  is still offered, and **Help ▸ Check for Updates** still finds it.

**Help ▸ Check for Updates** asks GitHub straight away, whenever you like. So does the
version number in the bottom-right corner of the window: right-click it (for example
**v0.7.6**) and choose **Check for Updates...**.

![Right-clicking the version number in the bottom-right corner, with Check for Updates in its menu](pictures/update-version-menu.png)

### Why a friend was offered a version before you

Each new version is built separately for Linux, Windows and Mac, and each is added to the
release page as soon as it is ready. Cut Craft only offers you a version once the download
for **your** kind of computer is there and signed. So for a little while after a release,
a Linux computer may be offered it while a Windows computer or a Mac still says it is up to
date. Check again later and it will be there. You are never offered a version you cannot
download.

Each kind of computer's download is only added once its own tests pass. If one kind is held
back long enough that a newer version comes out first, it skips the older version and goes
straight to the newer one. So now and then Linux, Windows and Mac are on different version
numbers. That is expected: each is offered the newest version made for it.

## What Install Now does

![Install Now: downloading, checking the download, then Cut Craft reopening on the same job](pictures/update-steps.gif)

1. **Downloads** the new version from GitHub, with a bar showing how far it has got. You can
   close the window and keep working while it downloads.
2. **Checks** the download is a genuine Cut Craft release, signed by its maker, and that it
   arrived whole. A download that fails either check is thrown away.
3. **Installs** it in place of the version you are running.
4. **Closes and reopens Cut Craft, keeping your work.** You are not asked to save first.
   Changes you have not saved yet are kept in a copy in Cut Craft's settings folder, and
   the new version opens them just as you left them: the same job name, still marked as
   not saved (the **\*** in the title). A job you never saved comes back as **Untitled**.
   Your job's own file is not changed until you press **Save**. A job with nothing unsaved
   simply opens again from its file.

The copy is kept in the settings folder, not the computer's temporary folder, because some
computers empty that folder when they restart. It stays until you save or close the job,
so if anything goes wrong while the new version starts, the next start opens it again (see
[the recovery copy](help:saving_jobs)).

**Relaunch Now** keeps your work the same way.

If the copy can't be written (a full disk, for example), you are asked whether to save
first, as when you close Cut Craft yourself. If you press **Cancel** there, Cut Craft stays
open and the Updates window offers **Relaunch Now** (when you are ready) or **Relaunch
Later** (the new version starts next time you open Cut Craft).

Keeping your work without asking is done by the version you are updating **from**, so it
starts with updates made from version 0.7.6 or later. Updating from an older version still
asks whether to save first.

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
- **A link or Open the Download Page does nothing**: links open in this computer's default
  web browser. Versions before 0.7.1 never opened links (on every system, not only the Mac).
  In those, type the release page's address into your browser yourself:
  `github.com/NoobyNull/Cut-Craft/releases`.

## Where updates come from

Cut Craft's releases are on GitHub, and they are moving to the Power-Whittle organisation's
page, `github.com/Power-Whittle/PowerWhittle`. While they move, each release goes on both
pages. Cut Craft looks at both and offers you the newest one built for your computer,
whichever page has it first, so there is nothing to change on your side. Every download
is still checked against Cut Craft's own signature before anything is installed, whichever
page it comes from.

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
- **Mac**: Cut Craft tells you when a new version's `.dmg` is ready. Download it and drag
  Cut Craft to Applications again.
- **Built from the source code**: `git pull`, then `cargo build --release`.

The AppImage, the Linux `.tar.gz` and the Windows `.zip` replace themselves. An AppImage
named for its version (`CutCraft-0.6.13-x86_64.AppImage`) takes the new version's name, and
the shortcuts that started the old one (in `~/.local/bin` and the application menu) start
the new one.

## Turning the check off

Turn off **Edit ▸ Options ▸ Updates ▸ Check for updates when Cut Craft starts** to stop the
startup check. **Help ▸ Check for Updates** still works.

See also: [Help Contents](help:contents)
