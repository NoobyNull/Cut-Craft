# Working on Cut Craft

Read [CONTRIBUTING.md](CONTRIBUTING.md) and [BUILDING.md](BUILDING.md) first. In short: tests
first (`cargo nextest`), `cargo fmt` and `cargo clippy -D warnings` clean, a branch named for
the area, and a pull request title that starts with its kind (`feat:`, `fix:`, ...).

**Help rule** (checked on every pull request): before the pull request, update the help page of
every tool or setting you added or changed, or set its `reviewed:` date. The steps are in
[CONTRIBUTING.md, Help for every tool](CONTRIBUTING.md#help-for-every-tool). Then run
`scripts/help-touched origin/main`: it names every page the change still owes.

Never name other CAD/CAM programs in anything a user sees (tests check this).

## Agent skills

### Issue tracker

Issues are tracked in GitHub Issues on `NoobyNull/Cut-Craft`. See `docs/agents/issue-tracker.md`.

### Triage labels

The five default triage labels (`needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human`, `wontfix`). See `docs/agents/triage-labels.md`.

### Domain docs

Single-context: one `CONTEXT.md` and `docs/adr/` at the repo root. See `docs/agents/domain.md`.
