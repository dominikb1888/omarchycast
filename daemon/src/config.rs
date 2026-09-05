//! User settings, stored as JSON so the QML settings panel can round-trip them
//! without a schema translation layer. Every field has a default, so a config
//! written by an older version still loads.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub const DEFAULT_HOTKEY: &str = "CTRL + SPACE";
pub const DEFAULT_SEARCH_URL: &str = "https://duckduckgo.com/?q={query}";

fn config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("omarchycast/config.json")
}

/// A single prefix-to-engine mapping. Typing `<prefix> <term>` routes the
/// search to the engine described by `url`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchPrefix {
    /// 1–3 lowercase alphanumeric characters, e.g. "x", "g".
    pub prefix: String,
    /// Human-readable engine name shown in the result row, e.g. "X", "Google".
    pub name: String,
    /// URL template; must contain the literal `{query}` placeholder.
    pub url: String,
}

impl Default for SearchPrefix {
    fn default() -> Self {
        SearchPrefix {
            prefix: String::new(),
            name: String::new(),
            url: DEFAULT_SEARCH_URL.to_string(),
        }
    }
}

fn default_prefixes() -> Vec<SearchPrefix> {
    vec![
        SearchPrefix {
            prefix: "x".into(),
            name: "X".into(),
            url: "https://x.com/search?q={query}".into(),
        },
        SearchPrefix {
            prefix: "g".into(),
            name: "Google".into(),
            url: "https://www.google.com/search?q={query}".into(),
        },
    ]
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Providers {
    pub apps: bool,
    pub calculator: bool,
    pub dates: bool,
    pub notes: bool,
    pub plugins: bool,
    pub omarchy: bool,
    pub websearch: bool,
    /// Cap per provider, applied before results are merged, so one chatty
    /// provider can't crowd out the others.
    pub apps_limit: usize,
    pub notes_limit: usize,
    pub plugins_limit: usize,
    pub omarchy_limit: usize,
    pub websearch_limit: usize,
    /// Where markdown notes live. Empty means the default, `~/Notes`.
    pub notes_directory: String,
    /// Default search engine URL template (used when no prefix matches).
    pub websearch_url: String,
    /// Prefix-to-engine mappings. Typing `<prefix> <term>` routes to that engine.
    pub websearch_prefixes: Vec<SearchPrefix>,
}

impl Default for Providers {
    fn default() -> Self {
        Providers {
            apps: true,
            calculator: true,
            dates: true,
            notes: true,
            plugins: true,
            omarchy: true,
            websearch: true,
            apps_limit: 20,
            notes_limit: 8,
            plugins_limit: 10,
            omarchy_limit: 8,
            websearch_limit: 1,
            notes_directory: String::new(),
            websearch_url: DEFAULT_SEARCH_URL.to_string(),
            websearch_prefixes: default_prefixes(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Appearance {
    pub width: u32,
    pub rows_visible: u32,
    pub corner_radius: u32,
    /// When false the overlay uses its own palette instead of the Omarchy theme.
    pub follow_theme: bool,
    /// Tighter rows and paddings; the default is the comfortable layout.
    pub compact: bool,
    /// Percentage applied to every font size the theme provides.
    pub font_scale: u32,
}

impl Default for Appearance {
    fn default() -> Self {
        Appearance {
            width: 720,
            rows_visible: 8,
            corner_radius: 16,
            follow_theme: true,
            compact: false,
            font_scale: 100,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Behaviour {
    pub hide_on_blur: bool,
    /// Escape clears a non-empty query before it dismisses the launcher.
    pub esc_clears_first: bool,
    pub show_recent_when_empty: bool,
    /// Cleared once the first-run tour has been shown, so it appears exactly once.
    pub tour_seen: bool,
}

impl Default for Behaviour {
    fn default() -> Self {
        Behaviour {
            hide_on_blur: true,
            esc_clears_first: true,
            show_recent_when_empty: true,
            tour_seen: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Config {
    pub hotkey: String,
    pub providers: Providers,
    pub appearance: Appearance,
    pub behaviour: Behaviour,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            hotkey: DEFAULT_HOTKEY.to_string(),
            providers: Providers::default(),
            appearance: Appearance::default(),
            behaviour: Behaviour::default(),
        }
    }
}

impl Config {
    /// Resolves the configured notes directory, falling back to the default when
    /// the setting is blank.
    pub fn notes_directory(&self) -> std::path::PathBuf {
        let configured = self.providers.notes_directory.trim();
        if configured.is_empty() {
            crate::providers::notes::default_directory()
        } else {
            std::path::PathBuf::from(shellexpand_home(configured))
        }
    }

    pub fn load() -> Config {
        let mut config: Config =
            crate::safeio::read_capped_optional(&config_path(), crate::limits::MAX_CONFIG_BYTES)
                .and_then(|raw| serde_json::from_str(&raw).ok())
                .unwrap_or_default();
        config.sanitise();
        config
    }

    pub fn save(&self) -> Result<()> {
        crate::safeio::write_atomic(&config_path(), &serde_json::to_string_pretty(self)?)
    }

    /// Clamps every setting into its legal range. Runs on load and on every
    /// config received over IPC, so no field is ever trusted merely because it
    /// deserialised: a hand-edited file and a socket message get the same rules.
    pub fn sanitise(&mut self) {
        use crate::limits::*;
        if self.hotkey.trim().is_empty() || self.hotkey.chars().count() > MAX_HOTKEY_CHARS {
            self.hotkey = DEFAULT_HOTKEY.to_string();
        }
        self.providers.apps_limit = self.providers.apps_limit.clamp(1, MAX_PROVIDER_RESULTS);
        self.providers.notes_limit = self.providers.notes_limit.clamp(1, MAX_PROVIDER_RESULTS);
        self.providers.plugins_limit = self.providers.plugins_limit.clamp(1, MAX_PROVIDER_RESULTS);
        self.providers.omarchy_limit = self.providers.omarchy_limit.clamp(1, MAX_PROVIDER_RESULTS);
        self.providers.websearch_limit = self.providers.websearch_limit.clamp(1, MAX_PROVIDER_RESULTS);
        if self.providers.notes_directory.chars().count() > MAX_PATH_SETTING_CHARS {
            self.providers.notes_directory = String::new();
        }
        if !valid_search_url(&self.providers.websearch_url) {
            self.providers.websearch_url = DEFAULT_SEARCH_URL.to_string();
        }
        sanitise_prefixes(&mut self.providers.websearch_prefixes);
        self.appearance.width = self.appearance.width.clamp(320, 1600);
        self.appearance.rows_visible = self.appearance.rows_visible.clamp(3, 20);
        self.appearance.corner_radius = self.appearance.corner_radius.min(48);
        self.appearance.font_scale = self.appearance.font_scale.clamp(70, 160);
    }

    pub fn provider_enabled(&self, id: &str) -> bool {
        match id {
            "apps" => self.providers.apps,
            "calc" => self.providers.calculator,
            "date" => self.providers.dates,
            "note" => self.providers.notes,
            "plug" => self.providers.plugins,
            "oma" => self.providers.omarchy,
            "web" => self.providers.websearch,
            _ => true,
        }
    }

    pub fn provider_limit(&self, id: &str) -> usize {
        match id {
            "apps" => self.providers.apps_limit.max(1),
            "note" => self.providers.notes_limit.max(1),
            "plug" => self.providers.plugins_limit.max(1),
            "oma" => self.providers.omarchy_limit.max(1),
            "web" => self.providers.websearch_limit.max(1),
            // Calculator and dates emit a single pinned row.
            _ => 4,
        }
    }
}

/// A valid search URL must be http(s), contain the `{query}` placeholder, and
/// be a reasonable length.
fn valid_search_url(url: &str) -> bool {
    let trimmed = url.trim();
    trimmed.starts_with("http://") || trimmed.starts_with("https://")
        && trimmed.contains("{query}")
        && trimmed.chars().count() <= 512
}

/// Validates and clamps the prefix list in place.
fn sanitise_prefixes(prefixes: &mut Vec<SearchPrefix>) {
    const MAX_PREFIXES: usize = 10;
    const MAX_PREFIX_LEN: usize = 3;
    const MAX_NAME_LEN: usize = 30;

    prefixes.truncate(MAX_PREFIXES);

    // Remove invalid entries and deduplicate by prefix.
    let mut seen = std::collections::HashSet::new();
    prefixes.retain(|p| {
        let prefix = p.prefix.trim().to_ascii_lowercase();
        let valid_prefix = !prefix.is_empty()
            && prefix.chars().count() <= MAX_PREFIX_LEN
            && prefix.chars().all(|c| c.is_ascii_alphanumeric());
        let valid_name = !p.name.trim().is_empty() && p.name.chars().count() <= MAX_NAME_LEN;
        let valid_url = valid_search_url(&p.url);
        if valid_prefix && valid_name && valid_url && seen.insert(prefix.clone()) {
            p.prefix = prefix;
            p.name = p.name.trim().to_string();
            p.url = p.url.trim().to_string();
            true
        } else {
            false
        }
    });

    if prefixes.is_empty() {
        *prefixes = default_prefixes();
    }
}

/// Expands a leading `~` so the setting can be typed the way people write paths.
fn shellexpand_home(path: &str) -> String {
    match path.strip_prefix("~/") {
        Some(rest) => dirs::home_dir()
            .map(|h| h.join(rest).to_string_lossy().into_owned())
            .unwrap_or_else(|| path.to_string()),
        None => path.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitise_clamps_every_field_into_range() {
        let mut c = Config {
            hotkey: "K".repeat(500),
            ..Config::default()
        };
        c.providers.apps_limit = 9999;
        c.providers.notes_limit = 0;
        c.appearance.width = 1;
        c.appearance.rows_visible = 999;
        c.appearance.corner_radius = 10_000;
        c.sanitise();
        assert_eq!(c.hotkey, DEFAULT_HOTKEY);
        assert_eq!(c.providers.apps_limit, crate::limits::MAX_PROVIDER_RESULTS);
        assert_eq!(c.providers.notes_limit, 1);
        assert_eq!(c.appearance.width, 320);
        assert_eq!(c.appearance.rows_visible, 20);
        assert_eq!(c.appearance.corner_radius, 48);
    }

    #[test]
    fn sanitise_resets_invalid_search_url() {
        let mut c = Config::default();
        c.providers.websearch_url = "not a url".into();
        c.sanitise();
        assert_eq!(c.providers.websearch_url, DEFAULT_SEARCH_URL);

        c.providers.websearch_url = "https://example.com/search".into();
        c.sanitise();
        assert_eq!(c.providers.websearch_url, DEFAULT_SEARCH_URL);

        c.providers.websearch_url = "https://example.com/search?q={query}".into();
        c.sanitise();
        assert_eq!(c.providers.websearch_url, "https://example.com/search?q={query}");
    }

    #[test]
    fn sanitise_removes_invalid_prefixes() {
        let mut c = Config::default();
        c.providers.websearch_prefixes.push(SearchPrefix {
            prefix: "waytoolong".into(),
            name: "Bad".into(),
            url: "https://x.com/?q={query}".into(),
        });
        c.providers.websearch_prefixes.push(SearchPrefix {
            prefix: "ok".into(),
            name: "Good".into(),
            url: "https://good.com/?q={query}".into(),
        });
        c.sanitise();
        // "waytoolong" is > 3 chars, should be removed. "ok" is valid.
        let prefixes: Vec<&str> = c.providers.websearch_prefixes.iter().map(|p| p.prefix.as_str()).collect();
        assert!(prefixes.contains(&"ok"));
        assert!(!prefixes.contains(&"waytoolong"));
    }

    #[test]
    fn sanitise_deduplicates_prefixes() {
        let mut c = Config::default();
        c.providers.websearch_prefixes.push(SearchPrefix {
            prefix: "x".into(),
            name: "Duplicate".into(),
            url: "https://dup.com/?q={query}".into(),
        });
        c.sanitise();
        let x_count = c.providers.websearch_prefixes.iter().filter(|p| p.prefix == "x").count();
        assert_eq!(x_count, 1);
    }

    #[test]
    fn sanitise_resets_to_defaults_when_all_invalid() {
        let mut c = Config::default();
        c.providers.websearch_prefixes = vec![SearchPrefix {
            prefix: "!!!".into(),
            name: "Bad".into(),
            url: "not-a-url".into(),
        }];
        c.sanitise();
        assert!(!c.providers.websearch_prefixes.is_empty());
        // Should have fallen back to defaults
        assert!(c.providers.websearch_prefixes.iter().any(|p| p.prefix == "x"));
    }

    #[test]
    fn default_prefixes_contain_x_and_g() {
        let p = Providers::default();
        assert!(p.websearch_prefixes.iter().any(|e| e.prefix == "x" && e.name == "X"));
        assert!(p.websearch_prefixes.iter().any(|e| e.prefix == "g" && e.name == "Google"));
    }
}
