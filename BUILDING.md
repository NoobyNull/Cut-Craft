# Updating, building and releasing Cut Craft

Every command below is run from the repository's root folder, the one with `Cargo.toml` in it:

```bash
cd ~/aspire-clone
```

## One-time setup

Do this once on a new machine or a fresh clone.

1. Install Rust (stable), then the test runner the commit hook uses:

   ```bash
   cargo install cargo-nextest --locked
   ```

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

   A push to `main` on GitHub starts **CI** (Linux, Windows and macOS tests) and **Release**,
   which rebuilds the rolling `nightly` pre-release for Linux and Windows.

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

Versions are automatic. Each night **Version bump** reads the titles of the pull requests
merged since the last version tag (see [CONTRIBUTING.md](CONTRIBUTING.md)), and if any of them
changes the program it commits "Version X.Y.Z", tags vX.Y.Z and starts **Release** and
**Builds** on the tag, which build and publish the release. To release before the night,
open Actions → Version bump → **Run workflow**. Afterwards, `git pull` and push `main` and the
new tag to the Gitea mirror too (`git push gitea main --tags`), since the bot pushes only to GitHub.

The night's job starts working once `main`'s version has a tag, so the first one (0.5.1) is
cut by hand, the same way you'd cut one to fix a failed night:

1. Get the latest code and make sure the gate passes (see above), with `git status --short`
   printing nothing.

2. Set the version. In the root `Cargo.toml`, under `[workspace.package]`, change the line
   `version = "0.4.3"` to the new version, then update the lock file to match:

   ```bash
   cargo update -w
   ```

3. Replace the bullets in `crates/app/whats-new.md` with this release's changes. The start
   page shows them under "What's new". Keep it to about four short lines, or the start page's
   links no longer fit and a test fails.

4. Commit the three files:

   ```bash
   git add Cargo.toml Cargo.lock crates/app/whats-new.md
   git commit -m "Version 0.5.1"
   ```

5. Tag the commit:

   ```bash
   git tag -a v0.5.1 -m "Cut Craft 0.5.1"
   ```

6. Push `main` and the tag to both remotes:

   ```bash
   git push origin main v0.5.1
   git push gitea main v0.5.1
   ```

7. Watch GitHub build the packages:

   ```bash
   gh run list --limit 6
   ```

   The tag starts **Release** (Linux `.tar.gz`, Windows `.zip`) and **Builds** (macOS DMG,
   Linux AppImage and Flatpak, Windows). When they finish, the files are attached to the
   release on GitHub's Releases page.

8. Build the same version locally (see "Build and run locally").

## When something goes wrong

- **The hook fails on formatting.** Run `cargo fmt --all`, `git add` the files again and
  commit again.
- **The hook fails on clippy.** Fix what it names. "Function has too many lines" means the
  function is over 100 lines: move a part of it into a function of its own.
- **A push is refused with "fetch first".** Someone pushed first. Run
  `git pull --rebase origin main`, then push again.
- **GitHub says "This repository moved".** Point `origin` at the new address:
  `git remote set-url origin git@github.com:NoobyNull/Cut-Craft.git`.
- **A tag was pushed on the wrong commit.** Delete it everywhere, then tag again:
  `git tag -d v0.5.0`, `git push origin :refs/tags/v0.5.0`, `git push gitea :refs/tags/v0.5.0`.
