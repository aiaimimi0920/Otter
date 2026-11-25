//! Two hosts for one parameter transaction owner: a complete local Art panel or multiline children in a port strip.
use super::*;

impl Ui {
    pub(super) fn capture(&self, state: &State) -> model::Model {
        model::Model::capture(state)
    }
}
impl Panel {
    pub(super) unsafe fn create(host: HWND, state: &State, parent: Option<HWND>) -> Result<Self> {
        let inline = parent.is_some();
        let parent = parent.unwrap_or_default();
        let model = model::Model::capture(state);
        let ui = Ui {
            host,
            host_identity: state as *const State as isize,
            inline,
            inline_rows: RefCell::default(),
            auxiliary: RefCell::default(),
            parent,
            parent_identity: GetWindowLongPtrW(parent, GWLP_USERDATA),
            hwnd: Cell::default(),
            model: RefCell::new(Rc::new(model)),
            bindings: RefCell::default(),
            placements: RefCell::default(),
            captions: RefCell::default(),
            language: Cell::new(crate::ui_text::current_language()),
            pending: RefCell::default(),
            closed: Cell::new(false),
            replacing: Cell::new(false),
            page: Cell::new(0),
            scroll: Cell::new(0),
            scroll_x: Cell::new(0),
            content_height: Cell::new(0),
            status: Cell::default(),
            status_dirty: Cell::default(),
            status_writing: Cell::default(),
            message: RefCell::default(),
            message_caption: RefCell::default(),
            capture_windows: state.capture_windows.clone(),
        };
        if ui.state().is_none() || inline && !IsWindow(Some(parent)).as_bool() {
            return Err(Error::new(
                E_HANDLE,
                "Parameter owner or inline parent is not current",
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
        let rect = if inline {
            RECT {
                right: 256,
                bottom: 240,
                ..Default::default()
            }
        } else {
            layout::initial(host)?
        };
        let panel = Self { ui: Rc::new(ui) };
        let hwnd = CreateWindowExW(
            WS_EX_CONTROLPARENT
                | if inline {
                    WINDOW_EX_STYLE::default()
                } else {
                    WS_EX_TOOLWINDOW
                },
            class,
            &HSTRING::from(crate::ui_text::text("parameter-panel-title")),
            WS_CLIPCHILDREN
                | if inline {
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
            Some(if inline { parent } else { host }),
            None,
            Some(instance),
            Some(Rc::as_ptr(&panel.ui).cast()),
        )?;
        panel.ui.hwnd.set(hwnd);
        if !inline {
            // Children inherit the port strip's top-level protection; DisplayAffinity APIs cannot be used on children.
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
        if inline {
            let mut actual = state.capture_protection.get();
            actual[2] = state.capture_protection_requested.get();
            state.capture_protection.set(actual);
        }
        state.parameters_ui.hwnd.set(hwnd);
        state
            .parameters_ui
            .identity
            .set(Rc::as_ptr(&panel.ui) as isize);
        Ok(panel)
    }
}
