//! PowerWhittle: a free, source-available app for 2D/3D design and CNC toolpaths.

// release builds on Windows open no console window behind the app
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

/// A piece of the app's text known by its English words, in the language in use:
/// `t!("Clear with bigger bits first")`, or a `&str` constant's: `t!(INSIDE_GROUP)`. For text
/// that is only words; a sentence with a value put in uses a key of its own
/// (`tr!("status-exported", path = …)`). See [`words`].
macro_rules! t {
    ($english:literal) => {
        cut_craft_i18n::text($english)
    };
    ($constant:ident) => {
        cut_craft_i18n::text($constant)
    };
}

mod addons;
mod announcements;
mod app;
mod axes;
mod bars;
mod borders;
mod busy;
mod clipart;
mod commands;
mod compute;
mod credits;
mod custom_toolbars;
mod depth;
mod display;
#[cfg(test)]
mod error_messages;
mod file_filters;
mod files;
mod flyout;
mod form;
mod frame_rate;
mod gadgets;
mod gui;
mod hints;
mod icons;
mod languages;
mod marks;
mod messages;
mod options;
mod pages;
mod polyline;
mod preview_detail;
mod shortcut;
mod spacemouse;
mod startup;
mod tool_model;
mod tool_names;
mod toolpath_order;
mod trust;
mod update;
mod view2d;
mod view3d;
mod words;

fn main() {
    // the command line first: --version and --help print and stop, touching nothing (a closed
    // pipe or, on Windows, no console to print to is no reason to panic)
    let (shots, open) = match startup::parse(std::env::args().skip(1)) {
        startup::Start::Version => return startup::print(&format!("{}\n", startup::version())),
        startup::Start::Help => return startup::print(&startup::help()),
        startup::Start::Screenshots(dir) => (Some(app::Pictures::Readme(dir)), None),
        startup::Start::HelpPictures(dir, tools) => (Some(app::Pictures::Help(dir, tools)), None),
        startup::Start::App(open) => (None, open),
    };
    // the log and, on a panic, the crash log the next start offers to report
    cut_craft_bug_report::logs::install(files::config_dir().as_deref());
    // a window that can't open says why, rather than nothing happening
    startup::explain_failures();
    // what the last update set aside, now that the new version is the one running
    if let Some(program) = cut_craft_updater::this_install().program() {
        cut_craft_updater::clean_up(program);
    }
    app::run(
        concat!("PowerWhittle v", env!("CARGO_PKG_VERSION")),
        shots,
        open.as_deref(),
    );
}
