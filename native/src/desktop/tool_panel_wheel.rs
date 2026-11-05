//! 系统滚轮偏好和亚像素余数；只读取设置，不修改系统状态，不建立轮询任务。
use windows::Win32::UI::WindowsAndMessaging::*;

pub(super) unsafe fn preference(horizontal: bool) -> u32 {
    let mut count = 3u32;
    let action = if horizontal {
        SPI_GETWHEELSCROLLCHARS
    } else {
        SPI_GETWHEELSCROLLLINES
    };
    if SystemParametersInfoW(
        action,
        0,
        Some((&mut count as *mut u32).cast()),
        SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS::default(),
    )
    .is_err()
    {
        return 3;
    }
    count
}
pub(super) fn distance(delta: i32, unit: i64, count: u32, page: u32, remainder: &mut i64) -> i64 {
    if count == 0 {
        *remainder = 0;
        return 0;
    }
    let step = if count == u32::MAX {
        page as i128
    } else {
        unit.max(0) as i128 * count as i128
    };
    // 使用宽整数计算；余数始终小于一个像素，即使异常配置也不溢出或无限累积。
    let numerator = delta as i128 * step + *remainder as i128;
    *remainder = (numerator % 120) as i64;
    (numerator / 120).clamp(i64::MIN as i128, i64::MAX as i128) as i64
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fractional_events_sum_to_full_notch_in_both_directions() {
        for sign in [-1, 1] {
            let mut remainder = 0;
            let fine: i64 = (0..120)
                .map(|_| distance(sign, 35, 3, 500, &mut remainder))
                .sum();
            assert_eq!(fine, 105 * sign as i64);
            assert_eq!(remainder, 0);
            assert_eq!(distance(sign * 120, 35, 3, 500, &mut remainder), fine);
        }
    }
    #[test]
    fn direction_changes_cancel_fraction_and_disabled_scroll_clears_it() {
        let mut remainder = 0;
        assert_eq!(distance(1, 35, 3, 500, &mut remainder), 0);
        assert_ne!(remainder, 0);
        assert_eq!(distance(-1, 35, 3, 500, &mut remainder), 0);
        assert_eq!(remainder, 0);
        distance(1, 35, 3, 500, &mut remainder);
        assert_eq!(distance(120, 35, 0, 500, &mut remainder), 0);
        assert_eq!(remainder, 0);
    }
    #[test]
    fn configured_counts_page_mode_and_extreme_values_are_bounded() {
        let mut remainder = 0;
        assert_eq!(distance(120, 8, 5, 500, &mut remainder), 40);
        assert_eq!(distance(-120, 35, u32::MAX, 423, &mut remainder), -423);
        assert_eq!(
            distance(i32::MAX, i64::MAX, u32::MAX - 1, 500, &mut remainder),
            i64::MAX
        );
        assert!(remainder.abs() < 120);
    }
}
