//! Synchronize native accessibility names after mini-ui releases its control borrows.
use neuro_mini_ui::{PropertyField, PropertyKind};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
use windows::{
    core::{Error, Result, HSTRING},
    Win32::{Foundation::*, UI::WindowsAndMessaging::*},
};

thread_local! {
    static ACTIVE: RefCell<Vec<(HWND, Rc<Cell<bool>>, Captions)>> = const { RefCell::new(Vec::new()) };
}
enum Entry {
    Fresh(Guard),
    Identical,
    Superseded,
}
struct Guard(HWND, Rc<Cell<bool>>);
impl Guard {
    fn enter(root: HWND, captions: &Captions) -> Entry {
        ACTIVE.with(|active| {
            let mut active = active.borrow_mut();
            if let Some((_, valid, pending)) = active.iter().find(|(hwnd, _, _)| *hwnd == root) {
                if valid.get() && pending == captions {
                    return Entry::Identical;
                }
                valid.set(false);
                return Entry::Superseded;
            }
            let valid = Rc::new(Cell::new(true));
            active.push((root, valid.clone(), captions.clone()));
            Entry::Fresh(Self(root, valid))
        })
    }
}
impl Drop for Guard {
    fn drop(&mut self) {
        ACTIVE.with(|active| {
            active
                .borrow_mut()
                .retain(|(root, valid, _)| *root != self.0 || !Rc::ptr_eq(valid, &self.1));
        });
    }
}
#[derive(Clone, PartialEq)]
pub(super) struct Captions {
    language: crate::ui_text::DisplayLanguage,
    labels: Vec<(i32, &'static str)>,
}
impl Captions {
    pub(super) fn new<'a>(fields: impl Iterator<Item = &'a PropertyField>) -> Self {
        Self {
            language: crate::ui_text::current_language(),
            labels: fields
                .filter_map(|field| {
                    (!matches!(
                        field.kind,
                        PropertyKind::Number { .. }
                            | PropertyKind::Text { .. }
                            | PropertyKind::ReadOnly { .. }
                    ))
                    .then_some((i32::from(field.id) * 4, field.label))
                })
                .collect(),
        }
    }
    /// The caller's predicate briefly checks owner/session identity, never holding a borrow.
    pub(super) unsafe fn sync(
        &self,
        root: HWND,
        owner: HWND,
        current: impl Fn() -> bool,
    ) -> Result<bool> {
        if !current() {
            return Ok(false);
        }
        let guard = match Guard::enter(root, self) {
            Entry::Fresh(guard) => guard,
            Entry::Identical => return Ok(true),
            Entry::Superseded => {
                // A different reentrant plan must run after the in-flight WM_SETTEXT returns.
                PostMessageW(Some(owner), WM_NULL, WPARAM(0), LPARAM(0))?;
                return Ok(false);
            }
        };
        let valid =
            || guard.1.get() && crate::ui_text::current_language() == self.language && current();
        if !valid() {
            return Ok(false);
        }
        let mut children = Vec::<HWND>::new();
        unsafe extern "system" fn collect(hwnd: HWND, data: LPARAM) -> windows::core::BOOL {
            (&mut *(data.0 as *mut Vec<HWND>)).push(hwnd);
            true.into()
        }
        let _ = EnumChildWindows(
            Some(root),
            Some(collect),
            LPARAM(&mut children as *mut _ as isize),
        );
        for &(id, label) in &self.labels {
            if !valid() {
                return Ok(false);
            }
            let Some(hwnd) = children
                .iter()
                .copied()
                .find(|hwnd| IsChild(root, *hwnd).as_bool() && GetDlgCtrlID(*hwnd) == id)
            else {
                return Err(Error::new(E_HANDLE, "Property caption control is missing"));
            };
            let mut class = [0u16; 32];
            let length = GetClassNameW(hwnd, &mut class) as usize;
            let class = String::from_utf16_lossy(&class[..length]);
            if !class.eq_ignore_ascii_case("Button") && !class.eq_ignore_ascii_case("Static") {
                return Err(Error::new(
                    E_INVALIDARG,
                    "Property caption is not a display control",
                ));
            }
            let parent = GetParent(hwnd)?;
            let identity = GetWindowLongPtrW(hwnd, GWLP_USERDATA);
            let owns = || {
                valid()
                    && IsChild(root, hwnd).as_bool()
                    && GetParent(hwnd).ok() == Some(parent)
                    && GetDlgCtrlID(hwnd) == id
                    && GetWindowLongPtrW(hwnd, GWLP_USERDATA) == identity
            };
            let expected: Vec<_> = label.encode_utf16().collect();
            let mut text = vec![0; expected.len() + 2];
            let length = GetWindowTextW(hwnd, &mut text) as usize;
            if !owns() {
                return Ok(false);
            }
            if text[..length] != expected {
                let result = SetWindowTextW(hwnd, &HSTRING::from(label));
                // A synchronous callback can close or refresh the owner before returning.
                if !owns() {
                    return Ok(false);
                }
                result?;
            }
        }
        Ok(valid())
    }
}
