# Working on Cut Craft

Read [CONTRIBUTING.md](CONTRIBUTING.md) and [BUILDING.md](BUILDING.md) first. In short: tests
first (`cargo nextest`), `cargo fmt` and `cargo clippy -D warnings` clean, a branch named for
the area, and a pull request title that starts with its kind (`feat:`, `fix:`, ...).

**Help rule** (checked on every pull request): before the pull request, update the help page of
every tool or setting you added or changed, or set its `reviewed:` date. The steps are in
[CONTRIBUTING.md, Help for every tool](CONTRIBUTING.md#help-for-every-tool). Then run
`scripts/help-touched origin/main`: it names every page the change still owes.

**Power-Whittle rule**: every pull request keeps the PowerWhittle timeline and the Power-Whittle
repositories up to date ([CONTRIBUTING.md, Keep Power-Whittle up to date](CONTRIBUTING.md#keep-power-whittle-up-to-date)).

Never name other CAD/CAM programs in anything a user sees (tests check this).
