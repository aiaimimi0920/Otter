//! otter.hlsl.v1 单帧离屏合同。straight RGBA 与旧 WebGL premultiplied 帧不可互换。
//! 只由受限 child 调用渲染；父侧应使用 process::render，驱动调用不能在线程内强中断。
use crate::pixels::Pixels;
use std::time::Instant;

#[path = "shader_hlsl_compile.rs"]
mod compile;
#[path = "shader_hlsl_contract.rs"]
mod contract;
#[path = "shader_hlsl_d3d11.rs"]
mod d3d11;
#[path = "shader_hlsl_d3d12.rs"]
mod d3d12;
#[path = "shader_hlsl_pixels.rs"]
mod pixels;
#[path = "shader_hlsl_process.rs"]
pub mod process;
#[path = "shader_hlsl_wire.rs"]
mod wire;

pub const ABI: &str = "otter.hlsl.v1";
pub const MAX_SOURCE: usize = 1024 * 1024;
pub const MAX_TEXTURE_BYTES: usize = 256 * 1024 * 1024;
pub const CHILD_LIMIT: usize = 256 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Backend {
    Auto,
    D3d12,
    D3d11,
    Warp,
    Software,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelectedBackend {
    D3d12,
    D3d11,
    Warp,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    Device,
    Resource,
    Pipeline,
    Submit,
    Readback,
}
#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    Layout,
    Budget,
    Unsupported,
    Isolation,
    Cancelled,
    TimedOut,
    Compile(String),
    Api { stage: Stage, code: i32 },
}
pub type Result<T> = std::result::Result<T, Error>;
#[derive(Clone, Copy)]
pub struct Texture<'a> {
    pub name: &'a str,
    pub width: u32,
    pub height: u32,
    pub rgba: &'a [u8],
}
#[derive(Clone, Copy)]
pub struct Uniform<'a> {
    pub name: &'a str,
    pub value: f32,
}
pub struct Request<'a> {
    pub abi: &'a str,
    pub source: &'a str,
    pub width: u32,
    pub height: u32,
    pub textures: &'a [Texture<'a>],
    pub uniforms: &'a [Uniform<'a>],
}
#[derive(Debug)]
pub struct Frame {
    pub width: u32,
    pub height: u32,
    /// top-down straight RGBA8 UNORM；保留 alpha=0 时的 RGB，不自动预乘或 gamma 变换。
    pub rgba: Pixels,
    pub backend: SelectedBackend,
}

pub fn validate(request: &Request<'_>) -> Result<()> {
    contract::validate(request)
}
fn api(stage: Stage, error: windows::core::Error) -> Error {
    Error::Api {
        stage,
        code: error.code().0,
    }
}
fn check(stopped: &impl Fn() -> bool, deadline: Instant) -> Result<()> {
    if stopped() {
        Err(Error::Cancelled)
    } else if Instant::now() >= deadline {
        Err(Error::TimedOut)
    } else {
        Ok(())
    }
}

/// 禁止在 UI/普通父进程调用。Job 准入并非包授权，授权仍由调用者原 owner 管理。
pub fn render(
    request: &Request<'_>,
    backend: Backend,
    stopped: &impl Fn() -> bool,
    deadline: Instant,
) -> Result<Frame> {
    prepare(request, backend, stopped, deadline)?.finish(stopped, deadline)
}
// worker 可在 GPU 准备完成后释放私有请求纹理，再分配 CPU 输出；不额外复制输入。
fn prepare(
    request: &Request<'_>,
    backend: Backend,
    stopped: &impl Fn() -> bool,
    deadline: Instant,
) -> Result<Prepared> {
    check(stopped, deadline)?;
    validate(request)?;
    if backend == Backend::Software {
        return Err(Error::Unsupported);
    }
    crate::bounded_worker_process::require_limits(CHILD_LIMIT).map_err(|_| Error::Isolation)?;
    let device = match backend {
        Backend::Auto => match d3d12::Device::new() {
            Ok(value) => Device::D3d12(value),
            // new() 尚未提交 GPU 工作；失败局部资源完整析构后才进入唯一兼容候选。
            Err(_) => {
                check(stopped, deadline)?;
                Device::D3d11(d3d11::Device::new(false)?)
            }
        },
        Backend::D3d12 => Device::D3d12(d3d12::Device::new()?),
        Backend::D3d11 | Backend::Warp => {
            Device::D3d11(d3d11::Device::new(backend == Backend::Warp)?)
        }
        Backend::Software => return Err(Error::Unsupported),
    };
    check(stopped, deadline)?;
    let program = compile::Program::new(request, stopped, deadline)?;
    match device {
        Device::D3d12(device) => device
            .prepare(request, &program, stopped, deadline)
            .map(Prepared::D3d12),
        Device::D3d11(device) => device
            .render(request, &program, stopped, deadline)
            .map(Prepared::Frame),
    }
}
enum Prepared {
    D3d12(d3d12::Readback),
    Frame(Frame),
}
impl Prepared {
    fn finish(self, stopped: &impl Fn() -> bool, deadline: Instant) -> Result<Frame> {
        check(stopped, deadline)?;
        match self {
            Self::D3d12(value) => value.finish(stopped, deadline),
            Self::Frame(frame) => Ok(frame),
        }
    }
}
enum Device {
    D3d12(d3d12::Device),
    D3d11(d3d11::Device),
}

#[cfg(test)]
#[path = "shader_hlsl_contract_tests.rs"]
mod contract_tests;
#[cfg(test)]
#[path = "shader_hlsl_runtime_tests.rs"]
mod runtime_tests;
