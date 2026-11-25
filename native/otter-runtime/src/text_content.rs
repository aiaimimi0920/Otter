//! 正文和预算租约同属一个 Arc owner；历史/快照共享正文不能提前归还预算。
use crate::byte_budget::{Budget, Lease};
use std::{
    ops::Deref,
    sync::{Arc, LazyLock},
};
const LIMIT: usize = 32 * 1024 * 1024;
static CONTENT: LazyLock<Arc<Budget>> = LazyLock::new(|| Budget::new(LIMIT));
#[derive(Debug)]
pub(super) struct Content {
    value: Box<str>,
    _lease: Lease,
}
impl Content {
    pub(super) fn new(value: &str) -> Option<Self> {
        Self::with_budget(value, &CONTENT)
    }
    pub(super) fn restored(value: &str) -> Option<Self> {
        Some(Self {
            _lease: CONTENT.reserve(value.len())?,
            value: value.into(),
        })
    }
    fn with_budget(value: &str, budget: &Arc<Budget>) -> Option<Self> {
        // 输入长度是归一化后 UTF-8 正文的保守上界；分配前预留，拒绝不改计数。
        let lease = budget.reserve(value.len())?;
        Some(Self {
            value: value
                .replace("\r\n", "\n")
                .replace('\r', "\n")
                .into_boxed_str(),
            _lease: lease,
        })
    }
}
impl Deref for Content {
    type Target = str;
    fn deref(&self) -> &str {
        &self.value
    }
}
impl PartialEq for Content {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}
impl Eq for Content {}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn clones_keep_one_charge_until_last_owner_and_rejection_is_atomic() {
        let budget = Budget::new(8);
        let text = Arc::new(Content::with_budget("a\r\nb", &budget).unwrap());
        assert_eq!(&**text, "a\nb");
        assert_eq!(budget.used(), 4);
        let clone = text.clone();
        assert!(Content::with_budget("12345", &budget).is_none());
        drop(text);
        assert_eq!(budget.used(), 4);
        drop(clone);
        assert_eq!(budget.used(), 0);
    }
}
