# Cut Craft: road to 1.0

Updated **2026-11-26**, at version **0.7.6**. The older feature-status roadmap (version 0.4.2)
is kept in `docs/feature-status-2026-11-01.md`.

## What 1.0 means

1.0 is "done fixing": the day all five goals below are met. New features start after 1.0;
until then, work that does not serve a goal waits.

1. **Main never breaks.**
2. **A beginner can go from blank sheet to saved G-code** with no known bugs and help on every step.
3. **The architecture work is finished**, ending with the app struct broken up.
4. **Every open issue and PR is decided**: fixed, merged, deferred or closed.
5. **Licence and credits are settled.**

## Timeline to 1 January 2027

Each open row carries its dates, and both boards hold the same items with Start and Target:
the [PowerWhittle timeline](https://github.com/orgs/Power-Whittle/projects/1) and
[CutCraft (PowerWhittle)](https://github.com/users/NoobyNull/projects/3).

| Week | Dates | Focus |
| --- | --- | --- |
| 1 | 25 Nov to 1 Dec | Land or close the open PRs; Matthew's decisions (rename by 1 Dec) |
| 2 | 2 to 8 Dec | cargo-deny, usvg, flaky test, keep-out zone, run-context, settings restore |
| 3 | 9 to 15 Dec | Walk the beginner path on Windows and Linux, help on every step, app struct and toolpath ids |
| 4 | 16 to 22 Dec | Fix the snags; main green from 16 Dec; feature freeze 18 Dec; release candidate 21 Dec |
| 5 | 23 to 31 Dec | Test the candidate, decide every issue and PR, release notes |
| | 1 Jan 2027 | 1.0 |

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
| 1.1 | Turn on "Require merge queue" in the main ruleset | Waiting on Matthew; target 27 Nov |
| 1.2 | Fix the flaky test: the Russian-options and languages tests race on the shared text catalogue | Not started; 2 to 4 Dec |
| 1.3 | Add cargo-deny (security advisories and licences) to CI; bump zip and rusqlite | Waiting on Matthew (linter survey); 2 to 5 Dec |
| 1.4 | Decide whether the Mac build must pass before merging | Waiting on Matthew; target 27 Nov |

### 2. Blank sheet to G-code, for a beginner

**Done when** a fresh install on Windows and Linux can set up a job, draw, make a profile and a
pocket, preview and save G-code with no known bugs, and every step's help page has steps and
pictures.

| # | Item | State |
| --- | --- | --- |
| 2.1 | Triage #668 (1/4" end mill) and #669 (material shows hollow) | Not started, with #718; 26 to 29 Nov |
| 2.2 | Updates reach users: #670 and #680 | Done (#670, #680) |
| 2.3 | Saving toolpaths: #672, #673, #674, #675 (in that order), including the machine-damage hard stop | Done (#672 to #675) |
| 2.3a | Switch to new layout: Ribbon by default and Classic listed last. #694 moves every layout onto one shared tool model; #697 adds the layout picker, the layout question in first-run setup and once after updating, a Did You Know tip and a help page per layout (both merged). A narrow Ribbon then goes on to a second row instead of folding groups (designed for 1920 wide, compact icons by default, Every Tool as a setting); each group name lists all its tools with pins. Lands before 2.4 so the walk-through and help pictures use the new screen A fourth layout, Canvas, gives the whole window to the drawing: a floating palette of tool groups, a floating form, chips for the job's lists and a Find a Tool box in the header; picture in picture 3D next. Canvas polish after trying it: the palette grows to fit (more columns, no scroll), groups open on hover (On Click is a setting), every tool in a list shows, and picture in picture shades the corner it will snap to | Done (#694, #697, #699, #704, #708) |
| 2.3b | Saving jobs with several bits: G-code file names that sort in cutting order and say the bit; per-machine Bit Changes (separate files or pauses, a bit change position, spin-up, opt-in Group by bit) and a check reminding to set Z zero with each bit | Done (#700, #701) |
| 2.3c | Tools and bits: a button is a tool, a cutter is a bit (Bit Library, the Bit section on every toolpath form, Bit Number, bit changes, Tool Changer (ATC)); Find Tool still finds the Bit Library by "tool database", "cutter" or "end mill". Lands before 2.4 so the walk-through and help pictures say bit | Done (#707) |
| 2.3d | Both views at once: a split with a divider to drag (past the edge to show one view) and ⇄ to swap places, plus picture in picture (the other view small in a corner, dragged to any corner, ⇄ to make it the big one) | Done (#705) |
| 2.3e | Did You Know tips get weights and come up unseen first (which were seen is kept on each computer only); a Canvas tip with Try Canvas comes up more often, not for those already in Canvas | Done (#711) |
| 2.3f | Toolbars of your own: right-click any tool or group ▸ Add to Toolbar (New Toolbar makes "My Tools"); each shows in every layout (a Ribbon and Canvas tab, a Rail button, a Classic section) or floats over the drawing where you leave it; drag or right-click to reorder, rename, delete. Favourites stay as they are | Done (#720) |
| 2.3g | Settings backup before the layout switch: Help ▸ Back Up Settings saves one `.ccbackup` file, and Cut Craft backs up by itself before updates, layout changes, first-time setup again and library rewrites, plus weekly (keeps 10). Restore with a preview (Merge or Replace per part) is stage 2 | Stage 1 done (#721); restore 28 Nov to 3 Dec |
| 2.3h | One button per shape: Draw Rectangle (Corner or Center) and Draw Circle (Center or Edge) each have one button, with **Draw from** at the top of the form picking the way and the button opening the way picked last. A setting gives each way its own button again | PR open |
| 2.4 | Walk the whole path by hand on both platforms and file every snag | Not started; 9 to 11 Dec, snags fixed by 18 Dec |
| 2.5 | Help on every step of the path, including the built-in actions with no help page | Not started; 9 to 15 Dec |
| 2.6 | Making a toolpath across a keep-out zone still refuses, against "tools never refuse" | Not started; 3 to 8 Dec |

### 3. Architecture finished

**Done when** the items below are merged or dropped, no Rust file is over 1,500 lines, and the
app struct is split into parts of about 40 fields or fewer.

| # | Item | State |
| --- | --- | --- |
| 3.1 | One toolpath output module for every way of saving | Done (#672 to #675) |
| 3.2 | Commands describe their own form; laser and rotary toolpaths only for machines that have them | #676, #679 done; run-context PR 1 to 5 Dec |
| 3.3 | One check before unsigned gadgets and add-ons run | Done (#678) |
| 3.4 | One list for built-in actions, one name everywhere | Done (#722, #723) |
| 3.5 | Break up the app struct, with the playback player folded in | Target 15 Dec. Playback player, previewed material, 3D view meshes, Help window, Options window, Snapping Options, Relief from Photo depth, Rail list, library files, save-check answers, Orientate 3D Model rings, 3D handle drag, Draw Text spelling, what the 3D view drew, Toolpath List picks, the 3D component settings popup and where the views were drawn merged; 3D view motion and detail PR open; more groups to follow |
| 3.6 | Stable toolpath ids, so cut order stops relying on a guess | Not started; 8 to 15 Dec |
| 3.7 | Model import session and one test driver | Deferred to after 1.0 |

### 4. Every issue and PR decided

**Done when** every open issue is closed or sits in the "After 1.0" milestone, and every open PR
is merged or closed.

| # | Item | State |
| --- | --- | --- |
| 4.1 | #636: close as a duplicate | Waiting on Matthew; target 28 Nov |
| 4.2 | #239 (tool-database links Worker): merge or close | Waiting on Matthew; target 28 Nov |
| 4.3 | #671 (toolpath list tree): merge now or hold until after 1.0 | Done (merged, #671) |
| 4.4 | Move #587, #595 and #597 to #603 to "After 1.0" | Not started; 26 Nov |
| 4.5 | Decide or shelve older open questions: depth warning on Pocket, shortcut keys in tooltips, DXF blocks, direction arrow in node editing | Waiting on Matthew; target 2 Dec |
| 4.6 | #702: tool names, tool groups, sub-tabs, page tabs and menu items move into Fluent so language packs translate them (a translator has Russian ready) | Done (PR #703) |
| 4.7 | Pig Latin test pack (Igpay Atinlay), made from English by the Language packs workflow, so text still in the code shows in plain English | Done (PR #709) |
| 4.8 | All on-screen text into Fluent (~3,000 strings: tool forms, app, other parts, help tours), with a CI check that warns on new hard-coded text | Building: tool forms and the warn-only check (PR #710); tooltips, status line and the job's right-click menus (PR #713); window hints and buttons (PR #715); machine, tool database and bug report windows (PR #732); tools' messages when they cannot run; help tours next; target 10 Dec |

### 5. Licence and credits settled

**Done when** LICENSE, About, README and the landing page all say the same thing, and the
credits list is generated and checked by CI.

| # | Item | State |
| --- | --- | --- |
| 5.1 | PolyForm Strict plus the any-use grant, generated credits folder, Help ▸ Credits window | Done (#684) |
| 5.2 | Move off the one MPL dependency (usvg 0.37 to 0.45); add missing licence fields on two rotary crates | Not started (with 1.3); 2 to 5 Dec |
| 5.3 | Update README, About text and landing page to the new terms | Not started; 9 to 12 Dec, after the rename decision |

## After 1.0

- **Rename** with a clean repo. Open question: whether the rename is the 1.0 launch itself.
  Settle it before 5.1, since the licence text names the product.
- **Paid subscription add-ons** with online activation and a grace period.
- **Online catalogue** (#597 to #603) and gadget network access (#587).
- **Auto Toolpath** (#595).
- **Rotary:** choose what rotary toolpaths cut (Material, Model, Vector, Level) and the true
  XYZA speed work.
- **Rotary:** a rotary toolpath that follows an open vector wrapped round the part (drive curve).
- **Cut list optimizer** stays a separate program.

Moving add-ons to the Power-Whittle organisation can happen any time, because it changes no
app code.
