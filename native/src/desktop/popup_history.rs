//! Hold the history session while expanded; paste and delete bind the entry ID used for painting without logging private content.
use crate::{
    clipboard_history::{
        session::{Session, State},
        Content, Entry,
    },
    native_menu::{cascade::Cascade, Item},
};
use windows::core::Result;
const BASE: usize = usize::MAX - 25;
const REMOVE: usize = BASE - 25;
const BATCH: usize = 18;

pub(super) struct History {
    index: usize,
    session: Option<Session>,
    opened: bool,
    ids: Vec<String>,
    loaded: usize,
}
impl History {
    pub(super) fn delete(&mut self, command: usize) -> bool {
        let Some(index) = command.checked_sub(REMOVE).filter(|index| *index < 25) else {
            return false;
        };
        if let Some(id) = self.ids.get(index) {
            if let Some(session) = &mut self.session {
                session.delete_entry(index, id);
            }
        }
        true
    }
    pub(super) fn new(index: usize) -> Self {
        Self {
            index,
            session: None,
            opened: false,
            ids: Vec::new(),
            loaded: BATCH,
        }
    }
    pub(super) fn attach(tree: &mut Vec<Item>) -> usize {
        let mut separator = row(0, String::new(), false);
        separator.separator = true;
        tree.push(separator);
        let index = tree.len();
        let mut parent = row(0, crate::ui_text::text("clipboard-history").into(), true);
        parent.children = Some(vec![status(State::Loading)]);
        tree.push(parent);
        index
    }
    pub(super) fn update(&mut self, menu: &mut Cascade) -> Result<()> {
        if !menu.submenu_open(self.index) {
            self.session = None;
            self.ids.clear();
            self.opened = false;
            self.loaded = BATCH;
            return Ok(());
        }
        let just_opened = !self.opened;
        if just_opened {
            self.opened = true;
            self.session = Session::open().ok();
        }
        let changed = self.session.as_mut().is_some_and(Session::tick);
        self.present(menu, just_opened || changed)
    }
    fn present(&mut self, menu: &mut Cascade, changed: bool) -> Result<()> {
        // Retain painted content during refresh; Session rejects old-ID actions in pending/task states.
        if self
            .session
            .as_ref()
            .is_some_and(|session| session.state() == State::Loading)
            && !self.ids.is_empty()
        {
            return Ok(());
        }
        let more = self.session.as_ref().is_some_and(|session| {
            session.state() == State::Ready
                && self.loaded < session.entries().len()
                && menu.gallery_at_end(self.index)
        });
        if more {
            self.loaded += BATCH;
        }
        let terminal = self
            .session
            .as_ref()
            .is_some_and(|session| session.state() != State::Ready)
            && !self.ids.is_empty();
        if !changed && !more && !terminal {
            return Ok(());
        }
        self.ids.clear();
        let rows = match self.session.as_ref() {
            Some(session) if session.state() == State::Ready => {
                self.ids = session
                    .entries()
                    .iter()
                    .take(self.loaded)
                    .map(|entry| entry.id.clone())
                    .collect();
                session
                    .entries()
                    .iter()
                    .take(self.loaded)
                    .enumerate()
                    .map(|(index, entry)| {
                        let mut item = row(BASE + index, label(&entry.content), true);
                        item.delete = Some(REMOVE + index);
                        item.preview = Some(match &entry.content {
                            Content::Bitmap {
                                thumbnail, preview, ..
                            } => {
                                item.thumbnail = Some(thumbnail.clone());
                                crate::native_menu::panel::Preview::Image(preview.clone())
                            }
                            Content::Text(text) => {
                                crate::native_menu::panel::Preview::Text(text.as_str().into())
                            }
                        });
                        item
                    })
                    .collect()
            }
            session => vec![status(session.map_or(State::Unavailable, Session::state))],
        };
        if self.ids.is_empty() {
            menu.replace_submenu(self.index, rows)
        } else {
            menu.replace_gallery(self.index, rows)
        }
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
        delete: None,
        preview: None,
        thumbnail: None,
    }
}
fn status(state: State) -> Item {
    row(
        0,
        crate::ui_text::text(match state {
            State::DeleteFailed => "clipboard-delete-failed",
            State::Loading => "clipboard-loading",
            State::Empty => "clipboard-empty",
            State::Disabled => "clipboard-disabled",
            State::Denied => "clipboard-denied",
            State::Unavailable => "clipboard-unavailable",
            State::Ready => "clipboard-empty",
        })
        .into(),
        false,
    )
}
fn label(content: &Content) -> String {
    match content {
        Content::Bitmap { width, height, .. } => crate::ui_text::format(
            "clipboard-image-size",
            &[("width", (*width).into()), ("height", (*height).into())],
        ),
        Content::Text(text) => {
            let mut preview = String::new();
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
            crate::ui_text::format("clipboard-text-preview", &[("preview", preview.into())])
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
            loaded: BATCH,
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
            crate::ui_text::format(
                "clipboard-text-preview",
                &[("preview", "&&A 中文 B".into())]
            )
        );
        let bounded = crate::ui_text::format(
            "clipboard-text-preview",
            &[("preview", format!("{}…", "😀".repeat(64)).into())],
        );
        assert_eq!(label(&Content::Text("😀".repeat(100))), bounded);
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
        assert_eq!(
            items[index].label,
            crate::ui_text::text("clipboard-history")
        );
        assert!(items[index].children.is_some());
    }
    #[test]
    fn history_appends_snapshot_entries_and_keeps_old_view_until_refresh_finishes() {
        use windows::{
            core::w,
            Win32::{
                Foundation::{HWND, POINT},
                UI::WindowsAndMessaging::*,
            },
        };
        struct Owner(HWND);
        impl Drop for Owner {
            fn drop(&mut self) {
                unsafe {
                    let _ = DestroyWindow(self.0);
                }
            }
        }
        let owner = Owner(unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                w!("STATIC"),
                w!("history test"),
                WS_OVERLAPPED,
                0,
                0,
                100,
                100,
                None,
                None,
                None,
                None,
            )
            .unwrap()
        });
        let mut tree = Vec::new();
        let root = History::attach(&mut tree);
        let mut menu = Cascade::open(owner.0, tree, POINT { x: 30, y: 30 }).unwrap();
        let snapshot = |suffix: &str| {
            Session::test_snapshot(crate::clipboard_history::Snapshot {
                status: crate::clipboard_history::Status::Ready,
                entries: (0..25)
                    .map(|n| Entry {
                        id: format!("{n}-{suffix}"),
                        content: Content::Text(format!("same label {n}")),
                    })
                    .collect(),
            })
        };
        let mut history = History::new(root);
        history.opened = true;
        history.session = Some(snapshot("before"));
        history.present(&mut menu, true).unwrap();
        assert_eq!(history.ids.len(), BATCH);
        menu.key(0x28).unwrap();
        menu.key(0x0d).unwrap();
        while !menu.gallery_at_end(root) {
            menu.key(0x28).unwrap();
        }
        history.present(&mut menu, false).unwrap();
        assert_eq!(history.ids.len(), 25);
        let view = menu.gallery_view(root).unwrap();
        history.session.as_ref().unwrap().test_invalidate();
        assert!(history.take(BASE).is_none());
        assert_eq!(history.session.as_ref().unwrap().state(), State::Loading);
        history.present(&mut menu, true).unwrap();
        assert_eq!(history.ids[0], "0-before");
        assert_eq!(menu.gallery_view(root).unwrap(), view);
        assert!(!history
            .session
            .as_mut()
            .unwrap()
            .delete_entry(0, "0-before"));
        history.session = Some(snapshot("after"));
        history.present(&mut menu, true).unwrap();
        assert_eq!(history.ids[0], "0-after");
        assert_eq!(menu.gallery_view(root).unwrap(), view);
        history.present(&mut menu, false).unwrap();
        assert_eq!(history.take(BASE + 24).unwrap().id, "24-after");
    }
}
