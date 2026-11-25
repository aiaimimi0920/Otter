//! Durable delivery identity of a logical workspace; not authorization, a path digest, or a runtime Workspace-pointer credential.
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
    /// Existing fields may only remain unchanged; saving must not silently repair/replace malformed values or another scope.
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
/// Legacy documents with absent fields may migrate; explicit null/invalid values must fail, never changing logical identity through compatibility paths.
pub(crate) fn optional<'de, D: Deserializer<'de>>(decoder: D) -> Result<Option<Scope>, D::Error> {
    let raw = <&serde_json::value::RawValue>::deserialize(decoder)?;
    // 64 ASCII characters require only 386 bytes even entirely escaped as \uXXXX; bound first, then decode escapes.
    if raw.get().len() > 386 {
        return Err(de::Error::custom("Workspace scope exceeds limit"));
    }
    serde_json::from_str(raw.get())
        .map(Some)
        .map_err(de::Error::custom)
}

/// Exact byte-inclusion relationship parsed in the background; committers must separately verify actual disk commit and current Workspace identity.
pub struct Proof {
    source: Source,
    scope: Scope,
    #[cfg(windows)]
    editing: crate::projection_edit::checkpoint::Index,
}
impl Proof {
    pub fn capture(source: &Source) -> Option<Self> {
        Self::capture_parsed(&source.parse().ok()?)
    }
    pub(crate) fn capture_parsed(parsed: &crate::session_source::Parsed<'_>) -> Option<Self> {
        let document = parsed.document();
        let scope = document.projection_scope?;
        Some(Self {
            source: parsed.source().clone(),
            scope,
            #[cfg(windows)]
            editing: crate::projection_edit::checkpoint::Index::capture(document)?,
        })
    }
    pub fn scope(&self) -> Scope {
        self.scope
    }
    /// UI-readable allocation credentials; no JSON rescanning or disk access.
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

#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-save")))]
#[path = "session_projection_scope_tests.rs"]
mod tests;
