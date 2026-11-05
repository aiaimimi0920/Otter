//! 只读协作接收的Editor候选；先核Context/单调Document/当前view，矢量与可选新底图原子交换。
use super::*;
use crate::projection_edit::{
    self as edit,
    journal::Context,
    view::{Size, View},
    workspace::Role,
    Document, Route,
};

pub(in crate::desktop) struct Prepared {
    snapshot: Snapshot,
    replacement: Option<edit::editor::Prepared>,
    unchanged: Option<Bridge>,
    pub(in crate::desktop) changed: bool,
}
pub(in crate::desktop) fn prepare(
    owner: Option<&Owner>,
    document: Document,
    context: Context,
    projection: &str,
    route: Route,
    frame: crate::crop_geometry::Rect,
    editor: &Editor,
) -> Result<Prepared> {
    let (previous, current) = if let Some(owner) = owner {
        let snapshot = owner.capture(editor);
        if snapshot.suspended().is_some()
            || snapshot.role() != Role::Receiver
            || snapshot.context() != &context
            || snapshot.projection_id() != projection
            || snapshot.route() != route
        {
            return Err(invalid());
        }
        let view = snapshot.editor_view().map_err(|_| invalid())?;
        (snapshot, view)
    } else {
        if editor.crop_view().is_some() {
            return Err(invalid());
        }
        let empty = View::from_basis(
            document.basis(),
            Size::new(frame.width, frame.height).map_err(|_| invalid())?,
            Default::default(),
        )
        .map_err(|_| invalid())?;
        let mapping = crate::session_vector_write::Mapping::from_components(
            1.0,
            crate::viewport::Point {
                x: -frame.x as f32,
                y: -frame.y as f32,
            },
        )
        .map_err(|_| invalid())?;
        let bridge = Bridge::from_checkpoint(empty.clone(), editor, mapping, metrics)
            .map_err(|_| invalid())?;
        let current = bridge.current(editor, metrics).map_err(|_| invalid())?;
        // 首次接入时旧Editor的标注是本地变化，不伪装成已经收到的远端基线。
        let sticky = (document.mode() == edit::Mode::OneWay
            && !empty
                .changes_to(&current)
                .map_err(|_| invalid())?
                .is_empty())
        .then_some(edit::Code::ReadOnly);
        let merged = edit::view::reconcile(&document, &empty, &document, &current, sticky)
            .map_err(|_| invalid())?;
        let initial = Snapshot::new(
            context,
            projection,
            route,
            Role::Receiver,
            document,
            merged.visible(),
            merged.conflict().map(|v| v.code()),
        )
        .and_then(|value| value.in_editor_frame(frame))
        .map_err(|_| invalid())?;
        return candidate(initial, &current, editor);
    };
    let next = previous
        .received(document, &current)
        .map_err(|_| invalid())?;
    let changed = owner.unwrap().snapshot != next;
    let mut prepared = candidate(next, &current, editor)?;
    prepared.changed = changed;
    Ok(prepared)
}
pub(super) fn candidate(snapshot: Snapshot, current: &View, editor: &Editor) -> Result<Prepared> {
    let visible = snapshot.editor_view().map_err(|_| invalid())?;
    let mapping = snapshot.to_view().map_err(|_| invalid())?;
    let (replacement, unchanged) = if visible == *current {
        (
            None,
            Some(
                Bridge::from_checkpoint(visible, editor, mapping, metrics)
                    .map_err(|_| invalid())?,
            ),
        )
    } else {
        (
            Some(
                edit::editor::prepare_mapped(&visible, editor, mapping, metrics)
                    .map_err(|_| invalid())?,
            ),
            None,
        )
    };
    Ok(Prepared {
        snapshot,
        replacement,
        unchanged,
        changed: true,
    })
}
impl Prepared {
    pub(in crate::desktop) fn apply(
        self,
        editor: &mut Editor,
        base: Option<Pixels>,
    ) -> Result<Owner> {
        let bridge = if let Some(replacement) = self.replacement {
            replacement
                .apply_rebased(editor, base)
                .map_err(|_| invalid())?
                .ok_or_else(invalid)?
        } else {
            if let Some(base) = base {
                if !editor.replace_source(base) {
                    return Err(invalid());
                }
            }
            self.unchanged.unwrap()
        };
        Ok(Owner {
            snapshot: self.snapshot,
            bridge: Some(bridge),
            base: Base::capture(editor),
            stamp: Stamp(Arc::default()),
        })
    }
}
