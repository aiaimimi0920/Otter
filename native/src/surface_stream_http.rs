//! 单次设备 Surface 流读取；不启动常驻轮询，不使用管理员实例接口。
use super::*;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StreamError {
    Transport(Error),
    Protocol(crate::surface_stream::Error),
}
impl Control<'_> {
    pub fn surface_stream(&self, after: u64) -> Result<crate::surface_stream::Batch, StreamError> {
        self.surface_stream_until(after, Instant::now() + Duration::from_secs(3))
    }
    pub(super) fn surface_stream_until(
        &self,
        after: u64,
        deadline: Instant,
    ) -> Result<crate::surface_stream::Batch, StreamError> {
        let bytes = self
            .send_until(
                "GET",
                &format!("/v1/surfaces/stream?after={after}&timeoutMs=0"),
                &[],
                deadline.min(Instant::now() + Duration::from_secs(3)),
                crate::surface_stream::MAX_BYTES,
            )
            .map_err(StreamError::Transport)?;
        crate::surface_stream::Batch::parse(&bytes, after).map_err(StreamError::Protocol)
    }
}
