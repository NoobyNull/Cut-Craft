---
reviewed: 2026-11-28
see-also: checks_before_cutting, check_plunges
---

# Drills and holes

![A drill used for a profile, and holes the wrong size](pictures/checks/drilling.svg)

## What it looks for

Two things. A bit marked **Drill only** in the Bit Library used by any toolpath but
Drilling. And for a Drilling toolpath, holes drawn bigger or smaller than the drill: a
drill only goes straight down, so every hole comes out the drill's size.

## Why it matters

A drill moved sideways snaps or tears the work. A hole drawn the wrong size comes out the
drill's size anyway, not what you drew.

## How to fix it

Pick an end mill for any toolpath that moves sideways. For a hole wider than the bit, use
a **Pocket** or **Profile (Inside)** toolpath. For holes smaller than the drill, pick a
smaller drill.

See also [Checks before cutting](help:checks_before_cutting).
