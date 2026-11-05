//! 只接受Loom真实Source video_policy；不把Viewer请求或滚退epoch当控制权限。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct VideoPolicy {
    pub epoch: u64,
    pub h264_allowed: bool,
    pub keyframe_sequence: u64,
}
#[derive(serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
enum Wire {
    VideoPolicy {
        epoch: u64,
        h264_allowed: bool,
        keyframe_sequence: u64,
    },
}
impl VideoPolicy {
    pub(super) fn parse(text: &str) -> Option<Self> {
        if text.len() > 256 {
            return None;
        }
        let Wire::VideoPolicy {
            epoch,
            h264_allowed,
            keyframe_sequence,
        } = serde_json::from_str(text).ok()?;
        (epoch > 0).then_some(Self {
            epoch,
            h264_allowed,
            keyframe_sequence,
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_exact_bounded_source_policy_is_accepted() {
        let text = r#"{"type":"video_policy","epoch":1,"h264_allowed":true,"keyframe_sequence":7}"#;
        assert_eq!(
            VideoPolicy::parse(text),
            Some(VideoPolicy {
                epoch: 1,
                h264_allowed: true,
                keyframe_sequence: 7
            })
        );
        for text in [
            text.replace("\"epoch\":1", "\"epoch\":0"),
            text.replace("true", "1"),
            text.replace("7}", "-1}"),
            text.replace("7}", "7,\"deviceId\":\"other\"}"),
            text.replace("7}", "7,\"keyframe_sequence\":8}"),
            format!("{text}{}", " ".repeat(256)),
            r#"{"type":"keyframe_request","epoch":1}"#.into(),
        ] {
            assert!(VideoPolicy::parse(&text).is_none(), "{text}");
        }
    }
}
