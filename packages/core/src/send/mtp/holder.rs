//! Who holds a Kindle the copy couldn't open (macOS): `ptpcamerad` claims MTP devices on
//! connect, Android File Transfer does too. IORegistry's `UsbExclusiveOwner` names the holder
//! (`"pid 45145, ptpcamerad"`). mangas names it and the command that frees it; it never kills
//! it (`docs/research/usb-delivery.md`, macOS).

/// The holder of the device and how the user frees it, as the error shows them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Holder {
    pub who: String,
    pub fix: String,
}

/// The `UsbExclusiveOwner` values in `ioreg -l -w 0` output, `ptpcamerad` first.
pub fn owners(ioreg: &str) -> Vec<String> {
    let mut found: Vec<String> = ioreg
        .lines()
        .filter(|line| line.contains("\"UsbExclusiveOwner\""))
        .filter_map(|line| {
            line.split_once('=')
                .map(|(_, v)| v.trim().trim_matches('"').trim())
        })
        .filter(|v| !v.is_empty())
        .map(str::to_string)
        .collect();
    found.sort_by_key(|owner| !owner.contains("ptpcamera"));
    found.dedup();
    found
}

/// What to tell the user about `owner` (a `UsbExclusiveOwner` value, `None` when unknown).
pub fn holder(owner: Option<&str>) -> Holder {
    let Some(owner) = owner else {
        return Holder {
            who: "another app".into(),
            fix: "quit Android File Transfer or any other app that uses the Kindle, then try \
                  again (`ioreg -l -w 0 | grep UsbExclusiveOwner` in Terminal names it)"
                .into(),
        };
    };
    let (pid, name) = match owner.strip_prefix("pid ").and_then(|r| r.split_once(", ")) {
        Some((pid, name)) => (Some(pid.trim()), name.trim()),
        None => (None, owner),
    };
    let who = match pid {
        Some(pid) => format!("{name} (pid {pid})"),
        None => name.to_string(),
    };
    let fix = if name.contains("ptpcamera") {
        "macOS's camera import service claims the Kindle and comes back when quit. Run \
         `while true; do pkill -9 ptpcamerad; sleep 1; done` in Terminal, copy again, then stop \
         it with Ctrl-C"
            .to_string()
    } else {
        format!("quit {name}, then try again")
    };
    Holder { who, fix }
}

/// Asks IORegistry who holds a USB device; `None` off macOS or when `ioreg` says nothing.
pub fn exclusive_owner() -> Option<String> {
    if !cfg!(target_os = "macos") {
        return None;
    }
    let out = std::process::Command::new("ioreg")
        .args(["-l", "-w", "0"])
        .output()
        .ok()?;
    owners(&String::from_utf8_lossy(&out.stdout))
        .into_iter()
        .next()
}

#[cfg(test)]
mod tests {
    use super::*;

    const IOREG: &str = r#"
    | |   "USB Product Name" = "Kindle"
    | |   "UsbExclusiveOwner" = "pid 812, Android File Transfer Agent"
    | |   "UsbExclusiveOwner" = "pid 45145, ptpcamerad"
    | |   "UsbExclusiveOwner" = ""
    "#;

    #[test]
    fn owners_put_ptpcamerad_first_and_skip_empty_values() {
        assert_eq!(
            owners(IOREG),
            [
                "pid 45145, ptpcamerad",
                "pid 812, Android File Transfer Agent"
            ]
        );
        assert!(owners("\"USB Product Name\" = \"Kindle\"").is_empty());
    }

    #[test]
    fn ptpcamerad_gets_the_kill_loop_and_never_a_kill_by_us() {
        let h = holder(Some("pid 45145, ptpcamerad"));
        assert_eq!(h.who, "ptpcamerad (pid 45145)");
        assert!(h
            .fix
            .contains("while true; do pkill -9 ptpcamerad; sleep 1; done"));
    }

    #[test]
    fn another_app_is_named_with_its_pid_and_asked_to_quit() {
        let h = holder(Some("pid 812, Android File Transfer Agent"));
        assert_eq!(h.who, "Android File Transfer Agent (pid 812)");
        assert_eq!(h.fix, "quit Android File Transfer Agent, then try again");
        assert_eq!(holder(Some("Preview")).fix, "quit Preview, then try again");
        assert_eq!(holder(None).who, "another app");
    }
}
