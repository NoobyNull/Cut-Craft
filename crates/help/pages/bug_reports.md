---
reviewed: 2026-11-26
see-also: settings, updates, contents, language
---

# Reporting a bug

Something wrong? **Right-click** the part of Cut Craft where it happened and choose
**Report a Bug Here…**. A form opens for you to say what went wrong. Cut Craft fills in the
rest for you: which part of the window you clicked, the version, and what it was doing.

Changed your mind? Click anywhere outside the menu, or press **Esc**, and it closes. Grabbing
a shape to move it closes it too.

If Cut Craft closes because of an error, it keeps a crash log. The next time it starts it
sends that log, or offers it to you in the same form, as you chose in
[Edit ▸ Options ▸ Bug Reports](help:settings).

![The Report a Bug form, with the details of this computer ticked and shown underneath](pictures/bug-report-form.png)

## Step by step

1. **Right-click** where the problem is and choose **Report a Bug Here…**.
2. Give the report a short **title**, and say what you did and what went wrong.
3. Look at what Cut Craft would send with it, and **untick** anything you would rather keep
   to yourself (see below).
4. Press **Submit to GitHub**. The report becomes a public issue on GitHub that the people fixing Cut
   Craft read.

## A wrong word in another language

While you use a [language pack](help:language), the form starts with a line such as **A
wrong or missing word in Русский?** and a **Tell its translators** link. Language packs are
written by volunteers, so a wrong or missing word goes to them on GitHub, not in this
form. Anything else that goes wrong still goes in the form as usual.

## What is sent

Each thing has its own tick box, and the form shows exactly what each one says:

- **A picture of the Cut Craft window**: unticked until you tick it. It shows your design.
- **Cut Craft's log** (or **The crash log**): what Cut Craft was doing, the tools used and
  how each ended. Your user name and home folder are taken out.
- **Details of my computer**: ticked unless you untick it. It says:
  - **System**: which operating system and its exact version, for example
    *Windows 11 Pro 23H2 (build 22631.4460)*, *macOS 14.5 Sonoma*, or a Linux such as
    *Debian GNU/Linux 12* or *Arch Linux*. Windows 11 still calls itself Windows 10
    inside, so Cut Craft tells the two apart by the build number.
  - **Processor**: whether this computer has an Intel or AMD processor (*x86_64*) or an
    ARM one (*aarch64*, as in Apple's M-series Macs and many small Linux boards).
  - **Graphics**: the graphics card and how Cut Craft draws with it (Vulkan, Metal,
    DirectX or OpenGL), which matters for problems with the 3D view.
  - On Linux, the kernel version and whether the desktop is Wayland or X11. On a Mac running
    the Intel build on an Apple processor, that it runs through Rosetta.

  Many bugs only happen on one kind of computer, and few people know these details by heart,
  so leaving them ticked helps a fix come sooner. They never include your computer's name,
  your user name or any of your files.

The version of Cut Craft and where you clicked are always sent; they are listed under
**Always sent** in the form.

## Switching the computer's details off

**Edit ▸ Options ▸ Bug Reports ▸ Send details of my computer** is on to start with. Switch it
off and press **Apply**, and:

- each new bug report starts with **Details of my computer** unticked (you can still tick it
  for one report), and
- a crash log sent by itself carries only the crash and its log. It says whether the
  computer runs Windows, macOS or Linux, and nothing more about it.

## Related

- [Changing settings](help:settings): how Apply, Cancel and Close work in Options.
- [Updates](help:updates): a bug may already be fixed in a newer version.
