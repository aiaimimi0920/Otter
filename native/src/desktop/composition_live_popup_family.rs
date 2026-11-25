//! Session 拥有附属弹窗会话：有界发现、非激活菜单镜像、几何跟随和关闭回收。
use super::*;
use crate::live_hover_owner::Control as ParentControl;
#[cfg(test)]
use crate::live_window_target::popup::OwnedPopup;
use crate::live_window_target::{popup::Scope, Target};
use std::{
    rc::Rc,
    time::{Duration, Instant},
};
use windows::Win32::{Foundation::*, UI::WindowsAndMessaging::*};

const LIMIT: usize = 4;
const SCAN_LIMIT: usize = 128;
#[path = "composition_live_popup_decoration.rs"]
mod decoration;
#[derive(Default)]
pub(super) struct Family {
    members: Vec<Member>,
    failed: Vec<Failure>,
    scanned: Option<Instant>,
    // 成员退休不能抹去其输入清理失败；只供父 Session 最终结算，不关闭仍工作的父 B。
    pub(super) input_cleanup_unconfirmed: bool,
}
struct Member {
    hwnd: usize,
    placement: RECT,
    session: Session,
}
struct Failure {
    hwnd: usize,
    scope: Scope,
}
impl Family {
    pub(super) fn completion_handle(&self) -> Option<BorrowedHandle<'_>> {
        self.members
            .iter()
            .find_map(|member| member.session.completion_handle())
    }
    pub(super) fn input_completion_handle(&self) -> Option<BorrowedHandle<'_>> {
        self.members
            .iter()
            .find_map(|member| member.session.input_completion_handle())
    }
    pub(super) fn stop(&mut self) {
        for member in &mut self.members {
            member.session.stop();
        }
    }
    pub(super) fn is_empty(&self) -> bool {
        self.members.is_empty()
    }
    fn retain_failed(&mut self, present: &[usize]) {
        // 只抑制原来源仍有效的失败；lease/父 epoch 换代后，同 HWND 也可重新准入。
        self.failed
            .retain(|failure| present.contains(&failure.hwnd) && failure.scope.target().is_ok());
    }
    fn failed(&self, hwnd: usize) -> bool {
        // 来源已在本轮 retain_failed 一次性复核；不为每个候选重复扫描/验证整套来源链。
        self.failed.iter().any(|failure| failure.hwnd == hwnd)
    }
}
struct Scan {
    windows: Vec<HWND>,
    visited: usize,
}
impl Scan {
    fn include(&mut self, hwnd: HWND) {
        if self.windows.len() < SCAN_LIMIT
            && unsafe { IsWindowVisible(hwnd).as_bool() && !IsIconic(hwnd).as_bool() }
            && !self.windows.contains(&hwnd)
        {
            self.windows.push(hwnd);
        }
    }
}
unsafe extern "system" fn collect(hwnd: HWND, data: LPARAM) -> windows::core::BOOL {
    let scan = &mut *(data.0 as *mut Scan);
    scan.visited += 1;
    scan.include(hwnd);
    windows::core::BOOL::from(scan.visited < SCAN_LIMIT)
}
fn discover(
    root: &Target,
    owner: Option<crate::live_hover_owner::Identity>,
    parents: &[(&Scope, Option<crate::live_hover_owner::Identity>)],
    focused: &[Scope],
) -> windows::core::Result<Vec<Scope>> {
    root.validate()?;
    let thread = unsafe { GetWindowThreadProcessId(root.hwnd(), None) };
    let mut scan = Scan {
        windows: Vec::new(),
        visited: 0,
    };
    // 合法 owned dialog 可以位于另一源线程。仅从已认证焦点/已有成员补候选，
    // 不扫描整个进程；成员优先保留，焦点回到 B 后仍能继续发现跨线程弹窗。
    for scope in parents.iter().map(|(scope, _)| *scope).chain(focused) {
        if scan.visited == SCAN_LIMIT {
            break;
        }
        scan.visited += 1;
        if let Ok(target) = scope.target() {
            scan.include(target.hwnd());
        }
    }
    if scan.visited < SCAN_LIMIT {
        let result = unsafe {
            EnumThreadWindows(thread, Some(collect), LPARAM(&mut scan as *mut _ as isize))
        };
        if scan.visited < SCAN_LIMIT && !result.as_bool() {
            return Err(windows::core::Error::from_win32());
        }
    }
    root.validate()?;
    Ok(scan
        .windows
        .into_iter()
        .filter_map(|hwnd| {
            descendant_scope(hwnd, parents).or_else(|| Scope::bind(root, owner, hwnd).ok())
        })
        // 装饰镜像不能成为独立输入目标，也不能挤占真正菜单/文本弹窗的四个名额。
        .filter(|scope| !decoration::is_decoration(root, owner, scope, parents))
        // PotPlayer 的 1x1 可见 owner 辅助窗没有可交互画面，不能先占满四个采集槽。
        // 每次重新检查尺寸，辅助窗变成真实弹窗时仍可进入；不按类名永久排除。
        .filter(|scope| {
            scope
                .target()
                .and_then(|target| target.frame_bounds())
                .is_ok_and(|r| {
                    i64::from(r.right) - i64::from(r.left) > 1
                        && i64::from(r.bottom) - i64::from(r.top) > 1
                })
        })
        // 发现集合受 128 窗枚举预算约束，不能在此按 Z 序截成会话数量。
        // 否则新弹窗会挤掉仍可见的父成员，stop 后连带撤销子菜单来源链。
        // LIMIT 只在创建 Session 时执行，已有成员先保留其名额。
        .collect())
}

