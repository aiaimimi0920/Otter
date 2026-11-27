//! The popup captures exact members. Re-resolve selection and group identity
//! after tracking so the command cannot silently apply to a different set.
use super::*;
use crate::desktop::context_menu::Menu;
const SELECT: usize = 0x21a0;
const GROUP: usize = 0x21b0;
const UNDO: usize = 0x21c0;
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
pub(in crate::desktop::application) struct Snapshot {
    instance: crate::workspace::IdentityStamp,
    selected: Vec<Target>,
    group: Vec<Target>,
    group_id: Option<UnitId>,
    undo: bool,
    pass: Option<Target>,
}
impl Snapshot {
    pub(in crate::desktop::application) unsafe fn capture(app: &Application, id: &UnitId) -> Self {
        let resolve = |ids: &[UnitId]| {
            ids.iter()
                .map(|id| app.layout_target(id))
                .collect::<Option<Vec<_>>>()
                .unwrap_or_default()
        };
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
            .filter(|v| v.contains(id))
            .map(|v| resolve(&v))
            .unwrap_or_default();
        Self {
            instance: app.workspace.identity_stamp(),
            selected,
            group,
            group_id,
            undo: !app.pin_layout.undo.is_empty(),
            pass: app.click_target(id),
        }
    }
    pub(in crate::desktop::application) unsafe fn append(&self, root: HMENU) -> Result<()> {
        if self.pass.is_some() {
            AppendMenuW(
                root,
                MF_STRING,
                crate::desktop::pin_click_through::ENTER,
                &HSTRING::from(crate::ui_text::text("pin-pass-enter")),
            )?;
        }
        for (members, base, key) in [
            (&self.selected, SELECT, "workspace-layout-selection"),
            (&self.group, GROUP, "workspace-layout-group"),
        ] {
            if members.len() < 2 {
                continue;
            }
            let sub = Menu(CreatePopupMenu()?);
            for (i, (layout, label)) in ENTRIES.iter().enumerate() {
                AppendMenuW(
                    sub.0,
                    MF_STRING
                        | if members.len() >= layout.min_count() {
                            MF_ENABLED
                        } else {
                            MF_GRAYED
                        },
                    base + i,
                    &HSTRING::from(crate::ui_text::text(label)),
                )?;
            }
            AppendMenuW(
                root,
                MF_POPUP,
                sub.0 .0 as usize,
                &HSTRING::from(crate::ui_text::text(key)),
            )?;
            std::mem::forget(sub);
        }
        if self.undo {
            AppendMenuW(
                root,
                MF_STRING,
                UNDO,
                &HSTRING::from(crate::ui_text::text("workspace-layout-undo")),
            )?
        }
        Ok(())
    }
    pub(in crate::desktop::application) unsafe fn execute(
        &self,
        app: &mut Application,
        id: &UnitId,
        cmd: usize,
    ) -> Result<bool> {
        if cmd == crate::desktop::pin_click_through::ENTER {
            let t = self
                .pass
                .as_ref()
                .filter(|t| &t.id == id && self.instance.matches(&app.workspace))
                .ok_or_else(stale)?;
            app.enter_click(t)?;
            return Ok(true);
        }
        if cmd != UNDO
            && !(SELECT..SELECT + ENTRIES.len()).contains(&cmd)
            && !(GROUP..GROUP + ENTRIES.len()).contains(&cmd)
        {
            return Ok(false);
        }
        if !self.instance.matches(&app.workspace) {
            return Err(stale());
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
        } else {
            (&self.selected, &current.selected, SELECT)
        };
        if expected != fresh {
            return Err(stale());
        }
        app.arrange_pins(expected, id, ENTRIES[cmd - base].0)?;
        Ok(true)
    }
}
