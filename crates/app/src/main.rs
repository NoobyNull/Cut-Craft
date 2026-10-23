//! Aspire Clone: an open-source take on Vectric Aspire for 2D/3D design and CNC toolpaths.

mod app;
mod commands;
mod form;
mod gui;
mod icons;
mod view2d;

fn main() {
    env_logger::init();
    app::run(concat!("Aspire Clone v", env!("CARGO_PKG_VERSION")));
}
