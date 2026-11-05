//! 有界 DisplayConfig 查询；最多两次尝试，不将读取失败伪装为系统实际白电平。
use windows::Win32::{Devices::Display::*, Foundation::ERROR_INSUFFICIENT_BUFFER};
pub(super) fn query(device_name: &[u16; 32]) -> Option<(bool, Option<u32>)> {
    for _ in 0..2 {
        let (mut path_count, mut mode_count) = (0, 0);
        if unsafe {
            GetDisplayConfigBufferSizes(QDC_ONLY_ACTIVE_PATHS, &mut path_count, &mut mode_count)
        }
        .0 != 0
            || !valid_counts(path_count, mode_count)
        {
            return None;
        }
        let (path_capacity, mode_capacity) = (path_count, mode_count);
        let mut paths = Vec::new();
        let mut modes = Vec::new();
        paths.try_reserve_exact(path_count as usize).ok()?;
        modes.try_reserve_exact(mode_count as usize).ok()?;
        paths.resize(path_count as usize, DISPLAYCONFIG_PATH_INFO::default());
        modes.resize(mode_count as usize, DISPLAYCONFIG_MODE_INFO::default());
        let result = unsafe {
            QueryDisplayConfig(
                QDC_ONLY_ACTIVE_PATHS,
                &mut path_count,
                paths.as_mut_ptr(),
                &mut mode_count,
                modes.as_mut_ptr(),
                None,
            )
        };
        if result == ERROR_INSUFFICIENT_BUFFER {
            continue;
        }
        if result.0 != 0 || path_count > path_capacity || mode_count > mode_capacity {
            return None;
        }
        for path in &paths[..path_count as usize] {
            if let Some(value) = target(path, device_name) {
                return Some(value);
            }
        }
        return None;
    }
    None
}
fn valid_counts(paths: u32, modes: u32) -> bool {
    (1..=128).contains(&paths) && (1..=512).contains(&modes)
}
fn target(path: &DISPLAYCONFIG_PATH_INFO, device_name: &[u16; 32]) -> Option<(bool, Option<u32>)> {
    let mut source = DISPLAYCONFIG_SOURCE_DEVICE_NAME {
        header: DISPLAYCONFIG_DEVICE_INFO_HEADER {
            r#type: DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME,
            size: std::mem::size_of::<DISPLAYCONFIG_SOURCE_DEVICE_NAME>() as u32,
            adapterId: path.sourceInfo.adapterId,
            id: path.sourceInfo.id,
        },
        ..Default::default()
    };
    if unsafe { DisplayConfigGetDeviceInfo(&mut source.header) } != 0
        || !same_name(&source.viewGdiDeviceName, device_name)
    {
        return None;
    }
    let header = |r#type, size| DISPLAYCONFIG_DEVICE_INFO_HEADER {
        r#type,
        size,
        adapterId: path.targetInfo.adapterId,
        id: path.targetInfo.id,
    };
    let mut advanced = DISPLAYCONFIG_GET_ADVANCED_COLOR_INFO {
        header: header(
            DISPLAYCONFIG_DEVICE_INFO_GET_ADVANCED_COLOR_INFO,
            std::mem::size_of::<DISPLAYCONFIG_GET_ADVANCED_COLOR_INFO>() as u32,
        ),
        ..Default::default()
    };
    if unsafe { DisplayConfigGetDeviceInfo(&mut advanced.header) } != 0 {
        return None;
    }
    let enabled = unsafe { advanced.Anonymous.value } & 2 != 0;
    let mut white = DISPLAYCONFIG_SDR_WHITE_LEVEL {
        header: header(
            DISPLAYCONFIG_DEVICE_INFO_GET_SDR_WHITE_LEVEL,
            std::mem::size_of::<DISPLAYCONFIG_SDR_WHITE_LEVEL>() as u32,
        ),
        ..Default::default()
    };
    let level = (unsafe { DisplayConfigGetDeviceInfo(&mut white.header) } == 0)
        .then_some(white.SDRWhiteLevel);
    Some((enabled, level))
}
fn same_name(left: &[u16], right: &[u16]) -> bool {
    let trim = |value: &[u16]| value.iter().position(|c| *c == 0).unwrap_or(value.len());
    let a = &left[..trim(left)];
    let b = &right[..trim(right)];
    let fold = |c: u16| if (65..=90).contains(&c) { c + 32 } else { c };
    !a.is_empty() && a.len() == b.len() && a.iter().zip(b).all(|(a, b)| fold(*a) == fold(*b))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn buffer_counts_are_bounded_and_device_names_ignore_ascii_case_not_suffixes() {
        assert!(valid_counts(128, 512));
        for (paths, modes) in [(0, 1), (1, 0), (129, 1), (1, 513), (u32::MAX, u32::MAX)] {
            assert!(!valid_counts(paths, modes));
        }
        let name: Vec<_> = "\\\\.\\DISPLAY1".encode_utf16().collect();
        let lower: Vec<_> = "\\\\.\\display1\0suffix".encode_utf16().collect();
        assert!(same_name(&name, &lower));
        assert!(!same_name(
            &name,
            &"\\\\.\\DISPLAY10".encode_utf16().collect::<Vec<_>>()
        ));
        assert!(!same_name(&[], &[]));
    }
}
