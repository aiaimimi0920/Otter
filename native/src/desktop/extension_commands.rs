//! 应用级一个扩展命令任务；按钮只登记轻量意图，资源准备和结果提交在外层消息结算复核。
use super::{
    capability_job::Source,
    document_window::DocumentWindow,
    extension_command_bridge::{Reply, Request},
    extension_command_flow::Spec,
    extension_command_job::Job,
    extension_command_route::{self, Context},
    extension_document::{self, Stamp},
    loom_execution_signal::Notify,
    State,
};
use crate::{
    loom_extension_session::{Error, Target},
    loom_extension_snapshot::Snapshot,
    workspace_id::UnitId,
};
use std::{cell::RefCell, rc::Rc, sync::Arc};
use windows::{
    core::Result,
    Win32::{Foundation::*, UI::WindowsAndMessaging::*},
};
#[path = "extension_command_cancel.rs"]
mod cancel;
pub(super) fn command_available(state: &State, command: &str) -> bool {
    cancel::available(state, command)
}
struct Intent {
    unit: UnitId,
    stamp: Stamp,
    catalog: Arc<Snapshot>,
    revision: u64,
    context: Context,
    command: String,
    input: serde_json::Value,
    cancel: Arc<std::sync::atomic::AtomicBool>,
}
#[derive(Default)]
pub(super) struct View {
    requested: Option<Intent>,
    busy: bool,
    closed: bool,
    active: Option<cancel::Active>,
}
pub(super) type Shared = Rc<RefCell<View>>;
#[cfg(test)]
pub(super) fn busy(state: &State) -> bool {
    let view = state.extension_commands.borrow();
    view.busy || view.closed
}
#[cfg(test)]
pub(super) fn requested_command(state: &State) -> Option<String> {
    state
        .extension_commands
        .borrow()
        .requested
        .as_ref()
        .map(|i| i.command.clone())
}
struct Pending {
    intent: Intent,
    job: Job,
    cancelled: bool,
}
pub(super) struct Owner {
    pub(super) view: Shared,
    pending: Option<Pending>,
    notify: Notify,
}
pub(super) fn available(state: &State, translation: bool) -> bool {
    let command =
        state.extensions.borrow().snapshot.as_ref().and_then(|s| {
            extension_command_route::command(s, &Context::capture(state), translation)
        });
    command.is_some_and(|c| command_available(state, &c))
}
pub(super) unsafe fn request(hwnd: HWND, state: &State, translation: bool) -> Result<()> {
    let command =
        state.extensions.borrow().snapshot.as_ref().and_then(|s| {
            extension_command_route::command(s, &Context::capture(state), translation)
        });
    let Some(command) = command else {
        return Ok(());
    };
    request_command(hwnd, state, &command)
}
pub(super) unsafe fn request_command(hwnd: HWND, state: &State, command: &str) -> Result<()> {
    request_input(hwnd, state, command, serde_json::json!({}))
}
pub(super) unsafe fn request_input(
    hwnd: HWND,
    state: &State,
    command: &str,
    input: serde_json::Value,
) -> Result<()> {
    if !state.lifetime.owns(hwnd) || state.modals.blocked() {
        return Ok(());
    }
    extension_document::observe(state);
    if !command_available(state, command) {
        return Ok(());
    }
    if state.extension_commands.borrow().busy {
        // 先确认能唤醒原 owner，再标记取消。原子标记也使已排队的 Apply 立即失效。
        PostMessageW(Some(hwnd), WM_NULL, WPARAM(0), LPARAM(0))?;
        if let Some(active) = &state.extension_commands.borrow().active {
            active.cancel();
        }
        return Ok(());
    }
    let Some(catalog) = state.extensions.borrow().snapshot.clone() else {
        return Ok(());
    };
    let document = state.extension_document.borrow();
    let context = Context::capture(state);
    if !context.with(|context| {
        catalog
            .command(command, context)
            .is_some_and(|c| c.permissions().is_some())
    }) || !document.store.available()
    {
        return Ok(());
    }
    let Some(revision) = document.revision() else {
        return Ok(());
    };
    let intent = Intent {
        unit: state.unit_id.clone(),
        stamp: document.stamp(),
        catalog,
        revision,
        context,
        command: command.into(),
        input,
        cancel: Arc::new(std::sync::atomic::AtomicBool::new(false)),
    };
    drop(document);
    let mut view = state.extension_commands.borrow_mut();
    view.active = Some(cancel::Active::new(&intent));
    view.requested = Some(intent);
    view.busy = true;
    if let Err(error) = PostMessageW(Some(hwnd), WM_NULL, WPARAM(0), LPARAM(0)) {
        view.requested = None;
        view.busy = false;
        view.active = None;
        return Err(error);
    }
    Ok(())
}
fn current(intent: &Intent, state: &State) -> bool {
    extension_document::observe(state);
    !intent.cancel.load(std::sync::atomic::Ordering::Acquire)
        && intent.stamp.matches(&state.extension_document.borrow())
        && state
            .extensions
            .borrow()
            .snapshot
            .as_ref()
            .is_some_and(|s| Arc::ptr_eq(s, &intent.catalog))
}
impl Owner {
    pub(super) fn new(view: Shared, notify: Notify) -> Self {
        Self {
            view,
            pending: None,
            notify,
        }
    }
    pub(super) fn deadline(&self) -> Option<std::time::Instant> {
        self.pending.as_ref().and_then(|p| p.job.deadline())
    }
    pub(super) unsafe fn process(
        &mut self,
        documents: &[DocumentWindow],
        settings: &crate::app_settings_document::Document,
    ) {
        self.process_with(documents, settings, || {
            Source::Discovery(crate::loom_discovery::Config::from_env())
        });
    }
    pub(super) unsafe fn process_with(
        &mut self,
        documents: &[DocumentWindow],
        settings: &crate::app_settings_document::Document,
        source: impl FnOnce() -> Source,
    ) {
        if let Some(pending) = &mut self.pending {
            let document = documents
                .iter()
                .find(|d| d.state.unit_id == pending.intent.unit && d.is_alive());
            if document.is_none_or(|d| !current(&pending.intent, &d.state)) {
                pending.cancelled = true;
                pending.job.cancel();
            }
            if let Some(packet) = pending.job.take_request() {
                let result = if pending.cancelled {
                    Err(Error::Cancelled)
                } else {
                    let document = document.unwrap();
                    handle(
                        document.window.0,
                        &document.state,
                        &packet.request,
                        &mut pending.intent,
                        settings,
                    )
                };
                packet.answer(result);
            }
            if let Some(result) = pending.job.poll() {
                if !pending.cancelled
                    && !pending
                        .intent
                        .cancel
                        .load(std::sync::atomic::Ordering::Acquire)
                {
                    if let Err(error) = result {
                        eprintln!("native-extension-command: {error}");
                        if let Some(document) = document {
                            if let Err(error) = super::extension_notices::show(
                                document.window.0,
                                &document.state,
                                None,
                                "扩展操作未完成",
                                "请检查 Loom 连接及扩展状态后重试。",
                            ) {
                                eprintln!("native-extension-notice: {error}");
                            }
                        }
                    }
                }
                self.pending.take();
                self.settled(documents);
            }
        }
        let intent = self.view.borrow_mut().requested.take();
        if let Some(intent) = intent {
            assert!(self.pending.is_none(), "extension commands overlapped");
            let document = documents
                .iter()
                .find(|d| d.state.unit_id == intent.unit && d.is_alive());
            if document.is_none_or(|d| !current(&intent, &d.state)) {
                self.settled(documents);
                return;
            }
            let spec = Spec {
                catalog: intent.catalog.clone(),
                target: Target::new(intent.unit.as_str(), intent.revision).unwrap(),
                context: intent.context.clone(),
                command: intent.command.clone(),
                input: intent.input.clone(),
            };
            match Job::start(source(), spec, self.notify.clone(), intent.cancel.clone()) {
                Ok(job) => {
                    self.pending = Some(Pending {
                        intent,
                        job,
                        cancelled: false,
                    })
                }
                Err(error) => {
                    self.view.borrow_mut().busy = false;
                    self.view.borrow_mut().active = None;
                    eprintln!("native-extension-command-start: {error}");
                }
            }
            for d in documents {
                super::tool_panel::refresh(d.window.0);
            }
        }
    }
    unsafe fn settled(&self, documents: &[DocumentWindow]) {
        {
            let mut view = self.view.borrow_mut();
            view.busy = false;
            view.active = None;
        }
        // 排队时已取消的任务没有 worker 完成消息；清槽也必须恢复实际按钮可用性。
        for document in documents {
            super::tool_panel::refresh(document.window.0);
        }
    }
    pub(super) fn shutdown(&mut self) {
        let mut view = self.view.borrow_mut();
        view.closed = true;
        view.busy = false;
        view.requested = None;
        view.active = None;
        self.pending.take();
    }
}
impl Drop for Owner {
    fn drop(&mut self) {
        self.shutdown();
    }
}
unsafe fn handle(
    hwnd: HWND,
    state: &State,
    request: &Request,
    intent: &mut Intent,
    settings: &crate::app_settings_document::Document,
) -> std::result::Result<Reply, Error> {
    if !current(intent, state) {
        return Err(Error::AuthorityChanged);
    }
    match request {
        Request::Input => {
            let (language, provider) = settings.translation_options();
            super::extension_ocr_input::prepare(
                &state.extension_document.borrow().store,
                intent.revision,
                language,
                provider,
            )
            .map(Reply::Input)
        }
        Request::Toggle { translation } => {
            let mut document = state.extension_document.borrow_mut();
            let changed = crate::extension_visual::toggle(
                &intent.catalog,
                &mut document.store,
                intent.revision,
                *translation,
            )
            .map_err(|_| Error::Unavailable)?;
            intent.stamp = document.stamp();
            drop(document);
            super::extension_overlay::sync(hwnd, state).map_err(|_| Error::Unavailable)?;
            super::tool_panel::refresh(hwnd);
            Ok(Reply::Toggled(changed))
        }
        Request::Resources { image, attachments } => {
            if state.canvas.borrow().editor.draft().is_some() {
                return Err(Error::Unavailable);
            }
            let image = if *image {
                let canvas = state.canvas.borrow();
                Some(
                    state
                        .resolved_display
                        .borrow()
                        .as_ref()
                        .map(|f| f.image.clone())
                        .or_else(|| {
                            canvas
                                .minified_source
                                .as_ref()
                                .filter(|s| s.matches(&canvas.editor))
                                .map(|s| s.image().clone())
                                .or_else(|| canvas.image.clone())
                        })
                        .ok_or(Error::Unavailable)?,
                )
            } else {
                None
            };
            let store = attachments.then(|| state.extension_document.borrow().store.clone());
            Ok(Reply::Resources { image, store })
        }
        Request::Apply { command, outcome } => {
            let result = super::extension_command_effects::apply(
                hwnd,
                state,
                &intent.catalog,
                intent.revision,
                command,
                outcome,
                &intent.cancel,
            );
            // 只接管本事务已完成的独立 CAS；后续真实用户编辑仍会使新 stamp 失效。
            intent.stamp = state.extension_document.borrow().stamp();
            super::extension_overlay::sync(hwnd, state).map_err(|_| Error::Unavailable)?;
            super::tool_panel::refresh(hwnd);
            result?;
            Ok(Reply::Applied(Context::capture(state)))
        }
    }
}
#[cfg(test)]
#[path = "extension_commands_tests.rs"]
mod tests;
