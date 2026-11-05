//! PNG响应中的编辑文档有独立256KiB门，并与同一压缩图源的摘要/实际尺寸绑定。
use super::Error;
use crate::projection_edit::Document;
use serde_json::value::RawValue;

pub(super) fn parse(
    raw: Option<&RawValue>,
    digest: &str,
    width: u32,
    height: u32,
) -> Result<Option<Document>, Error> {
    let Some(raw) = raw else { return Ok(None) };
    let document = Document::parse(raw.get().as_bytes()).map_err(|_| Error::Response)?;
    if document.basis().digest() != digest || document.basis().size() != (width, height) {
        return Err(Error::Response);
    }
    Ok(Some(document))
}

pub(super) fn initial(
    value: &super::Verified,
    ticket: &crate::projection_delivery::journal::Ticket,
    scope: crate::session_projection_scope::Scope,
    rect: crate::session_window::Rect,
) -> Result<
    Option<(
        crate::projection_edit::workspace::Snapshot,
        serde_json::Value,
    )>,
    Error,
> {
    use crate::projection_edit::{
        journal::Context,
        view::{Size, View},
        workspace::{Role, Snapshot},
        Route,
    };
    let Some(document) = value.editing() else {
        return Ok(None);
    };
    let view = View::from_document(
        document,
        Size::new(rect.w, rect.h).map_err(|_| Error::Response)?,
    )
    .map_err(|_| Error::Response)?;
    let context = Context::new(scope, value.unit(), ticket.origin(), ticket.device())
        .map_err(|_| Error::Response)?;
    let snapshot = Snapshot::new(
        context,
        value.envelope().projection_id(),
        if value.received.offline.is_some() {
            Route::Offline
        } else {
            Route::Shared
        },
        Role::Receiver,
        document.clone(),
        &view,
        None,
    )
    .map_err(|_| Error::Response)?;
    let objects = view.local_objects().map_err(|_| Error::Response)?;
    Ok(Some((
        snapshot,
        serde_json::json!({"serialCounter":0,"elements":objects.values().collect::<Vec<_>>()}),
    )))
}
