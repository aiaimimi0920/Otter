//! 父侧隔离 Shader API：可信 bundle 与 Art 数据分离，永不在父进程加载 ANGLE。
use crate::shader_angle::{self as angle, Backend, Frame, Identity, Request};
use std::{
    io,
    path::{Component, Path, PathBuf, Prefix},
    sync::Arc,
};
#[cfg(test)]
use std::{
    sync::LazyLock,
    time::{Duration, Instant},
};
#[cfg(test)]
#[path = "shader_process_client.rs"]
mod client;
#[path = "shader_process_request.rs"]
mod owned;
pub use owned::{OwnedInvocation, OwnedTexture};
#[cfg(test)]
pub(crate) const CHILD_LIMIT: usize = 256 * 1024 * 1024;
#[cfg(test)]
pub(crate) const TIMEOUT: Duration = Duration::from_secs(15);
// Shader 一次只运行一个 child；不扩张图片原有的两个 child / 256 MiB 池。
#[cfg(test)]
pub(crate) static CAPACITY: LazyLock<Arc<crate::byte_budget::Budget>> =
    LazyLock::new(|| crate::byte_budget::Budget::new(CHILD_LIMIT));
pub struct TrustedBundle {
    pub(crate) directory: PathBuf,
    #[cfg(test)]
    pub(crate) identity: Identity,
    // 显式候选 owner 的路径/ACL/文件租约必须覆盖所有后台 invocation。
    _files: Vec<std::fs::File>,
}
impl TrustedBundle {
    /// 调用方必须从可信发行清单提供 identity 和可信 ACL 的本地目录，不得来自 Art。
    /// 此构造只检查位置/清单形状；实际文件摘要和写锁由受限 child 在加载前核验。
    pub fn new(directory: &Path, identity: Identity) -> io::Result<Self> {
        let directory = directory.canonicalize()?;
        if !matches!(directory.components().next(), Some(Component::Prefix(prefix))
            if matches!(prefix.kind(), Prefix::Disk(_) | Prefix::VerbatimDisk(_)))
            || !directory.is_dir()
            || [identity.egl, identity.gles]
                .iter()
                .any(|item| item.bytes == 0 || item.bytes > 32 * 1024 * 1024)
        {
            return Err(crate::shader_worker_wire::invalid());
        }
        Ok(Self {
            directory,
            #[cfg(test)]
            identity,
            _files: Vec::new(),
        })
    }
    pub(crate) fn with_files(mut self, files: Vec<std::fs::File>) -> Self {
        self._files = files;
        self
    }
}
pub struct Invocation<'a> {
    pub bundle: &'a TrustedBundle,
    pub backend: Backend,
    pub width: u32,
    pub height: u32,
    pub request: Request<'a>,
}
#[derive(Debug)]
pub enum Failure {
    Transport(io::Error),
    Backend(angle::Error),
}
impl From<io::Error> for Failure {
    fn from(error: io::Error) -> Self {
        Self::Transport(error)
    }
}
#[cfg(not(test))]
pub fn render(_invocation: Invocation<'_>, _stopped: &impl Fn() -> bool) -> Result<Frame, Failure> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "Legacy ANGLE rendering is retired",
    )
    .into())
}
#[cfg(test)]
pub fn render(invocation: Invocation<'_>, stopped: &impl Fn() -> bool) -> Result<Frame, Failure> {
    client::run(
        invocation,
        Instant::now() + TIMEOUT,
        stopped,
        |endpoint, deadline| {
            let suffix = if cfg!(test) {
                " --exact shader_worker::tests::shader_worker_entry --ignored --nocapture"
            } else {
                " --native-shader-worker"
            };
            crate::bounded_worker_process::start(
                endpoint,
                deadline,
                stopped,
                crate::bounded_worker_process::Policy {
                    suffix,
                    maximum: CHILD_LIMIT,
                    pool: &CAPACITY,
                    environment: crate::bounded_worker_process::Environment::SystemRoot,
                },
            )
        },
    )
}
#[cfg(test)]
#[path = "shader_process_runtime_tests.rs"]
mod runtime_tests;
#[cfg(test)]
#[path = "shader_process_tests.rs"]
mod tests;
