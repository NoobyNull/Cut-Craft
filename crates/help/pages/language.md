---
reviewed: 2026-11-25
see-also: settings, addons, bug_reports
---

# Using Cut Craft in another language

Cut Craft is written in English, and English is always built in. Other languages come as
**language packs**: small downloads, each one language, installed only if you ask for one.
A pack never comes with Cut Craft itself.

## Install a language

1. Open **Gadgets ▸ Add-ons...**.
2. Under **Languages**, press **Install** beside the language you want. Each language is
   its own install, so you only get the ones you pick.
3. When it's done, the window says to pick it in **Edit ▸ Options ▸ Language**.

The packs are published together on GitHub, but each one downloads on its own. Each pack
says **Official, community supported**, then who translated it and how much is done, such
as **Translated by Ivan · 92% translated**.

## Official, community supported

- **Official**: Cut Craft checked the pack and signed it with its release key. A pack holds
  only words, so it can't run anything or change your files.
- **Community supported**: volunteers write the words and keep them up to date, not Cut
  Craft's author. So a pack can have the odd wrong word, and new parts of Cut Craft show in
  English until its translators catch up.

**Igpay Atinlay (test)** is a pack for testing: it shows Cut Craft in Pig Latin, the tools'
forms and settings too. Any word still in plain English while it is picked has no
translation key yet, so no pack can translate it. A name put into a sentence, such as a file
name, a tool's name or an error, stays as it is in every language: only the words around it
show in Pig Latin. Right-click it and choose **Report a Bug Here…** to tell Cut Craft.

A pack that isn't signed still works, but it is marked **⚠ not official**, the same as any
other [add-on](help:addons).

## Report a wrong or missing word

Tell the pack's translators, not Cut Craft:

- In **Edit ▸ Options ▸ Language**, with the pack picked, press **Report a wrong or missing
  word**. It opens the pack's page on GitHub, where you can describe it.
- In the **Report a Bug** form while you use a pack, the line at the top has **Tell its
  translators**, which goes to the same page. Use the form itself for anything else that
  goes wrong (see [Reporting a bug](help:bug_reports)).

## Switch language

![Edit ▸ Options ▸ Language, with a pack picked: "Official, community supported", who translated it, how much is done, and where to report a wrong word](pictures/options-language.png)

1. Open **Edit ▸ Options** and pick the **Language** page.
2. Pick the language. Beside each pack it says who translated it and how much of Cut
   Craft it covers, such as **Official, community supported · Translated by Ivan · 40%
   translated**.
3. Press **Apply**. Like every Options page, nothing changes until then (see
   [Changing settings](help:settings)).

To go back, pick **English (built in)** and press **Apply**.

## Words still in English

Translating takes time, so a pack can be partway done. Anything a pack has no translation
for shows in English, so nothing is ever missing or blank. A pack can translate the tools'
names too: the tool buttons, their groups, the sub-tabs and page tabs, the menus, **Find
Tool** (**Ctrl+K**) and the help pages' titles all use the same words, so a tool is named the
same way wherever you see it. It translates what each tool does in its tooltip, every
tool's form and settings, the right-click menus of your layers, sheets, levels and
toolpaths, what the status bar says, and the hints and buttons around the 2D view and the
layers list, Machine Configuration, the Tool Database and the Report a Bug form, and what a
tool says when it cannot run, too. When a newer pack is published, **Gadgets ▸ Add-ons**
offers to update it.

If a pack you picked is removed, Cut Craft goes back to English the next time it starts
and says so in the status bar.

## Test language

With **Show advanced settings** ticked, the Language page also offers a **Test language**.
It's English with every word accented and stretched about 40% longer, in brackets. It's for
checking that longer languages fit, because German or French text can run that much longer.
Any word that shows without brackets is still written straight into Cut Craft's code
instead of coming from a language file.

## Making a pack

Anyone can translate Cut Craft. See **docs/translating.md** in Cut Craft's source on
GitHub. It shows how to copy the English files, translate them, test your pack, and put
your name on it. A pack that passes the automatic checks is signed and published without
anyone reading it first.
