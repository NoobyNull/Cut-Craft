# src/clipart/properties.rs
clipart-properties-close = Close
clipart-properties-copies-where-the-file-is = Copies where the file is, to paste somewhere else.
clipart-properties-copy-path = Copy Path
clipart-properties-locate = Locate
clipart-properties-properties = Properties
clipart-properties-shows-the-file-in-its = Shows the file in its folder.

# A file's size on disk.
clipart-properties-bytes = { $count ->
    [one] { $count } byte
   *[other] { $count } bytes
}
clipart-properties-size-and-bytes = { $size } ({ $bytes } bytes)
# What a kind of file is called.
clipart-properties-kind-model = 3D model
clipart-properties-kind-drawing = 2D drawing
clipart-properties-kind-picture = Picture
clipart-properties-kind-depth-map = Depth map (heights as shades of grey)
clipart-properties-kind-file = { $kind } ({ $extension } file)
# The rows of the Properties window.
clipart-properties-row-kind = Kind
clipart-properties-row-size = Size
clipart-properties-row-model-size = Size
    .tooltip = Width (X) × length (Y) × thickness (Z), as saved in the file. You can change its size when you add it to a project.
clipart-properties-row-triangles = Triangles
    .tooltip = A model's surface is made of flat triangles. More of them means finer detail, and a slower file to open.
clipart-properties-row-preview = Preview
    .tooltip = The picture beside it with the same name, shown as its tile.
clipart-properties-row-file-size = File size
clipart-properties-row-modified = Modified
clipart-properties-row-tags = Tags
    .tooltip = Words to find it by: the search box looks through them.
clipart-properties-row-library-folder = Library folder
    .tooltip = The folder you added to the Library that this file is in.
clipart-properties-row-network-location = Network location
    .tooltip = Where it is on your network share; the path below is where it's mounted now.
clipart-properties-row-location = Location
clipart-properties-reading-the-file = Reading the file...
clipart-properties-could-not-measure = Couldn't measure it. { $error }
clipart-properties-pixels = { $width } × { $height } pixels
clipart-properties-reading = Reading...
clipart-properties-unknown = Unknown
clipart-properties-no-tags-yet = None yet. Right-click it and pick Add Tag... to add one.
