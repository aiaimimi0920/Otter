//! 发送目录和单目标动作；离线入口仍走本机配对Loom，不复用接收同意或猜测远端地址。
use super::{Request, View as AuthorizationView};
use crate::projection_delivery::targets::{Directory, Policy, Route, Status};
use windows::{
    core::{w, Error, Result, HSTRING},
    Win32::{Foundation::E_ABORT, UI::WindowsAndMessaging::*},
};
pub(in crate::desktop) const REFRESH: usize = 0x1600;
#[derive(Default)]
pub(in crate::desktop) struct View {
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
pub(super) fn request(view: &AuthorizationView) -> Result<Request> {
    if !available(view) {
        return Err(Error::new(
            E_ABORT,
            "Projection target directory unavailable",
        ));
    }
    Ok(Request::ProjectionTargets)
}
pub(in crate::desktop) fn reset(view: &mut AuthorizationView) {
    view.targets = View::default();
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
        w!("刷新投送目标"),
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
            if view.sender.group_stopping {
                w!("取消未受理的组停止")
            } else {
                w!("取消当前发送")
            },
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
            &HSTRING::from(format!(
                "停止投送：{}{}",
                label(if binding.name().is_empty() {
                    binding.target()
                } else {
                    binding.name()
                }),
                if binding.stopped() {
                    " · 停止中"
                } else {
                    ""
                }
            )),
        )?;
    }
    if view.sender.retry.as_ref() == Some(unit) {
        AppendMenuW(
            menu.0,
            MF_STRING,
            super::sender::RETRY,
            w!("重试原投送邀请"),
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
                Route::SharedLoom => "同一 Loom".to_owned(),
                Route::OfflinePeer {
                    advertised_available: true,
                    peer_name,
                    ..
                } => format!("离线互信 · {}", label(peer_name)),
                Route::OfflinePeer {
                    advertised_available: false,
                    ..
                } => "离线不可用".to_owned(),
            };
            let policy = match target.policy() {
                Policy::Auto => "自动",
                Policy::Confirm => "手动",
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
                &HSTRING::from(format!(
                    "发送组：{} · {} 项 · {} 不可用",
                    label(group.name()),
                    group.target_ids().len(),
                    group.unavailable_count()
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
                &HSTRING::from(format!("停止组：{}", label(group.name()))),
            )?;
        }
    }
    AppendMenuW(parent, MF_POPUP, menu.0 .0 as usize, w!("投送目标"))?;
    // 只有成功挂接后父菜单才接管；任一构建失败都由局部 owner 释放。
    std::mem::forget(menu);
    Ok(())
}
pub(in crate::desktop) fn status(directory: &Directory) -> String {
    let suffix = match directory.status() {
        Status::Complete => "",
        Status::Partial => " · 离线目录不完整",
        Status::Busy => " · 离线目录忙",
        Status::Unavailable => " · 离线目录不可用",
    };
    format!("投送目标：{} 项{suffix}", directory.targets().len())
}
