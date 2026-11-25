use super::LibraryFile;
use std::path::{Path, PathBuf};

fn written(file: &LibraryFile) -> (Result<(), String>, Option<PathBuf>) {
    let mut to = None;
    let result = file.save("Bit Library", |p: &Path| {
        to = Some(p.to_path_buf());
        Ok(())
    });
    (result, to)
}

#[test]
fn it_saves_to_its_file() {
    let mut file = LibraryFile::default();
    file.at(Some("bits.json".into()));
    assert_eq!(written(&file), (Ok(()), Some("bits.json".into())));
}

#[test]
fn one_kept_in_memory_saves_nowhere() {
    assert_eq!(written(&LibraryFile::default()), (Ok(()), None));
}

#[test]
fn a_file_that_could_not_be_read_is_never_written_over() {
    let mut file = LibraryFile::default();
    file.at(Some("bits.json".into()));
    file.unreadable();
    assert_eq!(file.path(), None);
    let (result, to) = written(&file);
    assert_eq!(to, None, "not written");
    let said = result.unwrap_err();
    assert!(said.starts_with("Bit Library not saved: bits.json could not be read"));
}

#[test]
fn a_failed_write_says_why() {
    let mut file = LibraryFile::default();
    file.at(Some("bits.json".into()));
    assert_eq!(
        file.save("Bit Library", |_| Err("disk full".into())),
        Err("disk full".into())
    );
}
