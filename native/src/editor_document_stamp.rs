//! 异步读取结果的文档世代凭证；持有身份而非裸指针，防止地址复用造成误匹配。
use super::Editor;
use std::sync::Arc;

#[derive(Clone)]
pub(crate) struct DocumentStamp {
    identity: Arc<()>,
    revision: u64,
    presentation_revision: u64,
}

impl Editor {
    pub(crate) fn document_stamp(&self) -> DocumentStamp {
        DocumentStamp {
            identity: self.identity.clone(),
            revision: self.revision,
            presentation_revision: self.presentation_revision,
        }
    }
}

impl DocumentStamp {
    /// 呈现历史仍使普通异步凭证失效，但不把移动、透明度或外框当作 OCR 源编辑。
    pub(crate) fn matches_content(&self, editor: &Editor) -> bool {
        self.same_document(editor)
            && self.revision.wrapping_sub(self.presentation_revision)
                == editor.revision.wrapping_sub(editor.presentation_revision)
    }
    pub(crate) fn same_document(&self, editor: &Editor) -> bool {
        Arc::ptr_eq(&self.identity, &editor.identity)
    }
    // 只验证提交态；具体宿主仍须拒绝活动手势、取消或已销毁窗口。
    pub(crate) fn matches(&self, editor: &Editor) -> bool {
        self.same_document(editor) && self.revision == editor.revision
    }
}
