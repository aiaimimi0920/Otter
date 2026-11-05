//! UI owner 的有界配方 LRU；仅保留成功正式提交的代码/响应纹理，不缓存输入、像素或 child。
use super::{capability_job::Loaded, loom_execution_tasks::Report, shader_payload::Payload, State};
use crate::{
    byte_budget::{Budget, Lease},
    workspace_id::UnitId,
};
use std::{rc::Rc, sync::Arc};
const MAX_ENTRIES: usize = 16;
const MAX_BYTES: usize = 64 * 1024 * 1024;
#[derive(Clone, Debug, PartialEq)]
pub(super) enum Context {
    Independent,
    Referenced {
        input: [u8; 32],
        reference: [u8; 32],
    },
}
pub(super) struct Acquired {
    pub payload: Arc<Payload>,
    pub context: Context,
}
pub(super) struct Recipe {
    pub payload: Arc<Payload>,
    // 运行中的缓存命中也持有租约；驱逐不能提前归还仍被 worker 引用的字节。
    _lease: Lease,
}
struct Key {
    unit: UnitId,
    selection: UnitId,
    art: String,
    catalog: Rc<Loaded>,
    context: Context,
}
impl Key {
    fn capture(state: &State, context: Context) -> Option<Self> {
        let selection = state.selected_art.borrow();
        let selection = selection.as_ref()?;
        Some(Self {
            unit: state.unit_id.clone(),
            selection: selection.identity.clone(),
            art: selection.parameters.art_id().into(),
            catalog: selection.catalog.clone(),
            context,
        })
    }
    fn matches(&self, state: &State) -> bool {
        self.unit == state.unit_id
            && state
                .selected_art
                .borrow()
                .as_ref()
                .is_some_and(|selected| {
                    self.selection == selected.identity
                        && self.art == selected.parameters.art_id()
                        && Rc::ptr_eq(&self.catalog, &selected.catalog)
                })
    }
    fn same(&self, other: &Self) -> bool {
        self.unit == other.unit
            && self.selection == other.selection
            && self.art == other.art
            && Rc::ptr_eq(&self.catalog, &other.catalog)
            && self.context == other.context
    }
}
struct Entry {
    key: Key,
    recipe: Arc<Recipe>,
}
pub(super) struct Cache {
    entries: Vec<Entry>,
    budget: Arc<Budget>,
}
impl Default for Cache {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
            budget: Budget::new(MAX_BYTES),
        }
    }
}
impl Cache {
    pub(super) fn retain(
        &mut self,
        documents: &[super::document_window::DocumentWindow],
        enabled: bool,
    ) {
        self.entries
            .retain(|entry| enabled && documents.iter().any(|doc| entry.key.matches(&doc.state)));
    }
    pub(super) fn bind(
        &mut self,
        state: &State,
        execution: &mut super::loom_execution_job::Execution,
    ) {
        let Some(runtime) = execution.shader.as_mut() else {
            return;
        };
        let Some(context) = runtime.context(&execution.inputs) else {
            return;
        };
        let Some(key) = Key::capture(state, context) else {
            return;
        };
        if let Some(index) = self.entries.iter().position(|entry| entry.key.same(&key)) {
            let entry = self.entries.remove(index);
            runtime.cached = Some(entry.recipe.clone());
            self.entries.push(entry);
        }
    }
    /// 必须在 Report::commit_checked 成功之后调用；迟到/取消/清理未确认的配方不回填。
    pub(super) fn record(&mut self, state: &State, report: &mut Report) {
        let super::loom_execution_tasks::Completion::Remote(Ok(completion)) =
            &mut report.completion
        else {
            return;
        };
        let Some(acquired) = completion.shader_recipe.take() else {
            return;
        };
        let Some(key) = Key::capture(state, acquired.context) else {
            return;
        };
        self.insert(key, acquired.payload);
    }
    fn insert(&mut self, key: Key, payload: Arc<Payload>) {
        // 保守计入 String capacity 和每个映射项开销；这不是驱动/编译器/总进程硬预算。
        let Some(bytes) = resident_bytes(&payload).filter(|bytes| *bytes <= MAX_BYTES) else {
            return;
        };
        self.entries.retain(|entry| !entry.key.same(&key));
        while self.entries.len() >= MAX_ENTRIES {
            self.entries.remove(0);
        }
        let lease = loop {
            if let Some(lease) = self.budget.reserve(bytes) {
                break lease;
            }
            if self.entries.is_empty() {
                return;
            }
            self.entries.remove(0);
        };
        if self.entries.try_reserve(1).is_err() {
            return;
        }
        self.entries.push(Entry {
            key,
            recipe: Arc::new(Recipe {
                payload,
                _lease: lease,
            }),
        });
    }
}
fn resident_bytes(payload: &Payload) -> Option<usize> {
    let mut bytes = std::mem::size_of::<Payload>()
        .checked_add(payload.vertex_shader.capacity())?
        .checked_add(payload.source.capacity())?
        .checked_add(payload.abi.as_ref().map_or(0, String::capacity))?
        .checked_add(payload.fragment_shader.capacity())?;
    for (name, value) in &payload.textures {
        bytes = bytes
            .checked_add(name.capacity())?
            .checked_add(value.capacity())?
            .checked_add(128)?;
    }
    for name in payload.uniforms.keys() {
        bytes = bytes.checked_add(name.capacity())?.checked_add(128)?;
    }
    Some(bytes)
}
#[cfg(test)]
#[path = "shader_recipe_cache_tests.rs"]
mod tests;
