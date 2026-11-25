//! 两个根级冻结库滚到底逐批追加；命令绑定不可变来源与全库条目身份。
use crate::{
    desktop::loom_execution_signal::Notify,
    native_menu::{cascade::Cascade, Item},
    session_library::Kind,
    session_source::Source,
};
use windows::core::Result;
#[path = "popup_frozen_index.rs"]
mod index;
#[path = "popup_frozen_preview.rs"]
mod preview;
#[path = "popup_frozen_remove.rs"]
mod remove;
#[cfg(any(test, debug_assertions, feature = "smoke-test"))]
#[path = "popup_frozen_smoke.rs"]
pub(in crate::desktop) mod smoke;
#[path = "popup_frozen_worker.rs"]
mod worker;
use super::super::frozen_library_ui::Choice;
const BATCH: usize = 64;
const CAPACITY: usize = crate::session_library::MAX_ENTRIES;
const BASE: usize = usize::MAX - 32768;
const REMOVE: usize = BASE + CAPACITY * 2;
#[derive(Default, Clone)]
pub(in crate::desktop) struct Workers {
    index: worker::Slot<index::Index>,
    previews: worker::Slot<preview::Images>,
    pub(in crate::desktop) notify: Option<Notify>,
}
impl Workers {
    pub(in crate::desktop) fn reap(&self) {
        self.index.reap();
        self.previews.reap();
    }
    pub(in crate::desktop) fn deadline(&self) -> Option<std::time::Instant> {
        self.index
            .deadline()
            .into_iter()
            .chain(self.previews.deadline())
            .min()
    }
}
pub(super) struct LibraryMenu {
    previews: preview::Previews,
    thumbnail_slots: [Vec<usize>; 2],
    roots: [usize; 2],
    source: Option<Source>,
    loaded: [usize; 2],
    data: Option<index::Index>,
    job: Option<index::Job>,
    started: bool,
    reference: Option<(usize, String, bool)>,
    removing: bool,
    reload_pending: bool,
    clear: [Option<(usize, bool)>; 2],
    workers: Workers,
}
impl LibraryMenu {
    pub fn attach(tree: &mut Vec<Item>, source: Option<Source>, workers: Workers) -> Self {
        use super::super::reference_ui::{
            CLEAR_RECYCLE, CLEAR_REFERENCE, RECYCLE_ROOT, REFERENCE_ROOT,
        };
        let clear = [CLEAR_RECYCLE, CLEAR_REFERENCE].map(|command| {
            tree.iter()
                .position(|item| item.command == command)
                .map(|index| (index, tree[index].enabled))
        });
        let mut roots = [0; 2];
        for (library, (command, label)) in [(RECYCLE_ROOT, "回收站"), (REFERENCE_ROOT, "收藏夹")]
            .into_iter()
            .enumerate()
        {
            let mut parent = row(0, label.into(), true);
            parent.children = Some(vec![row(
                0,
                if source.is_some() {
                    "展开后读取…"
                } else {
                    "当前会话暂不可用"
                }
                .into(),
                false,
            )]);
            if let Some(index) = tree.iter().position(|item| item.command == command) {
                roots[library] = index;
                parent.enabled = tree[index].enabled;
                tree[index] = parent;
            } else {
                roots[library] = tree.len();
                tree.push(parent);
            }
        }
        Self {
            previews: preview::Previews::new(workers.previews.clone(), workers.notify.clone()),
            thumbnail_slots: Default::default(),
            roots,
            source,
            loaded: [BATCH; 2],
            data: None,
            job: None,
            started: false,
            reference: None,
            removing: false,
            reload_pending: false,
            clear,
            workers,
        }
    }
    pub fn bind_reference(
        &mut self,
        tree: &[Item],
        unit: &crate::workspace_id::UnitId,
        allowed: bool,
    ) {
        if let Some(index) = tree
            .iter()
            .position(|item| item.command == super::super::reference_ui::SET)
        {
            self.reference = Some((index, unit.as_str().into(), allowed));
        }
    }
    pub fn update(&mut self, menu: &mut Cascade) -> Result<()> {
        if self.reload_pending {
            // The durable removal has already published; retry admission, not the transaction.
            self.job = self.start_index().map_err(|error| {
                windows::core::Error::new(windows::Win32::Foundation::E_FAIL, error.to_string())
            })?;
            self.reload_pending = self.job.is_none();
        }
        if !self.started
            && (self.reference.is_some() || self.roots.iter().any(|root| menu.submenu_open(*root)))
        {
            if self.source.is_some() {
                match self.start_index() {
                    Ok(Some(job)) => self.job = Some(job),
                    Ok(None) => return Ok(()),
                    Err(_) => {
                        self.data = Some([Err("Unavailable".into()), Err("Unavailable".into())])
                    }
                }
            }
            self.started = true;
            for library in 0..2 {
                self.replace(menu, library)?;
            }
            self.update_reference(menu)?;
        }
        if let Some(data) = self.job.as_mut().and_then(index::Job::poll) {
            self.job = None;
            self.data = Some(data);
            self.removing = false;
            for library in 0..2 {
                self.replace(menu, library)?;
            }
            self.update_reference(menu)?;
        }
        if !self.removing {
            for library in 0..2 {
                let Some(source) = self.source.as_ref() else {
                    break;
                };
                let Some(view) = menu.gallery_view(self.roots[library]) else {
                    continue;
                };
                let count = self.entries(library).map_or(0, Vec::len);
                let more = menu.gallery_at_end(self.roots[library]) && self.loaded[library] < count;
                if more {
                    self.loaded[library] = (self.loaded[library] + BATCH).min(count);
                }
                let images = self.previews.update(source, library, view, count);
                if more {
                    self.replace(menu, library)?;
                } else if images {
                    self.replace_thumbnails(menu, library)?;
                }
            }
        }
        Ok(())
    }
    fn start_index(&self) -> std::io::Result<Option<index::Job>> {
        index::Job::start(
            &self.workers.index,
            self.source
                .as_ref()
                .expect("index requires a source")
                .clone(),
            self.workers.notify.clone(),
        )
    }
    fn replace(&mut self, menu: &mut Cascade, library: usize) -> Result<()> {
        let rows = self.rows(library);
        let slots = &mut self.thumbnail_slots[library];
        slots.clear();
        slots.extend(
            rows.iter()
                .enumerate()
                .filter_map(|(slot, row)| row.thumbnail.as_ref().map(|_| slot)),
        );
        if self
            .entries(library)
            .is_some_and(|entries| !entries.is_empty())
        {
            menu.replace_gallery(self.roots[library], rows)
        } else {
            menu.replace_submenu(self.roots[library], rows)
        }
    }
    fn replace_thumbnails(&mut self, menu: &mut Cascade, library: usize) -> Result<()> {
        let entries = self.entries(library).expect("gallery requires an index");
        let loaded = self.loaded[library].min(entries.len());
        let images: Vec<_> = self
            .previews
            .batch(library)
            .filter(|&(slot, id, _)| slot < loaded && entries[slot].id == id)
            .map(|(slot, _, image)| (slot, image))
            .collect();
        let slots = &mut self.thumbnail_slots[library];
        menu.replace_gallery_thumbnails(self.roots[library], slots, &images)?;
        slots.clear();
        slots.extend(images.into_iter().map(|(slot, _)| slot));
        Ok(())
    }
    fn update_reference(&self, menu: &mut Cascade) -> Result<()> {
        use super::super::reference_ui::{CLEAR_RECYCLE, CLEAR_REFERENCE};
        for (library, (command, label)) in [
            (CLEAR_RECYCLE, "清空回收站"),
            (CLEAR_REFERENCE, "清空收藏夹"),
        ]
        .into_iter()
        .enumerate()
        {
            if let Some((index, allowed)) = self.clear[library] {
                menu.replace_action(index, command, label, allowed && !self.removing)?;
            }
        }
        let Some((index, _, _)) = &self.reference else {
            return Ok(());
        };
        let (command, label, enabled) = self.reference_action();
        menu.replace_action(*index, command, label, enabled)
    }
    fn reference_action(&self) -> (usize, &'static str, bool) {
        use super::super::reference_ui::{REMOVE, SET};
        let Some((_, unit, allowed)) = &self.reference else {
            return (SET, "收藏", false);
        };
        let entries = self.entries(1);
        let referenced =
            entries.is_some_and(|entries| entries.iter().any(|entry| &entry.source_unit == unit));
        (
            if referenced { REMOVE } else { SET },
            if referenced { "取消收藏" } else { "收藏" },
            *allowed && entries.is_some() && !self.removing,
        )
    }
    fn entries(&self, library: usize) -> Option<&Vec<index::Entry>> {
        self.data.as_ref()?.get(library)?.as_ref().ok()
    }
    fn rows(&self, library: usize) -> Vec<Item> {
        let Some(entries) = self.entries(library) else {
            let label = if self.source.is_none() {
                "当前会话暂不可用"
            } else if self.data.is_some() {
                "无法读取此库，原数据保留"
            } else {
                "正在读取…"
            };
            return vec![row(0, label.into(), false)];
        };
        if entries.is_empty() {
            return vec![row(0, "没有冻结贴纸".into(), false)];
        }
        let thumbnails: std::collections::HashMap<_, _> = self
            .previews
            .batch(library)
            .map(|(_, id, image)| (id, image))
            .collect();
        entries
            .iter()
            .take(self.loaded[library])
            .enumerate()
            .map(|(slot, entry)| {
                let offset = library * CAPACITY + slot;
                let mut parent = row(0, entry.label.clone(), !self.removing);
                parent.delete = Some(REMOVE + offset);
                parent.thumbnail = thumbnails
                    .get(entry.id.as_str())
                    .map(|&image| image.clone());
                parent.children = Some(vec![
                    row(
                        BASE + offset,
                        if library == 0 {
                            "恢复贴纸"
                        } else {
                            "复制贴纸"
                        }
                        .into(),
                        true,
                    ),
                    row(REMOVE + offset, "删除此条".into(), true),
                ]);
                parent
            })
            .collect()
    }
    pub fn take(&self, command: usize) -> Option<Choice> {
        if self.removing {
            return None;
        }
        let remove = command >= REMOVE;
        let offset = command.checked_sub(if remove { REMOVE } else { BASE })?;
        if offset >= CAPACITY * 2 {
            return None;
        }
        let library = offset / CAPACITY;
        let index = offset % CAPACITY;
        if index >= self.loaded[library] {
            return None;
        }
        let entry = self.entries(library)?.get(index)?;
        Some(Choice {
            remove,
            source: self.source.as_ref()?.clone(),
            kind: [Kind::RecycleBin, Kind::ReferenceLibrary][library],
            entry_id: entry.id.clone(),
        })
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

#[cfg(test)]
#[path = "popup_frozen_remove_tests.rs"]
pub(super) mod remove_tests;
#[cfg(test)]
#[path = "popup_frozen_tests.rs"]
mod tests;

#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-canvas")))]
pub(in crate::desktop) fn test_close_index_menu(workers: Workers) {
    let mut menu = LibraryMenu::attach(&mut Vec::new(), Some(b"{}".to_vec().into()), workers);
    menu.job = Some(menu.start_index().unwrap().unwrap());
    drop(menu);
}
