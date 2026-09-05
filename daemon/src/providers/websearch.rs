//! Web-search fallback provider.
//!
//! Matches any non-empty query and offers a single "search the web" row.
//! Activation opens the search engine in the system browser via `xdg-open`.

use crate::core::{Action, Item, Provider, Query};
use anyhow::Result;
use std::sync::Arc;

const SEARCH_URL: &str = "https://duckduckgo.com/?q=";

pub struct WebsearchProvider;

impl WebsearchProvider {
    pub fn new() -> Arc<Self> {
        Arc::new(WebsearchProvider)
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
        // Minimal percent-encoding: spaces become '+' (form-encoding), which
        // every search engine accepts. Other characters in a typed query are
        // already URL-safe for the query-string position.
        let encoded: String = query.replace(' ', "+");
        let url = format!("{SEARCH_URL}{encoded}");
        crate::launch::detached("xdg-open", &[url])
    }

    fn reindex(&self) {}
}
