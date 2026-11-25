//! Actual system COMBOBOX patterns and declarative parents, without another Selection-provider implementation.
use super::*;
use windows::Win32::System::Variant::VARIANT;
use windows::Win32::UI::Controls::{GetComboBoxInfo, COMBOBOXINFO};
use windows::{core::w, Win32::UI::Input::KeyboardAndMouse::GetFocus};
// Bind HWND and owner at failure time; do not infer the focus-stealing process from later stale-HWND queries.
fn foreground_owner() -> (HWND, u32, u32) {
    unsafe {
        let hwnd = GetForegroundWindow();
        let mut pid = 0;
        let thread = GetWindowThreadProcessId(hwnd, Some(&mut pid));
        (hwnd, pid, thread)
    }
}
fn baseline(hwnd: usize) -> Result<()> {
    unsafe {
        CoInitializeEx(None, COINIT_MULTITHREADED).ok()?;
        let _apartment = Apartment;
        let client: IUIAutomation2 = CoCreateInstance(&CUIAutomation8, None, CLSCTX_INPROC_SERVER)?;
        client.SetConnectionTimeout(3000)?;
        client.SetTransactionTimeout(3000)?;
        let root = client.ElementFromHandle(HWND(hwnd as *mut _))?;
        let select = root.FindFirst(
            TreeScope_Children,
            &client.CreatePropertyCondition(
                UIA_ControlTypePropertyId,
                &VARIANT::from(UIA_ComboBoxControlTypeId.0),
            )?,
        )?;
        let expand: IUIAutomationExpandCollapsePattern =
            select.GetCurrentPatternAs(UIA_ExpandCollapsePatternId)?;
        expand.Expand()?;
        let walker = client.RawViewWalker()?;
        let mut next = walker.GetFirstChildElement(&select).ok();
        for _ in 0..4 {
            let Some(element) = next.take() else { break };
            assert_eq!(
                element.CurrentProcessId()? as u32,
                std::process::id(),
                "baseline escaped"
            );
            next = walker.GetNextSiblingElement(&element).ok();
        }
        assert!(next.is_none());
        expand.Collapse()?;
    }
    Ok(())
}
#[test]
fn uia_select_unmodified_system_baseline() {
    let _com = crate::desktop::Apartment::new().unwrap();
    struct NativeParent(HWND);
    impl Drop for NativeParent {
        fn drop(&mut self) {
            unsafe {
                let _ = DestroyWindow(self.0);
            }
        }
    }
    let window = NativeParent(unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE(0),
            windows::core::w!("STATIC"),
            windows::core::w!("Select fixture"),
            WS_OVERLAPPEDWINDOW,
            0,
            0,
            400,
            300,
            None,
            None,
            None,
            None,
        )
        .unwrap()
    });
    let scene = crate::surface_scene::Scene::parse(br#"{"id":"select","type":"select","props":{"value":"a","options":[{"value":"a","label":"First"},{"value":"b","label":"Second"}]}}"#).unwrap();
    let control = crate::desktop::surface_select_control::Control::new(
        window.0,
        RECT {
            left: 0,
            top: 0,
            right: 200,
            bottom: 180,
        },
        crate::surface_select::Options::parse(scene.root()).unwrap(),
    )
    .unwrap();
    control.show().unwrap();
    unsafe {
        let _ = ShowWindow(window.0, SW_SHOWNOACTIVATE);
        SendMessageW(
            control.hwnd().unwrap(),
            CB_SHOWDROPDOWN,
            Some(WPARAM(1)),
            None,
        );
    }
    // Validate the v6 UIA path actually used by clients; archive legacy MSAA child-navigation defects separately,
    // without allowing non-UIA system diagnostics to prevent actual tree-traversal assertions here.
    query(window.0 .0 as usize, baseline);
}
pub(in crate::desktop::surface_window::uia) fn receive(hwnd: usize) -> Result<()> {
    let mut stage = "initialize";
    let result = receive_traced(hwnd, &mut stage);
    if let Err(error) = &result {
        eprintln!("UIA Select failed: stage={stage}, error={error:?}, root={hwnd:#x}");
    }
    result
}
fn receive_traced(hwnd: usize, stage: &mut &str) -> Result<()> {
    unsafe {
        CoInitializeEx(None, COINIT_MULTITHREADED).ok()?;
    }
    let _apartment = Apartment;
    unsafe {
        let client: IUIAutomation2 = CoCreateInstance(&CUIAutomation8, None, CLSCTX_INPROC_SERVER)?;
        client.SetConnectionTimeout(3000)?;
        client.SetTransactionTimeout(3000)?;
        client.SetAutoSetFocus(false)?;
        let root = client.ElementFromHandle(HWND(hwnd as *mut _))?;
        let condition =
            client.CreatePropertyCondition(UIA_AutomationIdPropertyId, &VARIANT::from("select"))?;
        *stage = "find select";
        let nodes = root.FindAll(TreeScope_Descendants, &condition)?;
        assert_eq!(nodes.Length()?, 1, "missing or duplicate Select");
        let select = nodes.GetElement(0)?;
        assert_eq!(select.CurrentControlType()?, UIA_ComboBoxControlTypeId);
        assert_eq!(
            client
                .ControlViewWalker()?
                .GetParentElement(&select)?
                .CurrentAutomationId()?
                .to_string(),
            "group"
        );
        // Record only failed stages; success adds no COM queries, pumping, activation, or focus retries.
        *stage = "set focus";
        select.SetFocus()?;
        *stage = "read focus";
        assert!(
            select.CurrentHasKeyboardFocus()?.as_bool(),
            "Select lost focus: root={hwnd:#x}, foreground_owner={:?}, own_pid={}",
            foreground_owner(),
            std::process::id()
        );
        *stage = "expand";
        let expand: IUIAutomationExpandCollapsePattern =
            select.GetCurrentPatternAs(UIA_ExpandCollapsePatternId)?;
        expand.Expand()?;
        assert_eq!(
            expand.CurrentExpandCollapseState()?,
            ExpandCollapseState_Expanded
        );
        let walker = client.RawViewWalker()?;
        let mut next = walker.GetFirstChildElement(&select).ok();
        for _ in 0..4 {
            let Some(element) = next.take() else { break };
            assert_eq!(
                element.CurrentProcessId()? as u32,
                root.CurrentProcessId()? as u32,
                "Select navigation escaped its process"
            );
            next = walker.GetNextSiblingElement(&element).ok();
        }
        assert!(next.is_none(), "Select child navigation did not terminate");
        *stage = "find items";
        let items = select.FindAll(
            TreeScope_Descendants,
            &client.CreatePropertyCondition(
                UIA_ControlTypePropertyId,
                &VARIANT::from(UIA_ListItemControlTypeId.0),
            )?,
        )?;
        let count = items.Length()?;
        if count != 2 {
            // On failure, read state only; do not retry expansion or change focus, preserving original failure conditions.
            let combo = FindWindowExW(Some(HWND(hwnd as *mut _)), None, w!("COMBOBOX"), None)?;
            eprintln!(
                "UIA Select missing items: count={count}, root={hwnd:#x}, combo={combo:?}, foreground_owner={:?}, own_pid={}, focus={:?}, expanded={:?}, selected={}, dropped={}",
                foreground_owner(),
                std::process::id(),
                select.CurrentHasKeyboardFocus(),
                expand.CurrentExpandCollapseState(),
                SendMessageW(combo, CB_GETCURSEL, None, None).0,
                SendMessageW(combo, CB_GETDROPPEDSTATE, None, None).0
            );
            let mut info = COMBOBOXINFO {
                cbSize: std::mem::size_of::<COMBOBOXINFO>() as u32,
                ..Default::default()
            };
            let status = GetComboBoxInfo(combo, &mut info);
            eprintln!(
                "UIA Select native list: info={status:?}, list={:?}, visible={}, native_count={}",
                info.hwndList,
                IsWindowVisible(info.hwndList).as_bool(),
                SendMessageW(combo, CB_GETCOUNT, None, None).0
            );
            if status.is_ok() && IsWindow(Some(info.hwndList)).as_bool() {
                let direct = client.ElementFromHandle(info.hwndList).and_then(|list| {
                    list.FindAll(
                        TreeScope_Descendants,
                        &client.CreatePropertyCondition(
                            UIA_ControlTypePropertyId,
                            &VARIANT::from(UIA_ListItemControlTypeId.0),
                        )?,
                    )?
                    .Length()
                });
                eprintln!("UIA Select direct list item count: {direct:?}");
            }
        }
        assert_eq!(count, 2);
        // System COMBOBOX may delegate Selection to its child List; the container need not supply the pattern itself.
        *stage = "find list";
        let list = select.FindFirst(
            TreeScope_Descendants,
            &client.CreatePropertyCondition(
                UIA_ControlTypePropertyId,
                &VARIANT::from(UIA_ListControlTypeId.0),
            )?,
        )?;
        *stage = "selection pattern";
        let selection: IUIAutomationSelectionPattern =
            list.GetCurrentPatternAs(UIA_SelectionPatternId)?;
        assert!(!selection.CurrentCanSelectMultiple()?.as_bool());
        let selected = selection.GetCurrentSelection()?;
        assert_eq!(selected.Length()?, 1);
        assert_eq!(selected.GetElement(0)?.CurrentName()?.to_string(), "First");
        let second = items.GetElement(1)?;
        assert_eq!(second.CurrentName()?.to_string(), "Second");
        let item: IUIAutomationSelectionItemPattern =
            second.GetCurrentPatternAs(UIA_SelectionItemPatternId)?;
        *stage = "select second";
        item.Select()?;
        assert!(item.CurrentIsSelected()?.as_bool());
        assert_eq!(
            selection
                .GetCurrentSelection()?
                .GetElement(0)?
                .CurrentName()?
                .to_string(),
            "Second"
        );
        *stage = "collapse";
        expand.Collapse()?;
        assert_eq!(
            expand.CurrentExpandCollapseState()?,
            ExpandCollapseState_Collapsed
        );
        *stage = "final combo identity";
        let combos = root.FindAll(
            TreeScope_Descendants,
            &client.CreatePropertyCondition(
                UIA_ControlTypePropertyId,
                &VARIANT::from(UIA_ComboBoxControlTypeId.0),
            )?,
        )?;
        assert_eq!(
            combos.Length()?,
            1,
            "COMBOBOX duplicated outside logical parent"
        );
    }
    Ok(())
}
#[test]
fn uia_select_system_relocation_preserves_selection_expand_and_change_queue() {
    let _serial = crate::surface_javascript::fixture::SERIAL.lock().unwrap();
    let _com = crate::desktop::Apartment::new().unwrap();
    let scene = crate::surface_scene::Scene::parse(br#"{"id":"group","type":"column","children":[{"id":"select","type":"select","props":{"value":"a","options":[{"value":"a","label":"First"},{"value":"b","label":"Second"}]},"events":{"change":"choose"}}]}"#).unwrap();
    let window = super::super::super::SurfaceWindow::new(scene, 400, 300).unwrap();
    let binding = crate::surface_live_binding::Binding::from_mount(br#"{"instance":{"descriptor":{"instanceId":"instance:test","generation":1},"attachments":{"attachment:test":{"descriptor":{"attachmentId":"attachment:test","instanceId":"instance:test","hookNodeId":"node:test","deviceId":"viewer:test"},"lifecycle":"active","lifecycleRevision":1}}}}"#, "https://localhost:1234", "viewer:test", "node:test").unwrap();
    window.bind_edits(&binding).unwrap();
    window.set_input_policy(true, true).unwrap();
    window.show().unwrap();
    let combo = unsafe { FindWindowExW(Some(window.hwnd), None, w!("COMBOBOX"), None).unwrap() };
    let _trace = InputTrace::new(combo);
    let _parent_trace = InputTrace::new(window.hwnd);
    // Retain the original system HasKeyboardFocus assertion, establishing owned foreground through actual click authorization first.
    let _foreground = unsafe { crate::surface_javascript::fixture::focus_foreground(window.hwnd) };
    query(window.hwnd.0 as usize, receive);
    let pending = window.has_pending_edit();
    if !pending {
        // Collect evidence only in failed branches and still assert the original result; extra pumping must not turn failure into success.
        unsafe {
            let combo = FindWindowExW(Some(window.hwnd), None, w!("COMBOBOX"), None).unwrap();
            eprintln!(
                "UIA Select missing commit: focus={:?}, combo={:?}, selected={}, dropped={}",
                GetFocus(),
                combo,
                SendMessageW(combo, CB_GETCURSEL, None, None).0,
                SendMessageW(combo, CB_GETDROPPEDSTATE, None, None).0
            );
            let mut queued = Vec::new();
            let mut message = MSG::default();
            for _ in 0..64 {
                if !PeekMessageW(&mut message, None, 0, 0, PM_REMOVE).as_bool() {
                    break;
                }
                queued.push((message.message, message.wParam.0, message.lParam.0));
                let _ = TranslateMessage(&message);
                DispatchMessageW(&message);
            }
            eprintln!(
                "UIA Select after diagnostic drain: pending={}, queued={queued:?}, error={:?}",
                window.has_pending_edit(),
                window.take_error()
            );
        }
    }
    assert!(pending);
    let events = window.prepare_edit(&binding, 4).unwrap();
    assert_eq!(events.len(), 1);
    let wire: serde_json::Value = serde_json::from_slice(events[0].bytes()).unwrap();
    assert_eq!(wire["payload"], serde_json::json!({"value":"b"}));
    assert_eq!(wire["class"], "commit");
    window.settle_edits(&events).unwrap();
    assert!(!window.has_pending_edit());
    assert!(window.take_error().is_none());
}
