//! Session owns attached popup sessions: bounded discovery, nonactivating menu mirrors, geometry tracking, and close reclamation.
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
    // Member retirement must not erase input-cleanup failures; these are only for final parent Session settlement, not closing a working parent B.
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
        // Suppress failures only while original provenance remains valid; lease/parent epoch changes permit readmission even for the same HWND.
        self.failed
            .retain(|failure| present.contains(&failure.hwnd) && failure.scope.target().is_ok());
    }
    fn failed(&self, hwnd: usize) -> bool {
        // This round's retain_failed already revalidated provenance once; do not rescan/revalidate the full chain for every candidate.
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
    // Valid owned dialogs may belong to another source thread. Add candidates only from authenticated focus/existing members,
    // without scanning the entire process; retain members first so cross-thread popups remain discoverable after focus returns to B.
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
        // Decoration mirrors must not become independent input targets or consume the four slots reserved for real menus/text popups.
        .filter(|scope| !decoration::is_decoration(root, owner, scope, parents))
        // PotPlayer's visible 1x1 owner helper windows have no interactive content and must not fill all four capture slots first.
        // Recheck size each time so helpers can enter when they become real popups; never exclude permanently by class name.
        .filter(|scope| {
            scope
                .target()
                .and_then(|target| target.frame_bounds())
                .is_ok_and(|r| {
                    i64::from(r.right) - i64::from(r.left) > 1
                        && i64::from(r.bottom) - i64::from(r.top) > 1
                })
        })
        // Discovery is bounded by a 128-window enumeration budget; do not truncate it by Z order to the session count here.
        // Otherwise new popups displace still-visible parent members, and stopping them also revokes submenu provenance chains.
        // Apply LIMIT only when creating Sessions; existing members retain their slots first.
        .collect())
}

fn descendant_scope(
    hwnd: HWND,
    parents: &[(&Scope, Option<crate::live_hover_owner::Identity>)],
) -> Option<Scope> {
    let mut ancestor = hwnd;
    // Search upward from the nearest real owner, or the exact lease's root; member insertion order must not
    // select a grandparent mirror, or child B's owner chain bypasses parent B and native_nested correctly rejects it.
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
        // Attached sessions do not recursively discover other windows; losing original ownership or becoming hidden is normal closure.
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
        // This ordinary-application wiring does not change the independently verified Chromium input/popup paths.
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
            // Without a mappable source rectangle, skip popup enumeration; existing members still settle above this round.
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
                // Parent and child may first appear in the same round; the parent mirror is available for binding only next round.
                // Update only mirrors still belonging to the original display instance; matching HWNDs cannot revive stale input state.
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
        // Every HWND comes from mirrors owned by this Session; process enumeration is not evidence for focus exemptions.
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
        // Focus checks may reuse immutable snapshots across windows rather than recursively cloning Scope for each mirror.
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
        // Between member.poll and discover, sources may hide/reopen the same HWND; new scope must not
        // overwrite the old instance's revocation evidence or reuse its window/queue before the old worker finishes releasing.
        if previous.target().is_err() || scope.target().is_err() || self.window.is_none() {
            self.stop();
            return Ok(false);
        }
        let mirror = self.window.as_ref().unwrap();
        mirror.reparent_popup(scope.mirror_owner().unwrap_or(fallback_owner))?;
        // Reattachment ultimately calls SetWindowPos and may reenter synchronously; still validate original scope before commit, not merely new provenance.
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
        // Multilevel lease mirrors establish owner chains through actual parent B, matching source-side nested-input validation.
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
