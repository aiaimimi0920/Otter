//! Recording uses sparse non-focus-stealing arrow windows and click-through previews; original Input/SessionJob retain business ownership.
use super::*;
use windows::{core::w, Win32::Graphics::Gdi::*};
#[path = "long_capture_recording_keyboard.rs"]
mod keyboard;
const BACKING: windows::core::PCWSTR = w!("Otter.LongCapture.Preview");
struct OwnedRegion(HRGN);
impl Drop for OwnedRegion {
    fn drop(&mut self) {
        unsafe {
            let _ = DeleteObject(self.0.into());
        }
    }
}

pub(super) unsafe fn run(owner: HWND, input: &Input) -> Result<Option<Image>> {
    run_with(owner, input, |_| Ok(()))
}
pub(super) unsafe fn run_with(
    owner: HWND,
    input: &Input,
    ready: impl FnOnce(HWND) -> Result<()>,
) -> Result<Option<Image>> {
    input.modeless.set(true);
    let mut template = layout::template();
    // Use borderless/nonactivating styles at creation; WM_INITDIALOG failures also follow modeless lifecycles.
    template[0] = (WS_POPUP | WS_CLIPCHILDREN).0 | DS_SETFONT as u32;
    template[1] = (WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW | WS_EX_TOPMOST).0;
    let window = crate::desktop::Window(CreateDialogIndirectParamW(
        None,
        template.as_ptr().cast(),
        Some(owner),
        Some(procedure),
        LPARAM(input as *const _ as isize),
    )?);
    let hwnd = window.0;
    if !IsWindow(Some(hwnd)).as_bool() {
        return Err(Error::new(
            E_FAIL,
            input
                .error
                .borrow_mut()
                .take()
                .unwrap_or_else(|| crate::ui_text::text("long-capture-start-failed").into()),
        ));
    }
    for id in [1, 2, 101, 102, 103, 104, 105, 106, 107, 112] {
        if let Ok(control) = GetDlgItem(Some(hwnd), id) {
            let _ = ShowWindow(control, SW_HIDE);
        }
    }
    for (id, text) in [(108, "↑"), (109, "←"), (110, "→"), (111, "↓")] {
        SetDlgItemTextW(hwnd, id, &HSTRING::from(text))?;
        let control = GetDlgItem(Some(hwnd), id)?;
        SetWindowLongPtrW(
            control,
            GWL_STYLE,
            (WS_CHILD | WS_VISIBLE | WINDOW_STYLE(BS_OWNERDRAW as u32)).0 as isize,
        );
        neuro_mini_ui::prepare_button(control)?;
    }
    let backing = crate::desktop::Window(CreateWindowExW(
        WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW | WS_EX_TOPMOST,
        w!("STATIC"),
        &windows::core::HSTRING::from(crate::ui_text::text("long-capture-preview-title")),
        WS_POPUP,
        0,
        0,
        1,
        1,
        Some(owner),
        None,
        None,
        None,
    )?);
    SetWindowDisplayAffinity(backing.0, WDA_EXCLUDEFROMCAPTURE)?;
    SetPropW(hwnd, BACKING, Some(HANDLE(backing.0 .0)))?;
    let _keys = keyboard::Hook::install(hwnd)?;
    present(hwnd, input)?;
    recording::start(hwnd, input);
    ready(hwnd)?;
    let mut message = MSG::default();
    while IsWindow(Some(hwnd)).as_bool() {
        let result = GetMessageW(&mut message, None, 0, 0).0;
        if result < 0 {
            return Err(Error::from_win32());
        }
        if result == 0 {
            PostQuitMessage(message.wParam.0 as i32);
            break;
        }
        if !keyboard::dispatch(hwnd, input, &message) {
            let _ = TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }
    input.recording.cancel();
    input.job.borrow_mut().cancel();
    if let Some(error) = input.error.borrow_mut().take() {
        return Err(Error::new(E_FAIL, error));
    }
    Ok(input.output.borrow_mut().take())
}
pub(super) unsafe fn end(hwnd: HWND, input: &Input, result: isize) {
    if input.modeless.get() {
        let backing = HWND(GetPropW(hwnd, BACKING).0);
        if !backing.is_invalid() {
            let _ = ShowWindow(backing, SW_HIDE);
        }
        let _ = DestroyWindow(hwnd);
    } else {
        let _ = EndDialog(hwnd, result);
    }
}
pub(super) unsafe fn refresh(hwnd: HWND) {
    if GetPropW(hwnd, BACKING).is_invalid() {
        return;
    }
    let pointer = GetWindowLongPtrW(hwnd, WINDOW_LONG_PTR_INDEX(USER)) as *const Input;
    if let Some(input) = pointer.as_ref() {
        if let Err(error) = present(hwnd, input) {
            *input.error.borrow_mut() = Some(crate::ui_text::format(
                "long-capture-preview-update-failed",
                &[("error", error.to_string().into())],
            ));
            close(hwnd, input);
        }
    }
}
fn px(value: i32, dpi: u32) -> i32 {
    (i64::from(value) * i64::from(dpi) / 96) as i32
}
fn placement(region: Region, screen: RECT, dpi: u32, image: (u32, u32)) -> RECT {
    let gap = px(12, dpi);
    let right = screen.right - (region.left + region.width) - gap * 2;
    let left = region.left - screen.left - gap * 2;
    let available = if right >= px(80, dpi) {
        right
    } else if left >= px(80, dpi) {
        left
    } else {
        screen.right - screen.left - gap * 2
    };
    let width = px(240, dpi).min(available).max(1);
    let max_height = px(640, dpi)
        .min(screen.bottom - screen.top - gap * 2)
        .max(1);
    let image_height = (u64::from(image.1) * width.saturating_sub(px(40, dpi)).max(1) as u64
        / u64::from(image.0.max(1))) as i64;
    let height = (image_height + i64::from(px(100, dpi)))
        .min(i64::from(max_height))
        .max(1) as i32;
    let x = if right >= px(80, dpi) {
        region.left + region.width + gap
    } else if left >= px(80, dpi) {
        region.left - gap - width
    } else {
        screen.right - gap - width
    };
    let y = region.top.clamp(
        screen.top + gap,
        (screen.bottom - gap - height).max(screen.top + gap),
    );
    RECT {
        left: x,
        top: y,
        right: x + width,
        bottom: y + height,
    }
}
unsafe fn present(hwnd: HWND, input: &Input) -> Result<()> {
    let backing = HWND(GetPropW(hwnd, BACKING).0);
    if backing.is_invalid() {
        return Ok(());
    }
    let region = input.movement.region();
    let anchor = RECT {
        left: region.left,
        top: region.top,
        right: region.left + region.width,
        bottom: region.top + region.height,
    };
    let mut monitor = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    GetMonitorInfoW(
        MonitorFromRect(&anchor, MONITOR_DEFAULTTONEAREST),
        &mut monitor,
    )
    .ok()?;
    let dpi = windows::Win32::UI::HiDpi::GetDpiForWindow(hwnd).max(96);
    let image = input.preview.borrow();
    let dimensions = image
        .as_ref()
        .map_or((region.width as u32, region.height as u32), |i| {
            (i.width, i.height)
        });
    let rect = placement(region, monitor.rcWork, dpi, dimensions);
    let (width, height) = (rect.right - rect.left, rect.bottom - rect.top);
    let edge = px(20, dpi);
    let header = px(48, dpi);
    let bottom = (height - px(28, dpi)).max(header + edge);
    // Content/state updates reuse arrow geometry; rebuild hit regions only after dimension/DPI changes.
    if input.recording.overlay_layout.get() != Some((width, height, dpi)) {
        let buttons = [
            (
                108,
                RECT {
                    left: edge,
                    top: header,
                    right: width - edge,
                    bottom: header + edge,
                },
            ),
            (
                109,
                RECT {
                    left: 0,
                    top: header + edge,
                    right: edge,
                    bottom,
                },
            ),
            (
                110,
                RECT {
                    left: width - edge,
                    top: header + edge,
                    right: width,
                    bottom,
                },
            ),
            (
                111,
                RECT {
                    left: edge,
                    top: bottom,
                    right: width - edge,
                    bottom: bottom + edge,
                },
            ),
        ];
        let combined = OwnedRegion(CreateRectRgn(0, 0, 0, 0));
        if combined.0.is_invalid() {
            return Err(Error::from_win32());
        }
        for (id, r) in buttons {
            let part = OwnedRegion(CreateRectRgn(r.left, r.top, r.right, r.bottom));
            if part.0.is_invalid()
                || CombineRgn(Some(combined.0), Some(combined.0), Some(part.0), RGN_OR) == RGN_ERROR
            {
                return Err(Error::from_win32());
            }
            let control = GetDlgItem(Some(hwnd), id)?;
            SetWindowPos(
                control,
                None,
                r.left,
                r.top,
                r.right - r.left,
                r.bottom - r.top,
                SWP_NOACTIVATE | SWP_NOZORDER,
            )?;
        }
        if SetWindowRgn(hwnd, Some(combined.0), false) == 0 {
            return Err(Error::from_win32());
        }
        std::mem::forget(combined); // The window owns the region on success; RAII still releases error paths.
        input
            .recording
            .overlay_layout
            .set(Some((width, height, dpi)));
    }
    let bitmap =
        crate::software_dib::Bitmap::new(windows::Win32::Graphics::Direct2D::Common::D2D_SIZE_U {
            width: width as u32,
            height: height as u32,
        })?;
    let dc = bitmap.dc();
    let palette = neuro_mini_ui::Palette::default();
    let color = |rgb: u32| COLORREF(((rgb & 255) << 16) | (rgb & 0xff00) | ((rgb >> 16) & 255));
    SetDCBrushColor(dc, color(palette.surface));
    FillRect(
        dc,
        &RECT {
            left: 0,
            top: 0,
            right: width,
            bottom: height,
        },
        HBRUSH(GetStockObject(DC_BRUSH).0),
    );
    let preview_rect = RECT {
        left: edge,
        top: header + edge,
        right: width - edge,
        bottom,
    };
    SetDCBrushColor(dc, color(palette.control));
    preview::paint_on(
        &windows::Win32::UI::Controls::DRAWITEMSTRUCT {
            hDC: dc,
            rcItem: preview_rect,
            ..Default::default()
        },
        image.as_ref(),
        HBRUSH(GetStockObject(DC_BRUSH).0),
    );
    SetBkMode(dc, TRANSPARENT);
    SetTextColor(dc, color(palette.text));
    let font = SendMessageW(hwnd, WM_GETFONT, None, None);
    let old = SelectObject(dc, HGDIOBJ(font.0 as *mut _));
    let mut text: Vec<u16> = crate::ui_text::text("long-capture-preview-help")
        .encode_utf16()
        .collect();
    DrawTextW(
        dc,
        &mut text,
        &mut RECT {
            left: 4,
            top: 3,
            right: width - 4,
            bottom: px(22, dpi),
        },
        DT_CENTER | DT_SINGLELINE | DT_VCENTER | DT_NOPREFIX,
    );
    let mut status = [0u16; 256];
    let length = GetDlgItemTextW(hwnd, 103, &mut status) as usize;
    DrawTextW(
        dc,
        &mut status[..length],
        &mut RECT {
            left: 4,
            top: px(23, dpi),
            right: width - 4,
            bottom: header,
        },
        DT_CENTER | DT_SINGLELINE | DT_VCENTER | DT_END_ELLIPSIS | DT_NOPREFIX,
    );
    SelectObject(dc, old);
    SetDCBrushColor(dc, color(palette.signal));
    FrameRect(
        dc,
        &RECT {
            left: 0,
            top: 0,
            right: width,
            bottom: height,
        },
        HBRUSH(GetStockObject(DC_BRUSH).0),
    );
    UpdateLayeredWindow(
        backing,
        None,
        Some(&POINT {
            x: rect.left,
            y: rect.top,
        }),
        Some(&SIZE {
            cx: width,
            cy: height,
        }),
        Some(dc),
        Some(&POINT::default()),
        COLORREF(0),
        Some(&BLENDFUNCTION {
            BlendOp: AC_SRC_OVER as u8,
            SourceConstantAlpha: 255,
            ..Default::default()
        }),
        ULW_ALPHA,
    )?;
    SetWindowPos(
        backing,
        Some(HWND_TOPMOST),
        0,
        0,
        0,
        0,
        SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_SHOWWINDOW,
    )?;
    SetWindowPos(
        hwnd,
        Some(HWND_TOPMOST),
        rect.left,
        rect.top,
        width,
        height,
        SWP_NOACTIVATE | SWP_SHOWWINDOW,
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows::Win32::System::Threading::{GetCurrentProcess, GetGuiResources, GR_GDIOBJECTS};

    #[test]
    fn recording_dialog_uses_no_erase_and_a_dark_arrow_fallback() {
        unsafe {
            let input = super::super::tests::input();
            let dialog = super::super::tests::hidden(&input);
            input.modeless.set(true);
            let arrow = GetDlgItem(Some(dialog.0), 109).unwrap();
            let bitmap = crate::software_dib::Bitmap::new(
                windows::Win32::Graphics::Direct2D::Common::D2D_SIZE_U {
                    width: 32,
                    height: 32,
                },
            )
            .unwrap();
            let dc = bitmap.dc();
            let sentinel = COLORREF(0x123456);
            assert_eq!(SetPixel(dc, 3, 3, sentinel), sentinel);
            let wp = Some(WPARAM(dc.0 as usize));
            assert_eq!(SendMessageW(dialog.0, WM_ERASEBKGND, wp, None).0, 1);
            assert_eq!(GetPixel(dc, 3, 3), sentinel);

            let brush = SendMessageW(dialog.0, WM_CTLCOLORBTN, wp, Some(LPARAM(arrow.0 as isize)));
            assert_eq!(brush.0, GetStockObject(DC_BRUSH).0 as isize);
            assert_eq!(GetDCBrushColor(dc), COLORREF(0x201711));
        }
    }

    #[test]
    fn failed_arrow_layout_releases_regions_and_does_not_commit_geometry() {
        unsafe {
            let input = super::super::tests::input();
            let dialog = super::super::tests::hidden(&input);
            let backing = crate::desktop::Window(
                CreateWindowExW(
                    WS_EX_LAYERED,
                    w!("STATIC"),
                    w!("preview failure fixture"),
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
            );
            SetPropW(dialog.0, BACKING, Some(HANDLE(backing.0 .0))).unwrap();
            // The fourth control is missing; the combined region and first three control positions already exist.
            DestroyWindow(GetDlgItem(Some(dialog.0), 111).unwrap()).unwrap();
            // Initial positioning may initialize Win32 resources; run the same failure path once before measuring repeated failures.
            assert!(present(dialog.0, &input).is_err());
            assert!(input.recording.overlay_layout.get().is_none());
            let before = GetGuiResources(GetCurrentProcess(), GR_GDIOBJECTS);
            for _ in 0..8 {
                assert!(present(dialog.0, &input).is_err());
                assert!(input.recording.overlay_layout.get().is_none());
                assert_eq!(GetGuiResources(GetCurrentProcess(), GR_GDIOBJECTS), before);
            }
        }
    }
}
