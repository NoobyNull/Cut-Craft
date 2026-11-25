---
reviewed: 2026-11-14
see-also: pocket_toolpath
---

# Stepover

![A bit clearing a pocket in rows](pictures/concepts/stepover.gif)

When a bit clears an area, such as the inside of a [pocket](help:pocket_toolpath), it goes
back and forth in rows. The **stepover** (S) is how far over each new row moves.

- It is always less than the bit's width (Dia), so each row overlaps the last one and no thin
  ridge is left standing between them.
- A **smaller** stepover leaves a smoother floor but takes longer.
- A **bigger** stepover is faster but leaves more marks, and loads the bit harder.
- 40 to 50 % of the bit's width is a safe start for most wood. A finishing pass can go
  smaller, around 10 to 20 %.

Stepover is saved with each bit in the Bit Library, so picking a bit in a toolpath's form
brings its stepover with it.
