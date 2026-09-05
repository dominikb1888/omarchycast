//! Web-search fallback provider.
//!
//! Matches any non-empty query and offers a single "search the web" row.
//! Activation opens the configured search engine in the system browser via
//! `xdg-open`. The URL template is user-configurable and must contain the
//! literal `{query}` placeholder.
//!
//! Prefix routing: if the query starts with a configured prefix followed by a
//! space (e.g. "x rust async"), the search is routed to that engine instead of
//! the default.

use crate::config::SearchPrefix;
use crate::core::{Action, Item, Provider, Query};
use anyhow::Result;
use std::sync::{Arc, RwLock};

const DEFAULT_URL: &str = "https://duckduckgo.com/?q={query}";
const DEFAULT_NAME: &str = "Web";

pub struct WebsearchProvider {
    url: RwLock<String>,
    prefixes: RwLock<Vec<SearchPrefix>>,
}

impl WebsearchProvider {
    pub fn new() -> Arc<Self> {
        Arc::new(WebsearchProvider {
            url: RwLock::new(DEFAULT_URL.to_string()),
            prefixes: RwLock::new(Vec::new()),
        })
    }

    /// Updates the default search URL template.
    pub fn set_url(&self, url: String) {
        if let Ok(mut current) = self.url.write() {
            *current = url;
        }
    }

    /// Updates the prefix-to-engine mapping list.
    pub fn set_prefixes(&self, prefixes: Vec<SearchPrefix>) {
        if let Ok(mut current) = self.prefixes.write() {
            *current = prefixes;
        }
    }

    /// Returns the currently configured default URL template.
    pub fn current_url(&self) -> String {
        self.url
            .read()
            .map(|u| u.clone())
            .unwrap_or_else(|_| DEFAULT_URL.to_string())
    }

    /// Returns the currently configured prefix list.
    pub fn current_prefixes(&self) -> Vec<SearchPrefix> {
        self.prefixes
            .read()
            .map(|p| p.clone())
            .unwrap_or_default()
    }
}

impl Provider for WebsearchProvider {
    fn id(&self) -> &'static str {
        "web"
    }

    fn query(&self, q: &Query) -> Vec<Item> {
        if q.is_empty() {
            return Vec::new();
        }

        let prefixes = self
            .prefixes
            .read()
            .map(|p| p.clone())
            .unwrap_or_default();

        // Check for a prefix match: the query must start with `<prefix> ` (prefix + space).
        for entry in &prefixes {
            let marker = format!("{} ", entry.prefix);
            if let Some(term) = q.trimmed.strip_prefix(&marker) {
                if term.is_empty() {
                    continue;
                }
                return vec![Item {
                    id: format!("web:{}:{}", entry.prefix, term),
                    provider: "web",
                    kind: "Web",
                    title: format!("Search {} for: {}", entry.name, term),
                    subtitle: Some(engine_display_url(&entry.url)),
                    glyph: Some("\u{1F310}".into()),
                    accessory: None,
                    icon: None,
                }];
            }
        }

        // No prefix matched — use the default engine.
        vec![Item {
            id: format!("web:default:{}", q.trimmed),
            provider: "web",
            kind: "Web",
            title: format!("Search web for: {}", q.trimmed),
            subtitle: Some("Open in browser".into()),
            glyph: Some("\u{1F310}".into()),
            accessory: None,
            icon: None,
        }]
    }

    fn activate(&self, id: &str, _action: Action) -> Result<()> {
        // id format: "web:<engine>:<term>"
        let rest = id.strip_prefix("web:").unwrap_or(id);
        let (engine, term) = rest.split_once(':').unwrap_or(("default", rest));

        let url = if engine == "default" {
            let template = self
                .url
                .read()
                .map(|u| u.clone())
                .unwrap_or_else(|_| DEFAULT_URL.to_string());
            build_url(&template, term)
        } else {
            let prefixes = self
                .prefixes
                .read()
                .map(|p| p.clone())
                .unwrap_or_default();
            match prefixes.iter().find(|p| p.prefix == engine) {
                Some(entry) => build_url(&entry.url, term),
                None => {
                    // Fallback to default if the prefix was removed after the item was created.
                    let template = self
                        .url
                        .read()
                        .map(|u| u.clone())
                        .unwrap_or_else(|_| DEFAULT_URL.to_string());
                    build_url(&template, term)
                }
            }
        };

        crate::launch::detached("xdg-open", &[url])
    }

    fn reindex(&self) {}
}

