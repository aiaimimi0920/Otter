//! 未改变的消息轮次只比较版本与共享像素身份，不重复构建显示图或克隆编辑快照。
use crate::desktop::{
    configuration_stamp::Stamp, document_window::DocumentWindow, image::Image,
    loom_result_state::OutputStamp,
};
use crate::{
    editor::DocumentStamp,
    workspace::{TopologyStamp, Workspace},
};
struct Document {
    configuration: Stamp,
    editor: DocumentStamp,
    output: OutputStamp,
    display: Option<Image>,
}
pub(in crate::desktop::application) struct Cache {
    topology: TopologyStamp,
    documents: Vec<Document>,
}
impl Cache {
    pub(in crate::desktop::application) fn capture(
        documents: &[DocumentWindow],
        workspace: &Workspace,
    ) -> Self {
        Self {
            topology: workspace.topology_stamp(),
            documents: documents
                .iter()
                .map(|doc| {
                    let results = doc.state.execution_results.borrow();
                    Document {
                        configuration: Stamp::capture(&doc.state),
                        editor: doc.state.canvas.borrow().editor.document_stamp(),
                        output: results.output_stamp(),
                        display: results.display().cloned(),
                    }
                })
                .collect(),
        }
    }
    pub(in crate::desktop::application) fn matches(
        &self,
        documents: &[DocumentWindow],
        workspace: &Workspace,
    ) -> bool {
        self.topology.matches(workspace)
            && self.documents.len() == documents.len()
            && self.documents.iter().zip(documents).all(|(stored, doc)| {
                let results = doc.state.execution_results.borrow();
                stored.configuration.matches(&doc.state)
                    && stored.editor.matches(&doc.state.canvas.borrow().editor)
                    && stored.output.matches(&results)
                    && match (&stored.display, results.display()) {
                        (None, None) => true,
                        (Some(a), Some(b)) => {
                            a.width == b.width
                                && a.height == b.height
                                && a.pixels.same_storage(&b.pixels)
                        }
                        _ => false,
                    }
            })
    }
}
