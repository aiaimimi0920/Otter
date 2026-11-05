//! 编码器析构可能继续写尾块；失败/取消后锁闭 sink，禁止后续隐式写入与重试。
use super::*;
pub(super) struct Sink<'a, W> {
    writer: &'a mut W,
    cancel: &'a AtomicBool,
    remaining: usize,
    failed: bool,
}
impl<'a, W: Write> Sink<'a, W> {
    pub(super) fn new(writer: &'a mut W, cancel: &'a AtomicBool, remaining: usize) -> Self {
        Self {
            writer,
            cancel,
            remaining,
            failed: false,
        }
    }
    fn check(&mut self) -> io::Result<()> {
        if self.failed || self.cancel.load(Ordering::Acquire) {
            self.failed = true;
            return Err(io::Error::new(
                io::ErrorKind::ConnectionAborted,
                "HDR encoding stopped",
            ));
        }
        Ok(())
    }
}
impl<W: Write> Write for Sink<'_, W> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.check()?;
        if bytes.len() > self.remaining {
            self.failed = true;
            return Err(io::Error::other("HDR PNG output limit exceeded"));
        }
        let result = self.writer.write(bytes);
        match result {
            Ok(n) if n <= bytes.len() && (n != 0 || bytes.is_empty()) => {
                self.remaining -= n;
                self.check()?;
                Ok(n)
            }
            Ok(_) => {
                self.failed = true;
                Err(io::Error::new(
                    io::ErrorKind::WriteZero,
                    "Invalid HDR PNG write count",
                ))
            }
            Err(error) => {
                self.failed = true;
                // 不返回 Interrupted，避免 write_all 重试已经失败的目的地。
                Err(io::Error::other(error))
            }
        }
    }
    fn flush(&mut self) -> io::Result<()> {
        self.check()?;
        if let Err(error) = self.writer.flush() {
            self.failed = true;
            return Err(io::Error::other(error));
        }
        self.check()
    }
}
