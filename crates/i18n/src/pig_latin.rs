//! The Pig Latin test pack (`language-packs/en-piglatin/`): every English word turned into
//! Pig Latin, made from the English files as they are, so it always has every key. Installed
//! like any language pack, it shows at a glance any text still written into the code, as that
//! stays plain English. Its `.ftl` files are not kept in the source: the Language packs
//! workflow makes them before it zips the pack (`write_pig_latin_pack`).

use fluent_syntax::ast::{Entry, Expression, Pattern, PatternElement};

/// One English `.ftl` file in Pig Latin: every message's words and every attribute's, with
/// the values put in (`{ $name }`), the choices (`[one]`) and the keys left as they are.
pub fn file(english: &str) -> String {
    let mut resource = match fluent_syntax::parser::parse(english.to_owned()) {
        Ok(r) | Err((r, _)) => r,
    };
    resource.body.retain(|e| matches!(e, Entry::Message(_)));
    for entry in &mut resource.body {
        if let Entry::Message(m) = entry {
            m.comment = None;
            if let Some(value) = &mut m.value {
                pattern(value);
            }
            for a in &mut m.attributes {
                pattern(&mut a.value);
            }
        }
    }
    let mut out = String::from(
        "# Made from English by the Language packs workflow (write_pig_latin_pack): not edited.\n\n",
    );
    out.push_str(&fluent_syntax::serializer::serialize(&resource));
    out
}

fn pattern(p: &mut Pattern<String>) {
    for element in &mut p.elements {
        match element {
            PatternElement::TextElement { value } => *value = text(value),
            PatternElement::Placeable {
                expression: Expression::Select { variants, .. },
            } => {
                for v in variants {
                    pattern(&mut v.value);
                }
            }
            PatternElement::Placeable { .. } => {}
        }
    }
}

/// Plain text in Pig Latin, its spaces, line breaks and punctuation kept. A file name, a web
/// address, a key such as Ctrl+K, a number or a short form in capitals (DXF) is kept as is.
pub fn text(english: &str) -> String {
    let mut out = String::with_capacity(english.len() * 2);
    let mut token = String::new();
    for c in english.chars() {
        if c.is_whitespace() {
            out.push_str(&token_in_pig_latin(&token));
            token.clear();
            out.push(c);
        } else {
            token.push(c);
        }
    }
    out.push_str(&token_in_pig_latin(&token));
    out
}

fn token_in_pig_latin(token: &str) -> String {
    let kept = token
        .chars()
        .any(|c| c.is_ascii_digit() || "/\\@_+=<>#%&|~".contains(c))
        || token.contains("://")
        || token.split('.').filter(|part| !part.is_empty()).count() > 1;
    if kept {
        return token.to_owned();
    }
    // each run of letters (an apostrophe inside a word belongs to it) becomes Pig Latin
    let mut out = String::new();
    let mut word = String::new();
    let chars: Vec<char> = token.chars().collect();
    for (i, &c) in chars.iter().enumerate() {
        let inside =
            c == '\'' && !word.is_empty() && chars.get(i + 1).is_some_and(|n| n.is_alphabetic());
        if c.is_alphabetic() || inside {
            word.push(c);
        } else {
            out.push_str(&word_in_pig_latin(&word));
            word.clear();
            out.push(c);
        }
    }
    out.push_str(&word_in_pig_latin(&word));
    out
}

/// One word: `string` is `ingstray`, `apple` is `appleway`, `Quick` is `Ickquay`.
pub fn word_in_pig_latin(word: &str) -> String {
    let letters: Vec<char> = word.chars().collect();
    if letters.is_empty()
        || !letters
            .iter()
            .all(|c| c.is_ascii_alphabetic() || *c == '\'')
    {
        return word.to_owned();
    }
    let capitals = letters.iter().filter(|c| c.is_ascii_uppercase()).count();
    if letters.len() > 1 && capitals == letters.len() {
        return word.to_owned();
    }
    let lower: Vec<char> = letters.iter().map(char::to_ascii_lowercase).collect();
    let vowel = |i: usize| match lower[i] {
        'a' | 'e' | 'i' | 'o' | 'u' => true,
        'y' => i > 0,
        _ => false,
    };
    let mut split = (0..lower.len()).find(|&i| vowel(i)).unwrap_or(lower.len());
    if split > 0 && split < lower.len() && lower[split - 1] == 'q' && lower[split] == 'u' {
        split += 1;
    }
    let mut out: String = if split == 0 {
        lower
            .iter()
            .chain("way".chars().collect::<Vec<_>>().iter())
            .collect()
    } else {
        lower[split..]
            .iter()
            .chain(&lower[..split])
            .chain(['a', 'y'].iter())
            .collect()
    };
    if letters[0].is_ascii_uppercase() {
        let first = out.remove(0).to_ascii_uppercase();
        out.insert(0, first);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Catalogue, ENGLISH, english};

    #[test]
    fn words_turn_into_pig_latin() {
        for (en, pig) in [
            ("string", "ingstray"),
            ("apple", "appleway"),
            ("Quick", "Ickquay"),
            ("Draw", "Awdray"),
            ("rhythm", "ythmrhay"),
            ("don't", "on'tday"),
        ] {
            assert_eq!(word_in_pig_latin(en), pig, "{en}");
        }
    }

    #[test]
    fn text_keeps_its_spaces_punctuation_and_short_forms() {
        assert_eq!(text("Save As..."), "Avesay Asway...");
        assert_eq!(
            text("Import DXF (Ctrl+I), see cutcraft.app"),
            "Importway DXF (Ctrl+I), eesay cutcraft.app"
        );
        assert_eq!(text("line one\nline two"), "inelay oneway\ninelay otway");
    }

    /// Made from English, the pack reads with no problems, has every English key, and keeps
    /// every value put in, so no word is missing or shows a wrong number.
    #[test]
    fn the_pack_has_every_english_key_and_keeps_its_values() {
        let files: Vec<(String, String)> = ENGLISH
            .iter()
            .map(|(name, src)| ((*name).to_owned(), file(src)))
            .collect();
        let borrowed: Vec<(&str, &str)> = files
            .iter()
            .map(|(n, t)| (n.as_str(), t.as_str()))
            .collect();
        let (pack, problems) = Catalogue::parse("en-piglatin", &borrowed);
        assert!(problems.is_empty(), "{}", problems.join("\n"));
        assert_eq!(pack.coverage(), english().coverage());
        for (name, src) in &files {
            let en = ENGLISH.iter().find(|(n, _)| n == name).unwrap().1;
            let placeables = |s: &str| s.matches("{ $").count();
            assert_eq!(
                placeables(src),
                placeables(en),
                "{name} lost a value put in"
            );
        }
    }

    /// Writes the pack's `.ftl` files (`cargo test -p cut-craft-i18n write_pig_latin_pack --
    /// --ignored`): the Language packs workflow runs it before zipping the packs.
    #[test]
    #[ignore = "writes files; run by the Language packs workflow"]
    fn write_pig_latin_pack() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../language-packs/en-piglatin");
        for (name, src) in ENGLISH {
            std::fs::write(dir.join(name), file(src)).unwrap();
        }
    }
}
