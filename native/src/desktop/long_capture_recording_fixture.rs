//! Control only a self-started Child's native scrolling window; bounded pipes, HWND/PID verification, and Drop reclaiming process/read threads.
use super::*;
use std::{
    io::{BufRead, BufReader, Read, Write},
    process::{Child, Command, Stdio},
    sync::mpsc::{sync_channel, Receiver},
    thread::JoinHandle,
    time::Duration,
};
use windows::{
    core::w,
    Win32::Graphics::{Dwm::DwmFlush, Gdi::*},
};
const CHILD: &str = "desktop::capture_input::long_capture_dialog::recording::runtime_tests::fixture::recording_window_child";
pub(super) struct Fixture {
    child: Child,
    reader: Option<JoinHandle<()>>,
    events: Receiver<String>,
    pub hwnd: HWND,
    pub region: Region,
}
impl Fixture {
    pub fn start() -> Self {
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                CHILD,
                "--ignored",
                "--nocapture",
                "--test-threads=1",
            ])
            .env("OTTER_CAPTURE_CHILD", "1")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        let (send, events) = sync_channel(8);
        let output = child.stdout.take().unwrap();
        let reader = std::thread::spawn(move || {
            for line in BufReader::new(output)
                .take(64 * 1024)
                .lines()
                .map_while(std::result::Result::ok)
            {
                if let Some((_, value)) = line.split_once("CAPTURE_FIXTURE=") {
                    let _ = send.try_send(value.trim().to_owned());
                }
            }
        });
        let mut owner = Self {
            child,
            reader: Some(reader),
            events,
            hwnd: HWND::default(),
            region: Region {
                left: 0,
                top: 0,
                width: 128,
                height: 96,
            },
        };
        let ready = owner
            .events
            .recv_timeout(Duration::from_secs(10))
            .expect("capture fixture not ready");
        let parts = ready.split_whitespace().collect::<Vec<_>>();
        assert_eq!(parts.len(), 3);
        owner.hwnd = HWND(usize::from_str_radix(parts[0], 16).unwrap() as *mut _);
        let mut pid = 0;
        unsafe {
            GetWindowThreadProcessId(owner.hwnd, Some(&mut pid));
        }
        assert_eq!(pid, owner.child.id(), "refusing foreign HWND");
        owner.region.left = parts[1].parse::<i32>().unwrap() + 8;
        owner.region.top = parts[2].parse::<i32>().unwrap() + 8;
        owner
    }
    pub fn wheel(&mut self, axis: Axis, forward: bool) {
        assert!(self.child.try_wait().unwrap().is_none());
        let (axis, delta) = match axis {
            Axis::Vertical => ("v", if forward { -120 } else { 120 }),
            Axis::Horizontal => ("h", if forward { 120 } else { -120 }),
        };
        writeln!(self.child.stdin.as_mut().unwrap(), "{axis} {delta}").unwrap();
        assert_eq!(
            self.events
                .recv_timeout(Duration::from_secs(5))
                .expect("fixture wheel not acknowledged"),
            "scrolled"
        );
    }
    pub fn move_window(&mut self) {
        writeln!(self.child.stdin.as_mut().unwrap(), "move").unwrap();
        assert_eq!(
            self.events.recv_timeout(Duration::from_secs(5)).unwrap(),
            "moved"
        );
    }
    pub fn drag(&mut self, axis: Axis, forward: bool) {
        let delta = if forward { -8 } else { 8 };
        let (x, y) = if axis == Axis::Horizontal {
            (delta, 0)
        } else {
            (0, delta)
        };
        writeln!(self.child.stdin.as_mut().unwrap(), "drag {x} {y}").unwrap();
        assert_eq!(
            self.events.recv_timeout(Duration::from_secs(5)).unwrap(),
            "dragged"
        );
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.child.stdin.take();
        let _ = self.child.kill();
        let _ = self.child.wait();
        if let Some(reader) = self.reader.take() {
            let _ = reader.join();
        }
    }
}
pub(super) fn pixel(x: i32, y: i32) -> [u8; 4] {
    let mut seed = (x as u32).wrapping_mul(7919) ^ (y as u32).wrapping_mul(104729);
    seed = (seed ^ (seed >> 16)).wrapping_mul(0x7feb352d);
    seed = (seed ^ (seed >> 15)).wrapping_mul(0x846ca68b);
    let value = (seed ^ (seed >> 16)) as u8;
    [value, value / 2, 255 - value, 255]
}
struct Scene {
    x: Cell<i32>,
    y: Cell<i32>,
    pointer: Cell<Option<(i32, i32)>>,
}
unsafe extern "system" fn procedure(hwnd: HWND, message: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    let pointer = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const Scene;
    if let Some(scene) = pointer.as_ref() {
        match message {
            WM_LBUTTONDOWN => {
                scene
                    .pointer
                    .set(Some((lp.0 as i16 as i32, (lp.0 >> 16) as i16 as i32)));
                return LRESULT(0);
            }
            WM_MOUSEMOVE => {
                let point = (lp.0 as i16 as i32, (lp.0 >> 16) as i16 as i32);
                if let Some(previous) = scene.pointer.get() {
                    scene.pointer.set(Some(point));
                    scene.x.set(scene.x.get() + previous.0 - point.0);
                    scene.y.set(scene.y.get() + previous.1 - point.1);
                    let _ = InvalidateRect(Some(hwnd), None, false);
                    let _ = UpdateWindow(hwnd);
                }
                return LRESULT(0);
            }
            WM_LBUTTONUP => {
                scene.pointer.set(None);
                return LRESULT(0);
            }
            value if value == WM_APP + 2 => {
                let pack =
                    |x: i32, y: i32| LPARAM((x as u16 as u32 | ((y as u16 as u32) << 16)) as isize);
                SendMessageW(hwnd, WM_LBUTTONDOWN, None, Some(pack(40, 40)));
                let end = pack(40 + wp.0 as i32, 40 + lp.0 as i32);
                SendMessageW(hwnd, WM_MOUSEMOVE, Some(WPARAM(1)), Some(end));
                SendMessageW(hwnd, WM_LBUTTONUP, None, Some(end));
                let _ = DwmFlush();
                println!("CAPTURE_FIXTURE=dragged");
                return LRESULT(0);
            }
            WM_MOUSEWHEEL | WM_MOUSEHWHEEL => {
                let delta = ((wp.0 >> 16) as u16 as i16 as i32) / 120 * 8;
                if message == WM_MOUSEWHEEL {
                    scene.y.set(scene.y.get() - delta);
                } else {
                    scene.x.set(scene.x.get() + delta);
                }
                let _ = InvalidateRect(Some(hwnd), None, false);
                let _ = UpdateWindow(hwnd);
                let _ = DwmFlush();
                println!("CAPTURE_FIXTURE=scrolled");
                return LRESULT(0);
            }
            WM_PAINT => {
                let mut paint = PAINTSTRUCT::default();
                let dc = BeginPaint(hwnd, &mut paint);
                let bytes = (0..128)
                    .flat_map(|y| {
                        (0..160).flat_map(move |x| pixel(scene.x.get() + x, scene.y.get() + y))
                    })
                    .collect::<Vec<_>>();
                let info = BITMAPINFO {
                    bmiHeader: BITMAPINFOHEADER {
                        biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                        biWidth: 160,
                        biHeight: -128,
                        biPlanes: 1,
                        biBitCount: 32,
                        biCompression: BI_RGB.0,
                        ..Default::default()
                    },
                    ..Default::default()
                };
                SetDIBitsToDevice(
                    dc,
                    0,
                    0,
                    160,
                    128,
                    0,
                    0,
                    0,
                    128,
                    bytes.as_ptr().cast(),
                    &info,
                    DIB_RGB_COLORS,
                );
                let _ = EndPaint(hwnd, &paint);
                return LRESULT(0);
            }
            value if value == WM_APP + 1 => {
                let mut rect = RECT::default();
                let _ = GetWindowRect(hwnd, &mut rect);
                let _ = SetWindowPos(
                    hwnd,
                    None,
                    rect.left + 1,
                    rect.top,
                    0,
                    0,
                    SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
                );
                println!("CAPTURE_FIXTURE=moved");
                return LRESULT(0);
            }
            WM_CLOSE => {
                let _ = DestroyWindow(hwnd);
                return LRESULT(0);
            }
            WM_DESTROY => {
                PostQuitMessage(0);
                return LRESULT(0);
            }
            _ => {}
        }
    }
    DefWindowProcW(hwnd, message, wp, lp)
}
#[test]
#[ignore = "owned child entry, invoked only by the explicit recording runtime gate"]
fn recording_window_child() {
    assert_eq!(std::env::var("OTTER_CAPTURE_CHILD").as_deref(), Ok("1"));
    unsafe {
        let _dpi = windows::Win32::UI::HiDpi::SetThreadDpiAwarenessContext(
            windows::Win32::UI::HiDpi::DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
        );
        let instance = windows::Win32::System::LibraryLoader::GetModuleHandleW(None).unwrap();
        let class = w!("Otter.Capture.OwnedScrollFixture");
        assert_ne!(
            RegisterClassW(&WNDCLASSW {
                lpfnWndProc: Some(procedure),
                hInstance: instance.into(),
                lpszClassName: class,
                ..Default::default()
            }),
            0
        );
        let x = GetSystemMetrics(SM_XVIRTUALSCREEN) + 40;
        let y = GetSystemMetrics(SM_YVIRTUALSCREEN) + 40;
        let window = crate::desktop::Window(
            CreateWindowExW(
                WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW | WS_EX_TOPMOST,
                class,
                w!("Otter owned scrolling fixture"),
                WS_POPUP,
                x,
                y,
                160,
                128,
                None,
                None,
                Some(instance.into()),
                None,
            )
            .unwrap(),
        );
        let scene = Scene {
            x: Cell::new(128),
            y: Cell::new(128),
            pointer: Cell::new(None),
        };
        SetWindowLongPtrW(window.0, GWLP_USERDATA, &scene as *const _ as isize);
        let _ = ShowWindow(window.0, SW_SHOWNOACTIVATE);
        let _ = UpdateWindow(window.0);
        DwmFlush().unwrap();
        println!("CAPTURE_FIXTURE={:x} {x} {y}", window.0 .0 as usize);
        let raw = window.0 .0 as usize;
        std::thread::spawn(move || {
            for line in std::io::stdin()
                .lock()
                .take(16 * 1024)
                .lines()
                .map_while(std::result::Result::ok)
            {
                let values = line.split_whitespace().collect::<Vec<_>>();
                let hwnd = HWND(raw as *mut _);
                let result = if values.first() == Some(&"move") {
                    PostMessageW(Some(hwnd), WM_APP + 1, WPARAM(0), LPARAM(0))
                } else if values.len() == 3 && values[0] == "drag" {
                    PostMessageW(
                        Some(hwnd),
                        WM_APP + 2,
                        WPARAM(values[1].parse::<i32>().unwrap() as usize),
                        LPARAM(values[2].parse::<i32>().unwrap() as isize),
                    )
                } else if values.len() == 2 {
                    let delta = values[1].parse::<i16>().unwrap();
                    PostMessageW(
                        Some(hwnd),
                        if values[0] == "h" {
                            WM_MOUSEHWHEEL
                        } else {
                            WM_MOUSEWHEEL
                        },
                        WPARAM((u32::from(delta as u16) << 16) as usize),
                        LPARAM(0),
                    )
                } else {
                    break;
                };
                if result.is_err() {
                    break;
                }
            }
            let _ = PostMessageW(Some(HWND(raw as *mut _)), WM_CLOSE, WPARAM(0), LPARAM(0));
        });
        let mut message = MSG::default();
        while GetMessageW(&mut message, None, 0, 0).0 > 0 {
            let _ = TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }
}
