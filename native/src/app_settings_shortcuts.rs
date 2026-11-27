//! Shortcut revisions belong to this profile's settings transaction, not to the Loom connection.
use super::Document;
use crate::shortcut_policy::{Policy, MAX_REVISION};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ShortcutSettings {
    pub revision: u64,
    pub policy: Policy,
    pub last_request: Option<Identity>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Identity {
    pub source: String,
    pub id: String,
}

impl Document {
    pub fn shortcut_settings(&self) -> Result<ShortcutSettings, String> {
        let Some(value) = self.root.get("shortcuts") else {
            return Ok(ShortcutSettings::default());
        };
        let s: ShortcutSettings =
            serde_json::from_value(value.clone()).map_err(|e| e.to_string())?;
        s.policy.validate()?;
        if s.revision > MAX_REVISION
            || s.last_request.as_ref().is_some_and(|identity| {
                s.revision == 0
                    || !super::valid_request_field(&identity.source, 2048)
                    || !crate::loom_locale_protocol::identifier(&identity.id)
            })
        {
            return Err("shortcut_identity_invalid".into());
        }
        Ok(s)
    }

    /// An exact retry returns the original revision; no other request at that revision can mutate.
    pub fn set_shortcuts(
        &mut self,
        source: &str,
        id: &str,
        expected: u64,
        policy: Policy,
    ) -> Result<u64, String> {
        policy.validate()?;
        if !super::valid_request_field(source, 2048) || !crate::loom_locale_protocol::identifier(id)
        {
            return Err("shortcut_identity_invalid".into());
        }
        let current = self.shortcut_settings()?;
        let identity = Identity {
            source: source.to_owned(),
            id: id.to_owned(),
        };
        if current.last_request.as_ref() == Some(&identity) {
            return if current.policy == policy && expected.checked_add(1) == Some(current.revision)
            {
                Ok(current.revision)
            } else {
                Err("shortcut_request_id_conflict".into())
            };
        }
        if current.revision != expected || expected >= MAX_REVISION {
            return Err("shortcut_revision_conflict".into());
        }
        let revision = expected + 1;
        let settings = ShortcutSettings {
            revision,
            policy,
            last_request: Some(identity),
        };
        self.root.insert(
            "shortcuts".into(),
            serde_json::to_value(settings).map_err(|e| e.to_string())?,
        );
        Ok(revision)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn retry_after_reopen_never_writes_over_foreign_fields_or_accepts_changed_content() {
        let mut doc = Document::decode(br#"{"schemaVersion":2,"foreign":{"keep":true}}"#).unwrap();
        assert_eq!(
            doc.set_shortcuts("loom|device|generation", "request-1", 0, Policy::default())
                .unwrap(),
            1
        );
        let bytes = doc.encode().unwrap();
        let mut copy = Document::decode(&bytes).unwrap();
        assert_eq!(
            copy.set_shortcuts("loom|device|generation", "request-1", 0, Policy::default())
                .unwrap(),
            1
        );
        let mut p = Policy::default();
        p.fullscreen.enabled = true;
        assert!(copy
            .set_shortcuts("loom|device|generation", "request-1", 0, p.clone())
            .is_err());
        assert!(copy.set_shortcuts("other", "request-1", 0, p).is_err());
        assert_eq!(bytes, copy.encode().unwrap());
        assert!(String::from_utf8(bytes)
            .unwrap()
            .contains(r#""foreign":{"keep":true}"#));
        assert!(Document::decode(br#"{"shortcuts":null}"#).is_err());
    }
}
