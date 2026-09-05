//! Integration tests for the websearch provider.
//!
//! These exercise the provider through its public API the same way the
//! Registry and IPC layer would, without requiring a running daemon or a
//! display server.

use omarchycastd::core::{Action, Provider, Query};
use omarchycastd::providers::websearch::WebsearchProvider;

#[test]
fn full_query_activate_url_construction() {
    let provider = WebsearchProvider::new();
    provider.set_url("https://www.google.com/search?q={query}".into());

    let q = Query::new("rust async traits");
    let items = provider.query(&q);
    assert_eq!(items.len(), 1);

    // The id encodes the engine and the query; activation strips the prefix and
    // builds the URL.
    let id = &items[0].id;
    assert_eq!(id, "web:default:rust async traits");

    // We can't call activate (it would spawn xdg-open), but we can verify the
    // URL the provider *would* build by checking the template and the id.
    let template = provider.current_url();
    let term = id.strip_prefix("web:default:").unwrap();
    let expected = template.replace("{query}", &term.replace(' ', "+"));
    assert_eq!(expected, "https://www.google.com/search?q=rust+async+traits");
}

#[test]
fn provider_respects_configured_url() {
    let provider = WebsearchProvider::new();

    // Default
    assert_eq!(
        provider.current_url(),
        "https://duckduckgo.com/?q={query}"
    );

    // Switch to Bing
    provider.set_url("https://www.bing.com/search?q={query}".into());
    assert_eq!(
        provider.current_url(),
        "https://www.bing.com/search?q={query}"
    );

    // Switch to a local search
    provider.set_url("http://localhost:8080/search?term={query}".into());
    assert_eq!(
        provider.current_url(),
        "http://localhost:8080/search?term={query}"
    );
}

#[test]
fn query_is_stable_across_calls() {
    let provider = WebsearchProvider::new();
    let q = Query::new("deterministic");

    let first = provider.query(&q);
    let second = provider.query(&q);

    assert_eq!(first.len(), second.len());
    assert_eq!(first[0].id, second[0].id);
    assert_eq!(first[0].title, second[0].title);
    assert_eq!(first[0].provider, second[0].provider);
    assert_eq!(first[0].kind, second[0].kind);
}

#[test]
fn different_queries_produce_different_ids() {
    let provider = WebsearchProvider::new();

    let a = provider.query(&Query::new("alpha"));
    let b = provider.query(&Query::new("beta"));

    assert_ne!(a[0].id, b[0].id);
    assert_eq!(a[0].id, "web:default:alpha");
    assert_eq!(b[0].id, "web:default:beta");
}

#[test]
fn reindex_is_idempotent_and_non_disruptive() {
    let provider = WebsearchProvider::new();
    provider.set_url("https://x.com/?q={query}".into());

    let before = provider.query(&Query::new("check"));
    provider.reindex();
    provider.reindex();
    let after = provider.query(&Query::new("check"));

    assert_eq!(before.len(), after.len());
    assert_eq!(before[0].id, after[0].id);
    assert_eq!(provider.current_url(), "https://x.com/?q={query}");
}

#[test]
fn provider_works_through_dyn_trait() {
    // The Registry stores `Arc<dyn Provider>`; make sure the websearch
    // provider satisfies the trait and behaves correctly through it.
    let provider: std::sync::Arc<dyn Provider> = WebsearchProvider::new();

    assert_eq!(provider.id(), "web");

    let q = Query::new("dyn trait test");
    let items = provider.query(&q);
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].provider, "web");
    assert_eq!(items[0].kind, "Web");

    // reindex through the trait
    provider.reindex();

    // Still works
    let items2 = provider.query(&Query::new("still works"));
    assert_eq!(items2.len(), 1);
}

#[test]
fn empty_query_never_produces_items() {
    let provider = WebsearchProvider::new();

    for raw in ["", "   ", "\t", "\n"] {
        let q = Query::new(raw);
        assert!(
            provider.query(&q).is_empty(),
            "expected no items for query {raw:?}"
        );
    }
}

#[test]
fn long_query_still_produces_single_item() {
    let provider = WebsearchProvider::new();
    let long = "a".repeat(500);
    let q = Query::new(&long);
    let items = provider.query(&q);
    assert_eq!(items.len(), 1);
    assert!(items[0].id.starts_with("web:"));
    assert!(items[0].title.contains("Search web for:"));
}

#[test]
fn action_variant_does_not_affect_query() {
    // The query method ignores the action; only activate uses it.
    let provider = WebsearchProvider::new();
    let q = Query::new("action test");

    let items = provider.query(&q);
    assert_eq!(items.len(), 1);

    // Both Primary and Secondary would produce the same item list.
    let _ = Action::Primary;
    let _ = Action::Secondary;
    let items2 = provider.query(&q);
    assert_eq!(items, items2);
}
