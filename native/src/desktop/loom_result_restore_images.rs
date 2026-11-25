//! Materialize session-inline images only in background; duplicate Data URLs share decoded pixels, without path/network access.
use super::{Error, Output, Port};
use crate::desktop::loom_image_value::ImageValue;
use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-loom")))]
#[path = "loom_result_restore_image_tests.rs"]
mod tests;

pub(super) fn decode(ports: &mut [Port], cancel: &AtomicBool) -> Result<(), Error> {
    let mut images: Vec<Option<ImageValue>> = Vec::new();
    images
        .try_reserve_exact(ports.len())
        .map_err(|_| Error::Allocation)?;
    let mut bytes = super::super::Meter(0);
    for (index, port) in ports.iter().enumerate() {
        if cancel.load(Ordering::Acquire) {
            return Err(Error::Cancelled);
        }
        let url = match &port.value {
            Output::Value(serde_json::Value::String(value)) if value.starts_with("data:image/") => {
                value
            }
            _ => {
                if let Output::Value(value) = &port.value {
                    serde_json::to_writer(&mut bytes, value).map_err(|_| Error::Budget)?;
                }
                images.push(None);
                continue;
            }
        };
        let prior = ports[..index].iter().position(|prior| match &prior.value {
            Output::Value(serde_json::Value::String(value)) => value == url,
            _ => false,
        });
        let image = match prior.and_then(|index| images[index].as_ref()) {
            Some(image) => image.clone(),
            None => ImageValue::inline(url, cancel).map_err(|_| {
                if cancel.load(Ordering::Acquire) {
                    Error::Cancelled
                } else {
                    Error::InvalidOutput
                }
            })?,
        };
        // Use ordinary official results' logical-port pixel budget; aliases count too so restoration cannot bypass result limits.
        bytes.add(image.bytes()).map_err(|_| Error::Budget)?;
        images.push(Some(image));
    }
    if cancel.load(Ordering::Acquire) {
        return Err(Error::Cancelled);
    }
    for (port, image) in ports.iter_mut().zip(images) {
        if let Some(image) = image {
            port.value = Output::from_image(image);
        }
    }
    Ok(())
}
