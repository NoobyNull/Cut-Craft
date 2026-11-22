//! Help: the Help Contents and pages, tips, tours, hints, tooltips and About.

use super::*;

#[test]
fn about_lists_the_github_contributors() {
    let listed = super::actions::contributors();
    assert!(
        !listed.is_empty(),
        "crates/app/contributors.tsv lists the contributors"
    );
    assert!(listed.iter().any(|(login, ..)| *login == "NoobyNull"));
    assert!(
        listed
            .iter()
            .all(|(_, commits, url)| commits.parse::<u32>().is_ok()
                && url.starts_with("https://github.com/")),
        "{listed:?}"
    );
    let mut h = Harness::new();
    h.app.info = Some(Info::About);
    h.frame(vec![]);
    h.frame(vec![]);
    assert!(h.text_at("Contributors on GitHub").is_some());
    assert!(h.text_at("NoobyNull").is_some());
    // a count takes its plural form only as a number
    assert_eq!(
        cut_craft_i18n::tr!("about-commits", count = 1_u64),
        "1 commit"
    );
    assert_eq!(
        cut_craft_i18n::tr!("about-commits", count = 2_u64),
        "2 commits"
    );
}

#[test]
fn about_thanks_vectric_and_says_cut_craft_is_independent() {
    let mut h = Harness::new();
    h.app.info = Some(Info::About);
    h.frame(vec![]);
    h.frame(vec![]);
    assert!(h.text_at("Acknowledgements").is_some());
    let thanks = super::actions::ACKNOWLEDGEMENT;
    assert!(thanks.contains("Vectric Aspire"), "{thanks}");
    assert!(thanks.contains("not affiliated with"), "{thanks}");
    assert!(thanks.contains("trademarks"), "{thanks}");
}

#[test]
fn about_states_cut_craft_s_own_licence() {
    let mut h = Harness::new();
    h.app.info = Some(Info::About);
    h.frame(vec![]);
    h.frame(vec![]);
    assert!(h.text_at("License").is_some());
    let licence = super::actions::LICENCE_SUMMARY;
    assert!(licence.contains("not open source"), "{licence}");
    assert!(licence.contains("noncommercial"), "{licence}");
    assert!(
        licence.contains("PolyForm Noncommercial License 1.0.0"),
        "{licence}"
    );
}

/// FNV-1a, so the test needs no hashing crate.
fn fnv1a(text: &str) -> u64 {
    text.bytes().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3)
    })
}

#[test]
fn licence_file_keeps_the_polyform_text_word_for_word() {
    let licence = include_str!("../../../../../LICENSE");
    let start = licence
        .find("# PolyForm Noncommercial License 1.0.0")
        .expect("the PolyForm text is in LICENSE");
    // a Windows checkout may turn line ends into CR LF
    let polyform = licence[start..].replace('\r', "");
    // the official text, from github.com/polyformproject/polyform-licenses at tag 1.0.0:
    // Cut Craft's own terms go above it, never into it
    assert_eq!(polyform.len(), 4_563, "the PolyForm text was changed");
    assert_eq!(
        fnv1a(&polyform),
        POLYFORM_FNV1A,
        "the PolyForm text was changed"
    );
}

const POLYFORM_FNV1A: u64 = 0xcf71_f592_48dd_cd09;

#[test]
fn licence_file_is_plain_polyform_noncommercial_with_the_credit_line() {
    let licence = include_str!("../../../../../LICENSE").replace('\r', "");
    let ours = &licence[..licence.find("# PolyForm").unwrap()];
    // nothing but the credit line goes above the PolyForm text: no extra terms
    assert_eq!(
        ours.trim(),
        "Required Notice: Copyright 2026 Matthew (NoobyNull) (https://github.com/NoobyNull/Cut-Craft)"
    );
}

#[test]
fn rail_and_ribbon_show_the_hovered_tools_guidance_in_a_tip_box() {
    use crate::options::WindowLayout;
    let mut h = Harness::new();
    h.app.options.general.layout = WindowLayout::Ribbon;
    h.frame(vec![]);
    h.frame(vec![]);
    // nothing hovered: the tool in hand and its next step, at the browser's foot
    assert_eq!(h.texts_at("Next:").len(), 1, "the Ribbon's tip box");
    let mut h = Harness::new();
    h.app.options.general.layout = WindowLayout::Rail;
    h.frame(vec![]);
    assert!(h.texts_at("Next:").is_empty(), "no list open, no tip box");
    h.app.rail_group = h
        .app
        .all_groups()
        .iter()
        .position(|g| g.name == "Create Vectors");
    h.frame(vec![]);
    let row = h.text_at("Draw Rectangle").expect("a tool row");
    h.frame(vec![Event::PointerMoved(row)]);
    h.frame(vec![]);
    assert_eq!(h.texts_at("Next:").len(), 1, "under the Rail's list");
}

#[test]
fn help_items_open_their_windows() {
    let mut h = Harness::new();
    for (id, info) in [
        ("about_cut_craft", Info::About),
        ("keyboard_shortcuts", Info::Shortcuts),
        ("third_party_licenses", Info::Licenses),
        ("toolpath_summary", Info::Summary),
    ] {
        h.app.invoke(&h.ctx, id);
        assert_eq!(h.app.info, Some(info));
    }
}

#[test]
fn the_3d_pencil_help_shows_what_it_cuts_with_pictures() {
    let h = Harness::new();
    let page = h.app.help_markdown("3d_pencil_toolpath");
    assert!(page.contains("pictures/pencil-before-after.png"));
    assert!(page.contains("pictures/pencil-pass.gif"));
    assert!(
        page.contains("help:3d_finishing_toolpath"),
        "linked to 3D Finishing"
    );
    assert!(
        page.contains("**Extra passes each side**"),
        "its settings listed"
    );
}

#[test]
fn a_tools_help_page_shows_its_toolbar_icon_by_its_name() {
    let h = Harness::new();
    let page = h.app.help_markdown("3d_pencil_toolpath");
    assert!(
        page.contains(
            "# ![3D Pencil Toolpath icon](bytes://help-icons/48/toolpaths/3d_pencil_toolpath.png) \
             3D Pencil Toolpath\n"
        ),
        "the same picture as its button, beside its name: {page}"
    );
}

#[test]
fn the_toolpaths_help_lists_each_tool_by_its_icon() {
    let h = Harness::new();
    let page = h.app.help_markdown("toolpaths");
    assert!(
        page.contains(
            "- ![](bytes://help-icons/24/toolpaths/3d_pencil_toolpath.png) \
             [3D Pencil Toolpath](help:3d_pencil_toolpath)\n"
        ),
        "{page}"
    );
    let design = h.app.help_markdown("design");
    assert!(
        design.contains("- ![](bytes://help-icons/24/drawing/create_vectors/draw_circle.png) "),
        "{design}"
    );
}

