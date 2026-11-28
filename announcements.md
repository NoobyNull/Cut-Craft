# Announcements

Cut Craft reads this file from GitHub when it starts (unless Edit ▸ Options ▸ Updates says not
to) and shows each note below at the first start of each day the note is meant for, until the
person ticks **Don't show this again**. Help ▸ Announcements shows the current ones at any time.
Nothing here is written by the program: a note shows only when one is added here.

To add a note, put a header and its words at the end of this file, and open a pull request:

    ---
    id: open-day-2026
    title: Workshop open day
    from: 2026-12-05
    until: 2026-12-06
    ---
    Come and **see the machines**. Markdown works: headings, **bold**, lists and
    [links](https://github.com/Power-Whittle/PowerWhittle).

The `---` lines start at the left edge (the example above is indented only so it is not read as
a note).

- `id`: a short name, never reused. "Don't show this again" is kept against it.
- `title`: up to 80 letters.
- `date: 2026-11-26` for one day, or `from:` and `until:` for a range (either can be left out
  for "any day before" or "any day after"). Days are the person's own calendar day.
- `support: yes` adds the Support us on GitHub row under the note.
- `lang: ru` with the same `id` as an English note is that note in Russian, shown instead of it
  to people using Cut Craft in Russian.

The words can be up to 4,000 letters. A note that will not read is left out, and the app's
tests (`announcements::tests`) say which line is wrong. Delete a note once its days are over.
