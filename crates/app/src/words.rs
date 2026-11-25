//! The app's own text that has no name of its own, known by its English words: a heading, a
//! hint, a choice in a window. The code writes it as `t!("English words")`, and
//! `locales/en/text.ftl` holds every one (a test checks), keyed by
//! [`cut_craft_i18n::text_key`]: `text-`, the first words and a check of the whole text, so a
//! reworded text gets a new key and an old translation never shows for it.

/// The key of a text known by its English words (shown by [`cut_craft_i18n::text`]).
#[cfg(test)]
pub fn text_key(english: &str) -> String {
    cut_craft_i18n::text_key("text", english)
}

#[cfg(test)]
mod tests {
    use super::text_key;
    use crate::app::hard_coded_text::{literal_at, program, short, sources};
    use cut_craft_i18n::english;
    use std::fmt::Write as _;

    /// Every `&str` constant's name and words in the app's code (`const NAME: &str = "…"`).
    fn constants(codes: &[(std::path::PathBuf, String)]) -> Vec<(String, String)> {
        let mut out = vec![];
        for (_, code) in codes {
            let bytes = code.as_bytes();
            let mut from = 0;
            while let Some(at) = code[from..].find("const ") {
                let start = from + at + "const ".len();
                from = start;
                let rest = &code[start..];
                let name: String = rest
                    .chars()
                    .take_while(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || *c == '_')
                    .collect();
                let after = rest[name.len()..].trim_start();
                if name.is_empty() || !after.starts_with(": &str =") {
                    continue;
                }
                let open = start + rest.len() - after.len() + ": &str =".len();
                let open = open + code[open..].len() - code[open..].trim_start().len();
                if let Some((words, _)) = literal_at(bytes, open) {
                    out.push((name, words));
                }
            }
        }
        out
    }

    /// Every `t!("…")` and `t!(CONSTANT)` in the app's code: its file and its English words,
    /// each text once.
    fn every_text() -> Vec<(String, String)> {
        let codes: Vec<(std::path::PathBuf, String)> = sources()
            .into_iter()
            .map(|f| {
                let code = program(&std::fs::read_to_string(&f).unwrap());
                (f, code)
            })
            .collect();
        let constants = constants(&codes);
        let mut out: Vec<(String, String)> = vec![];
        let mut add = |file: &std::path::Path, words: String| {
            if !out.iter().any(|(_, w)| *w == words) {
                out.push((short(file), words));
            }
        };
        for (file, code) in &codes {
            let bytes = code.as_bytes();
            let mut from = 0;
            while let Some(at) = code[from..].find("t!(") {
                let start = from + at;
                from = start + 3;
                // a whole name: not `format!(` or `assert!(`
                let whole = start == 0
                    || !(bytes[start - 1].is_ascii_alphanumeric() || bytes[start - 1] == b'_');
                let mut open = from;
                while bytes.get(open).is_some_and(u8::is_ascii_whitespace) {
                    open += 1;
                }
                if !whole {
                    continue;
                }
                if bytes.get(open) == Some(&b'"') {
                    if let Some((words, _)) = literal_at(bytes, open) {
                        add(file, words);
                    }
                } else {
                    let name: String = code[open..]
                        .chars()
                        .take_while(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || *c == '_')
                        .collect();
                    let words = constants.iter().find(|(n, _)| *n == name).map(|(_, w)| w);
                    assert!(
                        name.is_empty() || words.is_some(),
                        "{}: t!({name}) names no `const {name}: &str`",
                        short(file)
                    );
                    if let Some(words) = words {
                        add(file, words.clone());
                    }
                }
            }
        }
        out
    }

