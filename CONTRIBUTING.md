# Contributing

Work on a branch named for the area, keep `cargo fmt`, `cargo clippy` and `cargo nextest`
clean (`.githooks/pre-commit` runs all three), and open a pull request into `main`.
[BUILDING.md](BUILDING.md) has the setup.

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
