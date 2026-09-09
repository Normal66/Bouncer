pub fn path_matches_entry(path: &str, rule: &str) -> bool {
    let rule = rule.trim();
    if rule.is_empty() {
        return false;
    }

    let rule = rule.trim_end_matches('/');
    if path == rule {
        return true;
    }

    path.starts_with(&format!("{rule}/"))
}

pub fn matches_any_path(path: &str, rules: &[String]) -> bool {
    rules.iter().any(|rule| path_matches_entry(path, rule))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_and_prefix_match() {
        assert!(path_matches_entry("/.env", "/.env"));
        assert!(path_matches_entry("/wp-admin/setup.php", "/wp-admin"));
        assert!(!path_matches_entry("/api/env", "/.env"));
    }
}
