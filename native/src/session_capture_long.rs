//! 原生长截图的可选来源扩展；保留未知轴，方向来自已接受位移而非滚轮提示。
use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Recording {
    pub frame_count: usize,
    pub direction: Option<Direction>,
    pub spatial_position: Option<[i64; 2]>,
}
impl Recording {
    pub(super) fn valid(&self, kind: Option<super::Kind>, axis: Option<super::Axis>) -> bool {
        if kind != Some(super::Kind::Long)
            || self.frame_count == 0
            || self.frame_count > crate::long_capture::MAX_FRAMES
            || self.spatial_position.is_some_and(|point| {
                point
                    .iter()
                    .any(|v| v.unsigned_abs() > 9_007_199_254_740_991)
            })
        {
            return false;
        }
        // 单轴会话方向须与已确定轴一致；显式二维移动可带独立空间坐标。
        if self.spatial_position.is_none() {
            match (axis, self.direction) {
                (None, Some(_)) => return false,
                (Some(super::Axis::Horizontal), Some(Direction::Up | Direction::Down)) => {
                    return false
                }
                (Some(super::Axis::Vertical), Some(Direction::Left | Direction::Right)) => {
                    return false
                }
                _ => {}
            }
        }
        true
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::session_capture::{decode, Axis, Kind};
    #[test]
    fn recording_metadata_preserves_unknown_axis_and_checked_motion() {
        let mut recording = Recording {
            frame_count: 1,
            direction: None,
            spatial_position: None,
        };
        assert!(recording.valid(Some(Kind::Long), None));
        assert!(!recording.valid(Some(Kind::Region), None));
        recording.direction = Some(Direction::Down);
        assert!(!recording.valid(Some(Kind::Long), None));
        assert!(recording.valid(Some(Kind::Long), Some(Axis::Vertical)));
        assert!(!recording.valid(Some(Kind::Long), Some(Axis::Horizontal)));
        recording.spatial_position = Some([-12, 20]);
        assert!(recording.valid(Some(Kind::Long), Some(Axis::Horizontal)));
        recording.spatial_position = Some([i64::MIN, 0]);
        assert!(!recording.valid(Some(Kind::Long), Some(Axis::Horizontal)));
    }
    #[test]
    fn recording_metadata_decode_rejects_invalid_counts_and_keeps_legacy() {
        assert!(decode(r#"{"kind":"long"}"#).unwrap().recording.is_none());
        for count in [0, crate::long_capture::MAX_FRAMES + 1] {
            let raw = serde_json::json!({"kind":"long","otterNativeRecording":{"frameCount":count,"direction":null,"spatialPosition":null}});
            assert!(decode(&raw.to_string()).is_err());
        }
        let raw = r#"{"kind":"long","otterNativeRecording":{"frameCount":1,"direction":null,"spatialPosition":null}}"#;
        assert_eq!(decode(raw).unwrap().recording.unwrap().frame_count, 1);
    }
}
