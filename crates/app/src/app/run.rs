//! Starting Cut Craft: the window, the graphics and each frame handed to the app.

use super::{
    APP_ID, CutCraftApp, DIALOG_FOLDER, DIALOG_PARENT, app_icon, egui, files, help_pictures, mac,
    readable, screenshots, view3d,
};

impl eframe::App for CutCraftApp {
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        if let (Some(window), Ok(mut parent)) = (frame.winit_window(), DIALOG_PARENT.lock()) {
            *parent = Some(window.clone());
        }
        // dialog windows are kept above this one ([`cut_craft_window_owner`])
        let pass = ui.ctx().cumulative_pass_nr_for(egui::ViewportId::ROOT);
        cut_craft_window_owner::set_main_window(frame.winit_window(), pass);
        let job_folder = (self.options.general.file_dialog_default
            == crate::options::FileDialogDefault::Job)
            .then(|| self.session.path.as_deref()?.parent().map(Into::into))
            .flatten();
        if let Ok(mut folder) = DIALOG_FOLDER.lock() {
            *folder = job_folder;
        }
        self.pass(ui);
    }
}

/// The main window's frame: the desktop's title bar or Cut Craft's own (`system_title_bar`);
/// a Mac's window always keeps its traffic lights ([`mac::viewport`]).
pub(super) fn window_frame(
    builder: egui::ViewportBuilder,
    system_title_bar: bool,
) -> egui::ViewportBuilder {
    if egui::os::OperatingSystem::from_target_os() == egui::os::OperatingSystem::Mac {
        mac::viewport(builder, system_title_bar)
    } else {
        builder.with_decorations(system_title_bar)
    }
}

/// Pictures the app takes of itself, then quits.
pub enum Pictures {
    /// The README's ([`screenshots`]), into this folder.
    Readme(std::path::PathBuf),
    /// The help pages' step pictures ([`help_pictures`]), into this folder: every tour's, or
    /// only the tools named.
    Help(std::path::PathBuf, Vec<String>),
}

impl CutCraftApp {
    /// Takes `pictures` of itself from the next frame on, then quits.
    pub(super) fn take_pictures(&mut self, ctx: &egui::Context, pictures: Pictures) {
        match pictures {
            Pictures::Readme(dir) => self.shots = Some(screenshots::Shots::new(dir)),
            Pictures::Help(dir, only) => {
                self.help_shots = Some(help_pictures::HelpShots::new(ctx, dir, &only));
            }
        }
    }
}

/// Tells bug reports which graphics card draws Cut Craft, and how; `info` back.
pub(super) fn graphics_for_bug_reports(
    info: eframe::wgpu::AdapterInfo,
) -> eframe::wgpu::AdapterInfo {
    cut_craft_bug_report::system::set_graphics(
        &info.name,
        &format!("{:?}", info.backend),
        &info.driver,
        &info.driver_info,
    );
    info
}

/// The main window: its title, its icon (its pop-out windows take it too), its id for the
/// desktop, and its frame; with `shots`, the same size each time.
pub(super) fn main_viewport(
    shots: Option<&Pictures>,
    title: &str,
    frame: bool,
) -> egui::ViewportBuilder {
    window_frame(
        match shots {
            // the same size each time, whatever window size the user's runs keep
            Some(_) => egui::ViewportBuilder::default().with_inner_size(screenshots::WINDOW),
            None => egui::ViewportBuilder::default(),
        }
        .with_title(title)
        .with_app_id(APP_ID)
        .with_icon(app_icon::window_icon()),
        frame,
    )
}

