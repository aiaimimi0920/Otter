//! 有界 RBSP 位读取；参数集最多 1024 字节，不按网络声明动态扩容。
use super::Result;
pub(super) struct Bits {
    bytes: [u8; 1024],
    len: usize,
    at: usize,
}
impl Bits {
    pub(super) fn new(input: &[u8]) -> Result<Self> {
        if input.len() > 1024 {
            return Err("parameter set size budget");
        }
        let mut result = Self {
            bytes: [0; 1024],
            len: 0,
            at: 0,
        };
        let mut zeros = 0;
        for (index, &byte) in input.iter().enumerate() {
            if zeros >= 2 && byte == 3 {
                if input.get(index + 1).is_none_or(|next| *next > 3) {
                    return Err("invalid emulation prevention");
                }
                zeros = 0;
                continue;
            }
            result.bytes[result.len] = byte;
            result.len += 1;
            zeros = if byte == 0 { zeros + 1 } else { 0 };
        }
        Ok(result)
    }
    pub(super) fn read(&mut self, count: usize) -> Result<u32> {
        if count > 32 || self.at + count > self.len * 8 {
            return Err("truncated parameter set");
        }
        let mut value = 0;
        for _ in 0..count {
            value = (value << 1) | u32::from((self.bytes[self.at / 8] >> (7 - self.at % 8)) & 1);
            self.at += 1;
        }
        Ok(value)
    }
    pub(super) fn ue(&mut self) -> Result<u32> {
        let mut zeros = 0;
        while self.read(1)? == 0 {
            zeros += 1;
            if zeros > 30 {
                return Err("Exp-Golomb budget");
            }
        }
        Ok((1_u32 << zeros) - 1 + self.read(zeros)?)
    }
}
