# Cut Craft: detailed plan for the remaining work

The step-by-step plan behind `docs/ROADMAP.md`. The roadmap says what is done and in what
order the rest comes; this file says how each remaining item is built, what it changes,
how it is tested and when it counts as done. Written 2026-10-25, after Phase A and the
tracing core (`crates/core/src/trace.rs`).

## 1. How every item is built

The same cycle for each item, one at a time, nothing pushed until asked.

1. **Find the reference program's behaviour.** Only from its maker's public documentation (the help, extracted
   with `re/help2txt.py`; guides; tutorial videos) and hands-on use of its trial. Write
   what is found into `re/SPEC.md`, tagged [obs] (seen in the trial), [inf] (from the docs),
   [check] or [unk].
   - Never decompile, disassemble or otherwise analyse the reference program's files or resource
     DLLs, and never extract artwork or strings from them.
2. **Calibrate where exactness matters** (tracing, curve fitting, offsets, nesting, fonts):
   run the same input through the reference program under Wine and through Cut Craft, compare the
   results (node counts, distances between outlines, areas), and tune until they agree.
   The test inputs and its results stay outside git (`re/extracted/`); our tests
   assert the agreed numbers. See section 4.
3. **Write tests first**, then the code: a command crate per command (`scripts/new-command`),
   pure functions in `cut-craft-core` where several commands share them.
4. **Gate:** `.githooks/pre-commit` (fmt, clippy pedantic with `-D warnings`, every test);
   no real dialogs or programs from tests (`on_the_desktop` fails them).
5. **Wire it up:** menu item, Drawing / Modeling / Toolpaths tab button, right-click menu,
   shortcut; the unwired-menu check (section 5) shrinks.
6. **Commit**, update `docs/tool-inventory.md`, and this file's status column.
7. **After each milestone:** rebuild the launcher, try the milestone in the running app
   (screenshot rules in the project memory), update `docs/ROADMAP.md`, and report.

## 2. Status at a glance

Updated 2026-11-01 (version 0.4.2). Every milestone below is built; what is left inside
them, and what is not built, is in `docs/ROADMAP.md` (sections 3 and 4).

| Milestone | Contents | Status |
|---|---|---|
| A | Selection order, component and bitmap selection, groups | done (nested groups left) |
| T | Tracing core: outlines, simplify, fit lines / arcs / Béziers | done (`cc50ed1`) |
| M1 | Trace Bitmap, Fit Curves, Vector Boundary, calibration rig | done (`crates/calib`, blind-test harness) |
| M2 | Component tools (rest of Phase B) | done |
| M3 | STL import; small file and job items (Phase C) | done (Import Toolpaths left: format unknown) |
| M4 | Node editing and the vector tools on it (Phase D) | done |
| M5 | Text | done (TrueType kerning left) |
| M6 | Large modeling tools (Phase E) | done |
| M7 | Settings, snapping, guides, materials | done |
| M8 | Sheets, double-sided jobs, nesting | done |
| M9 | Machine configuration, keep-out zones | done |
| M10 | 3D view on the GPU; gaps inside strategies (Phase G) | done (lasers skipped by decision) |

## 3. Milestones

Sizes: **S** a day or less, **M** a few days, **L** a week or more.