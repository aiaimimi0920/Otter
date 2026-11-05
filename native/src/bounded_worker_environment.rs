//! 固定 child 环境白名单；硬件驱动需要 SystemRoot，但不能继承调用方可伪造的目录。
use std::io;
use windows::Win32::System::SystemInformation::GetWindowsDirectoryW;

#[derive(Clone, Copy)]
pub(crate) enum Environment {
    Empty,
    SystemRoot,
}
impl Environment {
    pub(super) fn block(self) -> io::Result<Vec<u16>> {
        if matches!(self, Self::Empty) {
            return Ok(vec![0, 0]);
        }
        // 不使用 std::env 的 SystemRoot/PATH；从 OS API 取真实本机 Windows 目录。
        let mut directory = vec![0u16; 32768];
        let count = unsafe { GetWindowsDirectoryW(Some(&mut directory)) } as usize;
        if count == 0
            || count >= directory.len()
            || directory[..count]
                .iter()
                .any(|unit| *unit == 0 || *unit == b'=' as u16)
        {
            return Err(io::Error::other("Invalid system Windows directory"));
        }
        directory.truncate(count);
        let mut block = "SystemRoot=".encode_utf16().collect::<Vec<_>>();
        block.extend(directory);
        block.extend([0, 0]);
        Ok(block)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bounded_worker_environment_is_empty_or_os_systemroot_only() {
        assert_eq!(Environment::Empty.block().unwrap(), [0, 0]);
        let block = Environment::SystemRoot.block().unwrap();
        assert_eq!(&block[block.len() - 2..], &[0, 0]);
        let value = String::from_utf16(&block[..block.len() - 2]).unwrap();
        assert!(value.starts_with("SystemRoot="));
        assert_eq!(value.matches('=').count(), 1);
        assert!(!value.contains('\0'));
        assert!(std::path::Path::new(value.strip_prefix("SystemRoot=").unwrap()).is_dir());
    }
}
