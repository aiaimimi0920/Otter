//! 私有持久 JS worker：先验证 Job，再读 pipe；无 profile/文件/网络/模块加载宿主。
use crate::{
    surface_javascript::CHILD_LIMIT,
    surface_javascript_wire::{self as wire, Command},
};
use std::{
    ffi::OsString,
    io,
    os::windows::io::AsHandle,
    time::{Duration, Instant},
};
#[path = "surface_javascript_engine.rs"]
mod engine;
pub const FLAG: &str = "--native-javascript-worker";
pub fn parse(args: &[OsString]) -> io::Result<bool> {
    if !args.iter().any(|arg| arg == FLAG) {
        return Ok(false);
    }
    if args.len() != 1 {
        return Err(wire::invalid());
    }
    Ok(true)
}
pub fn run_inherited() -> io::Result<()> {
    crate::bounded_worker_process::require_limits(CHILD_LIMIT)?;
    let mut pipe =
        crate::live_recovery_pipe::Pipe::adopt(std::io::stdin().as_handle().try_clone_to_owned()?)?;
    let mut engine = engine::Engine::new()?;
    let mut sequence = 0u64;
    loop {
        // 空闲时无线程/轮询；断开即退出，遗忘的调用者最多保留 child 五分钟。
        let request: wire::Request =
            wire::read(&mut pipe.io(Some(Instant::now() + Duration::from_secs(300)), None))?;
        sequence = sequence.checked_add(1).ok_or_else(wire::invalid)?;
        if request.sequence != sequence {
            return Err(wire::invalid());
        }
        if let Command::InstallImages { headers } = request.command {
            if sequence != 1 || request.viewport.is_some() {
                return Err(wire::invalid());
            }
            let mut channel = pipe.io(Some(Instant::now() + Duration::from_secs(2)), None);
            engine.install_images(&headers, &mut channel)?;
            wire::write(&mut channel, &wire::images::Installed { sequence })?;
            continue;
        }
        let disposing = matches!(request.command, Command::Dispose);
        let mut service_id = 0;
        let service_deadline = Instant::now() + Duration::from_secs(60);
        let output = engine
            .apply_services(request.command, request.viewport, &mut |call| {
                call.validate()?;
                service_id += 1;
                if call.id != service_id {
                    return Err(wire::invalid());
                }
                let deadline = service_deadline
                    .min(Instant::now() + Duration::from_millis(u64::from(call.timeout_ms)));
                let mut channel = pipe.io(Some(deadline), None);
                wire::write(
                    &mut channel,
                    &wire::Response {
                        sequence,
                        service: Some(call),
                        output: None,
                        #[cfg(test)]
                        diagnostic: None,
                    },
                )?;
                let mut reply: wire::service::Reply = wire::read(&mut channel)?;
                if reply.sequence != sequence || reply.id != service_id {
                    return Err(wire::invalid());
                }
                reply.result.validate()?;
                if let wire::service::ResultValue::Image { image, png, .. } = &mut reply.result {
                    *png = Some(wire::images::read(&mut channel, image)?);
                }
                Ok(reply.result)
            })
            .ok();
        let failed = output.is_none();
        #[cfg(test)]
        let diagnostic = wire::diagnostics::capture(failed).then(|| engine.diagnostic());
        wire::write(
            &mut pipe.io(Some(Instant::now() + Duration::from_secs(2)), None),
            &wire::Response {
                sequence,
                service: None,
                output,
                #[cfg(test)]
                diagnostic,
            },
        )?;
        if failed || disposing {
            return Ok(());
        }
    }
}
#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    #[test]
    fn javascript_private_mode_is_exclusive() {
        assert!(parse(&[FLAG.into()]).unwrap());
        assert!(!parse(&["--settings".into()]).unwrap());
        for other in [
            "--settings",
            "--smoke-test",
            "--native-shader-worker",
            "--native-svg-worker",
        ] {
            assert!(parse(&[FLAG.into(), other.into()]).is_err());
        }
    }
    #[test]
    #[ignore = "private bounded JavaScript worker; parent fixtures only"]
    fn javascript_worker_entry() {
        std::process::exit(if run_inherited().is_ok() { 0 } else { 1 });
    }
    #[test]
    #[ignore = "explicit diagnostic parent only; same bounded worker and operation deadlines"]
    fn javascript_diagnostic_worker_entry() {
        wire::diagnostics::with_success_reports(|| {
            std::process::exit(if run_inherited().is_ok() { 0 } else { 1 });
        });
    }
}
