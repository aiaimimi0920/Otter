//! Native link picker; three dropdowns reference opening catalogs only, enqueue on confirmation, and never modify workspaces inside modals.
use super::link_catalog::Catalog;
#[cfg(test)]
use super::State;
use std::rc::Rc;
use windows::{
    core::{Error, Result, HSTRING},
    Win32::{Foundation::*, UI::WindowsAndMessaging::*},
};
#[cfg(test)]
pub(super) const COMMAND: usize = 0x1090;
const SOURCE: i32 = 101;
const OUTPUT: i32 = 102;
const INPUT: i32 = 103;
const USER: i32 = (2 * std::mem::size_of::<isize>()) as i32;
struct Dialog {
    outgoing: bool,
    catalog: Rc<Catalog>,
    choices: Vec<usize>,
    fixed: usize,
    result: Option<(usize, usize, usize)>,
    failed: bool,
}
#[cfg(test)]
pub(super) unsafe fn append(menu: HMENU, state: &State) -> Result<Option<Rc<Catalog>>> {
    let catalog = state.link_catalog.borrow().clone();
    let enabled = !state.group_locked.get()
        && catalog.as_ref().is_some_and(|catalog| {
            catalog
                .endpoints
                .iter()
                .any(|e| e.unit == state.unit_id && !e.inputs.is_empty())
                && catalog
                    .endpoints
                    .iter()
                    .any(|e| e.unit != state.unit_id && !e.outputs.is_empty())
        });
    AppendMenuW(
        menu,
        MF_STRING | if enabled { MF_ENABLED } else { MF_GRAYED },
        COMMAND,
        &HSTRING::from(crate::ui_text::text("workspace-menu-create-link-current")),
    )?;
    Ok(catalog)
}
#[cfg(test)]
pub(super) unsafe fn apply(
    catalog: Option<Rc<Catalog>>,
    hwnd: HWND,
    state: &State,
    command: usize,
) -> Result<bool> {
    if command != COMMAND {
        return Ok(false);
    }
    let Some(catalog) = catalog else {
        return Ok(true);
    };
    if state.group_locked.get()
        || !IsWindow(Some(hwnd)).as_bool()
        || !state
            .link_catalog
            .borrow()
            .as_ref()
            .is_some_and(|current| Rc::ptr_eq(current, &catalog))
    {
        return Ok(true);
    }
    let Some(target) = catalog
        .endpoints
        .iter()
        .position(|e| e.unit == state.unit_id && !e.inputs.is_empty())
    else {
        return Ok(true);
    };
    let choices: Vec<_> = catalog
        .endpoints
        .iter()
        .enumerate()
        .filter(|(_, e)| e.unit != state.unit_id && !e.outputs.is_empty())
        .map(|(index, _)| index)
        .collect();
    if choices.is_empty() {
        return Ok(true);
    }
    let mut dialog = Dialog {
        outgoing: false,
        catalog: catalog.clone(),
        fixed: target,
        choices,
        result: None,
        failed: false,
    };
    {
        let Some(_guard) = super::file_dialog::Guard::begin(&state.file_dialog) else {
            return Ok(true);
        };
        show(hwnd, &mut dialog)?;
    }
    if let Some((source, output, input)) = dialog.result {
        catalog.queue(hwnd, state, source, output, input)?;
    }
    Ok(true)
}

