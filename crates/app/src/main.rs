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
mod pages;
mod shortcut;
mod view2d;
mod view3d;

fn main() {
    env_logger::init();
    app::run(concat!("Cut Craft v", env!("CARGO_PKG_VERSION")));
}
