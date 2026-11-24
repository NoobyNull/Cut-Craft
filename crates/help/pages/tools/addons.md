---
reviewed: 2026-11-23
see-also: relief_from_photo, open_gadgets_folder, settings, language, confirmation_choices, web_api
---

**Add-ons** brings in extra tools that are too big to come with Cut Craft, such as the Depth
add-on that [Relief from Photo](help:relief_from_photo) uses, and gadgets other people have
written. Everything you install is listed in **Gadgets ▸ My Gadgets**, and gets a
**Gadgets** section among the tool buttons.

## Steps

![The steps, one after another](pictures/addons/tour.gif)

1. Open **Gadgets ▸ Add-ons...** and press **Install** beside the add-on you want. One that is not official asks before it installs.

   ![Step 1](pictures/addons/step-1.png)

2. Once it is installed it is listed in **Gadgets ▸ My Gadgets**, and a **Gadgets** section after the other tools holds it. Click it there to use it.

   ![Step 2](pictures/addons/step-2.png)

## Install an add-on

1. Open **Gadgets ▸ Add-ons...** (also at the foot of **Gadgets ▸ My Gadgets**).
2. Press **Install** beside the add-on you want. For one that is not on the list, type its
   GitHub repository under **Install from GitHub** (such as `owner/name`, or paste its link)
   and press **Install**.
3. If the add-on is **not official**, Cut Craft says so (**⚠ Not official**) and asks before
   installing it. You can still install it with **Install Anyway**, but nobody has checked
   what it does, so do that only if you trust whoever made it.
4. When it is done, the add-on shows in **Gadgets ▸ My Gadgets** straight away.

## Languages

Under **Languages**, each language with a pack has its own **Install** button. A language
pack doesn't appear in My Gadgets. Pick it in **Edit ▸ Options ▸ Language** instead (see
[Using Cut Craft in another language](help:language)).

## Use what you installed

- **From the menu**: open **Gadgets ▸ My Gadgets** and click it. An add-on opens the tool
  it brings: **Depth (Relief from Photo)** opens Relief from Photo.
- **From the tool buttons**: once a gadget, or an add-on that brings a tool (such as Depth),
  is installed, a **Gadgets** section appears. An add-on Cut Craft has no tool for is only
  in My Gadgets, where clicking it opens Add-ons.
  It always comes after every other group, so no button you know moves for it.
  - **Classic**: on the Design page, under the sub-tabs. If the page shows only your
    favourites, right-click the **Design** side tab and pick **Show All Tools** first.
  - **Rail**: the last button in the column of group icons, after **★** Favourites.
  - **Ribbon**: a **Gadgets** tab, after Favourites.

While nothing is installed, My Gadgets says **Nothing installed yet** and there is no
Gadgets section.

## Official and not official

An **official** add-on is signed with Cut Craft's release key: Cut Craft's author made or
checked it, and Cut Craft checks the signature when it downloads it. The Depth add-on is
official. A copy installed before it was signed still says **not official**: press
**Remove**, then **Install** again.

Anything else is **not official**: a gadget you put in the My Gadgets folder yourself, or an
add-on from someone else's GitHub repository. It still installs and works, but:

- Cut Craft warns you before installing it, and asks before its code first runs (see
  below).
- The Add-ons window says so beside it, such as
  **Version 0.1.0 installed, ⚠ not official (not signed)**.
- **Gadgets ▸ My Gadgets** shows **⚠ (not official)** after its name.

## Running unsigned code

Nothing that is not official runs until you say so. The first time you use a gadget or
add-on that is not official, Cut Craft asks **Do you want to run this unsigned code?** and
explains that nobody has checked it, and that Cut Craft and its author are not responsible
for what it does. You choose:

1. **Run and Remember** runs it now, and from now on without asking, until it changes.
2. **Run This Time** runs it now, and asks again after Cut Craft closes.
3. **Don't Run** leaves it alone. Cut Craft asks again the next time you use it.

"Until it changes" means Cut Craft remembers the code itself, not just its name. If an
update arrives, you edit the file, or someone puts a different file in its place, you are
asked again.

A gadget you have not said yes to is listed under its file's name (such as
**star-burst.lua ⚠ (not official)**). None of its code runs, not even to read its name,
until you say yes. After that it shows its own name and settings.

Changed your mind? **Edit ▸ Options ▸ Confirmation Choices ▸ Forget These Answers** makes
every unsigned gadget and add-on ask again ([Confirmation Choices](help:confirmation_choices)).
Scripts using the [Web API](help:web_api) can't answer the question for you: they can use
unsigned code only after you have said yes in the window.

## Update or remove

Open **Add-ons...** again: an add-on with a newer release shows an **Update** button.
**Remove** asks first, because you would have to download it again to use it. A removed
add-on leaves My Gadgets at once; a removed gadget leaves the menu when Cut Craft restarts.

When one of Cut Craft's own add-ons moves to a new home on GitHub (the Depth add-on and the
language packs are moving to the Power-Whittle organisation), you don't need to do anything.
Cut Craft knows both addresses and looks in Power-Whittle first, falling back to the old
address while the new one is still empty. Your copy keeps updating, and its next update
quietly records the new address. You never need to remove it and install it again.

## Your own gadgets

A gadget is a small `.lua` file. Pick **Gadgets ▸ My Gadgets ▸ Open My Gadgets Folder**,
put the file there, and restart Cut Craft: it shows in My Gadgets and in the Gadgets
section. See [Open My Gadgets Folder](help:open_gadgets_folder).
