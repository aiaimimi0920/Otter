//! 面板行文字与正常字体的快捷键宽度随快照保存，不在重绘时反复转换。
use super::*;

pub(super) struct PaintRows {
    pub(super) rows: Vec<Row>,
    pub(super) text: Vec<PaintText>,
    shortcut_width: Option<i32>,
}

pub(super) struct PaintText {
    pub(super) full: Vec<u16>,
    tab: Option<usize>,
}
impl PaintText {
    fn new(label: &str) -> Self {
        let full: Vec<_> = label.encode_utf16().collect();
        let tab = full.iter().position(|&unit| unit == u16::from(b'\t'));
        Self { full, tab }
    }
    pub(super) fn parts(&mut self) -> (&mut [u16], &mut [u16]) {
        match self.tab {
            Some(tab) => {
                let (label, rest) = self.full.split_at_mut(tab);
                (label, &mut rest[1..])
            }
            None => (&mut self.full, &mut []),
        }
    }
    fn shortcut(&self) -> &[u16] {
        self.tab.map_or(&[], |tab| &self.full[tab + 1..])
    }
}

impl PaintRows {
    pub(super) fn new(rows: Vec<Row>) -> Self {
        Self {
            text: rows.iter().map(|row| PaintText::new(&row.label)).collect(),
            rows,
            shortcut_width: None,
        }
    }
    pub(super) fn replace(&mut self, rows: Vec<Row>) {
        // 焦点、勾选、缩略图及同文案换代不改变字体度量。
        if self.rows.len() != rows.len()
            || self
                .rows
                .iter()
                .zip(&rows)
                .any(|(old, new)| old.label != new.label)
        {
            *self = Self::new(rows);
        } else {
            self.rows = rows;
        }
    }
    /// 调用方已选入面板的正常字体；面板 DPI 改变时关闭而非复用旧度量。
    pub(super) unsafe fn shortcut_width(&mut self, dc: HDC) -> Option<i32> {
        if let Some(width) = self.shortcut_width {
            return Some(width);
        }
        let mut width = 0;
        for shortcut in self
            .text
            .iter()
            .map(PaintText::shortcut)
            .filter(|text| !text.is_empty())
        {
            let mut size = SIZE::default();
            if !GetTextExtentPoint32W(dc, shortcut, &mut size).as_bool() {
                return None;
            }
            width = width.max(size.cx);
        }
        self.shortcut_width = Some(width);
        Some(width)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(label: &str) -> Row {
        Row {
            key: 0,
            label: label.into(),
            thumbnail: None,
            deletable: false,
            enabled: true,
            checked: false,
            submenu: false,
            separator: false,
            focused: false,
            radio: false,
            default: false,
        }
    }

    #[test]
    fn same_label_state_reuses_encoded_rows_and_changed_label_invalidates_width() {
        let mut frame = PaintRows::new(vec![row("🦦&字\tCtrl+O")]);
        let (label, shortcut) = frame.text[0].parts();
        assert_eq!(String::from_utf16(label).unwrap(), "🦦&字");
        assert_eq!(String::from_utf16(shortcut).unwrap(), "Ctrl+O");
        let buffer = frame.text[0].full.as_ptr();
        frame.shortcut_width = Some(73);
        let mut next = row("🦦&字\tCtrl+O");
        next.focused = true;
        next.checked = true;
        frame.replace(vec![next]);
        assert_eq!(frame.text[0].full.as_ptr(), buffer);
        assert_eq!(frame.shortcut_width, Some(73));
        assert!(frame.rows[0].focused && frame.rows[0].checked);
        frame.replace(vec![row("改名\tAlt+F12"), row("\t")]);
        assert_eq!(frame.shortcut_width, None);
        assert_eq!(
            String::from_utf16(frame.text[0].shortcut()).unwrap(),
            "Alt+F12"
        );
        let (label, shortcut) = frame.text[1].parts();
        assert!(label.is_empty() && shortcut.is_empty());
    }
}
