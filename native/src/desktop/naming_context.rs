//! 对齐主干 buildUnitFileNamingContext：标题/标签是能力名称，process 是 artId，不查询 OS。
use super::State;
use crate::{
    file_naming::{Context, MAX_CONTEXT_BYTES},
    loom_capability::Capability,
};

#[derive(Default)]
pub(super) enum RestoredArtId {
    #[default]
    Absent,
    Value(String),
    Oversized,
}
impl RestoredArtId {
    pub fn from_id(id: Option<&str>) -> Self {
        match id {
            None => Self::Absent,
            Some(id) if id.len() <= MAX_CONTEXT_BYTES => Self::Value(id.to_owned()),
            Some(_) => Self::Oversized,
        }
    }
}
/// 借用当前选择的不可变目录，不克隆参数、目录或图像；回调不能跨模态留存上下文。
pub(super) fn with_context<T>(
    state: &State,
    dimensions: (u32, u32),
    render: impl FnOnce(&Context<'_>) -> T,
) -> Option<T> {
    let selected = state.selected_art.borrow();
    let process = if let Some(selected) = selected.as_ref() {
        selected.parameters.art_id()
    } else if let Some(id) = state.local_art_id.as_deref() {
        id
    } else {
        match &state.restored_naming_art {
            RestoredArtId::Absent => "",
            RestoredArtId::Value(id) => id,
            RestoredArtId::Oversized => return None,
        }
    };
    let label = selected
        .as_ref()
        .and_then(|selected| {
            selected
                .catalog
                .handshake
                .arts()
                .iter()
                .filter_map(Capability::new)
                .find(|cap| cap.id == selected.parameters.art_id())
                .map(|cap| cap.label)
        })
        .unwrap_or("");
    let kind = if state.is_art() { "art" } else { "image" };
    Some(render(&Context {
        app: "Otter",
        kind,
        label: if label.trim().is_empty() {
            kind
        } else {
            label.trim()
        },
        title: label.trim(),
        process,
        unit_id: state.unit_id.as_str(),
        width: Some(dimensions.0),
        height: Some(dimensions.1),
        ..Default::default()
    }))
}

#[cfg(test)]
#[path = "naming_context_tests.rs"]
mod tests;
