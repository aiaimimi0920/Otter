//! 异步读取结果的文档世代凭证；持有身份而非裸指针，防止地址复用造成误匹配。
use super::Editor;
use std::sync::Arc;

#[derive(Clone)]
pub(crate) struct DocumentStamp {
    identity: Arc<()>,
    revision: u64,
}

impl Editor {
    pub(crate) fn document_stamp(&self) -> DocumentStamp {
        DocumentStamp {
            identity: self.identity.clone(),
            revision: self.revision,
        }
    }
}

impl DocumentStamp {
    pub(crate) fn same_document(&self, editor: &Editor) -> bool {
        Arc::ptr_eq(&self.identity, &editor.identity)
    }
    // 只验证提交态；具体宿主仍须拒绝活动手势、取消或已销毁窗口。
    pub(crate) fn matches(&self, editor: &Editor) -> bool {
        self.same_document(editor) && self.revision == editor.revision
    }
}
