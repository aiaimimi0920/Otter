//! 只读恢复会话中的效果框；身份匹配不依赖 JSON 顺序，不从笔迹重新猜测几何。
use super::*;
use serde::Deserialize;
use serde_json::value::RawValue;
use std::borrow::Cow;
use std::collections::HashMap;

pub(super) fn read(raw: Option<&RawValue>, cancel: &AtomicBool) -> Result<HashMap<String, Rect>> {
    #[derive(Deserialize)]
    struct State<'a> {
        #[serde(borrow)]
        elements: Vec<&'a RawValue>,
    }
    #[derive(Deserialize)]
    struct Element<'a> {
        #[serde(borrow)]
        id: Cow<'a, str>,
        #[serde(rename = "type", borrow)]
        kind: Cow<'a, str>,
        x: Option<f64>,
        y: Option<f64>,
        w: Option<f64>,
        h: Option<f64>,
    }
    let invalid = || Error::new(E_INVALIDARG, "Invalid persisted blur box");
    let raw = raw
        .filter(|raw| raw.get().len() <= 8 * 1024 * 1024)
        .ok_or_else(invalid)?;
    let state: State<'_> = serde_json::from_str(raw.get()).map_err(|_| invalid())?;
    if state.elements.len() > 4096 {
        return Err(invalid());
    }
    let mut boxes = HashMap::new();
    for raw in state.elements {
        check(cancel)?;
        let element: Element<'_> = serde_json::from_str(raw.get()).map_err(|_| invalid())?;
        if element.kind != "blur" {
            continue;
        }
        let rect = Rect {
            x: element.x.ok_or_else(invalid)?,
            y: element.y.ok_or_else(invalid)?,
            width: element.w.ok_or_else(invalid)?,
            height: element.h.ok_or_else(invalid)?,
        };
        if element.id.is_empty()
            || element.id.len() > 256
            || element.id.chars().any(char::is_control)
            || ![rect.x, rect.y, rect.width, rect.height]
                .iter()
                .all(|v| v.is_finite())
            || rect.x.abs() > 1_000_000.0
            || rect.y.abs() > 1_000_000.0
            || rect.width <= 0.0
            || rect.height <= 0.0
            || rect.width > 8192.0
            || rect.height > 8192.0
        {
            return Err(invalid());
        }
        boxes.try_reserve(1).map_err(|_| {
            Error::new(
                windows::Win32::Foundation::E_OUTOFMEMORY,
                "Cannot reserve blur boxes",
            )
        })?;
        if boxes.insert(element.id.into_owned(), rect).is_some() {
            return Err(invalid());
        }
    }
    Ok(boxes)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn escaped_json_identity_and_kind_match_the_restored_annotation() {
        let raw = RawValue::from_string(
            r#"{"elements":[{"id":"\u0061","type":"bl\u0075r","x":1,"y":2,"w":3,"h":4}]}"#
                .to_owned(),
        )
        .unwrap();
        let boxes = read(Some(&raw), &AtomicBool::new(false)).unwrap();
        assert_eq!(
            boxes["a"],
            Rect {
                x: 1.0,
                y: 2.0,
                width: 3.0,
                height: 4.0
            }
        );
        assert!(read(Some(&raw), &AtomicBool::new(true)).is_err());
    }
}
