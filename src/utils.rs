//! Helper Functions & Data Logic
//! ---------------------------------------------------------
//! Provides low-level transformations used by both the XML parser and the
//! CLI pipeline:
//!
//! - `is_public_domain_license`: Detects public-domain statements.
//! - `clean_marc_subfields`: Removes MARC subfield codes (`$a`, `$b`)
//!   from text strings.
//! - `transform_url`: Converts Gutenberg URLs to mirror-based URLs,
//!   handling `files/` and `dirs/` paths as well as `cache/epub/` prefixes.
//! - `parse_lc_code`: Parses Library of Congress classification strings
//!   and performs prefix fallbacks (`D501` → `D` → `History`).

use crate::config::*;

/// Determines whether a license string indicates public-domain status.
///
/// Matches case-insensitively on the phrase `"public domain"`.
///
/// # Arguments
/// * `license` — Raw license text from the RDF `rights` node.
///
/// # Returns
/// `true` if the license contains `"public domain"` (case-insensitive).
pub fn is_public_domain_license(license: &str) -> bool {
    license.to_lowercase().contains("public domain")
}

/// Cleans MARC subfield markers from a text string.
///
/// Removes patterns such as `$a`, `$b`, etc., then collapses any resulting
/// whitespace (multiple spaces, tabs) into a single space and trims the
/// result.
///
/// Monetary amounts (`$100`, `$5`) and codes without a trailing boundary
/// (`$aThe Title`) are preserved by the underlying regex (`RE_MARC_SUBFIELD`).
///
/// # Arguments
/// * `s` — Raw string potentially containing MARC subfield codes.
///
/// # Returns
/// Cleaned, whitespace-normalized string.
pub fn clean_marc_subfields(s: &str) -> String {
    RE_MARC_SUBFIELD
        .replace_all(s, " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Transforms a Gutenberg URL to a mirror-compatible URL.
///
/// # Behavior
/// - If `url` is empty or `mirror_base` is empty, returns `url` unchanged.
/// - If `url` contains `/files/` or `/dirs/`, extracts the directory/file
///   components and rebuilds the path using the numeric ebook-ID prefix
///   structure required by Gutenberg mirrors.
/// - If `url` starts with a known Gutenberg base prefix (`www.gutenberg.org`,
///   `gutenberg.org`, etc.), strips it and applies the appropriate mirror
///   `cache/epub/` or direct mapping.
///
/// # Arguments
/// * `url` — Original URL from the RDF resource.
/// * `ebook_id` — Numeric ebook identifier (used for path reconstruction).
/// * `mirror_base` — Base URL of the selected mirror.
///
/// # Returns
/// Transformed URL, or `None` if the input `url` was `None`.
pub fn transform_url(url: Option<&str>, ebook_id: &str, mirror_base: &str) -> Option<String> {
    let url = url?;
    if url.is_empty() || mirror_base.is_empty() {
        return Some(url.to_string());
    }

    let mirror_clean = format!("{}/", mirror_base.trim_end_matches('/'));
    // If the mirror is the standard Gutenberg site, return unchanged.
    if mirror_clean == "https://www.gutenberg.org/" || mirror_clean == "http://www.gutenberg.org/" {
        return Some(url.to_string());
    }

    let ebook_id_clean = ebook_id.trim();
    let digit_path = if ebook_id_clean.len() <= 1 || !ebook_id_clean.chars().all(|c| c.is_ascii_digit()) {
        ebook_id_clean.to_string()
    } else {
        let chars: Vec<char> = ebook_id_clean.chars().collect();
        let prefix_parts: Vec<String> = chars[..chars.len() - 1].iter().map(|c| c.to_string()).collect();
        format!("{}/{}", prefix_parts.join("/"), ebook_id_clean)
    };

    // Handle `/files/` and `/dirs/` URLs.
    if url.contains("/files/") || url.contains("/dirs/") {
        if let Some(caps) = RE_FILES_DIRS.captures(url) {
            if &caps[1] == ebook_id_clean {
                return Some(format!("{}{}/{}", mirror_clean, digit_path, &caps[2]));
            }
        }
    }

    // Strip standard Gutenberg prefixes and remap.
    let prefixes = [
        "https://www.gutenberg.org/",
        "http://www.gutenberg.org/",
        "https://gutenberg.org/",
        "http://gutenberg.org/",
    ];

    for prefix in prefixes {
        if let Some(rel_path) = url.strip_prefix(prefix) {
            if let Some(file_part) = rel_path.strip_prefix("ebooks/") {
                return Some(format!("{}cache/epub/{}/pg{}", mirror_clean, ebook_id_clean, file_part));
            }
            return Some(format!("{}{}", mirror_clean, rel_path));
        }
    }

    Some(url.to_string())
}

/// Parses a Library of Congress classification string.
///
/// # Strategy
/// 1. Trim and uppercase the input.
/// 2. Validate with `RE_LC_CODE_VALID` (`A`-`ZZZ` + optional digits).
/// 3. Try direct lookup in `LC_MAP` (handles `D501`, `F350.5`).
/// 4. Fall back to prefix matches: 3-letter, 2-letter, 1-letter prefixes.
///
/// # Arguments
/// * `s` — Raw LC code string (e.g. `"DA"`, `"F350.5"`, `"D501"`).
///
/// # Returns
/// `Some((domain, sub_description))` if a match is found; `None` otherwise.
pub fn parse_lc_code(s: &str) -> Option<(&'static str, &'static str)> {
    let code = s.trim().to_uppercase();
    if !RE_LC_CODE_VALID.is_match(&code) {
        return None;
    }

    // Direct full-code match (handles numeric sub-codes like E186, D501).
    if let Some(&res) = LC_MAP.get(code.as_str()) {
        return Some(res);
    }

    // Prefix fallback: 3-letter → 2-letter → 1-letter.
    if let Some(caps) = RE_PREFIX.captures(&code) {
        let prefix = caps.get(1)?.as_str();
        if let Some(&res) = LC_MAP.get(prefix) {
            return Some(res);
        }
        if prefix.len() > 1 {
            if let Some(&res) = LC_MAP.get(&prefix[..2]) {
                return Some(res);
            }
        }
        if !prefix.is_empty() {
            if let Some(&res) = LC_MAP.get(&prefix[..1]) {
                return Some(res);
            }
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Unit Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Confirms `is_public_domain_license` detects public-domain text.
    #[test]
    fn public_domain_license_detected() {
        assert!(is_public_domain_license("Public domain in the USA."));
        assert!(!is_public_domain_license("Copyrighted"));
    }

    /// Basic subfield removal (`$a Hello`).
    #[test]
    fn clean_subfields_removes_codes() {
        assert_eq!(clean_marc_subfields("$a Hello"), "Hello");
    }

    /// Validates `parse_lc_code` direct numeric sub-code lookup (`D501`).
    #[test]
    fn parse_lc_code_fallback() {
        assert_eq!(parse_lc_code("D501"), Some(("History", "World War I")));
    }
}
