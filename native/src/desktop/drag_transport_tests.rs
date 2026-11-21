//! 真实 COM 数据对象往返；不调用 DoDragDrop，不操作系统剪贴板或用户文件。
use super::*;
#[path = "drag_shell_tests.rs"]
mod shell;

#[test]
fn shell_preview_attaches_without_dragging_and_reclaims_gdi_handles() {
    use windows::Win32::System::Threading::{GetCurrentProcess, GetGuiResources, GR_GDIOBJECTS};
    let _apartment = Apartment::new().unwrap();
    let preview = crate::desktop::drag_preview::Preview {
        width: 2,
        height: 2,
        pixels: vec![255; 16].into(),
    };
    let attach = || {
        let payload = data::prepare(&[PathBuf::from("C:/不存在/preview.png")]).unwrap();
        super::preview::attach(&payload.object, &preview).unwrap();
        unsafe {
            payload.object.QueryGetData(&data::format()).unwrap();
        }
    };
    attach(); // 预热 Shell 延迟初始化，不把首次缓存算作泄漏。
    let before = unsafe { GetGuiResources(GetCurrentProcess(), GR_GDIOBJECTS) };
    for _ in 0..16 {
        attach();
    }
    let after = unsafe { GetGuiResources(GetCurrentProcess(), GR_GDIOBJECTS) };
    assert!(
        after <= before + 2,
        "GDI resources grew: {before} -> {after}"
    );
    let payload = data::prepare(&[PathBuf::from("C:/不存在/preview.png")]).unwrap();
    let invalid = crate::desktop::drag_preview::Preview {
        width: 257,
        height: 1,
        pixels: vec![255; 1028].into(),
    };
    assert!(super::preview::attach(&payload.object, &invalid).is_err());
}

#[test]
fn source_prioritizes_cancellation_and_only_left_button_release_drops() {
    let flag = Arc::new(AtomicBool::new(false));
    let source: IDropSource = Source {
        cancel: flag.clone(),
    }
    .into();
    unsafe {
        assert_eq!(source.QueryContinueDrag(false, MK_LBUTTON), S_OK);
        assert_eq!(source.QueryContinueDrag(false, MK_LBUTTON | MK_SHIFT), S_OK);
        assert_eq!(
            source.QueryContinueDrag(false, MODIFIERKEYS_FLAGS(0)),
            DRAGDROP_S_DROP
        );
        assert_eq!(
            source.QueryContinueDrag(true, MK_LBUTTON),
            DRAGDROP_S_CANCEL
        );
        assert_eq!(
            source.QueryContinueDrag(false, MK_LBUTTON | MK_RBUTTON),
            DRAGDROP_S_CANCEL
        );
        flag.store(true, Ordering::Release);
        assert_eq!(
            source.QueryContinueDrag(false, MODIFIERKEYS_FLAGS(0)),
            DRAGDROP_S_CANCEL
        );
        assert_eq!(
            source.GiveFeedback(DROPEFFECT_COPY),
            DRAGDROP_S_USEDEFAULTCURSORS
        );
    }
}

#[test]
fn only_confirmed_transfer_is_success_and_errors_are_preserved() {
    assert_eq!(
        outcome(DRAGDROP_S_DROP, DROPEFFECT_COPY).unwrap(),
        Outcome::Copied
    );
    assert_eq!(
        outcome(DRAGDROP_S_DROP, DROPEFFECT_NONE).unwrap(),
        Outcome::Cancelled
    );
    assert_eq!(
        outcome(DRAGDROP_S_CANCEL, DROPEFFECT_COPY).unwrap(),
        Outcome::Cancelled
    );
    assert_eq!(
        outcome(DRAGDROP_S_DROP, DROPEFFECT_MOVE).unwrap(),
        Outcome::Moved
    );
    assert!(outcome(S_OK, DROPEFFECT_COPY).is_err());
    assert_eq!(
        outcome(E_ACCESSDENIED, DROPEFFECT_NONE).unwrap_err().code(),
        E_ACCESSDENIED
    );
}

#[test]
fn pre_cancel_does_not_initialize_ole_or_validate_paths() {
    assert_eq!(
        drag_files(&[], Arc::new(AtomicBool::new(true))).unwrap(),
        Outcome::Cancelled
    );
}

#[test]
fn logical_shell_receipt_overrides_cleanup_effect_but_never_overrides_cancel_or_error() {
    use windows::Win32::System::Memory::*;
    unsafe {
        let _apartment = Apartment::new().unwrap();
        let object: windows::Win32::System::Com::IDataObject =
            windows::Win32::UI::Shell::SHCreateDataObject(None, None, None).unwrap();
        assert!(data::logical_effect(&object).unwrap().is_none());
        assert_eq!(
            completion(DRAGDROP_S_DROP, DROPEFFECT_COPY, &object).unwrap(),
            Outcome::Copied
        );
        let memory = GlobalAlloc(GMEM_MOVEABLE | GMEM_ZEROINIT, 4).unwrap();
        let pointer = GlobalLock(memory);
        assert!(!pointer.is_null());
        std::ptr::write_unaligned(pointer.cast::<u32>(), DROPEFFECT_MOVE.0);
        let _ = GlobalUnlock(memory);
        let medium = windows::Win32::System::Com::STGMEDIUM {
            tymed: windows::Win32::System::Com::TYMED_HGLOBAL.0 as u32,
            u: windows::Win32::System::Com::STGMEDIUM_0 { hGlobal: memory },
            ..Default::default()
        };
        object
            .SetData(&data::logical_format().unwrap(), &medium, true)
            .unwrap();
        assert_eq!(
            completion(DRAGDROP_S_DROP, DROPEFFECT_NONE, &object).unwrap(),
            Outcome::Moved
        );
        assert_eq!(
            completion(DRAGDROP_S_CANCEL, DROPEFFECT_NONE, &object).unwrap(),
            Outcome::Cancelled
        );
        assert!(completion(E_FAIL, DROPEFFECT_NONE, &object).is_err());
    }
}

#[test]
fn shell_data_object_owns_unicode_packet_and_survives_preparation_scope() {
    use windows::Win32::System::Memory::*;
    let _apartment = Apartment::new().unwrap();
    let paths = vec![
        PathBuf::from("C:/不存在/原生 🦦.png"),
        PathBuf::from(r"\\host\share\image.png"),
    ];
    let payload = data::prepare(&paths).unwrap();
    unsafe {
        payload.object.QueryGetData(&data::format()).unwrap();
        // Shell 可从 CF_HDROP 派生文本等格式，不假设对象只宣告一种格式。
        // SetData 已接管 HGLOBAL；两次获取后分别释放 medium 不得破坏数据对象。
        for _ in 0..2 {
            let mut medium = payload.object.GetData(&data::format()).unwrap();
            let memory = medium.u.hGlobal;
            let pointer = GlobalLock(memory);
            assert!(!pointer.is_null());
            let bytes = std::slice::from_raw_parts(pointer.cast(), GlobalSize(memory));
            let expected = super::super::paste_input::file::encode(&paths).unwrap();
            let matches = bytes.starts_with(&expected)
                && bytes[expected.len()..].iter().all(|byte| *byte == 0);
            let _ = GlobalUnlock(memory);
            ReleaseStgMedium(&mut medium);
            assert!(matches);
        }
    }
    assert!(data::prepare(&[]).is_err());
    assert!(data::prepare(&[PathBuf::from("relative.png")]).is_err());
}