/// Callers own application modal scope and post-confirmation provenance revalidation; this layer selects endpoints without modifying workspaces.
pub(super) unsafe fn show_outgoing(
    hwnd: HWND,
    catalog: Rc<Catalog>,
    source: usize,
) -> Result<Option<(usize, usize, usize)>> {
    if catalog
        .endpoints
        .get(source)
        .is_none_or(|endpoint| endpoint.outputs.is_empty())
    {
        return Ok(None);
    }
    let choices = catalog
        .endpoints
        .iter()
        .enumerate()
        .filter(|(index, endpoint)| *index != source && !endpoint.inputs.is_empty())
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    if choices.is_empty() {
        return Ok(None);
    }
    let mut dialog = Dialog {
        outgoing: true,
        catalog,
        choices,
        fixed: source,
        result: None,
        failed: false,
    };
    show(hwnd, &mut dialog)?;
    Ok(dialog.result)
}
unsafe fn show(hwnd: HWND, dialog: &mut Dialog) -> Result<()> {
    let template = template(dialog.outgoing);
    if DialogBoxIndirectParamW(
        None,
        template.as_ptr().cast(),
        Some(hwnd),
        Some(procedure),
        LPARAM(dialog as *mut _ as isize),
    ) == -1
    {
        return Err(Error::from_win32());
    }
    if dialog.failed {
        Err(Error::new(E_FAIL, "Cannot populate link selection dialog"))
    } else {
        Ok(())
    }
}
fn endpoints(dialog: &Dialog, choice: usize) -> (usize, usize) {
    if dialog.outgoing {
        (dialog.fixed, choice)
    } else {
        (choice, dialog.fixed)
    }
}

