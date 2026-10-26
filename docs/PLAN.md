# Cut Craft: detailed plan for the remaining work

The step-by-step plan behind `docs/ROADMAP.md`. The roadmap says what is done and in what
order the rest comes; this file says how each remaining item is built, what it changes,
how it is tested and when it counts as done. Written 2026-10-25, after Phase A and the
tracing core (`crates/core/src/trace.rs`).

## 1. How every item is built

The same cycle for each item, one at a time, nothing pushed until asked.

1. **Find Aspire's behaviour.** In order: Aspire's help (extracted with `re/help2txt.py`),
   its resource DLL (dialog controls, labels, menu entries), data in `Aspire.exe` (strings,
   settings keys, form names from MFC runtime-class records, tables and constants), and the
   VCarve Pro 12.5 gadget Lua scripts. Write what is found into `re/SPEC.md`, tagged [obs],
   [dec], [inf] or [unk].
   - `Aspire.exe`'s **code** is encrypted on disk (see `re/SPEC.md`, Sources), so its
     functions cannot be read. Decrypting them (or using a memory dump of the running
     program) is not done: it gets around Vectric's copy protection.
2. **Calibrate where exactness matters** (tracing, curve fitting, offsets, nesting, fonts):
   run the same input through VCarve Pro 12.5 under Wine and through Cut Craft, compare the
   results (node counts, distances between outlines, areas), and tune until they agree.
   The test inputs and Vectric's results stay outside git (`re/extracted/`); our tests
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

| Milestone | Contents | Status |
|---|---|---|
| A | Selection order, component and bitmap selection, groups | done |
| T | Tracing core: outlines, simplify, fit lines / arcs / Béziers | done (`cc50ed1`) |
| M1 | Trace Bitmap, Fit Curves, Vector Boundary, calibration rig | next |
| M2 | Component tools (rest of Phase B) | |
| M3 | STL import; small file and job items (Phase C) | |
| M4 | Node editing and the vector tools on it (Phase D) | |
| M5 | Text | |
| M6 | Large modeling tools (Phase E) | |
| M7 | Settings, snapping, guides, materials | |
| M8 | Sheets, double-sided jobs, nesting | |
| M9 | Machine configuration, keep-out zones | |
| M10 | 3D view on the GPU; gaps inside strategies (Phase G) | |

## 3. Milestones

Sizes: **S** a day or less, **M** a few days, **L** a week or more.