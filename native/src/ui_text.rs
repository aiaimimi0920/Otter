//! Embedded interface messages with an English source contract and explicit fallback.

pub use fluent_bundle::FluentValue;
pub use otter_core::display_language::DisplayLanguage;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::OnceLock;

#[path = "ui_text_catalog.rs"]
mod catalog;
#[path = "ui_text_contract.rs"]
mod contract;

static LANGUAGE: AtomicU8 = AtomicU8::new(DisplayLanguage::English as u8);
static CATALOGS: OnceLock<catalog::Catalogs> = OnceLock::new();

fn catalogs() -> &'static catalog::Catalogs {
    CATALOGS.get_or_init(|| {
        catalog::Catalogs::embedded()
            .expect("embedded interface messages must satisfy their contract")
    })
}

pub fn current_language() -> DisplayLanguage {
    match LANGUAGE.load(Ordering::Relaxed) {
        0 => DisplayLanguage::English,
        1 => DisplayLanguage::SimplifiedChinese,
        _ => unreachable!("only DisplayLanguage discriminants are stored"),
    }
}

/// The Application owner persists settings and refreshes windows around this call.
pub fn set_language(language: DisplayLanguage) {
    catalogs();
    LANGUAGE.store(language as u8, Ordering::Relaxed);
}

#[track_caller]
pub fn text(id: &str) -> &'static str {
    text_for(current_language(), id)
}

/// Borrow from immutable, once-initialized storage rather than leaking allocations.
#[track_caller]
pub fn text_for(language: DisplayLanguage, id: &str) -> &'static str {
    catalogs().text(language, id)
}

#[track_caller]
pub fn format(id: &str, arguments: &[(&str, FluentValue<'_>)]) -> String {
    format_for(current_language(), id, arguments)
}

#[track_caller]
pub fn format_for(
    language: DisplayLanguage,
    id: &str,
    arguments: &[(&str, FluentValue<'_>)],
) -> String {
    catalogs().format(language, id, arguments)
}

#[cfg(test)]
#[path = "ui_text_source_tests.rs"]
mod source_tests;
#[cfg(test)]
#[path = "ui_text_tests.rs"]
mod tests;
