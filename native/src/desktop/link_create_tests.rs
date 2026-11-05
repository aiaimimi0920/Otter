//! 使用实际工作区目录验证隐藏系统对话框，不运行可见模态循环或修改工作区。
use super::*;

pub(crate) unsafe fn verify_outgoing(catalog: Rc<Catalog>, source: usize) {
    let choices: Vec<_> = catalog
        .endpoints
        .iter()
        .enumerate()
        .filter(|(index, endpoint)| *index != source && !endpoint.inputs.is_empty())
        .map(|(index, _)| index)
        .collect();
    assert!(choices.len() >= 2);
    let mut dialog = Dialog {
        outgoing: true,
        catalog,
        choices,
        fixed: source,
        result: None,
    };
    let template = template(true);
    let hwnd = CreateDialogIndirectParamW(
        None,
        template.as_ptr().cast(),
        None,
        Some(procedure),
        LPARAM(&mut dialog as *mut _ as isize),
    )
    .unwrap();
    assert!(!IsWindowVisible(hwnd).as_bool());
    let choice = GetDlgItem(Some(hwnd), SOURCE).unwrap();
    let output = GetDlgItem(Some(hwnd), OUTPUT).unwrap();
    let input = GetDlgItem(Some(hwnd), INPUT).unwrap();
    for (selection, target) in dialog.choices.iter().copied().enumerate() {
        SendMessageW(choice, CB_SETCURSEL, Some(WPARAM(selection)), None);
        SendMessageW(
            hwnd,
            WM_COMMAND,
            Some(WPARAM(SOURCE as usize | ((CBN_SELCHANGE as usize) << 16))),
            Some(LPARAM(choice.0 as isize)),
        );
        assert_eq!(endpoints(&dialog, target), (source, target));
        assert_eq!(
            SendMessageW(output, CB_GETCOUNT, None, None).0 as usize,
            dialog.catalog.endpoints[source].outputs.len()
        );
        assert_eq!(
            SendMessageW(input, CB_GETCOUNT, None, None).0 as usize,
            dialog.catalog.endpoints[target].inputs.len()
        );
        assert_eq!(selected(hwnd, INPUT), Some(0));
        assert!(dialog.result.is_none());
    }
    DestroyWindow(hwnd).unwrap();
    for control in [choice, output, input] {
        assert!(!IsWindow(Some(control)).as_bool());
    }
}

pub(crate) unsafe fn verify(catalog: Rc<Catalog>, target: usize) {
    let choices: Vec<_> = catalog
        .endpoints
        .iter()
        .enumerate()
        .filter(|(index, endpoint)| *index != target && !endpoint.outputs.is_empty())
        .map(|(index, _)| index)
        .collect();
    assert!(
        choices.len() >= 2,
        "fixture must exercise switching source windows"
    );
    let mut dialog = Dialog {
        outgoing: false,
        catalog,
        choices,
        fixed: target,
        result: None,
    };
    let template = template(false);
    let hwnd = CreateDialogIndirectParamW(
        None,
        template.as_ptr().cast(),
        None,
        Some(procedure),
        LPARAM(&mut dialog as *mut _ as isize),
    )
    .unwrap();
    assert!(!IsWindowVisible(hwnd).as_bool());
    let source = GetDlgItem(Some(hwnd), SOURCE).unwrap();
    let output = GetDlgItem(Some(hwnd), OUTPUT).unwrap();
    let input = GetDlgItem(Some(hwnd), INPUT).unwrap();
    assert_eq!(
        SendMessageW(source, CB_GETCOUNT, None, None).0 as usize,
        dialog.choices.len()
    );
    assert_eq!(
        SendMessageW(input, CB_GETCOUNT, None, None).0 as usize,
        dialog.catalog.endpoints[target].inputs.len()
    );
    let retained_input = dialog.catalog.endpoints[target].inputs.len() - 1;
    SendMessageW(input, CB_SETCURSEL, Some(WPARAM(retained_input)), None);
    for (selection, index) in dialog.choices.iter().copied().enumerate() {
        assert_ne!(index, target);
        SendMessageW(source, CB_SETCURSEL, Some(WPARAM(selection)), None);
        SendMessageW(
            hwnd,
            WM_COMMAND,
            Some(WPARAM(SOURCE as usize | ((CBN_SELCHANGE as usize) << 16))),
            Some(LPARAM(source.0 as isize)),
        );
        let ports = &dialog.catalog.endpoints[index].outputs;
        assert_eq!(
            SendMessageW(output, CB_GETCOUNT, None, None).0 as usize,
            ports.len()
        );
        assert_eq!(selected(hwnd, OUTPUT), Some(0));
        assert_eq!(selected(hwnd, INPUT), Some(retained_input));
        for (port_index, port) in ports.iter().enumerate() {
            let length = SendMessageW(output, CB_GETLBTEXTLEN, Some(WPARAM(port_index)), None).0;
            assert!(length >= 0);
            let mut buffer = vec![0u16; length as usize + 1];
            let copied = SendMessageW(
                output,
                CB_GETLBTEXT,
                Some(WPARAM(port_index)),
                Some(LPARAM(buffer.as_mut_ptr() as isize)),
            )
            .0;
            assert_eq!(copied, length);
            assert_eq!(
                String::from_utf16(&buffer[..length as usize]).unwrap(),
                *port
            );
        }
        assert!(
            dialog.result.is_none(),
            "changing selection must not commit"
        );
    }
    DestroyWindow(hwnd).unwrap();
    assert!(!IsWindow(Some(source)).as_bool());
    assert!(!IsWindow(Some(output)).as_bool());
    assert!(!IsWindow(Some(input)).as_bool());
    assert!(dialog.result.is_none());
}
