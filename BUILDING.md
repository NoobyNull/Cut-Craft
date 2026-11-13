# Updating, building and releasing Cut Craft

Every command below is run from the repository's root folder, the one with `Cargo.toml` in it:

```bash
cd ~/cut-craft   # or wherever you cloned it
```

## One-time setup

Do this once on a new machine or a fresh clone.

1. Install Rust (stable), then the test runner the commit hook uses:

   ```bash
   cargo install cargo-nextest --locked
   ```

   Lua gadgets compile Lua from C, so a C compiler is needed too: on Windows, Visual Studio
   Build Tools with "Desktop development with C++"; on Linux, `gcc` or `clang` (usually there
   already, e.g. `sudo apt install build-essential`).

2. Turn on the commit hook. It runs `cargo fmt --check`, `cargo clippy` with every warning as
   an error, and the whole test suite before each commit:

   ```bash
   git config core.hooksPath .githooks
   ```

3. Check the two remotes. `origin` is GitHub and `gitea` is the Gitea server:

   ```bash
   git remote -v
   ```

   You should see these two push lines:

   ```text
   gitea   https://git.yax.family/matthew/cut-craft.git (push)
   origin  git@github.com:NoobyNull/cut-craft.git (push)
   ```

   GitHub has renamed the repository to `Cut-Craft` and forwards the old address, so either
   spelling works.

   If `gitea` is missing, add it:

   ```bash
   git remote add gitea https://git.yax.family/matthew/cut-craft.git
   ```

## Get the latest code

1. Make sure you are on `main` with nothing uncommitted:

   ```bash
   git switch main
   git status --short
   ```

   `git status --short` should print nothing. If it lists files, commit them (see the next
   section) or set them aside before you go on.

2. Pull the latest `main` from GitHub:

   ```bash
   git pull --ff-only origin main
   ```

   If this stops with "Not possible to fast-forward", someone else pushed while you had local
   commits. Run `git pull --rebase origin main`, fix any conflicts it reports, and then
   continue.

## Commit and push a change

1. See what changed:

   ```bash
   git status --short
   git diff
   ```

2. Stage the files you mean to commit. Name them; do not stage everything blindly:

   ```bash
   git add path/to/file another/file
   ```

3. Commit. The hook runs the full gate first, which takes a minute or two. If it fails, the
   commit is not made: fix what it reports, `git add` again, and commit again.

   ```bash
   git commit -m "Area: what changed, in one line"
   ```

   Add a line `Fixes #123` to the message to close GitHub issue 123 when the commit reaches
   GitHub.

4. Push `main` to both remotes:

   ```bash
   git push origin main
   git push gitea main
   ```

   A push to `main` on GitHub starts **CI** (Linux, Windows and macOS tests), **Release**,
   which rebuilds the rolling `nightly` pre-release for Linux and Windows, and **Release tag**,
   which tags and releases `main`'s version if it has no tag yet.

## Build and run locally

1. Build the release binary:

   ```bash
   cargo build --release -p cut-craft
   ```

   The first build takes several minutes; later ones take about three. The program is
   `target/release/cut-craft`.

2. Run it:

   ```bash
   cargo run --release -p cut-craft
   ```

3. Put it in the desktop's application menu. Do this once; after that the menu entry starts
   whatever `target/release/cut-craft` holds, so a new build is all an update needs:

   ```bash
   scripts/install-launcher
   ```

   `scripts/install-launcher --remove` takes the menu entry out again.

4. To run the same checks the commit hook runs, without committing:

   ```bash
   .githooks/pre-commit
   ```

## Cut a versioned release

The version only changes when you decide to release, and then by one step. Merging pull
requests never changes it; between releases the rolling `nightly` pre-release carries the
newest code.

- **patch** (the usual release): 0.6.0 → 0.6.1 → 0.6.2
- **minor** (a milestone): 0.6.2 → 0.7.0

Whenever `main`'s version has no tag yet, **Release tag** makes it: it tags the commit the
version arrived in and starts **Builds** on that tag, which builds, signs and publishes the
release. Nobody pushes tags by hand, and a `v*` tag is never moved or deleted (see
"Repository protection").

