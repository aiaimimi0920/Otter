//! 展开期间持有只读历史会话；命令映射绑定绘制时的条目 ID，不记录私有内容。
use crate::{
    clipboard_history::{
        session::{Session, State},
        Content, Entry,
    },
    native_menu::{cascade::Cascade, Item},
};
use windows::core::Result;
const BASE: usize = usize::MAX - 25;

pub(super) struct History {
    index: usize,
    session: Option<Session>,
    opened: bool,
    ids: Vec<String>,
}
impl History {
    pub(super) fn new(index: usize) -> Self {
        Self {
            index,
            session: None,
            opened: false,
            ids: Vec::new(),
        }
    }
    pub(super) fn attach(tree: &mut Vec<Item>) -> usize {
        let mut separator = row(0, String::new(), false);
        separator.separator = true;
        tree.push(separator);
        let index = tree.len();
        let mut parent = row(0, "剪贴板".into(), true);
        parent.children = Some(vec![status(State::Loading)]);
        tree.push(parent);
        index
    }
    pub(super) fn update(&mut self, menu: &mut Cascade) -> Result<()> {
        if !menu.submenu_open(self.index) {
            self.session = None;
            self.ids.clear();
            self.opened = false;
            return Ok(());
        }
        let just_opened = !self.opened;
        if just_opened {
            self.opened = true;
            self.session = Session::open().ok();
        }
        let changed = self.session.as_mut().is_some_and(Session::tick);
        if !just_opened && !changed {
            return Ok(());
        }
        self.ids.clear();
        let rows = match self.session.as_ref() {
            Some(session) if session.state() == State::Ready => {
                self.ids = session
                    .entries()
                    .iter()
                    .map(|entry| entry.id.clone())
                    .collect();
                session
                    .entries()
                    .iter()
                    .enumerate()
                    .map(|(index, entry)| {
                        let mut item = row(BASE + index, label(&entry.content), true);
                        if let Content::Bitmap { thumbnail, .. } = &entry.content {
                            item.thumbnail = Some(thumbnail.clone());
                        }
                        item
                    })
                    .collect()
            }
            session => vec![status(session.map_or(State::Unavailable, Session::state))],
        };
        menu.replace_submenu(self.index, rows)
    }
    pub(super) fn take(&mut self, command: usize) -> Option<Entry> {
        let index = command.checked_sub(BASE)?;
        let id = self.ids.get(index)?;
        self.session.as_mut()?.take_entry(index, id)
    }
    pub(super) fn is_history(command: usize) -> bool {
        (BASE..BASE + 25).contains(&command)
    }
}
fn row(command: usize, label: String, enabled: bool) -> Item {
    Item {
        command,
        label,
        enabled,
        checked: false,
        radio: false,
        default: false,
        separator: false,
        children: None,
        thumbnail: None,
    }
}
fn status(state: State) -> Item {
    row(
        0,
        match state {
            State::Loading => "正在读取历史…",
            State::Empty => "没有可用的历史内容",
            State::Disabled => "Windows 剪贴板历史未启用",
            State::Denied => "Windows 拒绝访问剪贴板历史",
            State::Unavailable => "暂时无法读取剪贴板历史",
            State::Ready => "没有可用的历史内容",
        }
        .into(),
        false,
    )
}
fn label(content: &Content) -> String {
    match content {
        Content::Bitmap { width, height, .. } => format!("图片 {width} × {height}"),
        Content::Text(text) => {
            let mut preview = String::from("文字：");
            for ch in text.chars().take(64) {
                match ch {
                    '&' => preview.push_str("&&"),
                    ch if ch.is_control() => preview.push(' '),
                    ch => preview.push(ch),
                }
            }
            if text.chars().nth(64).is_some() {
                preview.push('…');
            }
            preview
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn ready() -> History {
        History {
            index: 0,
            session: Some(Session::test_snapshot(crate::clipboard_history::Snapshot {
                status: crate::clipboard_history::Status::Ready,
                entries: vec![Entry {
                    id: "painted-id".into(),
                    content: Content::Text("private fixture".into()),
                }],
            })),
            opened: true,
            ids: vec!["painted-id".into()],
        }
    }
    #[test]
    fn selection_requires_painted_identity_and_rechecks_changes_then_moves_once() {
        let mut history = ready();
        assert!(history.take(BASE + 1).is_none());
        assert!(history.take(BASE).is_some());
        assert!(history.take(BASE).is_none());
        let mut history = ready();
        history.ids[0] = "old-id".into();
        assert!(history.take(BASE).is_none());
        let mut history = ready();
        history.session.as_ref().unwrap().test_invalidate();
        assert!(history.take(BASE).is_none());
    }
    #[test]
    fn preview_cannot_inject_mnemonics_shortcuts_or_unbounded_labels() {
        assert_eq!(
            label(&Content::Text("&A\t中文\nB".into())),
            "文字：&&A 中文 B"
        );
        assert_eq!(label(&Content::Text("😀".repeat(100))).chars().count(), 68);
    }
    #[test]
    fn placeholder_is_disabled_and_commands_do_not_alias_existing_u32_ids() {
        assert!(!status(State::Denied).enabled);
        assert!(!History::is_history(u32::MAX as usize));
        assert!(History::is_history(BASE));
        assert!(!History::is_history(usize::MAX));
        assert!(History::new(0).take(BASE).is_none());
    }
    #[test]
    fn history_follows_existing_items_and_separator_without_renumbering_commands() {
        let mut items = vec![row(42, "原命令".into(), true)];
        let index = History::attach(&mut items);
        assert_eq!(index, 2);
        assert_eq!(items[0].command, 42);
        assert!(items[1].separator);
        assert_eq!(items[index].label, "剪贴板");
        assert!(items[index].children.is_some());
    }
}
