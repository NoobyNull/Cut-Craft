# Contributing

Work on a branch named for the area, keep `cargo fmt`, `cargo clippy` and `cargo nextest`
clean (`.githooks/pre-commit` runs all three), and open a pull request into `main`.
[BUILDING.md](BUILDING.md) has the setup.

## Pull request titles set the version

Cut Craft's version is **0.FEATURE.FIX**. Each night, if anything worth shipping merged that
day, `main` gets a new version and a signed release that installed copies offer as an update.
The pull request titles decide which number moves, so every title starts with a kind
(the **PR title** check fails until it does):

| Title starts with | Use it for | Version |
|---|---|---|
| `feat:` | a new feature, tool or option | 0.5.1 → 0.6.0 |
| `fix:` | a bug fix, big or small | 0.5.1 → 0.5.2 |
| `perf:` `refactor:` `revert:` | faster, reorganised or undone code users still run | 0.5.1 → 0.5.2 |
| any kind with `!`, e.g. `refactor!:` `feat!:` | a rewrite, or a change that breaks old project files or habits | 0.5.1 → 0.6.0 |
| `docs:` `chore:` `ci:` `test:` `style:` `build:` | nothing users run changes | none |

A scope is fine: `feat(toolpaths): Drill bank`. A night takes the biggest kind among its
merges, so a night with one `feat:` and three `fix:` is one feature version (0.5.1 → 0.6.0),
and a night of only `docs:` and `chore:` releases nothing. A title merged without a kind
counts as a fix. The rule lives in `scripts/next-version` (tests: `scripts/test-next-version`);
the nightly job is `.github/workflows/version-bump.yml`.
