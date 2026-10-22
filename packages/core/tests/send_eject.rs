//! Eject decisions over a fake OS: which strategy runs, and what the user reads when the
//! volume is busy or the OS refuses.

use std::cell::RefCell;
use std::path::{Path, PathBuf};

use mangas_core::send::eject::{
    busy_text, dbus_error, dissenter, eject_with, ejectable, parse_fuser, parse_lsof, strategies,
    Backend, Os, StepError, Strategy, STILL_THERE,
};
use mangas_core::send::KindleVolume;
use mangas_core::Error;

#[derive(Default)]
struct Fake {
    answers: Vec<(Strategy, Result<bool, StepError>)>,
    holders: Vec<String>,
    flush_fails: bool,
    ran: RefCell<Vec<String>>,
}

impl Backend for Fake {
    fn mount_point(&self, path: &Path) -> Option<PathBuf> {
        Some(path.to_path_buf())
    }
    fn flush(&self, files: &[PathBuf]) -> std::io::Result<()> {
        self.ran.borrow_mut().push(format!("flush {}", files.len()));
        if self.flush_fails {
            return Err(std::io::Error::other("I/O error"));
        }
        Ok(())
    }
    fn run(&self, strategy: Strategy, _: &Path) -> Result<bool, StepError> {
        self.ran.borrow_mut().push(format!("{strategy:?}"));
        let found = self.answers.iter().find(|(s, _)| *s == strategy);
        found.map_or(Ok(true), |(_, r)| r.clone())
    }
    fn holders(&self, _: &Path) -> Vec<String> {
        self.holders.clone()
    }
}

fn kindle() -> KindleVolume {
    KindleVolume {
        name: "KINDLE".into(),
        root: "/media/me/KINDLE".into(),
        documents: "/media/me/KINDLE/documents".into(),
        ejectable: true,
    }
}

fn book() -> Vec<PathBuf> {
    vec!["/media/me/KINDLE/documents/book.azw3".into()]
}

#[test]
fn linux_tries_udisks_then_umount_and_forcing_picks_one() {
    assert_eq!(
        strategies(Os::Linux, None),
        [Strategy::Udisks, Strategy::Umount]
    );
    assert_eq!(strategies(Os::Macos, None), [Strategy::Diskutil]);
    assert_eq!(strategies(Os::Windows, None), [Strategy::Windows]);
    assert_eq!(
        strategies(Os::Linux, Some(Strategy::Umount)),
        [Strategy::Umount]
    );
    assert_eq!(Strategy::parse("umount"), Some(Strategy::Umount));
}

#[test]
fn files_are_flushed_before_the_first_strategy_runs() {
    let fake = Fake::default();
    let done = eject_with(&fake, &kindle(), &book(), Some(Strategy::Udisks)).unwrap();
    assert_eq!(done.device, "KINDLE");
    assert!(done.powered_off);
    assert_eq!(*fake.ran.borrow(), ["flush 1", "Udisks"]);
}

#[test]
fn without_udisks_it_falls_back_to_umount() {
    let fake = Fake {
        answers: vec![(
            Strategy::Udisks,
            Err(StepError::Unavailable("no udisks".into())),
        )],
        ..Fake::default()
    };
    let done = eject_with(&fake, &kindle(), &book(), None);
    if cfg!(target_os = "linux") {
        assert_eq!(done.unwrap().strategy, Strategy::Umount);
        assert_eq!(*fake.ran.borrow(), ["flush 1", "Udisks", "Umount"]);
    }
}

#[test]
fn a_busy_volume_names_the_programs_to_close() {
    let fake = Fake {
        answers: vec![(
            Strategy::Udisks,
            Err(StepError::Busy("target is busy".into())),
        )],
        holders: vec!["less".into(), "vlc".into()],
        ..Fake::default()
    };
    let err = eject_with(&fake, &kindle(), &book(), Some(Strategy::Udisks)).unwrap_err();
    assert!(matches!(&err, Error::VolumeBusy { holders, .. } if holders.len() == 2));
    assert_eq!(
        err.to_string(),
        format!("can't eject KINDLE at /media/me/KINDLE: it's in use by less and vlc. Close them and eject again. {STILL_THERE}")
    );
}

