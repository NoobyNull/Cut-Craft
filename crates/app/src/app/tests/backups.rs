//! Backing up the settings: Help ▸ Back Up Settings, the backup made before the window layout
//! changes, and Options ▸ Backups' list.

use super::*;

/// A settings folder of its own for one test, with something in it to back up.
fn settings_folder(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "cut-craft-app-backups-{name}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("words.txt"), "Whittle\n").unwrap();
    dir
}

#[test]
fn back_up_settings_lists_every_part_ticked_and_cancel_closes_it() {
    let mut h = Harness::at_start();
    h.choose_on_menu_bar("Help", &["Back Up Settings..."]);
    h.frame(vec![]);
    h.frame(vec![]);
    for part in cut_craft_backup::Part::ALL {
        let name = cut_craft_i18n::tr(part.key());
        assert!(h.text_at(&name).is_some(), "{name}: {:?}", h.texts());
    }
    assert!(h.text_at("Include the Web API access token").is_some());
    assert!(h.text_at("Save Backup...").is_some());
    let cancel = h.text_at("Cancel").unwrap();
    h.click(cancel, Modifiers::NONE);
    h.frame(vec![]);
    h.frame(vec![]);
    assert!(h.text_at("Save Backup...").is_none(), "closed");
}

#[test]
fn changing_the_window_layout_backs_up_first_and_options_lists_it() {
    let dir = settings_folder("layout");
    let mut h = Harness::new();
    h.given_config_folder(dir.clone());
    let auto = h.automatic_backups().unwrap();
    let other = if h.options().general.layout == crate::options::WindowLayout::Canvas {
        "layout_ribbon"
    } else {
        "layout_canvas"
    };
    h.set_layout(other);
    let kept = auto.kept();
    assert_eq!(kept.len(), 1);
    assert_eq!(
        kept[0].manifest.reason,
        cut_craft_backup::Reason::BeforeLayoutChange
    );
    // picking the layout already in use changes nothing, so backs up nothing
    h.set_layout(other);
    assert_eq!(auto.kept().len(), 1);

    h.open_options_at("backups");
    h.frame(vec![]);
    h.frame(vec![]);
    assert!(
        h.text_at("Before changing the window layout").is_some(),
        "{:?}",
        h.texts()
    );
    assert!(h.text_at("Back up every week").is_some());
    assert!(h.text_at("Backups to keep").is_some());
}

#[test]
fn tests_back_up_nothing_unless_they_say_where() {
    let h = Harness::new();
    assert!(h.automatic_backups().is_none());
}
