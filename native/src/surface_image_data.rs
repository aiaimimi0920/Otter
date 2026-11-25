//! Bounded data URL decoding; percent encoding is not form encoding, and base64 follows forgiving rules.
use super::ResourceError;
#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-surface")))]
#[path = "surface_image_data_tests.rs"]
mod tests;
const LIMIT: usize = 1024 * 1024;
fn whitespace(byte: u8) -> bool {
    matches!(byte, b'\t' | b'\n' | b'\x0c' | b'\r' | b' ')
}
pub(super) fn mime(source: &str) -> Result<String, ResourceError> {
    header_mime(&parse(source)?)
}
fn parse(source: &str) -> Result<url::Url, ResourceError> {
    if source.len() > LIMIT {
        return Err(ResourceError::Limit);
    }
    if !source.starts_with("data:image/") {
        return Err(ResourceError::Unsupported);
    }
    let mut url = url::Url::parse(source).map_err(|_| ResourceError::Input)?;
    url.set_fragment(None);
    Ok(url)
}
fn header_mime(url: &url::Url) -> Result<String, ResourceError> {
    let header = url
        .as_str()
        .strip_prefix("data:")
        .and_then(|v| v.split_once(','))
        .ok_or(ResourceError::Input)?
        .0;
    // Declared SVG is handled by a restricted background XML child and must not fall back to bitmap sniffing.
    Ok(crate::surface_image_format::essence(header)
        .unwrap_or_default()
        .to_ascii_lowercase())
}
pub(crate) fn decode(source: &str, cancelled: impl Fn() -> bool) -> Result<Vec<u8>, ResourceError> {
    if cancelled() {
        return Err(ResourceError::Identity);
    }
    let url = parse(source)?;
    header_mime(&url)?;
    let (header, body) = url
        .as_str()
        .strip_prefix("data:")
        .and_then(|v| v.split_once(','))
        .ok_or(ResourceError::Input)?;
    if body.len() > LIMIT {
        return Err(ResourceError::Limit);
    }
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(body.len())
        .map_err(|_| ResourceError::Limit)?;
    let raw = body.as_bytes();
    let mut index = 0;
    let mut next_check = 0;
    fn hex(b: u8) -> Option<u8> {
        match b {
            b'0'..=b'9' => Some(b - b'0'),
            b'a'..=b'f' => Some(b - b'a' + 10),
            b'A'..=b'F' => Some(b - b'A' + 10),
            _ => None,
        }
    }
    while index < raw.len() {
        // Percent sequences consume three bytes at once and may cross a check boundary; exact index alignment cannot be required.
        if index >= next_check {
            if cancelled() {
                return Err(ResourceError::Identity);
            }
            next_check += 4096;
        }
        if raw[index] == b'%' && index + 2 < raw.len() {
            if let (Some(a), Some(b)) = (hex(raw[index + 1]), hex(raw[index + 2])) {
                bytes.push(a * 16 + b);
                index += 3;
                continue;
            }
        }
        bytes.push(raw[index]);
        index += 1;
    }
    if cancelled() {
        return Err(ResourceError::Identity);
    }
    let header = header.trim_matches(|c: char| c.is_ascii_whitespace());
    let encoded = header
        .rsplit_once(';')
        .is_some_and(|(_, tail)| tail.trim_start_matches(' ').eq_ignore_ascii_case("base64"));
    if !encoded {
        return Ok(bytes);
    }
    bytes.retain(|b| !whitespace(*b));
    if bytes.len() % 4 == 0 {
        for _ in 0..2 {
            if bytes.last() == Some(&b'=') {
                bytes.pop();
            }
        }
    }
    if bytes.len() % 4 == 1 {
        return Err(ResourceError::Input);
    }
    // Convert in place to avoid another encoded buffer; discard residual bits shorter than eight under the forgiving algorithm.
    let (mut accumulator, mut bits, mut write) = (0u32, 0u32, 0usize);
    for read in 0..bytes.len() {
        if read % 4096 == 0 && cancelled() {
            return Err(ResourceError::Identity);
        }
        let value = match bytes[read] {
            b'A'..=b'Z' => bytes[read] - b'A',
            b'a'..=b'z' => bytes[read] - b'a' + 26,
            b'0'..=b'9' => bytes[read] - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            _ => return Err(ResourceError::Input),
        };
        accumulator = (accumulator << 6) | u32::from(value);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            bytes[write] = (accumulator >> bits) as u8;
            write += 1;
        }
    }
    bytes.truncate(write);
    if cancelled() {
        return Err(ResourceError::Identity);
    }
    Ok(bytes)
}