/// Replaces the `{query}` placeholder in `template` with the form-encoded
/// search term. Spaces become `+` (form-encoding), which every search engine
/// accepts in the query-string position.
fn build_url(template: &str, query: &str) -> String {
    let encoded: String = query.replace(' ', "+");
    template.replace("{query}", &encoded)
}

/// Extracts a short display form from a URL for the subtitle, e.g.
/// "https://x.com/search?q={query}" → "x.com".
fn engine_display_url(url: &str) -> String {
    url.trim()
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .split('/')
        .next()
        .unwrap_or(url)
        .to_string()
}

// ------------------------------------------------------------------ tests

#[cfg(test)]
mod tests {
    use super::*;

    fn provider_with_prefixes() -> Arc<WebsearchProvider> {
        let p = WebsearchProvider::new();
        p.set_prefixes(vec![
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
        ]);
        p
    }

    #[test]
    fn id_returns_web() {
        let p = WebsearchProvider::new();
        assert_eq!(p.id(), "web");
    }

    #[test]
    fn query_empty_returns_no_items() {
        let p = WebsearchProvider::new();
        let q = Query::new("");
        assert!(p.query(&q).is_empty());
    }

    #[test]
    fn query_whitespace_only_returns_no_items() {
        let p = WebsearchProvider::new();
        let q = Query::new("   ");
        assert!(p.query(&q).is_empty());
    }

    #[test]
    fn query_nonempty_returns_single_item() {
        let p = WebsearchProvider::new();
        let q = Query::new("rust async");
        let items = p.query(&q);
        assert_eq!(items.len(), 1);
    }

    #[test]
    fn query_default_item_has_correct_fields() {
        let p = WebsearchProvider::new();
        let q = Query::new("hello world");
        let item = &p.query(&q)[0];

        assert_eq!(item.id, "web:default:hello world");
        assert_eq!(item.provider, "web");
        assert_eq!(item.kind, "Web");
        assert_eq!(item.title, "Search web for: hello world");
        assert_eq!(item.subtitle.as_deref(), Some("Open in browser"));
        assert_eq!(item.glyph.as_deref(), Some("\u{1F310}"));
        assert!(item.accessory.is_none());
        assert!(item.icon.is_none());
    }

    #[test]
    fn query_trims_input() {
        let p = WebsearchProvider::new();
        let q = Query::new("  padded  ");
        let item = &p.query(&q)[0];
        assert_eq!(item.id, "web:default:padded");
        assert_eq!(item.title, "Search web for: padded");
    }

    #[test]
    fn prefix_x_routes_to_x_com() {
        let p = provider_with_prefixes();
        let q = Query::new("x rust async");
        let item = &p.query(&q)[0];

        assert_eq!(item.id, "web:x:rust async");
        assert_eq!(item.title, "Search X for: rust async");
        assert_eq!(item.subtitle.as_deref(), Some("x.com"));
    }

    #[test]
    fn prefix_g_routes_to_google() {
        let p = provider_with_prefixes();
        let q = Query::new("g hello world");
        let item = &p.query(&q)[0];

        assert_eq!(item.id, "web:g:hello world");
        assert_eq!(item.title, "Search Google for: hello world");
        assert_eq!(item.subtitle.as_deref(), Some("www.google.com"));
    }

    #[test]
    fn prefix_without_space_does_not_match() {
        let p = provider_with_prefixes();
        // "xyz" should NOT match prefix "x" because there's no space after it.
        let q = Query::new("xyz");
        let item = &p.query(&q)[0];
        assert_eq!(item.id, "web:default:xyz");
        assert_eq!(item.title, "Search web for: xyz");
    }

    #[test]
    fn prefix_alone_is_empty_search_and_falls_through() {
        let p = provider_with_prefixes();
        // "x " trims to "x" which is non-empty but the term after "x " is empty.
        // Actually Query::new("x ") trims to "x", so strip_prefix("x ") won't match.
        let q = Query::new("x");
        let item = &p.query(&q)[0];
        // "x" alone doesn't have a space after it, so it's a default search.
        assert_eq!(item.id, "web:default:x");
    }

