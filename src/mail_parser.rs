use regex::Regex;
use std::sync::LazyLock;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MailEvent {
    InvalidUser { ip: String },
    AuthFailed { ip: String },
}

static BRACKET_IP: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\[([\da-fA-F.:]+)\]").expect("bracket ip regex"));

static RIP_IP: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"rip=([\da-fA-F.:]+)").expect("rip ip regex"));

static COMMA_IP: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r",([\da-fA-F.:]+)\)").expect("comma ip regex"));

pub fn parse_mail_line(line: &str) -> Option<MailEvent> {
    let line = line.trim();
    if line.is_empty() {
        return None;
    }

    let lower = line.to_ascii_lowercase();
    let ip = extract_ip(line)?;

    if is_invalid_user_line(&lower) {
        return Some(MailEvent::InvalidUser { ip });
    }

    if is_auth_failed_line(&lower) {
        return Some(MailEvent::AuthFailed { ip });
    }

    None
}

fn is_invalid_user_line(lower: &str) -> bool {
    lower.contains("unknown user")
        || lower.contains("user unknown")
        || lower.contains("invalid user")
        || lower.contains("recipient address rejected: user unknown")
        || lower.contains("no such user")
}

fn is_auth_failed_line(lower: &str) -> bool {
    lower.contains("authentication failed")
        || lower.contains("auth process failed")
        || lower.contains("auth failed")
        || lower.contains("password mismatch")
        || lower.contains("login failed")
}

fn extract_ip(line: &str) -> Option<String> {
    if let Some(caps) = RIP_IP.captures(line) {
        return caps.get(1).map(|m| m.as_str().to_owned());
    }

    if let Some(caps) = COMMA_IP.captures(line) {
        return caps.get(1).map(|m| m.as_str().to_owned());
    }

    BRACKET_IP
        .captures_iter(line)
        .filter_map(|caps| caps.get(1).map(|m| m.as_str().to_owned()))
        .find(|value| value.contains('.'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_postfix_sasl_failure() {
        let line = "postfix/smtpd[12345]: warning: unknown[203.0.113.10]: SASL LOGIN authentication failed: authentication failure";
        assert_eq!(
            parse_mail_line(line),
            Some(MailEvent::AuthFailed {
                ip: "203.0.113.10".into()
            })
        );
    }

    #[test]
    fn parses_postfix_unknown_user() {
        let line = "postfix/smtpd[12345]: NOQUEUE: reject: RCPT from unknown[203.0.113.10]: 550 5.1.1 User unknown in virtual alias table";
        assert_eq!(
            parse_mail_line(line),
            Some(MailEvent::InvalidUser {
                ip: "203.0.113.10".into()
            })
        );
    }

    #[test]
    fn parses_dovecot_auth_failed() {
        let line = "dovecot: auth-worker(9123): conn unix:login (pid=9124,uid=0): auth failed, rip=203.0.113.10, lip=10.0.0.1";
        assert_eq!(
            parse_mail_line(line),
            Some(MailEvent::AuthFailed {
                ip: "203.0.113.10".into()
            })
        );
    }

    #[test]
    fn parses_dovecot_unknown_user() {
        let line = "dovecot: auth: passwd-file(user@example.com,203.0.113.10): unknown user";
        assert_eq!(
            parse_mail_line(line),
            Some(MailEvent::InvalidUser {
                ip: "203.0.113.10".into()
            })
        );
    }
}
