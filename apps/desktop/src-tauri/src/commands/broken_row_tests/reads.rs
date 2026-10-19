//! With the settings row broken, read-only commands use the default settings while writes keep
//! refusing, so the broken row is never saved over.

use std::path::Path;

use mangas_core::library::updates::UpdatePolicy;
use mangas_core::library::Settings;

use super::super::{catalog, diagnostics, import, open_library, providers, set_settings, storage};
use super::chapter_dir;

pub(super) async fn reads_use_the_defaults_and_writes_refuse(home: &Path) {
    // A chapter refresh gets past the settings to the provider, which the dead proxy stops.
    let refresh = catalog::list_chapters("mangadex".into(), "solo".into(), "en".into(), true);
    let err = refresh.await.unwrap_err();
    assert!(!err.contains("settings"), "{err}");

    let groups = catalog::search_all("solo".into(), 5, vec!["mangadex".into()]).await;
    assert_eq!(groups.unwrap().len(), 1);
    providers::provider_status("mangadex".into()).await.unwrap();
    let report = diagnostics::diagnostics().unwrap().report;
    assert!(
        report.contains(&home.join("mangas").display().to_string()),
        "{report}"
    );
    storage::storage_usage().await.unwrap();

    let dirs = vec![chapter_dir(home, "Imported 1")];
    let imported = import::import_local(dirs, Some("Imported".into()), "en".into()).await;
    assert_eq!(imported.unwrap().imported.len(), 1);
    let lib = open_library().unwrap();
    let checked = mangas_providers::updates::check_updates(&lib, &UpdatePolicy::default()).await;
    assert!(checked.unwrap().checked > 0);

    let err = set_settings(Settings::default(), None).await.unwrap_err();
    assert!(err.contains("settings"), "{err}");
}
