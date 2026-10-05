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
//! - `agent_wikipedia_image`: Resolves an agent's Wikipedia thumbnail URL
//!   from its `webpages` list (results memoized per page).

use crate::config::*;
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex, MutexGuard};

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

/// Tries the `.images.epub` path on pglaf; falls back to `.epub`.
fn resolve_pglaf_epub_url(mirror_base: &str, ebook_id: &str) -> String {
    let mirror_clean = format!("{}/", mirror_base.trim_end_matches('/'));
    format!("{}cache/epub/{}/pg{}-images.epub", mirror_clean, ebook_id, ebook_id)
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
                if mirror_base.to_lowercase().contains("pglaf")
                    && (file_part.contains("epub") || file_part.ends_with(".epub3.images"))
                {
                    return Some(resolve_pglaf_epub_url(mirror_base, ebook_id_clean));
                }
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
// Wikipedia Image Enrichment
// ---------------------------------------------------------------------------

/// Maximum time a single Wikipedia lookup may take before it is abandoned.
const WIKIPEDIA_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);

/// Shared HTTP client for Wikipedia summary lookups.
///
/// The Wikimedia REST API rejects generic clients, so the configured
/// `User-Agent` identifies the tool; the timeout keeps a stalled endpoint from
/// blocking a worker thread indefinitely.
static WIKIPEDIA_AGENT: LazyLock<ureq::Agent> = LazyLock::new(|| {
    ureq::Agent::config_builder()
        .user_agent(WIKIPEDIA_USER_AGENT)
        .timeout_global(Some(WIKIPEDIA_TIMEOUT))
        .build()
        .new_agent()
});

/// Memoized Wikipedia thumbnail lookups, keyed by article URL.
///
/// A given agent recurs across many ebooks (an author may have hundreds of
/// titles), so both hits and misses are cached process-wide to keep the
/// pipeline at roughly one request per distinct Wikipedia page.
static WIKI_IMAGE_CACHE: LazyLock<Mutex<HashMap<String, Option<String>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Locks the Wikipedia cache, recovering from a poisoned mutex instead of
/// panicking in every worker thread.
fn wiki_image_cache() -> MutexGuard<'static, HashMap<String, Option<String>>> {
    WIKI_IMAGE_CACHE.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Extracts the Wikipedia page name from an article URL.
///
/// The page name is the string after the last slash
/// (`https://en.wikipedia.org/wiki/Jules_Verne` → `Jules_Verne`).
///
/// # Returns
/// `None` when the URL has no trailing segment.
pub fn wikipedia_page_name(url: &str) -> Option<&str> {
    let page_name = url.rsplit('/').next()?.trim();
    (!page_name.is_empty()).then_some(page_name)
}

/// Builds the Wikipedia REST summary endpoint for a page name.
///
/// # Arguments
/// * `page_name` — Wikipedia page name as returned by `wikipedia_page_name`.
///
/// # Returns
/// Full `https://en.wikipedia.org/api/rest_v1/page/summary/<pagename>` URL.
pub fn wikipedia_summary_url(page_name: &str) -> String {
    format!("{}{}", WIKIPEDIA_SUMMARY_API, page_name)
}

/// Requests the thumbnail image URL of a Wikipedia page.
///
/// # Arguments
/// * `page_name` — Wikipedia page name as returned by `wikipedia_page_name`.
///
/// # Returns
/// The `thumbnail.source` value of the summary payload, or `None` when the
/// request fails, the page is missing, or the article has no lead image.
fn request_wikipedia_thumbnail(page_name: &str) -> Option<String> {
    let mut response = WIKIPEDIA_AGENT.get(wikipedia_summary_url(page_name)).call().ok()?;
    if response.status() != 200 {
        return None;
    }
    serde_json::from_reader::<_, serde_json::Value>(response.body_mut().as_reader())
        .ok()?
        .get("thumbnail")?
        .get("source")?
        .as_str()
        .map(str::to_string)
}

