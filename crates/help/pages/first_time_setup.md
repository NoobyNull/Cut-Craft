---
reviewed: 2026-11-25
see-also: getting_started, interface_layout, settings, setup_material, bug_reports, post_processor_management
icon: file/setup_material.png
---

# First-Time Setup

The first time PowerWhittle starts, it asks five quick questions so your first job matches
your machine. **Help ▸ Run First-Time Setup Again** asks them again: for a new machine,
or if you set it up wrong the first time.

![First-Time Setup: your machine, millimetres or inches, your usual material, and crash reports](pictures/basics/first-time-setup.png)

## The five questions

1. **Which machine do you have?** Pick its **Maker**, then its **Model**. Not in the list?
   Pick **Mine is not listed** and type its **Working area width (X)** and **length
   (Y)**, and pick its **Controller**: the board or program that runs the machine. Not
   sure? Most hobby routers run grbl.
2. **Millimetres or inches?** Picked to match your machine. Choose whichever your rulers
   and bits use.
3. **What size material do you usually cut?** Its **Width (X)**, **Length (Y)** and
   **Thickness**. New jobs start this size; each job can still be changed with
   [Setup Material](help:setup_material).
4. **If PowerWhittle crashes, may it send a crash report to help fix it?** **Yes**, **Ask me
   each time** or **No**. A crash report says what PowerWhittle was doing when it closed;
   your user name, folders and file names are left out of it. See
   [Reporting a bug](help:bug_reports).
5. **How should the window be laid out?** Click **Ribbon**, **Rail** or **Classic**. Each
   shows a picture of it. Ribbon is picked to start with and suits most people. Change it
   any time with [Interface Layout](help:interface_layout).

Press **Start using PowerWhittle**. It adds the machine and makes it the one you use, sets
the job every new file starts with, and puts the starter bits in your units at the top of
the Bit Library, and arranges the window as picked. The status bar says what was set
up.

**Skip for now** starts with PowerWhittle's usual 600 × 400 × 20 mm job and no machine, and
keeps the layout already in use (Ribbon on a new install).

## Asking again

**Help ▸ Run First-Time Setup Again** opens the same questions, filled in with your
answers from before, so change only what is wrong:

- **A new machine**: pick it, and it is added and made the one you use. Your other
  machines are kept.
- **A machine you set up as not listed** (**My Machine**) is corrected, not added again:
  it takes the working area, units and controller you answer now. Post-processors you
  added to it yourself in Machine Configuration are kept.
- **The material and units** become the job every new file starts with (the same as
  [Job Defaults](help:settings) in Edit ▸ Options).

Run again, setup changes My Machine and the layout, so PowerWhittle
[backs up your settings](help:backups) first.

Every answer can also be changed on its own later: the machine in **Machine ▸ Machine
Configuration**, new jobs in **Edit ▸ Options ▸ Job Defaults**, crash reports in **Edit ▸
Options ▸ Bug Reports**.
