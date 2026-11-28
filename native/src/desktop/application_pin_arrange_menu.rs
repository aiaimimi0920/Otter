//! The popup captures exact members. Re-resolve selection and group identity
//! after tracking so the command cannot silently apply to a different set.
use super::*;
const ALL: usize = 0x2190;
const SELECT: usize = 0x21a0;
const GROUP: usize = 0x21b0;
const UNDO: usize = 0x21c0;
pub(in crate::desktop::application) const SHAKE_TOGGLE: usize = 0x21c2;
pub(in crate::desktop::application) const OPEN_PALETTE: usize = 0x21c3;
const ENTRIES: &[(Layout, &str)] = &[
    (Layout::Left, "workspace-layout-left"),
    (Layout::Top, "workspace-layout-top"),
    (Layout::CenterX, "workspace-layout-center-x"),
    (Layout::CenterY, "workspace-layout-center-y"),
    (Layout::CompactX, "workspace-layout-compact-x"),
    (Layout::CompactY, "workspace-layout-compact-y"),
    (Layout::DistributeX, "workspace-layout-distribute-x"),
    (Layout::DistributeY, "workspace-layout-distribute-y"),
    (Layout::Grid, "workspace-layout-grid"),
];
#[derive(Clone)]
pub(in crate::desktop::application) struct Snapshot {
    instance: crate::workspace::IdentityStamp,
    all: Vec<Target>,
    selected: Vec<Target>,
    group: Vec<Target>,
    group_id: Option<UnitId>,
    undo: bool,
    shake_hidden: bool,
    can_hide_others: bool,
}
impl Snapshot {
    pub(in crate::desktop::application) fn all_count(&self) -> usize {
        self.all.len()
    }
    pub(in crate::desktop::application) fn selected_count(&self) -> usize {
        self.selected.len()
    }
    pub(in crate::desktop::application) fn group_count(&self) -> usize {
        self.group.len()
    }
    pub(in crate::desktop::application) fn all_targets(&self) -> &[Target] {
        &self.all
    }
    pub(in crate::desktop::application) fn selected_targets(&self) -> &[Target] {
        &self.selected
    }
    pub(in crate::desktop::application) fn group_targets(&self) -> &[Target] {
        &self.group
    }
    pub(in crate::desktop::application) fn matches_workspace(
        &self,
        workspace: &crate::workspace::Workspace,
    ) -> bool {
        self.instance.matches(workspace)
    }
    pub(in crate::desktop::application) unsafe fn capture(app: &Application, id: &UnitId) -> Self {
        let resolve = |ids: &[UnitId]| {
            ids.iter()
                .map(|id| app.layout_target(id))
                .collect::<Option<Vec<_>>>()
                .unwrap_or_default()
        };
        let all = app
            .documents
            .iter()
            .filter_map(|d| app.layout_target(&d.state.unit_id))
            .collect::<Vec<_>>();
        let group_id = app.workspace.group_of(id).map(|g| g.id.clone());
        let group = group_id
            .as_ref()
            .map(|gid| {
                resolve(
                    &app.workspace
                        .units()
                        .iter()
                        .filter(|u| app.workspace.group_of(u).is_some_and(|g| &g.id == gid))
                        .cloned()
                        .collect::<Vec<_>>(),
                )
            })
            .unwrap_or_default();
        let selected = app
            .chrome
            .selected_units()
            .filter(|v| !v.is_empty())
            .map(|v| resolve(&v))
            .unwrap_or_default();
        let shake_hidden =
            app.pin_layout.signals.count.get() > 0 || app.pin_layout.shake_origin.is_some();
        let other_visible = app.documents.iter().any(|d| {
            &d.state.unit_id != id
                && d.is_alive()
                && app.workspace.contains(&d.state.unit_id)
                && d.state.canvas.try_borrow().is_ok_and(|c| c.pinned)
                && !d.state.group_hidden.get()
                && !app
                    .workspace
                    .group_of(&d.state.unit_id)
                    .is_some_and(|g| g.hidden)
                && !crate::desktop::tray::is_hidden(&d.state)
                && IsWindowVisible(d.window.0).as_bool()
        });
        Self {
            instance: app.workspace.identity_stamp(),
            all,
            selected,
            group,
            group_id,
            undo: !app.pin_layout.undo.is_empty(),
            shake_hidden,
            can_hide_others: other_visible,
        }
    }
    pub(in crate::desktop::application) unsafe fn append(&self, root: HMENU) -> Result<()> {
        if self.shake_hidden {
            AppendMenuW(
                root,
                MF_STRING,
                SHAKE_TOGGLE,
                &HSTRING::from(crate::ui_text::text("workspace-shake-show-all")),
            )?;
        } else if self.can_hide_others {
            AppendMenuW(
                root,
                MF_STRING,
                SHAKE_TOGGLE,
                &HSTRING::from(crate::ui_text::text("workspace-shake-hide-others")),
            )?;
        }
        if self.all.len() >= 2 || self.undo {
            AppendMenuW(
                root,
                MF_STRING,
                OPEN_PALETTE,
                &HSTRING::from(crate::ui_text::text("workspace-layout-open")),
            )?;
        }
        if self.undo {
            AppendMenuW(
                root,
                MF_STRING,
                UNDO,
                &HSTRING::from(crate::ui_text::text("workspace-layout-undo")),
            )?;
        }
        Ok(())
    }
    pub(in crate::desktop::application) unsafe fn execute(
        &self,
        app: &mut Application,
        id: &UnitId,
        cmd: usize,
    ) -> Result<bool> {
        if cmd != UNDO
            && cmd != SHAKE_TOGGLE
            && cmd != OPEN_PALETTE
            && !(ALL..ALL + ENTRIES.len()).contains(&cmd)
            && !(SELECT..SELECT + ENTRIES.len()).contains(&cmd)
            && !(GROUP..GROUP + ENTRIES.len()).contains(&cmd)
        {
            return Ok(false);
        }
        if !self.instance.matches(&app.workspace) {
            return Err(stale());
        }
        if cmd == SHAKE_TOGGLE {
            app.toggle_shake_from(id);
            return Ok(true);
        }
        if cmd == OPEN_PALETTE {
            if let Some(doc) = app
                .documents
                .iter()
                .find(|d| &d.state.unit_id == id && d.is_alive())
            {
                let hwnd = doc.window.0;
                let flag = doc.state.file_dialog.clone();
                if let Some(_guard) = crate::desktop::file_dialog::Guard::begin(&flag) {
                    app.pin_layout.palette_host.set(Some(hwnd));
                    *app.pin_layout.palette_gate.borrow_mut() = Some(flag.clone());
                    let res = super::palette::show(hwnd, app, id, self);
                    app.pin_layout.palette_host.set(None);
                    *app.pin_layout.palette_gate.borrow_mut() = None;
                    res?;
                };
            }
            return Ok(true);
        }
        if cmd == UNDO {
            if self.undo {
                app.undo_layout()?
            }
            return Ok(true);
        }
        let current = Self::capture(app, id);
        let (expected, fresh, base) = if cmd >= GROUP {
            if current.group_id != self.group_id {
                return Err(stale());
            }
            (&self.group, &current.group, GROUP)
        } else if cmd >= SELECT {
            (&self.selected, &current.selected, SELECT)
        } else {
            (&self.all, &current.all, ALL)
        };
        if expected != fresh {
            return Err(stale());
        }
        app.arrange_pins(expected, id, ENTRIES[cmd - base].0)?;
        Ok(true)
    }
}