#[test]
fn every_tools_help_page_shows_its_buttons_icon() {
    let h = Harness::new();
    let missing: Vec<&str> = h
        .app
        .help_topics()
        .into_iter()
        .filter(|t| !help::OWN_PAGES.contains(t))
        .filter(|t| match h.app.help_icon_uri(t, help::PAGE_ICON) {
            Some(uri) => !h.app.help_markdown(t).contains(&uri),
            None => true,
        })
        .collect();
    assert!(missing.is_empty(), "help with no icon: {missing:?}");
}

#[test]
fn help_search_finds_tools_by_name_not_by_their_icons_file() {
    let mut h = Harness::new();
    let found = h.app.help_search_results("pencil");
    assert_eq!(found[0].0, "3d_pencil_toolpath", "{found:?}");
    // the search reads each page's words, never an icon's file path or folder
    for (t, _, md) in &h.app.help_index {
        assert!(!md.contains("help-icons"), "{t}");
        assert!(!md.contains("create_vectors"), "{t}");
    }
}

#[test]
fn the_help_window_shows_tool_icons_on_pages_and_in_search_results() {
    let mut h = Harness::new();
    h.app.info = Some(Info::Help("3d_pencil_toolpath"));
    h.frame(vec![]);
    h.frame(vec![]);
    let big = "bytes://help-icons/48/toolpaths/3d_pencil_toolpath.png";
    let image = h.ctx.try_load_bytes(big);
    assert!(
        matches!(image, Ok(egui::load::BytesPoll::Ready { .. })),
        "the page's icon handed to egui"
    );
    h.app.help_search = "pencil".into();
    h.frame(vec![]);
    h.frame(vec![]);
    let small = "bytes://help-icons/24/toolpaths/3d_pencil_toolpath.png";
    assert!(
        matches!(
            h.ctx.try_load_bytes(small),
            Ok(egui::load::BytesPoll::Ready { .. })
        ),
        "each result beside its icon"
    );
    assert!(h.text_at("3D Pencil Toolpath").is_some());
}

#[test]
fn the_start_page_says_whats_new_and_links_out() {
    let h = Harness::at_start();
    let heading = format!("What's new in v{}", env!("CARGO_PKG_VERSION"));
    assert!(h.text_at(&heading).is_some(), "{heading}");
    let first = WHATS_NEW.lines().find(|l| !l.trim().is_empty()).unwrap();
    assert!(
        h.text_at(first.trim_start_matches("- ")).is_some(),
        "{first}"
    );
    assert!(h.text_at("•").is_some(), "each change is bulleted");
    // the links themselves are in Help ▸ Community (see
    // the_community_links_are_in_help_not_on_the_start_page)
    for (_, url, _) in LINKS {
        assert!(url.starts_with("https://"), "{url}");
    }
}

#[test]
fn the_wheel_zooms_about_the_cursor() {
    let mut h = Harness::new();
    let at = view_middle(&h) + vec2(40.0, -30.0);
    let (before, under) = {
        let t = h.app.view_transform.unwrap();
        (t.scale, t.to_job(at))
    };
    h.frame(vec![Event::PointerMoved(at)]);
    h.frame(vec![Event::MouseWheel {
        unit: egui::MouseWheelUnit::Point,
        delta: vec2(0.0, 120.0),
        phase: egui::TouchPhase::Move,
        source: egui::MouseWheelSource::Wheel,
        modifiers: Modifiers::NONE,
    }]);
    h.frame(vec![]);
    let t = h.app.view_transform.unwrap();
    assert!(t.scale > before, "scrolling up zooms in");
    assert!(t.to_job(at).distance(under) < 0.01, "about the cursor");
}

#[test]
fn every_tool_on_the_pages_can_be_asked_its_first_step() {
    let h = Harness::new();
    let mut with_step = 0;
    for group in h
        .app
        .toolbar
        .all_design_groups()
        .iter()
        .chain(&h.app.toolbar.toolpath_groups)
    {
        for id in group.tools {
            if h.app.first_step_of(id).is_some() {
                with_step += 1;
            }
        }
    }
    assert!(with_step >= 40, "{with_step} tools say what to select");
    for id in ["weld_vectors", "two_rail_sweep", "profile_toolpath", "move"] {
        assert!(h.app.first_step_of(id).is_some(), "{id}");
    }
    for id in ["draw_rectangle", "job_setup", "zoom_to_job", "undo"] {
        assert_eq!(h.app.first_step_of(id), None, "{id}");
    }
}

#[test]
fn the_design_page_shows_the_hovered_tools_guidance_under_its_icons() {
    let mut h = Harness::new();
    // tall enough for the buttons at their full size and what comes under them
    h.window = vec2(1280.0, 1600.0);
    h.app.pages.set_pinned(Page::Design, true);
    h.frame(vec![]);
    h.frame(vec![]);
    // nothing hovered: the tool in hand, Select, and its next step
    assert!(
        h.text_at("Select").is_some(),
        "the tip box names the tool in hand"
    );
    assert!(h.text_at("Next:").is_some());
    assert!(
        h.texts_at("Next:").len() == 1,
        "one box: {:?}",
        h.texts_at("Next:")
    );
    // hovering Draw Circle's button: its description, its next step, and the status bar's line
    h.app.options.general.fold_tool_groups = true;
    h.frame(vec![]);
    flyout_arrow_naming(&mut h, "Draw Circle");
    let hovered = guide::hovered(&h.ctx).expect("a hovered tool");
    assert_eq!(hovered.label, "Draw Circle");
    assert!(
        hovered
            .next
            .as_deref()
            .unwrap()
            .starts_with("Drag from the centre"),
        "{hovered:?}"
    );
    assert!(h.text_at("Draw Circle").is_some(), "the box shows its name");
    let hint = cut_craft_bug_report::hint::current(&h.ctx).unwrap_or_default();
    assert!(
        hint.starts_with("Draw Circle: Drag from the centre"),
        "{hint}"
    );
}

