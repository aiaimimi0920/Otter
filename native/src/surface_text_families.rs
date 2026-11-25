//! Sole bounded CSS font-list parsing/selection owner for native families; no network fonts or full enumeration.
use std::{collections::BTreeMap, rc::Rc};
use windows::{
    core::{Result, HSTRING},
    Win32::Graphics::DirectWrite::{IDWriteFactory, IDWriteFontCollection},
};

#[derive(Default)]
pub(super) struct Families {
    collection: Option<IDWriteFontCollection>,
    selected: BTreeMap<String, Rc<str>>,
}
struct Family {
    name: String,
    generic: bool,
}
fn name_character(ch: char) -> bool {
    ch.is_ascii_alphanumeric()
        || matches!(ch, ' ' | '_' | '-')
        || ('\u{3400}'..='\u{9fff}').contains(&ch)
}
fn generic(name: &str) -> Option<&'static [&'static str]> {
    match name {
        "sans-serif" | "system-ui" => Some(&["Segoe UI", "Arial"]),
        "serif" => Some(&["Times New Roman", "Georgia"]),
        "monospace" => Some(&["Consolas", "Courier New"]),
        _ => None,
    }
}
// Share CSS admission contracts: at most 8 entries/256 UTF-8 bytes, 31 UTF-16 units per entry, compatible with GDI faces.
fn parse(source: &str) -> Result<Vec<Family>> {
    if source.is_empty() || source.len() > 256 {
        return Err(super::invalid());
    }
    let mut output = Vec::new();
    for item in source.split(',') {
        if output.len() >= 8 {
            return Err(super::invalid());
        }
        let item = item.trim_matches(|c: char| c.is_ascii_whitespace());
        let quoted = item.starts_with(['\'', '"']);
        let name = if quoted {
            if item.len() < 3 || !item.ends_with(item.chars().next().unwrap()) {
                return Err(super::invalid());
            }
            item[1..item.len() - 1].to_owned()
        } else {
            item.split_ascii_whitespace().collect::<Vec<_>>().join(" ")
        };
        if name.is_empty() || name.encode_utf16().count() > 31 || !name.chars().all(name_character)
        {
            return Err(super::invalid());
        }
        let lower = name.to_ascii_lowercase();
        if !quoted
            && (matches!(
                lower.as_str(),
                "inherit"
                    | "initial"
                    | "unset"
                    | "revert"
                    | "revert-layer"
                    | "default"
                    | "cursive"
                    | "fantasy"
                    | "ui-serif"
                    | "ui-sans-serif"
                    | "ui-monospace"
                    | "ui-rounded"
                    | "emoji"
                    | "math"
                    | "fangsong"
            ) || name
                .split(' ')
                .any(|word| word.starts_with(|c: char| c.is_ascii_digit() || c == '-')))
        {
            return Err(super::invalid());
        }
        let is_generic = !quoted && generic(&lower).is_some();
        output.push(Family {
            name: if is_generic { lower } else { name },
            generic: is_generic,
        });
    }
    Ok(output)
}
impl Families {
    pub(super) fn resolve(&mut self, factory: &IDWriteFactory, source: &str) -> Result<Rc<str>> {
        if let Some(found) = self.selected.get(source) {
            return Ok(found.clone());
        }
        let families = parse(source)?;
        // Scenes without fontFamily do not query collections; pin this Engine's installed-font view without updates/downloads.
        if self.collection.is_none() {
            unsafe { factory.GetSystemFontCollection(&mut self.collection, false)? };
        }
        let collection = self.collection.as_ref().ok_or_else(super::invalid)?;
        let installed = |name: &str| -> Result<bool> {
            let mut index = 0;
            let mut exists = Default::default();
            unsafe { collection.FindFamilyName(&HSTRING::from(name), &mut index, &mut exists)? };
            Ok(exists.as_bool())
        };
        let mut chosen = None;
        for family in &families {
            let single = [family.name.as_str()];
            let names = if family.generic {
                generic(&family.name).unwrap()
            } else {
                &single
            };
            for name in names {
                if installed(name)? {
                    chosen = Some(Rc::<str>::from(*name));
                    break;
                }
            }
            if chosen.is_some() {
                break;
            }
        }
        // If all are missing, use the CSS default sans-serif; never let DWrite/GDI separately guess from comma-separated lists.
        if chosen.is_none() {
            for name in generic("sans-serif").unwrap() {
                if installed(name)? {
                    chosen = Some(Rc::<str>::from(*name));
                    break;
                }
            }
        }
        let chosen = chosen.ok_or_else(super::invalid)?;
        if self.selected.len() == 64 {
            self.selected.clear();
        }
        self.selected.insert(source.to_owned(), chosen.clone());
        Ok(chosen)
    }
}

#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-surface")))]
#[path = "surface_text_families_tests.rs"]
mod tests;
