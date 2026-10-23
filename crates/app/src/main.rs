//! Aspire Clone: an open-source take on Vectric Aspire for 2D/3D design and CNC toolpaths.

mod gui;
mod render;

fn main() {
    env_logger::init();
    render::run(concat!("Aspire Clone v", env!("CARGO_PKG_VERSION")));
}
