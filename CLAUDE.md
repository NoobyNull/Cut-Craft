# Working on Cut Craft

Read [CONTRIBUTING.md](CONTRIBUTING.md) and [BUILDING.md](BUILDING.md) first. In short: tests
first (`cargo nextest`), `cargo fmt` and `cargo clippy -D warnings` clean, a branch named for
the area, and a pull request title that starts with its kind (`feat:`, `fix:`, ...).

## The help rule (required, checked on every pull request)

At the end of every piece of work, before the pull request, go back and update the help for
every tool or setting you added or changed. Help is its own crate: `crates/help/pages/`, one
page per tool at `crates/help/pages/tools/<tool id>.md`.

1. Write what changed into the page: the steps a beginner follows, pictures (or an animation or
   a step-by-step sequence) where words alone leave them guessing, and the settings.
2. Link it to related pages: `see-also:` in its header, and `[words](help:<id>)` in the text.
3. If the page truly needs no change, set its `reviewed:` line to today's date (YYYY-MM-DD).
   That declares it checked, and counts as updating it.
4. Run `scripts/help-touched origin/main`. It names every page the change still owes. CI runs
   the same check and the pull request does not pass until it does.

Also keep the short help in the code: the crate's first `//!` sentence (tooltip), the `///`
docs on `Params`, every setting and every choice (`cargo nextest run -p cut-craft --profile
help`).

Never name other CAD/CAM programs in anything a user sees (tests check this).
