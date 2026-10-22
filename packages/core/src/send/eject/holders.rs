//! Which programs hold files open on a volume, from `lsof` or `fuser` when they're installed.

/// Program names from `lsof -F pc` output (`p<pid>` then `c<name>` lines), without `own_pid`,
/// each name once, in order.
pub fn parse_lsof(out: &str, own_pid: u32) -> Vec<String> {
    let mut names = Vec::new();
    let mut skip = false;
    for line in out.lines() {
        if let Some(pid) = line.strip_prefix('p') {
            skip = pid.trim().parse::<u32>().ok() == Some(own_pid);
        } else if let Some(name) = line.strip_prefix('c') {
            push(&mut names, name, skip);
        }
    }
    names
}

/// PIDs from `fuser -m` output ("/media/x:  412c  977"; the PIDs go to stdout, the rest to
/// stderr), without `own_pid`.
pub fn parse_fuser(out: &str, own_pid: u32) -> Vec<u32> {
    out.split_whitespace()
        .filter_map(|w| {
            w.trim_end_matches(|c: char| c.is_ascii_alphabetic())
                .parse()
                .ok()
        })
        .filter(|&pid| pid != own_pid)
        .collect()
}

fn push(names: &mut Vec<String>, name: &str, skip: bool) {
    let name = name.trim();
    if !skip && !name.is_empty() && !names.iter().any(|n| n == name) {
        names.push(name.to_string());
    }
}

/// The programs holding files under `mount`: `lsof`, then `fuser` + `/proc` on Linux.
#[cfg(unix)]
pub(super) fn holders(mount: &std::path::Path) -> Vec<String> {
    use std::process::Command;
    let own = std::process::id();
    let lsof = Command::new("lsof")
        .arg("-w")
        .arg("-Fpc")
        .arg(mount)
        .output();
    if let Ok(out) = lsof {
        let names = parse_lsof(&String::from_utf8_lossy(&out.stdout), own);
        if !names.is_empty() {
            return names;
        }
    }
    if !cfg!(target_os = "linux") {
        return Vec::new();
    }
    let Ok(out) = Command::new("fuser").arg("-m").arg(mount).output() else {
        return Vec::new();
    };
    let mut names = Vec::new();
    for pid in parse_fuser(&String::from_utf8_lossy(&out.stdout), own) {
        if let Ok(comm) = std::fs::read_to_string(format!("/proc/{pid}/comm")) {
            push(&mut names, &comm, false);
        }
    }
    names
}

/// Windows names no program: its lock failure is the message.
#[cfg(not(unix))]
pub(super) fn holders(_mount: &std::path::Path) -> Vec<String> {
    Vec::new()
}
