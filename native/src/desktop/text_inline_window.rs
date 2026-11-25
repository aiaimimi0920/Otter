//! 标注专属的无标题输入层，附着钉图坐标并置于 DComp/layered 图像之上；不是独立对话框。
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
}
impl Window {
    pub(super) unsafe fn new(
        owner: HWND,
        initial: &str,
        font: &crate::text_annotation::TextFont,
        style: crate::stroke_style::StrokeStyle,
        scale: f32,
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
            w!("输入标注文本"),
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
        // 只读继承真实宿主的截图保护；失败时不显示可能泄露输入的附属层。
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
                | WINDOW_STYLE(
                    (ES_MULTILINE | ES_AUTOVSCROLL | ES_AUTOHSCROLL | ES_WANTRETURN) as u32,
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
        SendMessageW(ui.edit, EM_SETLIMITTEXT, Some(WPARAM(limit)), None);
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
        SetWindowTextW(ui.edit, &HSTRING::from(initial))?;
        messages::install(ui.edit)?;
        Ok(ui)
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
        SetWindowPos(
            self.edit,
            None,
            1,
            1,
            size.x as i32 - 2,
            size.y as i32 - 2,
            SWP_NOACTIVATE | SWP_NOZORDER,
        )?;
        let rect = RECT {
            left: 3,
            top: 3,
            right: size.x as i32 - 5,
            bottom: size.y as i32 - 5,
        };
        SendMessageW(
            self.edit,
            EM_SETRECT,
            None,
            Some(LPARAM((&rect as *const RECT) as isize)),
        );
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
        // 新文本为空；重新编辑时光标落在末尾，不选择整个旧内容导致首字覆盖。
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
        if !self.alive() {
            return Err(Error::new(E_HANDLE, "Inline text window retired"));
        }
        let len = GetWindowTextLengthW(self.edit);
        if len < 0 || len as usize > crate::text_annotation::MAX_TEXT_UNITS {
            return Err(Error::new(E_INVALIDARG, "Inline text exceeds unit limit"));
        }
        let mut buffer = vec![0_u16; len as usize + 1];
        if GetWindowTextW(self.edit, &mut buffer) != len {
            return Err(Error::new(E_FAIL, "Inline text changed during read"));
        }
        String::from_utf16(&buffer[..len as usize])
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