    #[test]
    fn set_url_changes_default_template() {
        let p = WebsearchProvider::new();
        assert_eq!(p.current_url(), DEFAULT_URL);

        p.set_url("https://www.google.com/search?q={query}".into());
        assert_eq!(
            p.current_url(),
            "https://www.google.com/search?q={query}"
        );
    }

    #[test]
    fn set_prefixes_updates_list() {
        let p = WebsearchProvider::new();
        assert!(p.current_prefixes().is_empty());

        p.set_prefixes(vec![SearchPrefix {
            prefix: "b".into(),
            name: "Bing".into(),
            url: "https://www.bing.com/search?q={query}".into(),
        }]);
        assert_eq!(p.current_prefixes().len(), 1);
        assert_eq!(p.current_prefixes()[0].prefix, "b");
    }

    #[test]
    fn build_url_basic_substitution() {
        let url = build_url("https://duckduckgo.com/?q={query}", "rust");
        assert_eq!(url, "https://duckduckgo.com/?q=rust");
    }

    #[test]
    fn build_url_spaces_become_plus() {
        let url = build_url("https://duckduckgo.com/?q={query}", "hello world");
        assert_eq!(url, "https://duckduckgo.com/?q=hello+world");
    }

    #[test]
    fn build_url_multiple_spaces() {
        let url = build_url("https://x.com/s?q={query}", "a  b   c");
        assert_eq!(url, "https://x.com/s?q=a++b+++c");
    }

    #[test]
    fn build_url_no_placeholder_returns_template_unchanged() {
        let url = build_url("https://x.com/static", "anything");
        assert_eq!(url, "https://x.com/static");
    }

    #[test]
    fn build_url_multiple_placeholders_all_replaced() {
        let url = build_url("https://x.com/{query}/{query}", "hi");
        assert_eq!(url, "https://x.com/hi/hi");
    }

    #[test]
    fn build_url_unicode_query() {
        let url = build_url("https://x.com/?q={query}", "日本語 テスト");
        assert_eq!(url, "https://x.com/?q=日本語+テスト");
    }

    #[test]
    fn engine_display_url_extracts_host() {
        assert_eq!(engine_display_url("https://x.com/search?q={query}"), "x.com");
        assert_eq!(engine_display_url("https://www.google.com/search?q={query}"), "www.google.com");
        assert_eq!(engine_display_url("http://localhost:8080/s?q={query}"), "localhost:8080");
    }

    #[test]
    fn reindex_does_not_panic() {
        let p = WebsearchProvider::new();
        p.reindex();
        let q = Query::new("test");
        assert_eq!(p.query(&q).len(), 1);
    }

    #[test]
    fn provider_trait_object_query() {
        let p: Arc<dyn Provider> = WebsearchProvider::new();
        assert_eq!(p.id(), "web");
        let q = Query::new("trait test");
        let items = p.query(&q);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].provider, "web");
    }

    #[test]
    fn prefix_query_still_single_item() {
        let p = provider_with_prefixes();
        let q = Query::new("x only one");
        let items = p.query(&q);
        assert_eq!(items.len(), 1);
    }

    #[test]
    fn multiple_prefixes_first_match_wins() {
        // If we had both "x" and "xy" as prefixes, "xy term" should match "xy"
        // (longer prefix) only if it's listed first. In our design, we iterate
        // in order, so the first match wins.
        let p = WebsearchProvider::new();
        p.set_prefixes(vec![
            SearchPrefix {
                prefix: "x".into(),
                name: "X".into(),
                url: "https://x.com/?q={query}".into(),
            },
            SearchPrefix {
                prefix: "xy".into(),
                name: "XY".into(),
                url: "https://xy.com/?q={query}".into(),
            },
        ]);
        // "xy term" starts with "x " ? No — "xy term" does not start with "x ".
        // It starts with "xy ", so it matches the second prefix.
        let q = Query::new("xy term");
        let item = &p.query(&q)[0];
        assert_eq!(item.id, "web:xy:term");
        assert_eq!(item.title, "Search XY for: term");
    }
}
