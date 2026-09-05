//! Web-search fallback provider.
//!
//! Matches any non-empty query and offers a single "search the web" row.
//! Activation opens the configured search engine in the system browser via
//! `xdg-open`. The URL template is user-configurable and must contain the
//! literal `{query}` placeholder.

use crate::core::{Action, Item, Provider, Query};
use anyhow::Result;
use std::sync::{Arc, RwLock};

const DEFAULT_URL: &str = "https://duckduckgo.com/?q={query}";

pub struct WebsearchProvider {
    url: RwLock<String>,
}

impl WebsearchProvider {
    pub fn new() -> Arc<Self> {
        Arc::new(WebsearchProvider {
            url: RwLock::new(DEFAULT_URL.to_string()),
        })
    }

    /// Updates the search URL template. Called on config load and on every
    /// `SetConfig` so the user can switch engines without restarting.
    pub fn set_url(&self, url: String) {
        if let Ok(mut current) = self.url.write() {
            *current = url;
        }
    }

    /// Returns the currently configured URL template (useful for tests and
    /// diagnostics).
    pub fn current_url(&self) -> String {
        self.url
            .read()
            .map(|u| u.clone())
            .unwrap_or_else(|_| DEFAULT_URL.to_string())
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
        vec![Item {
            id: format!("web:{}", q.trimmed),
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
        let query = id.strip_prefix("web:").unwrap_or(id);
        let template = self
            .url
            .read()
            .map(|u| u.clone())
            .unwrap_or_else(|_| DEFAULT_URL.to_string());
        let url = build_url(&template, query);
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

// ------------------------------------------------------------------ tests

#[cfg(test)]
mod tests {
    use super::*;

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
    fn query_item_has_correct_fields() {
        let p = WebsearchProvider::new();
        let q = Query::new("hello world");
        let item = &p.query(&q)[0];

        assert_eq!(item.id, "web:hello world");
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
        assert_eq!(item.id, "web:padded");
        assert_eq!(item.title, "Search web for: padded");
    }

    #[test]
    fn set_url_changes_template() {
        let p = WebsearchProvider::new();
        assert_eq!(p.current_url(), DEFAULT_URL);

        p.set_url("https://www.google.com/search?q={query}".into());
        assert_eq!(
            p.current_url(),
            "https://www.google.com/search?q={query}"
        );
    }

    #[test]
    fn set_url_overwrites_previous() {
        let p = WebsearchProvider::new();
        p.set_url("https://a.com/?q={query}".into());
        p.set_url("https://b.com/?q={query}".into());
        assert_eq!(p.current_url(), "https://b.com/?q={query}");
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
    fn reindex_does_not_panic() {
        let p = WebsearchProvider::new();
        p.reindex();
        // Still functional after reindex.
        let q = Query::new("test");
        assert_eq!(p.query(&q).len(), 1);
    }

    #[test]
    fn provider_trait_object_query() {
        // Verify the provider works through the trait object (as the Registry
        // would use it).
        let p: Arc<dyn Provider> = WebsearchProvider::new();
        assert_eq!(p.id(), "web");
        let q = Query::new("trait test");
        let items = p.query(&q);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].provider, "web");
    }
}
