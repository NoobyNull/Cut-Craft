//! Aspire Clone: an open-source take on Vectric Aspire for 2D/3D design and CNC toolpaths.

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
    app::run(concat!("Aspire Clone v", env!("CARGO_PKG_VERSION")));
}
