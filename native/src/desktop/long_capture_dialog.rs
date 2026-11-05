//! 会话期手动长截图控制窗口；计时器只等待在途任务，无空闲采集或输入注入。
use crate::desktop::{
    image::Image,
    long_capture_source::{self, Region},
};
use crate::long_capture::{Append, Axis, Completion, MatchOptions, SessionJob};
use std::cell::{Cell, RefCell};
use windows::{
    core::{Error, Result, HSTRING},
    Win32::{
        Foundation::*,
        UI::{Input::KeyboardAndMouse::EnableWindow, WindowsAndMessaging::*},
    },
};
#[path = "long_capture_auto.rs"]
mod automatic;
#[path = "long_capture_dialog_finish.rs"]
mod finish;
#[path = "long_capture_dialog_layout.rs"]
mod layout;
#[path = "long_capture_dialog_movement.rs"]
mod movement;
#[path = "long_capture_outline.rs"]
mod outline;
#[path = "long_capture_dialog_preview.rs"]
mod preview;
#[path = "long_capture_dialog_controls.rs"]
mod ui;
use ui::{clear_preview, controls, frame_status, request_auto, status, waiting};
#[path = "long_capture_recording.rs"]
mod recording;
const USER: i32 = (2 * std::mem::size_of::<isize>()) as i32;
const TIMER: usize = 0x4f65;
struct Input {
    finish: finish::State,
    outline: RefCell<Option<outline::Outline>>,
    movement: movement::State,
    axis: Axis,
    forward: Cell<bool>,
    automatic: RefCell<automatic::Auto>,
    recording: recording::State,
    job: RefCell<SessionJob>,
    output: RefCell<Option<Image>>,
    preview: RefCell<Option<Image>>,
    error: RefCell<Option<String>>,
    protect: bool,
}
pub(super) struct Output {
    pub image: Image,
    pub axis: Option<Axis>,
    pub direction: Option<crate::long_capture::Direction>,
    pub spatial_position: Option<(i64, i64)>,
    pub frame_count: usize,
    pub source_region: Region,
}
impl Input {
    fn new(
        job: SessionJob,
        region: Region,
        axis: Axis,
        capture: &std::rc::Rc<Cell<bool>>,
        target: Option<long_capture_source::scroll::Target>,
    ) -> Self {
        Self {
            finish: finish::State::new(capture.clone()),
            outline: RefCell::new(None),
            movement: movement::State::new(region),
            axis,
            forward: Cell::new(true),
            automatic: RefCell::new(Default::default()),
            recording: recording::State::new(target),
            job: RefCell::new(job),
            output: RefCell::new(None),
            preview: RefCell::new(None),
            error: RefCell::new(None),
            protect: true,
        }
    }
}
pub(super) unsafe fn show_recording(
    owner: HWND,
    image: Image,
    origin: POINT,
    capture_owner: &std::rc::Rc<Cell<bool>>,
) -> Result<Option<Output>> {
    let region = Region {
        left: origin.x,
        top: origin.y,
        width: image.width as i32,
        height: image.height as i32,
    };
    let (job, target) = long_capture_source::start_recording(region, image.pixels)
        .map_err(|error| Error::new(E_FAIL, error))?;
    let input = Input::new(job, region, Axis::Vertical, capture_owner, Some(target));
    let image = run(owner, &input)?;
    let snapshot = input.job.borrow().snapshot();
    Ok(image.map(|image| Output {
        image,
        axis: snapshot.axis,
        direction: snapshot.direction,
        spatial_position: snapshot.spatial_position,
        frame_count: snapshot.frame_count,
        source_region: region,
    }))
}
pub(super) unsafe fn show(
    owner: HWND,
    image: Image,
    origin: POINT,
    axis: Axis,
    capture_owner: &std::rc::Rc<Cell<bool>>,
) -> Result<Option<Image>> {
    let length = match axis {
        Axis::Vertical => image.height,
        Axis::Horizontal => image.width,
    };
    let options = MatchOptions {
        minimum_overlap: (length / 4).clamp(1, 64),
        maximum_shift: length.saturating_sub(1),
        maximum_mean_error: 3.0,
    };
    let region = Region {
        left: origin.x,
        top: origin.y,
        width: image.width as i32,
        height: image.height as i32,
    };
    let job = long_capture_source::start_scrollable(region, image.pixels.clone(), axis, options)
        .or_else(|_| long_capture_source::start(region, image.pixels, axis, options))
        .map_err(|error| Error::new(E_FAIL, error))?;
    let input = Input::new(job, region, axis, capture_owner, None);
    run(owner, &input)
}
unsafe fn run(owner: HWND, input: &Input) -> Result<Option<Image>> {
    let template = layout::template();
    let result = DialogBoxIndirectParamW(
        None,
        template.as_ptr().cast(),
        Some(owner),
        Some(procedure),
        LPARAM(input as *const _ as isize),
    );
    input.recording.cancel();
    input.job.borrow_mut().cancel();
    if result == -1 {
        return Err(Error::from_win32());
    }
    if let Some(error) = input.error.borrow_mut().take() {
        return Err(Error::new(E_FAIL, error));
    }
    Ok(input.output.borrow_mut().take())
}
unsafe fn close(hwnd: HWND, input: &Input) {
    input.recording.cancel();
    input.finish.close();
    input.outline.borrow_mut().take();
    input.automatic.borrow_mut().stop();
    input.job.borrow_mut().cancel();
    let _ = KillTimer(Some(hwnd), TIMER);
    let _ = EndDialog(hwnd, 0);
}
unsafe fn handle(hwnd: HWND, message: u32, wp: WPARAM, lp: LPARAM) -> isize {
    if message == WM_INITDIALOG {
        SetWindowLongPtrW(hwnd, WINDOW_LONG_PTR_INDEX(USER), lp.0);
    }
    let pointer = GetWindowLongPtrW(hwnd, WINDOW_LONG_PTR_INDEX(USER)) as *const Input;
    let Some(input) = pointer.as_ref() else {
        return 0;
    };
    match message {
        WM_INITDIALOG => {
            if let Err(error) = input.finish.install(hwnd) {
                *input.error.borrow_mut() = Some(format!("无法建立长截图结束路由：{error}"));
                close(hwnd, input);
                return 1;
            }
            controls(hwnd, input, false);
            if input.protect {
                if let Err(error) = SetWindowDisplayAffinity(hwnd, WDA_EXCLUDEFROMCAPTURE) {
                    *input.error.borrow_mut() = Some(format!("无法排除长截图控制窗口：{error}"));
                    close(hwnd, input);
                    return 1;
                }
                match outline::Outline::new(hwnd, input.movement.region(), true, true) {
                    Ok(outline) => *input.outline.borrow_mut() = Some(outline),
                    Err(error) => {
                        *input.error.borrow_mut() = Some(format!("无法建立取景框边框：{error}"));
                        close(hwnd, input);
                        return 1;
                    }
                }
            }
            if input.recording.enabled() {
                recording::start(hwnd, input);
            }
            1
        }
        WM_DISPLAYCHANGE | WM_DPICHANGED if input.recording.enabled() => {
            input.recording.stop();
            let busy = input.job.borrow().is_pending();
            controls(hwnd, input, busy);
            status(
                hwnd,
                "屏幕布局或 DPI 已变化，录制已停止；可完成已有内容或取消",
            );
            recording::drive(hwnd, input);
            1
        }
        WM_CLOSE | WM_DISPLAYCHANGE | WM_DPICHANGED => {
            if message != WM_CLOSE {
                *input.error.borrow_mut() = Some("屏幕布局已改变，请重新选择截图区域。".into());
            }
            close(hwnd, input);
            1
        }
        WM_DESTROY => {
            input.recording.cancel();
            input.finish.close();
            input.outline.borrow_mut().take();
            input.automatic.borrow_mut().stop();
            input.job.borrow_mut().cancel();
            let _ = KillTimer(Some(hwnd), TIMER);
            SetWindowLongPtrW(hwnd, WINDOW_LONG_PTR_INDEX(USER), 0);
            0
        }
        super::finish_route::MESSAGE => {
            if input.finish.accepts(hwnd, wp, lp) {
                finish::request(hwnd, input);
            }
            1
        }
        recording::MESSAGE => {
            recording::notify(hwnd, input, wp);
            1
        }
        WM_COMMAND => {
            if input.finish.pending() && !matches!(wp.0 & 0xffff, 1 | 2) {
                return 1;
            }
            match wp.0 & 0xffff {
                1 => finish::request(hwnd, input),
                112 => recording::command(hwnd, input),
                command @ 108..=111 => movement::dispatch(hwnd, input, command),
                2 => close(hwnd, input),
                107 => {
                    // 禁用控件不是命令准入边界：拒绝任务在途或自动循环期间的迟到消息。
                    let allowed = {
                        let job = input.job.borrow();
                        job.can_scroll() && !job.is_pending() && !input.automatic.borrow().running()
                    };
                    if allowed {
                        input.forward.set(!input.forward.get());
                        controls(hwnd, input, false);
                    }
                }
                106 => {
                    if input.automatic.borrow().running() {
                        input.automatic.borrow_mut().stop();
                        let pending = input.job.borrow().is_pending();
                        controls(hwnd, input, pending);
                        status(
                            hwnd,
                            if pending {
                                "已停止后续滚动，等待当前任务结束"
                            } else {
                                "自动采集已停止"
                            },
                        );
                    } else {
                        let allowed = {
                            let job = input.job.borrow();
                            job.can_scroll() && !job.is_pending()
                        };
                        if allowed {
                            input
                                .automatic
                                .borrow_mut()
                                .start(std::time::Instant::now());
                            request_auto(hwnd, input);
                        }
                    }
                }
                command @ (101 | 102 | 105) => {
                    if input.automatic.borrow().running()
                        || (input.recording.running() && command != 102)
                    {
                        return 1;
                    }
                    let dispatched = {
                        let mut job = input.job.borrow_mut();
                        if command == 105 {
                            job.scroll_capture(input.forward.get())
                        } else if command == 102 {
                            job.preview()
                        } else {
                            job.capture()
                        }
                    };
                    if dispatched.is_ok() {
                        waiting(hwnd, input);
                    }
                }
                _ => return 0,
            }
            1
        }
        WM_TIMER if wp.0 == TIMER => {
            let reply = input.job.borrow_mut().take_ready();
            if let Some(reply) = reply {
                let _ = KillTimer(Some(hwnd), TIMER);
                recording::completed(input, &reply.completion);
                let compose_after = finish::completed(input, &reply.completion);
                let moved = input.movement.completed(&reply.completion);
                if moved {
                    // 显式移动转入手动空间会话，不能用原目标重新授权新区域。
                    input.recording.revoke();
                    let positioned = input
                        .outline
                        .borrow()
                        .as_ref()
                        .map_or(Ok(()), |outline| outline.position(input.movement.region()));
                    if let Err(error) = positioned {
                        *input.error.borrow_mut() = Some(format!("无法更新取景框边框：{error}"));
                        close(hwnd, input);
                        return 1;
                    }
                }
                let mut next = input
                    .automatic
                    .borrow_mut()
                    .completed(&reply.completion, reply.snapshot.confirmed_edges.is_some());
                if moved && next == automatic::Next::Stop && !input.finish.pending() {
                    next = automatic::Next::Preview;
                }
                controls(hwnd, input, false);
                match reply.completion {
                    Completion::Preview(Ok(crate::long_capture::Composite {
                        width,
                        height,
                        pixels,
                        ..
                    }))
                    | Completion::SpatialPreview(Ok(crate::long_capture::SpatialComposite {
                        width,
                        height,
                        pixels,
                        ..
                    })) => {
                        let prepared = preview::prepare(Image {
                            width,
                            height,
                            pixels,
                        });
                        match prepared {
                            Ok(image) => *input.preview.borrow_mut() = Some(image),
                            Err(error) => {
                                input.finish.retry();
                                input.automatic.borrow_mut().stop();
                                input.recording.stop();
                                controls(hwnd, input, false);
                                status(hwnd, &format!("无法准备预览：{error:?}"));
                                return 1;
                            }
                        }
                        if let Ok(control) = GetDlgItem(Some(hwnd), 104) {
                            let _ = windows::Win32::Graphics::Gdi::InvalidateRect(
                                Some(control),
                                None,
                                true,
                            );
                        }
                        frame_status(hwnd, input, reply.snapshot.frame_count);
                    }
                    Completion::Compose(Ok(crate::long_capture::Composite {
                        width,
                        height,
                        pixels,
                        ..
                    }))
                    | Completion::SpatialCompose(Ok(crate::long_capture::SpatialComposite {
                        width,
                        height,
                        pixels,
                        ..
                    })) => {
                        *input.output.borrow_mut() = Some(Image {
                            width,
                            height,
                            pixels,
                        });
                        input.finish.close();
                        let _ = EndDialog(hwnd, 1);
                    }
                    Completion::Append(Ok(Append::Added { .. }))
                    | Completion::SpatialAppend(Ok(crate::long_capture::SpatialAppend::Added {
                        ..
                    })) => {
                        if !input.recording.enabled() {
                            clear_preview(hwnd, input);
                        }
                        frame_status(hwnd, input, reply.snapshot.frame_count)
                    }
                    Completion::Append(Ok(Append::Duplicate))
                    | Completion::SpatialAppend(Ok(
                        crate::long_capture::SpatialAppend::Duplicate,
                    )) => status(
                        hwnd,
                        if input.recording.enabled() {
                            "等待目标页面滚动，重复画面已忽略"
                        } else {
                            "画面未变化"
                        },
                    ),
                    Completion::Append(Ok(Append::Rejected(_)))
                    | Completion::SpatialAppend(Ok(
                        crate::long_capture::SpatialAppend::Rejected(_),
                    )) => status(
                        hwnd,
                        if input.recording.enabled() && !input.recording.running() {
                            "未识别到可靠重叠，录制已停止；已接受内容保留"
                        } else {
                            "未识别到可靠重叠，请缩小滚动距离后重试"
                        },
                    ),
                    Completion::Append(Err(error))
                    | Completion::SpatialAppend(Err(error))
                    | Completion::SpatialCompose(Err(error))
                    | Completion::SpatialPreview(Err(error))
                    | Completion::Compose(Err(error))
                    | Completion::Preview(Err(error)) => match error {
                        crate::long_capture::Error::UnconfirmedEdges => {
                            status(hwnd, "请继续滚动并采集一帧，以确认固定边缘")
                        }
                        crate::long_capture::Error::ChangedEdges => {
                            status(hwnd, "固定边缘已变化，请取消并重新选择区域")
                        }
                        crate::long_capture::Error::CrossAxis if input.recording.enabled() => {
                            status(hwnd, "滚动轴已变化，录制已停止；可完成已有内容或取消")
                        }
                        other => status(hwnd, &format!("无法完成：{other:?}")),
                    },
                    Completion::CaptureFailed(error) => status(hwnd, &format!("采集失败：{error}")),
                    Completion::WorkerStopped => {
                        *input.error.borrow_mut() = Some("长截图后台任务已停止。".into());
                        close(hwnd, input);
                    }
                }
                if compose_after {
                    finish::compose(hwnd, input);
                    return 1;
                }
                match next {
                    automatic::Next::Scroll => request_auto(hwnd, input),
                    automatic::Next::Preview => {
                        let result = input.job.borrow_mut().preview();
                        if result.is_ok() {
                            waiting(hwnd, input);
                        } else {
                            input.automatic.borrow_mut().stop();
                            controls(hwnd, input, false);
                            status(hwnd, "无法生成自动采集预览，已停止后续滚动");
                        }
                    }
                    automatic::Next::Stop => {}
                }
            }
            recording::drive(hwnd, input);
            1
        }
        WM_DRAWITEM if wp.0 == 104 && lp.0 != 0 => {
            let item = &*(lp.0 as *const windows::Win32::UI::Controls::DRAWITEMSTRUCT);
            preview::paint(item, input.preview.borrow().as_ref());
            1
        }
        _ => 0,
    }
}
unsafe extern "system" fn procedure(hwnd: HWND, message: u32, wp: WPARAM, lp: LPARAM) -> isize {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        handle(hwnd, message, wp, lp)
    }))
    .unwrap_or_else(|_| {
        let _ = EndDialog(hwnd, 0);
        1
    })
}
#[cfg(test)]
#[path = "long_capture_dialog_finish_tests.rs"]
mod finish_tests;
#[cfg(test)]
#[path = "long_capture_dialog_modal_tests.rs"]
mod modal_tests;
#[cfg(test)]
#[path = "long_capture_dialog_tests.rs"]
mod tests;
#[cfg(test)]
#[path = "long_capture_dialog_timer_tests.rs"]
mod timer_tests;
