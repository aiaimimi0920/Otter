//! 会话图片引用的只读视图；不解码、不读取路径，也不自动请求网络资源。
use serde::{
    de::{self, Visitor},
    Deserializer,
};
use std::{borrow::Cow, fmt};

#[derive(Debug, PartialEq)]
pub struct References<'a> {
    pub source: Option<&'a str>,
    pub preview: Option<&'a str>,
    pub rasterized_annotations: Option<&'a str>,
    pub original_file: Option<&'a str>,
    pub restored_preview_locked: bool,
}
impl<'a> References<'a> {
    pub fn from_window(window: &'a crate::session_window::Window<'_>) -> Self {
        let source = window.src.as_deref();
        let preview = window
            .preview_src
            .as_deref()
            .filter(|value| !value.is_empty() && Some(*value) != source);
        Self {
            source,
            preview,
            rasterized_annotations: window
                .rasterized_annotation_layer_src
                .as_deref()
                .filter(|value| !value.is_empty()),
            original_file: window
                .file_path
                .as_deref()
                .filter(|value| !value.is_empty()),
            restored_preview_locked: window.is_art() && preview.is_some(),
        }
    }
}

/// Option<Cow<str>> 默认派生会拥有字符串；显式 visitor 保留未转义 JSON 字符串的借用。
pub(crate) fn optional_text<'de: 'a, 'a, D: Deserializer<'de>>(
    decoder: D,
) -> Result<Option<Cow<'a, str>>, D::Error> {
    struct Text;
    impl<'de> Visitor<'de> for Text {
        type Value = Cow<'de, str>;
        fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("a session resource string")
        }
        fn visit_borrowed_str<E: de::Error>(self, value: &'de str) -> Result<Self::Value, E> {
            Ok(Cow::Borrowed(value))
        }
        fn visit_str<E: de::Error>(self, value: &str) -> Result<Self::Value, E> {
            Ok(Cow::Owned(value.into()))
        }
        fn visit_string<E: de::Error>(self, value: String) -> Result<Self::Value, E> {
            Ok(Cow::Owned(value))
        }
    }
    struct Optional;
    impl<'de> Visitor<'de> for Optional {
        type Value = Option<Cow<'de, str>>;
        fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("a resource string or null")
        }
        fn visit_none<E: de::Error>(self) -> Result<Self::Value, E> {
            Ok(None)
        }
        fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
            Ok(None)
        }
        fn visit_some<D: Deserializer<'de>>(self, decoder: D) -> Result<Self::Value, D::Error> {
            decoder.deserialize_str(Text).map(Some)
        }
    }
    decoder.deserialize_option(Optional)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn raw_data_urls_are_borrowed_and_identical_preview_is_deduplicated() {
        let source = format!("data:image/png;base64,{}", "A".repeat(128 * 1024));
        let bytes = serde_json::to_vec(&serde_json::json!({"stickers":[{
            "id":"s","x":-10,"y":0,"w":100,"h":100,"type":"art",
            "src":source,"previewSrc":source,"rasterizedAnnotationLayerSrc":"data:image/png;base64,AAAA"
        }],"links":[]})).unwrap();
        let document = crate::session_document::decode(&bytes).unwrap();
        let window = &document.windows[0];
        assert!(matches!(window.src, Some(Cow::Borrowed(_))));
        assert!(matches!(window.preview_src, Some(Cow::Borrowed(_))));
        let references = References::from_window(window);
        assert_eq!(references.source, Some(source.as_str()));
        assert!(references.preview.is_none());
        assert!(!references.restored_preview_locked);
        let pointer = references.source.unwrap().as_ptr() as usize;
        assert!(
            pointer >= bytes.as_ptr() as usize
                && pointer + source.len() <= bytes.as_ptr() as usize + bytes.len()
        );
        assert!(references.rasterized_annotations.is_some());
    }
    #[test]
    fn escaped_paths_and_preview_lock_match_main_without_accessing_resources() {
        let bytes = br#"{"stickers":[{"id":"s","x":0,"y":0,"w":1,"h":1,"type":"art",
            "src":"","previewSrc":"https://example.invalid/preview.png","filePath":"C:\\images\\\u4e2d.png"}],"links":[]}"#;
        let document = crate::session_document::decode(bytes).unwrap();
        let window = &document.windows[0];
        assert!(matches!(window.file_path, Some(Cow::Owned(_))));
        let references = References::from_window(window);
        assert_eq!(references.source, Some(""));
        assert_eq!(references.original_file, Some("C:\\images\\中.png"));
        assert!(references.restored_preview_locked);
        assert_eq!(
            references.preview,
            Some("https://example.invalid/preview.png")
        );
    }
    #[test]
    fn null_missing_and_empty_optional_references_remain_distinct() {
        let bytes = br#"{"stickers":[{"id":"s","x":0,"y":0,"w":1,"h":1,"src":null,"previewSrc":"","filePath":""}],"links":[]}"#;
        let document = crate::session_document::decode(bytes).unwrap();
        let references = References::from_window(&document.windows[0]);
        assert_eq!(
            references,
            References {
                source: None,
                preview: None,
                rasterized_annotations: None,
                original_file: None,
                restored_preview_locked: false
            }
        );
    }
}
