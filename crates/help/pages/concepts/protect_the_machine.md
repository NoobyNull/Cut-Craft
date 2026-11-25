---
reviewed: 2026-11-25
see-also: checks_before_cutting, check_keep_out, check_bed, check_z_travel, check_spoilboard
---

# Protect the Machine

![A job bigger than the bed: refused, with no tick](pictures/checks/machine-damage.png)

Some things found before cutting could damage the machine itself, not just a bit or the
work:

- a cut into a **keep-out zone**, the job's or the machine's (a clamp fixed to the bed);
- toolpaths **bigger than the bed**, which run the machine into its end stops;
- toolpaths going further up and down than the **Z axis travels**;
- a cut **deeper into the spoilboard** than the machine allows, which can cut into the bed.

These show as **⛔ Could damage the machine** in [Checks before cutting](help:checks_before_cutting).
Unlike a ✖ Must fix, they have **no tick box** and there is no Save Anyway:

- **Save Toolpaths** and **Send to Machine** stay greyed;
- **View G-code** shows the code so you can see the problem, but **Copy All** is greyed;
- the [Web API](help:web_api) refuses to make the G-code, whatever the script sends.

Put it right (move the part or the zone, make the job smaller, tile it, cut less deep) and
the checks pass.

## Switching a check off

Each machine has its own **Protect the Machine** section in **Machine Configuration**, all on
when a machine is added:

1. Open **Machine Configuration** and pick the machine.
2. Open **Protect the Machine** (it says *every check on* while they all are).
3. Untick the check to switch off: **Keep-out zones**, **Bed size**, **Z travel** or
   **Spoilboard depth**.
4. Press **Apply**.

A check switched off is shown in the list as *not checked: Switched off for* (the machine)
*in Machine Configuration*, and every file made while it is off says so at the top, so
whoever runs it later knows:

```
(PROTECTION OFF: the checks for bed size were switched off for Shop Router in Machine Configuration as this was made)
```

Only do this when you know the check is wrong for your machine, such as a bed bigger
than its settings say. Tick it again afterwards.

## How deep into the spoilboard

**Allowed into spoilboard** is how far below the material a cut may go, 1 mm unless you
set it. Cutting a part out cleanly goes a little into the spoilboard on purpose; a thick
sacrificial spoilboard can allow more. A cut deeper than this is refused.

**Warn about cuts more than 1 mm into the spoilboard** (ticked unless you untick it) gives
a ⚠ to read and tick for a cut past 1 mm but still within the depth allowed. A wrong
material thickness usually shows first this way.

Flattening the spoilboard itself (Surfacing, Spoilboard) is never refused.
