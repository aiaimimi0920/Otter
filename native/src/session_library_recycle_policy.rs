//! 回收数量/天数策略；与导入硬上限分离，未知旧日期保留而非猜测删除。
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Policy {
    pub recycle_bin_max_entries: usize,
    pub recycle_bin_retention_days: u32,
}
impl Default for Policy {
    fn default() -> Self {
        Self {
            recycle_bin_max_entries: 15,
            recycle_bin_retention_days: 0,
        }
    }
}
impl Policy {
    pub fn validate(self) -> Result<Self, &'static str> {
        if self.recycle_bin_max_entries > 500 || self.recycle_bin_retention_days > 3650 {
            return Err("Recycle policy exceeds 500 entries or 3650 days");
        }
        Ok(self)
    }
    pub(in crate::session_library) fn retains(self, created_at: &str, now_ms: i64) -> bool {
        self.recycle_bin_retention_days == 0
            || timestamp(created_at).is_none_or(|created| {
                created
                    >= now_ms
                        .saturating_sub(i64::from(self.recycle_bin_retention_days) * 86_400_000)
            })
    }
}

// 接受应用生成的 ISO UTC 及带显式偏移/纯日期旧值；不猜无时区或本地化日期。
// 年份限定四位，全部运算在 i64 范围内；不使用系统当前时区和区域设置。
fn timestamp(value: &str) -> Option<i64> {
    let bytes = value.as_bytes();
    fn number(bytes: &[u8]) -> Option<i64> {
        bytes.iter().try_fold(0, |value, digit| {
            digit
                .is_ascii_digit()
                .then_some(value * 10 + i64::from(digit.wrapping_sub(b'0')))
        })
    }
    if bytes.len() < 10 || bytes[4] != b'-' || bytes[7] != b'-' {
        return None;
    }
    let year = number(&bytes[..4])?;
    let month = usize::try_from(number(&bytes[5..7])?).ok()?;
    let day = number(&bytes[8..10])?;
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let months = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    if !(1..=12).contains(&month) || day < 1 || day > months[month - 1] {
        return None;
    }
    let before_year =
        |year: i64| 365 * year + (year + 3) / 4 - (year + 99) / 100 + (year + 399) / 400;
    let days =
        before_year(year) - before_year(1970) + months[..month - 1].iter().sum::<i64>() + day - 1;
    if bytes.len() == 10 {
        return Some(days * 86_400_000);
    }
    if bytes.len() < 20
        || !matches!(bytes[10], b'T' | b't')
        || bytes[13] != b':'
        || bytes[16] != b':'
    {
        return None;
    }
    let hour = number(&bytes[11..13])?;
    let minute = number(&bytes[14..16])?;
    let second = number(&bytes[17..19])?;
    if hour > 23 || minute > 59 || second > 59 {
        return None;
    }
    let mut index = 19;
    let mut millis = 0;
    if bytes[index] == b'.' {
        index += 1;
        let start = index;
        while index < bytes.len() && bytes[index].is_ascii_digit() {
            if index - start < 3 {
                millis = millis * 10 + i64::from(bytes[index] - b'0');
            }
            index += 1;
        }
        if index == start {
            return None;
        }
        for _ in (index - start)..3 {
            millis *= 10;
        }
    }
    let offset = match bytes.get(index)? {
        b'Z' | b'z' if index + 1 == bytes.len() => 0,
        sign @ (b'+' | b'-') if index + 6 == bytes.len() && bytes[index + 3] == b':' => {
            let hour = number(&bytes[index + 1..index + 3])?;
            let minute = number(&bytes[index + 4..])?;
            if hour > 23 || minute > 59 {
                return None;
            }
            (hour * 60 + minute) * if *sign == b'+' { 1 } else { -1 }
        }
        _ => return None,
    };
    Some(days * 86_400_000 + ((hour * 60 + minute - offset) * 60 + second) * 1000 + millis)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn utc_offsets_leap_dates_and_fractional_seconds_are_deterministic() {
        assert_eq!(timestamp("1970-01-01T00:00:00.000Z"), Some(0));
        assert_eq!(timestamp("1970-01-01T08:00:00+08:00"), Some(0));
        assert_eq!(timestamp("1969-12-31T23:59:59.999Z"), Some(-1));
        assert_eq!(timestamp("1970-01-01T00:00:00.1Z"), Some(100));
        assert_eq!(timestamp("1970-01-01T00:00:00.1234Z"), Some(123));
        assert_eq!(
            timestamp("2000-03-01").unwrap() - timestamp("2000-02-28").unwrap(),
            2 * 86_400_000
        );
        assert_eq!(
            timestamp("1900-03-01").unwrap() - timestamp("1900-02-28").unwrap(),
            86_400_000
        );
        for date in [
            "1900-02-29",
            "2026-13-01",
            "2026-04-31",
            "2026-01-01T24:00:00Z",
            "2026-01-01T00:00:00",
            "bad",
        ] {
            assert_eq!(timestamp(date), None);
        }
    }
    #[test]
    fn cutoff_is_inclusive_zero_disables_and_unknown_dates_are_retained() {
        let policy = Policy {
            recycle_bin_max_entries: 0,
            recycle_bin_retention_days: 1,
        };
        let now = timestamp("2026-10-23T00:00:00Z").unwrap();
        assert!(policy.retains("2026-10-22T00:00:00Z", now));
        assert!(!policy.retains("2026-10-21T23:59:59.999Z", now));
        assert!(policy.retains("legacy date", now));
        assert!(Policy::default().retains("1900-01-01", now));
        assert!(Policy {
            recycle_bin_max_entries: 501,
            ..policy
        }
        .validate()
        .is_err());
        assert!(Policy {
            recycle_bin_retention_days: 3651,
            ..policy
        }
        .validate()
        .is_err());
    }
}
