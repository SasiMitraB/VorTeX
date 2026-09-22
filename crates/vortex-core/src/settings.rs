//! User settings, stored in `~/.vortex-editor/config.json`.
//!
//! Keys this struct doesn't know about are kept when saving, so older and
//! newer versions of the app can share the file.

use crate::fs_utils::{read_config, write_config};
use crate::grammar_checker::GrammarDialect;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "lowercase")]
pub enum ThemePreference {
    /// Follow the OS appearance.
    #[default]
    Auto,
    Light,
    Dark,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    /// Folder whose subfolders are listed as projects.
    pub projects_folder: Option<String>,
    /// The project that was open last.
    pub current_project: Option<String>,
    pub theme_preference: ThemePreference,
    pub grammar_dialect: GrammarDialect,
}

impl Settings {
    /// The saved settings; anything missing or unreadable falls back to its default.
    pub fn load() -> Self {
        Self::from_json(read_config())
    }

    pub fn from_json(value: serde_json::Value) -> Self {
        let mut settings = Settings::default();
        let serde_json::Value::Object(map) = value else {
            return settings;
        };
        // Field by field, so one bad value doesn't reset the rest.
        let get = |k: &str| map.get(k).cloned().unwrap_or(serde_json::Value::Null);
        settings.projects_folder = serde_json::from_value(get("projectsFolder")).unwrap_or_default();
        settings.current_project = serde_json::from_value(get("currentProject")).unwrap_or_default();
        settings.theme_preference = serde_json::from_value(get("themePreference")).unwrap_or_default();
        settings.grammar_dialect = serde_json::from_value(get("grammarDialect")).unwrap_or_default();
        settings
    }

    pub fn save(&self) -> std::io::Result<()> {
        let mut config = read_config();
        if !config.is_object() {
            config = serde_json::json!({});
        }
        let obj = config.as_object_mut().expect("config is an object");
        if let serde_json::Value::Object(ours) = serde_json::to_value(self)? {
            obj.extend(ours);
        }
        write_config(&config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn reads_the_gpui_app_config() {
        let s = Settings::from_json(json!({
            "projectsFolder": "/Users/me/Papers",
            "themePreference": "dark",
            "currentProject": "/Users/me/Papers/thesis",
            "somethingElse": 3
        }));
        assert_eq!(s.projects_folder.as_deref(), Some("/Users/me/Papers"));
        assert_eq!(s.theme_preference, ThemePreference::Dark);
        assert_eq!(s.grammar_dialect, GrammarDialect::British);
    }

    #[test]
    fn a_bad_value_only_resets_itself() {
        let s = Settings::from_json(json!({ "projectsFolder": "/p", "themePreference": 42 }));
        assert_eq!(s.projects_folder.as_deref(), Some("/p"));
        assert_eq!(s.theme_preference, ThemePreference::Auto);
    }
}
