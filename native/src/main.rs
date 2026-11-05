//! 独立原生入口；使用隔离的 OtterNative profile，--settings 优先，不自动访问主干目录。
#[cfg(windows)]
mod startup_session;
#[cfg(windows)]
mod startup_settings;
#[cfg(windows)]
fn main() -> windows::core::Result<()> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    // 已退役的 ANGLE child 不得进入任何私有 worker 或读取设置/继承句柄。
    if args.iter().any(|arg| arg == "--native-shader-worker") {
        return Err(windows::core::Error::new(
            windows::Win32::Foundation::E_INVALIDARG,
            "Legacy ANGLE worker is retired",
        ));
    }
    if otter_native::live_source_uia::process::parse(&args).map_err(|e| {
        windows::core::Error::new(windows::Win32::Foundation::E_INVALIDARG, e.to_string())
    })? {
        return otter_native::live_source_uia::process::run_inherited().map_err(|e| {
            windows::core::Error::new(windows::Win32::Foundation::E_FAIL, e.to_string())
        });
    }
    if otter_native::surface_javascript_worker::parse(&args).map_err(|e| {
        windows::core::Error::new(windows::Win32::Foundation::E_INVALIDARG, e.to_string())
    })? {
        return otter_native::surface_javascript_worker::run_inherited().map_err(|e| {
            windows::core::Error::new(windows::Win32::Foundation::E_FAIL, e.to_string())
        });
    }
    if otter_native::shader_hlsl::process::parse(&args).map_err(|e| {
        windows::core::Error::new(windows::Win32::Foundation::E_INVALIDARG, e.to_string())
    })? {
        return otter_native::shader_hlsl::process::run_inherited().map_err(|e| {
            windows::core::Error::new(windows::Win32::Foundation::E_FAIL, e.to_string())
        });
    }
    if otter_native::surface_webp_worker::parse(&args).map_err(|e| {
        windows::core::Error::new(windows::Win32::Foundation::E_INVALIDARG, e.to_string())
    })? {
        return otter_native::surface_webp_worker::run_inherited().map_err(|e| {
            windows::core::Error::new(windows::Win32::Foundation::E_FAIL, e.to_string())
        });
    }
    if otter_native::surface_svg_worker::parse(&args).map_err(|e| {
        windows::core::Error::new(windows::Win32::Foundation::E_INVALIDARG, e.to_string())
    })? {
        return otter_native::surface_svg_worker::run_inherited().map_err(|e| {
            windows::core::Error::new(windows::Win32::Foundation::E_FAIL, e.to_string())
        });
    }
    // 恢复子进程必须在设置、会话和桌面初始化之前分流，不能访问用户 profile。
    if let Some(parent) = otter_native::live_recovery_runtime::parse(&args).map_err(|error| {
        windows::core::Error::new(windows::Win32::Foundation::E_INVALIDARG, error.to_string())
    })? {
        return otter_native::live_recovery_runtime::run_inherited(parent).map_err(|error| {
            windows::core::Error::new(windows::Win32::Foundation::E_FAIL, error.to_string())
        });
    }
    if let Some(parent) = otter_native::emergency_watchdog::parse(&args).map_err(|error| {
        windows::core::Error::new(windows::Win32::Foundation::E_INVALIDARG, error.to_string())
    })? {
        return otter_native::emergency_watchdog::run_inherited(parent).map_err(|error| {
            windows::core::Error::new(windows::Win32::Foundation::E_FAIL, error.to_string())
        });
    }
    // 私有worker已分流；正常宿主独占一个watchdog，退出后只回收该自建child。
    let _emergency = otter_native::emergency_watchdog::install();
    if let Some(command) = otter_native::tile_cli::parse(&args).map_err(|message| {
        windows::core::Error::new(windows::Win32::Foundation::E_INVALIDARG, message)
    })? {
        return otter_native::desktop::tile_terminal::run(command);
    }
    let (args, options) = startup_settings::extract(args).map_err(|message| {
        windows::core::Error::new(windows::Win32::Foundation::E_INVALIDARG, message)
    })?;
    let settings = options
        .settings
        .map(std::path::absolute)
        .transpose()
        .map_err(|error| {
            windows::core::Error::new(windows::Win32::Foundation::E_INVALIDARG, error.to_string())
        })?;
    // 候选目录必须原样为绝对本地路径，不能将相对/设备路径预先绝对化或 canonicalize。
    let startup = otter_native::desktop::Startup {
        settings: settings.as_deref(),
        shader_candidate: options.candidate.as_deref(),
        shader_smoke_peer: options.peer.as_deref(),
    };
    if args.iter().any(|argument| argument == "--workspace-check") {
        if args.len() != 1 {
            return Err(windows::core::Error::new(
                windows::Win32::Foundation::E_INVALIDARG,
                "--workspace-check cannot be combined with workspace inputs",
            ));
        }
        return startup.check_workspace();
    }
    if let Some(session) = startup_session::parse(&args).map_err(|message| {
        windows::core::Error::new(windows::Win32::Foundation::E_INVALIDARG, message)
    })? {
        let absolute = |path: std::path::PathBuf| {
            std::path::absolute(path).map_err(|error| {
                windows::core::Error::new(
                    windows::Win32::Foundation::E_INVALIDARG,
                    error.to_string(),
                )
            })
        };
        return startup.run_session(
            absolute(session.path)?,
            session.assets.map(absolute).transpose()?,
            session.space,
            session.check,
        );
    }
    let mut image = None;
    let mut smoke = false;
    let mut pinned = false;
    let mut capture = false;
    let mut region = false;
    for argument in args {
        if argument == "--smoke-test" {
            smoke = true;
        } else if argument == "--pin" {
            pinned = true;
        } else if argument == "--capture" {
            capture = true;
        } else if argument == "--capture-region" {
            capture = true;
            region = true;
        } else if image.is_none() && !argument.to_string_lossy().starts_with('-') {
            image = Some(std::path::PathBuf::from(argument));
        } else {
            return Err(windows::core::Error::new(
                windows::Win32::Foundation::E_INVALIDARG,
                "Usage: otter-native [image-path] [--pin] [--smoke-test] [--settings path] | otter-native (--capture | --capture-region) [--pin] [--settings path]",
            ));
        }
    }
    if capture && (image.is_some() || smoke) {
        return Err(windows::core::Error::new(
            windows::Win32::Foundation::E_INVALIDARG,
            "--capture cannot be combined with image-path or --smoke-test",
        ));
    }
    if capture {
        return startup.run_capture(pinned, region);
    }
    if pinned && image.is_none() {
        return Err(windows::core::Error::new(
            windows::Win32::Foundation::E_INVALIDARG,
            "--pin requires an image path",
        ));
    }
    startup.run(image.as_deref(), smoke, pinned)
}

#[cfg(not(windows))]
fn main() {
    eprintln!("Otter native requires Windows");
    std::process::exit(1);
}