#[cfg(test)]
#[path = "link_create_tests.rs"]
pub(super) mod tests;
unsafe fn fill<'a>(hwnd: HWND, id: i32, items: impl Iterator<Item = &'a str>) -> bool {
    let Ok(combo) = GetDlgItem(Some(hwnd), id) else {
        return false;
    };
    SendMessageW(combo, CB_RESETCONTENT, None, None);
    for (index, item) in items.enumerate() {
        let item = HSTRING::from(item);
        let inserted = SendMessageW(
            combo,
            CB_ADDSTRING,
            None,
            Some(LPARAM(item.as_ptr() as isize)),
        )
        .0;
        // Selection indices address the immutable catalog, not just the rows
        // that survived insertion. Never accept a partial or reordered list.
        if usize::try_from(inserted).ok() != Some(index) {
            return false;
        }
    }
    SendMessageW(combo, CB_SETCURSEL, Some(WPARAM(0)), None).0 == 0
}
unsafe fn selected(hwnd: HWND, id: i32) -> Option<usize> {
    let result = SendMessageW(GetDlgItem(Some(hwnd), id).ok()?, CB_GETCURSEL, None, None).0;
    usize::try_from(result).ok()
}
unsafe fn refresh_choice_ports(hwnd: HWND, dialog: &Dialog) -> bool {
    if let Some(choice) = selected(hwnd, SOURCE).and_then(|index| dialog.choices.get(index)) {
        let (source, target) = endpoints(dialog, *choice);
        let (id, ports) = if dialog.outgoing {
            (INPUT, &dialog.catalog.endpoints[target].inputs)
        } else {
            (OUTPUT, &dialog.catalog.endpoints[source].outputs)
        };
        fill(hwnd, id, ports.iter().map(String::as_str))
    } else {
        false
    }
}
unsafe fn initialize(hwnd: HWND, dialog: &Dialog) -> bool {
    let fixed = &dialog.catalog.endpoints[dialog.fixed];
    let (id, ports) = if dialog.outgoing {
        (OUTPUT, &fixed.outputs)
    } else {
        (INPUT, &fixed.inputs)
    };
    let labels: Vec<_> = dialog
        .choices
        .iter()
        .map(|index| dialog.catalog.endpoints[*index].display_label())
        .collect();
    fill(hwnd, SOURCE, labels.iter().map(|label| label.as_ref()))
        && fill(hwnd, id, ports.iter().map(String::as_str))
        && refresh_choice_ports(hwnd, dialog)
}
unsafe fn fail(hwnd: HWND, pointer: *mut Dialog) {
    (*pointer).failed = true;
    let _ = EndDialog(hwnd, 0);
}
unsafe extern "system" fn procedure(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> isize {
    if message == WM_INITDIALOG {
        SetWindowLongPtrW(hwnd, WINDOW_LONG_PTR_INDEX(USER), lparam.0);
        let pointer = lparam.0 as *mut Dialog;
        if !initialize(hwnd, &*pointer) {
            fail(hwnd, pointer);
        }
        return 1;
    }
    if message == WM_CLOSE || message == WM_COMMAND && wparam.0 & 0xffff == IDCANCEL.0 as usize {
        let _ = EndDialog(hwnd, 0);
        return 1;
    }
    if message != WM_COMMAND {
        return 0;
    }
    let pointer = GetWindowLongPtrW(hwnd, WINDOW_LONG_PTR_INDEX(USER)) as *mut Dialog;
    if pointer.is_null() {
        return 0;
    }
    if (*pointer).failed {
        return 1;
    }
    let command = wparam.0 & 0xffff;
    if command == SOURCE as usize && wparam.0 >> 16 == CBN_SELCHANGE as usize {
        if !refresh_choice_ports(hwnd, &*pointer) {
            fail(hwnd, pointer);
        }
        return 1;
    }
    if command == IDOK.0 as usize {
        let dialog = &mut *pointer;
        if let (Some(choice), Some(output), Some(input)) = (
            selected(hwnd, SOURCE).and_then(|index| dialog.choices.get(index).copied()),
            selected(hwnd, OUTPUT),
            selected(hwnd, INPUT),
        ) {
            let (source, target) = endpoints(dialog, choice);
            if output < dialog.catalog.endpoints[source].outputs.len()
                && input < dialog.catalog.endpoints[target].inputs.len()
            {
                dialog.result = Some((choice, output, input));
                let _ = EndDialog(hwnd, 1);
            }
        }
        return 1;
    }
    0
}
fn template(outgoing: bool) -> Vec<u32> {
    fn string(data: &mut Vec<u16>, text: &str) {
        data.extend(text.encode_utf16());
        data.push(0);
    }
    fn dword(data: &mut Vec<u16>, value: u32) {
        data.extend([value as u16, (value >> 16) as u16]);
    }
    let mut data = Vec::new();
    dword(
        &mut data,
        WS_POPUP.0 | WS_CAPTION.0 | WS_SYSMENU.0 | DS_MODALFRAME as u32 | DS_SETFONT as u32,
    );
    dword(&mut data, 0);
    data.extend([8, 0, 0, 290, 155, 0, 0]);
    string(
        &mut data,
        crate::ui_text::text("workspace-menu-create-link"),
    );
    data.push(9);
    string(&mut data, "Microsoft YaHei UI");
    let combo = WS_TABSTOP.0 | WS_VSCROLL.0 | CBS_DROPDOWNLIST as u32;
    for (id, class, style, x, y, width, height, text) in [
        (
            201,
            0x82,
            0,
            10,
            12,
            65,
            14,
            if outgoing {
                crate::ui_text::text("workspace-menu-target-window")
            } else {
                crate::ui_text::text("workspace-menu-source-window")
            },
        ),
        (SOURCE as u16, 0x85, combo, 80, 10, 200, 120, ""),
        (
            202,
            0x82,
            0,
            10,
            46,
            65,
            14,
            crate::ui_text::text("workspace-menu-source-port"),
        ),
        (OUTPUT as u16, 0x85, combo, 80, 44, 200, 120, ""),
        (
            203,
            0x82,
            0,
            10,
            80,
            65,
            14,
            crate::ui_text::text("workspace-menu-target-port"),
        ),
        (INPUT as u16, 0x85, combo, 80, 78, 200, 120, ""),
        (
            IDOK.0 as u16,
            0x80,
            WS_TABSTOP.0 | BS_DEFPUSHBUTTON as u32,
            160,
            126,
            55,
            18,
            crate::ui_text::text("workspace-menu-connect"),
        ),
        (
            IDCANCEL.0 as u16,
            0x80,
            WS_TABSTOP.0,
            225,
            126,
            55,
            18,
            crate::ui_text::text("workspace-menu-cancel"),
        ),
    ] {
        if data.len() % 2 != 0 {
            data.push(0);
        }
        dword(&mut data, WS_CHILD.0 | WS_VISIBLE.0 | style);
        dword(&mut data, 0);
        data.extend([x, y, width, height, id, 0xffff, class]);
        string(&mut data, text);
        data.push(0);
    }
    if data.len() % 2 != 0 {
        data.push(0);
    }
    data.chunks_exact(2)
        .map(|v| v[0] as u32 | ((v[1] as u32) << 16))
        .collect()
}
