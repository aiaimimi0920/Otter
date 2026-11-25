//! 单条删除只在原事务确认后换来源；重读期间保留图库、滚动和缩略图。
use super::*;
impl LibraryMenu {
    pub(in crate::desktop::popup_menu) fn remove(
        &mut self,
        menu: &mut Cascade,
        command: Option<usize>,
        remove: &mut super::super::Remove<'_>,
    ) -> Result<()> {
        if let Some(command) = command {
            if let Some(choice) = self.take(command).filter(|choice| choice.remove) {
                remove(Some(choice))?;
                self.removing = true;
                for library in 0..2 {
                    self.replace(menu, library)?;
                }
                self.update_reference(menu)?;
            }
        }
        if self.removing && !self.reload_pending && self.job.is_none() {
            if let Some(source) = remove(None)? {
                self.source = Some(source);
                self.previews.invalidate();
                self.reload_pending = true;
            }
        }
        Ok(())
    }
}
