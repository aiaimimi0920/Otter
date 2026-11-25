//! Match baseline Intl zh-CN/base/numeric ordering; preencode names and reuse system ICU resource ownership.
use windows::{
    core::{s, Result},
    Win32::Globalization::*,
};
pub(super) fn sorted(names: Vec<String>) -> Result<Vec<String>> {
    let collator = super::super::text_collation::Collator::new(
        s!("zh_CN"),
        &[
            (UCOL_STRENGTH, UCOL_PRIMARY),
            (UCOL_NUMERIC_COLLATION, UCOL_ON),
            (UCOL_CASE_LEVEL, UCOL_OFF),
            (UCOL_NORMALIZATION_MODE, UCOL_ON),
            (UCOL_ALTERNATE_HANDLING, UCOL_NON_IGNORABLE),
            (UCOL_CASE_FIRST, UCOL_OFF),
        ],
    )?;
    // Names come from fixed GDI LOGFONTW fields; preencoding avoids repeated allocations in comparison hot paths.
    let mut names: Vec<_> = names
        .into_iter()
        .map(|name| (name.encode_utf16().collect::<Vec<_>>(), name))
        .collect();
    // stable sort preserves original order for equal base weights, matching JS Array.sort.
    names.sort_by(|(left, _), (right, _)| collator.compare(left, right));
    Ok(names.into_iter().map(|(_, name)| name).collect())
}
