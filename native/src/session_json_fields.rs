//! 借用会话原文字段；统一拒绝重复键与小字段洪泛，不重新解析或复制大型图片值。
use serde::Deserializer;
use serde_json::value::RawValue;
use std::{borrow::Cow, collections::BTreeMap};
pub(crate) fn fields(text: &str) -> Result<BTreeMap<String, Cow<'_, RawValue>>, String> {
    struct Fields;
    impl<'de> serde::de::Visitor<'de> for Fields {
        type Value = BTreeMap<String, Cow<'de, RawValue>>;
        fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("at most 256 unique session fields")
        }
        fn visit_map<A: serde::de::MapAccess<'de>>(
            self,
            mut map: A,
        ) -> Result<Self::Value, A::Error> {
            let mut fields = BTreeMap::new();
            while let Some(key) = map.next_key::<String>()? {
                if fields.len() == 256 || fields.contains_key(&key) {
                    return Err(serde::de::Error::custom("Session field limit or duplicate"));
                }
                fields.insert(key, Cow::Borrowed(map.next_value::<&RawValue>()?));
            }
            Ok(fields)
        }
    }
    serde_json::Deserializer::from_str(text)
        .deserialize_map(Fields)
        .map_err(|error| error.to_string())
}
