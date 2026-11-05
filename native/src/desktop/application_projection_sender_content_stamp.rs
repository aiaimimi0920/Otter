//! 持续更新只缓存弱身份；64项历史内容不能延长会话原文或像素预算租约。
use super::*;
use crate::session_source::Source;
use std::sync::{Arc, Weak};

pub(super) enum SourceStamp {
    Imported(Weak<Vec<u8>>),
    Encoded(Weak<crate::pixels::Storage>),
}
impl SourceStamp {
    pub(super) fn capture(source: &Source) -> Self {
        match source {
            Source::Imported(bytes) => Self::Imported(Arc::downgrade(bytes)),
            Source::Encoded(bytes) => Self::Encoded(bytes.downgrade()),
        }
    }
    pub(super) fn matches(&self, source: &Source) -> bool {
        match (self, source) {
            (Self::Imported(a), Source::Imported(b)) => a.ptr_eq(&Arc::downgrade(b)),
            (Self::Encoded(a), Source::Encoded(b)) => a.ptr_eq(&b.downgrade()),
            _ => false,
        }
    }
}
pub(in crate::desktop::application::projection_sender) struct ContentStamp {
    unit: UnitId,
    workspace: crate::workspace::IdentityStamp,
    canonical: SourceStamp,
    display: display_snapshot::Stamp,
    pixels: snapshot::Stamp,
    geometry: Option<geometry::Captured>,
}
impl From<Scope> for ContentStamp {
    fn from(scope: Scope) -> Self {
        Self {
            unit: scope.unit,
            workspace: scope.workspace,
            canonical: SourceStamp::capture(&scope.binding_source),
            display: scope.display,
            pixels: scope.pixels,
            geometry: scope.geometry,
        }
    }
}
impl ContentStamp {
    pub(in crate::desktop::application::projection_sender) unsafe fn matches(
        &self,
        app: &Application,
    ) -> bool {
        self.workspace.matches(&app.workspace)
            && app
                .workspace_checkpoint
                .as_ref()
                .and_then(|owner| owner.sender_source())
                .is_some_and(|source| self.canonical.matches(source))
            && display_matches(
                app,
                &self.unit,
                &self.display,
                &self.pixels,
                self.geometry.as_ref(),
            )
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn content_identity_cache_does_not_retain_imported_or_encoded_source_budget() {
        let imported: Source = vec![1; 1024].into();
        let Source::Imported(owner) = &imported else {
            unreachable!()
        };
        let weak = Arc::downgrade(owner);
        let stamp = SourceStamp::capture(&imported);
        assert!(stamp.matches(&imported));
        assert!(!stamp.matches(&vec![1; 1024].into()));
        drop(imported);
        assert!(weak.upgrade().is_none());
        let budget = crate::byte_budget::Budget::new(1024);
        let encoded: Source =
            crate::pixels::Pixels::from_reserved(vec![1; 1024], budget.reserve(1024).unwrap())
                .into();
        let stamp = SourceStamp::capture(&encoded);
        assert!(stamp.matches(&encoded));
        drop(encoded);
        assert_eq!(budget.used(), 0);
    }
}
