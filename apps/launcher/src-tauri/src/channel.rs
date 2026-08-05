//! Release channel of this build.
//!
//! The channel is baked in at compile time from `PRIVATE_CLIENT_CHANNEL`, so a
//! running launcher cannot be talked into changing it — not by a setting, not
//! by a frontend call, not by a file on disk. That matters because the channel
//! selects the data root: if it were mutable at runtime, a stable install could
//! be pointed at the beta instance (or the reverse) and quietly corrupt it.
//!
//! Stable is the default. An unrecognized value fails the build rather than
//! silently falling back, because "typo in the build script" must never
//! produce a beta binary that installs over the public one.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Channel {
    Stable,
    Beta,
}

const RAW: Option<&str> = option_env!("PRIVATE_CLIENT_CHANNEL");

pub const CHANNEL: Channel = match RAW {
    None => Channel::Stable,
    Some(value) => {
        // `match` on &str is not const-evaluable, so compare bytes instead.
        if const_eq(value, "stable") {
            Channel::Stable
        } else if const_eq(value, "beta") {
            Channel::Beta
        } else {
            panic!("PRIVATE_CLIENT_CHANNEL must be either \"stable\" or \"beta\"")
        }
    }
};

const fn const_eq(left: &str, right: &str) -> bool {
    let (left, right) = (left.as_bytes(), right.as_bytes());
    if left.len() != right.len() {
        return false;
    }
    let mut index = 0;
    while index < left.len() {
        if left[index] != right[index] {
            return false;
        }
        index += 1;
    }
    true
}

impl Channel {
    /// Directory name under `%LOCALAPPDATA%`. Distinct per channel so the two
    /// installs never share settings, mods, profiles, locks, or the instance.
    pub const fn data_dir_name(self) -> &'static str {
        match self {
            Channel::Stable => "Private Client",
            Channel::Beta => "Private Client Beta",
        }
    }

    pub const fn is_beta(self) -> bool {
        matches!(self, Channel::Beta)
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Channel::Stable => "stable",
            Channel::Beta => "beta",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_two_channels_never_share_a_data_directory() {
        assert_ne!(
            Channel::Stable.data_dir_name(),
            Channel::Beta.data_dir_name()
        );
    }

    #[test]
    fn beta_directory_is_not_a_prefix_collision_of_stable() {
        // "Private Client Beta" must not resolve inside "Private Client".
        let stable = std::path::Path::new(Channel::Stable.data_dir_name());
        let beta = std::path::Path::new(Channel::Beta.data_dir_name());
        assert!(!beta.starts_with(stable));
    }

    #[test]
    fn an_unset_channel_builds_as_stable() {
        // This binary is built without the variable in the default workflow.
        if option_env!("PRIVATE_CLIENT_CHANNEL").is_none() {
            assert_eq!(CHANNEL, Channel::Stable);
        }
    }

    #[test]
    fn const_eq_matches_str_equality() {
        assert!(const_eq("beta", "beta"));
        assert!(!const_eq("beta", "stable"));
        assert!(!const_eq("beta", "bet"));
        assert!(!const_eq("", "beta"));
    }
}
