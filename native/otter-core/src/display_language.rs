//! Supported display languages, independent of OCR translation targets.

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[repr(u8)]
pub enum DisplayLanguage {
    #[default]
    English = 0,
    SimplifiedChinese = 1,
}

impl DisplayLanguage {
    pub const ALL: [Self; 2] = [Self::English, Self::SimplifiedChinese];

    pub const fn code(self) -> &'static str {
        match self {
            Self::English => "en",
            Self::SimplifiedChinese => "zh-Hans",
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            Self::English => "English",
            Self::SimplifiedChinese => "简体中文",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "en" => Some(Self::English),
            "zh-Hans" => Some(Self::SimplifiedChinese),
            _ => None,
        }
    }

    /// Only persisted legacy settings accept the former regional identifier.
    pub fn parse_persisted(value: &str) -> Option<Self> {
        match value {
            "zh-CN" => Some(Self::SimplifiedChinese),
            _ => Self::parse(value),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::DisplayLanguage;

    #[test]
    fn canonical_codes_are_strict_and_round_trip() {
        for language in DisplayLanguage::ALL {
            assert_eq!(DisplayLanguage::parse(language.code()), Some(language));
            assert!(!language.name().is_empty());
        }
        for invalid in ["", "EN", "en-US", "zh", "zh-CN", "zh-Hant", " en"] {
            assert_eq!(DisplayLanguage::parse(invalid), None);
        }
    }

    #[test]
    fn legacy_alias_is_limited_to_persistence() {
        assert_eq!(
            DisplayLanguage::parse_persisted("zh-CN"),
            Some(DisplayLanguage::SimplifiedChinese)
        );
        assert_eq!(DisplayLanguage::parse_persisted("zh-Hant"), None);
    }
}
