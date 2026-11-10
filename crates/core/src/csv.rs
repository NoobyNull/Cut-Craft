//! Reading the CSV files spreadsheets save: quoted fields, and the encodings Excel writes.

/// CSV records: fields split at `delimiter`, quotes keeping delimiters and line breaks, a
/// doubled quote a quote.
pub fn records(text: &str, delimiter: char) -> Vec<Vec<String>> {
    let mut records = Vec::new();
    let (mut record, mut field, mut quoted) = (Vec::new(), String::new(), false);
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '"' if quoted && chars.peek() == Some(&'"') => {
                field.push('"');
                chars.next();
            }
            '"' => quoted = !quoted,
            c if c == delimiter && !quoted => record.push(std::mem::take(&mut field)),
            '\r' if !quoted => {}
            '\n' if !quoted => {
                record.push(std::mem::take(&mut field));
                records.push(std::mem::take(&mut record));
            }
            c => field.push(c),
        }
    }
    if !field.is_empty() || !record.is_empty() {
        record.push(field);
        records.push(record);
    }
    records
}

/// Text from a file a spreadsheet saved: UTF-8 (its byte-order mark dropped), or else read as
/// Latin-1, close enough to the Windows-1252 that Excel on Windows writes for "CSV" without
/// "UTF-8" for accented letters.
pub fn decode(bytes: &[u8]) -> String {
    let text = String::from_utf8(bytes.to_vec())
        .unwrap_or_else(|_| bytes.iter().map(|&b| char::from(b)).collect());
    text.trim_start_matches('\u{feff}').to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quotes_keep_commas_line_breaks_and_quotes_in_a_field() {
        let text =
            "name,date\r\n\"Smith, John\",\"June 3, 2024\"\n\"Say \"\"hi\"\"\",\"two\nlines\"\n";
        assert_eq!(
            records(text, ','),
            [
                vec!["name", "date"],
                vec!["Smith, John", "June 3, 2024"],
                vec!["Say \"hi\"", "two\nlines"],
            ]
        );
    }

    #[test]
    fn a_last_line_without_a_line_break_is_kept() {
        assert_eq!(records("a;b\nc;d", ';'), [vec!["a", "b"], vec!["c", "d"]]);
    }

    #[test]
    fn excel_windows_text_and_byte_order_marks_are_read() {
        assert_eq!(decode(b"Ren\xe9e"), "Renée");
        assert_eq!(decode("\u{feff}José".as_bytes()), "José");
    }
}
