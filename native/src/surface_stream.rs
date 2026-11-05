//! 设备过滤 Surface 消息批次；有界解析不等于消息已获当前附件的应用许可。
use serde::{
    de::{Error as _, SeqAccess, Visitor},
    Deserialize, Deserializer,
};
use serde_json::value::RawValue;
pub const MAX_BYTES: usize = 8 * 1024 * 1024;
const MAX_MESSAGE_BYTES: usize = 1024 * 1024;
const MAX_MESSAGES: usize = 512;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Limit,
    Json,
    Protocol,
    Cursor,
}
#[derive(Debug)]
pub struct Batch {
    after: u64,
    next: u64,
    reset: bool,
    messages: Vec<Box<RawValue>>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Wire {
    protocol_version: String,
    next: Option<u64>,
    reset: bool,
    #[serde(default, deserialize_with = "messages")]
    messages: Vec<Box<RawValue>>,
}
fn messages<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<Box<RawValue>>, D::Error> {
    struct Bounded;
    impl<'de> Visitor<'de> for Bounded {
        type Value = Vec<Box<RawValue>>;
        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("bounded Surface messages")
        }
        fn visit_seq<A: SeqAccess<'de>>(self, mut sequence: A) -> Result<Self::Value, A::Error> {
            let mut result = Vec::new();
            while let Some(message) = sequence.next_element::<Box<RawValue>>()? {
                if result.len() >= MAX_MESSAGES || message.get().len() > MAX_MESSAGE_BYTES {
                    return Err(A::Error::custom("Surface message limit exceeded"));
                }
                result.push(message);
            }
            Ok(result)
        }
    }
    deserializer.deserialize_seq(Bounded)
}
impl Batch {
    pub fn parse(bytes: &[u8], after: u64) -> Result<Self, Error> {
        if bytes.is_empty() || bytes.len() > MAX_BYTES {
            return Err(Error::Limit);
        }
        let wire: Wire = serde_json::from_slice(bytes).map_err(|_| Error::Json)?;
        if wire.protocol_version != "loom.surface-stream.v1" {
            return Err(Error::Protocol);
        }
        let next = wire.next.unwrap_or(after);
        if next < after {
            return Err(Error::Cursor);
        }
        Ok(Self {
            after,
            next,
            reset: wire.reset,
            messages: wire.messages,
        })
    }
    pub fn next(&self) -> u64 {
        self.next
    }
    pub fn after(&self) -> u64 {
        self.after
    }
    pub fn reset(&self) -> bool {
        self.reset
    }
    pub fn messages(&self) -> &[Box<RawValue>] {
        &self.messages
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cursor_defaults_and_reset_are_preserved_without_applying_messages() {
        let batch = Batch::parse(br#"{"protocolVersion":"loom.surface-stream.v1","reset":true,"messages":[{"method":"future","params":{}}]}"#, 7).unwrap();
        assert_eq!(batch.next(), 7);
        assert!(batch.reset());
        assert_eq!(batch.messages().len(), 1);
        assert!(Batch::parse(
            br#"{"protocolVersion":"loom.surface-stream.v1","reset":false}"#,
            0
        )
        .unwrap()
        .messages()
        .is_empty());
    }
    #[test]
    fn protocol_cursor_and_required_reset_are_checked() {
        assert!(matches!(
            Batch::parse(br#"{"protocolVersion":"other","reset":false}"#, 0),
            Err(Error::Protocol)
        ));
        assert!(matches!(
            Batch::parse(
                br#"{"protocolVersion":"loom.surface-stream.v1","next":2,"reset":false}"#,
                3
            ),
            Err(Error::Cursor)
        ));
        for raw in [
            br#"{"protocolVersion":"loom.surface-stream.v1"}"#.as_slice(),
            br#"{"protocolVersion":"loom.surface-stream.v1","reset":"false"}"#,
            br#"{"protocolVersion":"loom.surface-stream.v1","reset":false,"messages":{}}"#,
        ] {
            assert!(matches!(Batch::parse(raw, 0), Err(Error::Json)));
        }
    }
    #[test]
    fn response_count_and_individual_message_budgets_are_enforced() {
        assert!(matches!(
            Batch::parse(&vec![b' '; MAX_BYTES + 1], 0),
            Err(Error::Limit)
        ));
        for messages in [
            vec![serde_json::json!({}); MAX_MESSAGES + 1],
            vec![serde_json::json!({"payload":"x".repeat(MAX_MESSAGE_BYTES)})],
        ] {
            let bytes = serde_json::to_vec(&serde_json::json!({"protocolVersion":"loom.surface-stream.v1","reset":false,"messages":messages})).unwrap();
            assert!(Batch::parse(&bytes, 0).is_err());
        }
    }
}
