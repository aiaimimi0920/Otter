//! 会话原文的共享 owner：导入保留 Vec allocation，保存基线直接复用带预算的编码缓冲。
use crate::pixels::Pixels;
use std::{ops::Deref, sync::Arc};
#[derive(Clone)]
pub enum Source {
    Imported(Arc<Vec<u8>>),
    Encoded(Pixels),
}
impl Source {
    pub fn same_storage(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Imported(a), Self::Imported(b)) => Arc::ptr_eq(a, b),
            (Self::Encoded(a), Self::Encoded(b)) => a.same_storage(b),
            _ => false,
        }
    }
    pub fn as_slice(&self) -> &[u8] {
        self
    }
}
impl From<Vec<u8>> for Source {
    fn from(value: Vec<u8>) -> Self {
        Self::Imported(Arc::new(value))
    }
}
impl From<Pixels> for Source {
    fn from(value: Pixels) -> Self {
        Self::Encoded(value)
    }
}
impl Deref for Source {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        match self {
            Self::Imported(bytes) => bytes,
            Self::Encoded(bytes) => bytes,
        }
    }
}
impl AsRef<[u8]> for Source {
    fn as_ref(&self) -> &[u8] {
        self
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn source_preserves_both_allocations_and_encoded_budget_until_last_owner() {
        let input = vec![1, 2, 3];
        let pointer = input.as_ptr();
        let source = Source::from(input);
        assert_eq!(source.as_ptr(), pointer);
        assert!(source.same_storage(&source.clone()));
        let budget = crate::byte_budget::Budget::new(3);
        let bytes = vec![1, 2, 3];
        let pointer = bytes.as_ptr();
        let encoded = Source::from(Pixels::from_reserved(bytes, budget.reserve(3).unwrap()));
        assert_eq!(encoded.as_ptr(), pointer);
        assert!(!source.same_storage(&encoded));
        let clone = encoded.clone();
        drop(encoded);
        assert_eq!(budget.used(), 3);
        drop(clone);
        assert_eq!(budget.used(), 0);
    }
}
