---
reviewed: 2026-11-17
see-also: group, ungroup, ungroup_onto_current_layer, edit_text
---

# Groups

[Group](help:group) joins the selected vectors so a click picks them all and they move
together. Nothing about them changes: text grouped with a border is still text, and a part
keeps its name. A group can hold other groups.

![Step inside a group to change one thing in it](pictures/group-step-inside.png)

1. **Double-click** a group to step inside it. Everything outside it fades.
2. Inside, a **click** picks one item: move it, size it, node edit it, delete it, or change it
   in its own tool, and it stays in the group. A group inside it is picked whole;
   double-click that one to go further in.
3. **Double-click text** (inside a group or not) to open [Edit Text](help:draw_text) on it.
4. **Esc**, or a double-click on empty space, steps back out one level. Clicking something
   outside the group steps out too.

[Ungroup](help:ungroup) takes a group apart one level: groups and text that were in it
come back out as they were. Ungrouping the text itself turns it into plain curves, which can
no longer be changed with Edit Text.
