//! Encoder destruction may write trailing chunks; lock sinks after failure/cancellation to prevent implicit later writes/retries.
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
                // Do not return Interrupted, which would let write_all retry already-failed destinations.
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