#[test]
fn the_rails_tool_rows_and_a_flyouts_list_carry_the_guidance_too() {
    let mut h = Harness::new();
    h.app.options.general.layout = crate::options::WindowLayout::Rail;
    h.frame(vec![]);
    let groups = h.app.all_groups();
    h.app.rail_group = groups.iter().position(|g| g.name == "Create Vectors");
    h.frame(vec![]);
    // the Rail's list of Create Vectors: its rows name each tool
    let row = h.text_at("Draw Rectangle").expect("a tool row");
    h.frame(vec![Event::PointerMoved(row)]);
    h.frame(vec![]);
    let hovered = guide::hovered(&h.ctx).expect("the row's tip");
    assert_eq!(hovered.label, "Draw Rectangle");
    assert!(
        hovered
            .next
            .as_deref()
            .unwrap()
            .starts_with("Drag from one corner"),
        "{hovered:?}"
    );
    let hint = cut_craft_bug_report::hint::current(&h.ctx).unwrap_or_default();
    assert!(
        hint.starts_with("Draw Rectangle: Drag from one corner"),
        "{hint}"
    );
}

#[test]
fn the_3d_views_help_shows_auto_rotate_with_its_moving_picture() {
    let h = Harness::new();
    assert!(h.app.all_help_topics().contains(&"view_3d"));
    assert!(
        h.app.help_markdown("contents").contains("(help:view_3d)"),
        "linked from Contents"
    );
    let page = h.app.help_markdown("view_3d");
    assert!(page.contains("Auto-rotate") && page.contains("Speed"));
    assert!(page.contains("pictures/auto-rotate.gif"));
    assert!(
        cut_craft_help::PICTURES
            .iter()
            .any(|(name, bytes)| *name == "auto-rotate.gif" && bytes.starts_with(b"GIF8")),
        "its picture goes with the app"
    );
}

#[test]
fn the_3d_views_help_says_how_to_bring_up_a_models_height_buttons() {
    let h = Harness::new();
    let page = h.app.help_markdown("view_3d");
    for words in [
        "Changing a model's height",
        "**click** the model",
        "blue line",
        "Shift",
        "Transform",
        "Total Height",
        "Shape Height",
        "Base Height",
        "Esc",
        "The 2D view never shows these buttons",
        "Component Properties",
    ] {
        assert!(page.contains(words), "{words}");
    }
    for (picture, magic) in [
        ("model-height-buttons.gif", &b"GIF8"[..]),
        ("model-height-buttons.png", b"\x89PNG"),
    ] {
        assert!(page.contains(&format!("pictures/{picture}")), "{picture}");
        assert!(
            cut_craft_help::PICTURES
                .iter()
                .any(|(name, bytes)| *name == picture && bytes.starts_with(magic)),
            "{picture} goes with the app"
        );
    }
    assert!(
        h.app.help_markdown("contents").contains("height buttons"),
        "Contents names it"
    );
    for tool in ["transform", "scale_model_z_height"] {
        assert!(
            h.app.help_markdown(tool).contains("(help:view_3d)"),
            "{tool} links it"
        );
    }
}

#[test]
fn the_progress_help_explains_the_time_left_with_a_picture() {
    let h = Harness::new();
    assert!(h.app.all_help_topics().contains(&"progress"));
    assert!(
        h.app.help_markdown("contents").contains("(help:progress)"),
        "linked from Contents"
    );
    assert!(h.app.help_markdown("web_api").contains("(help:progress)"));
    assert!(h.app.help_markdown("toolpaths").contains("(help:progress)"));
    let page = h.app.help_markdown("progress");
    for words in [
        "pictures/progress-window.png",
        "working out how long it will take",
        "about 2 minutes",
        "almost done",
        "usually takes",
        "taking longer than usual",
        "Cancel",
        "job-times.json",
    ] {
        assert!(page.contains(words), "{words}");
    }
    // and the words it explains are the ones the window uses
    for estimate in [
        cut_craft_core::eta::Estimate::Estimating,
        cut_craft_core::eta::Estimate::AlmostDone,
        cut_craft_core::eta::Estimate::LongerThanUsual,
    ] {
        let said = estimate.words().unwrap();
        let said = said.trim_end_matches('…');
        assert!(page.contains(said), "{said}");
    }
}

#[test]
fn the_settings_help_says_changes_wait_for_apply_with_pictures() {
    let h = Harness::new();
    assert!(h.app.all_help_topics().contains(&"settings"));
    assert!(
        h.app.help_markdown("contents").contains("(help:settings)"),
        "linked from Contents"
    );
    assert!(
        h.app.help_markdown("web_api").contains("(help:settings)"),
        "and from the Web API's steps"
    );
    let page = h.app.help_markdown("settings");
    for words in [
        "Apply",
        "Cancel",
        "Close",
        super::options_apply::NOT_APPLIED,
        super::options_apply::ALL_APPLIED,
        super::options_apply::ASK,
        "Apply and Close",
        "Discard Changes",
        "Keep Editing",
        "Reset to defaults",
    ] {
        assert!(page.contains(words), "{words}");
    }
    for words in [
        "Snapping Options",
        "Machine Configuration",
        "Tool Database",
        "Materials Library",
    ] {
        assert!(page.contains(words), "{words}");
    }
    for picture in [
        "options-not-applied.png",
        "options-apply-question.png",
        "snapping-not-applied.png",
    ] {
        assert!(page.contains(&format!("pictures/{picture}")), "{picture}");
        assert!(
            cut_craft_help::PICTURES
                .iter()
                .any(|(name, bytes)| *name == picture && bytes.starts_with(b"\x89PNG")),
            "{picture} goes with the app"
        );
    }
}

#[test]
fn the_updates_help_shows_one_click_install_and_relaunch_with_pictures() {
    let h = Harness::new();
    assert!(h.app.all_help_topics().contains(&"updates"));
    assert!(
        h.app.help_markdown("contents").contains("(help:updates)"),
        "linked from Contents"
    );
    let page = h.app.help_markdown("updates");
    for words in [
        "Download, install and relaunch?",
        "Install Now",
        "Later",
        "Skip This Version",
        "Relaunch Now",
        "save",
        "Copy",
    ] {
        assert!(page.contains(words), "{words}");
    }
    for (picture, magic) in [
        ("update-offer.png", &b"\x89PNG"[..]),
        ("update-steps.gif", b"GIF8"),
        ("update-by-hand.png", b"\x89PNG"),
    ] {
        assert!(page.contains(&format!("pictures/{picture}")), "{picture}");
        assert!(
            cut_craft_help::PICTURES
                .iter()
                .any(|(name, bytes)| *name == picture && bytes.starts_with(magic)),
            "{picture} goes with the app"
        );
    }
}

