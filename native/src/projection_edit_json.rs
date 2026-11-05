//! 协作边界拒绝任意层重复key，避免磁盘恢复、签名请求与渲染器解释不同对象。
use super::Error;
use serde::{
    de::{self, DeserializeOwned, MapAccess, SeqAccess, Visitor},
    Deserialize, Deserializer,
};
use serde_json::Value;
use std::{collections::HashSet, fmt};
// 第一遍只扫描，不为整份磁盘日志构造第二棵通用Value树；第二遍直接解码typed形状。
// 所有v1形状的map最多256项、数组最多2048个点，扫描也遵守相同局部预算。
struct Unique;
impl<'de> Deserialize<'de> for Unique {
    fn deserialize<D: Deserializer<'de>>(decoder: D) -> Result<Self, D::Error> {
        struct Json;
        impl<'de> Visitor<'de> for Json {
            type Value = Unique;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("unique-key JSON")
            }
            fn visit_bool<E: de::Error>(self, _: bool) -> Result<Unique, E> {
                Ok(Unique)
            }
            fn visit_i64<E: de::Error>(self, _: i64) -> Result<Unique, E> {
                Ok(Unique)
            }
            fn visit_u64<E: de::Error>(self, _: u64) -> Result<Unique, E> {
                Ok(Unique)
            }
            fn visit_f64<E: de::Error>(self, v: f64) -> Result<Unique, E> {
                if v.is_finite() {
                    Ok(Unique)
                } else {
                    Err(E::custom("nonfinite JSON"))
                }
            }
            fn visit_str<E: de::Error>(self, _: &str) -> Result<Unique, E> {
                Ok(Unique)
            }
            fn visit_unit<E: de::Error>(self) -> Result<Unique, E> {
                Ok(Unique)
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Unique, A::Error> {
                let mut count = 0;
                while let Some(Unique) = seq.next_element()? {
                    count += 1;
                    if count > 2048 {
                        return Err(de::Error::custom("JSON array budget"));
                    }
                }
                Ok(Unique)
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Unique, A::Error> {
                let mut keys = HashSet::new();
                while let Some(key) = map.next_key::<String>()? {
                    if keys.len() >= 256 || !keys.insert(key) {
                        return Err(de::Error::custom("duplicate or excessive JSON keys"));
                    }
                    map.next_value::<Unique>()?;
                }
                Ok(Unique)
            }
        }
        decoder.deserialize_any(Json)
    }
}
pub(crate) fn parse(bytes: &[u8], limit: usize) -> Result<Value, Error> {
    decode(bytes, limit)
}
pub(super) fn decode<T: DeserializeOwned>(bytes: &[u8], limit: usize) -> Result<T, Error> {
    if bytes.len() > limit {
        return Err(Error::Limit);
    }
    serde_json::from_slice::<Unique>(bytes).map_err(|_| Error::Json)?;
    serde_json::from_slice(bytes).map_err(|_| Error::Json)
}
