//! 源端初始矢量与当前Editor合并；不改栅格、crop或history，期间编辑由当前值覆盖。
use super::*;
use crate::projection_edit::{
    self as edit,
    journal::Context,
    source::{Frozen, Layout},
    view::View,
    workspace::Role,
    Document, Route,
};
pub(in crate::desktop) fn initial(
    basis: &edit::Basis,
    layout: Layout,
    editor: &Editor,
) -> std::result::Result<edit::Objects, edit::Error> {
    let empty = View::from_basis(basis, layout.size()?, Default::default())
        .map_err(|_| edit::Error::Annotation)?;
    let bridge = Bridge::from_checkpoint(empty, editor, layout.mapping()?, metrics)
        .map_err(|_| edit::Error::Annotation)?;
    Ok(bridge
        .current(editor, metrics)
        .map_err(|_| edit::Error::Annotation)?
        .basis_objects()
        .clone())
}
pub(in crate::desktop) fn prepare(
    owner: Option<&Owner>,
    frozen: &Frozen,
    layout: Layout,
    context: Context,
    route: Route,
    document: Document,
    editor: &Editor,
) -> Result<receiver::Prepared> {
    if document.session_id() != frozen.session() || document.basis() != &frozen.basis() {
        return Err(invalid());
    }
    if let Some(owner) = owner {
        let snapshot = owner.capture(editor);
        if snapshot.role() != Role::Source
            || snapshot.context() != &context
            || snapshot.suspended().is_some()
            || snapshot.document().session_id() != frozen.session()
        {
            return Err(invalid());
        }
        let current = snapshot.editor_view().map_err(|_| invalid())?;
        let next = snapshot
            .received(document, &current)
            .map_err(|_| invalid())?;
        return receiver::candidate(next, &current, editor);
    }
    let observed = frozen
        .view()
        .map_err(|_| invalid())?
        .resized(layout.size().map_err(|_| invalid())?);
    let bridge = Bridge::from_staged(
        observed.clone(),
        editor,
        layout.mapping().map_err(|_| invalid())?,
        metrics,
    )
    .map_err(|_| invalid())?;
    let current = bridge.current(editor, metrics).map_err(|_| invalid())?;
    let merged = edit::view::reconcile(&document, &observed, &document, &current, None)
        .map_err(|_| invalid())?;
    let snapshot = Snapshot::new(
        context,
        frozen.anchor(),
        route,
        Role::Source,
        document,
        merged.visible(),
        merged.conflict().map(|v| v.code()),
    )
    .and_then(|snapshot| snapshot.in_source_layout(layout))
    .map_err(|_| invalid())?;
    receiver::candidate(snapshot, &current, editor)
}
