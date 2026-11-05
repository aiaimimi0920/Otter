//! 原生会话坐标扩展；仅描述保存时坐标空间，不授予资源路径或执行权限。
use crate::session_placement::Space;
use serde::{Deserialize, Deserializer};
use serde_json::{Map, Value};

pub const KEY: &str = "otterNativeCoordinateSpace";
/// 显式值代表用户有意重定位；否则只用文件元数据，缺省不推测历史桌面布局。
pub fn select(explicit: Option<Space>, embedded: Option<Space>) -> Result<Space, &'static str> {
    let space = explicit.or(embedded).ok_or(
        "Session has no coordinate metadata; supply --session-origin X,Y and --session-scale",
    )?;
    space
        .physical(crate::session_window::Rect {
            x: 0.0,
            y: 0.0,
            w: 1.0,
            h: 1.0,
        })
        .map_err(|_| "Invalid session coordinate context")?;
    Ok(space)
}
#[derive(Deserialize)]
pub struct Metadata {
    version: u32,
    origin: [i32; 2],
    scale: f64,
}
impl Metadata {
    pub fn space(&self) -> Option<Space> {
        (self.version == 1 && self.scale.is_finite() && self.scale > 0.0).then_some(Space {
            origin: (self.origin[0], self.origin[1]),
            scale: self.scale,
        })
    }
}

/// 缺省表示旧格式；显式 null/坏类型不是缺省，不能静默降级后覆盖。
pub fn optional<'de, D: Deserializer<'de>>(decoder: D) -> Result<Option<Metadata>, D::Error> {
    struct Object;
    impl<'de> serde::de::Visitor<'de> for Object {
        type Value = Option<Metadata>;
        fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter.write_str("a native coordinate metadata object")
        }
        fn visit_map<A: serde::de::MapAccess<'de>>(self, map: A) -> Result<Self::Value, A::Error> {
            Metadata::deserialize(serde::de::value::MapAccessDeserializer::new(map)).map(Some)
        }
    }
    decoder.deserialize_map(Object)
}

pub fn install(root: &mut Map<String, Value>, space: Space) -> Result<(), &'static str> {
    if !space.scale.is_finite() || space.scale <= 0.0 {
        return Err("Invalid native coordinate scale");
    }
    if let Some(value) = root.get(KEY) {
        if !value.is_object() {
            return Err("Invalid coordinate object");
        }
        let metadata =
            Metadata::deserialize(value).map_err(|_| "Invalid existing coordinate metadata")?;
        if metadata.space().is_none() {
            return Err("Unsupported existing coordinate metadata");
        }
    }
    // 验证后移动未知字段，不复制其中可能较大的扩展值。
    let mut fields = match root.remove(KEY) {
        Some(Value::Object(fields)) => fields,
        None => Map::new(),
        Some(_) => unreachable!("metadata was validated as object"),
    };
    fields.insert("version".into(), 1.into());
    fields.insert(
        "origin".into(),
        serde_json::json!([space.origin.0, space.origin.1]),
    );
    fields.insert("scale".into(), serde_json::json!(space.scale));
    root.insert(KEY.into(), fields.into());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{session_document::decode, session_write, workspace_session::Topology};
    use std::sync::atomic::AtomicBool;

    #[test]
    fn context_selection_never_guesses_or_falls_back_from_invalid_override() {
        let embedded = Space {
            origin: (-10, -20),
            scale: 1.5,
        };
        assert!(select(None, None).is_err());
        assert_eq!(
            select(None, Some(embedded)).unwrap().origin,
            embedded.origin
        );
        let explicit = Space {
            origin: (1, 2),
            scale: 2.0,
        };
        assert_eq!(select(Some(explicit), Some(embedded)).unwrap().scale, 2.0);
        for scale in [f64::NAN, 0.0, -1.0, 0.01, f64::MAX] {
            assert!(select(
                Some(Space {
                    origin: (0, 0),
                    scale
                }),
                Some(embedded)
            )
            .is_err());
        }
    }

    fn document(metadata: Value) -> Vec<u8> {
        serde_json::to_vec(&serde_json::json!({"stickers":[],"links":[],KEY:metadata})).unwrap()
    }
    #[test]
    fn legacy_is_distinct_from_malformed_or_future_metadata() {
        assert!(decode(br#"{"stickers":[],"links":[]}"#)
            .unwrap()
            .coordinate_space
            .is_none());
        for metadata in [
            Value::Null,
            serde_json::json!([1, [0, 0], 1]),
            serde_json::json!({}),
            serde_json::json!({"version":2,"origin":[0,0],"scale":1}),
            serde_json::json!({"version":1,"origin":[0,0],"scale":0}),
            serde_json::json!({"version":1,"origin":[0,0],"scale":-1}),
            serde_json::json!({"version":1,"origin":[0.5,0],"scale":1}),
            serde_json::json!({"version":1,"origin":[2147483648u64,0],"scale":1}),
        ] {
            assert!(decode(&document(metadata)).is_err());
        }
    }
    #[test]
    fn native_save_roundtrips_context_and_preserves_unknown_fields() {
        let source = document(
            serde_json::json!({"version":1,"origin":[0,0],"scale":1,"futureHint":{"keep":true}}),
        );
        let space = Space {
            origin: (-1920, -200),
            scale: 1.5,
        };
        let candidate = session_write::assemble_owned_with_space(
            &source,
            &Topology {
                stickers: vec![],
                links: vec![],
                groups: vec![],
            },
            &AtomicBool::new(false),
            Some(space),
            |_, _| panic!("Empty session has no window"),
        )
        .unwrap();
        let loaded = decode(&candidate.bytes).unwrap();
        let actual = loaded.coordinate_space.unwrap();
        assert_eq!(actual.origin, space.origin);
        assert_eq!(actual.scale, space.scale);
        assert_eq!(loaded.revision, 1);
        let root: Value = serde_json::from_slice(&candidate.bytes).unwrap();
        assert_eq!(root[KEY]["futureHint"]["keep"], true);
    }
    #[test]
    fn invalid_write_does_not_mutate_existing_extension() {
        let mut root = Map::new();
        root.insert(
            KEY.into(),
            serde_json::json!({"version":9,"origin":[0,0],"scale":1}),
        );
        let previous = root.clone();
        assert!(install(
            &mut root,
            Space {
                origin: (0, 0),
                scale: 1.0
            }
        )
        .is_err());
        assert_eq!(root, previous);
        for scale in [f64::NAN, f64::INFINITY, 0.0, -1.0] {
            assert!(install(
                &mut root,
                Space {
                    origin: (0, 0),
                    scale
                }
            )
            .is_err());
            assert_eq!(root, previous);
        }
    }
}
