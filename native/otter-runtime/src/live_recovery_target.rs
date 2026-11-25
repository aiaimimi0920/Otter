//! Bind recovery records to real source processes; accept only records from authenticated parent channels, since serialized data itself is not authorization.
use super::{
    snapshot::{Layered, Snapshot},
    Target,
};
use crate::live_recovery_wire::{Layered as WireLayered, Record, Restoration, WindowState};
#[path = "live_recovery_popup_layer.rs"]
pub mod popup_layer;
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
use windows::{
    core::{Error, Result},
    Win32::{Foundation::*, System::Threading::*, UI::WindowsAndMessaging::*},
};

fn creation_time(target: &Target) -> Result<u64> {
    target.validate()?;
    unsafe {
        // Request minimal query permissions separately, without expanding admission permissions for ordinary Live capture Targets.
        let raw = OpenProcess(
            PROCESS_QUERY_LIMITED_INFORMATION,
            false,
            target.identity.process,
        )?;
        let process = OwnedHandle::from_raw_handle(raw.0);
        let mut created = FILETIME::default();
        let mut exited = FILETIME::default();
        let mut kernel = FILETIME::default();
        let mut user = FILETIME::default();
        GetProcessTimes(
            HANDLE(process.as_raw_handle()),
            &mut created,
            &mut exited,
            &mut kernel,
            &mut user,
        )?;
        // The original Target's synchronization handle remains alive; exit during queries must not allow replacement by a new PID.
        target.validate()?;
        Ok((u64::from(created.dwHighDateTime) << 32) | u64::from(created.dwLowDateTime))
    }
}
fn rect(value: [i32; 4]) -> RECT {
    RECT {
        left: value[0],
        top: value[1],
        right: value[2],
        bottom: value[3],
    }
}
fn rect_array(value: RECT) -> [i32; 4] {
    [value.left, value.top, value.right, value.bottom]
}
fn point(value: [i32; 2]) -> POINT {
    POINT {
        x: value[0],
        y: value[1],
    }
}

impl Snapshot {
    pub fn recovery_record(&self) -> Result<Record> {
        make_record(
            &self.target,
            Restoration::Window(WindowState {
                rect: rect_array(self.rect),
                placement_flags: self.placement.flags.0,
                show_command: self.placement.showCmd,
                min_position: [
                    self.placement.ptMinPosition.x,
                    self.placement.ptMinPosition.y,
                ],
                max_position: [
                    self.placement.ptMaxPosition.x,
                    self.placement.ptMaxPosition.y,
                ],
                normal_position: rect_array(self.placement.rcNormalPosition),
                extended_style: self.extended_style as i64,
                layered: self.layered.map(|value| WireLayered {
                    color: value.color.0,
                    alpha: value.alpha,
                    flags: value.flags.0,
                }),
                iconic: self.iconic,
                visible: self.visible,
            }),
        )
    }
}
fn make_record(target: &Target, restoration: Restoration) -> Result<Record> {
    let record = Record {
        hwnd: target.identity.window as u64,
        process_id: target.identity.process,
        thread_id: target.identity.thread,
        process_created: creation_time(target)?,
        restoration,
    };
    record
        .validate()
        .map_err(|error| Error::new(E_INVALIDARG, error.to_string()))?;
    Ok(record)
}

/// Establish and retain this owner before registration ACK; recovery uses only this Target, without reopening the source process by PID.
pub struct Binding {
    state: Bound,
}
enum Bound {
    Window(Snapshot),
    // The request channel contains original identity and the in-flight slot; allocate only during registration, without expanding ordinary window-recovery records.
    PopupLayer(Box<popup_layer::Recovery>),
}
impl Binding {
    pub fn bind(record: &Record) -> Result<Self> {
        record
            .validate()
            .map_err(|error| Error::new(E_INVALIDARG, error.to_string()))?;
        let window = usize::try_from(record.hwnd)
            .map_err(|_| Error::new(E_INVALIDARG, "Live recovery HWND is out of range"))?;
        let target = Target::bind(HWND(window as *mut _))?;
        if target.identity.process != record.process_id
            || target.identity.thread != record.thread_id
            || creation_time(&target)? != record.process_created
        {
            return Err(Error::new(
                E_ABORT,
                "Live recovery source identity mismatch",
            ));
        }
        let state = match &record.restoration {
            Restoration::Window(state) => Bound::Window(snapshot(target, state)?),
            Restoration::PopupLayer(layer) => {
                Bound::PopupLayer(Box::new(popup_layer::Recovery::bind(target, layer)?))
            }
        };
        Ok(Self { state })
    }
    pub fn restore(&self) -> Result<()> {
        match &self.state {
            Bound::PopupLayer(layer) => layer.restore(),
            Bound::Window(snapshot) => snapshot.restore(),
        }
    }
    /// If the original source is gone, nothing remains to restore; query failure must not impersonate disappearance and silently discard recovery responsibility.
    pub fn restore_or_retire(&self) -> Result<()> {
        match &self.state {
            Bound::PopupLayer(layer) => layer.restore(),
            Bound::Window(snapshot) => snapshot.restore_or_retire(),
        }
    }
    pub fn drain_helper_notifications(&self) {
        if let Bound::PopupLayer(layer) = &self.state {
            layer.drain_helper_notifications();
        }
    }
}
fn snapshot(target: Target, state: &WindowState) -> Result<Snapshot> {
    let style = isize::try_from(state.extended_style)
        .map_err(|_| Error::new(E_INVALIDARG, "Live recovery style is out of range"))?;
    let snapshot = Snapshot {
        target,
        rect: rect(state.rect),
        placement: WINDOWPLACEMENT {
            length: std::mem::size_of::<WINDOWPLACEMENT>() as u32,
            flags: WINDOWPLACEMENT_FLAGS(state.placement_flags),
            showCmd: state.show_command,
            ptMinPosition: point(state.min_position),
            ptMaxPosition: point(state.max_position),
            rcNormalPosition: rect(state.normal_position),
        },
        iconic: state.iconic,
        visible: state.visible,
        extended_style: style,
        layered: state.layered.as_ref().map(|value| Layered {
            color: COLORREF(value.color),
            alpha: value.alpha,
            flags: LAYERED_WINDOW_ATTRIBUTES_FLAGS(value.flags),
        }),
    };
    snapshot.target.validate()?;
    Ok(snapshot)
}

#[cfg(test)]
#[path = "live_recovery_target_tests.rs"]
mod tests;
