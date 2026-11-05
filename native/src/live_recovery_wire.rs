//! 独立恢复进程的有界协议；结构校验不等于传输鉴权或窗口身份验证。
use serde::{Deserialize, Serialize};
use std::io::{self, Read, Write};
pub const SCHEMA: u8 = 1;
pub const MAX_FRAME_BYTES: usize = 4096;
pub const MAX_RECORDS: usize = 16;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Record {
    pub hwnd: u64,
    pub process_id: u32,
    pub thread_id: u32,
    pub process_created: u64,
    pub rect: [i32; 4],
    pub placement_flags: u32,
    pub show_command: u32,
    pub min_position: [i32; 2],
    pub max_position: [i32; 2],
    pub normal_position: [i32; 4],
    pub extended_style: i64,
    pub layered: Option<Layered>,
    pub iconic: bool,
    pub visible: bool,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Layered {
    pub color: u32,
    pub alpha: u8,
    pub flags: u32,
}

impl Record {
    pub fn validate(&self) -> io::Result<()> {
        // WS_EX_LAYERED / LWA_* 是协议数值；本模块不依赖 Windows 或调用任何窗口 API。
        let layered = self.extended_style & 0x0008_0000 != 0;
        let rect_valid = |rect: [i32; 4]| {
            let width = i64::from(rect[2]) - i64::from(rect[0]);
            let height = i64::from(rect[3]) - i64::from(rect[1]);
            width > 0 && height > 0 && width <= i64::from(i32::MAX) && height <= i64::from(i32::MAX)
        };
        if self.hwnd == 0
            || self.process_id == 0
            || self.thread_id == 0
            || self.process_created == 0
            || !rect_valid(self.rect)
            || !rect_valid(self.normal_position)
            || self.show_command > 11
            || self.placement_flags & !7 != 0
            || layered != self.layered.is_some()
            || self
                .layered
                .as_ref()
                .is_some_and(|value| value.flags == 0 || value.flags & !3 != 0)
        {
            return Err(invalid("Invalid Live recovery record"));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "camelCase", deny_unknown_fields)]
pub enum Command {
    Register { id: u64, record: Record },
    Retire { id: u64 },
    Shutdown,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Request {
    pub schema: u8,
    pub session: [u8; 16],
    pub sequence: u64,
    pub command: Command,
}
impl Request {
    pub fn validate(&self) -> io::Result<()> {
        if self.schema != SCHEMA || self.session == [0; 16] || self.sequence == 0 {
            return Err(invalid("Invalid Live recovery envelope"));
        }
        match &self.command {
            Command::Register { id, record } => {
                if *id == 0 {
                    return Err(invalid("Invalid Live recovery registration id"));
                }
                record.validate()
            }
            Command::Retire { id: 0 } => Err(invalid("Invalid Live recovery retirement id")),
            _ => Ok(()),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Status {
    Accepted,
    Rejected,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Ack {
    pub schema: u8,
    pub session: [u8; 16],
    pub sequence: u64,
    pub status: Status,
}

fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}
pub(crate) fn read_payload(reader: &mut impl Read) -> io::Result<Option<Vec<u8>>> {
    let mut header = [0u8; 4];
    // 只有首字节前的 EOF 是正常结束；部分头/正文必须作为截断错误保留。
    match reader.read_exact(&mut header[..1]) {
        Err(error) if error.kind() == io::ErrorKind::UnexpectedEof => return Ok(None),
        result => result?,
    }
    reader.read_exact(&mut header[1..])?;
    let length = u32::from_le_bytes(header) as usize;
    if length == 0 || length > MAX_FRAME_BYTES {
        return Err(invalid("Live recovery frame size exceeded"));
    }
    let mut bytes = vec![0; length];
    reader.read_exact(&mut bytes)?;
    Ok(Some(bytes))
}
pub(crate) fn write_payload(writer: &mut impl Write, value: &impl Serialize) -> io::Result<()> {
    let bytes = serde_json::to_vec(value).map_err(io::Error::other)?;
    if bytes.is_empty() || bytes.len() > MAX_FRAME_BYTES {
        return Err(invalid("Live recovery frame size exceeded"));
    }
    writer.write_all(&(bytes.len() as u32).to_le_bytes())?;
    writer.write_all(&bytes)?;
    writer.flush()
}
pub fn read_request(reader: &mut impl Read) -> io::Result<Option<Request>> {
    let Some(bytes) = read_payload(reader)? else {
        return Ok(None);
    };
    let value: Request = serde_json::from_slice(&bytes).map_err(io::Error::other)?;
    value.validate()?;
    Ok(Some(value))
}
pub fn write_request(writer: &mut impl Write, value: &Request) -> io::Result<()> {
    value.validate()?;
    write_payload(writer, value)
}
pub fn read_ack(reader: &mut impl Read, session: [u8; 16], sequence: u64) -> io::Result<Ack> {
    let bytes =
        read_payload(reader)?.ok_or_else(|| invalid("Missing Live recovery acknowledgement"))?;
    let value: Ack = serde_json::from_slice(&bytes).map_err(io::Error::other)?;
    if sequence == 0
        || session == [0; 16]
        || value.schema != SCHEMA
        || value.session != session
        || value.sequence != sequence
    {
        return Err(invalid("Mismatched Live recovery acknowledgement"));
    }
    Ok(value)
}
pub fn write_ack(writer: &mut impl Write, value: &Ack) -> io::Result<()> {
    if value.schema != SCHEMA || value.session == [0; 16] || value.sequence == 0 {
        return Err(invalid("Invalid Live recovery acknowledgement"));
    }
    write_payload(writer, value)
}

#[cfg(test)]
#[path = "live_recovery_wire_tests.rs"]
mod tests;