#[test]
fn help_contents_links_to_the_pages_and_their_tools() {
    let mut h = Harness::new();
    // the tools are listed by their icons, drawn at their own size, as the app does
    egui_extras::install_image_loaders(&h.ctx);
    h.app.pages.set_pinned(Page::Design, true);
    h.app.invoke(&h.ctx, "help_contents");
    assert_eq!(h.app.info, Some(Info::Help("contents")));
    // a new window measures itself on its first frame, and its scroll bar on the next
    h.frame(vec![]);
    h.frame(vec![]);
    h.frame(vec![]);
    // the icons decode on a background thread: one that lands after the link is found moves
    // the page under the click (Windows runners were slow enough), so wait for them all
    for _ in 0..500 {
        if !h.ctx.has_pending_images() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
        h.frame(vec![]);
    }
    assert!(
        !h.ctx.has_pending_images(),
        "the icons never finished loading"
    );
    h.frame(vec![]);
    // the window's link, drawn over the Design page's own title and side tab
    let at = *h
        .texts_at("Design")
        .last()
        .expect("Contents links the Design page");
    h.click(at, Modifiers::NONE);
    h.frame(vec![]);
    assert_eq!(h.app.info, Some(Info::Help("design")));
    // listed by their icons: wheeled down to the first group's last tools
    h.frame(vec![
        Event::PointerMoved(Pos2::new(200.0, 400.0)),
        Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: vec2(0.0, -200.0),
            phase: egui::TouchPhase::Move,
            source: egui::MouseWheelSource::Wheel,
            modifiers: Modifiers::NONE,
        },
    ]);
    // the wheel scrolls smoothly over a few frames
    for _ in 0..5 {
        h.frame(vec![]);
    }
    // beside its icon: the words themselves, not the space the icon leaves on their line
    let at = *h
        .words_at("Setup Material")
        .last()
        .expect("its tools are listed");
    h.click(at, Modifiers::NONE);
    h.frame(vec![]);
    assert_eq!(h.app.info, Some(Info::Help("setup_material")));
    let vars = h.app.help_markdown("document_variables");
    assert!(
        vars.contains("{depth}"),
        "a tool's help is its doc comments"
    );
    assert!(vars.contains("**Variables**"), "and its settings'");
}

#[test]
fn every_help_link_goes_to_a_help_page() {
    let h = Harness::new();
    let topics = h.app.all_help_topics();
    let problems = cut_craft_help::problems(|id| topics.contains(&id));
    assert!(problems.is_empty(), "{problems:#?}");
}

#[test]
fn every_tool_on_the_pages_has_a_help_page() {
    let h = Harness::new();
    let missing: Vec<&str> = h
        .app
        .help_topics()
        .into_iter()
        .filter(|id| cut_craft_help::page(id).is_none() && *id != "tips")
        .collect();
    assert!(
        missing.is_empty(),
        "add crates/help/pages/tools/<id>.md (a header is enough to start) for:\n{}",
        missing.join("\n")
    );
}

#[test]
fn every_command_has_a_help_page_to_update() {
    // scripts/help-touched asks a pull request changing a command to change this page
    let missing: Vec<&str> = crate::gui::command_ids()
        .filter(|id| cut_craft_help::page(id).is_none())
        .collect();
    assert!(
        missing.is_empty(),
        "add crates/help/pages/tools/<id>.md (a header is enough to start) for {missing:?}"
    );
}

#[test]
fn a_tools_help_page_shows_its_steps_settings_and_related_pages() {
    let h = Harness::new();
    let pocket = h.app.help_markdown("pocket_toolpath");
    assert!(pocket.contains("## Settings"), "{pocket}");
    // see-also on its own page, beside it
    assert!(side_bar(&h, "pocket_toolpath").contains(&"profile_toolpath".into()));
    // and back the other way: Profile's page never names Draw Circle, Draw Circle's names it
    assert!(side_bar(&h, "profile_toolpath").contains(&"draw_circle".into()));
}

#[test]
fn every_help_page_has_related_pages_beside_it() {
    // help-coverage: a page with nothing related is a dead end; name pages in its header's
    // see-also (crates/help/pages/...) or link them in its words
    let h = Harness::new();
    let lonely: Vec<&str> = h
        .app
        .all_help_topics()
        .into_iter()
        .filter(|t| !matches!(*t, "contents" | "design" | "toolpaths" | "tips"))
        .filter(|t| {
            let bar = h.app.help_sidebar(t, &h.app.help_page(t));
            // see-also naming tools of its own group lists them under Other tools like this
            let own = cut_craft_help::page(t).map_or_default(|p| cut_craft_help::links(&p));
            bar.related.is_empty() && own.iter().all(|l| *l == "contents")
        })
        .collect();
    assert!(
        lonely.is_empty(),
        "add see-also: to these pages' headers:\n{}",
        lonely.join("\n")
    );
}

#[test]
fn back_and_forward_walk_the_help_pages_read() {
    let mut h = Harness::new();
    h.app.invoke(&h.ctx, "help_contents");
    h.frame(vec![]);
    h.frame(vec![]);
    assert_eq!(h.app.info, Some(Info::Help("contents")));
    let at = *h.texts_at("Did You Know?").last().expect("the tips button");
    h.click(at, Modifiers::NONE);
    h.frame(vec![]);
    assert_eq!(h.app.info, Some(Info::Help("tips")));
    let at = h.text_at("◀ Back").expect("Back");
    h.click(at, Modifiers::NONE);
    h.frame(vec![]);
    assert_eq!(h.app.info, Some(Info::Help("contents")), "Back to Contents");
    let at = h.text_at("Forward ▶").expect("Forward");
    h.click(at, Modifiers::NONE);
    h.frame(vec![]);
    assert_eq!(
        h.app.info,
        Some(Info::Help("tips")),
        "Forward to the tips again"
    );
}

#[test]
fn a_question_finds_the_help_that_answers_it() {
    let mut h = Harness::new();
    let found: Vec<&str> = h
        .app
        .help_search_results("how do I hold the part with bridges?")
        .into_iter()
        .map(|(t, ..)| t)
        .take(5)
        .collect();
    assert!(found.contains(&"profile_toolpath"), "{found:?}");
}

#[test]
fn a_tools_help_page_falls_back_to_its_own_description() {
    let h = Harness::new();
    // Profile's params carry no doc comment of their own; its crate says what it does
    let profile = h.app.help_markdown("profile_toolpath");
    assert!(!profile.contains("No help written"), "{profile}");
    assert!(
        profile.contains("Cuts along the selected vectors, outside, inside or on the line"),
        "{profile}"
    );
    assert!(
        !profile.contains("Profile Toolpath: cuts"),
        "the heading names it already: {profile}"
    );
}

#[test]
fn a_pages_question_mark_opens_its_help() {
    let mut h = Harness::new();
    h.app.pages.set_pinned(Page::Design, true);
    h.frame(vec![]);
    let at = h.text_at("?").expect("the page's ? button");
    h.click(at, Modifiers::NONE);
    assert_eq!(h.app.info, Some(Info::Help("design")));
}

