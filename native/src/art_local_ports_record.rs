//! Saved port descriptions are only exact-package hints; restore does not start scripts, and execution still requires fresh online/offline signed proof.
use super::*;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Record {
    pub origin: String,
    pub art_id: String,
    pub art_version: String,
    pub package_digest: String,
    pub sdk_version: String,
    pub ports: Ports,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub visibility: Option<super::super::visibility::Visibility>,
}
impl Record {
    pub(crate) fn new(binding: &wire::Binding, ports: Ports) -> Self {
        Self {
            origin: binding.origin.clone(),
            art_id: binding.art_id.clone(),
            art_version: binding.art_version.clone(),
            package_digest: binding.package_digest.clone(),
            sdk_version: binding.sdk_version.clone(),
            ports,
            visibility: None,
        }
    }
    pub(crate) fn validate(&self, art: &str) -> Result<(), Error> {
        self.ports.validate()?;
        if let Some(visibility) = &self.visibility {
            visibility.validate(&self.ports)?;
        }
        let origin = url::Url::parse(&self.origin).map_err(|_| Error::Binding)?;
        if self.art_id != art
            || !text(&self.art_id, 256)
            || !text(&self.art_version, 128)
            || !digest(&self.package_digest)
            || self.sdk_version != "otter.art.v1"
            || self.origin.len() > 2048
            || !matches!(origin.scheme(), "http" | "https")
            || origin.origin().ascii_serialization() != self.origin
        {
            return Err(Error::Binding);
        }
        Ok(())
    }
}
