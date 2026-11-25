# Contributing

Work on a branch named for the area, keep `cargo fmt`, `cargo clippy` and `cargo nextest`
clean (`.githooks/pre-commit` runs all three), and open a pull request into `main`.
[BUILDING.md](BUILDING.md) has the setup.

## Help for every tool

Help lives in its own crate, `crates/help`: one Markdown page per tool at
`crates/help/pages/tools/<tool id>.md`, plus Contents, Design and Toolpaths. A page starts with
a header, then says in steps, with pictures where a beginner needs them, how to use the tool:

    ---
    reviewed: 2026-11-14
    see-also: profile_toolpath, vcarve_engrave_toolpath
    ---

`see-also` and every `help:` link in the words make the help's link graph: a tool's page lists
the pages it names **and** the pages that name it, so links work both ways from one list.
Every page names at least one related page (tests fail otherwise); they show in its side bar
under Related, with its tool group under Other tools like this. When people search with a word
the help doesn't use, add it to `crates/help/synonyms.txt`.

**The help rule.** At the end of every piece of work, go back and update the help for every
tool or setting you added or changed: its steps, pictures and settings, and links to the pages
around it. If the page truly needs no change, set its `reviewed:` line to today's date to
declare it checked. A pull request does not pass without this:

- `scripts/help-touched` (run by the **check** job on every pull request) fails when a command
  crate under `crates/commands/` changed and the page of each tool it makes did not, or when
  other program code changed and no help page did. Tests, `build.rs` and Cargo files are left
  out. Run it yourself before pushing: `scripts/help-touched origin/main`.
- The help crate's tests fail on a page whose header will not read, and the app's on a link to
  a page that does not exist or a command with no page.

