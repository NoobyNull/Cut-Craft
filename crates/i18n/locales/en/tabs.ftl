# Tabs: the small bridges of material that hold a cut-out piece in place.
# Shown at the foot of a Profile Toolpath form's Tabs, and in its Edit Tabs window.

# At the foot of the form's Tabs.
tabs-loose = This cuts the piece right out with nothing holding it, so it can move into the cutter or be thrown. Add tabs, or hold it down another way (tape, screws, a vacuum table).
# $count: how many of the shapes cut out have no tab.
tabs-untabbed = { $count ->
    [one] { $count } of the shapes this cuts out has no tab, so it comes loose. Each tab you click goes on the shape nearest it: click on it in Edit Tabs.
   *[other] { $count } of the shapes this cuts out have no tab, so they come loose. Each tab you click goes on the shape nearest it: click on each one in Edit Tabs.
}
tabs-add = Add Tabs
    .tooltip = Leave small bridges of material that hold the piece in place
    .tooltip-window = Put the tabs on the selected vectors, as set above, to move by hand (select the vectors first: with none selected, nothing changes)
tabs-edit = Edit Tabs…
    .tooltip = Place the tabs: how many, where, their size, 2D or 3D

# The form's tabs in one line. $length and $thickness come with their unit, such as "6 mm".
tabs-summary-size = { $length } long, { $thickness } thick
tabs-summary-size-from-tool = { $length } long, { $thickness } thick (from the tool)
# $distance comes with its unit.
tabs-summary-automatic = about one every { $distance }, 3 or more a shape (fewer if small)
tabs-summary-number = { $number } a shape
tabs-summary-distance = one every { $distance } ({ $minimum } to { $maximum } a shape)
# $size is one of the tabs-summary-size lines above.
tabs-summary-placed = { $size }, { $placed } placed by hand
# $size is a tabs-summary-size line, $spread the line for how many.
tabs-summary-evenly = { $size }, { $spread }
tabs-summary-at-start = { $size }, { $spread }, the first at the start
tabs-summary-avoid-corners = { $size }, { $spread }, off the corners

# The Edit Tabs window.
tabs-window-title = Edit Tabs
tabs-delete-all = Delete All Tabs
    .tooltip = Take every tab off; they stay placed by hand, so click the vectors to put new ones on
tabs-placed = { $count ->
    [one] { $count } tab placed
   *[other] { $count } tabs placed
}
tabs-how-to-place = In the 2D view: click a selected vector to add a tab, click a tab to take it off, drag a tab to move it
tabs-all-deleted = All tabs deleted: click the vectors in the 2D view to place new ones, or pick another spacing
tabs-select-first = select the vectors to put the tabs on
tabs-added = { $count ->
    [one] { $count } tab added
   *[other] { $count } tabs added
}

tabs-flat = 2D Tabs
    .tooltip = Up at the start of each tab, across it flat, down at its end
tabs-triangular = 3D Tabs
    .tooltip = Up to the tab's thickness at its middle and down the other side
tabs-size-from-tool = Size from the tool
    .tooltip = As thick as the pass depth (at most half the cut), twice as long
tabs-length = Length (width along the path)
tabs-thickness = Thickness (height)
tabs-how-many = How many, and where
tabs-automatic = Automatic
    .tooltip = About one every distance below, 3 or more on a shape, but never more than a third of it in tabs, so a small shape gets fewer (1 or 2): a safe start
tabs-number = Number
    .tooltip = This many on every shape, spread along it
tabs-distance-apart = Distance apart
    .tooltip = One about every this far along each shape
tabs-at-least = At least
tabs-at-most = At most
tabs-where-clicked = Where clicked
    .tooltip = Placed by hand: Add Tabs below, then click, drag or take them off
tabs-evenly = Evenly
tabs-first-at-start = First at the start
tabs-not-on-corners = Not on corners
    .tooltip = Moved along a little to keep them off corners and curves
