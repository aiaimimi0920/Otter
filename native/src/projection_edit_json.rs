//! Collaboration boundaries reject duplicate keys at every depth, preventing disk recovery, signed requests, and renderers from interpreting different objects.
use super::Error;
use serde::{
    de::{self, DeserializeOwned, MapAccess, SeqAccess, Visitor},
    Deserialize, Deserializer,
};
use serde_json::Value;
use std::{collections::HashSet, fmt};
// First pass scans only, without a second general Value tree for the entire disk journal; second pass directly decodes typed shapes.
// All v1 shapes allow at most 256 map entries/2048 array points; scanning obeys the same local budgets.
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
