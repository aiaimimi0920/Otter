//! 设备级新投送同意；与服务端 InboxPolicy 独立，不改变已接收关联的生命周期。
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

pub const MAX_BYTES: usize = 1024;
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Policy {
    Auto,
    #[default]
    Confirm,
    Reject,
}
#[derive(Clone, Copy)]
pub enum Change {
    Policy(Policy),
    Paused(bool),
}
impl From<Policy> for Change {
    fn from(policy: Policy) -> Self {
        Self::Policy(policy)
    }
}

#[derive(Clone, Debug)]
pub struct Document(Map<String, Value>);
impl Default for Document {
    fn default() -> Self {
        Self(Map::from_iter([
            ("version".into(), Value::from(1)),
            ("policy".into(), Value::from("confirm")),
        ]))
    }
}
impl Document {
    pub fn decode(bytes: &[u8]) -> Result<Self, &'static str> {
        if bytes.len() > MAX_BYTES {
            return Err("Receive policy exceeds 1 KiB");
        }
        let Value::Object(root) =
            serde_json::from_slice(bytes).map_err(|_| "Invalid receive policy JSON")?
        else {
            return Err("Receive policy must be an object");
        };
        if root.get("version").and_then(Value::as_u64) != Some(1) {
            return Err("Unsupported receive policy version");
        }
        Ok(Self(root))
    }
    /// 文件存在但字段不可读时拒绝；只有整个文件缺失由 profile owner 选择默认手动。
    pub fn policy(&self) -> Policy {
        self.0
            .get("policy")
            .and_then(|value| serde_json::from_value(value.clone()).ok())
            .unwrap_or(Policy::Reject)
    }
    pub fn with_policy(&self, policy: Policy) -> Result<Vec<u8>, &'static str> {
        self.with_change(Change::Policy(policy))
    }
    /// 缺字段兼容既有 native v1；字段存在但无效必须暂停，不能静默参与收件。
    pub fn paused(&self) -> bool {
        self.0
            .get("paused")
            .is_some_and(|value| value.as_bool() != Some(false))
    }
    pub fn with_change(&self, change: Change) -> Result<Vec<u8>, &'static str> {
        let mut root = self.0.clone();
        match change {
            Change::Policy(policy) => {
                root.insert(
                    "policy".into(),
                    serde_json::to_value(policy).map_err(|_| "Invalid receive policy")?,
                );
            }
            Change::Paused(paused) => {
                root.insert("paused".into(), Value::Bool(paused));
            }
        }
        let bytes = serde_json::to_vec(&root).map_err(|_| "Invalid receive policy JSON")?;
        if bytes.len() > MAX_BYTES {
            return Err("Receive policy exceeds 1 KiB");
        }
        Ok(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn receive_policy_missing_default_and_invalid_fields_fail_closed() {
        assert_eq!(Document::default().policy(), Policy::Confirm);
        for bytes in [
            br#"{"version":1}"#.as_slice(),
            br#"{"version":1,"policy":null}"#,
            br#"{"version":1,"policy":"AUTO"}"#,
            br#"{"version":1,"policy":true}"#,
        ] {
            assert_eq!(Document::decode(bytes).unwrap().policy(), Policy::Reject);
        }
        for bytes in [b"[]".as_slice(), b"{", br#"{"version":2,"policy":"auto"}"#] {
            assert!(Document::decode(bytes).is_err());
        }
        assert!(Document::decode(&vec![b' '; MAX_BYTES + 1]).is_err());
    }
    #[test]
    fn receive_policy_roundtrip_preserves_extensions_and_all_three_values() {
        let original =
            Document::decode(br#"{"version":1,"policy":"reject","future":{"x":[1,2]}}"#).unwrap();
        for policy in [Policy::Auto, Policy::Confirm, Policy::Reject] {
            let bytes = original.with_policy(policy).unwrap();
            let next = Document::decode(&bytes).unwrap();
            assert_eq!(next.policy(), policy);
            assert_eq!(next.0["future"], original.0["future"]);
        }
        assert_eq!(original.policy(), Policy::Reject);
    }
    #[test]
    fn receive_policy_pause_is_independent_compatible_and_fail_closed() {
        let legacy = Document::decode(br#"{"version":1,"policy":"auto","future":[1]}"#).unwrap();
        assert!(!legacy.paused());
        let paused = Document::decode(&legacy.with_change(Change::Paused(true)).unwrap()).unwrap();
        assert!(paused.paused());
        assert_eq!(paused.policy(), Policy::Auto);
        let manual = Document::decode(&paused.with_policy(Policy::Confirm).unwrap()).unwrap();
        assert!(manual.paused());
        assert_eq!(manual.0["future"], legacy.0["future"]);
        assert!(
            !Document::decode(&manual.with_change(Change::Paused(false)).unwrap())
                .unwrap()
                .paused()
        );
        for value in [Value::Null, Value::String("false".into()), Value::from(0)] {
            let bytes = serde_json::to_vec(
                &serde_json::json!({"version":1,"policy":"auto","paused":value}),
            )
            .unwrap();
            assert!(Document::decode(&bytes).unwrap().paused());
        }
    }
}