#[test]
fn a_page_whose_contents_ask_for_more_room_keeps_its_place() {
    // a field asking for a little more than the page has, as a long form in a big font does
    let ctx = egui::Context::default();
    let window = Rect::from_min_size(Pos2::ZERO, vec2(1280.0, 800.0));
    for frame in 0..20 {
        let input = RawInput {
            screen_rect: Some(window),
            time: Some(f64::from(frame) * 0.1),
            ..Default::default()
        };
        let mut rects = vec![];
        headless(ctx.run_ui(input, |ui| {
            for panel in [egui::Panel::left("l"), egui::Panel::right("r")] {
                rects.push(show_page(panel, 300.0, ui, |ui| {
                    ui.set_min_width(ui.available_width() + 40.0);
                }));
            }
            rects.push(ui.available_rect_before_wrap());
        }));
        assert_eq!(
            rects[0].x_range(),
            egui::Rangef::new(0.0, 300.0),
            "left page"
        );
        assert_eq!(
            rects[1].x_range(),
            egui::Rangef::new(980.0, 1280.0),
            "right page"
        );
        assert_eq!(
            rects[2].x_range(),
            egui::Rangef::new(300.0, 980.0),
            "the view between"
        );
    }
}

#[test]
fn orientate_3d_models_rings_turn_the_model_about_their_axis_and_the_sphere_about_all() {
    let mut h = Harness::new();
    let file = pyramid_stl("rings");
    h.app.import_file(&file);
    h.app.view = View::ThreeD;
    h.frame(vec![]);
    h.frame(vec![]);
    let (rings, sphere) = h.app.rings_3d.clone();
    let axes: Vec<&str> = rings.iter().map(|(a, _)| *a).collect();
    assert_eq!(axes, ["x", "y", "z"], "red, green and blue rings");
    let (centre, _) = sphere.expect("the grey sphere");
    let turn = |h: &Harness| h.app.form.as_ref().unwrap().params["turn"].clone();
    assert_eq!(
        turn(&h)["mode"],
        "view",
        "drags turn the view unless a handle is taken"
    );
    // a quarter of the way round the red ring, dragged up: a turn about X only
    let on_x = rings[0].1[rings[0].1.len() / 4];
    h.drag(on_x, on_x - vec2(0.0, 60.0));
    let t = turn(&h);
    assert!(t["x"].as_f64().unwrap().abs() > 1.0, "{t}");
    assert_eq!((t["y"].as_f64(), t["z"].as_f64()), (Some(0.0), Some(0.0)));
    assert!(h.app.turn_ring.is_none(), "let go");
    // the sphere, dragged across and up: about all three (X and Z here)
    let before = turn(&h);
    h.drag(centre, centre + vec2(60.0, -60.0));
    let t = turn(&h);
    assert!(t["z"].as_f64() != before["z"].as_f64() && t["x"].as_f64() != before["x"].as_f64());
    // anywhere else turns the view, not the model
    let orbit = h.app.orbit;
    let corner = h.app.fit_3d.unwrap().left_bottom() + vec2(10.0, -10.0);
    let before = turn(&h);
    h.drag(corner, corner + vec2(60.0, 0.0));
    assert_eq!(turn(&h), before);
    assert_ne!(h.app.orbit, orbit, "the view turned");
    let _ = std::fs::remove_file(file);
}

#[test]
fn an_unsigned_gadget_warns_once_before_it_runs() {
    let mut h = Harness::new();
    h.app.unsigned_gadgets = [("gadget_hole_pattern".to_owned(), "Hole Pattern".to_owned())].into();
    // the user says no: nothing opens, and they are asked again next time
    h.app.unsigned_gadget_ok = |_| false;
    h.app.invoke(&h.ctx, "gadget_hole_pattern");
    assert!(h.app.form.is_none(), "cancelled: no form");
    // the user says yes: the form opens, and the gadget is not asked about again this session
    h.app.unsigned_gadget_ok = |label| label == "Hole Pattern";
    h.app.invoke(&h.ctx, "gadget_hole_pattern");
    assert!(h.app.form.is_some());
    h.app.form = None;
    h.app.unsigned_gadget_ok = |_| panic!("asked twice");
    h.app.invoke(&h.ctx, "gadget_hole_pattern");
    assert!(h.app.form.is_some());
}

#[test]
fn the_unsigned_warning_says_who_is_responsible() {
    let text = super::actions::unsigned_warning("Star Burst");
    assert!(text.contains("Star Burst"));
    assert!(text.contains("not responsible"));
    assert!(text.contains("Undo"));
    assert!(text.contains("not an official Cut Craft gadget"), "{text}");
}

