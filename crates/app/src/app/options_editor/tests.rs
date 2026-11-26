use super::OptionsEditor;
use crate::options::Options;

#[test]
fn opening_at_a_category_shows_it_once() {
    let mut editor = OptionsEditor::default();
    editor.open_at("language");
    assert!(editor.window.open);
    assert_eq!(editor.take_goto(), Some("language"));
    assert_eq!(
        editor.take_goto(),
        None,
        "it stays where the user goes next"
    );
}

#[test]
fn closing_throws_away_what_was_not_applied_and_job_defaults_form() {
    let (options, mut editor) = (Options::default(), OptionsEditor::default());
    editor.open_at("appearance");
    let mut edited = options.clone();
    edited.controls.trackpad = !edited.controls.trackpad;
    editor.window.edited(&options, edited);
    editor.job_defaults_form = Some((serde_json::Value::Null, None));
    editor.close();
    assert!(!editor.window.open);
    assert!(!editor.window.dirty());
    assert_eq!(editor.job_defaults_form, None);
}
