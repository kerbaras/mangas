use super::*;
use std::os::unix::fs::PermissionsExt;

#[test]
fn a_folder_we_can_write_but_not_list_still_takes_the_export() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("drop");
    std::fs::create_dir(&dir).unwrap();
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o300)).unwrap();
    if std::fs::read_dir(&dir).is_ok() {
        return; // root ignores the mode; nothing to test
    }
    let wanted = vec!["Book.epub".to_string()];
    assert_eq!(free_names_in(&dir, &wanted).unwrap(), wanted);
    let written = write_file(&dir.join("Book.epub"), false, |_| Ok(()));
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700)).unwrap();
    assert_eq!(written.unwrap(), dir.join("Book.epub"));
}

#[test]
fn a_refused_unique_part_names_the_folder_once() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("title");
    std::fs::create_dir(&dir).unwrap();
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o500)).unwrap();
    let written = write_over_unique(&dir, &dir.join("4.cbz"), |_| Ok(()));
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700)).unwrap();
    let Err(err) = written else { return }; // root ignores the mode; nothing to test
    let msg = err.to_string();
    assert_eq!(msg.matches(&*dir.to_string_lossy()).count(), 1, "{msg}");
    assert!(!msg.contains(".part") && !msg.contains("at path"), "{msg}");
}
