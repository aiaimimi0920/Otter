//! PNG 传输编码的 owner；Base64 分配计入共享预算，取消或释放时立即归还。
use crate::byte_budget::Lease;
use std::sync::atomic::{AtomicBool, Ordering};
#[path = "loom_base64_stream.rs"]
mod stream;
pub use stream::encode_png_reader;
pub struct Encoded {
    text: String,
    _lease: Lease,
}
impl Encoded {
    pub fn as_str(&self) -> &str {
        &self.text
    }
    /// 仅供会话候选转移所有权；接收者必须让租约活到 JSON 字符串释放之后。
    pub(crate) fn into_parts(self) -> (String, Lease) {
        (self.text, self._lease)
    }
}
#[derive(Debug, PartialEq)]
pub enum Error {
    Cancelled,
    Size,
    Budget,
    Allocation,
    Read,
}
pub fn encode(bytes: &[u8], cancel: &AtomicBool) -> Result<Encoded, Error> {
    encode_with(bytes, cancel, crate::pixels::reserve)
}
/// 会话内联 PNG，与纯 Base64 共用循环，但前缀在同一次预算/分配中写入。
pub fn encode_png_data_url(bytes: &[u8], cancel: &AtomicBool) -> Result<Encoded, Error> {
    const PREFIX: &str = "data:image/png;base64,";
    if cancel.load(Ordering::Acquire) {
        return Err(Error::Cancelled);
    }
    if !bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Err(Error::Size);
    }
    if bytes
        .len()
        .div_ceil(3)
        .saturating_mul(4)
        .saturating_add(PREFIX.len())
        > crate::session_document::MAX_SESSION_BYTES
    {
        return Err(Error::Size);
    }
    encode_prefixed(bytes, cancel, PREFIX, crate::pixels::reserve)
}
fn encode_with(
    bytes: &[u8],
    cancel: &AtomicBool,
    reserve: impl FnOnce(usize) -> Option<Lease>,
) -> Result<Encoded, Error> {
    encode_prefixed(bytes, cancel, "", reserve)
}
fn encode_prefixed(
    bytes: &[u8],
    cancel: &AtomicBool,
    prefix: &str,
    reserve: impl FnOnce(usize) -> Option<Lease>,
) -> Result<Encoded, Error> {
    if cancel.load(Ordering::Acquire) {
        return Err(Error::Cancelled);
    }
    if bytes.is_empty() || bytes.len() > 64 * 1024 * 1024 {
        return Err(Error::Size);
    }
    let length = (bytes.len().div_ceil(3) * 4)
        .checked_add(prefix.len())
        .ok_or(Error::Size)?;
    let mut lease = reserve(length).ok_or(Error::Budget)?;
    let mut text = String::new();
    text.try_reserve_exact(length)
        .map_err(|_| Error::Allocation)?;
    if !lease.grow_to(text.capacity()) {
        return Err(Error::Budget);
    }
    text.push_str(prefix);
    for chunk in bytes.chunks(48 * 1024) {
        if cancel.load(Ordering::Acquire) {
            return Err(Error::Cancelled);
        }
        stream::append(&mut text, chunk);
    }
    if cancel.load(Ordering::Acquire) {
        return Err(Error::Cancelled);
    }
    Ok(Encoded {
        text,
        _lease: lease,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::byte_budget::Budget;
    #[test]
    fn standard_vectors_padding_and_budget_lifetime() {
        for (source, expected) in [
            ("f", "Zg=="),
            ("fo", "Zm8="),
            ("foo", "Zm9v"),
            ("foob", "Zm9vYg=="),
            ("fooba", "Zm9vYmE="),
            ("foobar", "Zm9vYmFy"),
        ] {
            let budget = Budget::new(100);
            let encoded = encode_with(source.as_bytes(), &AtomicBool::new(false), |size| {
                budget.reserve(size)
            })
            .unwrap();
            assert_eq!(encoded.as_str(), expected);
            assert!(budget.reserve(100).is_none());
            drop(encoded);
            assert!(budget.reserve(100).is_some());
        }
    }
    #[test]
    fn admission_and_mid_prepare_cancel_release_leases() {
        let budget = Budget::new(4);
        assert!(matches!(
            encode_with(b"long", &AtomicBool::new(false), |size| budget
                .reserve(size)),
            Err(Error::Budget)
        ));
        let cancelled = AtomicBool::new(false);
        assert!(matches!(
            encode_with(b"abc", &cancelled, |size| {
                cancelled.store(true, Ordering::Release);
                budget.reserve(size)
            }),
            Err(Error::Cancelled)
        ));
        assert!(budget.reserve(4).is_some());
        assert!(matches!(
            encode(b"abc", &AtomicBool::new(true)),
            Err(Error::Cancelled)
        ));
        assert!(matches!(
            encode(b"", &AtomicBool::new(false)),
            Err(Error::Size)
        ));
    }
    #[test]
    fn prefix_is_in_the_same_allocation_budget_and_cancel_releases_it() {
        let prefix = "data:image/png;base64,";
        let size = prefix.len() + 4;
        let budget = Budget::new(size);
        let encoded = encode_prefixed(b"abc", &AtomicBool::new(false), prefix, |bytes| {
            budget.reserve(bytes)
        })
        .unwrap();
        assert_eq!(encoded.as_str(), "data:image/png;base64,YWJj");
        assert_eq!(budget.used(), size);
        drop(encoded);
        assert_eq!(budget.used(), 0);
        let cancel = AtomicBool::new(false);
        assert!(matches!(
            encode_prefixed(b"abc", &cancel, prefix, |bytes| {
                cancel.store(true, Ordering::Release);
                budget.reserve(bytes)
            }),
            Err(Error::Cancelled)
        ));
        assert_eq!(budget.used(), 0);
        assert!(matches!(
            encode_png_data_url(b"not png", &AtomicBool::new(false)),
            Err(Error::Size)
        ));
        assert!(matches!(
            encode_png_data_url(b"not png", &AtomicBool::new(true)),
            Err(Error::Cancelled)
        ));
    }
    #[test]
    fn transfer_keeps_allocation_and_budget_until_json_is_released() {
        let budget = Budget::new(32);
        let encoded =
            encode_with(b"abc", &AtomicBool::new(false), |size| budget.reserve(size)).unwrap();
        let pointer = encoded.as_str().as_ptr();
        let (text, lease) = encoded.into_parts();
        let value = serde_json::Value::String(text);
        assert_eq!(value.as_str().unwrap().as_ptr(), pointer);
        assert_eq!(budget.used(), 4);
        drop(value);
        drop(lease);
        assert_eq!(budget.used(), 0);
    }
}
