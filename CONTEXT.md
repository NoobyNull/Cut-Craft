# Cut Craft

The words Cut Craft uses for its own ideas, so code, help, docs and conversations mean the same thing.

## Rotary (4th axis)

**A axis**:
The rotary axis that turns the material about the turning axis, in degrees.
_Avoid_: 4th axis (in code), rotation axis

**Free turning**:
A turns one way as far as the job needs, any number of turns, and never winds back. Set per machine.
_Avoid_: continuous mode, unlimited rotation

**Lathe mode**:
Cutting a part that is round all the way along (every slice a circle): A spins steadily like a lathe spindle while the bit follows the part's outline in X and Z (or Y and Z). Needs **Free turning**. Whether it cuts in a **Spiral** or **Rings** is the calculator's choice, to save time.
_Avoid_: using lathe mode for parts that are not round

**Step per turn**:
How far the bit moves along the axis for each full turn of A in **Lathe mode**. Roughing uses a big step per turn, finishing a small one; A's speed stays the same.
_Avoid_: pitch, feed per rev (in the UI)

**Turned part**:
A part that is round all the way along, such as a candlestick or table leg. Cut in **Lathe mode**, start to finish, by **Rotary Lathe**'s Turn.

**Carved part**:
A part that is not round, such as a statue. Needs the carving movements (**Typewriter**, locked angles, off centre) as well as or instead of **Lathe mode**.

**Unwind**:
The opposite of free turning: A is held to a set number of turns either way, then cuts back or spins back.

**Spiral**:
A movement where A keeps spinning while the bit feeds steadily along the axis: one unbroken cut from end to end, like a thread.
Always moving along the axis, so every cut leans a little: never for a level groove.
_Avoid_: helix, continuous pass

**Along the part** / **Round the part**:
The two ways a carving pass on a **Carved part** can run, named so on the form, each with a small picture. Along the part is the **Typewriter** action; Round the part uses **Spiral** and **Rings**, picked by the calculator. The user picks one; roughing defaults to Along, finishing to Round. Help notes that other CAM calls them axial and radial.
_Avoid_: Axial, Radial, Typewriter (as form names)

**Locked angles**:
A carving pass where A turns to an angle and holds still while the bit cuts that side like a flat 3D job, then on to the next angle (often called indexed or 4+1). Its own toolpath. The user sets the most angles; the calculator picks the best ones and stops early when another angle would reach nothing new (a cube needs 4, not 8).
_Avoid_: Indexed (old strategy name), tilts

**Rotary Details**:
The rest pass on a **Carved part**: a smaller bit cuts only what the earlier rotary toolpaths left. Its own toolpath.
_Avoid_: rest machining (in the UI), sides then details

**Off centre**:
The bit held a fixed distance to one side of the turning axis, so the side of the ball reaches steep walls facing that way. One side or Both sides (a second pass held to the other side).

**Offset**:
The bit follows the surface: Y and Z move together all the time while A turns, the calculator choosing how far to the side at each point, to reach steep walls and overhangs in one pass. Not for **Lathe mode**. Planned last, though expected to be the most used, being the least effort for the user.
_Avoid_: using Offset for the fixed **Off centre**

**Rings**:
A movement where the bit holds one spot along the axis while A turns a full turn, then steps along and does the next. Each ring is level, so a groove or bead round the part comes out even on both sides, which a **Spiral** cannot do.
_Avoid_: radial (the old strategy name), circles

**Typewriter**:
A movement where A holds still while the bit runs the whole length of the part along the axis, then A turns one step, and repeat. The rotary twin of a flat raster. "Typewriter" describes the action (in help and talk); it is not the name of a strategy on a form.
_Avoid_: raster along, indexed lines; Typewriter as a strategy name

**Cut direction**:
Climb, Conventional or Both, judged from which way the spindle turns against the way the material meets the bit. On a rotary job, A's turning direction decides it for a **Spiral**, and the direction of travel along the axis decides it for **Typewriter**. "Both" cuts every other line the opposite way.
_Avoid_: combined (say Both), zigzag

**Rotary Lathe**:
The toolpath that faces, turns and parts off with A spinning, in **Lathe mode**. Turn roughs and then finishes a **Turned part** to its outline.
_Avoid_: lathe mode (that means free turning)

## Relationships

- In **Lathe mode**, A's speed is worked out to keep the bit's chipload right, up to the machine's top A speed; finishing only changes the **Step per turn**.
- A **Carved part** is cut by separate toolpaths: Rotary Roughing, Rotary Finishing, **Locked angles**, **Rotary Details**; one bit each. Saving can group them by tool.
- Rotary toolpaths, including flat cuts on a locked face, can come in any order; the user's order in the toolpath list is the cutting order.
- "What's left" for a rotary toolpath = the material after every toolpath above it in the list. Reordering warns that later toolpaths are out of date, with a way to put the move back.
- An Advanced switch (a user setting in Settings, hinted from the toolpaths menu by "Turn on advanced mode", which opens Settings with it highlighted) turns off working out "what's left" for every rotary toolpath at once. Each toolpath's own rest machining (the bits listed as used before it) works as it does on the flat surfacing toolpaths.
- **Spiral** and **Rings** are not user choices: the calculator picks between them along the part (Matthew, 2026-11-24).

- A machine is either **Free turning** or **Unwind**; every rotary toolpath obeys it.

## Flagged ambiguities

- "Lathe mode" was first read as the machine's free-turning setting. Resolved: **Lathe mode** is a way of cutting **Turned parts**; **Free turning** is the machine setting it needs.
