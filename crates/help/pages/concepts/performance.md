---
reviewed: 2026-11-24
see-also: settings, progress, preview_detail, view_3d
---

# Keeping the computer responsive

Calculating a toolpath, a preview or a big import can keep every core of your computer
busy. Cut Craft keeps some of the computer for everything else, so a video, the browser
and Cut Craft's own window stay smooth while it works.

## What Cut Craft does while it calculates

- **It uses a share of the processor's cores**, 80% to start with: on a computer with 32
  cores it uses 25, on one with 4 it uses 3. It always keeps at least one core working.
- **It gives way to your other programs.** Its calculations run at a lower priority, so
  when a video or the browser wants the processor they get it first. When nothing else
  wants it, the calculation still runs at full speed, so on a quiet computer this costs
  nothing.
- **It shares the graphics card too.** Some previews and toolpaths are worked out on the
  graphics card, which also draws your desktop and your videos. Cut Craft gives the card
  its work in short pieces and lets it rest after each one, so at 80% the card spends about
  a fifth of the time on everything else. The status bar's word is about the processor
  only.

## Changing it: Edit ▸ Options ▸ Performance

1. Open **Edit ▸ Options** and pick **Performance** on the left.
2. **Limit calculations so other programs stay smooth** is ticked to start with. Untick it
   to let calculations use every core at full priority: the fastest, but other programs may
   stutter meanwhile.
3. **Share of the computer to use** sets how much of the processor calculations may use.
   Less keeps other programs smoother; more finishes sooner. It sets the graphics card's
   share too, never less than a tenth.
4. Press **Apply**. The change applies from the next calculation; one already running
   keeps going as it started.

The same page holds the other speed-or-detail choices: the default
[Preview detail](help:preview_detail), how finely the 3D view draws the model when still
and while moving, and **Adjust detail to keep the view smooth** with the smoothness it aims
for. They used to be on the 3D View page.

## How briskly it is going: the status bar

While a calculation runs, the status bar at the foot of the window shows how briskly it is
going, with a picture and a word:

![Crawl, Walk, Run and Sprint as the status bar shows them](pictures/concepts/performance-paces.png)

- **Run** (green): the calculation is getting what it asked for, within the limit set.
- **Walk** (amber): it is sharing the processor with your other programs.
- **Crawl** (orange): other programs are using most of the processor, so it is going
  slowly. Close what you don't need, or raise the share, to speed it up.
- **Sprint** (blue): no limit is set, so it uses every core at full speed.

Point at it to read what it is doing; click it to open **Options ▸ Performance**. Once
nothing is calculating it goes away.

## On each kind of computer

- **Windows**: calculations run at "below normal" priority.
- **Linux**: calculations run with a nice value of 10, below your desktop's programs.
- **macOS**: calculations run at the "utility" level. On Apple silicon the Mac may run them
  on its slower efficiency cores, so a calculation can take a little longer there.
- **A computer with 2 cores** calculates on 1 of them with the limit on, about half as fast.
  Untick the limit for speed when you are not using the computer for anything else.
