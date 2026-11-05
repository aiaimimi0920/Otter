//! 菜单本地导航的有界单槽交付；只供翻页等呈现更新，不在模态回调中执行业务。
use super::*;
#[derive(Default)]
pub(super) struct State {
    pub commands: Vec<usize>,
    pub pending: Option<usize>,
}

#[cfg(test)]
#[path = "native_menu_navigation_tests.rs"]
mod tests;
impl Cascade {
    /// 首次动态回调登记，后续必须保持相同映射；最多八个非零且唯一的导航命令。
    pub fn navigation_commands(&mut self, commands: &[usize]) -> Result<()> {
        if self.closed
            || commands.len() > 8
            || commands.contains(&0)
            || commands
                .iter()
                .enumerate()
                .any(|(index, id)| commands[..index].contains(id))
            || (!self.navigation.commands.is_empty() && self.navigation.commands != commands)
        {
            return Err(Error::new(E_INVALIDARG, "Invalid menu navigation mapping"));
        }
        if self.navigation.commands.is_empty() {
            self.navigation.commands.extend_from_slice(commands);
        }
        Ok(())
    }
    /// 只返回本地导航，不成为最终业务命令；调用方须在本次 update 内替换对应子菜单。
    pub fn take_navigation(&mut self) -> Option<usize> {
        self.navigation.pending.take()
    }
}
