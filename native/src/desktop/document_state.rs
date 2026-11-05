//! 单个文档宿主的独立状态。构造时不创建 HWND、不注册热键，也不启动后台任务。
use super::{canvas::Canvas, hit_region::HitRegion, image::Image, State};
use std::cell::{Cell, RefCell};
use windows::{core::Result, Win32::Foundation::HWND};
#[path = "document_local_art.rs"]
pub(super) mod local_art;

pub(super) fn new_unit_id() -> Result<crate::workspace_id::UnitId> {
    let guid = unsafe { windows::Win32::System::Com::CoCreateGuid()? };
    let tail = guid.data4;
    let text = format!(
        "{:08x}-{:04x}-{:04x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        guid.data1,
        guid.data2,
        guid.data3,
        tail[0],
        tail[1],
        tail[2],
        tail[3],
        tail[4],
        tail[5],
        tail[6],
        tail[7]
    );
    // 固定长度的 ASCII UUID 必然满足 UnitId 的有界字符串合同。
    Ok(crate::workspace_id::UnitId::parse(&text).expect("generated UUID must be a valid unit ID"))
}

impl State {
    pub(super) fn is_art(&self) -> bool {
        self.restored_art || self.local_art_id.is_some() || self.selected_art.borrow().is_some()
    }
    pub(super) fn from_prepared(prepared: super::open_image::PreparedDocument) -> Result<Self> {
        // alpha 区域已在后台准备；建窗阶段不可再次扫描整图。
        let mut state = Self::new(None, true)?;
        state.capture_origin = prepared.capture_origin;
        state.canvas.borrow_mut().replace_image(prepared.image)?;
        if let Some(editable) = prepared.editable {
            let mut canvas = state.canvas.borrow_mut();
            canvas.editor = editable.editor;
            canvas.normal_opacity = editable.opacity;
            canvas.window_opacity = editable.opacity;
            canvas.applied_crop = canvas.editor.annotation_frame();
            canvas.sync_raster_source();
        }
        state
            .canvas
            .borrow_mut()
            .attach_capture(prepared.capture_source);
        *state.hit_region.borrow_mut() = prepared.region;
        Ok(state)
    }
    pub(super) fn new(image: Option<Image>, pinned: bool) -> Result<Self> {
        // 在首次绘制之前准备 alpha 命中区域；普通工作区不需要扫描像素。
        let hit_region = if pinned {
            image
                .as_ref()
                .map(HitRegion::prepare)
                .transpose()?
                .unwrap_or_default()
        } else {
            HitRegion::default()
        };
        let mut canvas = Canvas::new(image)?;
        canvas.pinned = pinned;
        let unit_id = new_unit_id()?;
        Ok(Self {
            chrome: Default::default(),
            parameters_ui: Default::default(),
            capture_history: Default::default(),
            projection_edit: RefCell::new(None),
            screen_color: Default::default(),
            live_request: RefCell::new(None),
            drag: Default::default(),
            group_catalog: Default::default(),
            group_id: RefCell::new(None),
            group_locked: Cell::new(false),
            group_hidden: Cell::new(false),
            group_request: RefCell::new(None),
            execution_results: RefCell::new(super::loom_result_state::Results::new(
                unit_id.clone(),
            )),
            pending_execution: RefCell::new(None),
            pending_preparation: RefCell::new(None),
            pending_start: RefCell::new(None),
            preparation_error: RefCell::new(None),
            execution_cancel: RefCell::new(None),
            execution_configuration: RefCell::new(None),
            catalog: Default::default(),
            authorization: Default::default(),
            selected_art: RefCell::new(None),
            restored_art: false,
            local_art_id: None,
            restored_naming_art: Default::default(),
            parameter_view: RefCell::new(None),
            link_menu: RefCell::new(None),
            link_request: RefCell::new(None),
            link_catalog: RefCell::new(None),
            link_creation: RefCell::new(None),
            parameter_file_request: RefCell::new(None),
            sticker_input: Default::default(),
            accept_upstream: Cell::new(true),
            resolved_display: RefCell::new(None),
            pending_adoption: RefCell::new(None),
            unit_id,
            new_window: Cell::new(false),
            art_request: RefCell::new(None),
            save_session_requested: Cell::new(false),
            save_existing_requested: Cell::new(false),
            workspace_autosave: Default::default(),
            workflow_sync_requested: Cell::new(None),
            open_session_requested: Cell::new(false),
            naming: Default::default(),
            cache_policy: Default::default(),
            frozen_library: Default::default(),
            copy_authority: Default::default(),
            editable_clipboard: Default::default(),
            naming_import_requested: Cell::new(false),
            naming_save_requested: Cell::new(false),
            naming_update_requested: Cell::new(false),
            naming_autosave: Default::default(),
            open_session_assets_requested: Cell::new(false),
            exit_requested: std::rc::Rc::new(Cell::new(false)),
            session_end: Default::default(),
            session_end_reason: Cell::new(false),
            new_pin: RefCell::new(None),
            capture_origin: None,
            hotkey_toggle: Cell::new(false),
            hotkey_enabled: std::rc::Rc::new(Cell::new(false)),
            long_hotkey_enabled: Default::default(),
            live_hotkey_enabled: Default::default(),
            capture_active: std::rc::Rc::new(Cell::new(false)),
            capture_windows: std::rc::Rc::new(RefCell::new(Vec::new())),
            lifetime: Default::default(),
            geometry_wake: Default::default(),
            priority_watcher: RefCell::new(None),
            global_capture: Cell::new(false),
            global_long_capture: Cell::new(false),
            global_live_capture: Cell::new(false),
            tray: Default::default(),
            capture_protection: Cell::new([false; 3]),
            capture_protection_requested: Cell::new(false),
            tool_panel: Cell::new(HWND::default()),
            tool_panel_tool: Cell::new(None),
            tool_panel_style: Cell::new(None),
            tool_panel_enabled: Cell::new(0),
            tool_panel_wheel: Cell::new([0; 2]),
            canvas: RefCell::new(canvas),
            error: RefCell::new(None),
            export: RefCell::new(None),
            copy_job: RefCell::new(None),
            file_dialog: std::rc::Rc::new(Cell::new(false)),
            modals: Default::default(),
            open_job: RefCell::new(None),
            blur_job: RefCell::new(Default::default()),
            erase_job: RefCell::new(Default::default()),
            resize_job: RefCell::new(Default::default()),
            flip_job: RefCell::new(Default::default()),
            crop_job: RefCell::new(None),
            rasterize_job: RefCell::new(None),
            render_retries: Cell::new(0),
            render_active: Cell::new(false),
            render_deferred: Cell::new(false),
            hit_region: RefCell::new(hit_region),
            priority_anchor: Cell::new(0),
            priority_policy: RefCell::new(Default::default()),
        })
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn missing_capability_does_not_change_restored_node_kind() {
        let mut state = super::State::new(None, false).unwrap();
        assert!(!state.is_art());
        state.restored_art = true;
        assert!(state.is_art());
        assert!(state.selected_art.borrow().is_none());
        assert!(!crate::desktop::display_edit::prepare_content(&state));
        assert!(state.pending_start.borrow().is_none());
        unsafe {
            use windows::Win32::UI::WindowsAndMessaging::*;
            let menu = CreatePopupMenu().unwrap();
            let result = crate::desktop::sticker_propagation::append(menu, &state);
            let count = GetMenuItemCount(Some(menu));
            DestroyMenu(menu).unwrap();
            result.unwrap();
            assert_eq!(count, 0);
        }
    }
    use super::*;
    #[test]
    fn new_workspace_ids_are_distinct_uuid_strings() {
        let mut ids = std::collections::BTreeSet::new();
        for _ in 0..32 {
            let id = new_unit_id().unwrap();
            let value = id.as_str().as_bytes();
            assert_eq!(value.len(), 36);
            for (index, byte) in value.iter().enumerate() {
                if [8, 13, 18, 23].contains(&index) {
                    assert_eq!(*byte, b'-');
                } else {
                    assert!(byte.is_ascii_hexdigit());
                }
            }
            assert!(ids.insert(id));
        }
    }
    use crate::{editor::Tool, viewport::Point};

    fn image() -> Image {
        Image {
            width: 8,
            height: 8,
            pixels: vec![255; 256].into(),
        }
    }

    #[test]
    fn shared_pixels_do_not_share_document_identity_history_or_tool_state() {
        let source = image();
        let left = State::new(Some(source.clone()), false).unwrap();
        let right = State::new(Some(source.clone()), true).unwrap();
        let original = right.canvas.borrow().editor.document_stamp();
        {
            let mut canvas = left.canvas.borrow_mut();
            assert!(!original.matches(&canvas.editor));
            assert!(canvas
                .image
                .as_ref()
                .unwrap()
                .pixels
                .same_storage(&source.pixels));
            assert!(!canvas.pinned);
            canvas.select_tool(Tool::Brush);
            let origin = canvas.viewport.origin;
            assert!(canvas.pointer_down(Point {
                x: origin.x + 1.0,
                y: origin.y + 1.0
            }));
            canvas.pointer_up(Point {
                x: origin.x + 5.0,
                y: origin.y + 5.0,
            });
            assert_eq!(canvas.editor.elements().len(), 1);
            assert!(canvas.editor.undo());
            assert!(canvas.editor.redo());
        }
        left.file_dialog.set(true);
        left.tool_panel_wheel.set([37, -29]);
        {
            let canvas = right.canvas.borrow();
            assert!(original.matches(&canvas.editor));
            assert!(canvas.editor.elements().is_empty());
            assert_eq!(canvas.tool, Tool::Pan);
            assert!(canvas.pinned);
            assert!(canvas
                .image
                .as_ref()
                .unwrap()
                .pixels
                .same_storage(&source.pixels));
        }
        assert!(!right.file_dialog.get());
        assert_eq!(right.tool_panel_wheel.get(), [0; 2]);
        drop(left);
        // 一个文档释放后，另一个文档保留有效的共享像素和独立编辑历史。
        let mut canvas = right.canvas.borrow_mut();
        assert_eq!(canvas.image.as_ref().unwrap().pixels[0], 255);
        assert!(!canvas.editor.undo());
        canvas.replace_image(image()).unwrap();
        assert!(!original.matches(&canvas.editor));
    }

    #[test]
    fn empty_document_starts_without_windows_or_queued_output() {
        let state = State::new(None, false).unwrap();
        assert!(state.canvas.borrow().image.is_none());
        assert!(state.tool_panel.get().0.is_null());
        assert!(!state.global_capture.get());
        assert!(state.open_job.borrow().is_none());
        assert!(state.export.borrow().is_none());
        assert!(state.copy_job.borrow().is_none());
    }
}
