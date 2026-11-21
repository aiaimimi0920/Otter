//! 对真实 Shell 文件夹目标交付本测试的导出副本；不运行鼠标拖动、不触碰用户目录。
use super::*;
use std::{
    fs,
    fs::OpenOptions,
    os::windows::fs::OpenOptionsExt,
    time::{Duration, Instant},
};
use windows::{
    core::HSTRING,
    Win32::{Storage::FileSystem::*, System::Com::CoCreateGuid, UI::Shell::*},
};

#[test]
fn shift_drop_moves_only_the_export_copy_and_retains_original_and_cache_lease() {
    unsafe {
        let _apartment = Apartment::new().unwrap();
        // Shell parsing names 使用本机反斜杠路径；Win32 文件 IO 接受的混合分隔符不是 Shell 合同。
        let root = PathBuf::from(r"C:\Users\Public\nas_home\AI\GameEditor\linshi")
            .join(format!("otter-shell-drop-{:?}", CoCreateGuid().unwrap()));
        let folder = root.join("target");
        fs::create_dir_all(&folder).unwrap();
        let bytes = b"\x89PNG\r\n\x1a\nexport snapshot";
        let original = root.join("original.png");
        fs::write(&original, bytes).unwrap();
        let signal = AtomicBool::new(false);
        let state = crate::desktop::State::new(
            Some(crate::desktop::image::Image {
                width: 2,
                height: 2,
                pixels: vec![255; 16].into(),
            }),
            true,
        )
        .unwrap();
        let name = crate::desktop::export_naming::suggested_drag(&state).unwrap();
        let stem = name.strip_suffix(".png").unwrap();
        let prepared =
            crate::clipboard_cache::stage(&root, stem, bytes.to_vec().into(), &signal).unwrap();
        let (export, lease) = prepared.commit_drag(&signal).unwrap();
        let file = OpenOptions::new()
            .read(true)
            .share_mode(FILE_SHARE_READ.0 | FILE_SHARE_DELETE.0)
            .open(&export)
            .unwrap();
        let payload = data::prepare(std::slice::from_ref(&export)).unwrap();
        let item: IShellItem =
            SHCreateItemFromParsingName(&HSTRING::from(folder.as_os_str()), None).unwrap();
        let target: IDropTarget = item.BindToHandler(None, &BHID_SFUIObject).unwrap();
        let point = POINTL::default();
        let mut effect = DROPEFFECT_COPY;
        target
            .DragEnter(&payload.object, MK_LBUTTON | MK_SHIFT, point, &mut effect)
            .unwrap();
        target.DragLeave().unwrap();
        assert_eq!(
            effect, DROPEFFECT_NONE,
            "COPY-only is rejected for a Shift MOVE"
        );
        effect = DROPEFFECT_COPY | DROPEFFECT_MOVE;
        target
            .DragEnter(&payload.object, MK_LBUTTON | MK_SHIFT, point, &mut effect)
            .unwrap();
        assert_eq!(effect, DROPEFFECT_MOVE);
        target
            .Drop(&payload.object, MK_SHIFT, point, &mut effect)
            .unwrap();
        // 优化移动已由目标删除副本，pdwEffect 可为 NONE；仍须确认 Shell 的逻辑 MOVE 回执。
        assert_eq!(
            data::logical_effect(&payload.object).unwrap(),
            Some(DROPEFFECT_MOVE)
        );
        assert_eq!(
            completion(DRAGDROP_S_DROP, effect, &payload.object).unwrap(),
            Outcome::Moved
        );
        let destination = folder.join(&name);
        let deadline = Instant::now() + Duration::from_secs(5);
        while !destination.exists() || export.exists() {
            assert!(
                Instant::now() < deadline,
                "Shell did not persist the export"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
        assert_eq!(fs::read(&destination).unwrap(), bytes);
        assert_eq!(fs::read(&original).unwrap(), bytes);
        assert!(OpenOptions::new()
            .write(true)
            .open(root.join("clipboard_cache/cache.lock"))
            .is_err());
        drop((file, lease, payload, target, item));
        // 留存本次 linshi 输出以便复核，不做目录清理或用户文件删除。
    }
}
