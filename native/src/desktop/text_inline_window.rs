//! Annotation-specific captionless input layer, attached to pin coordinates above DComp/layered images; not a standalone dialog.
use super::*;
use windows::Win32::UI::Controls::*;
const MARKER: PCWSTR = w!("Otter.Annotation.Inline.Owner");
pub(super) const EPOCH: PCWSTR = w!("Otter.Annotation.Inline.Epoch");
pub(super) struct Window {
    hwnd: HWND,
    edit: HWND,
    marker: Box<u8>,
    font: HFONT,
    epoch: usize,
    backing: Option<desktop::Window>,
    keyed: bool,
    inset: Option<i32>,
    radius_px: i32,
    view: Option<Rc<boxed::View>>,
}
impl Window {
    pub(super) unsafe fn new(
        owner: HWND,
        initial: &str,
        font: &crate::text_annotation::TextFont,
        style: crate::stroke_style::StrokeStyle,
        scale: f32,
        geometry: Option<crate::text_annotation::TextBox>,
    ) -> Result<Self> {
        let limit = crate::text_annotation::MAX_TEXT_UNITS;
        if initial.contains('\0') || initial.encode_utf16().count() > limit {
            return Err(Error::new(
                E_INVALIDARG,
                "Text cannot be represented by EDIT; original retained",
            ));
        }
        let class = w!("Otter.Native.Annotation.Inline");
        static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(1);
        let epoch = NEXT
            .fetch_update(
                std::sync::atomic::Ordering::Relaxed,
                std::sync::atomic::Ordering::Relaxed,
                |value| value.checked_add(1),
            )
            .map_err(|_| Error::new(E_FAIL, "Inline text epoch exhausted"))?;
        if RegisterClassW(&WNDCLASSW {
            lpfnWndProc: Some(messages::popup),
            lpszClassName: class,
            hCursor: LoadCursorW(None, IDC_IBEAM)?,
            ..Default::default()
        }) == 0
            && GetLastError() != ERROR_CLASS_ALREADY_EXISTS
        {
            return Err(Error::from_win32());
        }
        let paint = paint::Style::new(style);
        let hwnd = CreateWindowExW(
            WS_EX_TOOLWINDOW
                | if paint.keyed() {
                    WS_EX_LAYERED
                } else {
                    WINDOW_EX_STYLE(0)
                },
            class,
            &windows::core::HSTRING::from(crate::ui_text::text("action-inline-text")),
            WS_POPUP | WS_CLIPCHILDREN,
            0,
            0,
            1,
            1,
            Some(owner),
            None,
            None,
            None,
        )?;
        let mut ui = Self {
            hwnd,
            edit: HWND::default(),
            marker: Box::new(0),
            font: HFONT::default(),
            epoch,
            backing: None,
            keyed: paint.keyed(),
            view: None,
            inset: geometry.map(|b| (b.padding() * scale).round() as i32),
            radius_px: if geometry.is_some() && paint.has_fill() {
                (style.corner_radius() * scale).round() as i32
            } else {
                0
            },
        };
        if let Err(error) = SetPropW(
            hwnd,
            MARKER,
            Some(HANDLE((&*ui.marker as *const u8).cast_mut().cast())),
        ) {
            let _ = DestroyWindow(hwnd);
            return Err(error);
        }
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, owner.0 as isize);
        SetPropW(hwnd, EPOCH, Some(HANDLE(epoch as *mut _)))?;
        // Inherit actual-host screenshot protection read-only; on failure hide the attached layer that might expose input.
        let mut affinity = 0;
        GetWindowDisplayAffinity(owner, &mut affinity)?;
        SetWindowDisplayAffinity(hwnd, WINDOW_DISPLAY_AFFINITY(affinity))?;
        paint.install(hwnd)?;
        ui.backing = paint.backing(class, owner)?;
        if let Some(backing) = &ui.backing {
            SetWindowDisplayAffinity(backing.0, WINDOW_DISPLAY_AFFINITY(affinity))?;
        }
        ui.edit = CreateWindowExW(
            WINDOW_EX_STYLE(0),
            w!("EDIT"),
            w!(""),
            WS_CHILD
                | WS_TABSTOP
                | if geometry.is_some() {
                    WS_CLIPCHILDREN
                } else {
                    WINDOW_STYLE(0)
                }
                | WINDOW_STYLE(
                    (ES_MULTILINE
                        | ES_AUTOVSCROLL
                        | ES_WANTRETURN
                        | if geometry.is_none() {
                            ES_AUTOHSCROLL
                        } else {
                            0
                        }) as u32,
                ),
            1,
            1,
            1,
            1,
            Some(hwnd),
            None,
            None,
            None,
        )?;
        ui.font = CreateFontW(
            -(font.font_size() * scale).round().clamp(1.0, 2048.0) as i32,
            0,
            0,
            0,
            if font.bold() {
                FW_BOLD.0 as i32
            } else {
                FW_MEDIUM.0 as i32
            },
            0,
            0,
            0,
            DEFAULT_CHARSET,
            OUT_DEFAULT_PRECIS,
            CLIP_DEFAULT_PRECIS,
            // Anti-aliasing against the key leaves a colored fringe after composition.
            if ui.keyed {
                NONANTIALIASED_QUALITY
            } else {
                CLEARTYPE_QUALITY
            },
            DEFAULT_PITCH.0 as u32,
            &HSTRING::from(font.family()),
        );
        if ui.font.is_invalid() {
            return Err(Error::from_win32());
        }
        SendMessageW(ui.edit, WM_SETFONT, Some(WPARAM(ui.font.0 as usize)), None);
        // EDIT stores CRLF pairs; the model counts one unit for a normalized newline.
        SendMessageW(
            ui.edit,
            EM_SETLIMITTEXT,
            Some(WPARAM(if geometry.is_some() { limit * 2 } else { limit })),
            None,
        );
        let status = SendMessageW(
            ui.edit,
            EM_GETIMESTATUS,
            Some(WPARAM(EMSIS_COMPOSITIONSTRING as usize)),
            None,
        );
        SendMessageW(
            ui.edit,
            EM_SETIMESTATUS,
            Some(WPARAM(EMSIS_COMPOSITIONSTRING as usize)),
            Some(LPARAM(status.0 | EIMES_COMPLETECOMPSTRKILLFOCUS as isize)),
        );
        let initial = if geometry.is_some() {
            initial
                .replace("\r\n", "\n")
                .replace('\r', "\n")
                .replace('\n', "\r\n")
        } else {
            initial.into()
        };
        SetWindowTextW(ui.edit, &HSTRING::from(initial))?;
        messages::install(ui.edit)?;
        if let Some(b) = geometry {
            ui.view = Some(boxed::View::install(ui.edit, scale, b.padding(), ui.keyed)?);
        }
        Ok(ui)
    }
    pub(super) unsafe fn measure(
        &self,
        text: &TextAnnotation,
        crlf: Vec<(u32, u32)>,
    ) -> Result<Point> {
        match &self.view {
            Some(view) => view.update(text, crlf),
            None => desktop::annotation_text::measure(text),
        }
    }
    pub(super) fn clear_layout(&self) {
        if let Some(view) = &self.view {
            view.clear();
        }
    }
    #[cfg(test)]
    pub(super) fn hwnd(&self) -> HWND {
        self.hwnd
    }
    pub(super) fn matches(&self, epoch: WPARAM, sender: LPARAM) -> bool {
        epoch.0 == self.epoch && sender.0 == self.hwnd.0 as isize
    }
    pub(super) unsafe fn alive(&self) -> bool {
        GetPropW(self.hwnd, MARKER).0 == (&*self.marker as *const u8).cast_mut().cast()
    }
    pub(super) unsafe fn contains(&self, hwnd: HWND) -> bool {
        self.alive() && (hwnd == self.hwnd || hwnd == self.edit)
    }
    pub(super) unsafe fn forward_click(&self, owner: HWND, message: &MSG) -> bool {
        if message.hwnd != owner
            || !matches!(
                message.message,
                WM_LBUTTONDOWN | WM_LBUTTONDBLCLK | WM_RBUTTONDOWN | WM_MBUTTONDOWN
            )
        {
            return false;
        }
        let Some(point) = self.keyed_edit_point(message.pt) else {
            return false;
        };
        // Color-keyed pixels hit the owned document beneath the EDIT. Route only
        // that document's click, never a click targeting another window.
        let _ = SetFocus(Some(self.edit));
        SendMessageW(
            self.edit,
            message.message,
            Some(message.wParam),
            Some(LPARAM(
                (u32::from(point.x as u16) | (u32::from(point.y as u16) << 16)) as isize,
            )),
        );
        true
    }
    pub(super) unsafe fn keyed_edit_point(&self, mut point: POINT) -> Option<POINT> {
        let mut rect = RECT::default();
        (self.keyed
            && self.alive()
            && ScreenToClient(self.edit, &mut point).as_bool()
            && GetClientRect(self.edit, &mut rect).is_ok()
            && PtInRect(&rect, point).as_bool())
        .then_some(point)
    }
    pub(super) unsafe fn place(&self, origin: POINT, size: Point) -> Result<()> {
        if !self.alive() {
            return Err(Error::new(E_HANDLE, "Inline text window retired"));
        }
        SetWindowPos(
            self.hwnd,
            None,
            origin.x,
            origin.y,
            size.x as i32,
            size.y as i32,
            SWP_NOACTIVATE | SWP_NOZORDER,
        )?;
        if let Some(backing) = &self.backing {
            SetWindowPos(
                backing.0,
                Some(self.hwnd),
                origin.x,
                origin.y,
                size.x as i32,
                size.y as i32,
                SWP_NOACTIVATE,
            )?;
        }
        if self.radius_px > 0 {
            let (w, h) = (size.x as i32, size.y as i32);
            paint::rounded(self.hwnd, w, h, self.radius_px)?;
            if let Some(backing) = &self.backing {
                paint::rounded(backing.0, w, h, self.radius_px)?;
            }
        }
        SetWindowPos(
            self.edit,
            None,
            1,
            1,
            size.x as i32 - 2,
            size.y as i32 - 2,
            SWP_NOACTIVATE | SWP_NOZORDER,
        )?;
        let rect = if let Some(inset) = self.inset {
            RECT {
                left: inset,
                top: inset,
                right: (size.x as i32 - 2 - inset).max(inset + 1),
                bottom: (size.y as i32 - 2 - inset).max(inset + 1),
            }
        } else {
            RECT {
                left: 3,
                top: 3,
                right: size.x as i32 - 5,
                bottom: size.y as i32 - 5,
            }
        };
        SendMessageW(
            self.edit,
            EM_SETRECT,
            None,
            Some(LPARAM((&rect as *const RECT) as isize)),
        );
        if let Some(view) = &self.view {
            view.place(size.x as i32 - 2, size.y as i32 - 2)?;
        }
        Ok(())
    }
    pub(super) unsafe fn show(&self) -> Result<()> {
        if let Some(backing) = &self.backing {
            let _ = ShowWindow(backing.0, SW_SHOWNOACTIVATE);
        }
        let _ = ShowWindow(self.edit, SW_SHOWNOACTIVATE);
        let _ = ShowWindow(self.hwnd, SW_SHOWNOACTIVATE);
        RedrawWindow(
            Some(self.hwnd),
            None,
            None,
            RDW_INVALIDATE | RDW_ALLCHILDREN | RDW_UPDATENOW,
        )
        .ok()?;
        let _ = SetFocus(Some(self.edit));
        if GetFocus() != self.edit {
            return Err(Error::new(E_FAIL, "Inline text focus not acquired"));
        }
        // New text starts empty; re-editing places the caret at the end instead of selecting all old content and replacing it with the first character.
        SendMessageW(
            self.edit,
            EM_SETSEL,
            Some(WPARAM(usize::MAX)),
            Some(LPARAM(-1)),
        );
        Ok(())
    }
    pub(super) unsafe fn hide(&self) {
        if self.alive() {
            let _ = ShowWindow(self.hwnd, SW_HIDE);
        }
        if let Some(backing) = &self.backing {
            let _ = ShowWindow(backing.0, SW_HIDE);
        }
    }
    pub(super) unsafe fn text(&self) -> Result<String> {
        self.draft(false).map(|(value, _)| value)
    }
    pub(super) unsafe fn draft(&self, map_offsets: bool) -> Result<(String, Vec<(u32, u32)>)> {
        if !self.alive() {
            return Err(Error::new(E_HANDLE, "Inline text window retired"));
        }
        let len = GetWindowTextLengthW(self.edit);
        let limit = crate::text_annotation::MAX_TEXT_UNITS;
        if len < 0
            || len as usize
                > if self.inset.is_some() {
                    2 * limit
                } else {
                    limit
                }
        {
            return Err(Error::new(E_INVALIDARG, "Inline text exceeds unit limit"));
        }
        let mut buffer = vec![0_u16; len as usize + 1];
        if GetWindowTextW(self.edit, &mut buffer) != len {
            return Err(Error::new(E_FAIL, "Inline text changed during read"));
        }
        buffer.truncate(len as usize);
        let mut crlf = Vec::new();
        if self.inset.is_some() {
            let (mut read, mut write) = (0, 0);
            while read < buffer.len() {
                let mut unit = buffer[read];
                if unit == 13 {
                    unit = 10;
                    if buffer.get(read + 1) == Some(&10) {
                        if map_offsets {
                            crlf.push((write as u32, read as u32));
                        }
                        read += 1;
                    }
                }
                buffer[write] = unit;
                write += 1;
                read += 1;
            }
            buffer.truncate(write);
        }
        if buffer.len() > limit {
            return Err(Error::new(E_INVALIDARG, "Inline text exceeds unit limit"));
        }
        String::from_utf16(&buffer)
            .map(|s| (s, crlf))
            .map_err(|_| Error::new(E_INVALIDARG, "Invalid inline UTF-16"))
    }
}
impl Drop for Window {
    fn drop(&mut self) {
        unsafe {
            if self.alive() {
                let _ = RemovePropW(self.hwnd, MARKER);
                let _ = DestroyWindow(self.hwnd);
            }
            if !self.font.is_invalid() {
                let _ = DeleteObject(HGDIOBJ(self.font.0));
            }
        }
    }
}
