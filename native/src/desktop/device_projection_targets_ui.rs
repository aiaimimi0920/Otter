//! Send catalogs/single-target actions; offline entry still uses locally paired Loom, without reusing receive consent or guessing remote addresses.
use super::{Request, View as AuthorizationView};
use crate::projection_delivery::targets::{Directory, Policy, Route, Status};
use windows::{
    core::{Error, Result, HSTRING},
    Win32::{Foundation::E_ABORT, UI::WindowsAndMessaging::*},
};
pub(in crate::desktop) const REFRESH: usize = 0x1600;
#[derive(Default)]
pub(in crate::desktop) struct View {
    pub automatic: bool,
    pub next: Option<std::time::Instant>,
    pub directory: Option<std::sync::Arc<Directory>>,
    pub busy: bool,
    pub status: String,
}
fn available(view: &AuthorizationView) -> bool {
    !view.closed
        && !view.busy
        && !view.projection.busy
        && !view.targets.busy
        && view.request.is_none()
        && view.origin.is_some()
}
pub(in crate::desktop) fn request(view: &AuthorizationView) -> Result<Request> {
    if !available(view) {
        return Err(Error::new(
            E_ABORT,
            "Projection target directory unavailable",
        ));
    }
    Ok(Request::ProjectionTargets)
}
pub(in crate::desktop) fn reset(view: &mut AuthorizationView) {
    view.targets = View {
        automatic: view.targets.automatic,
        ..View::default()
    };
    if matches!(view.request, Some(Request::ProjectionTargets)) {
        view.request = None;
    }
}
struct Menu(HMENU);
impl Drop for Menu {
    fn drop(&mut self) {
        unsafe {
            let _ = DestroyMenu(self.0);
        }
    }
}
fn label(value: &str) -> String {
    let shortened: String = value.chars().take(64).collect();
    format!(
        "{shortened}{}",
        if value.chars().count() > 64 {
            "…"
        } else {
            ""
        }
    )
    .replace('&', "&&")
}
pub(super) unsafe fn append_menu(
    parent: HMENU,
    view: &AuthorizationView,
    unit: &crate::workspace_id::UnitId,
) -> Result<()> {
    let menu = Menu(CreatePopupMenu()?);
    AppendMenuW(
        menu.0,
        MF_STRING
            | if available(view) {
                MF_ENABLED
            } else {
                MF_GRAYED
            },
        REFRESH,
        &windows::core::HSTRING::from(crate::ui_text::text("integration-refresh-targets")),
    )?;
    if !view.targets.status.is_empty() {
        AppendMenuW(
            menu.0,
            MF_STRING | MF_GRAYED,
            0,
            &HSTRING::from(label(&view.targets.status)),
        )?;
    }
    if !view.sender.status.is_empty() {
        AppendMenuW(
            menu.0,
            MF_STRING | MF_GRAYED,
            0,
            &HSTRING::from(label(&view.sender.status)),
        )?;
    }
    if view.sender.group_unit.as_ref() == Some(unit) {
        for result in &view.sender.group_results {
            AppendMenuW(
                menu.0,
                MF_STRING | MF_GRAYED,
                0,
                &HSTRING::from(label(result)),
            )?;
        }
    }
    if view.sender.cancelable {
        AppendMenuW(
            menu.0,
            MF_STRING,
            super::sender::CANCEL,
            &HSTRING::from(if view.sender.group_stopping {
                crate::ui_text::text("integration-cancel-group-stop")
            } else {
                crate::ui_text::text("integration-cancel-send")
            }),
        )?;
    }
    for (index, binding) in super::sender::stops(view, unit).enumerate() {
        AppendMenuW(
            menu.0,
            MF_STRING
                | if super::sender::stop_enabled(view, binding) {
                    MF_ENABLED
                } else {
                    MF_GRAYED
                },
            super::sender::STOP_BASE + index,
            &HSTRING::from(crate::ui_text::format(
                "integration-stop-delivery",
                &[
                    (
                        "name",
                        label(if binding.name().is_empty() {
                            binding
                                .target()
                                .unwrap_or(crate::ui_text::text("integration-share-invitation"))
                        } else {
                            binding.name()
                        })
                        .into(),
                    ),
                    (
                        "stopping",
                        if binding.stopped() { "yes" } else { "no" }.into(),
                    ),
                ],
            )),
        )?;
    }
    if view.sender.retry.as_ref() == Some(unit) {
        AppendMenuW(
            menu.0,
            MF_STRING,
            super::sender::RETRY,
            &windows::core::HSTRING::from(crate::ui_text::text("integration-retry-invitation")),
        )?;
    }
    if let Some(directory) = view
        .targets
        .directory
        .as_ref()
        .filter(|directory| view.origin.as_deref() == Some(directory.origin()))
    {
        for (index, target) in directory.targets().iter().enumerate() {
            let route = match target.route() {
                Route::SharedLoom => crate::ui_text::text("integration-same-loom").to_owned(),
                Route::OfflinePeer {
                    advertised_available: true,
                    peer_name,
                    ..
                } => crate::ui_text::format(
                    "integration-offline-peer",
                    &[("name", label(peer_name).into())],
                ),
                Route::OfflinePeer {
                    advertised_available: false,
                    ..
                } => crate::ui_text::text("integration-offline-unavailable").to_owned(),
            };
            let policy = match target.policy() {
                Policy::Auto => crate::ui_text::text("integration-automatic"),
                Policy::Confirm => crate::ui_text::text("integration-receive"),
            };
            let name = if target.name().is_empty() {
                target.id()
            } else {
                target.name()
            };
            AppendMenuW(
                menu.0,
                MF_STRING
                    | if super::sender::enabled(view, unit, index) {
                        MF_ENABLED
                    } else {
                        MF_GRAYED
                    }
                    | if super::sender::bound(view, unit, target.id()) {
                        MF_CHECKED
                    } else {
                        MENU_ITEM_FLAGS(0)
                    },
                super::sender::BASE + index,
                &HSTRING::from(format!("{} · {policy} · {route}", label(name))),
            )?;
        }
        for (index, group) in directory.groups().into_iter().flatten().enumerate() {
            AppendMenuW(
                menu.0,
                MF_STRING
                    | if super::sender::groups::enabled(view, unit, index) {
                        MF_ENABLED
                    } else {
                        MF_GRAYED
                    },
                super::sender::groups::BASE + index,
                &HSTRING::from(crate::ui_text::format(
                    "integration-send-group",
                    &[
                        ("name", label(group.name()).into()),
                        ("count", group.target_ids().len().into()),
                        ("unavailable", group.unavailable_count().into()),
                    ],
                )),
            )?;
            AppendMenuW(
                menu.0,
                MF_STRING
                    | if super::sender::groups::stop_enabled(view, unit, index) {
                        MF_ENABLED
                    } else {
                        MF_GRAYED
                    },
                super::sender::groups::STOP_BASE + index,
                &HSTRING::from(crate::ui_text::format(
                    "integration-stop-group",
                    &[("name", label(group.name()).into())],
                )),
            )?;
        }
    }
    AppendMenuW(
        parent,
        MF_POPUP,
        menu.0 .0 as usize,
        &windows::core::HSTRING::from(crate::ui_text::text("integration-delivery-targets")),
    )?;
    // The parent menu takes ownership only after successful attachment; local owners release every construction failure.
    std::mem::forget(menu);
    Ok(())
}
pub(in crate::desktop) fn status(directory: &Directory) -> String {
    let suffix = match directory.status() {
        Status::Complete => "",
        Status::Partial => crate::ui_text::text("integration-offline-directory-partial"),
        Status::Busy => crate::ui_text::text("integration-offline-directory-busy"),
        Status::Unavailable => crate::ui_text::text("integration-offline-directory-unavailable"),
    };
    crate::ui_text::format(
        "integration-target-count",
        &[
            ("count", directory.targets().len().into()),
            ("suffix", suffix.into()),
        ],
    )
}
