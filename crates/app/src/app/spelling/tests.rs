use super::Spelling;
use cut_craft_core::spell::Speller;

fn dictionary() -> Speller {
    Speller::from_hunspell("TRY esianrtolcdugmphbyfvkwz\n", "2\nwood\nsign\n")
}

fn fresh_file(name: &str) -> std::path::PathBuf {
    let dir =
        std::env::temp_dir().join(format!("cut-craft-spelling-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir.join("words.txt")
}

#[test]
fn the_dictionary_is_looked_for_once() {
    let mut spelling = Spelling::default();
    let mut looks = 0;
    assert!(
        spelling
            .speller_from(|| {
                looks += 1;
                None
            })
            .is_none()
    );
    assert!(
        spelling
            .speller_from(|| {
                looks += 1;
                Some(dictionary())
            })
            .is_none(),
        "not again"
    );
    assert_eq!(looks, 1);
}

#[test]
fn the_users_own_words_are_read_with_the_dictionary() {
    let path = fresh_file("read");
    std::fs::write(&path, "Zed\nQux").unwrap();
    let mut spelling = Spelling::default();
    spelling.words_at(Some(path));
    let speller = spelling.speller_from(|| Some(dictionary())).unwrap();
    assert_eq!(speller.misspelled("wood Zed Qux Blah"), ["Blah"]);
}

#[test]
fn an_added_word_is_kept_in_the_words_file_and_a_removed_one_taken_out() {
    let path = fresh_file("kept");
    let mut spelling = Spelling::default();
    spelling.words_at(Some(path.clone()));
    spelling.given(dictionary());
    assert_eq!(spelling.user_word("Zed", true), Ok(()));
    assert_eq!(spelling.user_word("Qux", true), Ok(()));
    assert_eq!(
        std::fs::read_to_string(&path).unwrap(),
        "qux\nzed",
        "kept lower-case"
    );
    assert_eq!(spelling.user_word("Zed", false), Ok(()));
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "qux");
}

#[test]
fn a_words_file_that_cannot_be_written_says_why() {
    let dir = fresh_file("unwritable");
    // a folder where the file should be: it cannot be written over
    std::fs::create_dir_all(&dir).unwrap();
    let mut spelling = Spelling::default();
    spelling.words_at(Some(dir));
    spelling.given(dictionary());
    let e = spelling.user_word("Zed", true).unwrap_err();
    assert!(e.contains("could not be kept"), "{e}");
    assert!(
        spelling.speller().unwrap().misspelled("Zed").is_empty(),
        "checked until close"
    );
}

#[test]
fn without_a_dictionary_there_is_nothing_to_add_to() {
    let mut spelling = Spelling::default();
    let _ = spelling.speller_from(|| None);
    assert_eq!(spelling.user_word("Zed", true), Ok(()));
}