#[test]
fn old_tile_files_are_asked_about_on_saving_and_on_sending_with_tiling_turned_off() {
    use crate::options::EarlierGcode;
    let mut h = Harness::new();
    two_tools(&mut h);
    let dir = std::env::temp_dir().join(format!("cut-craft-old-tiles-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("card").join("sign")).unwrap();
    h.app.options.confirmations.earlier_gcode = EarlierGcode::Ask;
    let form = SaveForm {
        mode: crate::files::SaveMode::VisibleToOneFile,
        group: false,
        post: None,
        add_side: false,
        changes: crate::files::ToolChanges::ToolChanger,
        job_sheet: false,
    };
    let post = h.app.form_post(&form).unwrap();
    let ext = post.extension.clone();
    for n in 1..=2 {
        let tile = format!("sign_tile{n}.{ext}");
        std::fs::write(dir.join(&tile), "old").unwrap();
        std::fs::write(dir.join("card").join("sign").join(&tile), "old").unwrap();
    }
    // saving sign.nc where an earlier tiled save left its tiles asks Replace or Archive
    let path = dir.join(format!("sign.{ext}"));
    h.app.save_over_earlier(&form, post.clone(), path);
    let ask = h.app.earlier_ask.take().expect("asked");
    assert!(!ask.sending);
    assert_eq!(ask.earlier.len(), 2, "{:?}", ask.earlier);
    // and so does sending, the answer then sending on
    h.app.machines.add();
    h.app.machines.active_mut().unwrap().send_to.folder = dir.join("card").display().to_string();
    h.app.session.path = Some(std::path::PathBuf::from("/jobs/sign.ccjob"));
    h.app.send_choosing(&form, post, EarlierGcode::Ask);
    let ask = h.app.earlier_ask.as_ref().expect("asked");
    assert!(ask.sending);
    assert_eq!(ask.earlier.len(), 2);
    h.app.answer_earlier(Some(EarlierGcode::Replace), false);
    let mut left: Vec<String> = std::fs::read_dir(dir.join("card").join("sign"))
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    left.sort();
    assert_eq!(left, [format!("sign.{ext}")]);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn the_status_bar_keeps_its_hint_while_the_pointer_is_on_it() {
    let mut h = Harness::new();
    h.app.status.set_message("Ready");
    h.frame(vec![]);
    let hint = "0331. Flowers.STL\nDouble-click to add it to the project";
    cut_craft_bug_report::hint::hint(&h.ctx, hint);
    h.frame(vec![]);
    let (at, _) = status_text(&h, "Flowers.STL").expect("the hint is shown");
    // moving onto the bar to read it: nothing sets a hint now, yet it stays
    h.frame(vec![Event::PointerMoved(at)]);
    h.frame(vec![]);
    h.frame(vec![]);
    assert!(
        status_text(&h, "Flowers.STL").is_some(),
        "the hint stays while the pointer is on the bar"
    );
    // and goes once the pointer leaves
    h.frame(vec![Event::PointerMoved(egui::pos2(640.0, 300.0))]);
    h.frame(vec![]);
    assert!(status_text(&h, "Flowers.STL").is_none(), "gone after");
}

#[test]
fn the_community_links_are_in_help_not_on_the_start_page() {
    let h = Harness::at_start();
    assert!(h.text_at("Facebook").is_none());
    assert!(h.text_at("Cut Craft on GitHub").is_some());
    let help = h
        .app
        .current_menus()
        .menus
        .iter()
        .find(|m| m.name == "Help")
        .unwrap()
        .items
        .clone();
    let community = leaf(&help, &["Community"]);
    assert_eq!(community.children.len(), super::LINKS.len());
    assert!(
        community
            .children
            .iter()
            .all(|i| h.app.available(&i.action))
    );
}

fn help_menu(app: &CutCraftApp) -> Vec<crate::gui::MenuItem> {
    app.current_menus()
        .menus
        .iter()
        .find(|m| m.name == "Help")
        .unwrap()
        .items
        .clone()
}

#[test]
fn help_contents_opens_from_the_help_menu_with_or_without_a_job() {
    for start in [Harness::at_start as fn() -> Harness, Harness::new] {
        let mut h = start();
        for item in help_menu(&h.app)
            .iter()
            .filter(|i| !i.action.is_empty() && i.children.is_empty())
        {
            assert!(h.app.available(&item.action), "{} is greyed", item.name);
        }
        h.choose(help_menu, &["Help Contents"]);
        assert_eq!(h.app.info, Some(Info::Help("contents")));
    }
}

#[test]
fn help_search_lists_matching_pages_and_opens_one() {
    let mut h = Harness::new();
    let found = h.app.help_search_results("job setup");
    assert_eq!(
        found[0].0, "job_setup",
        "a name that matches first: {found:?}"
    );
    let tabs: Vec<&str> = h
        .app
        .help_search_results("tabs")
        .into_iter()
        .map(|(t, ..)| t)
        .collect();
    assert_eq!(tabs.first(), Some(&"tabs"), "its own page first: {tabs:?}");
    assert!(
        tabs.contains(&"profile_toolpath"),
        "then the pages using it: {tabs:?}"
    );
    assert!(
        h.app
            .help_search_results("did you know")
            .iter()
            .any(|(t, ..)| *t == "tips"),
        "the tips are a page of their own"
    );
    h.app.invoke(&h.ctx, "help_contents");
    h.app.help_search = "job setup".into();
    h.frame(vec![]);
    h.frame(vec![]);
    // the first result, just under the count
    let count = h
        .text_at(&format!("{} pages found, best first", found.len()))
        .expect("the results counted");
    let at = *h
        .texts_at("Job Setup")
        .iter()
        .filter(|p| p.y > count.y)
        .min_by(|a, b| a.y.total_cmp(&b.y))
        .expect("listed");
    h.click(at, Modifiers::NONE);
    h.frame(vec![]);
    assert_eq!(h.app.info, Some(Info::Help("job_setup")));
    assert!(
        h.app.help_search.is_empty(),
        "the page shown in place of the list"
    );
}

#[test]
fn did_you_know_steps_through_the_tips() {
    let mut h = Harness::at_start();
    assert_eq!(h.app.tip, None, "tests start without it");
    h.choose(help_menu, &["Did You Know?..."]);
    assert_eq!(h.app.tip, Some(0));
    h.frame(vec![]);
    h.frame(vec![]);
    assert!(
        h.text_at(&format!("Tip 1 of {}", super::tips::TIPS.len()))
            .is_some()
    );
    let next = h.text_at("Next ▶").unwrap();
    h.click(next, Modifiers::NONE);
    h.frame(vec![]);
    assert_eq!(h.app.tip, Some(1));
    let previous = h.text_at("◀ Previous").unwrap();
    h.click(previous, Modifiers::NONE);
    h.frame(vec![]);
    assert_eq!(h.app.tip, Some(0));
    // the tip's length moves the buttons
    let previous = h.text_at("◀ Previous").unwrap();
    h.click(previous, Modifiers::NONE);
    h.frame(vec![]);
    assert_eq!(
        h.app.tip,
        Some(super::tips::TIPS.len() - 1),
        "round to the last"
    );
    let show = h.text_at("Show a tip when Cut Craft starts").unwrap();
    h.click(show, Modifiers::NONE);
    h.frame(vec![]);
    assert!(!h.app.options.general.show_tips_at_startup, "turned off");
    let close = h.text_at("Close").unwrap();
    h.click(close, Modifiers::NONE);
    h.frame(vec![]);
    assert_eq!(h.app.tip, None);
}

#[test]
fn a_different_tip_shows_each_start_unless_turned_off() {
    let mut h = Harness::at_start();
    h.app.options.next_tip = 3;
    h.app.tip_at_startup();
    assert_eq!(h.app.tip, Some(3));
    assert_eq!(h.app.options.next_tip, 4, "the next one next time");
    h.app.tip = None;
    h.app.options.next_tip = super::tips::TIPS.len() - 1;
    h.app.tip_at_startup();
    assert_eq!(
        h.app.options.next_tip, 0,
        "back to the first after the last"
    );
    h.app.tip = None;
    h.app.options.general.show_tips_at_startup = false;
    h.app.tip_at_startup();
    assert_eq!(h.app.tip, None, "turned off");
    h.app.options.general.show_tips_at_startup = true;
    h.app.open_first_run();
    h.app.tip_at_startup();
    assert_eq!(h.app.tip, None, "not over first-time setup");
}

#[test]
fn every_tip_says_something_and_its_more_is_a_help_page() {
    let h = Harness::new();
    let topics = h.app.all_help_topics();
    let mut seen = std::collections::HashSet::new();
    for tip in super::tips::TIPS {
        assert!(tip.text.split_whitespace().count() >= 10, "{}", tip.text);
        assert!(seen.insert(tip.text), "said twice: {}", tip.text);
        assert!(
            topics.contains(&tip.topic),
            "{} is not a help page: {}",
            tip.topic,
            tip.text
        );
        for name in ["Aspire", "Vectric"] {
            assert!(!tip.text.contains(name), "{name} named: {}", tip.text);
        }
    }
    let page = h.app.help_markdown("tips");
    assert!(page.contains(super::tips::TIPS[0].text));
}

#[test]
fn every_tip_tells_more_on_a_page_of_its_own_not_help_contents() {
    // Tell Me More… on Help Contents leaves a beginner to hunt for what the tip meant
    let vague: Vec<&str> = super::tips::TIPS
        .iter()
        .filter(|t| t.topic == "contents")
        .map(|t| t.text)
        .collect();
    assert!(
        vague.is_empty(),
        "point these at their own page:\n{vague:#?}"
    );
}

#[test]
fn there_are_well_over_a_hundred_tips_on_every_subject() {
    use super::tips::{Area, TIPS};
    assert!(TIPS.len() >= 129, "only {} tips", TIPS.len());
    for area in Area::ALL {
        let n = TIPS.iter().filter(|t| t.area == area).count();
        assert!(n >= 5, "{area:?} has only {n} tips");
    }
}

#[test]
fn the_tips_page_sorts_them_by_subject_and_links_each_to_its_help() {
    let h = Harness::new();
    let page = h.app.help_markdown("tips");
    let mut at = 0;
    for area in super::tips::Area::ALL {
        let heading = format!("\n## {}\n", area.heading());
        let found = page[at..]
            .find(&heading)
            .unwrap_or_else(|| panic!("{heading:?} missing, or out of order"));
        at += found + heading.len();
    }
    for tip in super::tips::TIPS {
        let line = format!("- {} [Tell me more](help:{})", tip.text, tip.topic);
        assert!(page.contains(&line), "not listed with its link: {line}");
    }
}

#[test]
fn every_tip_has_an_illustration_of_its_own_or_its_tools_icon() {
    use super::tips::Picture;
    let h = Harness::new();
    let icons = icon_set();
    let mut seen = std::collections::HashSet::new();
    let mut missing = vec![];
    for tip in super::tips::TIPS {
        match tip.picture {
            Picture::Scene(picture) => {
                assert!(
                    picture.starts_with("tips/")
                        && std::path::Path::new(picture)
                            .extension()
                            .is_some_and(|e| e == "png"),
                    "{picture}: a drawn illustration from assets/icons/tips"
                );
                assert!(icons.contains(picture), "{picture} is not drawn");
                assert!(seen.insert(picture), "{picture} illustrates two tips");
            }
            Picture::Icon(id) => {
                let Some(key) = h.app.tip_icon(id) else {
                    missing.push(id);
                    continue;
                };
                assert!(!key.starts_with("tips/"), "{key}: an icon, not a scene");
            }
        }
    }
    let unused: Vec<_> = icons
        .iter()
        .filter(|k| k.starts_with("tips/") && !seen.contains(k.as_str()))
        .collect();
    assert!(unused.is_empty(), "illustrations no tip shows: {unused:?}");
    assert!(missing.is_empty(), "no icon for these tips: {missing:?}");
}

#[test]
fn a_tip_about_a_tool_shows_that_tools_icon() {
    use super::tips::{Picture, TIPS};
    let h = Harness::new();
    let tip = TIPS
        .iter()
        .find(|t| t.picture == Picture::Icon("pocket_toolpath"))
        .expect("a tip about pockets");
    assert_eq!(
        h.app.tip_picture(tip),
        Some("toolpaths/pocket_toolpath.png"),
        "the same picture as the button and its help page"
    );
    assert_eq!(h.app.tip_picture(&TIPS[0]), Some("tips/sample_project.png"));
}

#[test]
fn the_tips_window_shows_the_illustration_above_the_words() {
    let mut h = Harness::at_start();
    h.choose(help_menu, &["Did You Know?..."]);
    h.frame(vec![]);
    h.frame(vec![]);
    let starting = |start: &str| {
        h.shapes.iter().find_map(|clipped| match &clipped.shape {
            egui::Shape::Text(t) if t.galley.text().starts_with(start) => Some(t.pos),
            _ => None,
        })
    };
    let words = starting("New to Cut Craft?").expect("the first tip's words");
    let tip = starting("Tip 1 of").expect("the count");
    assert!(
        words.y - tip.y > 6.0 * 18.0,
        "room for the picture between the count and the words: {tip:?} {words:?}"
    );
}

#[test]
fn every_picture_in_the_help_is_in_the_icon_set() {
    let known = icon_set();
    let h = Harness::new();
    let mut pictures = vec![];
    for topic in h.app.all_help_topics() {
        let md = h.app.help_markdown(topic);
        let mut at = md.as_str();
        while let Some(i) = at.find("](bytes://icons/") {
            let after = &at[i + "](bytes://icons/".len()..];
            let end = after.find(')').expect("a picture's link ends");
            pictures.push(after[..end].to_owned());
            at = &after[end..];
        }
    }
    for p in &pictures {
        assert!(
            known.contains(p),
            "{p} is not in assets/icons: add its PNG there"
        );
    }
    // the clearing pictures: the light step's bite, the edge pass, a channel's loops
    for p in [
        "diagrams/light_step.png",
        "diagrams/edge_pass.png",
        "diagrams/adaptive_channel.png",
    ] {
        assert!(pictures.iter().any(|q| q == p), "the help shows {p}");
    }
}

#[test]
fn the_tool_changer_is_offered_only_for_a_machine_with_one_and_warned_about_without() {
    use crate::files::ToolChanges;
    let mut h = Harness::new();
    two_tools(&mut h);
    let plan = vec![h.app.doc.toolpaths.keys().copied().collect::<Vec<_>>()];
    // no machine set up: no tool changer to offer, and choosing it anyway warns in red
    assert!(!h.app.has_tool_changer());
    let warned = |h: &Harness, changes| {
        h.app
            .preflight(&plan, changes)
            .iter()
            .any(|f| f.message.contains("no tool changer"))
    };
    assert!(warned(&h, ToolChanges::ToolChanger));
    assert!(!warned(&h, ToolChanges::SeparateFiles));
    h.app.machines.add();
    assert!(!h.app.has_tool_changer());
    assert!(warned(&h, ToolChanges::ToolChanger));
    h.app.machines.active_mut().unwrap().set_slot_count(4);
    assert!(h.app.has_tool_changer());
    assert!(!warned(&h, ToolChanges::ToolChanger));
}

#[test]
fn saving_warns_about_a_tool_changer_the_machine_does_not_have() {
    // the checks Save Toolpaths runs, not only the test-only `preflight`, warn: without it
    // the file changes tools with M6 on a machine that has no changer
    use crate::files::ToolChanges;
    let mut h = Harness::new();
    two_tools(&mut h);
    let plan = vec![h.app.doc.toolpaths.keys().copied().collect::<Vec<_>>()];
    let warned = |h: &mut Harness, changes| {
        h.app
            .save_checks(&plan, changes)
            .expect("checked")
            .iter()
            .any(|f| f.message.contains("no tool changer"))
    };
    assert!(warned(&mut h, ToolChanges::ToolChanger));
    assert!(!warned(&mut h, ToolChanges::SeparateFiles));
}

#[test]
fn save_toolpaths_lists_every_check_with_how_it_came_out_and_its_help() {
    // Matthew: "what is it actually checking ... what is it doing for me"
    use cut_craft_core::preflight::{Check, Finding};
    let mut h = ready_for_output();
    h.app.invoke(&h.ctx, "save_toolpaths");
    let form = h.app.save_form.clone().unwrap();
    let post = h.app.form_post(&form).unwrap();
    h.app.save_checks = vec![super::SaveChecks {
        plan: h.app.plan(&form, &post).unwrap(),
        changes: form.changes,
        revision: h.app.history.revision(),
        machine: h.app.machines.active,
        found: Some(vec![
            Finding::error(Check::Rapids, "Profile would crash".into()),
            Finding::warning(Check::GenericTool, "Profile: a generic bit".into()),
        ]),
    }];
    h.frame(vec![]);
    h.frame(vec![]);
    let texts = h.texts();
    let shown = |t: &str| texts.iter().any(|s| s == t);
    assert!(shown("Checks before cutting"), "{texts:?}");
    // what needs a look comes first, by name, with what was found under it
    assert!(shown(Check::Rapids.name()), "{texts:?}");
    assert!(shown("Must fix: Profile would crash"), "{texts:?}");
    assert!(shown(Check::GenericTool.name()), "{texts:?}");
    assert!(shown("⚠ Profile: a generic bit"), "{texts:?}");
    assert!(
        texts
            .iter()
            .any(|t| t.starts_with("✔ ") && t.contains("passed · ⚠ 1 to read · ✖ 1 must fix")),
        "{texts:?}"
    );
    // the heading's ? opens the help about every check
    let heading = h.text_at("Checks before cutting").unwrap();
    let help = h
        .texts_at("?")
        .into_iter()
        .filter(|p| (p.y - heading.y).abs() < 4.0)
        .min_by(|a, b| (a.x - heading.x).abs().total_cmp(&(b.x - heading.x).abs()))
        .unwrap();
    h.click(help, Modifiers::NONE);
    h.frame(vec![]);
    assert!(
        matches!(
            h.app.info,
            Some(super::Info::Help(super::check_list::OVERVIEW))
        ),
        "opens the checks' help"
    );
}

#[test]
fn the_collet_hitting_the_wood_is_always_asked_about_before_saving() {
    let mut h = ready_for_output();
    let form = one_file_form();
    let post = h.app.form_post(&form).unwrap();
    let message = "Profile 1: the collet nut would hit the material".to_owned();
    let found = |severity| {
        Some(vec![cut_craft_core::preflight::Finding {
            severity,
            message: message.clone(),
            check: cut_craft_core::preflight::Check::Collisions,
        }])
    };
    let checks = |h: &Harness, severity| {
        vec![super::SaveChecks {
            plan: h.app.plan(&form, &post).unwrap(),
            changes: form.changes,
            revision: h.app.history.revision(),
            machine: h.app.machines.active,
            found: found(severity),
        }]
    };
    // an ordinary warning, ticked as read, saves without asking again
    h.app.acknowledged.insert(message.clone());
    h.app.save_checks = checks(&h, cut_craft_core::preflight::Severity::Warning);
    assert!(h.app.ready_to_save(&form, false).is_some());
    // a collision, ticked too, still asks
    h.app.save_checks = checks(&h, cut_craft_core::preflight::Severity::MustAsk);
    assert!(h.app.ready_to_save(&form, false).is_none());
    assert!(h.app.save_anyway.is_some());
}

#[test]
fn the_collision_help_explains_the_sizes_with_a_picture() {
    let h = Harness::new();
    assert!(h.app.all_help_topics().contains(&"collisions"));
    assert!(
        h.app
            .help_markdown("contents")
            .contains("(help:collisions)"),
        "linked from Contents"
    );
    assert!(
        h.app
            .help_markdown("toolpaths")
            .contains("(help:collisions)")
    );
    let page = h.app.help_markdown("collisions");
    for words in [
        "Stick-out",
        "Collet nut across",
        "Spindle across",
        "Holder per bit",
        "Own holder",
        "This question cannot be switched",
        "Clamp Height",
        "(help:keep_out_zones)",
        "pictures/collet-hits.png",
    ] {
        assert!(page.contains(words), "{words}");
    }
    assert!(
        cut_craft_help::PICTURES
            .iter()
            .any(|(name, bytes)| *name == "collet-hits.png" && bytes.starts_with(b"\x89PNG")),
        "its picture goes with the app"
    );
}

#[test]
fn model_position_help_is_on_both_forms_with_pictures_and_links_that_go_somewhere() {
    let h = Harness::new();
    let topics = h.app.all_help_topics();
    assert!(topics.contains(&"material_setup"));
    for form in ["material_setup", "orientate_3d_model"] {
        let page = h.app.help_markdown(form);
        for picture in [
            "model-top-at-top.png",
            "model-bottom-on-bottom.png",
            "model-sticks-out.png",
        ] {
            assert!(page.contains(&format!("pictures/{picture}")), "{picture}");
            assert!(
                cut_craft_help::PICTURES
                    .iter()
                    .any(|(name, _)| *name == picture),
                "{picture} is loaded"
            );
        }
        for link in page.split("(help:").skip(1) {
            let topic = link.split(')').next().unwrap();
            assert!(topics.contains(&topic), "help:{topic} goes nowhere");
        }
    }
}

#[test]
fn help_pages_live_in_the_help_crate_not_the_app() {
    // the app shows only what crates/help carries: a page or picture left in
    // crates/app/help is never shown
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("help");
    let strays: Vec<String> = std::fs::read_dir(&dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|name| name != "coverage-baseline.txt")
        .collect();
    assert!(
        strays.is_empty(),
        "move these to crates/help/pages (or crates/help/pictures): {strays:?}"
    );
}
