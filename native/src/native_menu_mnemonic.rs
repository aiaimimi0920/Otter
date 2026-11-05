//! 菜单助记符只读取标签部分；转义 && 是字面符号，不从快捷键列猜测命令。
use super::*;
fn mnemonic(label: &str) -> Option<char> {
    let mut chars = label.split('\t').next()?.chars();
    while let Some(value) = chars.next() {
        if value == '&' {
            let next = chars.next()?;
            if next != '&' {
                return (!next.is_control()).then_some(next);
            }
        }
    }
    None
}
fn character(pending: &mut Option<u16>, unit: u16) -> Option<char> {
    if (0xd800..=0xdbff).contains(&unit) {
        *pending = Some(unit);
        return None;
    }
    let high = pending.take();
    if (0xdc00..=0xdfff).contains(&unit) {
        return char::from_u32(
            0x10000 + (u32::from(high?) - 0xd800) * 1024 + u32::from(unit) - 0xdc00,
        );
    }
    char::from_u32(u32::from(unit)).filter(|value| !value.is_control())
}
impl Cascade {
    /// WM_CHAR/WM_SYSCHAR 为 UTF-16 单元；无效代理项不触发命令。
    pub fn character(&mut self, unit: u16) -> Result<()> {
        self.sync_geometry()?;
        if self.closed || self.pages.is_empty() {
            return Ok(());
        }
        if self.pages.iter().any(|page| !page.panel.alive()) {
            self.cancel();
            return Ok(());
        }
        let Some(value) = character(&mut self.pending_surrogate, unit) else {
            return Ok(());
        };
        let level = self.pages.len() - 1;
        let page = &self.pages[level];
        let matches: Vec<usize> = self
            .items(&page.path)
            .iter()
            .enumerate()
            .filter_map(|(index, item)| {
                (item.enabled
                    && !item.separator
                    && mnemonic(&item.label)
                        .is_some_and(|key| key.to_lowercase().eq(value.to_lowercase())))
                .then_some(index)
            })
            .collect();
        let Some(&first) = matches.first() else {
            return Ok(());
        };
        let index = page
            .focus
            .and_then(|focus| matches.iter().copied().find(|&index| index > focus))
            .unwrap_or(first);
        let result = (|| {
            self.focus(level, Some(index))?;
            if matches.len() == 1 {
                self.activate(level, index)?;
                if !self.closed && self.pages.len() > level + 1 {
                    self.navigate(level + 1, 1)?;
                }
            }
            Ok(())
        })();
        if result.is_err() {
            self.cancel();
        }
        self.sync_accessibility();
        result
    }
}

#[cfg(test)]
#[path = "native_menu_mnemonic_tests.rs"]
mod tests;
