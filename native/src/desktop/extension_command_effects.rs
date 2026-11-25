//! Host commits from complete results into document attachments; independent CAS per item, with no system side effects after permission/provenance failure.
use super::{extension_command_route::Context, State};
use crate::{
    loom_extension_session::{EffectKind, Error, Outcome},
    loom_extension_snapshot::Snapshot,
};
use std::sync::atomic::{AtomicBool, Ordering};
use windows::Win32::Foundation::HWND;
pub(super) unsafe fn apply(
    hwnd: HWND,
    state: &State,
    catalog: &Snapshot,
    revision: u64,
    command: &str,
    outcome: &Outcome,
    cancelled: &AtomicBool,
) -> Result<(), Error> {
    if outcome.generation() != catalog.generation() {
        return Err(Error::AuthorityChanged);
    }
    let context = Context::capture(state);
    let mut failed = false;
    for effect in outcome.effects() {
        if cancelled.load(Ordering::Acquire) {
            return Err(Error::Cancelled);
        }
        let result = context.with(|context| {
            super::extension_document::observe(state);
            if state.extension_document.borrow().revision() != Some(revision)
                || !state
                    .extensions
                    .borrow()
                    .snapshot
                    .as_ref()
                    .is_some_and(|s| std::ptr::eq(s.as_ref(), catalog))
                || state.canvas.borrow().editor.draft().is_some()
            {
                return Err(Error::AuthorityChanged);
            }
            let command = catalog
                .command(command, context)
                .ok_or(Error::Unavailable)?;
            if command.plugin() != outcome.plugin() || command.scope() != outcome.scope() {
                return Err(Error::Denied);
            }
            let p = effect.payload().as_object().ok_or(Error::Wire)?;
            match effect.kind() {
                EffectKind::AttachmentUpsert | EffectKind::AttachmentRemove => {
                    if !command.permits("hook.unit.attachments.write") {
                        return Err(Error::Denied);
                    }
                    let mut document = state.extension_document.borrow_mut();
                    let current = document.revision().ok_or(Error::Unavailable)?;
                    let result = if *effect.kind() == EffectKind::AttachmentUpsert {
                        document.store.upsert(
                            catalog,
                            outcome.scope(),
                            revision,
                            current,
                            effect.payload().clone(),
                        )
                    } else {
                        document.store.remove(
                            catalog,
                            outcome.scope(),
                            revision,
                            current,
                            p.get("attachmentId")
                                .and_then(|v| v.as_str())
                                .ok_or(Error::Wire)?,
                            p.get("priorRevision")
                                .and_then(|v| v.as_u64())
                                .ok_or(Error::Wire)?,
                        )
                    };
                    result.map_err(|_| Error::AuthorityChanged)
                }
                EffectKind::ClipboardWriteText => {
                    if !command.permits("hook.clipboard.write") {
                        return Err(Error::Denied);
                    }
                    let text = p.get("text").and_then(|v| v.as_str()).ok_or(Error::Wire)?;
                    let stamp = state.extension_document.borrow().stamp();
                    super::clipboard::copy_extension_text_checked(hwnd, text, || {
                        super::extension_document::observe(state);
                        super::host_lifetime::owns(hwnd)
                            && !cancelled.load(Ordering::Acquire)
                            && stamp.matches(&state.extension_document.borrow())
                            && state
                                .extensions
                                .borrow()
                                .snapshot
                                .as_ref()
                                .is_some_and(|s| std::ptr::eq(s.as_ref(), catalog))
                    })
                    .map_err(|_| Error::Unavailable)
                }
                EffectKind::NoticeShow => {
                    if !command.permits("hook.notice.show") {
                        return Err(Error::Denied);
                    }
                    super::extension_notices::show(
                        hwnd,
                        state,
                        Some(outcome.scope()),
                        p.get("title")
                            .and_then(|v| v.as_str())
                            .unwrap_or(crate::ui_text::text("integration-extension-capability")),
                        p.get("message")
                            .and_then(|v| v.as_str())
                            .unwrap_or(crate::ui_text::text("integration-action-completed")),
                    )
                    .map_err(|_| Error::Unavailable)
                }
                // Unmigrated effects must not pretend success; retain successful independent attachment CAS updates and report partial failure as in the baseline.
                _ => Err(Error::Unsupported),
            }
        });
        if let Err(error) = result {
            failed = true;
            eprintln!("native-extension-effect: {:?}: {error:?}", effect.kind());
        }
    }
    if failed {
        Err(Error::Unsupported)
    } else {
        Ok(())
    }
}
