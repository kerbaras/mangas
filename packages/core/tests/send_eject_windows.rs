//! The pure half of the Windows eject: what a Plug and Play veto means, where a VHD's file is.

use mangas_core::send::eject::{backing_path, pnp_veto, StepError};

#[test]
fn an_open_handle_veto_is_busy_and_names_the_program() {
    let busy = pnp_veto(23, 5, "\\Device\\HarddiskVolume7 explorer.exe\0\0\0");
    assert_eq!(
        busy,
        StepError::Busy(
            "a program or service still has it open (\\Device\\HarddiskVolume7 explorer.exe)"
                .into()
        )
    );
    assert!(matches!(pnp_veto(23, 4, "WSearch"), StepError::Busy(_)));
}

#[test]
fn other_vetoes_refuse_in_plain_words() {
    let fixed = pnp_veto(23, 8, "");
    assert_eq!(
        fixed,
        StepError::Failed("Windows can't remove this kind of drive".into())
    );
    let odd = pnp_veto(23, 6, "USB\\VID_1949");
    assert_eq!(
        odd,
        StepError::Failed("Windows refused to remove the drive (veto 6) (USB\\VID_1949)".into())
    );
    assert_eq!(
        pnp_veto(13, 0, ""),
        StepError::Failed("Windows refused to remove the drive (error 13)".into())
    );
}

#[test]
fn a_vhd_path_joins_its_host_volume_and_relative_path() {
    let host = "\\\\?\\Volume{1b3c}\\";
    assert_eq!(
        backing_path(host, "a\\_temp\\kindle.vhdx"),
        "\\\\?\\Volume{1b3c}\\a\\_temp\\kindle.vhdx"
    );
    assert_eq!(
        backing_path(host, "\\a\\kindle.vhdx"),
        "\\\\?\\Volume{1b3c}\\a\\kindle.vhdx"
    );
}
