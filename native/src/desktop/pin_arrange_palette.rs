//! Interactive design-tool palette for desktop pin alignment, distribution, and custom-gap packing.
use super::*;
use crate::desktop::dialog_controls;
use std::cell::{Cell, RefCell};
use windows::{
    core::HSTRING,
    Win32::{
        Graphics::Gdi::InvalidateRect,
        UI::{
            Controls::DRAWITEMSTRUCT,
            Input::KeyboardAndMouse::{EnableWindow, SetFocus},
        },
    },
};

#[path = "pin_arrange_palette_paint.rs"]
mod paint;

const USER: WINDOW_LONG_PTR_INDEX =
    WINDOW_LONG_PTR_INDEX((2 * std::mem::size_of::<isize>()) as i32);
const SCOPE_ALL: i32 = 301;
const SCOPE_SEL: i32 = 302;
const SCOPE_GRP: i32 = 303;
const ACTION_BASE: i32 = 400;
const GAP_MINUS: i32 = 501;
const GAP_EDIT: i32 = 502;
const GAP_PLUS: i32 = 503;
const UNDO_BTN: i32 = 601;
const STATUS_LBL: i32 = 602;

pub(in crate::desktop::application) const ACTIONS: &[(Layout, &str)] = &[
    (Layout::Left, "workspace-layout-left"),
    (Layout::CenterX, "workspace-layout-center-x"),
    (Layout::Right, "workspace-layout-right"),
    (Layout::Top, "workspace-layout-top"),
    (Layout::CenterY, "workspace-layout-center-y"),
    (Layout::Bottom, "workspace-layout-bottom"),
    (Layout::DistributeX, "workspace-layout-distribute-x"),
    (Layout::DistributeY, "workspace-layout-distribute-y"),
    (Layout::CompactX, "workspace-layout-compact-x"),
    (Layout::CompactY, "workspace-layout-compact-y"),
    (Layout::Grid, "workspace-layout-grid"),
];

#[derive(Clone, Copy, PartialEq, Eq)]
enum Scope {
    All,
    Selected,
    Group,
}

struct Session<'a> {
    app: &'a mut Application,
    anchor: UnitId,
    initial: menu::Snapshot,
    scope: Cell<Scope>,
    last_layout: Cell<Option<Layout>>,
    status: RefCell<String>,
}

