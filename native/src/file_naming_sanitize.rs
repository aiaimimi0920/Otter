//! 按主干的替换合并、120 字符截断和设备名规避规则，增量构建有界 stem。
use super::Error;
#[derive(Default)]
pub(super) struct Stem {
    text: String,
    count: usize,
    replacement: bool,
}
impl Stem {
    pub fn push(&mut self, value: &str) -> Result<(), Error> {
        if value.len() > 65536 {
            return Err(Error::Capacity);
        }
        for ch in value.chars() {
            if self.count >= 120 {
                break;
            }
            if ch.is_control() || matches!(ch, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|')
            {
                if !self.replacement {
                    self.text.push('_');
                    self.count += 1;
                    self.replacement = true;
                }
            } else {
                self.text.push(ch);
                self.count += 1;
                self.replacement = false;
            }
        }
        Ok(())
    }
    pub fn finish(mut self) -> String {
        while self.text.ends_with([' ', '.']) {
            self.text.pop();
        }
        if self.text.is_empty() {
            return "Otter".into();
        }
        let base = self
            .text
            .split('.')
            .next()
            .unwrap_or("")
            .to_ascii_uppercase();
        let reserved = matches!(
            base.as_str(),
            "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$"
        ) || ["COM", "LPT"].iter().any(|prefix| {
            base.strip_prefix(prefix)
                .is_some_and(|suffix| matches!(suffix, "0" | "¹" | "²" | "³"))
                || base
                    .strip_prefix(prefix)
                    .and_then(|suffix| suffix.parse::<u8>().ok())
                    .is_some_and(|n| (1..=9).contains(&n))
        });
        if reserved {
            self.text.insert(0, '_');
            self.text = self.text.chars().take(120).collect();
        }
        self.text
    }
}
