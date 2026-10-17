use std::time::Duration;

use chrono::{TimeZone, Utc};
use mangas_providers::http::{parse_retry_after, retryable_status, RetryPolicy};

const POLICY: RetryPolicy = RetryPolicy {
    max_retries: 3,
    base_delay: Duration::from_millis(100),
    max_delay: Duration::from_secs(1),
};

#[test]
fn backoff_doubles_and_is_capped() {
    assert_eq!(POLICY.backoff(0), Duration::from_millis(100));
    assert_eq!(POLICY.backoff(1), Duration::from_millis(200));
    assert_eq!(POLICY.backoff(2), Duration::from_millis(400));
    assert_eq!(POLICY.backoff(5), Duration::from_secs(1));
    assert_eq!(POLICY.backoff(40), Duration::from_secs(1));
}

#[test]
fn delay_gives_up_after_max_retries() {
    assert_eq!(POLICY.delay(2, None), Some(Duration::from_millis(400)));
    assert_eq!(POLICY.delay(3, None), None);
    assert_eq!(RetryPolicy::NONE.delay(0, None), None);
}

#[test]
fn delay_honours_retry_after_up_to_the_cap() {
    let ms = Duration::from_millis;
    assert_eq!(POLICY.delay(0, Some(ms(700))), Some(ms(700)));
    assert_eq!(POLICY.delay(0, Some(Duration::ZERO)), Some(Duration::ZERO));
    assert_eq!(POLICY.delay(0, Some(Duration::from_secs(120))), None);
}

#[test]
fn retryable_statuses() {
    for status in [429, 500, 502, 503, 504] {
        assert!(retryable_status(status, true), "{status}");
    }
    for status in [200, 301, 400, 401, 403, 404, 501] {
        assert!(!retryable_status(status, true), "{status}");
    }
    assert!(retryable_status(429, false));
    assert!(retryable_status(503, false));
    assert!(!retryable_status(500, false));
    assert!(!retryable_status(502, false));
}

#[test]
fn retry_after_seconds_and_http_dates() {
    let now = Utc.with_ymd_and_hms(2015, 10, 21, 7, 28, 0).unwrap();
    assert_eq!(parse_retry_after("5", now), Some(Duration::from_secs(5)));
    assert_eq!(parse_retry_after(" 0 ", now), Some(Duration::ZERO));
    assert_eq!(
        parse_retry_after("Wed, 21 Oct 2015 07:28:30 GMT", now),
        Some(Duration::from_secs(30))
    );
    assert_eq!(
        parse_retry_after("Wed, 21 Oct 2015 07:00:00 GMT", now),
        Some(Duration::ZERO)
    );
    assert_eq!(parse_retry_after("soon", now), None);
    assert_eq!(parse_retry_after("-3", now), None);
}