pub(in crate::desktop::application) unsafe fn show(
    owner: HWND,
    app: &mut Application,
    anchor: &UnitId,
    initial: &menu::Snapshot,
) -> Result<()> {
    let default_scope = if initial.selected_count() >= 2 {
        Scope::Selected
    } else if initial.group_count() >= 2 {
        Scope::Group
    } else {
        Scope::All
    };
    let mut session = Session {
        app,
        anchor: anchor.clone(),
        initial: initial.clone(),
        scope: Cell::new(default_scope),
        last_layout: Cell::new(None),
        status: RefCell::new(String::new()),
    };
    let data = template();
    let res = DialogBoxIndirectParamW(
        None,
        data.as_ptr().cast(),
        Some(owner),
        Some(procedure),
        LPARAM(&mut session as *mut Session<'_> as isize),
    );
    if res == -1 {
        return Err(Error::from_win32());
    }
    Ok(())
}

impl Session<'_> {
    unsafe fn member_count(&self, scope: Scope) -> usize {
        let _tx = self.app.pin_layout.suspend_palette_gate();
        let fresh = menu::Snapshot::capture(self.app, &self.anchor);
        match scope {
            Scope::All => fresh.all_count(),
            Scope::Selected => fresh.selected_count(),
            Scope::Group => fresh.group_count(),
        }
    }

    unsafe fn sync_controls(&self, hwnd: HWND) {
        let scope = self.scope.get();
        let count = self.member_count(scope);
        for (id, s, available) in [
            (SCOPE_ALL, Scope::All, self.member_count(Scope::All) >= 2),
            (
                SCOPE_SEL,
                Scope::Selected,
                self.member_count(Scope::Selected) >= 2,
            ),
            (
                SCOPE_GRP,
                Scope::Group,
                self.member_count(Scope::Group) >= 2,
            ),
        ] {
            if let Ok(btn) = GetDlgItem(Some(hwnd), id) {
                let _ = EnableWindow(btn, available || scope == s);
                let _ = InvalidateRect(Some(btn), None, false);
            }
        }
        for (i, (layout, _)) in ACTIONS.iter().enumerate() {
            if let Ok(btn) = GetDlgItem(Some(hwnd), ACTION_BASE + i as i32) {
                let _ = EnableWindow(btn, count >= layout.min_count());
                let _ = InvalidateRect(Some(btn), None, false);
            }
        }
        if let Ok(undo) = GetDlgItem(Some(hwnd), UNDO_BTN) {
            let _ = EnableWindow(undo, !self.app.pin_layout.undo.is_empty());
            let _ = InvalidateRect(Some(undo), None, false);
        }
        let _ = SetDlgItemTextW(hwnd, STATUS_LBL, &HSTRING::from(&*self.status.borrow()));
    }

    unsafe fn read_gap(&self, hwnd: HWND) -> Option<u16> {
        let mut buf = [0u16; 16];
        let len = GetDlgItemTextW(hwnd, GAP_EDIT, &mut buf) as usize;
        let parsed = String::from_utf16_lossy(&buf[..len])
            .trim()
            .parse::<u16>()
            .ok()?;
        (parsed <= 512).then_some(parsed)
    }

    unsafe fn write_gap(&mut self, hwnd: HWND, gap: u16) {
        let clamped = gap.min(512);
        self.app.pin_layout.gap_dip.set(clamped);
        let _ = SetDlgItemTextW(hwnd, GAP_EDIT, &HSTRING::from(clamped.to_string()));
    }

    unsafe fn apply(&mut self, hwnd: HWND, layout: Layout, label_key: &str) {
        if let Some(gap) = self.read_gap(hwnd) {
            self.app.pin_layout.gap_dip.set(gap);
        } else {
            let fallback = self.app.pin_layout.gap_dip.get();
            let _ = SetDlgItemTextW(hwnd, GAP_EDIT, &HSTRING::from(fallback.to_string()));
        }
        if !self.initial.matches_workspace(&self.app.workspace) {
            *self.status.borrow_mut() =
                crate::ui_text::text("workspace-layout-unavailable").to_owned();
            self.sync_controls(hwnd);
            return;
        }
        let res = {
            let _tx = self.app.pin_layout.suspend_palette_gate();
            let fresh = menu::Snapshot::capture(self.app, &self.anchor);
            let targets = match self.scope.get() {
                Scope::All => fresh.all_targets().to_vec(),
                Scope::Selected => fresh.selected_targets().to_vec(),
                Scope::Group => fresh.group_targets().to_vec(),
            };
            self.app.arrange_pins(&targets, &self.anchor, layout)
        };
        match res {
            Ok(()) => {
                self.last_layout.set(Some(layout));
                let mut msg = crate::ui_text::text(label_key).to_owned();
                if layout.uses_gap() {
                    msg = format!("{} · {} DIP", msg, self.app.pin_layout.gap_dip.get());
                }
                *self.status.borrow_mut() = msg;
            }
            Err(e) => {
                *self.status.borrow_mut() = e.message().to_string();
            }
        }
        self.sync_controls(hwnd);
    }
}

