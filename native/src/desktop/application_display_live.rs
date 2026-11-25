//! Live 连线快照与显式刷新：仅当前 owner 的成功读回进入显示图并通知，不覆盖原图。
use super::*;
use crate::workspace_id::UnitId;
pub(super) struct Entry {
    pub unit: UnitId,
    pub key: String,
    pub image: Image,
}
impl Application {
    fn wants_live_display(&self, unit: &UnitId) -> bool {
        self.workspace
            .links()
            .iter()
            .any(|link| &link.from_unit == unit)
    }

    pub(super) fn poll_live_display(&self) -> bool {
        let mut cache = self.display_composition.borrow_mut();
        let changed = cache.retain_live(|unit, identity| {
            self.wants_live_display(unit)
                && self.live_sessions.iter().any(|session| {
                    session.unit_id() == unit
                        && session.is_alive()
                        && session.belongs_to(&self.workspace)
                        && std::sync::Arc::ptr_eq(identity, &session.input_identity())
                })
        });
        if changed {
            self.display_cache.borrow_mut().take();
        }
        if !self.modals.blocked() {
            for session in &self.live_sessions {
                if !session.take_snapshot_request() {
                    continue;
                }
                if !session.is_alive()
                    || !session.belongs_to(&self.workspace)
                    || !self.wants_live_display(session.unit_id())
                    || self
                        .workspace
                        .group_of(session.unit_id())
                        .is_some_and(|group| group.locked || group.hidden)
                {
                    continue;
                }
                match session.snapshot_frame() {
                    Ok(Some(frame)) => {
                        if cache
                            .refresh_live(
                                session.unit_id().clone(),
                                session.input_identity(),
                                frame,
                            )
                            .is_err()
                        {
                            eprintln!("Live display refresh budget exhausted");
                        }
                    }
                    _ => eprintln!("Live display refresh has no available frame"),
                }
            }
        }
        let mut pending = false;
        for session in self.live_sessions.iter().filter(|session| {
            session.is_alive()
                && session.belongs_to(&self.workspace)
                && self.wants_live_display(session.unit_id())
        }) {
            pending |= cache.poll_live_source(
                session.unit_id(),
                session.input_identity(),
                || session.snapshot_frame().map_err(|_| ()),
                || self.execution_wake.callback(),
            );
        }
        let updates = if self.modals.blocked() {
            Vec::new()
        } else {
            cache.take_live_updates()
        };
        drop(cache);
        // 读回成功且 retain 已复核当前 owner 后才通知；不跨消息调用持有缓存借用。
        for (unit, _) in updates {
            unsafe {
                self.propagate_live_snapshot(&unit);
            }
        }
        pending
    }

    // 只有显示图需要重建时才创建键和像素引用，顺序仍与 Live 会话一致。
    pub(super) fn live_display_images(&self) -> Vec<Entry> {
        let cache = self.display_composition.borrow();
        self.live_sessions
            .iter()
            .filter(|session| session.is_alive() && session.belongs_to(&self.workspace))
            .filter_map(|session| {
                let unit = session.unit_id();
                let image = cache.live_image(unit, &session.input_identity())?;
                Some(Entry {
                    unit: unit.clone(),
                    key: format!("live:{unit}"),
                    image: image.clone(),
                })
            })
            .collect()
    }
}
pub(super) fn publish(
    doc: &crate::desktop::document_window::DocumentWindow,
    source: &UnitId,
    image: &Image,
    art: bool,
) {
    let canvas = doc.state.canvas.borrow();
    // Live 不拥有编辑器；静态中继只展示快照，Art 保留目标自己的显示覆盖层。
    let editor = art.then_some(&canvas.editor);
    let mut frame = doc.state.resolved_display.borrow_mut();
    if frame
        .as_ref()
        .is_some_and(|frame| frame.matches(source, image, editor, art))
    {
        return;
    }
    let next = if art {
        match Frame::capture_overlay(
            source.clone(),
            doc.state.unit_id.clone(),
            image.clone(),
            &canvas.editor,
        ) {
            Ok(frame) => frame,
            Err(error) => {
                eprintln!("Live Art overlay mapping failed: {error:?}");
                return;
            }
        }
    } else {
        Frame::capture(source.clone(), image.clone(), None)
    };
    *frame = Some(next);
    drop(frame);
    drop(canvas);
    super::invalidate(doc);
}
