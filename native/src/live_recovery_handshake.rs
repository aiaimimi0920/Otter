//! 已认证私有管道上的 session 握手；Hello/Ready 与业务请求严格区分，不携带恢复记录。
//! 读取本身不实现超时；传输 owner 必须提供有截止时间的 I/O，Ready 前不能登记或隐藏。
use crate::live_recovery_wire::{read_payload, write_payload, SCHEMA};
use serde::{Deserialize, Serialize};
use std::io::{self, Read, Write};

#[derive(Serialize, Deserialize)]
#[serde(tag = "stage", rename_all = "camelCase", deny_unknown_fields)]
enum Message {
    Hello { schema: u8, session: [u8; 16] },
    Ready { schema: u8, session: [u8; 16] },
}
fn invalid() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        "Invalid Live recovery handshake",
    )
}
fn read(reader: &mut impl Read) -> io::Result<Message> {
    let bytes = read_payload(reader)?.ok_or_else(invalid)?;
    serde_json::from_slice(&bytes).map_err(|_| invalid())
}
pub fn write_hello(writer: &mut impl Write, session: [u8; 16]) -> io::Result<()> {
    if session == [0; 16] {
        return Err(invalid());
    }
    write_payload(
        writer,
        &Message::Hello {
            schema: SCHEMA,
            session,
        },
    )
}
pub fn read_hello(reader: &mut impl Read) -> io::Result<[u8; 16]> {
    match read(reader)? {
        Message::Hello {
            schema: SCHEMA,
            session,
        } if session != [0; 16] => Ok(session),
        _ => Err(invalid()),
    }
}
pub fn write_ready(writer: &mut impl Write, session: [u8; 16]) -> io::Result<()> {
    if session == [0; 16] {
        return Err(invalid());
    }
    write_payload(
        writer,
        &Message::Ready {
            schema: SCHEMA,
            session,
        },
    )
}
pub fn read_ready(reader: &mut impl Read, expected: [u8; 16]) -> io::Result<()> {
    match read(reader)? {
        Message::Ready {
            schema: SCHEMA,
            session,
        } if session != [0; 16] && session == expected => Ok(()),
        _ => Err(invalid()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hello_ready_match_session_and_cannot_be_reflected_or_substituted() {
        let mut hello = Vec::new();
        write_hello(&mut hello, [7; 16]).unwrap();
        assert_eq!(read_hello(&mut &hello[..]).unwrap(), [7; 16]);
        assert!(read_ready(&mut &hello[..], [7; 16]).is_err());
        let mut ready = Vec::new();
        write_ready(&mut ready, [7; 16]).unwrap();
        read_ready(&mut &ready[..], [7; 16]).unwrap();
        assert!(read_ready(&mut &ready[..], [8; 16]).is_err());
        assert!(read_hello(&mut &ready[..]).is_err());
        assert!(read_ready(&mut &[][..], [7; 16]).is_err());
        for end in 0..ready.len() {
            assert!(read_ready(&mut &ready[..end], [7; 16]).is_err());
        }
    }
    #[test]
    fn zero_session_wrong_schema_and_unknown_fields_are_rejected() {
        assert!(write_hello(&mut Vec::new(), [0; 16]).is_err());
        assert!(write_ready(&mut Vec::new(), [0; 16]).is_err());
        for message in [
            Message::Hello {
                schema: SCHEMA + 1,
                session: [7; 16],
            },
            Message::Hello {
                schema: SCHEMA,
                session: [0; 16],
            },
        ] {
            let mut bytes = Vec::new();
            write_payload(&mut bytes, &message).unwrap();
            assert!(read_hello(&mut &bytes[..]).is_err());
        }
        let mut value = serde_json::to_value(Message::Hello {
            schema: SCHEMA,
            session: [7; 16],
        })
        .unwrap();
        value["record"] = serde_json::json!({});
        let mut bytes = Vec::new();
        write_payload(&mut bytes, &value).unwrap();
        assert!(read_hello(&mut &bytes[..]).is_err());
    }
}
