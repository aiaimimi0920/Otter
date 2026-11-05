//! 使用自建 Shell 数据包，不调用系统拖放、不读取用户文件或剪贴板。
use super::*;
use windows::Win32::System::Memory::*;

unsafe fn packet(names: &[&str]) -> DropFiles {
    let mut names: Vec<u16> = names
        .iter()
        .flat_map(|name| name.encode_utf16().chain([0]))
        .collect();
    names.push(0);
    if names.len() == 1 {
        names.push(0);
    }
    let header = std::mem::size_of::<DROPFILES>();
    let memory = GlobalAlloc(GMEM_MOVEABLE | GMEM_ZEROINIT, header + names.len() * 2).unwrap();
    let owned = DropFiles(HDROP(memory.0));
    let data = GlobalLock(memory).cast::<u8>();
    assert!(!data.is_null());
    data.cast::<DROPFILES>().write(DROPFILES {
        pFiles: header as u32,
        fWide: true.into(),
        ..Default::default()
    });
    std::ptr::copy_nonoverlapping(
        names.as_ptr().cast::<u8>(),
        data.add(header),
        names.len() * 2,
    );
    let _ = GlobalUnlock(memory);
    owned
}

#[test]
fn unicode_spaces_and_long_paths_are_not_truncated() {
    unsafe {
        for name in [
            r"C:\图片 空格\水獭.png".to_string(),
            format!("C:\\{}\\image.png", "目录".repeat(500)),
        ] {
            assert_eq!(paths(packet(&[&name])).unwrap(), vec![PathBuf::from(name)]);
        }
    }
}

#[test]
fn multi_file_order_is_preserved_and_invalid_batches_are_rejected() {
    unsafe {
        assert!(paths(packet(&[])).is_err());
        assert_eq!(
            paths(packet(&["first.png", "second.png"])).unwrap(),
            vec![PathBuf::from("first.png"), PathBuf::from("second.png")]
        );
        assert!(paths(packet(&["a.png"; 33])).is_err());
        assert!(paths(packet(&[&"a".repeat(32768)])).is_err());
    }
}

#[test]
fn decoder_does_not_access_or_require_existing_files() {
    unsafe {
        let name = r"C:\otter-nonexistent-test-path\absent.png";
        assert_eq!(paths(packet(&[name])).unwrap(), vec![PathBuf::from(name)]);
    }
}
