//! Shader 配方的流式字段准入；wrapper JSON 在构造过量 map 前终止，不替代上游消息上限。
use serde::{
    de::{self, MapAccess, Visitor},
    Deserialize, Deserializer,
};
use std::{collections::BTreeMap, fmt, marker::PhantomData};

pub(super) fn valid_source(text: &str) -> bool {
    !text.is_empty() && text.len() <= crate::shader_worker_wire::MAX_SOURCE && !text.contains('\0')
}
pub(super) fn source<'de, D: Deserializer<'de>>(deserializer: D) -> Result<String, D::Error> {
    let text = String::deserialize(deserializer)?;
    if !valid_source(&text) {
        return Err(de::Error::custom("Shader source exceeds policy"));
    }
    Ok(text)
}
pub(super) fn uniforms<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<BTreeMap<String, f32>, D::Error> {
    deserializer.deserialize_map(Bounded::<f32, 256>(PhantomData))
}
pub(super) fn textures<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<BTreeMap<String, String>, D::Error> {
    deserializer.deserialize_map(Bounded::<String, 31>(PhantomData))
}
struct Bounded<T, const MAX: usize>(PhantomData<T>);
impl<'de, T: Deserialize<'de>, const MAX: usize> Visitor<'de> for Bounded<T, MAX> {
    type Value = BTreeMap<String, T>;
    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "at most {MAX} Shader bindings")
    }
    fn visit_map<M: MapAccess<'de>>(self, mut map: M) -> Result<Self::Value, M::Error> {
        if map.size_hint().is_some_and(|count| count > MAX) {
            return Err(de::Error::custom("Too many Shader bindings"));
        }
        let mut result = BTreeMap::new();
        while let Some(name) = map.next_key::<String>()? {
            // 检查位于 next_value/insert 之前；同名重复也不获得额外值解析或分配权。
            if result.len() >= MAX
                || result.contains_key(&name)
                || crate::shader_angle::name(&name).is_err()
            {
                return Err(de::Error::custom("Invalid or excessive Shader bindings"));
            }
            result.insert(name, map.next_value()?);
        }
        Ok(result)
    }
}
