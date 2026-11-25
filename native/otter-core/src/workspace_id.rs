//! 工作区对象的持久身份值；不绑定 HWND、像素资源或编辑版本，旧 ID 原样保留。
use std::fmt;

#[derive(Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct UnitId(Box<str>);
impl UnitId {
    /// 兼容已有工作区的不透明字符串 ID，不要求旧数据一定使用 UUID。
    pub fn parse(value: &str) -> Option<Self> {
        if value.is_empty() || value.len() > 256 || value.chars().any(char::is_control) {
            return None;
        }
        Some(Self(value.into()))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl fmt::Display for UnitId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn legacy_and_uuid_identifiers_roundtrip_without_normalization() {
        for value in [
            "legacy-sticker-17",
            "工作区-🦦",
            "D2F4F738-09D9-4F46-829E-262381E63387",
        ] {
            let id = UnitId::parse(value).unwrap();
            assert_eq!(id.as_str(), value);
            assert_eq!(id.to_string(), value);
            assert_eq!(id, id.clone());
        }
    }
    #[test]
    fn invalid_or_unbounded_identifiers_are_rejected() {
        for value in ["", "a\0b", "a\nb", "a\u{7f}b"] {
            assert!(UnitId::parse(value).is_none());
        }
        assert!(UnitId::parse(&"a".repeat(256)).is_some());
        assert!(UnitId::parse(&"a".repeat(257)).is_none());
        assert!(UnitId::parse(&"中".repeat(86)).is_none());
    }
}
