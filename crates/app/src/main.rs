//! Cut Craft: an open-source, Vectric Aspire-style app for 2D/3D design and CNC toolpaths.

// release builds on Windows open no console window behind the app
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod app;
mod clipart;
mod commands;
mod display;
mod files;
mod form;
mod gui;
mod icons;
mod options;
mod pages;
mod shortcut;
mod view2d;
mod view3d;

fn main() {
    env_logger::init();
    // cut-craft --screenshots DIR: the README's pictures (see app/screenshots.rs)
    let mut args = std::env::args().skip(1);
    let shots = match args.next().as_deref() {
        Some("--screenshots") => Some(
            args.next()
                .map_or_else(|| "docs/screenshots".into(), Into::into),
        ),
        _ => None,
    };
    app::run(concat!("Cut Craft v", env!("CARGO_PKG_VERSION")), shots);
}
