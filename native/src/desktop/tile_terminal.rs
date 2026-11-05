//! 独立 Tile 管理/输出路径；复用设备配对与授权 owner，普通截图工作区不初始化。
use crate::{
    device_authorization_config::{Action, Config},
    device_authorization_owner::{Event, Owner},
    device_authorization_runtime::Task,
    tile_cli::Command,
    tile_runtime::{now_ms, Runtime},
};
use std::{
    path::Path,
    time::{Duration, Instant},
};
use windows::{
    core::{Error, Result},
    Win32::{
        Foundation::*,
        System::{Com::*, Threading::*},
        UI::{HiDpi::*, WindowsAndMessaging::*},
    },
};
#[path = "tile_manager.rs"]
mod manager;
#[path = "tile_output_owner.rs"]
mod output;
#[path = "tile_output_status.rs"]
mod status;
fn fail(message: impl Into<String>) -> Error {
    Error::new(E_FAIL, message.into())
}
pub(super) fn config(origin: &str, profile: Option<&Path>) -> Result<Config> {
    Config::from_lookup(origin, |key| {
        if key == "OTTER_NATIVE_PROFILE_DIR" {
            profile
                .map(|p| p.as_os_str().to_owned())
                .or_else(|| std::env::var_os(key))
        } else {
            std::env::var_os(key)
        }
    })
    .map_err(|e| fail(format!("Tile configuration: {e:?}")))
}
pub fn run(command: Command) -> Result<()> {
    if command == Command::Outputs {
        let outputs = crate::tile_outputs::enumerate().map_err(fail)?;
        println!(
            "{}",
            serde_json::to_string(&outputs).map_err(|_| fail("Tile output JSON"))?
        );
        return Ok(());
    }
    let Command::Run {
        output,
        origin,
        profile,
    } = command
    else {
        unreachable!()
    };
    let name = format!(
        "Local\\Otter.Native.Tile.{}",
        output.as_deref().unwrap_or("control")
    );
    let wide: Vec<u16> = name.encode_utf16().chain(Some(0)).collect();
    let handle = unsafe { CreateMutexW(None, false, windows::core::PCWSTR(wide.as_ptr()))? };
    struct Guard(HANDLE);
    impl Drop for Guard {
        fn drop(&mut self) {
            unsafe {
                let _ = CloseHandle(self.0);
            }
        }
    }
    let _guard = Guard(handle);
    if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
        return Err(fail("Tile output already running"));
    }
    unsafe {
        CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok()?;
    }
    struct Com;
    impl Drop for Com {
        fn drop(&mut self) {
            unsafe {
                CoUninitialize();
            }
        }
    }
    let _com = Com;
    let prior = unsafe { SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) };
    if prior.0.is_null() {
        return Err(Error::from_win32());
    }
    struct Dpi(DPI_AWARENESS_CONTEXT);
    impl Drop for Dpi {
        fn drop(&mut self) {
            unsafe {
                let _ = SetThreadDpiAwarenessContext(self.0);
            }
        }
    }
    let _dpi = Dpi(prior);
    match output {
        Some(id) => output_run(&id, &origin, profile.as_deref()),
        None => manager::run(&origin, profile.as_deref()),
    }
}
fn authorize(origin: &str, profile: Option<&Path>) -> Result<Owner> {
    let mut owner = Owner::new();
    owner
        .start(Task::Configured {
            config: config(origin, profile)?,
            action: Action::Renew,
        })
        .map_err(|e| fail(format!("Tile authorization: {e:?}")))?;
    loop {
        pump()?;
        for event in owner.poll() {
            match event {
                Event::Authorized { .. } => return Ok(owner),
                Event::Failed { .. } => {
                    return Err(fail(
                        "Tile device authorization failed; pair/approve in Tile manager first",
                    ))
                }
                _ => {}
            }
        }
        wait(50);
    }
}
fn output_run(id: &str, origin: &str, profile: Option<&Path>) -> Result<()> {
    let mut owner = authorize(origin, profile)?;
    let status = status::StatusWindow::new()?;
    let mut output = output::Output::new(id);
    output.observe(crate::tile_outputs::enumerate());
    let mut probe = Instant::now();
    let mut error = None;
    loop {
        if let Err(e) = pump() {
            error = Some(e);
            break;
        }
        if status.stopped() {
            output.stop();
        }
        if probe.elapsed() >= Duration::from_millis(500) {
            output.observe(crate::tile_outputs::enumerate());
            probe = Instant::now();
        }
        match output.tick(&mut owner, origin, profile) {
            Ok(output::Status::Running) => status.hide(),
            Ok(output::Status::Waiting) => status.show("Otter Tile · 等待显示器"),
            Ok(output::Status::Draining) => status.show("Otter Tile · 正在回收输出"),
            Ok(output::Status::Stopped) => break,
            Err(e) => {
                error = Some(e);
                break;
            }
        }
        wait(100);
    }
    output.stop();
    while !output.drain() {
        if let Err(e) = pump() {
            if error.is_none() {
                error = Some(e);
            }
        }
        wait(25);
    }
    owner.close();
    error.map_or(Ok(()), Err)
}
pub(super) fn pump() -> Result<()> {
    unsafe {
        let mut message = MSG::default();
        while PeekMessageW(&mut message, None, 0, 0, PM_REMOVE).as_bool() {
            if message.message == WM_QUIT {
                return Err(fail("Tile stopped"));
            }
            let _ = TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }
    Ok(())
}
pub(super) fn wait(ms: u32) {
    unsafe {
        let _ = MsgWaitForMultipleObjectsEx(None, ms, QS_ALLINPUT, MWMO_INPUTAVAILABLE);
    }
}
