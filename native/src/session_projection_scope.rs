//! 逻辑工作区的持久投送身份；不是授权、路径摘要或运行期 Workspace 指针凭证。
use crate::session_source::Source;
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

pub const FIELD: &str = "otterNativeProjectionScope";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Scope([u8; 32]);
impl Scope {
    #[cfg(windows)]
    pub fn generate() -> windows::core::Result<Self> {
        use windows::Win32::Security::Cryptography::{
            BCryptGenRandom, BCRYPT_USE_SYSTEM_PREFERRED_RNG,
        };
        let mut bytes = [0; 32];
        unsafe { BCryptGenRandom(None, &mut bytes, BCRYPT_USE_SYSTEM_PREFERRED_RNG).ok()? };
        if bytes == [0; 32] {
            return Err(windows::core::Error::new(
                windows::Win32::Foundation::E_FAIL,
                "Invalid workspace scope entropy",
            ));
        }
        Ok(Self(bytes))
    }
    /// 已有字段只能原样保留；格式错误或另一 scope 不能被保存操作静默修复/替换。
    pub(crate) fn install(
        self,
        root: &mut serde_json::Map<String, serde_json::Value>,
    ) -> Result<(), crate::session_write::Error> {
        if let Some(value) = root.get(FIELD) {
            if serde_json::from_value::<Self>(value.clone()).ok() != Some(self) {
                return Err(crate::session_write::Error::Source);
            }
        } else {
            root.insert(FIELD.into(), self.to_string().into());
        }
        Ok(())
    }
}
impl fmt::Display for Scope {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in self.0 {
            write!(formatter, "{byte:02x}")?;
        }
        Ok(())
    }
}
impl Serialize for Scope {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}
impl<'de> Deserialize<'de> for Scope {
    fn deserialize<D: Deserializer<'de>>(decoder: D) -> Result<Self, D::Error> {
        struct Hex;
        impl de::Visitor<'_> for Hex {
            type Value = Scope;
            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a nonzero 64-character lowercase workspace scope")
            }
            fn visit_str<E: de::Error>(self, value: &str) -> Result<Scope, E> {
                if value.len() != 64
                    || !value
                        .bytes()
                        .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
                {
                    return Err(E::custom("Invalid workspace scope"));
                }
                let mut bytes = [0; 32];
                for (destination, pair) in bytes.iter_mut().zip(value.as_bytes().chunks_exact(2)) {
                    let digit = |byte: u8| {
                        if byte <= b'9' {
                            byte - b'0'
                        } else {
                            byte - b'a' + 10
                        }
                    };
                    *destination = digit(pair[0]) * 16 + digit(pair[1]);
                }
                if bytes == [0; 32] {
                    return Err(E::custom("Empty workspace scope"));
                }
                Ok(Scope(bytes))
            }
        }
        decoder.deserialize_str(Hex)
    }
}
/// 缺省旧文档可迁移；显式 null/坏值必须失败，不能借兼容入口换逻辑身份。
pub(crate) fn optional<'de, D: Deserializer<'de>>(decoder: D) -> Result<Option<Scope>, D::Error> {
    let raw = <&serde_json::value::RawValue>::deserialize(decoder)?;
    // 64 个 ASCII 字符即使全部使用 \uXXXX 也只有 386 字节，先界定再解码转义。
    if raw.get().len() > 386 {
        return Err(de::Error::custom("Workspace scope exceeds limit"));
    }
    serde_json::from_str(raw.get())
        .map(Some)
        .map_err(de::Error::custom)
}

/// 后台解析的精确字节包含关系；提交者仍须另核真实磁盘提交和当前 Workspace 身份。
pub struct Proof {
    source: Source,
    scope: Scope,
    #[cfg(windows)]
    editing: crate::projection_edit::checkpoint::Index,
}
impl Proof {
    pub fn capture(source: &Source) -> Option<Self> {
        let document = crate::session_document::decode(source).ok()?;
        let scope = document.projection_scope?;
        Some(Self {
            source: source.clone(),
            scope,
            #[cfg(windows)]
            editing: crate::projection_edit::checkpoint::Index::capture(&document)?,
        })
    }
    pub fn scope(&self) -> Scope {
        self.scope
    }
    /// UI 可读的 allocation 凭证；不重新扫描 JSON 或访问磁盘。
    pub fn is_for(&self, source: &Source) -> bool {
        self.source.same_storage(source)
    }
    #[cfg(windows)]
    pub(crate) fn includes_edit(
        &self,
        snapshot: &crate::projection_edit::workspace::Snapshot,
    ) -> bool {
        snapshot.context().scope() == self.scope && self.editing.includes(snapshot)
    }
}

#[cfg(test)]
#[path = "session_projection_scope_tests.rs"]
mod tests;
