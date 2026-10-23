//! Cut Craft: an open-source, Vectric Aspire-style app for 2D/3D design and CNC toolpaths.

mod app;
mod commands;
mod files;
mod form;
mod gui;
mod icons;
mod shortcut;
mod view2d;
mod view3d;

fn main() {
    env_logger::init();
    app::run(concat!("Cut Craft v", env!("CARGO_PKG_VERSION")));
}
