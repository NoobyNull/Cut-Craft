//! Cut Craft: a free, source-available app for 2D/3D design and CNC toolpaths.

// release builds on Windows open no console window behind the app
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod addons;
mod app;
mod busy;
mod clipart;
mod commands;
mod compute;
mod depth;
mod display;
#[cfg(test)]
mod error_messages;
mod files;
mod flyout;
mod form;
mod frame_rate;
mod gadgets;
mod gui;
mod hints;
mod icons;
mod options;
mod pages;
mod polyline;
mod shortcut;
mod spacemouse;
mod startup;
mod toolpath_order;
mod update;
mod view2d;
mod view3d;

fn main() {
    // the command line first: --version and --help print and stop, touching nothing (a closed
    // pipe or, on Windows, no console to print to is no reason to panic)
    let shots = match startup::parse(std::env::args().skip(1)) {
        startup::Start::Version => return startup::print(&format!("{}\n", startup::version())),
        startup::Start::Help => return startup::print(&startup::help()),
        startup::Start::Screenshots(dir) => Some(dir),
        startup::Start::App => None,
    };
    // the log and, on a panic, the crash log the next start offers to report
    cut_craft_bug_report::logs::install(files::config_dir().as_deref());
    // a window that can't open says why, rather than nothing happening
    startup::explain_failures();
    // what the last update set aside, now that the new version is the one running
    if let Some(program) = cut_craft_updater::this_install().program() {
        cut_craft_updater::clean_up(program);
    }
    app::run(concat!("Cut Craft v", env!("CARGO_PKG_VERSION")), shots);
}
