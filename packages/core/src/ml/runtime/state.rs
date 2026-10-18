//! The last runtime download or load failure in this process, shown by the status.

use std::sync::Mutex;

static FAILURE: Mutex<Option<String>> = Mutex::new(None);

pub fn record(reason: String) {
    if let Ok(mut failure) = FAILURE.lock() {
        *failure = Some(reason);
    }
}

pub fn clear() {
    if let Ok(mut failure) = FAILURE.lock() {
        *failure = None;
    }
}

pub fn failure() -> Option<String> {
    FAILURE.lock().ok().and_then(|f| f.clone())
}
