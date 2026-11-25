//! Asynchronous content replacement preserves gallery HWND/scroll position while retiring old gestures/accessibility nodes.
use super::*;

impl Cascade {
    /// Update original root commands after background indexing; do not replace HWNDs or reuse old press gestures.
    pub(crate) fn replace_action(
        &mut self,
        index: usize,
        command: usize,
        label: &str,
        enabled: bool,
    ) -> Result<()> {
        if self.closed
            || self
                .tree
                .get(index)
                .is_none_or(|item| item.children.is_some() || item.separator)
        {
            return Err(Error::new(E_INVALIDARG, "Invalid dynamic action target"));
        }
        let item = &mut self.tree[index];
        if item.command == command && item.label == label && item.enabled == enabled {
            return Ok(());
        }
        item.command = command;
        item.label = label.into();
        item.enabled = enabled;
        self.pending_surrogate = None;
        let root = &mut self.pages[0];
        root.pressed = None;
        root.drag = None;
        root.panel.discard_pointer_events();
        // Matching labels/slots may now hold different commands; never accept queued Invoke from old nodes.
        self.refresh_frame(0, true).inspect_err(|_| self.cancel())?;
        self.sync_accessibility();
        Ok(())
    }
    pub fn submenu_open(&self, index: usize) -> bool {
        !self.closed
            && self
                .pages
                .get(1)
                .is_some_and(|page| page.path.first() == Some(&index))
    }
    pub(crate) fn gallery_view(&self, root: usize) -> Option<std::ops::Range<usize>> {
        let page = self
            .pages
            .get(1)
            .filter(|page| page.path == [root] && page.layout.columns == 3)?;
        Some(page.first..(page.first + page.visible).min(self.items(&page.path).len()))
    }
    pub(crate) fn gallery_at_end(&self, root: usize) -> bool {
        self.gallery_view(root)
            .is_some_and(|range| range.end == self.items(&[root]).len())
    }

