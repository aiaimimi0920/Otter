//! 当前外观覆盖替换旧原文，且只有外观的初始宿主也必须写入。
use super::tests::{data, doc};
use super::*;
use crate::{session_source::Source, workspace::ports::Visibility, workspace_id::UnitId};
#[test]
fn save_reopen_preserves_overrides_unknown_fields_and_reset_does_not_resurrect_old_map() {
    for reset in [false, true] {
        let mut snapshot = data(vec![doc("unit")]);
        snapshot.original = Source::from(br#"{"futureRoot":{"keep":1},"stickers":[{"id":"unit","x":0,"y":0,"w":40,"h":30,"portVisibility":{"old":false},"futureWindow":17}],"links":[]}"#.to_vec());
        if !reset {
            snapshot.port_visibility.push((
                UnitId::parse("unit").unwrap(),
                Visibility::read(r#"{"output":false,"中文":true}"#).unwrap(),
            ));
        }
        let candidate = snapshot
            .assemble(&AtomicBool::new(false), |_, fields| {
                Ok(WindowCandidate::new(fields))
            })
            .unwrap();
        let json: Value = serde_json::from_slice(&candidate.bytes).unwrap();
        assert_eq!(json["futureRoot"]["keep"], 1);
        assert_eq!(json["stickers"][0]["futureWindow"], 17);
        let reopened = crate::session_document::decode(&candidate.bytes).unwrap();
        let visibility = reopened
            .workspace
            .port_visibility(&UnitId::parse("unit").unwrap());
        assert_eq!(visibility.get("old"), None);
        assert_eq!(visibility.get("output"), (!reset).then_some(false));
        assert_eq!(json["stickers"][0].get("portVisibility").is_none(), reset);
    }
}
#[test]
fn appearance_only_initial_host_is_not_omitted_and_plain_empty_host_still_is() {
    for edited in [false, true] {
        let mut snapshot = data(vec![doc("unit")]);
        snapshot.transient_host = Some(UnitId::parse("unit").unwrap());
        if edited {
            snapshot.port_visibility.push((
                UnitId::parse("unit").unwrap(),
                Visibility::read(r#"{"output":false}"#).unwrap(),
            ));
        }
        let candidate = snapshot
            .assemble(&AtomicBool::new(false), |_, fields| {
                Ok(WindowCandidate::new(fields))
            })
            .unwrap();
        let reopened = crate::session_document::decode(&candidate.bytes).unwrap();
        assert_eq!(reopened.windows.len(), usize::from(edited));
    }
}