Afterwards, `git pull` and push `main` and the new tag to the Gitea mirror too
(`git push gitea main --tags`), since the bots push only to GitHub.

To cut a release:

1. Get the latest code and make sure the gate passes (see above), with `git status --short`
   printing nothing. Run `git fetch --tags` so the last release's tag is there.

2. Move the version one step:

   ```bash
   scripts/release-version          # 0.6.0 -> 0.6.1
   scripts/release-version minor    # 0.6.0 -> 0.7.0, for a milestone
   ```

   It updates `Cargo.toml` and `Cargo.lock`, and prints the pull requests merged since the last
   release, sorted into New and Fixed by their titles. It refuses if the current version was
   never released, since a version pull request is then already waiting.

3. Replace the bullets in `crates/app/whats-new.md` with this release's changes, using that
   list. The start page shows them under "What's new". Keep it to about four short lines, or
   the start page's links no longer fit and a test fails.

4. Take the README's pictures again, so they show this version in the status bar and the
   screens as they are now:

   ```bash
   cargo run --release -p cut-craft -- --screenshots docs/screenshots
   ```

   The app opens, sets up each scene, saves it as `docs/screenshots/NN-name.png` and quits
   (about half a minute). It needs a display: on a Linux machine without one, run it under
   `xvfb-run -a` (packages `xvfb`, `libxkbcommon-x11-0` and `mesa-vulkan-drivers`). Look over
   the pictures before committing them; a picture the README doesn't use can be left out.

5. `main` is protected, so the version goes in through a pull request like any other change.
   Commit the files on a branch and open a pull request titled `chore: version 0.6.1`:

   ```bash
   git switch -c version-0.6.1
   git add Cargo.toml Cargo.lock crates/app/whats-new.md docs/screenshots
   git commit -m "Version 0.6.1"
   git push -u origin version-0.6.1
   gh pr create --title "chore: version 0.6.1" --body "Version 0.6.1"
   ```

6. When CI is green, merge it. **Release tag** tags v0.6.1 on the merge and **Builds** makes
   the macOS DMG, Linux `.tar.gz`, AppImage and Flatpak, and Windows `.zip`, signs each and
   attaches them to the release on GitHub's Releases page. Watch with `gh run list --limit 6`.

7. Build the same version locally (see "Build and run locally").

## Repository protection

Two rulesets (Settings → Rules → Rulesets) guard the GitHub repository:

- **Protect main**: `main` can't be deleted or force-pushed, and changes reach it only through
  pull requests whose CI (`check (ubuntu-latest)`, `check (windows-latest)`) and title check
  (`kind`) pass. No review is required. Nobody bypasses it, admins included, except deploy
  keys (none is set up, and nothing needs one). To override in an emergency, set the ruleset
  to Disabled and back to Active afterwards.
- **Protect release tags**: a `v*` tag can't be deleted or moved once it exists. New tags can
  still be made, which is how **Release tag** makes them with the workflow's own token. Keep
  "Restrict creations" off in this ruleset, or Release tag can't tag.

## When something goes wrong

- **The hook fails on formatting.** Run `cargo fmt --all`, `git add` the files again and
  commit again.
- **The hook fails on clippy.** Fix what it names. "Function has too many lines" means the
  function is over 100 lines: move a part of it into a function of its own.
- **A push is refused with "fetch first".** Someone pushed first. Run
  `git pull --rebase origin main`, then push again.
- **GitHub says "This repository moved".** Point `origin` at the new address:
  `git remote set-url origin git@github.com:NoobyNull/Cut-Craft.git`.
- **A release came out wrong.** Its tag can't be moved or deleted. Fix it on `main` and cut the
  next version (`scripts/release-version`, above).
- **A version merged but no release appeared.** Open Actions → **Release tag** and read the run
  for that merge; **Run workflow** there tries again. If the tag exists but **Builds** failed,
  open Actions → Builds → **Run workflow**, leave "Use workflow from" on `main`, and type the
  tag (such as `v0.5.3`) in the box. A release goes out once Linux and Windows are built; when
  GitHub couldn't start the macOS build, run it again the same way later to add the DMG.
