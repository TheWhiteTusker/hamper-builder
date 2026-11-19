//! CSV as the old app wrote and read it with PapaParse: `\r\n` rows, fields
//! quoted only when they must be, blank lines skipped on read.

use crate::import::Grid;

fn needs_quotes(s: &str) -> bool {
    s.contains([',', '"', '\r', '\n']) || s.starts_with(' ') || s.ends_with(' ')
}

/// `Papa.unparse({ fields, data })`.
pub fn to_csv<S: AsRef<str>>(rows: &[Vec<S>]) -> String {
    rows.iter()
        .map(|row| {
            row.iter()
                .map(|c| {
                    let c = c.as_ref();
                    if needs_quotes(c) { format!("\"{}\"", c.replace('"', "\"\"")) } else { c.to_string() }
                })
                .collect::<Vec<_>>()
                .join(",")
        })
        .collect::<Vec<_>>()
        .join("\r\n")
}

/// `Papa.parse(text, { skipEmptyLines: "greedy" })`: quoted fields may hold
/// commas, quotes ("") and line breaks; rows of only whitespace are dropped.
pub fn parse_csv(text: &str) -> Grid {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let (mut rows, mut row, mut field) = (Vec::new(), Vec::new(), String::new());
    let (mut quoted, mut chars) = (false, text.chars().peekable());
    let end_row = |row: &mut Vec<String>, field: &mut String, rows: &mut Grid| {
        row.push(std::mem::take(field));
        let r = std::mem::take(row);
        if r.iter().any(|c| !c.trim().is_empty()) {
            rows.push(r);
        }
    };
    while let Some(c) = chars.next() {
        match (quoted, c) {
            (true, '"') if chars.peek() == Some(&'"') => {
                chars.next();
                field.push('"');
            }
            (true, '"') => quoted = false,
            (true, c) => field.push(c),
            (false, '"') if field.is_empty() => quoted = true,
            (false, ',') => row.push(std::mem::take(&mut field)),
            (false, '\r') if chars.peek() == Some(&'\n') => {}
            (false, '\n' | '\r') => end_row(&mut row, &mut field, &mut rows),
            (false, c) => field.push(c),
        }
    }
    if !field.is_empty() || !row.is_empty() {
        end_row(&mut row, &mut field, &mut rows);
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quotes_only_when_needed() {
        let rows = vec![vec!["Code", "Name"], vec!["LC/0001/WL", "Lamp, \"tall\""], vec!["X", " padded"]];
        assert_eq!(to_csv(&rows), "Code,Name\r\nLC/0001/WL,\"Lamp, \"\"tall\"\"\"\r\nX,\" padded\"");
    }

    #[test]
    fn reads_back_what_it_writes() {
        let rows = vec![vec!["a".to_string(), "b,c".into()], vec!["line\nbreak".into(), "\"q\"".into()]];
        assert_eq!(parse_csv(&to_csv(&rows)), rows);
    }

    #[test]
    fn skips_blank_lines_and_the_bom() {
        assert_eq!(parse_csv("\u{feff}a,b\n\n , \nc,d\n"), vec![vec!["a", "b"], vec!["c", "d"]]);
    }
}
