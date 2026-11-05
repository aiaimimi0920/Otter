//! 区域选择后的最终捕获；冻结 SDR 图只作选区参考，不冒充最终 HDR 帧。
use super::*;
pub(super) enum Delivery {
    Recording(super::long_capture_dialog::Output),
    Image(Image, Option<POINT>),
    Long(Image, POINT, crate::long_capture::Axis, (u32, u32)),
    Prepared(Box<crate::capture_job::PreparedCapture>, POINT),
}
pub(super) fn recording_context(
    output: &super::long_capture_dialog::Output,
) -> Result<(POINT, crate::capture_job::Context)> {
    let origin = POINT {
        x: output.source_region.left,
        y: output.source_region.top,
    };
    let width = u32::try_from(output.source_region.width)
        .map_err(|_| Error::new(E_INVALIDARG, "Invalid recorded capture width"))?;
    let height = u32::try_from(output.source_region.height)
        .map_err(|_| Error::new(E_INVALIDARG, "Invalid recorded capture height"))?;
    screen(origin, width, height)?;
    let mut context = crate::capture_job::Context::region(Some(origin), width, height, "gdi");
    context.kind = crate::session_capture::Kind::Long;
    context.axis = output.axis.map(|axis| match axis {
        crate::long_capture::Axis::Vertical => crate::session_capture::Axis::Vertical,
        crate::long_capture::Axis::Horizontal => crate::session_capture::Axis::Horizontal,
    });
    context.recording = Some(crate::session_capture::Recording {
        frame_count: output.frame_count,
        direction: output.direction.map(|direction| match direction {
            crate::long_capture::Direction::Up => crate::session_capture::Direction::Up,
            crate::long_capture::Direction::Down => crate::session_capture::Direction::Down,
            crate::long_capture::Direction::Left => crate::session_capture::Direction::Left,
            crate::long_capture::Direction::Right => crate::session_capture::Direction::Right,
        }),
        spatial_position: output.spatial_position.map(|(x, y)| [x, y]),
    });
    Ok((origin, context))
}
fn screen(origin: POINT, width: u32, height: u32) -> Result<RECT> {
    crate::viewport::checked_image_bytes(width, height)
        .ok_or_else(|| Error::new(E_INVALIDARG, "Invalid capture region dimensions"))?;
    let right = i32::try_from(i64::from(origin.x) + i64::from(width))
        .map_err(|_| Error::new(E_INVALIDARG, "Capture region overflows desktop coordinates"))?;
    let bottom = i32::try_from(i64::from(origin.y) + i64::from(height))
        .map_err(|_| Error::new(E_INVALIDARG, "Capture region overflows desktop coordinates"))?;
    Ok(RECT {
        left: origin.x,
        top: origin.y,
        right,
        bottom,
    })
}
pub(super) unsafe fn prepare(
    hwnd: HWND,
    state: &State,
    image: Image,
    origin: POINT,
) -> Result<Option<Delivery>> {
    let screen = screen(origin, image.width, image.height)?;
    // 释放冻结预览再启动 WGC，避免把整屏选择缓冲和新帧池同时留在预算中。
    drop(image);
    let mode = std::env::var("OTTER_CAPTURE_DYNAMIC_RANGE").ok();
    let root = std::env::var_os("OTTER_NATIVE_PROFILE_DIR");
    let local = std::env::var_os("LOCALAPPDATA");
    let profile = crate::settings_profile::select(None, root.as_deref(), local.as_deref(), false)
        .map_err(|error| Error::new(E_FAIL, error))?
        .and_then(|selection| selection.path.parent().map(|path| path.to_owned()))
        .ok_or_else(|| Error::new(E_FAIL, "Missing native capture profile"))?;
    // 选择窗口已销毁；等待其 DWM 变化提交后捕获最终桌面，宿主仍由外层 Hidden 隐藏。
    DwmFlush()?;
    super::super::capture::wait_cached(
        hwnd,
        super::super::capture::CachedRequest {
            screen,
            mode: crate::capture_dynamic::Mode::parse(mode.as_deref()),
            standard_region: true,
            gain: 1.0,
            profile,
            stem: "Otter_hdr_capture".into(),
            policy: state.cache_policy.get(),
        },
    )
    .map(|result| result.map(|prepared| Delivery::Prepared(Box::new(prepared), origin)))
    .map_err(|error| Error::new(E_FAIL, error))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn recording_delivery_retains_unknown_axis_and_source_rectangle() {
        unsafe {
            let _apartment = crate::desktop::Apartment::new().unwrap();
            let state = State::new(None, true).unwrap();
            let window = window();
            run_delivery(window.0, &state, true, || {
                Ok(Some(Delivery::Recording(
                    super::super::long_capture_dialog::Output {
                        image: Image {
                            width: 1,
                            height: 1,
                            pixels: vec![255; 4].into(),
                        },
                        axis: None,
                        direction: None,
                        spatial_position: None,
                        frame_count: 1,
                        source_region: crate::desktop::long_capture_source::Region {
                            left: -30,
                            top: 10,
                            width: 1,
                            height: 1,
                        },
                    },
                )))
            })
            .unwrap();
            super::super::super::open_job::finish(window.0, &state, true);
            let documents = state.new_pin.borrow_mut().take().unwrap();
            let metadata = documents[0].capture_source.as_ref().unwrap().metadata();
            assert_eq!(metadata["otterNativeRecording"]["frameCount"], 1);
            assert!(metadata.get("scrollAxis").is_none());
            assert_eq!(metadata["sourceRect"]["x"], -30.0);
        }
    }
    fn prepared() -> crate::capture_job::PreparedCapture {
        crate::capture_job::PreparedCapture::prepare(
            crate::capture_dynamic::Outcome {
                frame: crate::capture_dynamic::Frame::Sdr(crate::capture_color::SdrFrame {
                    width: 1,
                    height: 1,
                    pixels: vec![188, 188, 188, 255].into(),
                }),
                reason: Some(crate::capture_dynamic::Reason::ExplicitSdr),
                downgraded: false,
                display: None,
                diagnostic: None,
            },
            std::path::Path::new("C:/unused-sdr-capture-profile"),
            "fixture",
            Default::default(),
            &std::sync::atomic::AtomicBool::new(false),
        )
        .unwrap()
    }
    unsafe fn window() -> crate::desktop::Window {
        crate::desktop::Window(
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                w!("STATIC"),
                w!("capture delivery fixture"),
                WS_POPUP,
                0,
                0,
                1,
                1,
                None,
                None,
                None,
                None,
            )
            .unwrap(),
        )
    }
    #[test]
    fn prepared_region_delivery_opens_new_pin_with_source_and_origin() {
        unsafe {
            let _apartment = crate::desktop::Apartment::new().unwrap();
            let state = State::new(None, true).unwrap();
            let window = window();
            run_delivery(window.0, &state, true, || {
                Ok(Some(Delivery::Prepared(
                    Box::new(prepared()),
                    POINT { x: -30, y: 10 },
                )))
            })
            .unwrap();
            super::super::super::open_job::finish(window.0, &state, true);
            let documents = state.new_pin.borrow_mut().take().unwrap();
            assert_eq!(documents.len(), 1);
            assert_eq!(documents[0].capture_origin.unwrap().x, -30);
            assert_eq!(documents[0].image.pixels[0], 188);
            assert_eq!(
                documents[0].capture_source.as_ref().unwrap().reason,
                Some(crate::capture_dynamic::Reason::ExplicitSdr)
            );
        }
    }
    #[test]
    fn long_delivery_preserves_initial_viewport_not_stitched_output_dimensions() {
        unsafe {
            let _apartment = crate::desktop::Apartment::new().unwrap();
            let state = State::new(None, true).unwrap();
            let window = window();
            run_delivery(window.0, &state, true, || {
                Ok(Some(Delivery::Long(
                    Image {
                        width: 3,
                        height: 1,
                        pixels: vec![255; 12].into(),
                    },
                    POINT { x: -30, y: 10 },
                    crate::long_capture::Axis::Horizontal,
                    (1, 1),
                )))
            })
            .unwrap();
            super::super::super::open_job::finish(window.0, &state, true);
            let documents = state.new_pin.borrow_mut().take().unwrap();
            let metadata = documents[0].capture_source.as_ref().unwrap().metadata();
            assert_eq!(documents[0].image.width, 3);
            assert_eq!(metadata["sourceRect"]["w"], 1.0);
            assert_eq!(metadata["sourceRect"]["x"], -30.0);
            assert_eq!(metadata["kind"], "long");
            assert_eq!(metadata["scrollAxis"], "horizontal");
            assert_eq!(metadata["captureBackend"], "gdi");
        }
    }
    #[test]
    fn recorded_delivery_preserves_unknown_axis_and_rejects_invalid_metadata_before_worker() {
        unsafe {
            let _apartment = crate::desktop::Apartment::new().unwrap();
            let state = State::new(None, true).unwrap();
            let window = window();
            let origin = POINT { x: -30, y: 10 };
            let context = |count| {
                let mut value = crate::capture_job::Context::region(Some(origin), 1, 1, "gdi");
                value.kind = crate::session_capture::Kind::Long;
                value.recording = Some(crate::session_capture::Recording {
                    frame_count: count,
                    direction: None,
                    spatial_position: None,
                });
                value
            };
            let image = || Image {
                width: 1,
                height: 1,
                pixels: vec![255; 4].into(),
            };
            assert!(super::super::super::open_job::start_long_context(
                window.0,
                &state,
                image(),
                origin,
                context(0)
            )
            .is_err());
            assert!(state.open_job.borrow().is_none());
            assert!(state.new_pin.borrow().is_none());
            super::super::super::open_job::start_long_context(
                window.0,
                &state,
                image(),
                origin,
                context(1),
            )
            .unwrap();
            super::super::super::open_job::finish(window.0, &state, true);
            let documents = state.new_pin.borrow_mut().take().unwrap();
            let metadata = documents[0].capture_source.as_ref().unwrap().metadata();
            assert_eq!(metadata["otterNativeRecording"]["frameCount"], 1);
            assert!(metadata.get("scrollAxis").is_none());
            assert_eq!(metadata["sourceRect"]["x"], -30.0);
        }
    }
    #[test]
    fn document_change_during_wait_rejects_prepared_delivery_before_opening() {
        unsafe {
            let _apartment = crate::desktop::Apartment::new().unwrap();
            let state = State::new(None, true).unwrap();
            let window = window();
            let result = run_delivery(window.0, &state, false, || {
                state.canvas.borrow_mut().editor = crate::editor::Editor::default();
                Ok(Some(Delivery::Prepared(
                    Box::new(prepared()),
                    POINT::default(),
                )))
            });
            assert!(result.is_err());
            assert!(state.open_job.borrow().is_none());
            assert!(state.new_pin.borrow().is_none());
        }
    }
    #[test]
    fn selected_rectangle_keeps_physical_negative_coordinates_and_rejects_overflow() {
        assert_eq!(
            screen(POINT { x: -1920, y: -200 }, 300, 400).unwrap(),
            RECT {
                left: -1920,
                top: -200,
                right: -1620,
                bottom: 200
            }
        );
        assert!(screen(POINT { x: i32::MAX, y: 0 }, 1, 1).is_err());
        assert!(screen(POINT { x: 0, y: i32::MAX }, 1, 1).is_err());
        assert!(screen(POINT::default(), 0, 1).is_err());
    }
}
