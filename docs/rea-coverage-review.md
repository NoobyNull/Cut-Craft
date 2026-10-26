# Is the Aspire analysis good enough? (REA 6.8 review, 2026-10-25)

Static analysis only. Nothing here quotes Vectric material.

## Bottom line

The code-level analysis cannot get better with REA 6.8, because there is no readable Vectric
code to analyse. Behaviour has to keep coming from data sources (help, resources, settings,
sample files) plus black-box calibration under Wine. For the "Not built yet" list those
sources cover nearly everything. The real gaps are a few unmined data sources, not more
decompiling. **No large analysis run is worth starting.**

## What the new 6.8 tools showed

| Tool (new in 6.x) | Target | Finding | Evidence |
|---|---|---|---|
| `inspect_pe_load_config` | Aspire.exe | Two TLS callbacks; the first sits in `.reloc`, a section that should hold only relocations. That is the usual sign of a protector stub that decrypts `.text` before `main`. | `ev_ed21a40b…` |
| `inspect_byte_entropy` | Aspire.exe | `.text` is high-entropy (≥7.2) except one 64 KiB window at 19.46–19.53 MB, which holds the second TLS callback (the unpacker's own code). Confirms the earlier 7.99 bits/byte finding. | `ev_70438ffd…` |
| `inspect_pe_imports` | Aspire.exe | 43 DLLs. All Vectric logic is in the one exe: the rest are MFC, CRT, GDI+, OpenMP (`VCOMP140`), pstill, OpenThreads, BugSplat, and `n64dll.dll` (41 imports; the licensing library). | in session, 626 KB |
| same two | VCarve Pro 12.5 exe | Same protector: `.reloc` TLS callback, high-entropy `.text`. No unencrypted sibling build to fall back on. | `ev_e8335b21…`, `ev_00f72e7c…` |

The other x64 DLLs (OSG, OpenNURBS, SketchUp, pstill, at3dx) are third-party, so
`analyze_function`, call graphs, `trace_feature` and the reconstruction-ledger tools have
nothing to work on. Decrypting the exe stays declined.

## Gaps against "Not built yet", by source that can fill them

1. **`.vfont` format (blocks Text, Text on a Curve, Auto Layout Text).** SPEC open question 1
   says there are no samples. Wrong: `Fonts/` has 23 `.vfont` files, and they are OLE
   compound documents (`D0 CF 11 E0` header, entropy 2.8, so not encrypted). The layout can
   be worked out from the samples with a stdlib CFB reader. Highest-value gap.
2. **Job / clipart formats (`.crv3d`, `.3dclip`, `.v3m`, Import Component, Export as 3D
   Clipart).** Probably the same OLE container. No samples on disk; saving a few small files
   from VCarve Pro under Wine (already set up in `~/.wine-vcarve`) would give them.
3. **Dialog and XAML forms (273 dialogs, 107 `RT_XAML`).** `peres.py` decodes them, but the
   spec only uses them for Quick Engrave. Each unbuilt tool's dialog gives its exact fields,
   labels, ranges and default radio choices. Mine one per tool as it comes up (step 1 of
   `docs/PLAN.md`).
4. **`Default.prf` field meanings (Options, Snap Options, display settings).** Change one
   setting in VCarve Pro's Options under Wine, diff its saved profile against the default.
5. **`.mo` catalogs.** Validation and error messages per subsystem (`Nest`, `Sd`, `Vd`, …)
   give edge cases the help omits: limits, refusals, warnings. Worth a pass per tool.
6. **Help.** Every unbuilt item has a help topic (Nest Parts, Two Rail Sweep, Trim Objects,
   Dimensions, KeepOutZonesForm, Job Setup Double, Sheet Management, …). Covered.
7. **Algorithms (fillets, trim, nesting, sweep, emboss, slice).** No static source exists.
   This is the calibration rig in `docs/PLAN.md` §2: same input through VCarve Pro under Wine
   and through Cut Craft. Aspire-only tools (Two Rail Sweep, Emboss, Sculpt, …) can't be
   calibrated this way, because VCarve Pro lacks them; help is all there is for those.

## Recommended next analysis, in order (all small)

1. Decode the `.vfont` container from the 23 samples and write the layout into `re/SPEC.md`
   (closes open question 1 for fonts, unblocks M5 Text).
2. Dump the dialog/XAML controls for each M1–M4 tool into `re/extracted/` and add the
   fields to each tool's SPEC section as it is built.
3. Make a handful of sample `.crv3d` / clipart files and `.prf` diffs in VCarve Pro under
   Wine. That is runtime use of the app, not static analysis, so it needs your OK.
4. Correct the SPEC: open question 1 (fonts have samples), and record the `.reloc` TLS
   callback as the static evidence for encryption.
