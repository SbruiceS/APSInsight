//! Stable identifiers and identifier hygiene (injection resistance at the edges).

use crate::error::{ApsiError, Result};

/// Slugs used for tenants, domains, entity kinds and rule ids.
/// Restricting them to `[a-z0-9][a-z0-9_-]{0,63}` keeps them safe in paths, logs,
/// CSV and file names.
pub fn validate_slug(kind: &str, value: &str) -> Result<()> {
    let ok = !value.is_empty()
        && value.len() <= 64
        && value.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-' || b == b'_')
        && value.as_bytes()[0].is_ascii_alphanumeric();
    if ok { Ok(()) } else { Err(ApsiError::InvalidInput(format!("{kind} must match [a-z0-9][a-z0-9_-]{{0,63}}"))) }
}

/// Field names additionally may not start with a digit, so they map cleanly onto
/// JSON, CSV headers and most external schema languages.
pub fn validate_field_name(value: &str) -> Result<()> {
    validate_slug("field name", value)?;
    if value.as_bytes()[0].is_ascii_digit() {
        return Err(ApsiError::InvalidInput("field name must not start with a digit".into()));
    }
    Ok(())
}

/// Record identifiers: `rec_<uuidv7>` — time ordered, globally unique, opaque.
pub fn new_record_id() -> String {
    new_id("rec")
}

pub fn new_id(prefix: &str) -> String {
    format!("{prefix}_{}", uuid::Uuid::now_v7().simple())
}

pub fn validate_record_id(value: &str) -> Result<()> {
    let ok = value.len() == 36 && value.starts_with("rec_") && value[4..].bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b));
    if ok { Ok(()) } else { Err(ApsiError::InvalidInput("malformed record id".into())) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn slugs() {
        assert!(validate_slug("t", "soil-profile_1").is_ok());
        let long = "a".repeat(65);
        for bad in ["", "-x", "A", "a b", "a/b", "a;drop", "../etc", long.as_str()] {
            assert!(validate_slug("t", bad).is_err(), "{bad}");
        }
        assert!(validate_field_name("1abc").is_err());
        assert!(validate_field_name("ph_value").is_ok());
    }

    #[test]
    fn record_ids_are_unique_and_valid() {
        let a = new_record_id();
        let b = new_record_id();
        assert_ne!(a, b);
        validate_record_id(&a).unwrap();
        assert!(validate_record_id("rec_../../etc").is_err());
        assert!(validate_record_id("rec_ü").is_err());
    }

    proptest! {
        #[test]
        fn validation_never_panics(s in ".*") {
            let _ = validate_slug("x", &s);
            let _ = validate_field_name(&s);
            let _ = validate_record_id(&s);
        }

        #[test]
        fn accepted_slugs_contain_only_safe_bytes(s in "[a-z0-9][a-z0-9_-]{0,63}") {
            prop_assert!(validate_slug("x", &s).is_ok());
        }
    }
}
