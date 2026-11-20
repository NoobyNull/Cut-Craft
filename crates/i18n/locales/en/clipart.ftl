# src/clipart.rs
clipart-add-folder = Add Folder...
clipart-add = Add
clipart-cancel = Cancel
clipart-clear = Clear
clipart-click-a-folder-in-the = Click a folder in the list first.
clipart-click-one-of-your-folders = Click one of your folders in the list first.
clipart-deletes-the-file-for-good = Deletes the file for good. You're asked first.
clipart-each-model-on-a-new = Each model on a new level named after it.
clipart-folders = Folders
clipart-its-size-date-tags-and = Its size, date, tags and where it is.
clipart-locate = Locate
clipart-new-level = New Level
clipart-no-folders-yet-add-a = No folders yet. Add a folder of your models and pictures to see them here.
clipart-nothing-here-matches-clear-the = Nothing here matches. Clear the search or choose Everything.
clipart-nothing-here-yet-3d-models = Nothing here yet. 3D models (STL, OBJ, .ccclip), pictures and drawings (SVG, DXF) show up here.
clipart-open-folder = Open Folder
clipart-properties = Properties
clipart-reading-your-folders = Reading your folders...
clipart-remove-folder = Remove Folder
clipart-remove-tag = Remove Tag
clipart-search-names-and-tags = Search names and tags
clipart-show-one-kind-of-file = Show one kind of file. Depth maps are greyscale pictures, or pictures you've tagged "depth map".
clipart-shows-the-file-in-its = Shows the file in its folder.
clipart-shows-the-folder-in-your = Shows the folder in your file manager.
clipart-shows-what-s-in-every = Shows what's in every folder at once.
clipart-takes-the-folder-out-of = Takes the folder out of the Library; its files stay on disk.
clipart-e-g-animals-depth-map = e.g. animals, depth map
# Where the files shown are.
clipart-place-everywhere = all your folders
clipart-place-folder = { $folder } and the folders in it
# $place is one of the clipart-place lines.
clipart-files-in = { $count ->
    [one] { $count } file in { $place }
   *[other] { $count } files in { $place }
}
clipart-some-files-in = { $count ->
    [one] { $shown } of { $count } file in { $place }
   *[other] { $shown } of { $count } files in { $place }
}
clipart-connecting = Connecting to { $place }...
clipart-no-preview = No preview: { $trouble }
clipart-tag-files = { $count ->
    [one] Tag:
   *[other] Tag for { $count }:
}
clipart-tags = Tags: { $tags }
clipart-too-many-files = There are more than { $most } files in your folders, so only the first { $most } are shown. Add smaller folders to see the rest.
clipart-picked = { $count } picked

# The Show list: which kinds of file the Library shows.
clipart-show-everything = Everything
clipart-show-models = 3D models
clipart-show-drawings = 2D drawings
clipart-show-pictures = Pictures
clipart-show-depth-maps = Depth maps
clipart-could-not-delete = Couldn't delete { $name }: { $error }
clipart-could-not-read = Couldn't read { $name }: { $error }
clipart-nothing-to-show = { $name } has nothing in it to show
clipart-could-not-save-tags = Couldn't save the Library's tags: { $error }
clipart-all-folders = All folders ({ $count })
# Beside a Library folder that isn't there right now.
clipart-folder-connecting = connecting...
clipart-folder-cant-reach = can't reach
clipart-folder-not-found = not found
clipart-cant-reach-on-network = Can't reach { $place } on the network right now. Check that the computer or NAS it's on is turned on and that you're on the same network. If your file manager asks for a password for it, open it there once. It shows up here again by itself as soon as it can be reached.
clipart-folder-is-not-there = { $place } isn't there. If it's on a drive that isn't plugged in, plug it in; if it was moved, use Remove Folder and add it again from where it is now.
clipart-tile-hint = { $name }: double-click to add it to the project, or drag it onto a view. Click to pick it (Ctrl or Shift for more), right-click for more.
# The right-click menu on Library files; with more than one picked, how many.
clipart-menu-count = { $what } ({ $count })
clipart-menu-add-to-project = Add to Project
clipart-menu-add-to-level = Add to Level
clipart-menu-add-tag = Add Tag...
clipart-menu-delete-from-disk = Delete from Disk...
