//! Guard literal message references without treating user values or protocol strings as message IDs.
use super::catalog::Catalogs;
use std::path::Path;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Token<'a> {
    Word(&'a str),
    Text(&'a str),
    Punct(u8),
}

// Only lexical structure is needed: comments and quoted contents cannot create calls.
// Escaped and raw message IDs are deliberately not accepted as literal references.
fn tokens(source: &str) -> Vec<Token<'_>> {
    let bytes = source.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i..].starts_with(b"//") {
            i += bytes[i..]
                .iter()
                .position(|b| *b == b'\n')
                .unwrap_or(bytes.len() - i);
        } else if bytes[i..].starts_with(b"/*") {
            let mut depth = 1;
            i += 2;
            while depth > 0 && i < bytes.len() {
                if bytes[i..].starts_with(b"/*") {
                    depth += 1;
                    i += 2;
                } else if bytes[i..].starts_with(b"*/") {
                    depth -= 1;
                    i += 2;
                } else {
                    i += 1;
                }
            }
        } else if bytes[i] == b'r' && bytes.get(i + 1).is_some_and(|b| matches!(b, b'#' | b'"')) {
            let start = i;
            i += 1;
            while bytes.get(i) == Some(&b'#') {
                i += 1;
            }
            if bytes.get(i) == Some(&b'"') {
                let suffix = format!("\"{}", "#".repeat(i - start - 1));
                i += 1;
                i += source[i..]
                    .find(&suffix)
                    .map_or(bytes.len() - i, |end| end + suffix.len());
            }
        } else if bytes[i] == b'\'' {
            let start = i + 1;
            let end = if bytes.get(start) == Some(&b'\\') {
                start + 2
            } else {
                source[start..]
                    .chars()
                    .next()
                    .map_or(start, |value| start + value.len_utf8())
            };
            if bytes.get(end) == Some(&b'\'') {
                i = end + 1;
            } else {
                i += 1; // A lifetime is not a quoted character.
            }
        } else if bytes[i] == b'"' {
            let start = i + 1;
            i += 1;
            let mut escaped = false;
            while i < bytes.len() && bytes[i] != b'"' {
                if bytes[i] == b'\\' {
                    escaped = true;
                    i += 1;
                }
                i += 1;
            }
            if !escaped {
                out.push(Token::Text(&source[start..i]));
            }
            i = (i + 1).min(bytes.len());
        } else if bytes[i].is_ascii_alphabetic() || bytes[i] == b'_' {
            let start = i;
            i += 1;
            while bytes
                .get(i)
                .is_some_and(|b| b.is_ascii_alphanumeric() || *b == b'_')
            {
                i += 1;
            }
            out.push(Token::Word(&source[start..i]));
        } else {
            if !bytes[i].is_ascii_whitespace() {
                out.push(Token::Punct(bytes[i]));
            }
            i += 1;
        }
    }
    out
}

fn references(source: &str) -> Vec<(&str, bool)> {
    use Token::{Punct as P, Text, Word as W};
    let tokens = tokens(source);
    let mut references = Vec::new();
    for i in 0..tokens.len() {
        let (mut argument, static_only) = match &tokens[i..] {
            [W("ui_text"), P(b':'), P(b':'), W(name @ ("text" | "format" | "text_for" | "format_for")), P(b'('), ..] =>
            {
                let mut argument = i + 5;
                if name.ends_with("_for") {
                    while argument < tokens.len() && tokens[argument] != P(b',') {
                        argument += 1;
                    }
                    argument += 1;
                }
                (argument, name.starts_with("text"))
            }
            [W("Caption"), P(b':'), P(b':'), W(name @ ("plain" | "checked" | "message")), P(b'('), ..] => {
                (i + 5, *name != "message")
            }
            [P(b'.'), W("localized_error"), P(b'('), ..] => (i + 3, true),
            _ => continue,
        };
        // Parenthesized literals are equivalent to direct literal arguments.
        while tokens.get(argument) == Some(&P(b'(')) {
            argument += 1;
        }
        if let Some(Text(id)) = tokens.get(argument) {
            references.push((*id, static_only));
        }
    }
    references
}

fn inspect_tree(path: &Path, catalogs: &Catalogs, count: &mut usize) {
    for entry in std::fs::read_dir(path).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if entry.file_type().unwrap().is_dir() {
            inspect_tree(&path, catalogs, count);
        } else if path.extension().is_some_and(|extension| extension == "rs")
            && !path
                .file_stem()
                .unwrap()
                .to_string_lossy()
                .ends_with("_tests")
        {
            let source = std::fs::read_to_string(&path).unwrap();
            for (id, static_only) in references(&source) {
                let parameters = catalogs.source_parameters(id).unwrap_or_else(|| {
                    panic!("{} references missing message {id}", path.display())
                });
                assert!(
                    !static_only || parameters.is_empty(),
                    "{} reads parameterized message {id} as a static label",
                    path.display()
                );
                *count += 1;
            }
        }
    }
}

#[test]
fn literal_ui_message_references_exist_and_static_lookups_have_no_arguments() {
    let catalogs = Catalogs::embedded().unwrap();
    let mut count = 0;
    inspect_tree(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("src"),
        &catalogs,
        &mut count,
    );
    assert!(
        count > 300,
        "the guard must inspect the actual native UI source tree"
    );
}

#[test]
fn reference_scanner_ignores_comments_raw_strings_and_unrelated_values() {
    let source = r###"
        // ui_text::text("ignored-comment")
        /* /* nested */ ui_text::text("ignored-block") */
        let raw = r#"ui_text::text("ignored-raw")"#;
        let ordinary = "ui_text::text(\"ignored-quoted\")";
        crate::ui_text::text("common-save");
        ui_text::format("parameter-incomplete", &[]);
        ui_text::text_for(language, "common-close");
        Caption::plain("parameter-title");
        Caption::message("parameter-incomplete", &[]);
        ui.localized_error("parameter-queue-full");
    "###;
    assert_eq!(
        references(source),
        vec![
            ("common-save", true),
            ("parameter-incomplete", false),
            ("common-close", true),
            ("parameter-title", true),
            ("parameter-incomplete", false),
            ("parameter-queue-full", true),
        ]
    );
}
