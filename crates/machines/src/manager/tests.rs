use super::*;

/// `rows` rows of the theme's interactive height, with their spacing.
fn ui_rows(ctx: &egui::Context, rows: f32) -> f32 {
    ctx.global_style().spacing.interact_size.y * rows * 1.5
}

fn frame(ctx: &egui::Context, draw: impl FnMut(&egui::Context)) {
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            // tall enough for the whole Machine Configuration, so a window sized to the
            // screen and not its contents shows
            egui::vec2(1200.0, 1400.0),
        )),
        ..Default::default()
    };
    // no renderer takes a headless frame's texture changes
    ctx.run_pass(input, draw).textures_delta.clear();
}

#[test]
fn install_offline_package_is_a_button_that_asks_for_a_package() {
    let ctx = egui::Context::default();
    let library = Library::new(None);
    let look = Look { picture: &|_| None };
    let mut manager = MachineManager::adding(&Machines::default());
    let label = "Install Offline Package: add the machines in a package file you downloaded";
    let mut at = None;
    let mut asked = vec![];
    for n in 0..8 {
        let events = match (n, at) {
            (4, Some(pos)) => vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
            (5, Some(pos)) => vec![egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            }],
            _ => vec![],
        };
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1200.0, 1000.0),
            )),
            events,
            ..Default::default()
        };
        let mut output = ctx.run_pass(input, |ctx| {
            asked.extend(manager.show(ctx, &library, &look));
        });
        output.textures_delta.clear();
        at = at.or_else(|| {
            output
                .shapes
                .iter()
                .find_map(|clipped| match &clipped.shape {
                    egui::Shape::Text(t) if t.galley.text() == label => {
                        Some(t.pos + t.galley.size() / 2.0)
                    }
                    _ => None,
                })
        });
    }
    assert!(at.is_some(), "the button is drawn");
    assert_eq!(asked, vec![MachineOutcome::InstallPackage]);
}

#[test]
fn the_windows_draw_with_and_without_machines() {
    let ctx = egui::Context::default();
    let library = Library::new(None);
    let look = Look { picture: &|_| None };
    let mut none = MachineManager::new(&Machines::default());
    let mut one = MachineManager::adding(&Machines::default());
    one.draft.machines[0].posts = vec![library.entries()[0].name.clone(), "Gone (*.nc)".into()];
    // a drill bank folds away to one line
    one.draft.machines[0].drill_bank =
        Some(cut_craft_core::drill_bank::DrillBank::new(5, 3, 32.0, 5.0));
    let mut posts = PostManager::default();
    for _ in 0..10 {
        frame(&ctx, |ctx| {
            assert_eq!(none.show(ctx, &library, &look), None);
            assert_eq!(one.show(ctx, &library, &look), None);
            assert_eq!(posts.show(ctx, &library, &look), None);
        });
    }
    assert_eq!(one.draft.machines[0].name, "My Machine");
    let window = ctx
        .memory(|m| m.area_rect(machine_window()))
        .expect("drawn");
    assert!(
        window.height() < ctx.content_rect().height() - 2.0 * window.top(),
        "fits its contents, not the screen: {window:?}"
    );
}

#[test]
fn a_posts_text_fills_the_window_below_its_buttons() {
    let ctx = egui::Context::default();
    let text = "G0 X0\n".repeat(400);
    let (mut shown, mut panel) = (egui::Rect::NOTHING, egui::Rect::NOTHING);
    for _ in 0..3 {
        frame(&ctx, |ctx| {
            egui::Window::new("fixed")
                .fixed_size([600.0, 800.0])
                .show(ctx, |ui| {
                    panel = ui.max_rect();
                    shown = text_view(ui, &mut text.as_str());
                });
        });
    }
    assert!(
        (panel.bottom() - shown.bottom()).abs() < 1.0,
        "reaches the bottom: {shown:?} in {panel:?}"
    );
}

#[test]
fn the_post_list_fills_the_window_down_to_its_buttons() {
    let ctx = egui::Context::default();
    let library = Library::new(None);
    let look = Look { picture: &|_| None };
    let mut posts = PostManager::default();
    let (mut list, mut panel) = (egui::Rect::NOTHING, egui::Rect::NOTHING);
    for _ in 0..3 {
        frame(&ctx, |ctx| {
            egui::Window::new("fixed")
                .fixed_size([600.0, 800.0])
                .show(ctx, |ui| {
                    panel = ui.max_rect();
                    (list, _) = posts.contents(ui, &library, &look);
                });
        });
    }
    let footer = ui_rows(&ctx, 3.0);
    assert!(
        list.bottom() > panel.bottom() - footer,
        "only the buttons below it: {list:?} in {panel:?}"
    );
}

#[test]
fn a_drill_banks_layout_draws_on_both_arms_and_in_inches() {
    let ctx = egui::Context::default();
    let mut bank = cut_craft_core::drill_bank::DrillBank::new(5, 3, 32.0, 5.0);
    for (per, length) in [(1.0, "mm"), (INCH, "in")] {
        frame(&ctx, |ctx| {
            egui::Window::new("bank").show(ctx, |ui| {
                bank_layout(ui, &mut bank, per, length);
            });
        });
    }
    assert_eq!(
        bank,
        cut_craft_core::drill_bank::DrillBank::new(5, 3, 32.0, 5.0)
    );
}

#[test]
fn changelog_shows_a_posts_history_under_its_name() {
    let entry = Library::new(None).entries()[0].clone();
    let (title, text) = change_log(&entry).unwrap();
    assert_eq!(title, format!("Post-Processor Change Log: {}", entry.name));
    assert!(text.contains("2026-10-31"), "{text}");
}

#[test]
fn a_custom_post_is_edited_and_saved_to_its_file() {
    let dir = std::env::temp_dir().join(format!("cut-craft-edit-{}", std::process::id()));
    let library = Library::new(Some(dir.clone()));
    let copy = library.customize(&library.entries()[0]).unwrap();
    let own = library.entries().into_iter().last().unwrap();
    assert!(
        Editing::of(&own).is_none(),
        "Cut Craft's own are customised, not edited"
    );
    let mut edit = Editing::of(&copy).expect("a custom post opens");
    edit.text.push_str("\n+ my change\n");
    edit.save().unwrap();
    assert!(Library::text(&copy).unwrap().ends_with("+ my change\n"));
    assert_eq!(edit.saved, edit.text);
    let mut editing = Some(edit);
    let ctx = egui::Context::default();
    for _ in 0..3 {
        frame(&ctx, |ctx| assert_eq!(editor(ctx, &mut editing), None));
    }
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_post_is_deleted_only_when_asked_twice() {
    let dir = std::env::temp_dir().join(format!("cut-craft-pm-{}", std::process::id()));
    let library = Library::new(Some(dir.clone()));
    let copy = library.customize(&library.entries()[0]).unwrap();
    let mut posts = PostManager {
        deleting: Some(copy.clone()),
        ..PostManager::default()
    };
    let ctx = egui::Context::default();
    frame(&ctx, |ctx| {
        ctx.root_ui(|ui| {
            egui::CentralPanel::default().show(ui, |ui| assert_eq!(posts.confirm_delete(ui), None));
        });
    });
    assert!(
        library.find(&copy.name).is_some(),
        "still there until Delete is pressed"
    );
    let _ = std::fs::remove_dir_all(dir);
}