    /// Every command's code, the file formats', the border tiles' and the core's: the files
    /// whose messages go in `errors.ftl`.
    fn error_sources() -> Vec<(std::path::PathBuf, String)> {
        fn walk(dir: &std::path::Path, files: &mut Vec<std::path::PathBuf>) {
            let Ok(entries) = std::fs::read_dir(dir) else {
                return;
            };
            for entry in entries {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    walk(&path, files);
                } else if path.extension().is_some_and(|e| e == "rs")
                    && !path.to_string_lossy().ends_with("tests.rs")
                {
                    files.push(path);
                }
            }
        }
        let app = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let crates = app.parent().unwrap_or(app);
        let mut files = vec![];
        walk(&crates.join("core/src"), &mut files);
        walk(&crates.join("border-tile/src"), &mut files);
        let mut commands: Vec<_> = ["commands", "formats"]
            .iter()
            .flat_map(|group| std::fs::read_dir(crates.join(group)).unwrap())
            .map(|e| e.unwrap().path().join("src"))
            .collect();
        commands.sort();
        for dir in commands {
            walk(&dir, &mut files);
        }
        files.sort();
        files
            .into_iter()
            .map(|f| {
                let code = program(&std::fs::read_to_string(&f).unwrap());
                (f, code)
            })
            .collect()
    }

    /// The texts in the brackets that open at `open`: every literal and `&str` constant there,
    /// in each branch of an `if` or `match`, but none inside a `format!` (it has values in it,
    /// so no key).
    fn texts_in(code: &str, open: usize, constants: &[(String, String)]) -> Vec<String> {
        let bytes = code.as_bytes();
        let mut found = vec![];
        let (mut depth, mut skip) = (0usize, None);
        let mut i = open;
        while i < bytes.len() {
            match bytes[i] {
                b'(' => {
                    depth += 1;
                    let before = code[..i].trim_end();
                    if skip.is_none() && before.ends_with("format!") {
                        skip = Some(depth);
                    }
                }
                b')' => {
                    if skip == Some(depth) {
                        skip = None;
                    }
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                b'\'' if bytes.get(i + 2) == Some(&b'\'') => i += 2,
                b'"' => {
                    let Some((words, end)) = literal_at(bytes, i) else {
                        break;
                    };
                    if skip.is_none() {
                        found.push(words);
                    }
                    i = end;
                    continue;
                }
                c if c.is_ascii_uppercase()
                    && !(bytes[i - 1].is_ascii_alphanumeric() || bytes[i - 1] == b'_') =>
                {
                    let name: String = code[i..]
                        .chars()
                        .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                        .collect();
                    if skip.is_none()
                        && let Some((_, words)) = constants.iter().find(|(n, _)| *n == name)
                    {
                        found.push(words.clone());
                    }
                    i += name.len();
                    continue;
                }
                _ => {}
            }
            i += 1;
        }
        found
    }

    /// Every message a tool writes as it is, in `Failed(…)`, `InvalidParams(…)` or the `Err(…)`
    /// of the code that hands a tool its message (a file format's reader, the core), and
    /// [`cut_craft_core::SELECT_FIRST`]: its file and its words, each once.
    fn every_error() -> Vec<(String, String)> {
        let codes = error_sources();
        let constants = constants(&codes);
        let mut out: Vec<(String, String)> = vec![(
            "core/src/command.rs".to_owned(),
            cut_craft_core::SELECT_FIRST.to_owned(),
        )];
        for (file, code) in &codes {
            let bytes = code.as_bytes();
            for call in ["Failed(", "InvalidParams(", "Err(", "map_err("] {
                let mut from = 0;
                while let Some(at) = code[from..].find(call) {
                    let start = from + at;
                    from = start + call.len();
                    if start > 0
                        && (bytes[start - 1].is_ascii_alphanumeric() || bytes[start - 1] == b'_')
                    {
                        continue;
                    }
                    for words in texts_in(code, from - 1, &constants) {
                        // a word or two is a name handed to a message (`gone("sheet")`), not one
                        if words.split_whitespace().count() >= 3
                            && !out.iter().any(|(_, w)| *w == words)
                        {
                            out.push((short(file), words));
                        }
                    }
                }
            }
        }
        out.sort_by(|a, b| a.0.cmp(&b.0));
        out
    }

    fn error_key(words: &str) -> String {
        cut_craft_i18n::text_key("error", words)
    }

    #[test]
    fn every_command_message_has_its_english_words() {
        let problems: Vec<String> = every_error()
            .into_iter()
            .filter_map(|(file, words)| {
                let key = error_key(&words);
                match english().words(&key, None) {
                    Some(got) if got == words => None,
                    Some(got) => Some(format!(
                        "{file}: {key} = {got:?}, but the code says {words:?}"
                    )),
                    None => Some(format!("{file}: {key} is missing ({words:?})")),
                }
            })
            .collect();
        assert!(
            problems.is_empty(),
            "crates/i18n/locales/en/errors.ftl does not match the commands; run `cargo test -p \
             cut-craft --bin cut-craft write_error_words -- --ignored` to write it again:\n{}",
            problems.join("\n")
        );
        let keys: Vec<String> = every_error().iter().map(|(_, w)| error_key(w)).collect();
        let stale: Vec<&String> = english()
            .keys()
            .iter()
            .filter(|k| k.starts_with("error-") && !keys.contains(k))
            .collect();
        assert!(
            stale.is_empty(),
            "no command says these any more: {stale:?}"
        );
    }

    #[test]
    fn a_command_message_shows_in_the_chosen_language() {
        let message = cut_craft_core::CommandError::Failed("select vectors to copy".into());
        assert_eq!(message.to_string(), "Select vectors to copy.");
        assert_eq!(
            cut_craft_core::error_words("a message nobody keyed"),
            "a message nobody keyed"
        );
    }

    /// Writes `locales/en/errors.ftl` from the commands (`cargo test -p cut-craft --bin
    /// cut-craft write_error_words -- --ignored`), after a command's message is added or
    /// changed.
    #[test]
    #[ignore = "writes a file; run by hand"]
    fn write_error_words() {
        let mut out = String::from(
            "# What a tool says when it cannot run, keyed by its English words (`error-`, its \
             first words and a\n# check of the whole text). The app starts each with a capital \
             and ends it with a full stop,\n# so write them as the English is. Each line says \
             which tool says it.\n",
        );
        let mut last = String::new();
        for (file, words) in every_error() {
            if file != last {
                let _ = write!(out, "\n## {file}\n");
                last = file;
            }
            let _ = writeln!(
                out,
                "{} ={}",
                error_key(&words),
                cut_craft_i18n::ftl_value(&words)
            );
        }
        let path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../i18n/locales/en/errors.ftl");
        std::fs::write(path, out).unwrap();
    }

    #[test]
    fn every_text_has_its_english_words() {
        let problems: Vec<String> = every_text()
            .into_iter()
            .filter_map(|(file, words)| {
                let key = text_key(&words);
                match english().words(&key, None) {
                    Some(got) if got == words => None,
                    Some(got) => Some(format!(
                        "{file}: {key} = {got:?}, but the code says {words:?}"
                    )),
                    None => Some(format!("{file}: {key} is missing ({words:?})")),
                }
            })
            .collect();
        assert!(
            problems.is_empty(),
            "crates/i18n/locales/en/text.ftl does not match the code; run `cargo test -p \
             cut-craft --bin cut-craft write_text_words -- --ignored` to write it again:\n{}",
            problems.join("\n")
        );
    }

    #[test]
    fn no_text_is_left_over() {
        let keys: Vec<String> = every_text().iter().map(|(_, w)| text_key(w)).collect();
        let stale: Vec<&String> = english()
            .keys()
            .iter()
            .filter(|k| k.starts_with("text-") && !keys.contains(k))
            .collect();
        assert!(stale.is_empty(), "no code shows these any more: {stale:?}");
    }

    #[test]
    fn a_text_with_no_key_shows_as_written() {
        assert_eq!(
            cut_craft_i18n::text("Words nobody wrote a key for"),
            "Words nobody wrote a key for"
        );
    }

    /// Writes `locales/en/text.ftl` from the code (`cargo test -p cut-craft --bin cut-craft
    /// write_text_words -- --ignored`), after a `t!` text is added or changed.
    #[test]
    #[ignore = "writes a file; run by hand"]
    fn write_text_words() {
        let mut out = String::from(
            "# The app's own text with no name of its own: headings, hints and choices in its \
             windows,\n# each keyed by its English words (`text-`, its first words and a check \
             of the whole text),\n# so a key changes when its English does. Each line says \
             where the text is.\n",
        );
        let mut last = String::new();
        for (file, words) in every_text() {
            if file != last {
                let _ = write!(out, "\n## {file}\n");
                last = file;
            }
            let _ = writeln!(
                out,
                "{} ={}",
                text_key(&words),
                cut_craft_i18n::ftl_value(&words)
            );
        }
        let path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../i18n/locales/en/text.ftl");
        std::fs::write(path, out).unwrap();
    }
}
