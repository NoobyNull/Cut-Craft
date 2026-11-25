use super::OptionsEditor;
use crate::options::Options;

fn changed(options: &Options) -> Options {
    let mut edited = options.clone();
    edited.controls.trackpad = !edited.controls.trackpad;
    edited
}

#[test]
fn opening_at_a_category_shows_it_once() {
    let mut editor = OptionsEditor::default();
    editor.open_at("language");
    assert!(editor.open);
    assert_eq!(editor.take_goto(), Some("language"));
    assert_eq!(
        editor.take_goto(),
        None,
        "it stays where the user goes next"
    );
}

#[test]
fn changes_wait_until_taken() {
    let (options, mut editor) = (Options::default(), OptionsEditor::default());
    let edited = changed(&options);
    editor.edited(&options, edited.clone());
    assert!(editor.dirty());
    assert_eq!(editor.shown(&options), &edited, "it shows the change");
    assert_eq!(editor.take_changes(), Some(edited));
    assert!(!editor.dirty(), "Apply used them");
    assert_eq!(editor.shown(&options), &options);
}

#[test]
fn editing_back_to_the_options_as_they_are_leaves_nothing_waiting() {
    let (options, mut editor) = (Options::default(), OptionsEditor::default());
    editor.edited(&options, changed(&options));
    editor.edited(&options, options.clone());
    assert!(!editor.dirty());
}

#[test]
fn closing_throws_away_what_was_not_applied() {
    let (options, mut editor) = (Options::default(), OptionsEditor::default());
    editor.open_at("appearance");
    editor.edited(&options, changed(&options));
    editor.job_defaults_form = Some((serde_json::Value::Null, None));
    editor.closing = true;
    editor.close();
    assert!(!editor.open);
    assert!(!editor.dirty());
    assert_eq!(editor.job_defaults_form, None);
    assert!(!editor.closing, "Close's question goes too");
}
