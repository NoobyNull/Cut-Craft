# Linux packages and where they are published

Every release (`.github/workflows/builds.yml`) attaches, besides the AppImage, Flatpak and
`.tar.gz`, a `.deb` and an `.rpm` made by `scripts/linux-distro-packages`. Both install:

| File | What it is |
|---|---|
| `/usr/bin/cut-craft` | the program |
| `/usr/share/applications/family.yax.CutCraft.desktop` | the menu entry |
| `/usr/share/icons/hicolor/*/apps/family.yax.CutCraft.*` | the icon (SVG, and PNGs at 48, 128 and 256) |
| `/usr/share/doc/cut-craft/copyright` (.deb), `/usr/share/licenses/cut-craft/LICENSE` (.rpm) | the licence |
| `/usr/lib/cut-craft/packaged-by` | one sentence on how this package is updated |

`packaged-by` tells Cut Craft that a package manager owns it: the updater
(`crates/updater/src/install.rs`) then never replaces the program, doesn't check for updates
at startup, and Help ▸ Check for Updates shows that sentence. A program anywhere in `/usr`
is treated the same way even without the note.

## Where they are published

Each is off until its repository variable is `true` (Settings ▸ Secrets and variables ▸
Actions ▸ Variables), so nothing is published by accident.

| Where | Workflow | Switch | Needs |
|---|---|---|---|
| AUR, `cut-craft-bin` (Arch, CachyOS, Manjaro) | `aur.yml` | `PUBLISH_AUR` | an AUR account with an SSH key; the private key as the secret `AUR_SSH_KEY` |
| apt and dnf repositories on GitHub Pages | `linux-repo.yml` | `PUBLISH_LINUX_REPOS` | the secret `REPO_SIGNING_KEY` (`scripts/make-repo-key`); Settings ▸ Pages ▸ Source: GitHub Actions |

Both run after **Builds** finishes on a `v*` tag, and can be started by hand from the Actions
tab. The AUR one runs `scripts/aur-update`, which points `packaging/aur/cut-craft-bin/PKGBUILD`
at the release, refuses a download not signed with Cut Craft's release key, builds it on Arch
and pushes the PKGBUILD and `.SRCINFO`. The repository one takes the newest three releases'
`.deb` and `.rpm` (each checked against the release key), signs them and the indexes with the
repository key (`scripts/linux-repo`) and publishes the site, with setup instructions on
its front page.

### Not published, and why

- **Debian, Ubuntu and Fedora's own archives, Copr, Launchpad PPAs:** free-software licences
  only; PolyForm Noncommercial is not one.
- **Cloudsmith's free plan:** open-source projects only. **packagecloud:** from US$89 a month.
- **Flathub:** builds everything from source on its own servers, and its rules for
  AI-assisted code and submissions would need a careful look first. The `.flatpak` on each
  release installs anywhere Flatpak does.

## Setting it up (once)

1. **AUR:** make an account at <https://aur.archlinux.org/register>, then
   `ssh-keygen -t ed25519 -f ~/.ssh/aur -C cut-craft-aur` and paste `~/.ssh/aur.pub` into
   *My Account ▸ SSH Public Key*. Store the private key: `gh secret set AUR_SSH_KEY -R
   NoobyNull/Cut-Craft < ~/.ssh/aur`. Put your AUR name in the PKGBUILD's `# Maintainer:` line.
2. **apt and dnf:** run `scripts/make-repo-key` (it makes the key and stores the secret), then
   Settings ▸ Pages ▸ Source: **GitHub Actions**.
3. Set `PUBLISH_AUR` and `PUBLISH_LINUX_REPOS` to `true`, and run each workflow once from the
   Actions tab (or wait for the next release).

The first release to publish should be one whose updater knows about `packaged-by`
(0.6.6 or later): 0.6.5 and earlier would try to replace themselves and fail, as `/usr/bin`
isn't theirs to write.
