//! Bounded streaming PNG encoding; retain only one 48 KiB input block and the final budgeted Data URL.
use super::*;
use std::io::Read;
pub fn encode_png_reader(
    mut reader: impl Read,
    length: usize,
    cancel: &AtomicBool,
) -> Result<Encoded, Error> {
    const PREFIX: &str = "data:image/png;base64,";
    check(cancel)?;
    let size = length
        .div_ceil(3)
        .checked_mul(4)
        .and_then(|size| size.checked_add(PREFIX.len()))
        .ok_or(Error::Size)?;
    if length < 8 || size > crate::session_document::MAX_SESSION_BYTES {
        return Err(Error::Size);
    }
    let mut signature = [0u8; 8];
    reader.read_exact(&mut signature).map_err(|_| Error::Read)?;
    if &signature != b"\x89PNG\r\n\x1a\n" {
        return Err(Error::Size);
    }
    check(cancel)?;
    let mut lease = crate::pixels::reserve(size).ok_or(Error::Budget)?;
    let mut text = String::new();
    text.try_reserve_exact(size)
        .map_err(|_| Error::Allocation)?;
    if !lease.grow_to(text.capacity()) {
        return Err(Error::Budget);
    }
    text.push_str(PREFIX);
    let mut input = std::io::Cursor::new(signature).chain(reader);
    let mut remaining = length;
    let mut buffer = [0u8; 48 * 1024]; // Nonfinal chunks are multiples of 3; never write Base64 padding in the middle.
    while remaining != 0 {
        check(cancel)?;
        let count = remaining.min(buffer.len());
        input
            .read_exact(&mut buffer[..count])
            .map_err(|_| Error::Read)?;
        append(&mut text, &buffer[..count]);
        remaining -= count;
    }
    check(cancel)?;
    if input.read(&mut buffer[..1]).map_err(|_| Error::Read)? != 0 {
        return Err(Error::Size);
    }
    check(cancel)?;
    Ok(Encoded {
        text,
        _lease: lease,
    })
}
/// Performance audit optimization (Tier 0.3):
/// Delegate chunk Base64 encoding to `base64::engine::general_purpose::STANDARD.encode_string` (`opt-level = 3`
/// vectorized lookup engine) instead of pushing one `char` at a time (~5.6M `String::push` calls per 4 MiB PNG).
pub(super) fn append(text: &mut String, bytes: &[u8]) {
    use base64::Engine as _;
    base64::engine::general_purpose::STANDARD.encode_string(bytes, text);
}
fn check(cancel: &AtomicBool) -> Result<(), Error> {
    if cancel.load(Ordering::Acquire) {
        Err(Error::Cancelled)
    } else {
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn streaming_matches_existing_encoder_across_block_and_padding_boundaries() {
        for length in [8, 9, 10, 48 * 1024 - 1, 48 * 1024, 48 * 1024 + 1, 100_001] {
            let mut bytes = vec![123; length];
            bytes[..8].copy_from_slice(b"\x89PNG\r\n\x1a\n");
            let cancel = AtomicBool::new(false);
            assert_eq!(
                encode_png_reader(bytes.as_slice(), length, &cancel)
                    .unwrap()
                    .as_str(),
                encode_png_data_url(&bytes, &cancel).unwrap().as_str()
            );
        }
    }
    #[test]
    fn invalid_length_signature_truncation_and_cancellation_do_not_return_partial_output() {
        let cancel = AtomicBool::new(false);
        assert!(encode_png_reader(&b"invalid!"[..], 8, &cancel).is_err());
        assert!(encode_png_reader(&b"\x89PNG\r\n\x1a\n"[..], 9, &cancel).is_err());
        assert!(encode_png_reader(&b"\x89PNG\r\n\x1a\nextra"[..], 8, &cancel).is_err());
        assert!(matches!(
            encode_png_reader(std::io::empty(), usize::MAX, &cancel),
            Err(Error::Size)
        ));
        cancel.store(true, Ordering::Release);
        assert!(matches!(
            encode_png_reader(std::io::empty(), 8, &cancel),
            Err(Error::Cancelled)
        ));
    }
}
