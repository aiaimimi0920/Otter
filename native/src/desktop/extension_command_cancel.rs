//! 同目标、目录、插件和命名输入的重复触发取消；只撤回已有工作，不签发新的资源许可。
use super::*;
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(PartialEq, Eq)]
struct Key {
    plugin: String,
    scope: String,
    context: String,
}
fn key(catalog: &Snapshot, context: &Context, id: &str) -> Option<(Key, bool)> {
    use extension_command_route::{OCR, OCR_OVERLAY, TRANSLATE, TRANSLATION_OVERLAY};
    // 主干 overlay 无缓存时递归进入实际命令；别为同一次执行建立两个取消身份。
    let id = match id {
        OCR_OVERLAY => OCR,
        TRANSLATION_OVERLAY => TRANSLATE,
        id => id,
    };
    context.with(|context| {
        let command = catalog.command(id, context)?;
        command.permissions()?;
        let payload = command.payload()?;
        let input = payload
            .get("inputContext")
            .and_then(serde_json::Value::as_str);
        if input.is_none() && id != OCR {
            return None;
        }
        Some((
            Key {
                plugin: command.plugin().into(),
                scope: command.scope().into(),
                context: input.unwrap_or(id).into(),
            },
            payload
                .get("toggleAttachmentType")
                .and_then(serde_json::Value::as_str)
                .is_some_and(|s| !s.is_empty()),
        ))
    })
}
pub(super) struct Active {
    unit: UnitId,
    catalog: Arc<Snapshot>,
    revision: u64,
    key: Option<Key>,
    signal: Arc<AtomicBool>,
}
impl Active {
    pub fn new(intent: &Intent) -> Self {
        Self {
            unit: intent.unit.clone(),
            catalog: intent.catalog.clone(),
            revision: intent.revision,
            key: key(&intent.catalog, &intent.context, &intent.command).map(|(key, _)| key),
            signal: intent.cancel.clone(),
        }
    }
    pub fn permits(&self, state: &State, catalog: &Arc<Snapshot>, command: &str) -> bool {
        !self.signal.load(Ordering::Acquire)
            && self.unit == state.unit_id
            && Some(self.revision) == state.extension_document.borrow().revision()
            && Arc::ptr_eq(&self.catalog, catalog)
            && key(catalog, &Context::capture(state), command)
                .is_some_and(|(key, toggle)| toggle && self.key.as_ref() == Some(&key))
    }
    pub fn cancel(&self) {
        self.signal.store(true, Ordering::Release);
    }
}
pub(super) fn available(state: &State, command: &str) -> bool {
    let view = state.extension_commands.borrow();
    if view.closed {
        return false;
    }
    let catalog = state.extensions.borrow().snapshot.clone();
    let Some(catalog) = catalog else {
        return false;
    };
    if !state.extension_document.borrow().store.available()
        || state.extension_document.borrow().revision().is_none()
        || !Context::capture(state).with(|context| {
            catalog
                .command(command, context)
                .is_some_and(|c| c.permissions().is_some())
        })
    {
        return false;
    }
    !view.busy
        || view
            .active
            .as_ref()
            .is_some_and(|a| a.permits(state, &catalog, command))
}
