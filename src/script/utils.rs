/// Check if a string is a valid bash identifier
///
/// A valid bash identifier must:
/// - Not be empty
/// - Start with a letter (a-z, A-Z) or underscore (_)
/// - Contain only alphanumeric characters (a-z, A-Z, 0-9) or underscores (_)
pub fn is_valid_bash_identifier(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }

    // First character must be letter or underscore
    // Safety: we already checked `is_empty()` above, so `first` is always `Some`.
    let Some(first_char) = name.chars().next() else {
        return false;
    };
    if !first_char.is_ascii_alphabetic() && first_char != '_' {
        return false;
    }

    // Remaining characters must be alphanumeric or underscore
    name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_valid_bash_identifier_valid_names() {
        assert!(is_valid_bash_identifier("valid_name"));
        assert!(is_valid_bash_identifier("_underscore"));
        assert!(is_valid_bash_identifier("name123"));
        assert!(is_valid_bash_identifier("CamelCase"));
        assert!(is_valid_bash_identifier("UPPERCASE"));
        assert!(is_valid_bash_identifier("mixed_Case_123"));
        assert!(is_valid_bash_identifier("snake_case_123"));
    }

    #[test]
    fn test_is_valid_bash_identifier_invalid_names() {
        assert!(!is_valid_bash_identifier(""));
        assert!(!is_valid_bash_identifier("123start"));
        assert!(!is_valid_bash_identifier("has-dash"));
        assert!(!is_valid_bash_identifier("has space"));
        assert!(!is_valid_bash_identifier("has.dot"));
        assert!(!is_valid_bash_identifier("has$dollar"));
        assert!(!is_valid_bash_identifier("has@at"));
        assert!(!is_valid_bash_identifier("has!bang"));
    }
}
