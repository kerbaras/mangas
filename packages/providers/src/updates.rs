//! Update checks over the registered providers: [`mangas_core::library::updates::check`] with
//! each provider built from the stored settings and asked for the user's languages.

use mangas_core::domain::Result;
use mangas_core::library::updates::{self, SourceFor, UpdatePolicy, UpdateReport};
use mangas_core::library::{get_settings, Library, Settings};

use crate::{languages_for, registry};

/// Runs a check with the providers configured as the user set them up.
pub async fn check_updates(lib: &Library, policy: &UpdatePolicy) -> Result<UpdateReport> {
    let settings = get_settings(lib)?;
    let resolve = |id: &str| -> Result<SourceFor> {
        Ok(SourceFor {
            source: registry::source(id, &settings)?,
            languages: languages(id, &settings),
        })
    };
    updates::check(lib, &resolve, policy).await
}

/// Like [`check_updates`] with every provider's requests sent to `base` (tests).
pub async fn check_updates_at(
    lib: &Library,
    policy: &UpdatePolicy,
    base: &str,
) -> Result<UpdateReport> {
    let settings = get_settings(lib)?;
    let resolve = |id: &str| -> Result<SourceFor> {
        Ok(SourceFor {
            source: registry::source_at(id, &settings, base)?,
            languages: languages(id, &settings),
        })
    };
    updates::check(lib, &resolve, policy).await
}

fn languages(id: &str, settings: &Settings) -> Vec<String> {
    languages_for(id, &settings.languages_for(id))
}
