//! otter.hlsl.v1 single-frame offscreen contract. Straight RGBA and legacy WebGL premultiplied frames are not interchangeable.
//! Only constrained children invoke rendering; parents should use process::render, since driver calls cannot be forcibly interrupted within a thread.
use crate::pixels::Pixels;
use std::time::Instant;

#[path = "shader_hlsl_compile.rs"]
pub mod compile;
#[path = "shader_hlsl_contract.rs"]
pub mod contract;
#[path = "shader_hlsl_d3d11.rs"]
pub mod d3d11;
#[path = "shader_hlsl_d3d12.rs"]
pub mod d3d12;
#[path = "shader_hlsl_pixels.rs"]
pub mod pixels;
#[path = "shader_hlsl_process.rs"]
pub mod process;
#[path = "shader_hlsl_session.rs"]
pub mod session;
#[path = "shader_hlsl_wire.rs"]
pub mod wire;

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
    /// Top-down straight RGBA8 UNORM; preserve RGB at alpha=0, without automatic premultiplication or gamma transforms.
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

/// Never call from UI/ordinary parent processes. Job admission is not package authorization, which remains with the caller's original owner.
pub fn render(
    request: &Request<'_>,
    backend: Backend,
    stopped: &impl Fn() -> bool,
    deadline: Instant,
) -> Result<Frame> {
    prepare(request, backend, stopped, deadline)?.finish(stopped, deadline)
}
// Workers may release private request textures after GPU preparation, then allocate CPU output; no extra input copying.
fn prepare(
    request: &Request<'_>,
    backend: Backend,
    stopped: &impl Fn() -> bool,
    deadline: Instant,
) -> Result<Prepared> {
    check(stopped, deadline)?;
    validate(request)?;
    let device = create_device(backend, stopped, deadline)?;
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
fn create_device(
    backend: Backend,
    stopped: &impl Fn() -> bool,
    deadline: Instant,
) -> Result<Device> {
    if backend == Backend::Software {
        return Err(Error::Unsupported);
    }
    crate::bounded_worker_process::require_limits(CHILD_LIMIT).map_err(|_| Error::Isolation)?;
    match backend {
        Backend::Auto => match d3d12::Device::new() {
            Ok(value) => Ok(Device::D3d12(value)),
            // new() has not submitted GPU work; enter the sole compatibility candidate only after failed local resources are completely destroyed.
            Err(_) => {
                check(stopped, deadline)?;
                Ok(Device::D3d11(d3d11::Device::new(false)?))
            }
        },
        Backend::D3d12 => Ok(Device::D3d12(d3d12::Device::new()?)),
        Backend::D3d11 | Backend::Warp => {
            Ok(Device::D3d11(d3d11::Device::new(backend == Backend::Warp)?))
        }
        Backend::Software => Err(Error::Unsupported),
    }
}
pub(super) struct WarmEngine {
    backend: Backend,
    device: Device,
    compiler: compile::CompilerSession,
}
impl WarmEngine {
    pub fn prepare(
        slot: &mut Option<Self>,
        request: &Request<'_>,
        backend: Backend,
        stopped: &impl Fn() -> bool,
        deadline: Instant,
    ) -> Result<Prepared> {
        check(stopped, deadline)?;
        validate(request)?;
        if slot.as_ref().is_none_or(|engine| engine.backend != backend) {
            *slot = None;
            let device = create_device(backend, stopped, deadline)?;
            check(stopped, deadline)?;
            let compiler = compile::CompilerSession::new()?;
            *slot = Some(Self {
                backend,
                device,
                compiler,
            });
        }
        let engine = slot.as_mut().unwrap();
        let program = engine
            .compiler
            .compile_request(request, stopped, deadline)?;
        match &engine.device {
            Device::D3d12(device) => device
                .prepare(request, program, stopped, deadline)
                .map(Prepared::D3d12),
            Device::D3d11(device) => device
                .render(request, program, stopped, deadline)
                .map(Prepared::Frame),
        }
    }
}
pub(super) enum Prepared {
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
