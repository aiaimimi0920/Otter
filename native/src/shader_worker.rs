//! 私有 Shader child：先核验 Job，再读取单帧、加载可信 bundle、原样渲染；不读 UI/profile。
use crate::{
    shader_angle::Renderer,
    shader_process::{CHILD_LIMIT, TIMEOUT},
    shader_worker_wire as wire,
};
use std::{ffi::OsString, io, os::windows::io::AsHandle, sync::atomic::AtomicBool, time::Instant};
pub const FLAG: &str = "--native-shader-worker";
pub fn parse(args: &[OsString]) -> io::Result<bool> {
    if !args.iter().any(|arg| arg == FLAG) {
        return Ok(false);
    }
    if args.len() != 1 || args[0] != FLAG {
        return Err(wire::invalid());
    }
    Ok(true)
}
pub fn run_inherited() -> io::Result<()> {
    crate::bounded_worker_process::require_limits(CHILD_LIMIT)?;
    let mut pipe =
        crate::live_recovery_pipe::Pipe::adopt(std::io::stdin().as_handle().try_clone_to_owned()?)?;
    let mut channel = pipe.io(Some(Instant::now() + TIMEOUT), None);
    let request = wire::read_request(&mut channel)?;
    let result = request.with_invocation(|value| {
        let mut renderer = Renderer::new(
            &value.bundle.directory,
            value.bundle.identity,
            value.width,
            value.height,
            value.backend,
        )?;
        renderer.render(value.request, &AtomicBool::new(false))
    });
    // 编译失败是明确 Backend failure 帧，不是成功图片；父侧仍要求正常退出与 EOF。
    wire::write_response(&mut channel, &result)
}
#[cfg(test)]
#[path = "shader_worker_fixture.rs"]
mod fixtures;
#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    #[test]
    fn shader_private_mode_is_exclusive_before_settings() {
        assert!(parse(&[FLAG.into()]).unwrap());
        assert!(!parse(&["--smoke-test".into()]).unwrap());
        for suffix in [
            "--settings",
            "--native-svg-worker",
            "--native-webp-worker",
            "--smoke-test",
        ] {
            assert!(parse(&[FLAG.into(), suffix.into()]).is_err());
        }
    }
    #[test]
    #[ignore = "private bounded Shader child entry; invoked by parent fixtures"]
    fn shader_worker_entry() {
        std::process::exit(if run_inherited().is_ok() { 0 } else { 1 });
    }
}
