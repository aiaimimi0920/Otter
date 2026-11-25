//! I/O-free application-settings editing boundary; update migrated naming/cache fields while preserving other JSON values and the old schema.
use crate::file_naming_settings::Settings;
use serde_json::{Map, Value};
use std::io::{self, Write};

pub const MAX_BYTES: usize = 1024 * 1024;

#[derive(Debug, Clone)]
pub struct Document {
    root: Map<String, Value>,
    naming: Settings,
}
impl Default for Document {
    fn default() -> Self {
        let mut root = Map::new();
        root.insert("schemaVersion".into(), Value::from(2));
        Self {
            root,
            naming: Settings::default(),
        }
    }
}

impl Document {
    /// Match mainline normalizeAppSettings; the native UI is currently Chinese, so an empty target language defaults to Chinese.
    pub fn translation_options(&self) -> (&str, &str) {
        let language = match self
            .root
            .get("translationTargetLanguage")
            .and_then(Value::as_str)
        {
            Some("en") => "en",
            _ => "zh-CN",
        };
        let provider = match self
            .root
            .get("translationProviderMode")
            .and_then(Value::as_str)
        {
            Some("local") => "local",
            Some("gateway") => "gateway",
            _ => "auto",
        };
        (language, provider)
    }
    /// Do not read default directories or back up/reset corrupt files; callers explicitly present errors.
    pub fn decode(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() > MAX_BYTES {
            return Err("App settings exceed 1 MiB".into());
        }
        let value: Value = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
        let Value::Object(root) = value else {
            return Err("App settings must be an object".into());
        };
        if let Some(version) = root.get("schemaVersion") {
            if !matches!(version.as_u64(), Some(1 | 2)) {
                return Err("Unsupported app settings schemaVersion".into());
            }
        }
        let naming = match root.get("fileNaming") {
            None => Settings::default(),
            Some(value) if value.is_object() => {
                Settings::deserialize(value).map_err(|e| e.to_string())?
            }
            Some(_) => return Err("fileNaming must be an object".into()),
        };
        naming
            .validate()
            .map_err(|e| format!("Invalid fileNaming: {e:?}"))?;
        Ok(Self { root, naming })
    }

    pub fn naming(&self) -> &Settings {
        &self.naming
    }
    /// Deny by default without explicit trust; never promote a connection address or TLS root to an Art issuer pin.
    pub fn art_issuer_trust(&self) -> Result<crate::art_issuer_trust::Document, String> {
        let document = match self.root.get("artIssuerTrust") {
            None => crate::art_issuer_trust::Document::default(),
            Some(value) => serde_json::from_value(value.clone()).map_err(|e| e.to_string())?,
        };
        document.validate()?;
        Ok(document)
    }
    /// Only the explicit settings-confirmation path may call this; a network issuer response is not confirmation.
    pub fn set_art_issuer_trust(
        &mut self,
        document: crate::art_issuer_trust::Document,
    ) -> Result<(), String> {
        document.validate()?;
        self.root.insert(
            "artIssuerTrust".into(),
            serde_json::to_value(document).map_err(|e| e.to_string())?,
        );
        Ok(())
    }
    /// Independent soft policy for close-time reclamation; do not apply temporary-file cache policy to the frozen library.
    pub fn recycle_policy(
        &self,
    ) -> Result<crate::session_library::recycle::policy::Policy, String> {
        use crate::session_library::recycle::policy::Policy;
        let policy = match self.root.get("cache") {
            None => Policy::default(),
            Some(value) if value.is_object() => {
                Policy::deserialize(value).map_err(|error| error.to_string())?
            }
            Some(_) => return Err("cache must be an object".into()),
        };
        policy.validate().map_err(str::to_owned)
    }
    /// Apply at startup; naming import/save preserves cache JSON without implicitly switching other runtime policies.
    pub fn cache_policy(&self) -> Result<crate::cache_policy::Policy, String> {
        let policy = match self.root.get("cache") {
            None => crate::cache_policy::Policy::default(),
            Some(value) if value.is_object() => crate::cache_policy::Policy::deserialize(value)
                .map_err(|error| error.to_string())?,
            Some(_) => return Err("cache must be an object".into()),
        };
        policy.validate().map_err(str::to_owned)
    }

    /// Validate before mutation; preserve unknown root/fileNaming fields and the version number.
    pub fn set_naming(&mut self, naming: Settings) -> Result<(), String> {
        naming
            .validate()
            .map_err(|e| format!("Invalid fileNaming: {e:?}"))?;
        let Value::Object(fields) = serde_json::to_value(&naming).map_err(|e| e.to_string())?
        else {
            return Err("fileNaming serialization must be an object".into());
        };
        let entry = self
            .root
            .entry("fileNaming")
            .or_insert_with(|| Value::Object(Map::new()));
        let Value::Object(target) = entry else {
            return Err("fileNaming must be an object".into());
        };
        target.extend(fields);
        self.naming = naming;
        Ok(())
    }

    pub fn set_cache_policy(&mut self, policy: crate::cache_policy::Policy) -> Result<(), String> {
        policy.validate().map_err(str::to_owned)?;
        self.extend_cache(serde_json::to_value(policy).map_err(|error| error.to_string())?)
    }
    pub fn set_recycle_policy(
        &mut self,
        policy: crate::session_library::recycle::policy::Policy,
    ) -> Result<(), String> {
        policy.validate().map_err(str::to_owned)?;
        self.extend_cache(serde_json::to_value(policy).map_err(|error| error.to_string())?)
    }
    fn extend_cache(&mut self, fields: Value) -> Result<(), String> {
        let Value::Object(fields) = fields else {
            return Err("Cache policy serialization must be an object".into());
        };
        let entry = self
            .root
            .entry("cache")
            .or_insert_with(|| Value::Object(Map::new()));
        let Value::Object(target) = entry else {
            return Err("cache must be an object".into());
        };
        target.extend(fields);
        Ok(())
    }

    /// Enforce output capacity during serialization, before generating an unbounded expanded result.
    pub fn encode(&self) -> Result<Vec<u8>, String> {
        let mut output = BoundedOutput(Vec::new());
        serde_json::to_writer(&mut output, &self.root).map_err(|e| e.to_string())?;
        Ok(output.0)
    }
}

use serde::Deserialize;

struct BoundedOutput(Vec<u8>);
impl Write for BoundedOutput {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.len() > MAX_BYTES - self.0.len() {
            return Err(io::Error::other("App settings exceed 1 MiB"));
        }
        self.0.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-save")))]
#[path = "app_settings_document_tests.rs"]
mod tests;
