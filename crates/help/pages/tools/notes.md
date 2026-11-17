---
reviewed: 2026-11-17
see-also: document_variables, print_job_sheet, create_job_sheet
---

## Writing notes

**Notes** (Design page, or Edit ▸ Notes) keeps notes with the job: how to clamp it, which
bit goes in first, what finish to use. The box fills the panel, so there is room to write.

![Writing notes, step by step: an empty box, a heading and a check list typed with the toolbar, a variable put in from the Variable Explorer, then Preview](pictures/notes-steps.gif)

1. Press **Notes**. The box takes the whole panel, with the **Variable Explorer** under it.
2. Type, and use the toolbar for the look: **B** bold, **I** italic, **H** a heading,
   **•** a bulleted list, **1.** a numbered list, **☐** a check list and **Link** a link
   to a web page. Pick some words first to change just those, or the lines they are on.
3. Put in a live value (see below).
4. Press **Preview** to see the notes as they will read (tick a check box there to tick
   it in the notes), then **Apply** to keep them.

The toolbar only types plain marks you can also type yourself, like a text message:
`# Heading`, `- bullet`, `1. numbered`, `- [ ] check box`, `**bold**`, `*italic*` and
`[words](https://example.com)`. Nothing typed is ever read as a web page's code (HTML), so
notes cannot run anything. Only web pages (`https://`) and e-mail addresses (`mailto:`)
are links; anything else is shown as you typed it.

## Live values: variables

A **variable** is a value Cut Craft fills in for you, written between double braces, like
`{{material.thickness}}`. It always shows the value now: change the material in Job Setup
and the notes change with it. While writing, a variable shows as a coloured chip (red if
it is spelt wrong); point at one to see its value. **Preview** shows every value.

![The Variable Explorer searched for "pocket": the pocket toolpath's tool, feed rate, plunge rate and spindle speed, each with its value now](pictures/notes-explorer.png)

The **Variable Explorer** lists every variable by where it comes from:

- **Job**: its name, file and folder, and its units.
- **Material**: size, width, height, thickness, where Z zero is, the XY origin and the
  safe Z height.
- **Model**: the 3D model's height and the gaps above and below it in the material (see
  [Material Setup](help:material_setup)).
- **Machine**: the machine, its controller, post processor, bed size and tool changer.
- **Tools**: each bit the toolpaths use, numbered, with its diameter, pass depth,
  stepover, feeds, speed and flutes, plus a list of them all.
- **Toolpaths**: each toolpath, numbered in list order, with its tool and tool number,
  feeds and speed, stepover, pass depth, run time and its own notes from the Toolpath
  Summary, plus a list of them all and the total run time.
- **Document Variables**: the ones you set in Edit ▸ Document Variables.
- **System**: today's date, the time and the Cut Craft version.

Each row shows a name and its value now; point at it for what it is and how to type it.
To put one in the notes:

- **double-click** it (it goes where the text cursor is), or
- **drag** it into the box and let go where you want it, or
- **click** it, then press **Insert**.

Type in **Search** to find one by its name, its value or what it is: `feed`, `thick`,
`pocket`. **{x} Variables** hides the explorer to give the box the whole panel.

## Where the notes go

![The notes at the top of the G-code, as comments, before the first move](pictures/notes-gcode.png)

- **Show these notes when the job opens**: they open by themselves next time, a good
  place for "read me first".
- **The job sheet** (Toolpaths ▸ Create / Print Job Sheet, or ticked in Save Toolpaths)
  shows them with headings, lists and ticks, every variable filled in.
- **Put these notes at the top of the G-code**: each saved file starts with them as
  comments, which the machine skips. They come before the first move, so a sender that
  shows comments shows them before cutting. Brackets and semicolons are swapped for ones
  that cannot end the comment early, and long lines are broken to fit small controllers.
- The **Web API** reads and writes them with `GET notes` and `POST notes` (see
  [Web API](help:web_api)).

![Preview: the notes read with headings, ticks and every value filled in](pictures/notes-preview.png)
