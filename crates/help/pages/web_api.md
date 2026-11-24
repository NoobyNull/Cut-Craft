---
reviewed: 2026-11-23
see-also: updates, smooth_selected_components, component_properties
---

# Web API

The Web API lets a program drive Cut Craft the way you do with the mouse: pick vectors, run
any tool with the settings you choose, and press any button or menu item. Use it for scripts
that repeat a job, for tests, or to let an assistant such as Claude work in Cut Craft while
you watch. Everything it does shows in the window and can be undone, one step per call.

It is **off** until you switch it on, and it never sends anything to your machine: Cut Craft
saves files for your machine, it does not drive it.

## Switching it on

1. Open **Edit ▸ Options ▸ Web API**.
2. Tick **Turn on the Web API**.
3. Press **Apply** at the foot of the window. Settings wait for Apply: see
   [Changing settings](help:settings).

That is all for programs on this computer: they need no password.

![The Web API page in Options, switched on, with a command to try](pictures/web-api-options.png)

## Letting other computers in

Tick **Allow other computers on my network** to let another computer (a workshop PC, a
laptop) drive Cut Craft. Then every call, from any computer, must send the **access token**:

1. Press **Copy** beside the access token.
2. Give it to the program that will call Cut Craft, on the other computer.

![The Web API page with other computers allowed, showing the access token](pictures/web-api-network.png)

- Calls are not encrypted, so only allow other computers on a home or workshop network you
  trust. On Windows, the first time, Windows asks whether to let Cut Craft through its
  firewall: allow it on private networks only.
- **Port** is the number programs call, as in `http://localhost:7340`. Pick another if a
  different program already uses it.
- **Make New Token** stops the old token working, for example if someone you did not mean
  to have it does.

## The status bar

While it is switched on, the status bar says how it is doing. Click it to open its settings.

![Green WEB API Running in the status bar](pictures/web-api-running.png)

Green **WEB API Running**: programs can call it. Point at it to see the address.

![Red WEB API Not Running in the status bar, pointed at, saying the port is in use](pictures/web-api-not-running.png)

Red **WEB API Not Running**: it is switched on but could not start. Point at it to see why,
fix that (most often by picking another port), then press **Try Again** in Options.

## Trying it

**Copy Command** under **Try it** copies a command (with your token in it, when other
computers are allowed) to paste in a terminal (on Windows, in PowerShell use `curl.exe`). Cut Craft answers with its version and
what it is doing. **Open Docs Page** opens the docs page (below).

## The docs page

Every call, and every tool and menu item, is listed on the **docs page**, ready to try in your
web browser. While the Web API is running, press **Open Docs Page** in Options, or go to
`http://localhost:7340/docs/` (with your own port in place of 7340).

![The docs page in a web browser, listing every call](pictures/web-api-docs.png)

1. Click a call, such as **POST /run**, to open it.
2. Under **Examples**, pick a tool (or, under **POST /invoke**, a menu item): its settings
   fill in, ready to change.
3. Press **Execute**. Cut Craft does it in the window, and the page shows its answer, and
   the `curl` command that does the same from a terminal or script.

The page always shows what this copy of Cut Craft has, add-ons included. While other computers
are allowed, press **Authorize** first and paste the access token: the page itself opens
without it, but every call tried from it needs it. The page works without the internet.

## The calls

Every call starts `http://localhost:7340/api/v1/` (from another computer, this computer's
address in place of `localhost`). While other computers are allowed, every call also sends
the header `Authorization: Bearer <token>`. Calls that send settings use POST with
`Content-Type: application/json`. Lengths are in the job's units unless a call says
`"units": "mm"` or `"in"`.

- **GET status**: the version, whether a job is open, whether it is busy, the job's units,
  the open form and the status bar's message, and **progress** as GET progress gives it.
- **GET commands**: every tool, by id, with its name and group.
- **GET commands/draw_circle**: one tool's settings: what each one is and what its form
  starts with.
