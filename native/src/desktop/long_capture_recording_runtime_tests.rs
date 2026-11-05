//! 自建跨进程原生滚动窗 + 真 GDI/DialogBox/worker/preview/finish；只有 wheel 通知是合成的。
use super::*;
use std::time::Duration;
use windows::core::BOOL;
use windows::Win32::System::Threading::GetCurrentThreadId;
#[path = "long_capture_recording_fixture.rs"]
mod fixture;
struct Driver {
    fixture: fixture::Fixture,
    axis: Axis,
    forward: bool,
    stage: u8,
    deadline: Instant,
    error: Option<String>,
}
thread_local! { static DRIVER: RefCell<Option<Driver>> = const { RefCell::new(None) }; }
unsafe fn initialize_process_dpi() {
    use windows::Win32::UI::HiDpi::*;
    // 与产品 desktop::run 的前置条件一致；只改测试进程，不写系统或用户设置。
    if let Err(error) = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) {
        assert!(
            AreDpiAwarenessContextsEqual(
                GetThreadDpiAwarenessContext(),
                DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2
            )
            .as_bool(),
            "runtime process DPI initialization: {error}"
        );
    }
}
unsafe extern "system" fn find_dialog(hwnd: HWND, lp: LPARAM) -> BOOL {
    if GetDlgItem(Some(hwnd), 112).is_ok()
        && GetWindowLongPtrW(hwnd, WINDOW_LONG_PTR_INDEX(USER)) != 0
    {
        *(lp.0 as *mut HWND) = hwnd;
        return BOOL(0);
    }
    BOOL(1)
}
unsafe extern "system" fn tick(_: HWND, _: u32, _: usize, _: u32) {
    let mut hwnd = HWND::default();
    let _ = EnumThreadWindows(
        GetCurrentThreadId(),
        Some(find_dialog),
        LPARAM(&mut hwnd as *mut HWND as isize),
    );
    if hwnd.is_invalid() {
        return;
    }
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        DRIVER.with(|slot| {
            let mut value = slot.borrow_mut();
            let driver = value.as_mut().unwrap();
            let input = &*(GetWindowLongPtrW(hwnd, WINDOW_LONG_PTR_INDEX(USER)) as *const Input);
            assert!(
                Instant::now() < driver.deadline,
                "recording runtime timed out at stage {}; snapshot={:?}; pending={}; phase={:?}; preview={}; status={}",
                driver.stage, input.job.borrow().snapshot(), input.job.borrow().is_pending(), input.recording.schedule.borrow().phase(), input.preview.borrow().is_some(), {
                    let mut text = [0u16; 256]; let count = GetDlgItemTextW(hwnd, 103, &mut text); String::from_utf16_lossy(&text[..count as usize])
                }
            );
            match driver.stage {
                0 => {
                    // 只移动自己的控制窗，不覆盖 fixture，不更改用户光标或焦点。
                    SetWindowPos(
                        hwnd,
                        None,
                        driver.fixture.region.left + 200,
                        driver.fixture.region.top,
                        0,
                        0,
                        SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
                    )
                    .unwrap();
                    assert!(input.recording.running());
                    driver.fixture.wheel(driver.axis, driver.forward);
                    emit(driver, input);
                    driver.stage = 1;
                }
                1 | 2
                    if !input.job.borrow().is_pending()
                        && input.preview.borrow().is_some()
                        && input.job.borrow().snapshot().frame_count
                            == usize::from(driver.stage) + 1 =>
                {
                    if driver.stage == 1 {
                        driver.fixture.wheel(driver.axis, driver.forward);
                        emit(driver, input);
                        driver.stage = 2;
                    } else {
                        super::super::handle(hwnd, WM_COMMAND, WPARAM(112), LPARAM(0));
                        assert!(input.recording.hook.borrow().is_none());
                        super::super::handle(hwnd, WM_COMMAND, WPARAM(1), LPARAM(0));
                        driver.stage = 3;
                    }
                }
                _ => {}
            }
        });
    }));
    if result.is_err() {
        DRIVER.with(|slot| {
            if let Some(driver) = slot.borrow_mut().as_mut() {
                driver.error = Some("runtime driver assertion failed".into());
            }
        });
        super::super::handle(hwnd, WM_CLOSE, WPARAM(0), LPARAM(0));
    }
}
unsafe fn emit(driver: &Driver, input: &Input) {
    let point = POINT {
        x: driver.fixture.region.left + 16,
        y: driver.fixture.region.top + 16,
    };
    assert_eq!(
        input
            .recording
            .target
            .as_ref()
            .unwrap()
            .accepts_point(point),
        Ok(true),
        "owned fixture must remain the bound wheel target"
    );
    let region = driver.fixture.region;
    let sample =
        crate::desktop::capture::region(region.left, region.top, region.width, region.height)
            .unwrap();
    let delta = (i32::from(driver.stage) + 1) * if driver.forward { 8 } else { -8 };
    assert_eq!(
        &sample.pixels[..4],
        &fixture::pixel(
            136 + if driver.axis == Axis::Horizontal {
                delta
            } else {
                0
            },
            136 + if driver.axis == Axis::Vertical {
                delta
            } else {
                0
            }
        ),
        "owned fixture must really scroll before the synthetic notification"
    );
    hook::test_notify(point, driver.fixture.hwnd.0 as isize);
}
#[test]
#[ignore = "explicit visible owned-window runtime gate; never touches user application input"]
fn owned_native_window_records_all_four_directions_to_exact_final_pixels() {
    unsafe {
        initialize_process_dpi();
        let previous = windows::Win32::UI::HiDpi::SetThreadDpiAwarenessContext(
            windows::Win32::UI::HiDpi::DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
        );
        for axis in [Axis::Vertical, Axis::Horizontal] {
            for forward in [true, false] {
                let fixture = fixture::Fixture::start();
                let region = fixture.region;
                let first = crate::desktop::capture::region(
                    region.left,
                    region.top,
                    region.width,
                    region.height,
                )
                .unwrap();
                assert_eq!(
                    &first.pixels[..4],
                    &fixture::pixel(136, 136),
                    "initial owned GDI pixel"
                );
                DRIVER.with(|slot| {
                    *slot.borrow_mut() = Some(Driver {
                        fixture,
                        axis,
                        forward,
                        stage: 0,
                        deadline: Instant::now() + Duration::from_secs(15),
                        error: None,
                    })
                });
                let timer = SetTimer(None, 0, 30, Some(tick));
                assert_ne!(timer, 0);
                let result = super::super::show_recording(
                    HWND::default(),
                    first,
                    POINT {
                        x: region.left,
                        y: region.top,
                    },
                    &std::rc::Rc::new(Cell::new(true)),
                );
                KillTimer(None, timer).unwrap();
                let driver = DRIVER.with(|slot| slot.borrow_mut().take().unwrap());
                assert!(driver.error.is_none(), "{:?}", driver.error);
                let output = result.unwrap().expect("recording cancelled");
                assert_eq!(driver.stage, 3);
                assert_eq!(output.axis, Some(axis));
                assert_eq!(
                    output.direction,
                    crate::long_capture::Direction::from_offset(axis, if forward { 8 } else { -8 })
                );
                assert_eq!(output.frame_count, 3);
                assert_eq!(output.source_region, region);
                let (width, height) = if axis == Axis::Vertical {
                    (128, 112)
                } else {
                    (144, 96)
                };
                assert_eq!((output.image.width, output.image.height), (width, height));
                let x0 = 136
                    - if !forward && axis == Axis::Horizontal {
                        16
                    } else {
                        0
                    };
                let y0 = 136
                    - if !forward && axis == Axis::Vertical {
                        16
                    } else {
                        0
                    };
                for y in 0..height {
                    for x in 0..width {
                        let at = ((y * width + x) * 4) as usize;
                        assert_eq!(
                            &output.image.pixels[at..at + 4],
                            fixture::pixel(x0 + x as i32, y0 + y as i32).as_slice(),
                            "pixel {x},{y}"
                        );
                    }
                }
                if let Ok(directory) = std::env::var("OTTER_CAPTURE_EVIDENCE") {
                    let path = std::path::Path::new(&directory)
                        .join(format!("recording-{axis:?}-{forward}.png"));
                    let file = std::fs::File::create(path).unwrap();
                    let mut encoder = png::Encoder::new(file, width, height);
                    encoder.set_color(png::ColorType::Rgba);
                    encoder.set_depth(png::BitDepth::Eight);
                    let rgba = output
                        .image
                        .pixels
                        .chunks_exact(4)
                        .flat_map(|p| [p[2], p[1], p[0], p[3]])
                        .collect::<Vec<_>>();
                    encoder
                        .write_header()
                        .unwrap()
                        .write_image_data(&rgba)
                        .unwrap();
                }
                assert!(hook::test_inactive());
            }
        }
        windows::Win32::UI::HiDpi::SetThreadDpiAwarenessContext(previous);
    }
}
#[test]
#[ignore = "explicit visible owned-window target lifetime gate"]
fn owned_target_move_is_rejected_without_publishing_new_pixels() {
    unsafe {
        initialize_process_dpi();
    }
    let previous = unsafe {
        windows::Win32::UI::HiDpi::SetThreadDpiAwarenessContext(
            windows::Win32::UI::HiDpi::DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
        )
    };
    let mut fixture = fixture::Fixture::start();
    let region = fixture.region;
    let first =
        crate::desktop::capture::region(region.left, region.top, region.width, region.height)
            .unwrap();
    let (mut job, _) = long_capture_source::start_recording(region, first.pixels).unwrap();
    fixture.move_window();
    job.capture().unwrap();
    let end = Instant::now() + Duration::from_secs(5);
    let reply = loop {
        if let Some(reply) = job.take_ready() {
            break reply;
        }
        assert!(Instant::now() < end);
        std::thread::sleep(Duration::from_millis(5));
    };
    assert!(matches!(reply.completion, Completion::CaptureFailed(_)));
    assert_eq!(reply.snapshot.frame_count, 1);
    assert_eq!(reply.snapshot.axis, None);
    unsafe {
        windows::Win32::UI::HiDpi::SetThreadDpiAwarenessContext(previous);
    }
}