/// Opens the main window and runs until it is closed; with `shots`, takes those pictures
/// instead and quits. `open` is a file named on the command line, opened once the window is up.
pub fn run(title: &str, shots: Option<Pictures>, open: Option<&std::path::Path>) {
    // winit 0.29 delivered dropped files only on X11; 0.30 should on Wayland too, but until that
    // is tried by hand a Wayland desktop runs Cut Craft through XWayland unless Options says to
    // use Wayland directly
    let settings = crate::options::Options::load();
    let (wayland, theme) = (settings.advanced.wayland_native, settings.appearance.theme);
    let frame = settings.appearance.system_title_bar;
    let text_scale = settings.appearance.text_scale();
    let x11 = !wayland && std::env::var_os("DISPLAY").is_some();
    let options = eframe::NativeOptions {
        event_loop_builder: Some(Box::new(move |builder| {
            // only Linux and the BSDs have X11 to choose
            #[cfg(all(unix, not(target_os = "macos")))]
            if x11 {
                use winit::platform::x11::EventLoopBuilderExtX11;
                builder.with_x11();
            }
            #[cfg(not(all(unix, not(target_os = "macos"))))]
            let _ = (builder, x11);
        })),
        viewport: main_viewport(shots.as_ref(), title, frame),
        persist_window: shots.is_none(),
        renderer: eframe::Renderer::Wgpu,
        multisampling: settings.view_3d.anti_aliasing.samples(),
        // the 3D view sorts its faces by depth
        depth_buffer: view3d::gpu::DEPTH_BITS,
        wgpu_options: eframe::egui_wgpu::WgpuConfiguration {
            wgpu_setup: eframe::egui_wgpu::WgpuSetup::CreateNew(
                eframe::egui_wgpu::WgpuSetupCreateNew {
                    device_descriptor: std::sync::Arc::new(view3d::gpu::device),
                    // a card that can run the 3D view before OpenGL or software (see `rank`)
                    native_adapter_selector: Some(std::sync::Arc::new(view3d::gpu::choose_adapter)),
                    ..eframe::egui_wgpu::WgpuSetupCreateNew::without_display_handle()
                },
            ),
            ..Default::default()
        },
        ..Default::default()
    };
    let samples = options.multisampling;
    // the pictures' run keeps its egui state apart from the user's
    let name = shots.as_ref().map_or(title, |_| "Cut Craft screenshots");
    eframe::run_native(
        name,
        options,
        Box::new(|cc| {
            cc.egui_ctx.set_theme(theme.preference());
            egui_extras::install_image_loaders(&cc.egui_ctx);
            readable(&cc.egui_ctx, text_scale);
            let mut app = CutCraftApp::new();
            app.title_bar_at_start = frame;
            if let Some(rs) = &cc.wgpu_render_state {
                let info = graphics_for_bug_reports(rs.adapter.get_info());
                log::info!(
                    "GPU: {} ({:?}, driver: {}), buffers up to {} MB",
                    info.name,
                    info.backend,
                    info.driver,
                    rs.device.limits().max_storage_buffer_binding_size / 1_000_000
                );
                let flags = rs.adapter.get_downlevel_capabilities().flags;
                if view3d::gpu::fits(flags, &rs.device.limits()) {
                    app.gpu_3d = Some(view3d::gpu::Gpu::new(rs, samples));
                    // the grid kernels (preview, 3D toolpaths) on the same card
                    cut_craft_core::compute::use_compute(Some(std::sync::Arc::new(
                        crate::compute::GpuCompute::new(rs.device.clone(), rs.queue.clone()),
                    )));
                } else {
                    // OpenGL on an older card: no compute shaders or storage buffers
                    log::warn!(
                        "GPU can't run the 3D view's shaders: drawing it without them, \
                         grid work on the processor"
                    );
                }
            }
            if let Some(pictures) = shots {
                // what a fresh clone shows: its icons, the starter tools, nothing of the user's
                app.take_pictures(&cc.egui_ctx, pictures);
                crate::startup::window_opened();
                return Ok(Box::new(app));
            }
            app.load_tools();
            app.load_materials();
            app.load_machines();
            app.load_options();
            app.load_words_and_job_times();
            app.load_addons();
            app.load_recent();
            app.clipart = crate::clipart::Clipart::load();
            app.start_recovery();
            app.open_at_start(open);
            // a crash last time: offer its crash log as a report
            if let Some(crash) =
                files::config_dir().and_then(|d| cut_craft_bug_report::logs::take_crash(&d))
            {
                app.offer_crash(&cc.egui_ctx, crash);
            }
            app.space_mouse = Some(crate::spacemouse::listen(cc.egui_ctx.clone()));
            app.check_for_updates_at_startup(&cc.egui_ctx);
            // only now: a panic setting up the app (above) still says why in a message box,
            // rather than the window never appearing and nothing said
            crate::startup::window_opened();
            Ok(Box::new(app))
        }),
    )
    .expect("event loop error");
}
