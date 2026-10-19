//! Version comparison between the running app and a published release.

use semver::Version;

use super::Channel;

/// `0.5.0`, `v0.5.0-rc.2` (a leading `v` is allowed).
pub fn parse_version(text: &str) -> Result<Version, semver::Error> {
    let text = text.trim();
    Version::parse(text.strip_prefix('v').unwrap_or(text))
}

/// Whether `remote` should replace `current` on `channel`: only a strictly newer version, and a
/// pre-release only on the rc channel. The same version and older ones (a downgrade) never do.
pub fn should_update(current: &Version, remote: &Version, channel: Channel) -> bool {
    remote > current && (channel == Channel::Rc || remote.pre.is_empty())
}

/// Whether the release offered is older than the running app: refused, and said so.
pub fn is_older(current: &Version, remote: &Version) -> bool {
    remote < current
}

#[cfg(test)]
mod tests {
    use super::*;

    fn update(current: &str, remote: &str, channel: Channel) -> bool {
        let (Ok(current), Ok(remote)) = (parse_version(current), parse_version(remote)) else {
            panic!("bad test version");
        };
        should_update(&current, &remote, channel)
    }

    #[test]
    fn a_newer_stable_release_updates_both_channels() {
        assert!(update("0.4.0", "0.5.0", Channel::Stable));
        assert!(update("0.5.0-rc.3", "0.5.0", Channel::Rc));
        assert!(update("0.5.0-rc.3", "0.5.0", Channel::Stable));
    }

    #[test]
    fn release_candidates_only_reach_the_rc_channel() {
        assert!(update("0.4.0", "0.5.0-rc.1", Channel::Rc));
        assert!(update("0.5.0-rc.1", "0.5.0-rc.2", Channel::Rc));
        assert!(!update("0.4.0", "0.5.0-rc.1", Channel::Stable));
    }

    #[test]
    fn the_same_version_is_up_to_date() {
        assert!(!update("0.5.0", "v0.5.0", Channel::Stable));
        assert!(!update("0.5.0-rc.2", "0.5.0-rc.2", Channel::Rc));
    }

    #[test]
    fn downgrades_are_refused() {
        assert!(!update("0.5.0", "0.4.9", Channel::Stable));
        assert!(!update("0.5.0", "0.5.0-rc.9", Channel::Rc));
        assert!(!update("0.5.0-rc.2", "0.5.0-rc.10", Channel::Stable));
    }

    #[test]
    fn older_is_only_strictly_older() {
        let v = |s: &str| parse_version(s).unwrap_or_else(|_| panic!("bad test version"));
        assert!(is_older(&v("0.3.9"), &v("0.3.0")));
        assert!(is_older(&v("0.5.0"), &v("0.5.0-rc.9")));
        assert!(!is_older(&v("0.5.0"), &v("0.5.0")));
        assert!(!is_older(&v("0.5.0"), &v("0.6.0-rc.1")));
    }

    #[test]
    fn rc_numbers_compare_numerically() {
        assert!(update("0.5.0-rc.2", "0.5.0-rc.10", Channel::Rc));
    }
}
