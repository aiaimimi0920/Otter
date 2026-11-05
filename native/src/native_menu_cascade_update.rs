//! 异步内容只替换指定根条目的子树；根 HWND/捕获不变，旧子窗口和手势失效。
use super::*;

impl Cascade {
    pub fn submenu_open(&self, index: usize) -> bool {
        !self.closed
            && self
                .pages
                .get(1)
                .is_some_and(|page| page.path.first() == Some(&index))
    }

    pub fn replace_submenu(&mut self, index: usize, children: Vec<Item>) -> Result<()> {
        if self.closed
            || self
                .tree
                .get(index)
                .is_none_or(|item| item.children.is_none())
        {
            return Err(Error::new(E_INVALIDARG, "Invalid dynamic submenu target"));
        }
        // 先检查包括静态业务条目在内的总预算，失败时保留旧内容和窗口。
        crate::native_menu::model::validate(&self.tree, Some((index, &children)))?;
        let reopen = self.submenu_open(index);
        if reopen {
            self.pages.truncate(1);
        }
        self.tree[index].children = Some(children);
        self.pending_surrogate = None;
        for page in &mut self.pages {
            page.pressed = None;
            page.drag = None;
        }
        if reopen {
            if let Err(error) = self.expand(0, index) {
                // 窗口重建失败不能让新模型与旧呈现继续响应命令。
                self.cancel();
                return Err(error);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "native_menu_cascade_update_tests.rs"]
mod tests;
