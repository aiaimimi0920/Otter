//! 同一参数控件 owner 的两种窗口宿主：完整面板，或端口条内唯一单字段 child。
use super::*;

impl Ui {
    pub(super) fn capture(&self, state: &State) -> model::Model {
        let mut model = model::Model::capture(state);
        if let Some(field) = &self.field {
            model.rows.retain(|row| &row.id == field);
        }
        model
    }
}
impl Panel {
    pub(super) unsafe fn create(
        host: HWND,
        state: &State,
        embedded: Option<(HWND, &str)>,
    ) -> Result<Self> {
        let parent = embedded.map_or(HWND::default(), |(parent, _)| parent);
        let mut model = model::Model::capture(state);
        if let Some((_, field)) = embedded {
            model.rows.retain(|row| row.id == field);
        }
        let ui = Ui {
            host,
            host_identity: state as *const State as isize,
            field: embedded.map(|(_, field)| field.into()),
            parent,
            parent_identity: GetWindowLongPtrW(parent, GWLP_USERDATA),
            committed: Cell::new(false),
            hwnd: Cell::default(),
            model: RefCell::new(Rc::new(model)),
            bindings: RefCell::default(),
            placements: RefCell::default(),
            pending: RefCell::default(),
            closed: Cell::new(false),
            replacing: Cell::new(false),
            page: Cell::new(0),
            scroll: Cell::new(0),
            scroll_x: Cell::new(0),
            content_height: Cell::new(0),
            status: Cell::default(),
            message: RefCell::default(),
            capture_windows: state.capture_windows.clone(),
        };
        if ui.state().is_none()
            || embedded.is_some_and(|_| {
                !IsWindow(Some(parent)).as_bool() || ui.model.borrow().rows.len() != 1
            })
        {
            return Err(Error::new(
                E_HANDLE,
                "Parameter owner or capsule field is not current",
            ));
        }
        let instance = HINSTANCE(GetModuleHandleW(None)?.0);
        let class = w!("Otter.Native.Parameters");
        if RegisterClassW(&WNDCLASSW {
            hInstance: instance,
            lpszClassName: class,
            lpfnWndProc: Some(events::procedure),
            hCursor: LoadCursorW(None, IDC_ARROW)?,
            ..Default::default()
        }) == 0
            && GetLastError() != ERROR_CLASS_ALREADY_EXISTS
        {
            return Err(Error::from_win32());
        }
        let rect = if embedded.is_some() {
            RECT {
                right: 300,
                bottom: 240,
                ..Default::default()
            }
        } else {
            layout::initial(host)?
        };
        let panel = Self { ui: Rc::new(ui) };
        let hwnd = CreateWindowExW(
            WS_EX_CONTROLPARENT
                | if embedded.is_some() {
                    WINDOW_EX_STYLE::default()
                } else {
                    WS_EX_TOOLWINDOW
                },
            class,
            w!("Otter · 参数（Tab）"),
            WS_CLIPCHILDREN
                | if embedded.is_some() {
                    WS_CHILD
                } else {
                    WS_OVERLAPPED
                        | WS_CAPTION
                        | WS_SYSMENU
                        | WS_THICKFRAME
                        | WS_VSCROLL
                        | WS_HSCROLL
                },
            rect.left,
            rect.top,
            rect.right - rect.left,
            rect.bottom - rect.top,
            Some(if embedded.is_some() { parent } else { host }),
            None,
            Some(instance),
            Some(Rc::as_ptr(&panel.ui).cast()),
        )?;
        panel.ui.hwnd.set(hwnd);
        if embedded.is_none() {
            // Child继承端口条的顶层保护；DisplayAffinity API不能用于child。
            let enabled = state.capture_protection_requested.get();
            SetWindowDisplayAffinity(
                hwnd,
                if enabled {
                    WDA_EXCLUDEFROMCAPTURE
                } else {
                    WDA_NONE
                },
            )?;
            let mut actual = state.capture_protection.get();
            actual[2] = enabled;
            state.capture_protection.set(actual);
        }
        desktop::tool_panel::font::update(hwnd, GetDpiForWindow(hwnd))?;
        controls::rebuild(&panel.ui)?;
        if panel.ui.state().is_none() || !IsWindow(Some(hwnd)).as_bool() {
            return Err(Error::new(
                E_HANDLE,
                "Parameter owner retired during creation",
            ));
        }
        if embedded.is_none() {
            state.parameters_ui.hwnd.set(hwnd);
            state
                .parameters_ui
                .identity
                .set(Rc::as_ptr(&panel.ui) as isize);
        }
        Ok(panel)
    }
    pub(in crate::desktop) unsafe fn embedded(
        host: HWND,
        state: &State,
        parent: HWND,
        field: &str,
    ) -> Result<Self> {
        Self::create(host, state, Some((parent, field)))
    }
    pub(in crate::desktop) unsafe fn focus_embedded(&self) {
        if self.ui.field.is_some() && self.ui.available() {
            keyboard::enter(&self.ui);
        }
    }
    pub(in crate::desktop) unsafe fn owns_focus(&self) -> bool {
        let focus = GetFocus();
        focus == self.handle() || IsChild(self.handle(), focus).as_bool()
    }
    pub(in crate::desktop) fn embedded_height(&self) -> i32 {
        self.ui.content_height.get().clamp(1, 400)
    }
    pub(in crate::desktop) fn take_commit(&self) -> bool {
        self.ui.committed.replace(false)
    }
    pub(in crate::desktop) unsafe fn retire_embedded(&self) {
        self.ui.closed.set(true);
        self.ui.pending.borrow_mut().clear();
        let _ = ShowWindow(self.handle(), SW_HIDE);
    }
    pub(in crate::desktop) unsafe fn place_embedded(&self, rect: RECT) -> Result<()> {
        if self.ui.field.is_none() || self.retired() {
            return Err(Error::from(E_HANDLE));
        }
        let old_font = desktop::tool_panel::font::object(self.handle());
        desktop::tool_panel::font::update(self.handle(), GetDpiForWindow(self.ui.parent))?;
        if self.retired() {
            return Ok(());
        }
        let font = desktop::tool_panel::font::object(self.handle());
        if old_font != font {
            let placements = self.ui.placements.borrow().clone();
            for (child, _) in placements {
                SendMessageW(
                    child,
                    WM_SETFONT,
                    Some(WPARAM(font.0 as usize)),
                    Some(LPARAM(1)),
                );
                if self.retired() {
                    return Ok(());
                }
            }
        }
        SetWindowPos(
            self.handle(),
            None,
            rect.left,
            rect.top,
            rect.right - rect.left,
            rect.bottom - rect.top,
            SWP_NOACTIVATE | SWP_NOZORDER,
        )?;
        if self.retired() {
            return Ok(());
        }
        layout::place(&self.ui);
        if !self.retired() {
            let _ = ShowWindow(self.handle(), SW_SHOWNOACTIVATE);
        }
        Ok(())
    }
}
