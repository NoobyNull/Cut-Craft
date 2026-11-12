//! What the app's own tools and menu items do, for those with no command crate to say it:
//! their tooltip and status bar text, and their help page when they have no form.

/// Each built-in action's id and what it does, in the person's words.
pub const HINTS: &[(&str, &str)] = &[
    (
        "create_shape_round",
        "Create Shape, Round: raises the selected closed vectors into a 3D dome that rises at \
         the angle you set and rounds over the top",
    ),
    (
        "create_shape_angled",
        "Create Shape, Angled: raises the selected closed vectors into a 3D shape with straight \
         slopes, like a roof or a pyramid",
    ),
    (
        "create_shape_smooth",
        "Create Shape, Smooth: raises the selected closed vectors into a 3D shape whose sides \
         curve gently up out of the edge and over the top",
    ),
    (
        "create_shape_flat",
        "Create Shape, Flat: raises the selected closed vectors into a flat-topped 3D shape, \
         all one height, like a plateau",
    ),
    (
        "edit_text",
        "Edit Text: opens the selected text in its form, to change its words, font or size",
    ),
];

/// What built-in action `id` does, if it is listed.
pub fn hint(id: &str) -> Option<&'static str> {
    HINTS.iter().find(|(i, _)| *i == id).map(|(_, h)| *h)
}
