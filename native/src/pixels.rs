//! 共享像素与预算租约位于同一个 Arc 内，缓存或导出克隆不能提前释放预算。
use crate::byte_budget::{Budget, Lease};
use std::{
    ops::Deref,
    sync::{Arc, LazyLock},
};

pub const DECODED_PIXEL_LIMIT: usize = 512 * 1024 * 1024;
static DECODED: LazyLock<Arc<Budget>> = LazyLock::new(|| Budget::new(DECODED_PIXEL_LIMIT));

pub fn reserve(bytes: usize) -> Option<Lease> {
    DECODED.reserve(bytes)
}

#[derive(Debug)]
pub(crate) struct Storage {
    bytes: Vec<u8>,
    _lease: Lease,
}
#[derive(Debug, Clone)]
pub struct Pixels(Arc<Storage>);
impl Pixels {
    pub(crate) fn from_reserved(bytes: Vec<u8>, lease: Lease) -> Self {
        assert!(
            bytes.capacity() <= lease.bytes(),
            "pixel allocation exceeds reserved capacity"
        );
        Self(Arc::new(Storage {
            bytes,
            _lease: lease,
        }))
    }
    pub fn same_storage(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
    pub(crate) fn downgrade(&self) -> std::sync::Weak<Storage> {
        Arc::downgrade(&self.0)
    }
}
impl Deref for Pixels {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        &self.0.bytes
    }
}
impl PartialEq for Pixels {
    fn eq(&self, other: &Self) -> bool {
        **self == **other
    }
}
impl Eq for Pixels {}

#[cfg(test)]
impl From<Vec<u8>> for Pixels {
    fn from(bytes: Vec<u8>) -> Self {
        let lease = reserve(bytes.capacity()).expect("test pixel budget");
        Self::from_reserved(bytes, lease)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn clones_hold_one_lease_until_last_reference_is_dropped() {
        let budget = Budget::new(100);
        let data = vec![1; 80];
        let pixels = Pixels::from_reserved(data, budget.reserve(80).unwrap());
        let clone = pixels.clone();
        assert!(clone.same_storage(&pixels));
        assert_eq!(budget.used(), 80);
        drop(pixels);
        assert_eq!(budget.used(), 80);
        drop(clone);
        assert_eq!(budget.used(), 0);
    }
}
