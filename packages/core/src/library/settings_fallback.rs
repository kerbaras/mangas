//! Reads that must not stop on a broken settings row fall back to the defaults (platform data
//! folder, system proxy). Writes keep [`get_settings`], so a broken row is never saved over.

use std::path::Path;
use std::sync::Mutex;

use crate::domain::Error;
use crate::library::{get_settings, Library, Settings};

/// The last fallback warning logged, so a broken row is reported once rather than on every read.
static LOGGED: Mutex<Option<String>> = Mutex::new(None);

/// The settings stored in the library at `db`, or the defaults when it can't be read, with the
/// warning to show.
pub fn settings_or_default(db: &Path) -> (Settings, Option<String>) {
    match crate::library::open(db) {
        Ok(lib) => settings_or_default_in(&lib),
        Err(e) => (Settings::default(), Some(fallback_warning(&e))),
    }
}

/// [`settings_or_default`] for a library that is already open.
pub fn settings_or_default_in(lib: &Library) -> (Settings, Option<String>) {
    match get_settings(lib) {
        Ok(settings) => (settings, None),
        Err(e) => (Settings::default(), Some(fallback_warning(&e))),
    }
}

/// [`settings_or_default_in`] with the warning logged through [`log_fallback`].
pub fn readable_settings(lib: &Library) -> Settings {
    let (settings, warning) = settings_or_default_in(lib);
    log_fallback(warning);
    settings
}

/// Logs a fallback warning unless it is the one logged last, so search-as-you-type and the
/// commands one screen runs together log a broken row once. A clean read (`None`) resets it.
pub fn log_fallback(warning: Option<String>) {
    let Ok(mut logged) = LOGGED.lock() else {
        return;
    };
    if is_new(&logged, &warning) {
        if let Some(w) = &warning {
            log::warn!("{w}");
        }
    }
    *logged = warning;
}

fn is_new(logged: &Option<String>, warning: &Option<String>) -> bool {
    warning.is_some() && logged != warning
}

fn fallback_warning(e: &Error) -> String {
    format!("could not read the settings, using the default data folder and the system proxy: {e}")
}

#[cfg(test)]
mod tests {
    use super::is_new;

    #[test]
    fn a_warning_is_new_until_a_clean_read_resets_it() {
        let broken = Some("broken".to_string());
        assert!(is_new(&None, &broken));
        assert!(!is_new(&broken, &broken));
        assert!(!is_new(&broken, &None));
        assert!(is_new(&Some("other".into()), &broken));
    }
}
