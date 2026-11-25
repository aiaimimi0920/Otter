//! Empty-workspace Live reception uses the ordinary directory and authorization request slot.
use super::*;

pub(in crate::desktop) const WATCH_DIRECT: usize = STOP_BASE + MAX_VIEWERS + 1;

fn catalog(view: &AuthorizationView) -> Option<&crate::live_relay_catalog_runtime::Catalog> {
    view.catalog.as_ref().filter(|catalog| {
        !view.closed
            && !view.busy
            && !view.catalog_busy
            && view.request.is_none()
            && view.origin.as_deref() == Some(catalog.origin.as_str())
    })
}
fn eligible(entry: &crate::live_relay_http::CatalogEntry, device: &str) -> bool {
    entry.direct_view && entry.identity.source_device() != device
}
pub(super) fn current(view: &AuthorizationView, selected: &Selection, device: &str) -> bool {
    catalog(view).is_some_and(|catalog| {
        catalog.origin == selected.origin
            && catalog.device_id == device
            && catalog
                .entries
                .iter()
                .any(|entry| eligible(entry, device) && entry.identity == selected.identity)
    }) && !duplicate(view, &selected.origin, selected.identity.session())
}
pub(in crate::desktop::device_authorization_ui) unsafe fn append(
    menu: HMENU,
    view: &AuthorizationView,
) -> Result<()> {
    let ready = catalog(view).is_some_and(|catalog| {
        catalog.entries.iter().any(|entry| {
            eligible(entry, &catalog.device_id)
                && !duplicate(view, &catalog.origin, entry.identity.session())
        })
    });
    AppendMenuW(
        menu,
        MF_STRING | if ready { MF_ENABLED } else { MF_GRAYED },
        WATCH_DIRECT,
        &windows::core::HSTRING::from(crate::ui_text::text("integration-watch-live-projection")),
    )
}
pub(in crate::desktop::device_authorization_ui) unsafe fn choose(
    hwnd: HWND,
    state: &State,
    tray: bool,
) -> Result<()> {
    let selections = {
        let view = state.authorization.borrow();
        let catalog =
            catalog(&view).ok_or_else(|| Error::new(E_ABORT, "Live directory unavailable"))?;
        catalog
            .entries
            .iter()
            .filter(|entry| {
                eligible(entry, &catalog.device_id)
                    && !duplicate(&view, &catalog.origin, entry.identity.session())
            })
            .map(|entry| Selection {
                origin: catalog.origin.clone(),
                identity: entry.identity.clone(),
                binding: ViewerHost::Direct {
                    origin: catalog.origin.clone(),
                    device: catalog.device_id.clone(),
                },
            })
            .collect::<Vec<_>>()
    };
    if selections.is_empty() || selections.len() > MAX_CATALOG {
        return Ok(());
    }
    let menu = Menu(CreatePopupMenu()?);
    for (index, selection) in selections.iter().enumerate() {
        AppendMenuW(
            menu.0,
            MF_STRING,
            index + 1,
            &HSTRING::from(format!(
                "{} · {}",
                label(selection.identity.source_device()),
                label(selection.identity.session())
            )),
        )?;
    }
    let mut point = POINT::default();
    GetCursorPos(&mut point)?;
    let picked = if tray {
        let Some(_modal) = crate::desktop::file_dialog::Guard::begin(&state.file_dialog) else {
            return Ok(());
        };
        crate::native_menu::cascade::track_tray(hwnd, crate::native_menu::snapshot(menu.0)?, point)?
    } else {
        Some(crate::desktop::popup_menu::track(
            hwnd,
            state,
            menu.0,
            point,
            || true,
        )?)
    };
    if let Some(index) = picked.and_then(|id| id.checked_sub(1)) {
        let selected = selections
            .into_iter()
            .nth(index)
            .ok_or_else(|| Error::new(E_ABORT, "Live selection expired"))?;
        if state.lifetime.owns(hwnd) {
            select(hwnd, state, selected)?;
        }
    }
    Ok(())
}
