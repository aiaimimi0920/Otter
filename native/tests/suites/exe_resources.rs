//! Map the actual built EXE as read-only resources without executing its entry point or reading user files.
#![cfg(windows)]
use std::os::windows::ffi::OsStrExt;
use windows::{
    core::PCWSTR,
    Win32::{Foundation::*, System::LibraryLoader::*, UI::WindowsAndMessaging::*},
};

struct Module(HMODULE);
impl Drop for Module {
    fn drop(&mut self) {
        unsafe {
            let _ = FreeLibrary(self.0);
        }
    }
}
unsafe fn resource(module: &Module, kind: PCWSTR, id: usize) -> Vec<u8> {
    // MAKEINTRESOURCEW is a low-order integer resource ID, not a dereferenceable string address.
    assert!(id > 0 && id <= u16::MAX as usize);
    let entry = FindResourceW(
        Some(module.0),
        PCWSTR(std::ptr::without_provenance(id)),
        kind,
    );
    assert!(!entry.is_invalid(), "Missing resource");
    let length = SizeofResource(Some(module.0), entry) as usize;
    assert!(length > 0 && length < 4 * 1024 * 1024);
    let loaded = LoadResource(Some(module.0), entry).unwrap();
    let data = LockResource(loaded);
    assert!(!data.is_null());
    std::slice::from_raw_parts(data.cast::<u8>(), length).to_vec()
}
#[test]
fn built_executable_contains_brand_icon_and_package_version() {
    let path: Vec<u16> = std::ffi::OsStr::new(env!("CARGO_BIN_EXE_otter-native"))
        .encode_wide()
        .chain([0])
        .collect();
    unsafe {
        let module = Module(
            LoadLibraryExW(
                PCWSTR(path.as_ptr()),
                None,
                LOAD_LIBRARY_AS_DATAFILE | LOAD_LIBRARY_AS_IMAGE_RESOURCE,
            )
            .unwrap(),
        );
        let icons = resource(&module, RT_GROUP_ICON, 1);
        assert_eq!(&icons[..4], &[0, 0, 1, 0]);
        let source = include_bytes!("../../assets/branding/icon.ico");
        let count = u16::from_le_bytes([icons[4], icons[5]]) as usize;
        assert!(count > 0);
        assert_eq!(&icons[..6], &source[..6]);
        // Validate actual image bytes at every size; an arbitrary icon or merely present group placeholder must not pass.
        for index in 0..count {
            let entry = &source[6 + index * 16..6 + (index + 1) * 16];
            let group = &icons[6 + index * 14..6 + (index + 1) * 14];
            assert_eq!(&entry[..4], &group[..4]);
            assert_eq!(&entry[8..12], &group[8..12]);
            let length = u32::from_le_bytes(entry[8..12].try_into().unwrap()) as usize;
            let offset = u32::from_le_bytes(entry[12..16].try_into().unwrap()) as usize;
            let id = u16::from_le_bytes(group[12..14].try_into().unwrap()) as usize;
            let image = &source[offset..offset + length];
            // RC reconstructs planes/bitcount from actual image headers and does not guarantee copying zero values from the ICO directory.
            let format = if image.starts_with(b"\x89PNG\r\n\x1a\n") {
                &[1, 0, 32, 0][..]
            } else {
                &image[12..16]
            };
            assert_eq!(&group[4..8], format);
            assert_eq!(resource(&module, RT_ICON, id), image);
        }
        let version = resource(&module, RT_VERSION, 1);
        let key: Vec<u8> = "VS_VERSION_INFO"
            .encode_utf16()
            .chain([0])
            .flat_map(u16::to_le_bytes)
            .collect();
        assert_eq!(&version[6..6 + key.len()], key.as_slice());
        let start = (6 + key.len()).next_multiple_of(4);
        let word = |offset: usize| {
            u32::from_le_bytes(
                version[start + offset..start + offset + 4]
                    .try_into()
                    .unwrap(),
            )
        };
        assert_eq!(word(0), 0xfeef04bd);
        let expected: Vec<u32> = env!("CARGO_PKG_VERSION")
            .split('.')
            .map(|part| part.parse().unwrap())
            .collect();
        assert_eq!(word(8), (expected[0] << 16) | expected[1]);
        assert_eq!(word(12), expected[2] << 16);
        assert_eq!(word(16), word(8));
        assert_eq!(word(20), word(12));
    }
}
