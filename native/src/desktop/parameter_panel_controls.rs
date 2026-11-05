//! 原生控件拥有本地草稿；绑定固定快照索引，UI 不解释最新参数目录或直接改业务状态。
use super::*;
#[path = "parameter_panel_candidates.rs"]
pub(super) mod candidates;
#[path = "parameter_panel_capsule.rs"]
mod capsule;
#[path = "parameter_panel_rows.rs"]
mod rows;
#[path = "parameter_panel_sticker.rs"]
mod sticker;
#[derive(Clone)]
pub(super) enum Kind {
    Text {
        snapshot: Rc<Snapshot>,
        index: usize,
        spec: desktop::parameter_value::Spec,
        initial: String,
    },
    Choice(Rc<Snapshot>, usize),
    Options(Rc<Snapshot>, Vec<usize>),
    Step(HWND, desktop::parameter_value::Spec, bool),
    Slider(
        Rc<desktop::parameter_slider::Control>,
        HWND,
        Rc<Snapshot>,
        usize,
    ),
    Setting(Stamp, usize),
    Run(Stamp),
    Stop(Stamp),
    Relay(Stamp),
    Ports(Stamp),
    Sticker(Stamp, desktop::sticker_input::Action),
    Candidate(Rc<candidates::Choice>),
    Page(usize),
    Reload,
}
#[derive(Clone)]
pub(super) struct Binding {
    pub hwnd: HWND,
    pub key: String,
    pub kind: Kind,
    pub enabled: bool,
}
struct Builder<'a> {
    ui: &'a Ui,
    bindings: Vec<Binding>,
    placements: Vec<(HWND, RECT)>,
    y: i32,
    next: usize,
}
impl Builder<'_> {
    unsafe fn child(
        &mut self,
        class: windows::core::PCWSTR,
        text: &str,
        style: WINDOW_STYLE,
        x: i32,
        width: i32,
        height: i32,
    ) -> Result<HWND> {
        let rect = layout::field_rect(
            self.ui,
            RECT {
                left: x,
                top: self.y,
                right: x + width,
                bottom: self.y + height,
            },
        );
        let scale = GetDpiForWindow(self.ui.hwnd.get()).max(96) as i32;
        let hwnd = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            class,
            &HSTRING::from(text),
            WS_CHILD | WS_VISIBLE | style,
            rect.left * scale / 96,
            rect.top * scale / 96,
            (rect.right - rect.left) * scale / 96,
            (rect.bottom - rect.top) * scale / 96,
            Some(self.ui.hwnd.get()),
            Some(HMENU(self.next as *mut _)),
            None,
            None,
        )?;
        self.next += 1;
        self.placements.push((hwnd, rect));
        SendMessageW(
            hwnd,
            WM_SETFONT,
            Some(WPARAM(
                desktop::tool_panel::font::object(self.ui.hwnd.get()).0 as usize,
            )),
            Some(LPARAM(1)),
        );
        Ok(hwnd)
    }
    unsafe fn label(&mut self, text: &str) -> Result<HWND> {
        self.child(
            w!("STATIC"),
            text,
            WINDOW_STYLE::default(), // Win32 STATIC 的 SS_LEFT 为零。
            12,
            416,
            22,
        )
    }
    unsafe fn button(
        &mut self,
        key: &str,
        text: &str,
        x: i32,
        width: i32,
        kind: Kind,
        enabled: bool,
    ) -> Result<HWND> {
        let hwnd = self.child(
            w!("BUTTON"),
            text,
            WS_TABSTOP | WINDOW_STYLE(BS_OWNERDRAW as u32),
            x,
            width,
            28,
        )?;
        self.bindings.push(Binding {
            hwnd,
            key: key.into(),
            kind,
            enabled,
        });
        Ok(hwnd)
    }
}
pub(super) unsafe fn rebuild(ui: &Ui) -> Result<()> {
    // 所有旧 handle/草稿先移出借用再销毁；同步失焦不能提交被替换的控件。
    let focus = GetFocus();
    let focused = ui
        .bindings
        .borrow()
        .iter()
        .find(|b| b.hwnd == focus)
        .map(|b| b.key.clone());
    let foreground = GetForegroundWindow();
    let restore = focused.is_some() && (foreground == ui.hwnd.get() || foreground == ui.host);
    ui.replacing.set(true);
    let old = ui.placements.take();
    ui.bindings.borrow_mut().clear();
    for (hwnd, _) in old {
        if IsWindow(Some(hwnd)).as_bool() {
            let _ = DestroyWindow(hwnd);
        }
    }
    let result = build(ui);
    ui.replacing.set(false);
    result?;
    if restore && ui.available() && GetForegroundWindow() == foreground {
        let child = ui
            .bindings
            .borrow()
            .iter()
            .find(|b| Some(&b.key) == focused.as_ref())
            .map(|b| b.hwnd);
        if let Some(child) = child {
            let _ = SetFocus(Some(child));
        }
    }
    availability(ui);
    layout::place(ui);
    Ok(())
}
unsafe fn build(ui: &Ui) -> Result<()> {
    if ui.field.is_some() {
        return capsule::build(ui);
    }
    let model = ui.model.borrow().clone();
    let page = ui.page.get().min(model.pages() - 1);
    ui.page.set(page);
    let mut b = Builder {
        ui,
        bindings: Vec::new(),
        placements: Vec::new(),
        y: 10,
        next: 1000,
    };
    b.label(&model.title)?;
    b.y += 26;
    b.button("reload", "重新加载", 12, 416, Kind::Reload, true)?;
    b.y += 34;
    for (index, (label, checked)) in model.settings.iter().enumerate() {
        let kind = if model.snapshot.is_some() {
            Kind::Setting(model.stamp.clone(), index)
        } else {
            Kind::Relay(model.stamp.clone())
        };
        b.button(
            &format!("setting:{index}"),
            &format!("{} {label}", if *checked { "☑" } else { "☐" }),
            12,
            416,
            kind,
            true,
        )?;
        b.y += 32;
    }
    if model.snapshot.is_some() {
        b.button(
            "run",
            "执行一次",
            12,
            202,
            Kind::Run(model.stamp.clone()),
            true,
        )?;
        b.button(
            "stop",
            "停止",
            226,
            202,
            Kind::Stop(model.stamp.clone()),
            true,
        )?;
        b.y += 36;
    }
    if let Some(endpoint) = model.ports.as_ref().and_then(|ports| {
        ports
            .endpoints
            .iter()
            .find(|p| ui.state().is_some_and(|s| s.unit_id == p.unit))
    }) {
        b.button(
            "ports",
            "端口显示…",
            12,
            416,
            Kind::Ports(model.stamp.clone()),
            true,
        )?;
        b.y += 32;
        for (prefix, ports) in [("输入", &endpoint.inputs), ("输出", &endpoint.outputs)] {
            // 只列真实目录名称，不复制像素或把只读端口清单冒充连线画布。
            b.label(&format!(
                "{prefix} · {}",
                ports
                    .iter()
                    .take(8)
                    .map(|p| model::clean(p))
                    .collect::<Vec<_>>()
                    .join(" · ")
            ))?;
            b.y += 24;
        }
    }
    sticker::build(&mut b, &model)?;
    candidates::build(&mut b, &model)?;
    if let Some(snapshot) = &model.snapshot {
        for row in model
            .rows
            .iter()
            .skip(page * model::PAGE_SIZE)
            .take(model::PAGE_SIZE)
        {
            rows::build(&mut b, snapshot, row)?;
        }
    }
    if model.pages() > 1 {
        b.button(
            "previous",
            "上一页",
            12,
            100,
            Kind::Page(page.saturating_sub(1)),
            page > 0,
        )?;
        b.button(
            "next",
            "下一页",
            124,
            100,
            Kind::Page(page + 1),
            page + 1 < model.pages(),
        )?;
        b.child(
            w!("STATIC"),
            &format!("{} / {}", page + 1, model.pages()),
            WINDOW_STYLE::default(),
            238,
            190,
            24,
        )?;
        b.y += 34;
    }
    let message = ui.message.borrow().clone();
    ui.status.set(b.label(&message)?);
    b.y += 28;
    ui.content_height.set(b.y);
    *ui.placements.borrow_mut() = b.placements;
    *ui.bindings.borrow_mut() = b.bindings;
    Ok(())
}
pub(super) unsafe fn availability(ui: &Ui) {
    let available = ui.available();
    let bindings = ui.bindings.borrow().clone();
    for binding in bindings {
        let enabled = available
            && binding.enabled
            && match binding.kind {
                Kind::Run(_) => ui.state().is_some_and(desktop::execution_start::available),
                Kind::Candidate(ref choice) => ui.state().is_some_and(|s| choice.available(s)),
                _ => true,
            };
        if IsWindowEnabled(binding.hwnd).as_bool() != enabled {
            let _ = EnableWindow(binding.hwnd, enabled);
        }
    }
}
pub(super) unsafe fn text(hwnd: HWND) -> Option<String> {
    let length = GetWindowTextLengthW(hwnd).max(0) as usize;
    if length > desktop::parameter_value::MAX_TEXT {
        return None;
    }
    let mut text = vec![0; length + 1];
    let count = GetWindowTextW(hwnd, &mut text).max(0) as usize;
    String::from_utf16(&text[..count]).ok()
}
pub(super) unsafe fn commit(ui: &Ui, binding: &Binding) {
    if let Kind::Text {
        snapshot,
        index,
        spec,
        initial,
    } = &binding.kind
    {
        if keyboard::composing(binding.hwnd) {
            return;
        }
        let Some(text) = text(binding.hwnd) else {
            ui.error("输入过长或 UTF-16 无效");
            return;
        };
        if &text == initial {
            return;
        }
        if spec.parse(&text).is_none() {
            ui.error("参数值无效");
            return;
        }
        ui.queue(Intent::Choice(snapshot.clone(), *index, Some(text)));
    }
}
pub(super) unsafe fn drafts(ui: &Ui) -> std::result::Result<Vec<Intent>, ()> {
    let bindings = ui.bindings.borrow().clone();
    let mut drafts = Vec::new();
    let mut owner = None;
    for binding in bindings {
        if !binding.enabled {
            continue;
        }
        if let Kind::Text {
            snapshot,
            index,
            spec,
            initial,
        } = binding.kind
        {
            if keyboard::composing(binding.hwnd) {
                return Err(());
            }
            let text = text(binding.hwnd).ok_or(())?;
            if text != initial {
                spec.parse(&text).ok_or(())?;
                owner = Some(snapshot);
                drafts.push((index, text));
            }
        }
    }
    Ok(owner.map_or_else(Vec::new, |snapshot| vec![Intent::Texts(snapshot, drafts)]))
}
