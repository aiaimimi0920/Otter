//! 与主干 Intl zh-CN/base/numeric 对齐；预编码名称，复用系统 ICU 的资源 owner。
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
    // 名称来自 GDI 的固定 LOGFONTW 字段；预编码避免比较热路径反复分配。
    let mut names: Vec<_> = names
        .into_iter()
        .map(|name| (name.encode_utf16().collect::<Vec<_>>(), name))
        .collect();
    // stable sort 保留 base 权重相同项的原顺序，与 JS Array.sort 一致。
    names.sort_by(|(left, _), (right, _)| collator.compare(left, right));
    Ok(names.into_iter().map(|(_, name)| name).collect())
}
