//! 运行时命名设置模型；字段与主干一致，不自动读写主干配置目录。
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CollisionPolicy {
    Increment,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub sticker_save_pattern: String,
    pub drag_export_pattern: String,
    pub clipboard_file_pattern: String,
    pub title_max_length: usize,
    pub collision_policy: CollisionPolicy,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            sticker_save_pattern: crate::file_naming::DEFAULT_SAVE_PATTERN.into(),
            drag_export_pattern: "{label}_{shortId}_{date}_{time}".into(),
            clipboard_file_pattern: "Otter_{kind}_{date}_{time}".into(),
            title_max_length: 80,
            collision_policy: CollisionPolicy::Increment,
        }
    }
}
impl Settings {
    pub fn validate(&self) -> Result<(), crate::file_naming::Error> {
        if !(1..=240).contains(&self.title_max_length) {
            return Err(crate::file_naming::Error::TitleLimit);
        }
        for pattern in [
            &self.sticker_save_pattern,
            &self.drag_export_pattern,
            &self.clipboard_file_pattern,
        ] {
            crate::file_naming::validate(pattern)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn partial_main_fields_keep_defaults_and_validate() {
        let settings: Settings =
            serde_json::from_str(r#"{"stickerSavePattern":"图_{shortId}"}"#).unwrap();
        settings.validate().unwrap();
        assert_eq!(settings.title_max_length, 80);
        assert_eq!(
            settings.clipboard_file_pattern,
            "Otter_{kind}_{date}_{time}"
        );
        assert_eq!(
            serde_json::to_value(&settings).unwrap()["collisionPolicy"],
            "increment"
        );
    }
    #[test]
    fn invalid_templates_limits_and_future_collision_modes_are_rejected() {
        let settings = Settings {
            sticker_save_pattern: "{unknown}".into(),
            ..Default::default()
        };
        assert!(settings.validate().is_err());
        let settings = Settings {
            title_max_length: 241,
            ..Default::default()
        };
        assert!(settings.validate().is_err());
        assert!(serde_json::from_str::<Settings>(r#"{"collisionPolicy":"overwrite"}"#).is_err());
    }
}
