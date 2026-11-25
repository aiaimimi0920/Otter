//! 工具栏只读取实际有界历史的可用性，不以点击次数或标注数量推断撤销/重做。
impl super::Editor {
    pub fn history_state(&self) -> (bool, bool) {
        if self.draft.is_some() {
            (false, false)
        } else {
            self.history.availability()
        }
    }
}