    pub fn replace_submenu(&mut self, index: usize, children: Vec<Item>) -> Result<()> {
        self.replace_layout(index, children, false)
    }
    /// Change only the selected root submenu's presentation; retain model, commands, and old-gesture retirement rules.
    pub(crate) fn replace_gallery(&mut self, index: usize, children: Vec<Item>) -> Result<()> {
        self.replace_layout(index, children, true)
    }
    /// A pixel-only update cannot change commands, hit regions, scroll or UIA generation.
    pub(crate) fn replace_gallery_thumbnails(
        &mut self,
        index: usize,
        previous: &[usize],
        images: &[(usize, &crate::image_thumbnail::Thumbnail)],
    ) -> Result<()> {
        let items = self
            .galleries
            .get_mut(&index)
            .filter(|_| !self.closed)
            .ok_or_else(|| Error::new(E_INVALIDARG, "Invalid thumbnail gallery"))?;
        let changed = || {
            previous
                .iter()
                .copied()
                .chain(images.iter().map(|&(slot, _)| slot))
        };
        if changed().any(|slot| slot >= items.len()) {
            return Err(Error::new(E_INVALIDARG, "Invalid thumbnail slot"));
        }
        for &slot in previous {
            items[slot].thumbnail = None;
        }
        for &(slot, image) in images {
            items[slot].thumbnail = Some(image.clone());
        }
        if let Some(page) = self.pages.get(1).filter(|page| page.path == [index]) {
            let view = page.first..page.first + page.visible;
            if changed().any(|slot| view.contains(&slot)) {
                page.panel
                    .update_thumbnails(|slot| items[slot].thumbnail.clone())
                    .inspect_err(|_| self.cancel())?;
            }
        }
        Ok(())
    }
    fn replace_layout(&mut self, index: usize, children: Vec<Item>, gallery: bool) -> Result<()> {
        if self.closed
            || self
                .tree
                .get(index)
                .is_none_or(|item| item.children.is_none())
        {
            return Err(Error::new(E_INVALIDARG, "Invalid dynamic submenu target"));
        }
        // Galleries hold only bounded logical rows; static menus retain original whole-tree budgets.
        if gallery {
            if self.galleries.len() >= 3 && !self.galleries.contains_key(&index) {
                return Err(Error::new(E_INVALIDARG, "Gallery count limit"));
            }
            crate::native_menu::model::validate_gallery(&children)?;
        } else {
            crate::native_menu::model::validate(&self.tree, Some((index, &children)))?;
        }
        let reopen = self.submenu_open(index);
        let reuse = reopen && gallery && self.pages[1].layout.columns == 3;
        let first = if reopen { self.pages[1].first } else { 0 };
        let focus = if reuse {
            self.pages[1].focus.filter(|&key| {
                self.items(&[index])
                    .get(key)
                    .zip(children.get(key))
                    .is_some_and(|(old, new)| {
                        old.command == new.command
                            && old.delete == new.delete
                            && old.label == new.label
                    })
            })
        } else {
            None
        };
        if reopen && !reuse {
            self.preview = None;
        }
        self.pending_delete = None;
        if reopen {
            self.pages.truncate(if reuse { 2 } else { 1 });
        }
        if gallery {
            self.galleries.insert(index, children);
        } else {
            self.galleries.remove(&index);
            self.tree[index].children = Some(children);
        }
        self.pending_surrogate = None;
        for page in &mut self.pages {
            page.pressed = None;
            page.delete_pressed = None;
            page.drag = None;
            page.panel.discard_pointer_events();
            if !reuse {
                page.panel.set_delete_hover(None);
            }
        }
        if reuse {
            self.resize_gallery().inspect_err(|_| self.cancel())?;
            let count = self.items(&[index]).len();
            let page = &mut self.pages[1];
            page.first = page
                .layout
                .align(first)
                .min(page.layout.max_first(count, page.visible));
            page.focus = focus;
            if page.first != first {
                page.panel.set_delete_hover(None);
            }
            self.refresh_frame(1, true).inspect_err(|_| self.cancel())?;
            if let Some(view) = self.preview.as_ref() {
                // When deletion shifts the last page upward, reading targets follow original visible card slots, not old whole-library indices.
                let target = view.index.checked_sub(first).and_then(|slot| {
                    (slot < self.pages[1].visible).then_some(self.pages[1].first + slot)
                });
                let content = target.and_then(|target| {
                    self.items(&[index])
                        .get(target)
                        .and_then(|item| item.preview.clone())
                        .map(|content| (target, content))
                });
                if let Some((target, content)) = content {
                    let view = self.preview.as_mut().unwrap();
                    view.index = target;
                    view.panel.set_preview(content);
                } else {
                    self.preview = None;
                }
            }
            self.sync_accessibility();
        } else if reopen {
            if let Err(error) = self.expand(0, index) {
                // Window-recreation failure cannot leave new models and old presentations responding to commands.
                self.cancel();
                return Err(error);
            }
            self.set_first(1, first).inspect_err(|_| self.cancel())?;
        }
        Ok(())
    }
    fn resize_gallery(&mut self) -> Result<()> {
        let count = self.items(&self.pages[1].path).len();
        let page = &mut self.pages[1];
        let mut monitor = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        unsafe {
            if !GetMonitorInfoW(
                MonitorFromRect(&page.bounds, MONITOR_DEFAULTTONEAREST),
                &mut monitor,
            )
            .as_bool()
            {
                return Err(Error::from_win32());
            }
            let work = monitor.rcWork;
            let rows = count
                .div_ceil(3)
                .clamp(1, 5)
                .min(((work.bottom - work.top) / page.layout.row).max(1) as usize);
            page.visible = rows * 3;
            let height = rows as i32 * page.layout.row;
            let top = page.bounds.top.min(work.bottom - height).max(work.top);
            if page.bounds.bottom - page.bounds.top != height || top != page.bounds.top {
                SetWindowPos(
                    page.panel.handle()?,
                    None,
                    page.bounds.left,
                    top,
                    page.bounds.right - page.bounds.left,
                    height,
                    SWP_NOACTIVATE | SWP_NOZORDER,
                )?;
                page.bounds.top = top;
                page.bounds.bottom = top + height;
            }
        }
        Ok(())
    }
}

#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-canvas")))]
#[path = "native_menu_cascade_update_tests.rs"]
mod tests;
