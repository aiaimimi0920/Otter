//! Owned cross-process native sources plus real GDI/modeless recording/workers/previews/finish; the drag branch receives no wheel notifications.
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
    drag: bool,
    stage: u8,
    deadline: Instant,
    error: Option<String>,
    exercise_cover: bool,
    cover: Option<crate::desktop::Window>,
}
thread_local! { static DRIVER: RefCell<Option<Driver>> = const { RefCell::new(None) }; }
unsafe fn initialize_process_dpi() {
    use windows::Win32::UI::HiDpi::*;
    // Match desktop::run prerequisites; modify only the test process, not system/user settings.
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
            // Win32 may reenter timers during window creation/layout; drive sources only after actual recording startup.
            // Startup failure is still reported by the bounded deadline above and actual status text, never mistaken for success.
            if driver.stage == 0 && !input.recording.running() {
                return;
            }
            match driver.stage {
                0 => {
                    // Move only owned control windows without covering fixtures or changing user cursors/focus.
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
                    if driver.exercise_cover {
                        let region = driver.fixture.region;
                        driver.cover = Some(show_cover(corner_cover(region), true));
                        assert_center_is_source(region, driver.fixture.hwnd);
                        driver.stage = 4;
                    } else {
                        move_content(driver);
                        emit(driver, input);
                        driver.stage = 1;
                    }
                }
                4 if input.recording.covered.get() => {
                    assert!(input.recording.running());
                    assert!(input.recording.hook.borrow().is_some());
                    assert_eq!(input.job.borrow().snapshot().frame_count, 1);
                    let mut text = [0u16; 128];
                    let count = GetDlgItemTextW(hwnd, 103, &mut text) as usize;
                    assert!(String::from_utf16_lossy(&text[..count]).contains("随后自动继续"));
                    drop(driver.cover.take().unwrap());
                    driver.exercise_cover = false;
                    driver.stage = 0;
                }
                1 | 2
                    if !input.job.borrow().is_pending()
                        && input.preview.borrow().is_some()
                        && input.job.borrow().snapshot().frame_count
                            == usize::from(driver.stage) + 1 =>
                {
                    if driver.stage == 1 {
                        move_content(driver);
                        emit(driver, input);
                        driver.stage = 2;
                    } else {
                        assert!(!input.recording.covered.get());
                        let mut text = [0u16; 32];
                        let count = GetDlgItemTextW(hwnd, 103, &mut text) as usize;
                        assert_eq!(String::from_utf16_lossy(&text[..count]), "录制中");
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
    let delta = (i32::from(driver.stage) + 1) * if driver.forward { 8 } else { -8 };
    let expected = fixture::pixel(
        136 + if driver.axis == Axis::Horizontal {
            delta
        } else {
            0
        },
        136 + if driver.axis == Axis::Vertical {
            delta
        } else {
            0
        },
    );
    let sample = presented_sample(region, expected);
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
    if !driver.drag {
        hook::test_notify(point, driver.fixture.hwnd.0 as isize);
    }
}
fn presented_sample(region: Region, expected: [u8; 4]) -> Image {
    // Child-process drawing completion is not desktop-composition presentation; observe actual GDI pixels before driving the next step.
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        let sample =
            crate::desktop::capture::region(region.left, region.top, region.width, region.height)
                .unwrap();
        if sample.pixels[..4] == expected {
            return sample;
        }
        assert!(
            Instant::now() < deadline,
            "owned fixture was not presented: {:?}, expected {expected:?}",
            &sample.pixels[..4]
        );
        std::thread::sleep(Duration::from_millis(15));
    }
}
unsafe fn show_cover(region: Region, raise_over_overlay: bool) -> crate::desktop::Window {
    use std::sync::OnceLock;
    use windows::core::w;
    use windows::Win32::Graphics::{Dwm::DwmFlush, Gdi::*};
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;

    unsafe extern "system" fn cover_proc(
        hwnd: HWND,
        message: u32,
        wp: WPARAM,
        lp: LPARAM,
    ) -> LRESULT {
        DefWindowProcW(hwnd, message, wp, lp)
    }

    let class = w!("Otter.LongCapture.OwnedCoverFixture");
    let instance = GetModuleHandleW(None).unwrap();
    static CLASS: OnceLock<()> = OnceLock::new();
    CLASS.get_or_init(|| {
        assert_ne!(
            RegisterClassW(&WNDCLASSW {
                lpfnWndProc: Some(cover_proc),
                hInstance: instance.into(),
                hbrBackground: HBRUSH(GetStockObject(WHITE_BRUSH).0),
                lpszClassName: class,
                ..Default::default()
            }),
            0
        );
    });
    let cover = crate::desktop::Window(
        CreateWindowExW(
            WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW | WS_EX_TOPMOST,
            class,
            w!("Otter long capture cover fixture"),
            WS_POPUP,
            region.left,
            region.top,
            region.width,
            region.height,
            None,
            None,
            Some(instance.into()),
            None,
        )
        .unwrap(),
    );
    let _ = ShowWindow(cover.0, SW_SHOWNOACTIVATE);
    if raise_over_overlay {
        // The running recording already owns topmost windows on this desktop.
        SetWindowPos(
            cover.0,
            Some(HWND_TOPMOST),
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
        )
        .unwrap();
    }
    let _ = UpdateWindow(cover.0);
    DwmFlush().unwrap();
    let _ = presented_sample(region, [255, 255, 255, 255]);
    assert_eq!(
        WindowFromPoint(POINT {
            x: region.left + region.width / 2,
            y: region.top + region.height / 2,
        }),
        cover.0
    );
    cover
}
fn move_content(driver: &mut Driver) {
    if driver.drag {
        driver.fixture.drag(driver.axis, driver.forward);
    } else {
        driver.fixture.wheel(driver.axis, driver.forward);
    }
}
fn corner_cover(region: Region) -> Region {
    Region {
        width: region.width / 3,
        height: region.height / 3,
        ..region
    }
}
unsafe fn assert_center_is_source(region: Region, source: HWND) {
    let center = POINT {
        x: region.left + region.width / 2,
        y: region.top + region.height / 2,
    };
    assert_eq!(
        WindowFromPoint(center),
        source,
        "center must stay uncovered"
    );
}
#[test]
#[ignore = "explicit visible owned-window runtime gate; never touches user application input"]
fn owned_native_window_records_wheel_and_drag_in_four_directions_to_exact_final_pixels() {
    unsafe {
        initialize_process_dpi();
        let previous = windows::Win32::UI::HiDpi::SetThreadDpiAwarenessContext(
            windows::Win32::UI::HiDpi::DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
        );
        for (axis, drag) in [Axis::Vertical, Axis::Horizontal]
            .into_iter()
            .flat_map(|axis| [false, true].map(|drag| (axis, drag)))
        {
            for forward in [true, false] {
                let fixture = fixture::Fixture::start();
                let region = fixture.region;
                let first = presented_sample(region, fixture::pixel(136, 136));
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
                        drag,
                        stage: 0,
                        deadline: Instant::now() + Duration::from_secs(15),
                        error: None,
                        exercise_cover: axis == Axis::Vertical && forward && !drag,
                        cover: None,
                    })
                });
                let timer = SetTimer(None, 0, 30, Some(tick));
                assert_ne!(timer, 0);
                let result = super::super::show_recording(
                    HWND::default(),
                    RECT {
                        left: region.left,
                        top: region.top,
                        right: region.left + region.width,
                        bottom: region.top + region.height,
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
                        .join(format!("recording-{axis:?}-{forward}-drag-{drag}.png"));
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
    let _ = presented_sample(region, fixture::pixel(136, 136));
    let (mut job, _) = long_capture_source::start_recording(region).unwrap();
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

#[path = "long_capture_first_frame_runtime_tests.rs"]
mod first_frame;
