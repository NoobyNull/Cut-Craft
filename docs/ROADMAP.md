# Cut Craft: road to 1.0

Updated **2026-11-23**, at version **0.7.5**. The older feature-status roadmap (version 0.4.2)
is kept in `docs/feature-status-2026-11-01.md`.

## What 1.0 means

1.0 is "done fixing": the day all five goals below are met. New features start after 1.0;
until then, work that does not serve a goal waits.

1. **Main never breaks.**
2. **A beginner can go from blank sheet to saved G-code** with no known bugs and help on every step.
3. **The architecture work is finished**, ending with the app struct broken up.
4. **Every open issue and PR is decided**: fixed, merged, deferred or closed.
5. **Licence and credits are settled.**

## Rules until 1.0

- **Serve a goal or wait.** Each new request is tagged with the goal it serves. If none fits,
  it goes on the after-1.0 list.
- **At most 3 designs waiting on Matthew.** A fourth design isn't started until one gets a
  build go or is shelved.
- **At most 3 open PRs** (a stacked chain counts as one). Finishing beats starting.
- **One architecture item a week**, so structural work never stalls behind bug fixes.
- **Bug reports from real users jump the queue** if they hit the beginner path (goal 2).

## The five goals

Items run in the order listed. The state column is as of the date above.

### 1. Main never breaks

**Done when** main stays green for two weeks with every PR going through the merge queue.

| # | Item | State |
| --- | --- | --- |
| 1.1 | Turn on "Require merge queue" in the main ruleset | Waiting on Matthew |
| 1.2 | Fix the flaky test: the Russian-options and languages tests race on the shared text catalogue | Not started |
| 1.3 | Add cargo-deny (security advisories and licences) to CI; bump zip and rusqlite | Waiting on Matthew (linter survey) |
| 1.4 | Decide whether the Mac build must pass before merging | Waiting on Matthew |

### 2. Blank sheet to G-code, for a beginner

**Done when** a fresh install on Windows and Linux can set up a job, draw, make a profile and a
pocket, preview and save G-code with no known bugs, and every step's help page has steps and
pictures.

| # | Item | State |
| --- | --- | --- |
| 2.1 | Triage #668 (1/4" end mill) and #669 (material shows hollow) | Not started |
| 2.2 | Updates reach users: #670 and #680 | PRs open |
| 2.3 | Saving toolpaths: #672, #673, #674, #675 (in that order), including the machine-damage hard stop | PRs open |
| 2.4 | Walk the whole path by hand on both platforms and file every snag | Not started |
| 2.5 | Help on every step of the path, including the built-in actions with no help page | Not started (with 3.4) |
| 2.6 | Making a toolpath across a keep-out zone still refuses, against "tools never refuse" | Not started |

### 3. Architecture finished

**Done when** the items below are merged or dropped, no Rust file is over 1,500 lines, and the
app struct is split into parts of about 40 fields or fewer.

| # | Item | State |
| --- | --- | --- |
| 3.1 | One toolpath output module for every way of saving | PRs #672 to #675 open (same as 2.3) |
| 3.2 | Commands describe their own form; laser and rotary toolpaths only for machines that have them | PRs #676, #679 open; run-context PR after #675 |
| 3.3 | One check before unsigned gadgets and add-ons run | PR #678 open |
| 3.4 | One list for built-in actions, one name everywhere | Waiting on Matthew (build go) |
| 3.5 | Break up the app struct, with the playback player folded in | Not started |
| 3.6 | Stable toolpath ids, so cut order stops relying on a guess | Not started |
| 3.7 | Model import session and one test driver | Deferred to after 1.0 |

### 4. Every issue and PR decided

**Done when** every open issue sits in either a "1.0" or an "After 1.0" milestone and no PR is
more than a week old.

| # | Item | State |
| --- | --- | --- |
| 4.1 | #636: close as a duplicate | Waiting on Matthew |
| 4.2 | #239 (tool-database links Worker): merge or close | Waiting on Matthew |
| 4.3 | #671 (toolpath list tree): merge now or hold until after 1.0 | Waiting on Matthew |
| 4.4 | Move #587, #595 and #597 to #603 to "After 1.0" | Not started |
| 4.5 | Decide or shelve older open questions: depth warning on Pocket, shortcut keys in tooltips, DXF blocks, direction arrow in node editing | Waiting on Matthew |

### 5. Licence and credits settled

**Done when** LICENSE, About, README and the landing page all say the same thing, and the
credits list is generated and checked by CI.

| # | Item | State |
| --- | --- | --- |
| 5.1 | PolyForm Strict plus the any-use grant, generated credits folder, Help ▸ Credits window | Waiting on Matthew (build go) |
| 5.2 | Move off the one MPL dependency (usvg 0.37 to 0.45); add missing licence fields on two rotary crates | Not started (with 1.3) |
| 5.3 | Update README, About text and landing page to the new terms | Not started |

## After 1.0

- **Rename** with a clean repo. Open question: whether the rename is the 1.0 launch itself.
  Settle it before 5.1, since the licence text names the product.
- **Paid subscription add-ons** with online activation and a grace period.
- **Online catalogue** (#597 to #603) and gadget network access (#587).
- **Auto Toolpath** (#595).
- **Rotary:** choose what rotary toolpaths cut (Material, Model, Vector, Level) and the true
  XYZA speed work.
- **Cut list optimizer** stays a separate program.

Moving add-ons to the Power-Whittle organisation can happen any time, because it changes no
app code.