#[test]
fn a_busy_volume_without_names_gives_the_os_answer() {
    let text = busy_text("E:", &[], "Windows couldn't lock E: (Access is denied.)");
    assert!(
        text.contains("it's in use (Windows couldn't lock E: (Access is denied.))"),
        "{text}"
    );
    assert!(text.ends_with(STILL_THERE));
    let one = busy_text("K", &["bash".into()], "");
    assert!(one.contains("in use by bash. Close it and"), "{one}");
}

#[test]
fn a_refused_eject_says_the_books_are_still_there() {
    let fake = Fake {
        answers: vec![(
            Strategy::Diskutil,
            Err(StepError::Failed("Disk not ejected".into())),
        )],
        ..Fake::default()
    };
    let err = eject_with(&fake, &kindle(), &book(), Some(Strategy::Diskutil)).unwrap_err();
    assert!(matches!(err, Error::EjectRefused { .. }));
    assert_eq!(
        err.to_string(),
        format!("can't eject KINDLE at /media/me/KINDLE: Disk not ejected. {STILL_THERE}")
    );
}

#[test]
fn a_failed_flush_stops_before_unmounting() {
    let fake = Fake {
        flush_fails: true,
        ..Fake::default()
    };
    let err = eject_with(&fake, &kindle(), &book(), None).unwrap_err();
    assert!(
        err.to_string()
            .contains("couldn't be written to it (I/O error)"),
        "{err}"
    );
    assert_eq!(*fake.ran.borrow(), ["flush 1"]);
}

#[test]
fn an_mtp_kindle_has_nothing_to_eject() {
    let root = Path::new("/run/user/1000/gvfs/mtp:host=Amazon_Kindle_G000/Internal Storage");
    assert!(!ejectable(root));
    assert!(ejectable(Path::new("/media/me/KINDLE")));
    let mtp = KindleVolume {
        ejectable: false,
        ..kindle()
    };
    let fake = Fake::default();
    let err = eject_with(&fake, &mtp, &book(), None).unwrap_err();
    assert!(err
        .to_string()
        .contains("connected over MTP, so there's nothing to eject"));
    assert!(fake.ran.borrow().is_empty());
}

#[test]
fn udisks_errors_are_classified_by_name() {
    let busy = dbus_error("org.freedesktop.UDisks2.Error.DeviceBusy", "target is busy");
    assert_eq!(busy, StepError::Busy("target is busy".into()));
    let gone = dbus_error("org.freedesktop.DBus.Error.ServiceUnknown", "");
    assert!(matches!(gone, StepError::Unavailable(_)));
    let masked = dbus_error(
        "org.freedesktop.systemd1.UnitMasked",
        "Unit udisks2.service is masked.",
    );
    assert!(matches!(masked, StepError::Unavailable(_)));
    let failed = dbus_error("org.freedesktop.DBus.Error.Spawn.ChildExited", "exited 1");
    assert!(matches!(failed, StepError::Unavailable(_)));
    let denied = dbus_error("org.freedesktop.UDisks2.Error.NotAuthorizedCanObtain", "no");
    assert_eq!(denied, StepError::Failed("no".into()));
}

#[test]
fn holder_names_come_from_lsof_fuser_and_diskutil() {
    let lsof = "p412\ncless\nf3\np977\ncvlc\np413\ncless\np1\ncmangas\n";
    assert_eq!(parse_lsof(lsof, 1), ["less", "vlc"]);
    assert_eq!(parse_fuser(" 412c  977 1m", 1), [412, 977]);
    let mac = "Volume KINDLE on disk4s1 failed to unmount: dissented by PID 412 (/usr/bin/less)";
    assert_eq!(dissenter(mac).as_deref(), Some("less"));
    assert_eq!(dissenter("Disk /dev/disk4 ejected"), None);
}