- **GET actions**: every menu item and tool button, by id, with whether it can be used now.
- **GET job**: the layers, every vector (its id, layer, part name, size, open or closed,
  selected), the toolpaths and the selection.
- **POST select** `{"by": "layer", "layer": "Cut"}`: picks vectors, as clicking them does.
  `by` is `ids` (with `"ids": [3, 4]`), `layer`, `name` (a part's name), `inside`
  (with `x1`, `y1`, `x2`, `y2`), `all`, `open`, `closed` or `none`. Add `"add": true` to
  keep what was picked already. Vectors on hidden or locked layers are never picked.
- **POST select** `{"by": "component", "component": "Prop"}`: picks one part of the 3D
  model (a component) by its name, or by its id (`"component": 4`), as clicking it in the
  Components panel does. **GET state** lists each component's id and name. The component
  tools ([Smooth](help:smooth_selected_components), Emboss, Scale Z and the rest) then
  change only that component. Add `"add": true` to keep the components picked already. If
  two components share a name, the call asks for the id instead. Picking a component leaves
  the picked vectors as they are.
- **POST run** `{"command": "draw_circle", "params": {"radius": 10}}`: runs a tool with
  these settings straight away, with no form. Settings left out are what its form starts
  with; a misspelt setting is refused with the names there are.
  A tool that works on one component, such as Component Properties with
  `"params": {"component": 4, "visible": false}`, picks that component first, as
  right-clicking it in the window does: settings left out stay as they are on that
  component.
- **POST invoke** `{"action": "undo"}`: presses a button or menu item. A greyed-out one is
  refused. Exit cannot be pressed from the Web API. An item that opens a file window waits
  for you to pick a file in Cut Craft.

A call that cannot be done changes nothing and answers with `{"error": "..."}` saying why
and what to do. While Cut Craft is busy (a toolpath calculating), calls wait their turn.

## Seeing the job, and making its G-code

These calls show what the job looks like and makes, and run the same slow jobs as the
Toolpaths page's buttons, each with its progress window as usual.

- **GET view.png**: a picture of the job, as a PNG. Open
  `http://localhost:7340/api/v1/view.png?view=iso` in your web browser to see it.
  - `view` is `2d` (the 2D view), or the 3D view from the `top`, `front`, `right` or at an
    angle, `iso` (the default).
  - `preview=after` (the default) shows the toolpath preview, if there is one;
    `preview=before` shows the model as it is before cutting.
  - `width=800` makes the picture 800 pixels wide; left out, it is the view's own size.

  The window shows that view for a moment while the picture is taken, then goes back to how
  you left it. Windows and menus open over the view are left out of the picture. Cut
  Craft's window must be open on screen (not minimised) to take it.
- **GET state**: the material (size, thickness, where Z zero and the datum are), where the
  3D model sits in it, each part of the model with its heights, and what is picked:
  vectors, model parts and the toolpath picked in the toolpath list.
- **GET toolpaths**: every toolpath with its tool, its run time (as the Toolpath Summary
  works it out, for example `"time": "12m 30s"`) and what to look at before cutting it:
  `warnings` (out of date, entries made another way, what the checks before cutting found)
  and `errors`, which stop its G-code being made. `total_time` is the whole job's, the
  trip back home included (see [How long the job takes to cut](help:cutting_time)).
- **POST calculate** `{"toolpaths": "all"}`: recalculates toolpaths, as **Recalculate**
  does. `toolpaths` is `all` (the default), `visible` (the ticked ones), one toolpath's id,
  or a list of ids.
- **POST preview** `{"toolpaths": "all"}`: previews toolpaths into fresh material, as
  **Preview** does, cut at once rather than played. **GET view.png** then shows it.
- **POST preview/reset**: **Reset Preview**. **Undo Last Preview** brings it back.
- **GET progress**: while a calculation or preview runs, what it is doing and how far along
  (`"fraction"`, 0 to 1), how long it has run (`"seconds"`), and the time left worked out
  just as the progress window does ([Long jobs and the time left](help:progress)):
  `"seconds_left"` once it can judge, and `"time_left"` in words ("about 2 minutes left").
  It is answered straight away, even while Cut Craft is busy.
  **POST cancel** presses **Cancel** on the progress window: nothing changes.

  **POST calculate** and **POST preview** answer once they are done. Send `"wait": false`
  to have them answer straight away, then follow them with **GET progress**.
- **GET posts**: the post processors for this job on the active machine, and the one used
  when a call names none.
- **POST gcode** `{}`: the G-code **Save Toolpaths** would write for every shown toolpath:
  each file's name and text. It runs the same checks before cutting first:
  - `"toolpath": 3` makes one toolpath's G-code; `"group": "Lid"` a toolpath group's;
    `"each": true` puts each shown toolpath in a file of its own.
  - `"post": "GRBL (mm)"` uses that post processor (from **GET posts**).
  - `"save_to": "/home/me/sign.nc"` saves the files there as well. Give the whole path,
    with its folder. An earlier save there (the same file, or the tile, numbered and side
    files it made) is kept unless you send `"replace": true`. Then it is archived or
    replaced just as **Save Toolpaths** does, as **Edit ▸ Options ▸ Confirmation Choices**
    say (replaced when they say to ask), and put back if the new files can't be written.
  - An error the checks find stops it: nothing is made. A warning (a toolpath out of date
    is one) stops it too, as **Save Toolpaths** asks you, until you send
    `"accept_warnings": true`, which is like answering **Save Anyway**. Each warning is
    listed once, word for word as **Save Toolpaths** shows it.

  The G-code is only made and saved: it is never sent to your machine.
- **POST job/new** `{"settings": {"size": {"width": 300, "height": 150, "thickness": 12}}}`:
  **New**, with **OK** pressed in **Job Setup**. Settings left out are what Job Setup starts
  with (your **Set as Default** settings, if you saved some); **GET commands/job_setup**
  lists them all. Settings Job Setup will not take are refused, and the job open stays
  as it was.
- **POST job/open** `{"path": "/home/me/sign.ccjob"}`: **Open** on that job file.

  Neither throws away unsaved changes: they are refused until you save, or send
  `"discard": true`.
- **POST job/save** `{}`: **Save**. With `{"path": "/home/me/sign.ccjob"}` it is
  **Save As**: the job is that file from then on.
- **POST job/export** `{"format": "export_dxf", "path": "/home/me/sign.dxf"}`: an
  **Export** menu item. The docs page lists each format.
- **GET notes**: the job's [notes](help:notes) as typed (`text`), whether they open with
  the job (`opens_with_job`) and go at the top of the G-code (`in_gcode`), the notes with
  every variable filled in as plain text (`plain`) and as HTML (`html`), and every variable
  the Variable Explorer lists (`variables`: each one's `key`, `group`, `label`,
  `description` and `value` now).
- **POST notes** `{"text": "# Setup\n- [ ] Clamp the {{material.thickness}} board"}`:
  changes the notes, as **Apply** on the Notes form does (one undo). Send `in_gcode` or
  `opens_with_job` (`true` or `false`) to change those; anything left out stays as it is.
  It answers as **GET notes** does.

### An example: see a job, then make its G-code

```
curl "http://localhost:7340/api/v1/view.png?view=iso" -o before.png
curl -X POST http://localhost:7340/api/v1/calculate -H "Content-Type: application/json" -d "{}"
curl -X POST http://localhost:7340/api/v1/preview -H "Content-Type: application/json" -d "{}"
curl "http://localhost:7340/api/v1/view.png?view=top&width=800" -o after.png
curl http://localhost:7340/api/v1/toolpaths
curl -X POST http://localhost:7340/api/v1/gcode -H "Content-Type: application/json" -d "{\"save_to\": \"/home/me/sign.nc\"}"
```

[Toolpaths](help:toolpaths) explains the Toolpath Summary's run times and what the checks
before cutting look for.

See also: [Toolpaths](help:toolpaths), [Design](help:design).
