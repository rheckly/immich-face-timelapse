//! Shared utility functions.

use unicode_normalization::{char::is_combining_mark, UnicodeNormalization};

/// Create a safe snake_case folder name from person name or ID.
///
/// This sanitizes user input to create safe filesystem paths by:
/// - Using the person name if provided and non-empty, otherwise falling back to ID
/// - Removing accents and diacritical marks
/// - Converting to lowercase
/// - Replacing spaces with underscores
/// - Replacing unsafe characters with underscores
/// - Trimming whitespace and underscores
/// - Limiting length to 50 characters
///
/// Note: The frontend has a JavaScript version of this function that must be
/// kept in sync. See `frontend/src/lib/components/App.svelte`.
pub fn sanitize_folder_name(name: Option<&str>, id: &str) -> String {
    let base = name.filter(|n| !n.is_empty()).unwrap_or(id);

    // Remove accents by decomposing to NFD and filtering combining marks
    let without_accents: String = base.nfd().filter(|c| !is_combining_mark(*c)).collect();

    // Convert to lowercase and replace unsafe characters with underscores
    let sanitized: String = without_accents
        .to_lowercase()
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();

    // Trim whitespace/underscores and limit length (char-based to avoid UTF-8 boundary panics)
    let trimmed = sanitized.trim_matches(|c: char| c.is_whitespace() || c == '_');
    trimmed.chars().take(50).collect()
}

/// Build a free, versioned output folder name for one run: `<base>_<YYYY-MM-DD_HHMM>`.
///
/// If that folder already exists (two runs in the same minute), seconds are
/// appended (`_HHMMSS`), then a counter (`_HHMMSS_2`, `_3`, ...).
/// `exists` reports whether a candidate folder name is already taken.
pub fn versioned_folder_name(
    base: &str,
    now: chrono::NaiveDateTime,
    exists: impl Fn(&str) -> bool,
) -> String {
    let minute = format!("{}_{}", base, now.format("%Y-%m-%d_%H%M"));
    if !exists(&minute) {
        return minute;
    }
    let second = format!("{}_{}", base, now.format("%Y-%m-%d_%H%M%S"));
    if !exists(&second) {
        return second;
    }
    (2u32..)
        .map(|n| format!("{}_{}", second, n))
        .find(|candidate| !exists(candidate))
        .expect("unbounded counter always finds a free name")
}

/// Split an output folder name into its person part and optional version stamp.
///
/// `john_doe_2026-11-12_1430` → (`john_doe`, Some(`2026-11-12_1430`)).
/// Folders from before versioning (`john_doe`) → (`john_doe`, None).
/// Version stamps sort chronologically as plain strings.
pub fn split_folder_version(name: &str) -> (&str, Option<&str>) {
    for (i, _) in name.match_indices('_') {
        let suffix = &name[i + 1..];
        if i > 0 && is_version_stamp(suffix) {
            return (&name[..i], Some(suffix));
        }
    }
    (name, None)
}

/// Pick the versioned folders of `base` to delete so that only the `keep` newest remain.
///
/// `keep == 0` means unlimited. Folders without a version stamp (from before
/// versioning) and `current` are never selected.
pub fn versions_to_prune(names: &[String], base: &str, current: &str, keep: u32) -> Vec<String> {
    if keep == 0 {
        return Vec::new();
    }
    let mut versions: Vec<(&str, &String)> = names
        .iter()
        .filter_map(|name| match split_folder_version(name) {
            (person, Some(stamp)) if person == base => Some((stamp, name)),
            _ => None,
        })
        .collect();
    // Newest first (version stamps sort chronologically as strings)
    versions.sort();
    versions.reverse();
    versions
        .into_iter()
        .skip(keep as usize)
        .filter(|(_, name)| name.as_str() != current)
        .map(|(_, name)| name.clone())
        .collect()
}

