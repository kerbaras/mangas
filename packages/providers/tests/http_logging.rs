//! Debug request logs name query parameters without their values (search terms).

use mangas_providers::http::redact_query;

#[test]
fn query_values_are_hidden() {
    let query = [("title", "one piece".to_string()), ("limit", "10".into())];
    let logged = redact_query(&query);
    assert_eq!(logged, "?title=…&limit=…");
    assert!(!logged.contains("piece"));
}

#[test]
fn no_query_logs_nothing() {
    assert_eq!(redact_query(&[]), "");
}
