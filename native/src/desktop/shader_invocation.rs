//! 单个桌面请求只选择一条派发路径；HLSL 帧不会被当作旧预乘 Canvas 帧处理。
pub enum Invocation {
    Native(super::shader_native_invocation::Invocation),
    Legacy(crate::shader_process::OwnedInvocation),
}
pub enum Frame {
    Native(crate::shader_hlsl::Frame),
    Legacy(crate::shader_angle::Frame),
}
impl From<crate::shader_process::OwnedInvocation> for Invocation {
    fn from(value: crate::shader_process::OwnedInvocation) -> Self {
        Self::Legacy(value)
    }
}
impl From<crate::shader_angle::Frame> for Frame {
    fn from(frame: crate::shader_angle::Frame) -> Self {
        Self::Legacy(frame)
    }
}
#[derive(Debug)]
pub enum Failure {
    Native(crate::shader_hlsl::process::Failure),
    Legacy(crate::shader_process::Failure),
}
impl Invocation {
    pub fn validate(&self) -> std::io::Result<()> {
        match self {
            Self::Native(invocation) => invocation.validate().map_err(|error| {
                std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("{error:?}"))
            }),
            Self::Legacy(invocation) => invocation.validate(),
        }
    }
    pub fn render(&self, stopped: &impl Fn() -> bool) -> Result<Frame, Failure> {
        match self {
            Self::Native(invocation) => invocation
                .render(stopped)
                .map(Frame::Native)
                .map_err(Failure::Native),
            Self::Legacy(invocation) => invocation
                .render(stopped)
                .map(Frame::Legacy)
                .map_err(Failure::Legacy),
        }
    }
}