Each tool also needs short help in the code: a tooltip (the crate's first `//!` sentence) that
says what it does, a help page intro (the `///` doc comment on its `Params`), and a `///`
doc comment on every setting and every option of a choice. The help coverage suite checks
this apart from the other tests:

    cargo nextest run -p cut-craft --profile help

It fails only on gaps not listed in `crates/app/help/coverage-baseline.txt`. When you fix a
gap, delete its line there. The Help coverage workflow keeps one GitHub issue (label
`help-coverage`) listing every gap still open.

## Rotary tests

Rotary work (the turning axis, the rotary toolpaths, the posts that write A, and the
`cut-craft-rotary` crate that knows where the material and the model are in space) has its own
suite. Run it after any change that touches rotary:

    cargo nextest run --workspace --profile rotary

## Words the app shows

Every word the app shows lives in a Fluent file under `crates/i18n/locales/en/`, one file per
area (`menus.ftl`, `forms.ftl`, `start.ftl`, ...), under a key. The code asks for the key,
never the words, so a language pack can swap them; English is built in and fills in any key a
pack has no words for yet.

    commit-calculate = Calculate
        .tooltip = Work out the toolpath with these settings
    start-whats-new = What's new in v{ $version }

    ui.button(tr!("commit-calculate")).on_hover_hint(tr!("commit-calculate.tooltip"));
    ui.strong(tr!("start-whats-new", version = env!("CARGO_PKG_VERSION")));

- Keys are lower-case words joined by hyphens, starting with their area (`menu-`, `form-`).
  A button's tooltip is an attribute of the same message (`.tooltip`), so they move together.
- A length or other measured number goes in already written in the job's units, by the code
  that writes the number box beside it. A count that picks a plural goes in as a number:
  `{ $count -> [one] { $count } toolpath *[other] { $count } toolpaths }`.
- Leave room: labels sit in fixed areas and never reflow, and other languages run up to 40%
  longer than English. `cut_craft_i18n::pseudo()` is a test language that stretches every word
  by that much.

Languages have their own test suite, run by the **Languages** workflow:

    cargo nextest run -p cut-craft -p cut-craft-i18n --profile languages

The **hard-coded text** check in it (also in the ordinary run) warns of words handed
straight to a button, label, tooltip, menu, window or the status line in `crates/app/src`: a
warning on the pull request, never a fail. It does fail on a key with no English words. Text
already in the code before the move is listed in `crates/app/i18n/hard-coded-baseline.txt`;
when you move one into Fluent, delete its line (a stale line is warned of too, so the list
only shrinks).

A form's words (each setting's name, tooltip and choices, from the command's doc comments)
are copied into `crates/i18n/locales/en/settings.ftl`, keyed by their English words. When you
change a form, write that file again: `cargo test -p cut-craft --bin cut-craft
write_settings_words -- --ignored` (a test fails until you do).

## Test suites by area

Every test belongs to exactly one suite, by area of the app: Core geometry, Toolpath
generation, 3D modelling, Drawing and editing, Display and preview, Files, Help, How-to tours,
App, and Whole-app workflows. `.config/test-suites.toml` says which packages and app modules
each owns, and which other suites a change to it can break ("affects"). On a pull request the
**check** jobs run only the suites the changed files reach, plus the suites those affect;
Format and Clippy always cover the whole workspace. Main, the nightly run and a run by hand run
every suite, and check that no test is left out of one. See what a change needs, or run it:

    scripts/pick-suites origin/main          # which suites, why, and the command
    scripts/pick-suites origin/main --run    # run them
    scripts/pick-suites --suite how-to --run # one suite
    scripts/pick-suites --list               # what each suite owns

A file the suites file does not know runs every suite, to be safe. When you add a crate or an
app module that belongs to an area, add it to that suite (a new `cmd-*` tool lands in Drawing
and editing unless a suite before it names it). A new area gets its own suite there, with the
packages and modules it owns moved into it and the suites it affects listed; run
`scripts/test-pick-suites` and `scripts/pick-suites --check` after.

The **How-to tours** suite plays every tool's tour (`crates/app/src/help_tours.rs`) in the app
headless, step by step, and fails when a step can no longer find the button, field or tab it
rings, so the help pictures never point at something that moved.

## Keep Power-Whittle up to date

Cut Craft is moving to the [Power-Whittle](https://github.com/Power-Whittle) organisation. Until
it has moved, every pull request keeps that side current too (the pull request template has the
boxes to tick):

- **The timeline.** Put the pull request, or the issue it fixes, on the
  [PowerWhittle timeline](https://github.com/orgs/Power-Whittle/projects/1) with a start date
  and a target date, and keep its status right. When it merges, mark it done. The board needs
  write access; without it, ask a maintainer in the pull request to add or update it for you.
- **The release mirror.** Power-Whittle/Cut-Craft is a read-only copy for releases. Once the
  release workflow's mirror step lands (it pushes each release's tag and `main` there and adds
  the same files), never commit to the mirror's `main` by hand: the next release could no longer
  push `main` there. Change its description, rulesets or settings when a
  change here makes them wrong (for example a new release step or a renamed branch).
- **The add-ons and the org.** When a change touches where add-ons, updates or links point
  (the updater's homes, add-on repositories, links shown in the app), check the matching
  Power-Whittle repository and the organisation settings still agree.

## Pull request titles and versions

Cut Craft's version is **0.MINOR.PATCH**, and it only changes when a release is cut, one step at
a time: usually the last number (0.6.0 → 0.6.1 → 0.6.2), and the middle one (0.6.2 → 0.7.0)
for a milestone. Merging a pull request never changes it. See "Cut a versioned release" in
[BUILDING.md](BUILDING.md).

Every pull request title still starts with a kind (the **PR title** check fails until it does),
because the release notes are sorted by it:

| Title starts with | Use it for | Release notes |
|---|---|---|
| `feat:` | a new feature, tool or option | New |
| `fix:` | a bug fix, big or small | Fixed |
| `perf:` `refactor:` `revert:` | faster, reorganised or undone code users still run | Fixed |
| any kind with `!`, e.g. `refactor!:` `feat!:` | a rewrite, or a change that breaks old project files or habits | New |
| `docs:` `chore:` `ci:` `test:` `style:` `build:` | nothing users run changes | left out |

A scope is fine: `feat(toolpaths): Drill bank`. The rule lives in `scripts/next-version`
(tests: `scripts/test-next-version`); `scripts/release-version` uses it to draft the notes.
