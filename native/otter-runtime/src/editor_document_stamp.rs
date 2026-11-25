//! Document-generation evidence for asynchronous read results; retain identity rather than raw pointers, preventing false matches from address reuse.
use super::Editor;
use std::sync::Arc;

#[derive(Clone)]
pub struct DocumentStamp {
    identity: Arc<()>,
    revision: u64,
    presentation_revision: u64,
}

impl Editor {
    pub fn document_stamp(&self) -> DocumentStamp {
        DocumentStamp {
            identity: self.identity.clone(),
            revision: self.revision,
            presentation_revision: self.presentation_revision,
        }
    }
}

impl DocumentStamp {
    /// Presentation history still invalidates ordinary asynchronous evidence, but movement, opacity, and outer frames are not OCR-source edits.
    pub fn matches_content(&self, editor: &Editor) -> bool {
        self.same_document(editor)
            && self.revision.wrapping_sub(self.presentation_revision)
                == editor.revision.wrapping_sub(editor.presentation_revision)
    }
    pub fn same_document(&self, editor: &Editor) -> bool {
        Arc::ptr_eq(&self.identity, &editor.identity)
    }
    // Validate committed state only; specific hosts must still reject active gestures, cancellation, or destroyed windows.
    pub fn matches(&self, editor: &Editor) -> bool {
        self.same_document(editor) && self.revision == editor.revision
    }
}