fn descendant_scope(
    hwnd: HWND,
    parents: &[(&Scope, Option<crate::live_hover_owner::Identity>)],
) -> Option<Scope> {
    let mut ancestor = hwnd;
    // 从最近的真实 owner（或精确 lease 的 root）向上查找；不能由成员插入顺序
    // 选择祖父镜像，否则子 B 的 owner 链绕过父 B，会被 native_nested 正确拒绝。
    for _ in 0..32 {
        ancestor = unsafe { GetWindow(ancestor, GW_OWNER) }
            .ok()
            .filter(|owner| !owner.0.is_null())
            .or_else(|| {
                otter_live_hover_hook::native_popup::Lease::read(ancestor).map(|lease| lease.root)
            })?;
        if ancestor == hwnd {
            return None;
        }
        for (parent, mirror) in parents {
            if parent
                .target()
                .is_ok_and(|target| target.hwnd() == ancestor)
            {
                if let Ok(scope) = parent.bind_descendant(*mirror, hwnd) {
                    return Some(scope);
                }
            }
        }
    }
    None
}

impl Session {
    pub(in crate::desktop) fn popup_layer_leases(
        &self,
        leases: &mut Vec<otter_live_hover_hook::native_popup::Lease>,
    ) {
        if self.stopping
            || self.remote_input.is_some()
            || self
                .window
                .as_ref()
                .is_none_or(|w| !w.popup_visible() || !w.popup_control_allowed())
        {
            return;
        }
        leases.extend(
            self.popups
                .members
                .iter()
                .filter(|m| {
                    !m.session.stopping
                        && m.session
                            .window
                            .as_ref()
                            .is_some_and(LiveWindow::popup_visible)
                })
                .filter_map(|m| m.session.popup_scope.as_ref()?.layer_lease()),
        );
    }
    pub(super) fn poll_popups(&mut self) {
        // 附属会话不递归发现其他窗口；失去原归属或隐藏就是正常关闭。
        if let Some(scope) = &self.popup_scope {
            if scope.target().is_err()
                || self
                    .target
                    .as_ref()
                    .is_none_or(|target| unsafe { !IsWindowVisible(target.hwnd()).as_bool() })
            {
                self.stop();
            }
            return;
        }
        if self.stopping
            || self.remote_input.is_some()
            || self
                .window
                .as_ref()
                .is_none_or(|window| !window.popup_visible())
        {
            self.popups.stop();
        }
        let control = self
            .window
            .as_ref()
            .filter(|window| {
                !self.stopping && self.remote_input.is_none() && window.popup_visible()
            })
            .and_then(LiveWindow::popup_control_grant);
        for member in &self.popups.members {
            if !member.session.stopping {
                if let Some(mirror) = &member.session.window {
                    mirror.sync_popup_control(control);
                }
            }
        }
        let cleanup_unconfirmed = &mut self.popups.input_cleanup_unconfirmed;
        self.popups
            .members
            .retain_mut(|member| match member.session.poll() {
                None => true,
                Some(result) => {
                    *cleanup_unconfirmed |= member.session.input_cleanup_unconfirmed();
                    if let Err(error) = result {
                        eprintln!("native-live-popup-end: {error}");
                    }
                    false
                }
            });
        let Some(window) = self.window.as_ref().filter(|window| window.popup_visible()) else {
            return;
        };
        if self.stopping || self.remote_input.is_some() {
            return;
        }
        if self
            .popups
            .scanned
            .is_some_and(|at| at.elapsed() < Duration::from_millis(80))
        {
            return;
        }
        self.popups.scanned = Some(Instant::now());
        let Some(root) = &self.target else {
            return;
        };
        // 本轮普通应用接线不改变已经独立验证的 Chromium 输入/弹窗路径。
        if crate::live_browser_scroll::eligible(root) {
            return;
        }
        let Some(output) = window.popup_image_rect() else {
            return;
        };
        let Ok(source) = root
            .frame_bounds()
            .and_then(|frame| self.input_crop.map_or(Ok(frame), |crop| crop.screen(frame)))
        else {
            // 没有可映射的源矩形时，不枚举弹窗；已存在的成员仍在本轮上方结算。
            return;
        };
        let parents: Vec<_> = self
            .popups
            .members
            .iter()
            .filter(|member| !member.session.stopping)
            .filter_map(|member| {
                Some((
                    member.session.popup_scope.as_ref()?,
                    member.session.window.as_ref()?.hover_identity(),
                ))
            })
            .collect();
        let focused: Vec<_> = std::iter::once(window)
            .chain(
                self.popups
                    .members
                    .iter()
                    .filter(|m| !m.session.stopping)
                    .filter_map(|m| m.session.window.as_ref()),
            )
            .filter_map(LiveWindow::focused_source_popup)
            .collect();
        let Ok(popups) = discover(root, window.hover_identity(), &parents, &focused) else {
            return;
        };
        let present: Vec<_> = popups
            .iter()
            .filter_map(|p| p.target().ok())
            .map(|t| t.hwnd().0 as usize)
            .collect();
        self.popups.retain_failed(&present);
        for member in &mut self.popups.members {
            if !present.contains(&member.hwnd) {
                member.session.stop();
            }
        }
        for scope in popups.into_iter().rev() {
            let Ok(target) = scope.target() else {
                continue;
            };
            let hwnd = target.hwnd().0 as usize;
            let Ok(bounds) = target.frame_bounds() else {
                continue;
            };
            let Some(placement) = map_rect(source, output, bounds) else {
                continue;
            };
            if let Some(member) = self.popups.members.iter_mut().find(|m| m.hwnd == hwnd) {
                // 父子可能在同一轮首次出现；下一轮才有父镜像可供绑定。
                // 只更新仍属于原显示实例的镜像，不能凭相同 HWND 复活旧输入状态。
                match member
                    .session
                    .rebind_popup_scope(scope, window.popup_hwnd())
                {
                    Ok(true) => {}
                    Ok(false) => continue,
                    Err(error) => {
                        eprintln!("native-live-popup-owner: {error}");
                        member.session.stop();
                        continue;
                    }
                }
                if member.placement != placement {
                    if let Some(mirror) = &member.session.window {
                        if let Err(error) = mirror.popup_place(placement) {
                            eprintln!("native-live-popup-place: {error}");
                            member.session.stop();
                        } else {
                            member.placement = placement;
                        }
                    }
                }
                continue;
            }
            if self.popups.members.len() >= LIMIT || self.popups.failed(hwnd) {
                continue;
            }
            let result = Self::start_popup(
                scope.clone(),
                placement,
                window.popup_hwnd(),
                window.popup_control_grant(),
            );
            match result {
                Ok(session) => self.popups.members.push(Member {
                    hwnd,
                    placement,
                    session,
                }),
                Err(error) => {
                    self.popups.failed.push(Failure { hwnd, scope });
                    eprintln!("native-live-popup-start: {error}");
                }
            }
        }
        // HWND 全部来自本 Session 持有的镜像，不接受进程枚举结果作为焦点豁免。
        let mut peers = vec![window.popup_hwnd()];
        peers.extend(
            self.popups
                .members
                .iter()
                .filter_map(|m| m.session.window.as_ref())
                .map(LiveWindow::popup_hwnd),
        );
        window.popup_peers(peers.clone());
        let scopes: Vec<_> = self
            .popups
            .members
            .iter()
            .filter(|member| !member.session.stopping)
            .filter_map(|member| member.session.popup_scope.clone())
            .collect();
        // 焦点核验可在窗口间复用不可变快照，不为每个镜像递归克隆 Scope。
        let scopes = (!scopes.is_empty()).then(|| Rc::new(scopes));
        window.popup_scopes(scopes.clone());
        for member in &self.popups.members {
            if let Some(mirror) = &member.session.window {
                mirror.popup_peers(peers.clone());
                mirror.popup_scopes(scopes.clone());
                mirror.sync_popup_control(window.popup_control_grant());
            }
        }
    }
    fn rebind_popup_scope(
        &mut self,
        scope: Scope,
        fallback_owner: HWND,
    ) -> windows::core::Result<bool> {
        if self.stopping {
            return Ok(false);
        }
        let previous = self
            .popup_scope
            .as_ref()
            .expect("Live popup member scope missing")
            .clone();
        // member.poll 与 discover 之间源可隐藏/重开同一个 HWND；不能以新 scope
        // 覆盖旧实例的撤销证据，也不能在旧 worker 释放完成前复用其窗口/队列。
        if previous.target().is_err() || scope.target().is_err() || self.window.is_none() {
            self.stop();
            return Ok(false);
        }
        let mirror = self.window.as_ref().unwrap();
        mirror.reparent_popup(scope.mirror_owner().unwrap_or(fallback_owner))?;
        // 重挂最终调用 SetWindowPos，可同步重入；提交前仍验原 scope，而非只验新来源。
        if !mirror.is_alive() || previous.target().is_err() || scope.target().is_err() {
            self.stop();
            return Ok(false);
        }
        self.popup_scope = Some(scope);
        Ok(true)
    }
    fn start_popup(
        scope: Scope,
        screen: RECT,
        owner: HWND,
        control: Option<ParentControl>,
    ) -> io::Result<Self> {
        let target = scope.target().map_err(io::Error::other)?;
        // 多层 lease 的镜像沿实际父 B 建立 owner 链，与源端嵌套输入校验保持一致。
        let owner = scope.mirror_owner().unwrap_or(owner);
        let mut class = [0u16; 64];
        let len = unsafe { GetClassNameW(target.hwnd(), &mut class) } as usize;
        let noactivate = unsafe { GetWindowLongPtrW(target.hwnd(), GWL_EXSTYLE) }
            & WS_EX_NOACTIVATE.0 as isize
            != 0
            || String::from_utf16_lossy(&class[..len]) == "#32768";
        let (job, window) = LiveJob::start_window_region_bound(target.clone(), None, |consumer| {
            let window = LiveWindow::new_at(consumer, screen).map_err(io::Error::other)?;
            window
                .configure_popup(owner, noactivate)
                .map_err(io::Error::other)?;
            window.popup_place(screen).map_err(io::Error::other)?;
            Ok(window)
        })?;
        let mut session = Self::new(job, window);
        session.target = Some(target);
        session.popup_scope = Some(scope);
        let window = session.window.as_ref().unwrap();
        if control.is_some_and(|control| control.valid()) {
            window.allow_input().map_err(io::Error::other)?;
        }
        window.sync_popup_control(control);
        window.show().map_err(io::Error::other)?;
        Ok(session)
    }
}

fn map_rect(source: RECT, output: RECT, popup: RECT) -> Option<RECT> {
    let map = |v: i32, a: i32, b: i32, x: i32, y: i32| {
        let span = i64::from(b) - i64::from(a);
        if span <= 0 || y <= x {
            return None;
        }
        let result = f64::from(x)
            + (f64::from(v) - f64::from(a)) * (f64::from(y) - f64::from(x)) / span as f64;
        (result >= i32::MIN as f64 && result <= i32::MAX as f64).then(|| result.round() as i32)
    };
    let rect = RECT {
        left: map(
            popup.left,
            source.left,
            source.right,
            output.left,
            output.right,
        )?,
        right: map(
            popup.right,
            source.left,
            source.right,
            output.left,
            output.right,
        )?,
        top: map(
            popup.top,
            source.top,
            source.bottom,
            output.top,
            output.bottom,
        )?,
        bottom: map(
            popup.bottom,
            source.top,
            source.bottom,
            output.top,
            output.bottom,
        )?,
    };
    (rect.right > rect.left && rect.bottom > rect.top).then_some(rect)
}

#[cfg(test)]
#[path = "composition_live_popup_family_tests.rs"]
mod tests;