unsafe extern "system" fn procedure(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> isize {
    if msg == WM_INITDIALOG {
        SetWindowLongPtrW(hwnd, USER, lp.0);
        let _ = SetWindowDisplayAffinity(hwnd, WDA_EXCLUDEFROMCAPTURE);
        let s = &mut *(lp.0 as *mut Session<'_>);
        let mut ids = vec![
            SCOPE_ALL, SCOPE_SEL, SCOPE_GRP, GAP_MINUS, GAP_PLUS, UNDO_BTN, IDCANCEL.0,
        ];
        for i in 0..ACTIONS.len() {
            ids.push(ACTION_BASE + i as i32);
        }
        for id in ids {
            if let Ok(btn) = GetDlgItem(Some(hwnd), id) {
                let _ = dialog_controls::button(btn);
            }
        }
        if let Ok(edit) = GetDlgItem(Some(hwnd), GAP_EDIT) {
            let _ = dialog_controls::edit(edit);
            SendMessageW(edit, 0x00C5, Some(WPARAM(3)), Some(LPARAM(0)));
        }
        let gap = s.app.pin_layout.gap_dip.get();
        s.write_gap(hwnd, gap);
        *s.status.borrow_mut() = crate::ui_text::text(ACTIONS[0].1).to_owned();
        s.sync_controls(hwnd);
        if let Ok(first) = GetDlgItem(Some(hwnd), ACTION_BASE) {
            let _ = SetFocus(Some(first));
        }
        return 0;
    }
    if let Some(bg) = dialog_controls::background(msg, wp, lp) {
        return bg;
    }
    let ptr = GetWindowLongPtrW(hwnd, USER) as *mut Session<'_>;
    if ptr.is_null() {
        return 0;
    }
    if msg == WM_NCDESTROY {
        SetWindowLongPtrW(hwnd, USER, 0);
        return 0;
    }
    let s = &mut *ptr;
    if msg == WM_DRAWITEM {
        let item = &*(lp.0 as *const DRAWITEMSTRUCT);
        let id = item.CtlID as i32;
        if (ACTION_BASE..ACTION_BASE + ACTIONS.len() as i32).contains(&id) {
            let idx = (id - ACTION_BASE) as usize;
            let layout = ACTIONS[idx].0;
            paint::draw_icon_button(item, layout, s.last_layout.get() == Some(layout));
        } else {
            let active = (id == SCOPE_ALL && s.scope.get() == Scope::All)
                || (id == SCOPE_SEL && s.scope.get() == Scope::Selected)
                || (id == SCOPE_GRP && s.scope.get() == Scope::Group);
            dialog_controls::draw_button(item, active);
        }
        return 1;
    }
    if msg == WM_CLOSE {
        let _ = EndDialog(hwnd, 0);
        return 1;
    }
    if msg != WM_COMMAND {
        return 0;
    }
    let id = (wp.0 & 0xffff) as i32;
    let code = ((wp.0 >> 16) & 0xffff) as u32;
    if id == GAP_EDIT {
        if code == EN_CHANGE {
            if let Some(gap) = s.read_gap(hwnd) {
                s.app.pin_layout.gap_dip.set(gap);
            }
        }
        return 1;
    }
    match id {
        x if x == IDCANCEL.0 || x == IDOK.0 => {
            if x == IDOK.0 {
                if let Some(layout) = s.last_layout.get().filter(|l| l.uses_gap()) {
                    let label = ACTIONS
                        .iter()
                        .find(|(l, _)| *l == layout)
                        .map_or("workspace-layout-compact-x", |(_, k)| *k);
                    s.apply(hwnd, layout, label);
                    return 1;
                }
            }
            let _ = EndDialog(hwnd, 0);
        }
        SCOPE_ALL => {
            s.scope.set(Scope::All);
            s.sync_controls(hwnd);
        }
        SCOPE_SEL if s.member_count(Scope::Selected) >= 2 => {
            s.scope.set(Scope::Selected);
            s.sync_controls(hwnd);
        }
        SCOPE_GRP if s.member_count(Scope::Group) >= 2 => {
            s.scope.set(Scope::Group);
            s.sync_controls(hwnd);
        }
        GAP_MINUS | GAP_PLUS => {
            let cur = s
                .read_gap(hwnd)
                .unwrap_or_else(|| s.app.pin_layout.gap_dip.get());
            let next = if id == GAP_MINUS {
                cur.saturating_sub(4)
            } else {
                (cur + 4).min(512)
            };
            s.write_gap(hwnd, next);
            if let Some(layout) = s.last_layout.get().filter(|l| l.uses_gap()) {
                let label = ACTIONS
                    .iter()
                    .find(|(l, _)| *l == layout)
                    .map_or("workspace-layout-compact-x", |(_, k)| *k);
                s.apply(hwnd, layout, label);
            }
        }
        UNDO_BTN => {
            let res = {
                let _tx = s.app.pin_layout.suspend_palette_gate();
                s.app.undo_layout()
            };
            match res {
                Ok(()) => {
                    s.last_layout.set(None);
                    *s.status.borrow_mut() =
                        crate::ui_text::text("workspace-layout-undo").to_owned();
                }
                Err(e) => *s.status.borrow_mut() = e.message().to_string(),
            }
            s.sync_controls(hwnd);
        }
        x if (ACTION_BASE..ACTION_BASE + ACTIONS.len() as i32).contains(&x) => {
            let (layout, label) = ACTIONS[(x - ACTION_BASE) as usize];
            s.apply(hwnd, layout, label);
        }
        _ => return 0,
    }
    1
}

fn template() -> Vec<u32> {
    let mut d = Vec::new();
    let push_str = |d: &mut Vec<u16>, s: &str| {
        d.extend(s.encode_utf16());
        d.push(0);
    };
    let style = WS_POPUP.0
        | WS_CLIPCHILDREN.0
        | WS_CAPTION.0
        | WS_SYSMENU.0
        | (DS_SETFONT | DS_MODALFRAME) as u32;
    let items: u16 = 3 + 2 + 11 + 4 + 3;
    d.extend([
        style as u16,
        (style >> 16) as u16,
        0,
        0,
        items,
        0,
        0,
        236,
        192,
        0,
        0,
    ]);
    push_str(&mut d, crate::ui_text::text("workspace-layout-title"));
    d.push(9);
    push_str(&mut d, "Microsoft YaHei UI");

    let item = |d: &mut Vec<u16>,
                st: u32,
                x: i16,
                y: i16,
                w: i16,
                h: i16,
                id: i32,
                cls: u16,
                txt: &str| {
        if !d.len().is_multiple_of(2) {
            d.push(0);
        }
        d.extend([
            st as u16,
            (st >> 16) as u16,
            0,
            0,
            x as u16,
            y as u16,
            w as u16,
            h as u16,
            id as u16,
            0xffff,
            cls,
        ]);
        push_str(d, txt);
        d.push(0);
    };
    let btn = (WS_CHILD | WS_VISIBLE | WS_TABSTOP).0 | 0x000B; // BS_OWNERDRAW
    let lbl = (WS_CHILD | WS_VISIBLE).0;
    let edt = (WS_CHILD | WS_VISIBLE | WS_BORDER | WS_TABSTOP).0
        | (ES_CENTER | ES_NUMBER | ES_AUTOHSCROLL) as u32;

    item(
        &mut d,
        btn,
        12,
        10,
        68,
        18,
        SCOPE_ALL,
        0x0080,
        crate::ui_text::text("workspace-layout-scope-all"),
    );
    item(
        &mut d,
        btn,
        84,
        10,
        68,
        18,
        SCOPE_SEL,
        0x0080,
        crate::ui_text::text("workspace-layout-scope-selection"),
    );
    item(
        &mut d,
        btn,
        156,
        10,
        68,
        18,
        SCOPE_GRP,
        0x0080,
        crate::ui_text::text("workspace-layout-scope-group"),
    );

    item(
        &mut d,
        lbl,
        12,
        34,
        212,
        13,
        701,
        0x0082,
        crate::ui_text::text("workspace-layout-section-align"),
    );
    for i in 0..6 {
        let x = 12 + (i as i16) * 35 + if i >= 3 { 5 } else { 0 };
        item(&mut d, btn, x, 49, 32, 24, ACTION_BASE + i, 0x0080, "");
    }

    item(
        &mut d,
        lbl,
        12,
        80,
        212,
        13,
        702,
        0x0082,
        crate::ui_text::text("workspace-layout-section-distribute"),
    );
    for i in 0..5 {
        let x = 12 + (i as i16) * 42 + if i >= 2 { 6 } else { 0 };
        item(&mut d, btn, x, 95, 38, 24, ACTION_BASE + 6 + i, 0x0080, "");
    }

    item(
        &mut d,
        lbl,
        12,
        128,
        116,
        14,
        703,
        0x0082,
        crate::ui_text::text("workspace-layout-gap"),
    );
    item(&mut d, btn, 134, 126, 22, 18, GAP_MINUS, 0x0080, "−");
    item(&mut d, edt, 159, 126, 40, 18, GAP_EDIT, 0x0081, "12");
    item(&mut d, btn, 202, 126, 22, 18, GAP_PLUS, 0x0080, "+");

    item(&mut d, lbl, 12, 150, 212, 13, STATUS_LBL, 0x0082, "");
    item(
        &mut d,
        btn,
        112,
        167,
        54,
        18,
        UNDO_BTN,
        0x0080,
        crate::ui_text::text("common-undo"),
    );
    item(
        &mut d,
        btn,
        170,
        167,
        54,
        18,
        IDCANCEL.0,
        0x0080,
        crate::ui_text::text("common-close"),
    );
    if !d.len().is_multiple_of(2) {
        d.push(0);
    }
    d.chunks_exact(2)
        .map(|p| u32::from(p[0]) | (u32::from(p[1]) << 16))
        .collect()
}
