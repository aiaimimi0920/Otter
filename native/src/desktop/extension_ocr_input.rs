//! Bounded whitelist projection for baseline ocr-text.v1; output named text/geometry only, not cross-plugin opaque state.
use crate::{loom_extension_session::Error, unit_attachment_store::Store};
use serde_json::{json, Value};
fn text(value: &Value) -> Result<&str, Error> {
    value
        .as_str()
        .filter(|s| !s.trim().is_empty() && s.chars().count() <= 16_384)
        .ok_or(Error::Limit)
}
fn number(value: &Value) -> Result<f64, Error> {
    value.as_f64().filter(|v| v.is_finite()).ok_or(Error::Wire)
}
fn dimension(value: &Value) -> Result<f64, Error> {
    number(value).and_then(|v| {
        if (1. ..=100_000.).contains(&v) {
            Ok(v)
        } else {
            Err(Error::Wire)
        }
    })
}
fn color<'a>(value: &'a Value, fallback: &'a str) -> &'a str {
    value
        .as_str()
        .filter(|s| {
            s.len() == 7
                && s.starts_with('#')
                && s.as_bytes()[1..].iter().all(u8::is_ascii_hexdigit)
        })
        .unwrap_or(fallback)
}
pub(super) fn prepare(
    store: &Store,
    revision: u64,
    language: &str,
    provider: &str,
) -> Result<Option<Value>, Error> {
    let Some(item) = store.items().iter().find(|i| {
        i.attachment.plugin() == crate::extension_visual::OCR
            && i.attachment.type_id() == crate::extension_visual::OCR_RESULT
            && i.attachment.schema() == "1"
    }) else {
        return Ok(None);
    };
    let p = item.attachment.payload().ok_or(Error::Wire)?;
    if p.get("fullText").is_none_or(|v| v.is_null() || v == "") {
        return Ok(None);
    }
    let full = text(&p["fullText"])?;
    let width = dimension(&p["sourceWidth"])?;
    let height = dimension(&p["sourceHeight"])?;
    let blocks = p["textBlocks"]
        .as_array()
        .filter(|v| v.len() <= 128)
        .ok_or(Error::Limit)?;
    let blocks = blocks.iter().map(|b| {
        let t = text(&b["text"])?;
        let (x, y, w, h) = (number(&b["left"])?, number(&b["top"])?, number(&b["width"])?, number(&b["height"])?);
        if x < 0. || y < 0. || w <= 0. || h <= 0. || x + w > width + 1. || y + h > height + 1. { return Err(Error::Wire); }
        Ok(json!({"text":t,"left":x,"top":y,"width":w,"height":h,
            "textColor":color(&b["textColor"], "#ffffff"), "backgroundColor":color(&b["backgroundColor"], "#111111")}))
    }).collect::<Result<Vec<_>, Error>>()?;
    let mut source =
        json!({"attachmentId":item.attachment.id(), "revision":item.attachment.revision()});
    if let Some(digest) = &item.digest {
        source["digest"] = digest.as_ref().into();
    }
    let mut input = json!({"text":full, "targetLanguage":language, "sourceRevision":revision,
        "sourceAttachment":source, "sourceWidth":width, "sourceHeight":height, "textBlocks":blocks});
    if provider != "auto" {
        input["providerMode"] = provider.into();
    }
    Ok(Some(input))
}
#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-loom")))]
#[path = "extension_ocr_input_tests.rs"]
mod tests;
