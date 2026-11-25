//! when 词法边界与主干的字符串转义、ECMAScript 空白及 UTF-16 字符预算一致。
use super::Error;
#[derive(Debug, PartialEq)]
pub(super) enum Token {
    Identifier(String),
    Text(String),
    Bool(bool),
    Operator(&'static str),
    End,
}
impl Token {
    pub(super) fn value(&self) -> &str {
        match self {
            Self::Identifier(s) | Self::Text(s) => s,
            Self::Bool(true) => "true",
            Self::Bool(false) => "false",
            Self::Operator(s) => s,
            Self::End => "",
        }
    }
}
pub(super) fn space(c: char) -> bool {
    matches!(c, '\t'..='\r' | ' ' | '\u{a0}' | '\u{1680}' | '\u{2000}'..='\u{200a}'
        | '\u{2028}' | '\u{2029}' | '\u{202f}' | '\u{205f}' | '\u{3000}' | '\u{feff}')
}
pub(super) fn tokenize(source: &str) -> Result<Vec<Token>, Error> {
    if source.encode_utf16().nth(2048).is_some() {
        return Err(Error::Budget);
    }
    let mut chars = source.chars().peekable();
    let mut tokens = Vec::new();
    while let Some(c) = chars.next() {
        if space(c) {
            continue;
        }
        let token = match c {
            '\'' | '"' => {
                let mut text = String::new();
                loop {
                    let next = chars.next().ok_or(Error::Token)?;
                    if next == c {
                        break;
                    }
                    if next == '\\' {
                        let escaped = chars.next().ok_or(Error::Token)?;
                        if escaped != c && escaped != '\\' {
                            return Err(Error::Token);
                        }
                        text.push(escaped);
                    } else {
                        text.push(next);
                    }
                }
                Token::Text(text)
            }
            'a'..='z' | 'A'..='Z' | '_' => {
                let mut id = String::from(c);
                while chars
                    .peek()
                    .is_some_and(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-'))
                {
                    id.push(chars.next().unwrap());
                }
                match id.as_str() {
                    "true" => Token::Bool(true),
                    "false" => Token::Bool(false),
                    _ => Token::Identifier(id),
                }
            }
            '(' => Token::Operator("("),
            ')' => Token::Operator(")"),
            ',' => Token::Operator(","),
            '!' | '=' | '&' | '|' => {
                let operator = match (c, chars.peek()) {
                    ('!', Some('=')) => "!=",
                    ('=', Some('=')) => "==",
                    ('&', Some('&')) => "&&",
                    ('|', Some('|')) => "||",
                    ('!', _) => "!",
                    _ => return Err(Error::Token),
                };
                if operator.len() == 2 {
                    chars.next();
                }
                Token::Operator(operator)
            }
            _ => return Err(Error::Token),
        };
        tokens.push(token);
        if tokens.len() > 256 {
            return Err(Error::Budget);
        }
    }
    tokens.push(Token::End);
    Ok(tokens)
}
