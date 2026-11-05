//! 两个根级冻结库的完整分页；只展示当前页，命令绑定绘制时来源与条目身份。
use crate::{
    native_menu::{cascade::Cascade, Item},
    session_library::Kind,
    session_source::Source,
};
use windows::core::Result;
#[path = "popup_frozen_index.rs"]
mod index;
#[path = "popup_frozen_smoke.rs"]
pub(in crate::desktop) mod smoke;
use super::super::frozen_library_ui::Choice;
const PAGE: usize = 64;
const BASE: usize = usize::MAX - 1024;
const NAV: [usize; 4] = [BASE + 128, BASE + 129, BASE + 130, BASE + 131];
const REMOVE: usize = BASE + 256;
pub(super) struct LibraryMenu {
    roots: [usize; 2],
    source: Option<Source>,
    pages: [usize; 2],
    data: Option<index::Index>,
    job: Option<index::Job>,
    started: bool,
}
impl LibraryMenu {
    pub fn attach(tree: &mut Vec<Item>, source: Option<Source>) -> Self {
        let roots = [tree.len(), tree.len() + 1];
        for label in ["回收站", "参考库"] {
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
            tree.push(parent);
        }
        Self {
            roots,
            source,
            pages: [0, 0],
            data: None,
            job: None,
            started: false,
        }
    }
    pub fn update(&mut self, menu: &mut Cascade) -> Result<()> {
        menu.navigation_commands(&NAV)?;
        if let Some(command) = menu.take_navigation() {
            if let Some(nav) = NAV.iter().position(|id| *id == command) {
                let library = nav / 2;
                let count = self
                    .entries(library)
                    .map_or(0, |entries| entries.len().div_ceil(PAGE));
                self.pages[library] = if nav % 2 == 0 {
                    self.pages[library].saturating_sub(1)
                } else {
                    (self.pages[library] + 1).min(count.saturating_sub(1))
                };
                menu.replace_submenu(self.roots[library], self.rows(library))?;
            }
        }
        if !self.started && self.roots.iter().any(|root| menu.submenu_open(*root)) {
            self.started = true;
            if let Some(source) = &self.source {
                match index::Job::start(source.clone()) {
                    Ok(job) => self.job = Some(job),
                    Err(_) => {
                        self.data = Some([Err("Unavailable".into()), Err("Unavailable".into())])
                    }
                }
            }
            for library in 0..2 {
                menu.replace_submenu(self.roots[library], self.rows(library))?;
            }
        }
        if let Some(data) = self.job.as_mut().and_then(index::Job::poll) {
            self.job = None;
            self.data = Some(data);
            for library in 0..2 {
                menu.replace_submenu(self.roots[library], self.rows(library))?;
            }
        }
        Ok(())
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
        let page = self.pages[library];
        let count = entries.len().div_ceil(PAGE);
        let mut rows = vec![
            row(
                0,
                format!("第 {} / {} 页 · {} 项", page + 1, count, entries.len()),
                false,
            ),
            row(NAV[library * 2], "上一页".into(), page > 0),
        ];
        rows.extend(entries.iter().skip(page * PAGE).take(PAGE).enumerate().map(
            |(slot, entry)| {
                let offset = library * PAGE + slot;
                let mut parent = row(0, entry.label.clone(), true);
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
            },
        ));
        rows.push(row(NAV[library * 2 + 1], "下一页".into(), page + 1 < count));
        rows
    }
    pub fn take(&self, command: usize) -> Option<Choice> {
        let remove = command >= REMOVE;
        let offset = command.checked_sub(if remove { REMOVE } else { BASE })?;
        if offset >= PAGE * 2 {
            return None;
        }
        let library = offset / PAGE;
        let entry = self
            .entries(library)?
            .get(self.pages[library] * PAGE + offset % PAGE)?;
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
        thumbnail: None,
    }
}

#[cfg(test)]
#[path = "popup_frozen_tests.rs"]
mod tests;
