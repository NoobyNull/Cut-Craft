[Contents](help:contents)

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

![The Web API page in Options, switched on, with a command to try](bytes://help/web-api-options.png)

## Letting other computers in

Tick **Allow other computers on my network** to let another computer (a workshop PC, a
laptop) drive Cut Craft. Then every call, from any computer, must send the **access token**:

1. Press **Copy** beside the access token.
2. Give it to the program that will call Cut Craft, on the other computer.

![The Web API page with other computers allowed, showing the access token](bytes://help/web-api-network.png)

- Calls are not encrypted, so only allow other computers on a home or workshop network you
  trust. On Windows, the first time, Windows asks whether to let Cut Craft through its
  firewall: allow it on private networks only.
- **Port** is the number programs call, as in `http://localhost:7340`. Pick another if a
  different program already uses it.
- **Make New Token** stops the old token working, for example if someone you did not mean
  to have it does.

## The status bar

While it is switched on, the status bar says how it is doing. Click it to open its settings.

![Green WEB API Running in the status bar](bytes://help/web-api-running.png)

Green **WEB API Running**: programs can call it. Point at it to see the address.

![Red WEB API Not Running in the status bar, pointed at, saying the port is in use](bytes://help/web-api-not-running.png)

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

![The docs page in a web browser, listing every call](bytes://help/web-api-docs.png)

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
  the open form and the status bar's message.
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
- **POST run** `{"command": "draw_circle", "params": {"radius": 10}}`: runs a tool with
  these settings straight away, with no form. Settings left out are what its form starts
  with; a misspelt setting is refused with the names there are.
- **POST invoke** `{"action": "undo"}`: presses a button or menu item. A greyed-out one is
  refused. Exit cannot be pressed from the Web API. An item that opens a file window waits
  for you to pick a file in Cut Craft.

A call that cannot be done changes nothing and answers with `{"error": "..."}` saying why
and what to do. While Cut Craft is busy (a toolpath calculating), calls wait their turn.

See also: [Toolpaths](help:toolpaths), [Design](help:design).