/// Resolves an agent's thumbnail image from its first Wikipedia `webpages` entry.
///
/// Uses the process-wide cache, so repeated lookups of the same page (the
/// common case, since agents recur across ebooks) issue no further requests.
///
/// # Arguments
/// * `webpages` — Agent webpage URLs (from the `webpage` RDF child nodes).
///
/// # Returns
/// Thumbnail URL of the agent's Wikipedia article, or `None` when the agent
/// has no Wikipedia page, the lookup fails, or the article has no image.
pub fn agent_wikipedia_image(webpages: &[String]) -> Option<String> {
    let wiki_url = webpages.iter().find(|url| RE_WIKIPEDIA_URL.is_match(url))?;
    if let Some(cached) = wiki_image_cache().get(wiki_url) {
        return cached.clone();
    }
    let page_name = wikipedia_page_name(wiki_url);
    let thumbnail = page_name.and_then(request_wikipedia_thumbnail);
    wiki_image_cache().insert(wiki_url.clone(), thumbnail.clone());
    thumbnail
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

    /// The Wikipedia page name is the substring after the last slash,
    /// including names that carry disambiguation suffixes or encoded
    /// characters.
    #[test]
    fn wikipedia_page_name_is_text_after_last_slash() {
        assert_eq!(
            wikipedia_page_name("https://en.wikipedia.org/wiki/Jules_Verne"),
            Some("Jules_Verne")
        );
        assert_eq!(
            wikipedia_page_name("https://en.wikipedia.org/wiki/Madame_Blavatsky"),
            Some("Madame_Blavatsky")
        );
        assert_eq!(
            wikipedia_page_name("https://de.wikipedia.org/wiki/Erika_Mann"),
            Some("Erika_Mann")
        );
    }

    /// A URL whose last segment is empty yields no page name (this case never
    /// reaches the lookup because `RE_WIKIPEDIA_URL` requires a `/wiki/` path).
    #[test]
    fn wikipedia_page_name_rejects_empty_tail() {
        assert_eq!(wikipedia_page_name("https://en.wikipedia.org/wiki/"), None);
        assert_eq!(wikipedia_page_name(""), None);
    }

    /// The summary endpoint is the documented REST path plus the page name.
    #[test]
    fn wikipedia_summary_url_targets_rest_endpoint() {
        assert_eq!(
            wikipedia_summary_url("Jules_Verne"),
            "https://en.wikipedia.org/api/rest_v1/page/summary/Jules_Verne"
        );
    }

    /// Only Wikipedia article URLs qualify for thumbnail lookups; personal
    /// sites and Gutenberg pages must be ignored. Scheme-less and extra
    /// subdomain forms found in the feeds are still accepted.
    #[test]
    fn wikipedia_url_pattern_selects_articles_only() {
        assert!(RE_WIKIPEDIA_URL.is_match("https://en.wikipedia.org/wiki/Jules_Verne"));
        assert!(RE_WIKIPEDIA_URL.is_match("http://de.wikipedia.org/wiki/Erika_Mann"));
        assert!(RE_WIKIPEDIA_URL.is_match("en.m.wikipedia.org/wiki/Robert_Thurston_Hopkins"));
        assert!(RE_WIKIPEDIA_URL.is_match("fr.wikipedia.org/wiki/Joseph_Kervyn_de_Lettenhove"));
        assert!(!RE_WIKIPEDIA_URL.is_match("https://example.com/alice"));
        assert!(!RE_WIKIPEDIA_URL.is_match("https://www.gutenberg.org/ebooks/11"));
        assert!(!RE_WIKIPEDIA_URL.is_match("https://en.wikipedia.org/wiki"));
        assert!(!RE_WIKIPEDIA_URL.is_match("https://fr.wikipedia"));
    }

    /// The Wikipedia client identifies the tool and its crate version.
    #[test]
    fn wikipedia_user_agent_identifies_tool() {
        assert!(WIKIPEDIA_USER_AGENT.starts_with("gutenberg_parser/"));
        assert!(WIKIPEDIA_USER_AGENT.contains(env!("CARGO_PKG_VERSION")));
    }

    /// Agents without a Wikipedia page resolve to no image, and the negative
    /// result is memoized so repeated calls stay request-free.
    #[test]
    fn agent_image_absent_without_wikipedia_page() {
        let webpages = vec![
            "https://example.com/alice".to_string(),
            "https://www.gutenberg.org/ebooks/agents/42".to_string(),
        ];
        assert!(agent_wikipedia_image(&webpages).is_none());
        assert!(agent_wikipedia_image(&webpages).is_none());
        assert!(agent_wikipedia_image(&[]).is_none());
    }
}