/// Matches `YYYY-MM-DD_HHMM`, optionally followed by `SS` and/or `_<n>`.
fn is_version_stamp(s: &str) -> bool {
    let b = s.as_bytes();
    let digits = |r: std::ops::Range<usize>| b[r].iter().all(u8::is_ascii_digit);
    if b.len() < 15
        || !digits(0..4)
        || b[4] != b'-'
        || !digits(5..7)
        || b[7] != b'-'
        || !digits(8..10)
        || b[10] != b'_'
        || !digits(11..15)
    {
        return false;
    }
    let mut rest = &s[15..];
    if rest.len() >= 2 && rest.as_bytes()[..2].iter().all(u8::is_ascii_digit) {
        rest = &rest[2..];
    }
    match rest.strip_prefix('_') {
        None => rest.is_empty(),
        Some(n) => !n.is_empty() && n.bytes().all(|c| c.is_ascii_digit()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(s: &str) -> chrono::NaiveDateTime {
        chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S").unwrap()
    }

    #[test]
    fn test_versioned_folder_name_minute() {
        let name = versioned_folder_name("anna", at("2026-11-12 14:30:05"), |_| false);
        assert_eq!(name, "anna_2026-11-12_1430");
    }

    #[test]
    fn test_versioned_folder_name_collisions() {
        let now = at("2026-11-12 14:30:05");
        let taken = ["anna_2026-11-12_1430", "anna_2026-11-12_143005"];

        let name = versioned_folder_name("anna", now, |n| taken.contains(&n));
        assert_eq!(name, "anna_2026-11-12_143005_2");

        let name = versioned_folder_name("anna", now, |n| taken[..1].contains(&n));
        assert_eq!(name, "anna_2026-11-12_143005");
    }

    #[test]
    fn test_split_folder_version() {
        assert_eq!(
            split_folder_version("john_doe_2026-11-12_1430"),
            ("john_doe", Some("2026-11-12_1430"))
        );
        assert_eq!(
            split_folder_version("john_doe_2026-11-12_143005_2"),
            ("john_doe", Some("2026-11-12_143005_2"))
        );
        // Not versioned: kept as a whole
        for name in [
            "john_doe",
            "2026-11-12_1430",
            "anna_2026-11-12",
            "anna_2026-11-12_14x0",
        ] {
            assert_eq!(split_folder_version(name), (name, None));
        }
        // Round trip with the generator
        let name = versioned_folder_name("a_b", at("2026-01-02 03:04:05"), |_| false);
        assert_eq!(
            split_folder_version(&name),
            ("a_b", Some("2026-01-02_0304"))
        );
    }

    #[test]
    fn test_versions_to_prune() {
        let names: Vec<String> = [
            "anna",
            "anna_2026-11-10_0900",
            "anna_2026-11-11_0900",
            "anna_2026-11-12_0900",
            "anna_maria_2026-11-01_0900",
            "bob_2026-11-01_0900",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        let current = "anna_2026-11-12_0900";
        assert!(versions_to_prune(&names, "anna", current, 0).is_empty());
        assert_eq!(
            versions_to_prune(&names, "anna", current, 2),
            vec!["anna_2026-11-10_0900".to_string()]
        );
        assert_eq!(
            versions_to_prune(&names, "anna", current, 1),
            vec![
                "anna_2026-11-11_0900".to_string(),
                "anna_2026-11-10_0900".to_string(),
            ]
        );
        assert!(versions_to_prune(&names, "anna", current, 5).is_empty());
    }

    #[test]
    fn test_version_stamps_sort_chronologically() {
        let mut v = vec![
            "2026-11-12_143005_2",
            "2026-11-12_1431",
            "2026-11-12_1430",
            "2026-11-12_143005",
        ];
        v.sort();
        let expected = [
            "2026-11-12_1430",
            "2026-11-12_143005",
            "2026-11-12_143005_2",
            "2026-11-12_1431",
        ];
        assert_eq!(v, expected);
    }

    #[test]
    fn test_sanitize_folder_name_with_name() {
        assert_eq!(sanitize_folder_name(Some("John Doe"), "abc123"), "john_doe");
    }

    #[test]
    fn test_sanitize_folder_name_falls_back_to_id() {
        assert_eq!(sanitize_folder_name(None, "abc123"), "abc123");
        assert_eq!(sanitize_folder_name(Some(""), "abc123"), "abc123");
    }

    #[test]
    fn test_sanitize_folder_name_replaces_unsafe_chars() {
        assert_eq!(
            sanitize_folder_name(Some("John/Doe:Test"), "id"),
            "john_doe_test"
        );
    }

    #[test]
    fn test_sanitize_folder_name_trims_whitespace() {
        assert_eq!(sanitize_folder_name(Some("  John Doe  "), "id"), "john_doe");
    }

    #[test]
    fn test_sanitize_folder_name_limits_length() {
        let long_name = "A".repeat(100);
        let result = sanitize_folder_name(Some(&long_name), "id");
        assert_eq!(result.chars().count(), 50);
    }

    #[test]
    fn test_sanitize_folder_name_limits_length_multibyte() {
        // CJK characters are multi-byte in UTF-8 but pass is_alphanumeric()
        let long_name = "\u{4e2d}".repeat(100); // 100 CJK characters
        let result = sanitize_folder_name(Some(&long_name), "id");
        assert_eq!(result.chars().count(), 50);
    }

    #[test]
    fn test_sanitize_folder_name_removes_accents() {
        // French accents
        assert_eq!(sanitize_folder_name(Some("José"), "id"), "jose");
        assert_eq!(sanitize_folder_name(Some("René"), "id"), "rene");
        assert_eq!(sanitize_folder_name(Some("François"), "id"), "francois");

        // German umlauts
        assert_eq!(sanitize_folder_name(Some("Müller"), "id"), "muller");
        assert_eq!(sanitize_folder_name(Some("Björk"), "id"), "bjork");

        // Spanish
        assert_eq!(sanitize_folder_name(Some("Peña"), "id"), "pena");

        // Mixed accents
        assert_eq!(
            sanitize_folder_name(Some("Élève Français"), "id"),
            "eleve_francais"
        );
    }
}
