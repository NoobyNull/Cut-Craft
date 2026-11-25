# src/app/check_list.rs
check-list-checking-the-toolpaths-a-big = Checking the toolpaths: a big 3D toolpath takes a little while
check-list-checks-before-cutting = Checks before cutting
check-list-tick-to-say-you-have = Tick to say you have read this and it is what you want
check-list-tick-to-save-with-error = Tick to say you have read this: the toolpaths can then be saved with it, and Save Anyway asks once more
check-list-not-checked = not checked: { $why }

# The line over the finished checks; the parts are joined with " · ".
check-list-count-passed = ✔ { $count } passed
check-list-count-to-read = ⚠ { $count } to read
check-list-count-damage = ⛔ { $count } could damage the machine
check-list-count-must-fix = ✖ { $count } must fix
check-list-count-not-checked = { $count } not checked
check-list-what-each-check-looks-for = What each check looks for, and how to fix what it finds
check-list-read-each-and-tick = Read each ⚠ and tick it to say you have: Save Toolpaths and Send to Machine wait until every one is ticked
check-list-refused = ⛔ Not saved or sent while this could damage the machine: put it right, or switch its check off in Machine Configuration's Protect the Machine
check-list-switch-off-to-save = No tick: this could damage the machine. Put it right, or switch the check off in Machine Configuration's Protect the Machine
# A check's problem that could damage the machine: never saved while so.
check-list-damage-message = Could damage the machine: { $message }
# A check's problem that has to be fixed before cutting.
check-list-must-fix-message = Must fix: { $message }
