use super::*;
#[test]
fn actual_ports_button_commits_drafts_then_rebinds_only_its_successor_stamp() {
    unsafe {
        let f = Fixture::new();
        let mut catalog = crate::desktop::application::chrome::tests::catalog();
        Rc::get_mut(&mut catalog).unwrap().endpoints[0].unit = f.state.unit_id.clone();
        *f.state.link_catalog.borrow_mut() = Some(catalog);
        *f.panel.ui.model.borrow_mut() = Rc::new(model::Model::capture(&f.state));
        controls::rebuild(&f.panel.ui).unwrap();
        f.text("text:edit", "先提交");
        f.click("ports");
        assert_eq!(f.panel.ui.pending.borrow().len(), 2);
        assert!(matches!(
            f.panel.ui.pending.borrow().back(),
            Some(Intent::Ports(_))
        ));
        f.tick();
        assert_eq!(f.value("text"), "先提交");
        // fixture故意没有绑定chrome，证明不是旧配置Stamp拦截；真实菜单/单槽另验。
        assert_eq!(&*f.panel.ui.message.borrow(), "端口设置暂不可用");
        let before = Stamp::capture(&f.state);
        f.state
            .selected_art
            .borrow_mut()
            .as_mut()
            .unwrap()
            .parameters
            .set("text", Some(serde_json::json!("外部变化")))
            .unwrap();
        f.panel.apply(&f.state, Intent::Ports(before)).unwrap();
        assert_eq!(&*f.panel.ui.message.borrow(), "配置已变化，请重试");
    }
}
#[test]
fn draft_then_run_is_ordered_once_and_promotes_only_its_parameter_trigger() {
    unsafe {
        let f = Fixture::new();
        f.text("text:edit", "中文🙂 draft");
        controls::commit(&f.panel.ui, &f.binding("text:edit"));
        f.click("run");
        assert_eq!(f.panel.ui.pending.borrow().len(), 2);
        f.tick();
        assert_eq!(f.value("text"), "中文🙂 draft");
        assert_eq!(
            f.state.pending_start.borrow().as_ref().unwrap().trigger,
            crate::graph_execution_config::Trigger::Manual
        );
        assert!(f.panel.ui.pending.borrow().is_empty());
        assert!(f.panel.exit_idle());
    }
}
#[test]
fn invalid_json_keeps_draft_blocks_run_and_explicit_reload_is_local() {
    unsafe {
        let f = Fixture::new();
        let edit = f.text("json:edit", "{invalid");
        f.click("run");
        assert!(f.panel.ui.pending.borrow().is_empty());
        assert!(!f.panel.exit_idle());
        assert_eq!(controls::text(edit).unwrap(), "{invalid");
        assert_eq!(f.value("json"), serde_json::json!({"value":1}));
        f.click("reload");
        f.tick();
        assert!(f.panel.exit_idle());
        assert!(f.state.pending_start.borrow().is_none());
    }
}
#[test]
fn external_configuration_and_linked_value_refresh_preserve_dirty_edit_but_reject_commit() {
    unsafe {
        let f = Fixture::new();
        let edit = f.text("text:edit", "未提交");
        f.state
            .selected_art
            .borrow_mut()
            .as_mut()
            .unwrap()
            .parameters
            .set("n", Some(serde_json::json!(4)))
            .unwrap();
        f.tick();
        assert_eq!(f.binding("text:edit").hwnd, edit);
        assert_eq!(controls::text(edit).unwrap(), "未提交");
        controls::commit(&f.panel.ui, &f.binding("text:edit"));
        f.tick();
        assert_eq!(f.value("text"), "原值");
        assert!(f.panel.ui.message.borrow().contains("草稿未提交"));
        f.click("reload");
        f.tick();
        let mut values = serde_json::Map::new();
        values.insert("n".into(), serde_json::json!(8));
        desktop::parameter_view::View::update(&f.state, values, vec!["n".into()]);
        f.tick();
        assert!(!IsWindowEnabled(f.binding("n:edit").hwnd).as_bool());
        assert_eq!(controls::text(f.binding("n:edit").hwnd).unwrap(), "8");
        assert_eq!(f.value("n"), 4);
    }
}
#[test]
fn combo_keeps_scalar_types_and_number_step_uses_existing_spec() {
    unsafe {
        let f = Fixture::new();
        let combo = f.binding("mode:options").hwnd;
        SendMessageW(combo, CB_SETCURSEL, Some(WPARAM(1)), None);
        SendMessageW(
            f.panel.handle(),
            WM_COMMAND,
            Some(WPARAM((CBN_SELCHANGE as usize) << 16)),
            Some(LPARAM(combo.0 as isize)),
        );
        f.tick();
        assert_eq!(f.value("mode"), 2);
        f.click("n:step:true");
        f.tick();
        assert_eq!(f.value("n"), 3.5);
    }
}
#[test]
fn bounded_pending_queue_does_not_overwrite_accepted_actions() {
    unsafe {
        let f = Fixture::new();
        for _ in 0..6 {
            f.click("stop");
        }
        assert_eq!(f.panel.ui.pending.borrow().len(), 4);
        assert!(f.panel.ui.message.borrow().contains("队列已满"));
        f.tick();
        assert!(f.panel.ui.pending.borrow().is_empty());
    }
}
#[test]
fn multiple_drafts_are_atomic_and_a_following_choice_is_not_stale() {
    unsafe {
        let f = Fixture::new();
        f.text("text:edit", "两个字段🙂");
        f.text("n:edit", "7");
        controls::commit(&f.panel.ui, &f.binding("text:edit"));
        f.click("enabled:boolean");
        assert_eq!(f.panel.ui.pending.borrow().len(), 2);
        f.tick();
        assert_eq!(f.value("text"), "两个字段🙂");
        assert_eq!(f.value("n"), 7);
        assert_eq!(f.value("enabled"), true);
        assert!(f.panel.exit_idle());
    }
}
#[test]
fn focus_loss_then_step_coalesces_the_final_value_instead_of_dropping_it() {
    unsafe {
        let f = Fixture::new();
        f.text("n:edit", "7");
        controls::commit(&f.panel.ui, &f.binding("n:edit"));
        f.click("n:step:true");
        assert_eq!(f.panel.ui.pending.borrow().len(), 1);
        f.tick();
        assert_eq!(f.value("n"), 7.5);
        assert!(f.panel.exit_idle());
    }
}
#[test]
fn invalid_later_draft_cannot_publish_an_earlier_valid_field() {
    unsafe {
        let f = Fixture::new();
        f.text("text:edit", "必须保留");
        f.text("json:edit", "{invalid");
        controls::commit(&f.panel.ui, &f.binding("text:edit"));
        f.tick();
        assert_eq!(f.value("text"), "原值");
        assert_eq!(
            controls::text(f.binding("text:edit").hwnd).unwrap(),
            "必须保留"
        );
        assert!(!f.panel.exit_idle());
    }
}
#[test]
fn local_text_batch_keeps_linked_parameter_read_only_through_successor_choice() {
    unsafe {
        let f = Fixture::new();
        let mut values = serde_json::Map::new();
        values.insert("n".into(), serde_json::json!(8));
        desktop::parameter_view::View::update(&f.state, values, vec!["n".into()]);
        f.tick();
        f.text("text:edit", "本地");
        f.click("enabled:boolean");
        f.tick();
        assert_eq!(f.value("text"), "本地");
        assert_eq!(f.value("n"), 3);
        // 非文本 Choice 会等待 Application 的原有效值刷新；不能先把旧 linked 解锁。
        let binding = f.binding("n:edit");
        assert!(!IsWindowEnabled(binding.hwnd).as_bool());
        assert_eq!(controls::text(binding.hwnd).unwrap(), "8");
    }
}
#[test]
fn all_sixteen_dirty_fields_fit_one_bounded_batch_before_run() {
    unsafe {
        let params = (0..16)
            .map(|i| serde_json::json!({"id":format!("p{i}"),"type":"string","default":""}))
            .collect::<Vec<_>>();
        let f = Fixture::with_parameters(params.into());
        for i in 0..16 {
            f.text(&format!("p{i}:edit"), &format!("value{i}"));
        }
        f.click("run");
        assert_eq!(f.panel.ui.pending.borrow().len(), 2);
        f.tick();
        for i in 0..16 {
            assert_eq!(f.value(&format!("p{i}")), format!("value{i}"));
        }
        assert!(f.panel.exit_idle());
    }
}
#[test]
fn untouched_multiline_lf_and_crlf_do_not_create_drafts_or_block_session_end() {
    unsafe {
        for value in ["a\nb", "a\r\nb", "中文🙂\r\n"] {
            let f = Fixture::with_parameters(
                serde_json::json!([{"id":"body","widget":"textarea","default":value}]),
            );
            assert!(f.panel.exit_idle(), "{value:?}");
            f.click("run");
            f.tick();
            assert_eq!(f.value("body"), value);
        }
    }
}
#[test]
fn compact_json_expansion_cannot_reinterpret_a_following_choice_index() {
    unsafe {
        let f = Fixture::with_parameters(serde_json::json!([
            {"id":"payload","type":"json","multiline":true,"default":{}},
            {"id":"flag","type":"boolean","default":false},
            {"id":"other","type":"boolean","default":false}
        ]));
        let text = serde_json::to_string(&vec![0; 4000]).unwrap();
        assert!(text.len() < desktop::parameter_value::MAX_TEXT);
        assert!(
            serde_json::to_string_pretty(&vec![0; 4000]).unwrap().len()
                > desktop::parameter_value::MAX_TEXT
        );
        f.text("payload:edit", &text);
        f.click("flag:boolean");
        f.tick();
        assert_eq!(f.value("flag"), true);
        assert_eq!(f.value("other"), false);
        assert_eq!(f.value("payload"), serde_json::json!(vec![0; 4000]));
    }
}
