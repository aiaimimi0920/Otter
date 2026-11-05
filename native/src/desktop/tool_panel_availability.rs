//! 面板可用性来自文档/任务 owner；执行时再次判断，过期点击不启动冲突操作。
use super::{context_menu as menu, State};
use crate::editor::Tool;
#[derive(Clone, Copy, Default)]
pub(super) struct Availability {
    image: bool,
    busy: bool,
    copying: bool,
    exporting: bool,
    target: bool,
    blur_tool: bool,
    mosaic_tool: bool,
    selected_blur: bool,
    capture_blocked: bool,
    locked: bool,
    minified: bool,
}
impl Availability {
    pub(super) fn capture(state: &State) -> Option<Self> {
        let canvas = state.canvas.try_borrow().ok()?;
        let image = canvas.image.is_some() || state.resolved_display.try_borrow().ok()?.is_some();
        let target = if canvas.tool == Tool::Select {
            canvas.selected_index().is_some() || canvas.selected_group().is_some()
        } else {
            !canvas.editor.elements().is_empty()
        };
        let busy = state.file_dialog.get()
            || state.open_job.try_borrow().ok()?.is_some()
            || state.erase_job.try_borrow().ok()?.is_busy()
            || state.crop_job.try_borrow().ok()?.is_some()
            || state.rasterize_job.try_borrow().ok()?.is_some()
            || state.resize_job.try_borrow().ok()?.is_busy()
            || state.flip_job.try_borrow().ok()?.is_busy();
        Some(Self {
            locked: state.group_locked.get(),
            minified: canvas.minified.is_some(),
            capture_blocked: !super::capture_input::exclusive_available(state)
                || state.new_pin.try_borrow().ok()?.is_some(),
            blur_tool: canvas.tool == Tool::Blur,
            mosaic_tool: canvas.tool == Tool::Mosaic,
            selected_blur: super::blur_input::selected_strength(&canvas).is_some(),
            image,
            busy,
            target,
            copying: state.copy_job.try_borrow().ok()?.is_some(),
            exporting: state.export.try_borrow().ok()?.is_some(),
        })
    }
    pub(super) fn allows(self, command: usize) -> bool {
        if self.minified
            && !matches!(
                command,
                menu::OPEN
                    | menu::EXPORT
                    | super::copy_job::COMMAND
                    | super::capture_input::NEW_PIN
                    | super::capture_input::NEW_FULL_PIN
                    | super::capture_input::LONG_RECORDING
                    | super::capture_input::LONG_VERTICAL
                    | super::capture_input::LONG_HORIZONTAL
            )
        {
            return false;
        }
        if self.locked && !super::group_lock::allows(command) {
            return false;
        }
        if self.busy {
            return false;
        }
        if let Some(entry) = super::tool_commands::TOOLS
            .iter()
            .find(|entry| entry.id == command)
        {
            return self.image || matches!(entry.tool, Tool::Pan | Tool::Select);
        }
        match command {
            menu::OPEN => true,
            super::capture_input::COMMAND
            | super::capture_input::FULL_SCREEN
            | super::capture_input::NEW_PIN
            | super::capture_input::NEW_FULL_PIN
            | super::capture_input::LONG_RECORDING
            | super::capture_input::LONG_VERTICAL
            | super::capture_input::LONG_HORIZONTAL => !self.capture_blocked,
            menu::EXPORT => self.image && !self.exporting,
            super::copy_job::COMMAND => self.image && !self.copying,
            menu::APPLY_STYLE => self.image && self.target,
            super::blur_numeric::COMMAND => self.image && self.selected_blur,
            super::blur_default::COMMAND => self.image && self.blur_tool,
            menu::COLOR
            | menu::FILL_COLOR
            | menu::WIDTH_DOWN
            | menu::WIDTH_UP
            | 0x4F
            | 0x46
            | super::numeric_style::WIDTH
            | super::numeric_style::OPACITY => self.image,
            super::numeric_style::MOSAIC => self.image && self.mosaic_tool,
            _ => false,
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn minified_panel_preserves_independent_output_but_disables_edit_controls() {
        let ready = Availability {
            image: true,
            minified: true,
            target: true,
            blur_tool: true,
            selected_blur: true,
            mosaic_tool: true,
            ..Default::default()
        };
        for entry in super::super::tool_commands::TOOLS {
            assert!(!ready.allows(entry.id));
        }
        for command in [
            menu::COLOR,
            menu::FILL_COLOR,
            menu::APPLY_STYLE,
            super::super::blur_default::COMMAND,
            super::super::blur_numeric::COMMAND,
            super::super::numeric_style::WIDTH,
            super::super::numeric_style::OPACITY,
            super::super::numeric_style::MOSAIC,
            super::super::capture_input::COMMAND,
            super::super::capture_input::FULL_SCREEN,
        ] {
            assert!(!ready.allows(command));
        }
        for command in [
            menu::OPEN,
            menu::EXPORT,
            super::super::copy_job::COMMAND,
            super::super::capture_input::NEW_PIN,
            super::super::capture_input::NEW_FULL_PIN,
            super::super::capture_input::LONG_RECORDING,
            super::super::capture_input::LONG_VERTICAL,
            super::super::capture_input::LONG_HORIZONTAL,
        ] {
            assert!(ready.allows(command));
            assert!(!Availability {
                busy: true,
                ..ready
            }
            .allows(command));
        }
    }
    #[test]
    fn external_only_host_has_output_availability_without_adopting_image() {
        let state = State::new(None, true).unwrap();
        let stamp = state.canvas.borrow().editor.document_stamp();
        *state.resolved_display.borrow_mut() = Some(super::super::display_frame::Frame::capture(
            crate::workspace_id::UnitId::parse("availability-source").unwrap(),
            super::super::image::Image {
                width: 1,
                height: 1,
                pixels: vec![255; 4].into(),
            },
            None,
        ));
        let availability = Availability::capture(&state).unwrap();
        assert!(availability.allows(menu::EXPORT));
        assert!(availability.allows(super::super::copy_job::COMMAND));
        assert!(state.canvas.borrow().image.is_none());
        assert!(stamp.matches(&state.canvas.borrow().editor));
    }
    #[test]
    fn shared_capture_block_does_not_disable_independent_editing() {
        let state = Availability {
            image: true,
            capture_blocked: true,
            ..Default::default()
        };
        for command in [
            super::super::capture_input::COMMAND,
            super::super::capture_input::FULL_SCREEN,
            super::super::capture_input::DEFAULT,
            super::super::capture_input::NEW_FULL_PIN,
            super::super::capture_input::LONG_RECORDING,
            super::super::capture_input::LONG_VERTICAL,
            super::super::capture_input::LONG_HORIZONTAL,
        ] {
            assert!(!state.allows(command));
        }
        assert!(state.allows(menu::OPEN));
        assert!(state.allows(0x42));
    }
    #[test]
    fn effect_controls_require_their_actual_tool_or_selected_blur_target() {
        let commands = [
            super::super::blur_default::COMMAND,
            super::super::blur_numeric::COMMAND,
            super::super::numeric_style::MOSAIC,
        ];
        let states = [
            Availability {
                image: true,
                blur_tool: true,
                ..Default::default()
            },
            Availability {
                image: true,
                selected_blur: true,
                ..Default::default()
            },
            Availability {
                image: true,
                mosaic_tool: true,
                ..Default::default()
            },
        ];
        for (index, state) in states.into_iter().enumerate() {
            for (other, command) in commands.into_iter().enumerate() {
                assert_eq!(state.allows(command), index == other);
                assert!(!Availability {
                    busy: true,
                    ..state
                }
                .allows(command));
                assert!(!Availability {
                    image: false,
                    ..state
                }
                .allows(command));
            }
        }
    }
    #[test]
    fn empty_document_exposes_only_navigation_and_open() {
        let state = Availability::default();
        for entry in super::super::tool_commands::TOOLS {
            assert_eq!(
                state.allows(entry.id),
                matches!(entry.tool, Tool::Pan | Tool::Select)
            );
        }
        assert!(state.allows(menu::OPEN));
        assert!(state.allows(super::super::capture_input::COMMAND));
        assert!(state.allows(super::super::capture_input::FULL_SCREEN));
        assert!(state.allows(super::super::capture_input::DEFAULT));
        assert!(state.allows(super::super::capture_input::NEW_FULL_PIN));
        assert!(!state.allows(menu::EXPORT));
        assert!(!state.allows(menu::COLOR));
        assert!(!state.allows(usize::MAX));
    }
    #[test]
    fn editing_busy_is_exclusive_and_output_workers_have_independent_gates() {
        let ready = Availability {
            image: true,
            target: true,
            ..Default::default()
        };
        for command in [
            menu::OPEN,
            super::super::capture_input::COMMAND,
            super::super::capture_input::FULL_SCREEN,
            super::super::capture_input::NEW_PIN,
            super::super::capture_input::NEW_FULL_PIN,
            super::super::capture_input::LONG_RECORDING,
            super::super::capture_input::LONG_VERTICAL,
            super::super::capture_input::LONG_HORIZONTAL,
            menu::EXPORT,
            menu::APPLY_STYLE,
            0x42,
            0x47,
            super::super::copy_job::COMMAND,
        ] {
            assert!(ready.allows(command));
            assert!(!Availability {
                busy: true,
                ..ready
            }
            .allows(command));
        }
        assert!(!Availability {
            target: false,
            ..ready
        }
        .allows(menu::APPLY_STYLE));
        let copying = Availability {
            copying: true,
            ..ready
        };
        assert!(!copying.allows(super::super::copy_job::COMMAND));
        assert!(copying.allows(menu::EXPORT));
        assert!(copying.allows(0x42));
        let exporting = Availability {
            exporting: true,
            ..ready
        };
        assert!(!exporting.allows(menu::EXPORT));
        assert!(exporting.allows(super::super::copy_job::COMMAND));
    }
}
