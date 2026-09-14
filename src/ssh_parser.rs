use regex::Regex;
use std::sync::LazyLock;

static SSH_INVALID_USER: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"Invalid user \S+ from (\S+) port \d+").expect("ssh invalid user regex")
});

static SSH_FAILED_PASSWORD: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"Failed password for \S+ from (\S+) port \d+").expect("ssh failed password regex")
});

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SshEvent {
    InvalidUser { ip: String },
    FailedPassword { ip: String },
}

pub fn parse_ssh_line(line: &str) -> Option<SshEvent> {
    let line = line.trim();
    if line.is_empty() {
        return None;
    }

    if let Some(caps) = SSH_INVALID_USER.captures(line) {
        return Some(SshEvent::InvalidUser {
            ip: caps.get(1)?.as_str().to_owned(),
        });
    }

    // "Failed password for invalid user …" follows "Invalid user …" on the same attempt — ignore duplicate.

    if let Some(caps) = SSH_FAILED_PASSWORD.captures(line) {
        return Some(SshEvent::FailedPassword {
            ip: caps.get(1)?.as_str().to_owned(),
        });
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_user_is_instant_signal() {
        let line = "Invalid user lee from 223.27.18.80 port 36398";
        assert_eq!(
            parse_ssh_line(line),
            Some(SshEvent::InvalidUser {
                ip: "223.27.18.80".into()
            })
        );
    }

    #[test]
    fn parses_failed_password() {
        let line = "Failed password for root from 203.0.113.10 port 54321 ssh2";
        assert_eq!(
            parse_ssh_line(line),
            Some(SshEvent::FailedPassword {
                ip: "203.0.113.10".into()
            })
        );
    }

    #[test]
    fn failed_password_invalid_user_is_ignored() {
        let line = "Failed password for invalid user admin from 203.0.113.10 port 54321 ssh2";
        assert_eq!(parse_ssh_line(line), None);
    }

    #[test]
    fn ignores_successful_auth() {
        let line = "Accepted password for root from 203.0.113.10 port 54321 ssh2";
        assert_eq!(parse_ssh_line(line), None);
    }
}
