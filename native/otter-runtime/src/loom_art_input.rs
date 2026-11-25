//! Descriptors must correspond one-to-one with candidate image ports; the encoder holds no shared memory, so callers must retain resource owners.
use super::*;
pub enum Input<'a> {
    InlinePng {
        base64: &'a str,
        width: u32,
        height: u32,
    },
    SharedRgba {
        handle: &'a str,
        width: u32,
        height: u32,
        size: usize,
    },
}
impl Input<'_> {
    fn dimensions(&self) -> (u32, u32) {
        match self {
            Self::InlinePng { width, height, .. } | Self::SharedRgba { width, height, .. } => {
                (*width, *height)
            }
        }
    }
    fn write_validated(&self, json: &mut Json) -> Result<(), Error> {
        match self {
            Self::InlinePng { base64, .. } => {
                // Enter only through validated borrows; standard Base64 contains no characters requiring JSON escaping.
                json.raw("{\"kind\":\"inline_resource\",\"mime\":\"image/png\",\"dataBase64\":\"")?;
                json.raw(base64)?;
                json.raw("\"")?;
            }
            Self::SharedRgba { handle, size, .. } => {
                json.raw("{\"kind\":\"shared_memory\",\"handle\":")?;
                json.string(handle)?;
                json.raw(",\"size\":")?;
                json.raw(&size.to_string())?;
                json.raw(",\"format\":\"rgba8\"")?;
            }
        }
        let (width, height) = self.dimensions();
        json.raw(",\"width\":")?;
        json.raw(&width.to_string())?;
        json.raw(",\"height\":")?;
        json.raw(&height.to_string())?;
        json.raw("}")
    }
}
pub(super) struct ValidatedInputs<'a, 'data>(&'a [(&'data str, Input<'data>)]);

impl<'a, 'data> ValidatedInputs<'a, 'data> {
    pub(super) fn new(
        ports: &[String],
        inputs: &'a [(&'data str, Input<'data>)],
    ) -> Result<Self, Error> {
        validate(ports, inputs)?;
        Ok(Self(inputs))
    }

    pub(super) fn write(&self, json: &mut Json) -> Result<(), Error> {
        let mut first = true;
        for (name, input) in self.0 {
            json.field(name, &mut first)?;
            input.write_validated(json)?;
        }
        Ok(())
    }
}

fn validate(ports: &[String], inputs: &[(&str, Input<'_>)]) -> Result<(), Error> {
    if inputs.len() > 32 || inputs.len() != ports.len() {
        return Err(Error::InvalidValue);
    }
    let (mut rgba, mut encoded) = (0u64, 0usize);
    for (index, (name, input)) in inputs.iter().enumerate() {
        if name.is_empty()
            || name.len() > 256
            || name.chars().any(char::is_control)
            || inputs[..index].iter().any(|(previous, _)| previous == name)
            || !ports.iter().any(|port| port == name)
        {
            return Err(Error::InvalidValue);
        }
        let (width, height) = input.dimensions();
        let pixels = u64::from(width) * u64::from(height);
        if width == 0 || height == 0 || width > 32768 || height > 32768 || pixels > 64 * 1024 * 1024
        {
            return Err(Error::InvalidValue);
        }
        let size = pixels * 4;
        rgba += size;
        if rgba > 256 * 1024 * 1024 {
            return Err(Error::MessageTooLarge);
        }
        match input {
            Input::InlinePng { base64, .. } => {
                if base64.is_empty() || base64.len() > MAX_INLINE_CHARS || base64.len() % 4 != 0 {
                    return Err(Error::InvalidValue);
                }
                let data = base64.trim_end_matches('=');
                let padding = base64.len() - data.len();
                if padding > 2
                    || !data
                        .bytes()
                        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'/'))
                {
                    return Err(Error::InvalidValue);
                }
                if base64.len() / 4 * 3 - padding > 64 * 1024 * 1024 {
                    return Err(Error::MessageTooLarge);
                }
                if padding != 0 {
                    let last = *data.as_bytes().last().ok_or(Error::InvalidValue)?;
                    let bits = match last {
                        b'A'..=b'Z' => last - b'A',
                        b'a'..=b'z' => last - b'a' + 26,
                        b'0'..=b'9' => last - b'0' + 52,
                        b'+' => 62,
                        b'/' => 63,
                        _ => return Err(Error::InvalidValue),
                    };
                    if (padding == 2 && bits & 15 != 0) || (padding == 1 && bits & 3 != 0) {
                        return Err(Error::InvalidValue);
                    }
                }
                encoded += base64.len();
                if encoded > MAX_INLINE_CHARS * 2 {
                    return Err(Error::MessageTooLarge);
                }
            }
            Input::SharedRgba {
                handle,
                size: actual,
                ..
            } => {
                if *actual as u64 != size
                    || !handle.starts_with("otter-art-input-")
                    || handle.len() <= 16
                    || handle.len() > 256
                    || handle.chars().any(char::is_control)
                {
                    return Err(Error::InvalidValue);
                }
            }
        }
    }
    Ok(())
}
