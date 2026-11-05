//! 受限图片 child 的管道宿主；不初始化 UI/profile，不拥有具体格式解析器。
use crate::{live_recovery_pipe::Pipe, surface_image_worker_wire as wire};
use std::{
    io::{self, Read, Write},
    os::windows::io::AsHandle,
    time::{Duration, Instant},
};

pub(crate) struct Decoded {
    pub logical_size: [f32; 2],
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

pub(crate) fn require_limits() -> io::Result<()> {
    // 任何 decoder 回调之前先验证 Job；单独调用私有入口也不能绕过硬限制。
    crate::bounded_worker_process::require_limits(128 * 1024 * 1024)
}

pub(crate) fn run(decode: impl FnOnce(&[u8]) -> io::Result<Decoded>) -> io::Result<()> {
    run_for(wire::Codec::WebP, decode)
}
pub(crate) fn run_for(
    codec: wire::Codec,
    decode: impl FnOnce(&[u8]) -> io::Result<Decoded>,
) -> io::Result<()> {
    require_limits()?;
    let mut pipe = Pipe::adopt(std::io::stdin().as_handle().try_clone_to_owned()?)?;
    let mut channel = pipe.io(Some(Instant::now() + Duration::from_secs(15)), None);
    let mut header = [0u8; 8];
    channel.read_exact(&mut header)?;
    let length = wire::request_for(codec, &header)?;
    let mut bytes = zeroize::Zeroizing::new(vec![0; length]);
    channel.read_exact(&mut bytes)?;
    let image = decode(&bytes)?;
    write_frame_for(codec, &mut channel, &image)
}

#[cfg(test)]
fn write_frame(writer: &mut impl Write, image: &Decoded) -> io::Result<()> {
    write_frame_for(wire::Codec::WebP, writer, image)
}
fn write_frame_for(codec: wire::Codec, writer: &mut impl Write, image: &Decoded) -> io::Result<()> {
    let count = wire::pixel_count(image.width, image.height)?;
    if count != image.pixels.len() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Invalid decoded pixel count",
        ));
    }
    // 完整准入后才发成功 header；pixel 数据已是 PBGRA，宿主不能再次预乘。
    if codec == wire::Codec::Svg {
        wire::logical_size(image.logical_size, [image.width, image.height])?;
    }
    writer.write_all(codec.response())?;
    for value in [image.width, image.height, count as u32] {
        writer.write_all(&value.to_le_bytes())?;
    }
    if codec == wire::Codec::Svg {
        for value in image.logical_size {
            writer.write_all(&value.to_le_bytes())?;
        }
    }
    writer.write_all(&image.pixels)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn image_worker_host_validates_before_writing_and_preserves_pbgra() {
        let mut image = Decoded {
            logical_size: [1., 1.],
            width: 1,
            height: 1,
            pixels: vec![5, 10, 15, 20],
        };
        let mut bytes = Vec::new();
        write_frame(&mut bytes, &image).unwrap();
        assert_eq!(
            wire::response(bytes[..16].try_into().unwrap()).unwrap(),
            (1, 1, 4)
        );
        assert_eq!(&bytes[16..], &[5, 10, 15, 20]);
        for count in [0, 3, 5] {
            image.pixels.resize(count, 0);
            bytes.clear();
            assert!(write_frame(&mut bytes, &image).is_err());
            assert!(bytes.is_empty());
        }
        image.width = u32::MAX;
        bytes.clear();
        assert!(write_frame(&mut bytes, &image).is_err());
        assert!(bytes.is_empty());
    }
}
