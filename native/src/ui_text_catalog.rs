//! Parse the bundled catalogs once and retain formatted static labels for window lifetimes.

use super::contract::{self, Contracts};
use super::{DisplayLanguage, FluentValue};
use fluent_bundle::{concurrent::FluentBundle, FluentArgs, FluentResource};
use std::collections::{BTreeMap, BTreeSet};

macro_rules! resources {
    ($language:literal) => {
        &[
            include_str!(concat!("../assets/locales/", $language, "/common.ftl")),
            include_str!(concat!("../assets/locales/", $language, "/actions.ftl")),
            include_str!(concat!("../assets/locales/", $language, "/menus.ftl")),
            include_str!(concat!("../assets/locales/", $language, "/dialogs.ftl")),
            include_str!(concat!("../assets/locales/", $language, "/application.ftl")),
            include_str!(concat!("../assets/locales/", $language, "/integration.ftl")),
            include_str!(concat!("../assets/locales/", $language, "/settings.ftl")),
            include_str!(concat!("../assets/locales/", $language, "/workspace.ftl")),
            include_str!(concat!(
                "../assets/locales/",
                $language,
                "/workspace-menus.ftl"
            )),
            include_str!(concat!("../assets/locales/", $language, "/native.ftl")),
        ]
    };
}

struct Catalog {
    bundle: FluentBundle<FluentResource>,
    contracts: Contracts,
    labels: BTreeMap<String, String>,
}

impl Catalog {
    fn new(language: DisplayLanguage, sources: &[&str]) -> Result<Self, String> {
        let resources = sources
            .iter()
            .map(|source| {
                FluentResource::try_new((*source).to_owned())
                    .map_err(|(_, errors)| format!("{}: {errors:?}", language.code()))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let contracts = contract::messages(&resources)?;
        let mut bundle = FluentBundle::new_concurrent(vec![language
            .code()
            .parse()
            .expect("supported display languages have valid identifiers")]);
        // Win32 GDI DrawTextW renders U+2068/U+2069 bidi isolate marks as missing-glyph tofu boxes.
        bundle.set_use_isolating(false);
        for resource in resources {
            bundle
                .add_resource(resource)
                .map_err(|errors| format!("{}: {errors:?}", language.code()))?;
        }
        let mut catalog = Self {
            bundle,
            contracts,
            labels: BTreeMap::new(),
        };
        for (id, parameters) in &catalog.contracts {
            if parameters.is_empty() {
                catalog
                    .labels
                    .insert(id.clone(), catalog.resolve(id, None)?);
            }
        }
        Ok(catalog)
    }

    fn resolve(&self, id: &str, arguments: Option<&FluentArgs<'_>>) -> Result<String, String> {
        let value = self
            .bundle
            .get_message(id)
            .and_then(|message| message.value())
            .ok_or_else(|| format!("interface message has no value: {id}"))?;
        let mut errors = Vec::new();
        let result = self
            .bundle
            .format_pattern(value, arguments, &mut errors)
            .into_owned();
        if errors.is_empty() {
            Ok(result)
        } else {
            Err(format!("interface message {id}: {errors:?}"))
        }
    }
}

pub(super) struct Catalogs {
    english: Catalog,
    chinese: Catalog,
}

impl Catalogs {
    pub(super) fn embedded() -> Result<Self, String> {
        Self::new(resources!("en"), resources!("zh-Hans"))
    }

    pub(super) fn new(english: &[&str], chinese: &[&str]) -> Result<Self, String> {
        let english = Catalog::new(DisplayLanguage::English, english)?;
        let chinese = Catalog::new(DisplayLanguage::SimplifiedChinese, chinese)?;
        for (id, parameters) in &chinese.contracts {
            let source_parameters = english.contracts.get(id).ok_or_else(|| {
                format!("translated interface message has no English source: {id}")
            })?;
            if !parameters.is_subset(source_parameters) {
                return Err(format!(
                    "translated interface message requires extra arguments: {id}"
                ));
            }
        }
        Ok(Self { english, chinese })
    }

    fn localized(&self, language: DisplayLanguage) -> &Catalog {
        match language {
            DisplayLanguage::English => &self.english,
            DisplayLanguage::SimplifiedChinese => &self.chinese,
        }
    }

    #[track_caller]
    pub(super) fn text(&self, language: DisplayLanguage, id: &str) -> &str {
        let source = self
            .english
            .labels
            .get(id)
            .unwrap_or_else(|| panic!("unknown or parameterized interface message: {id}"));
        self.localized(language)
            .labels
            .get(id)
            .unwrap_or(source)
            .as_str()
    }

    #[track_caller]
    pub(super) fn format(
        &self,
        language: DisplayLanguage,
        id: &str,
        values: &[(&str, FluentValue<'_>)],
    ) -> String {
        let required = self
            .english
            .contracts
            .get(id)
            .unwrap_or_else(|| panic!("unknown interface message: {id}"));
        let supplied: BTreeSet<String> = values.iter().map(|(key, _)| (*key).to_owned()).collect();
        assert_eq!(values.len(), supplied.len(), "duplicate arguments for {id}");
        assert_eq!(&supplied, required, "incorrect arguments for {id}");
        let mut arguments = FluentArgs::new();
        for (key, value) in values {
            arguments.set(*key, value.clone());
        }
        let localized = self.localized(language);
        if localized.contracts.contains_key(id) {
            if let Ok(result) = localized.resolve(id, Some(&arguments)) {
                return result;
            }
        }
        self.english
            .resolve(id, Some(&arguments))
            .unwrap_or_else(|error| panic!("{error}"))
    }

    #[cfg(test)]
    pub(super) fn source_parameters(&self, id: &str) -> Option<&BTreeSet<String>> {
        self.english.contracts.get(id)
    }

    #[cfg(test)]
    pub(super) fn validate_embedded_completeness(&self) {
        assert_eq!(
            self.english.contracts.keys().collect::<Vec<_>>(),
            self.chinese.contracts.keys().collect::<Vec<_>>(),
            "the bundled Simplified Chinese translation must be complete"
        );
        for (id, parameters) in &self.english.contracts {
            let mut arguments = FluentArgs::new();
            for parameter in parameters {
                arguments.set(parameter.as_str(), 2);
            }
            for catalog in [&self.english, &self.chinese] {
                catalog.resolve(id, Some(&arguments)).unwrap();
            }
        }
    }
}
