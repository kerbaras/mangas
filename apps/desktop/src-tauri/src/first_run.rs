//! First launch: no library database yet means a fresh install, which gets the setup sheet.

use mangas_core::library::{self, Settings};

use crate::paths;

/// Checked before anything opens (and so creates) the database.
pub fn is_fresh_install() -> bool {
    paths::library_db().is_ok_and(|db| !db.exists())
}

/// Stores `setup_complete: false`, which the UI reads as "show the setup sheet".
pub fn mark_setup_pending() {
    let marked = paths::library_db().and_then(|db| {
        let lib = library::open(&db).map_err(|e| e.to_string())?;
        let current = library::get_settings(&lib).map_err(|e| e.to_string())?;
        let edited = Settings {
            setup_complete: Some(false),
            ..current.clone()
        };
        library::edit_settings(&lib, Some(&current), &edited).map_err(|e| e.to_string())
    });
    if let Err(e) = marked {
        tracing::warn!("first-run setup won't show: {e}");
    }
}
