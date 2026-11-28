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
//! - `agent_wikipedia_enrichment`: Resolves an agent's Wikipedia lead image
//!   and plain-text biographical summary from its `webpages` list, using
//!   language-aware endpoints and process-wide memoization.

use crate::config::*;
use serde::Deserialize;
use std::collections::{HashMap, VecDeque};
use std::sync::{LazyLock, Mutex, MutexGuard};
use std::time::{Duration, Instant};

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
// Wikipedia Enrichment (image + biographical summary)
// ---------------------------------------------------------------------------

/// Lead image URL and plain-text biographical summary of a Wikipedia article.
///
/// Both fields are optional: an article may have only one of them, and both
/// are omitted from serialized agents when missing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct WikipediaEnrichment {
    pub image: Option<String>,
    pub bio: Option<String>,
}

/// Typed subset of the Wikipedia REST summary payload used by the parser.
#[derive(Debug, Default, Deserialize)]
pub(crate) struct WikipediaSummaryResponse {
    /// Plain-text lead section of the article.
    #[serde(default)]
    pub extract: Option<String>,

    /// Page type; disambiguation pages have no usable enrichment.
    #[serde(rename = "type", default)]
    pub page_type: Option<String>,

    /// Lead image metadata of the article.
    #[serde(default)]
    pub thumbnail: Option<WikipediaThumbnail>,
}

/// Lead image metadata nested under `thumbnail` in the summary payload.
#[derive(Debug, Default, Deserialize)]
pub(crate) struct WikipediaThumbnail {
    #[serde(default)]
    pub source: Option<String>,
}

/// Maximum time a single Wikipedia lookup may take before it is abandoned.
const WIKIPEDIA_TIMEOUT: Duration = Duration::from_secs(10);

/// Wikimedia's published limit for identified unauthenticated clients.
const WIKIPEDIA_MAX_PER_MINUTE: usize = 200;

/// Rolling window used by the per-minute request cap.
const WIKIPEDIA_WINDOW: Duration = Duration::from_secs(60);

/// Minimum gap between requests (~3.3 req/s, below the robot guidance of 5).
const WIKIPEDIA_MIN_INTERVAL: Duration = Duration::from_millis(300);

/// Maximum number of redirect hops followed for one summary lookup.
/// Each hop is throttled separately, so it counts toward the rate cap.
const WIKIPEDIA_MAX_REDIRECTS: usize = 3;

/// Upper bound for an announced `Retry-After` delay.
const WIKIPEDIA_MAX_RETRY_AFTER: Duration = Duration::from_secs(60);

/// Fallback delay (before exponential backoff) when no `Retry-After` is sent.
const WIKIPEDIA_RETRY_BASE_SECS: u64 = 5;

/// Timestamps of recent Wikipedia requests, for the rolling 60-second window.
static WIKIPEDIA_REQUEST_TIMES: LazyLock<Mutex<VecDeque<Instant>>> = LazyLock::new(|| Mutex::new(VecDeque::new()));

/// Serializes Wikipedia lookups across worker threads.
///
/// The guard is held for the entire lookup (including rate-limit sleeps and
/// retries), so concurrency stays at one request even though the parser has
/// many workers. The process-wide cache means this does not materially hurt
/// throughput.
static WIKIPEDIA_REQUEST_GATE: Mutex<()> = Mutex::new(());

/// Shared HTTP client for Wikipedia summary lookups.
///
/// The Wikimedia REST API rejects generic clients, so the configured
/// `User-Agent` identifies the tool; the timeout keeps a stalled endpoint from
/// blocking a worker thread indefinitely.
static WIKIPEDIA_AGENT: LazyLock<ureq::Agent> = LazyLock::new(|| {
    ureq::Agent::config_builder()
        .user_agent(WIKIPEDIA_USER_AGENT)
        .timeout_global(Some(WIKIPEDIA_TIMEOUT))
        .max_redirects(0)
        .build()
        .new_agent()
});

/// Memoized Wikipedia enrichment, keyed by article URL.
///
/// A given agent recurs across many ebooks (an author may have hundreds of
/// titles), so both hits and misses are cached process-wide to keep the
/// pipeline at roughly one request per distinct Wikipedia page.
static WIKIPEDIA_CACHE: LazyLock<Mutex<HashMap<String, Option<WikipediaEnrichment>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Locks the Wikipedia cache, recovering from a poisoned mutex instead of
/// panicking in every worker thread.
fn wikipedia_cache() -> MutexGuard<'static, HashMap<String, Option<WikipediaEnrichment>>> {
    WIKIPEDIA_CACHE.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Locks the Wikipedia request gate, recovering from a poisoned mutex.
fn wikipedia_request_gate() -> MutexGuard<'static, ()> {
    WIKIPEDIA_REQUEST_GATE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Returns the decoded bytes as a string, preserving the original text when
/// the decoded bytes are not valid UTF-8.
fn percent_decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' && index + 2 < bytes.len() {
            if let (Some(high), Some(low)) = (hex_digit(bytes[index + 1]), hex_digit(bytes[index + 2])) {
                decoded.push((high << 4) | low);
                index += 3;
                continue;
            }
        }
        decoded.push(bytes[index]);
        index += 1;
    }
    String::from_utf8(decoded).unwrap_or_else(|_| value.to_string())
}

fn hex_digit(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

/// Percent-encodes everything outside the RFC 3986 unreserved set.
fn percent_encode_title(title: &str) -> String {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let mut encoded = String::with_capacity(title.len());
    for byte in title.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => encoded.push(byte as char),
            _ => {
                encoded.push('%');
                encoded.push(HEX[(byte >> 4) as usize] as char);
                encoded.push(HEX[(byte & 0x0f) as usize] as char);
            }
        }
    }
    encoded
}

fn is_language_label(label: &str) -> bool {
    label.bytes().any(|b| b.is_ascii_alphanumeric()) && label.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
}

/// Extracts the lowercase language and canonical encoded title from a
/// Wikipedia article URL.
///
/// Scheme-less URLs and `www`, `m`, and `mobile` host labels are accepted;
/// articles without a recognizable language subdomain default to English.
/// Already percent-encoded titles are decoded first so they are not encoded
/// twice.
pub fn wikipedia_page(url: &str) -> Option<(String, String)> {
    let lowercase = url.to_ascii_lowercase();
    let after_scheme = if lowercase.starts_with("https://") {
        &url[8..]
    } else if lowercase.starts_with("http://") {
        &url[7..]
    } else if lowercase.starts_with("//") {
        &url[2..]
    } else {
        url
    };
    let lowercase_after_scheme = after_scheme.to_ascii_lowercase();
    let host_end = lowercase_after_scheme
        .find(['/', '?', '#'])
        .unwrap_or(lowercase_after_scheme.len());
    let host = lowercase_after_scheme[..host_end].split(':').next().unwrap_or("");
    let labels: Vec<&str> = host.split('.').filter(|label| !label.is_empty()).collect();
    if labels.len() < 2 || labels[labels.len() - 2] != "wikipedia" || labels[labels.len() - 1] != "org" {
        return None;
    }

    let mut candidates = labels[..labels.len() - 2]
        .iter()
        .copied()
        .filter(|label| !matches!(*label, "www" | "m" | "mobile"))
        .filter(|label| is_language_label(label));
    let language = match (candidates.next(), candidates.next()) {
        (Some(label), None) => label.to_string(),
        _ => "en".to_string(),
    };

    let remainder = &lowercase_after_scheme[host_end..];
    let path_end = host_end + remainder.find(['?', '#']).unwrap_or(remainder.len());
    let path = &lowercase_after_scheme[host_end..path_end];
    let wiki_index = host_end + path.find("/wiki/")?;
    let raw_title = &after_scheme[wiki_index + "/wiki/".len()..];
    let raw_title = &raw_title[..raw_title.find(['?', '#']).unwrap_or(raw_title.len())];
    if raw_title.is_empty() {
        return None;
    }

    let decoded = percent_decode(raw_title).replace(' ', "_");
    let title = percent_encode_title(&decoded);
    (!title.is_empty()).then_some((language, title))
}

/// Builds the language-specific Wikipedia REST summary endpoint.
pub fn wikipedia_summary_url(language: &str, title: &str) -> String {
    let endpoint = WIKIPEDIA_SUMMARY_API.replace("{language}", language);
    format!("{endpoint}{title}")
}

/// True when a URL's host is a `wikipedia.org` domain.
fn is_wikipedia_host(url: &str) -> bool {
    let lowercase = url.to_ascii_lowercase();
    let without_scheme = if let Some((_, rest)) = lowercase.split_once("://") {
        rest
    } else {
        lowercase.as_str()
    };
    let host = without_scheme.split(['/', '?', '#']).next().unwrap_or("");
    let host = host.split(':').next().unwrap_or("");
    let labels: Vec<&str> = host.split('.').filter(|label| !label.is_empty()).collect();
    labels.len() >= 2 && labels[labels.len() - 2] == "wikipedia" && labels[labels.len() - 1] == "org"
}

/// Resolves a `Location` header from a summary redirect.
///
/// Only Wikipedia hosts are followed; relative and protocol-relative
/// locations are resolved against the article's language subdomain.
fn wikipedia_redirect_url(language: &str, location: &str) -> Option<String> {
    if location.starts_with("//") {
        let absolute = format!("https:{location}");
        return is_wikipedia_host(&absolute).then_some(absolute);
    }
    if location.starts_with('/') {
        return Some(format!("https://{language}.wikipedia.org{location}"));
    }
    if location.starts_with("https://") || location.starts_with("http://") {
        return is_wikipedia_host(location).then(|| location.to_string());
    }
    None
}

/// Extracts usable fields from a typed Wikipedia summary payload.
///
/// Disambiguation pages are skipped entirely. Missing or blank `extract` and
/// `thumbnail.source` values simply leave the corresponding field unset; long
/// extracts are preserved in full.
pub(crate) fn parse_wikipedia_summary(response: WikipediaSummaryResponse) -> Option<WikipediaEnrichment> {
    if response
        .page_type
        .as_deref()
        .is_some_and(|kind| kind.eq_ignore_ascii_case("disambiguation"))
    {
        return None;
    }

    Some(WikipediaEnrichment {
        image: non_empty_trimmed(response.thumbnail.and_then(|thumbnail| thumbnail.source)),
        bio: non_empty_trimmed(response.extract),
    })
}

fn non_empty_trimmed(value: Option<String>) -> Option<String> {
    let value = value?;
    let trimmed = value.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

/// Returns delay for a 429/503 response, honoring integer `Retry-After`
/// values and falling back to exponential backoff for dates or missing headers.
fn wikipedia_retry_delay(retry_after: Option<&str>, attempt: usize) -> Duration {
    let fallback_secs = WIKIPEDIA_RETRY_BASE_SECS.saturating_mul(1u64 << attempt.min(4));
    retry_after
        .and_then(|value| value.trim().parse::<u64>().ok())
        .map(Duration::from_secs)
        .unwrap_or_else(|| Duration::from_secs(fallback_secs))
        .min(WIKIPEDIA_MAX_RETRY_AFTER)
}

fn wikipedia_transport_backoff(attempt: usize) -> Duration {
    Duration::from_millis(500).saturating_mul(1u32 << attempt.min(4))
}

/// Returns how long the next request must wait, or `None` when it may proceed.
///
/// The rolling cap takes precedence over the minimum gap once the window is
/// full; expired entries are removed by the caller before this is called.
fn wikipedia_request_wait(recent: &VecDeque<Instant>, now: Instant) -> Option<Duration> {
    if recent.len() >= WIKIPEDIA_MAX_PER_MINUTE {
        recent
            .front()
            .map(|oldest| WIKIPEDIA_WINDOW.saturating_sub(now.duration_since(*oldest)))
    } else {
        recent
            .back()
            .map(|last| WIKIPEDIA_MIN_INTERVAL.saturating_sub(now.duration_since(*last)))
            .filter(|wait| !wait.is_zero())
    }
}

/// Sleeps until a Wikipedia request is allowed under both the minimum spacing
/// and the rolling 200-requests-per-minute cap.
///
/// ureq is configured not to follow redirects automatically; every redirect
/// hop is issued explicitly through this function, so one throttled call is
/// exactly one HTTP request against the published Wikimedia limit.
fn throttle_wikipedia_request() {
    loop {
        let mut recent = WIKIPEDIA_REQUEST_TIMES
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let now = Instant::now();
        recent.retain(|time| now.duration_since(*time) < WIKIPEDIA_WINDOW);

        match wikipedia_request_wait(&recent, now) {
            Some(wait) => {
                drop(recent);
                std::thread::sleep(wait);
            }
            None => {
                recent.push_back(Instant::now());
                return;
            }
        }
    }
}

/// Deserializes a summary response body; malformed payloads are treated as a
/// failed lookup, which is cached as a negative result by the caller.
fn parse_wikipedia_summary_body(body: impl std::io::Read) -> Option<WikipediaEnrichment> {
    let value: serde_json::Value = serde_json::from_reader(body).ok()?;
    if !value.is_object() {
        return None;
    }
    let payload: WikipediaSummaryResponse = serde_json::from_value(value).ok()?;
    parse_wikipedia_summary(payload)
}

/// Requests and parses the Wikipedia summary for an article.
///
/// The caller holds `WIKIPEDIA_REQUEST_GATE`, keeping retries, redirects and
/// cache insertion atomic across workers. Redirects are followed manually (up
/// to `WIKIPEDIA_MAX_REDIRECTS` hops) so every hop is throttled and counted
/// against the published rate cap. Transport errors and 429/503 responses get
/// at most one retry.
fn request_wikipedia_summary(language: &str, title: &str) -> Option<WikipediaEnrichment> {
    let mut url = wikipedia_summary_url(language, title);
    let mut redirects = 0;
    let mut attempt = 0;

    loop {
        throttle_wikipedia_request();
        let mut response = match WIKIPEDIA_AGENT.get(url.as_str()).call() {
            Ok(response) => response,
            Err(_) => {
                if attempt == 0 {
                    std::thread::sleep(wikipedia_transport_backoff(attempt));
                    attempt += 1;
                    continue;
                }
                return None;
            }
        };

        match response.status().as_u16() {
            200 => return parse_wikipedia_summary_body(response.body_mut().as_reader()),
            301 | 302 | 303 | 307 | 308 if redirects < WIKIPEDIA_MAX_REDIRECTS => {
                let location = response
                    .headers()
                    .get("location")
                    .and_then(|value| value.to_str().ok())?;
                url = wikipedia_redirect_url(language, location)?;
                redirects += 1;
            }
            429 | 503 if attempt == 0 => {
                let retry_after = response
                    .headers()
                    .get("retry-after")
                    .and_then(|value| value.to_str().ok());
                std::thread::sleep(wikipedia_retry_delay(retry_after, attempt));
                attempt += 1;
            }
            _ => return None,
        }
    }
}

/// Resolves an agent's Wikipedia enrichment from its first Wikipedia
/// `webpages` entry (choosing the first match is intentional).
///
/// Uses the process-wide cache, so repeated lookups of the same page (the
/// common case, since agents recur across ebooks) issue no further requests.
/// The request gate is re-checked under the lock to prevent duplicate lookups
/// from concurrent workers.
pub fn agent_wikipedia_enrichment(webpages: &[String]) -> Option<WikipediaEnrichment> {
    let wiki_url = webpages.iter().find(|url| RE_WIKIPEDIA_URL.is_match(url))?;
    if let Some(cached) = wikipedia_cache().get(wiki_url) {
        return cached.clone();
    }

    let _gate = wikipedia_request_gate();
    if let Some(cached) = wikipedia_cache().get(wiki_url) {
        return cached.clone();
    }

    let enrichment =
        wikipedia_page(wiki_url).and_then(|(language, title)| request_wikipedia_summary(&language, &title));
    wikipedia_cache().insert(wiki_url.clone(), enrichment.clone());
    enrichment
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

    /// Article URLs yield their language subdomain and encoded page title.
    #[test]
    fn wikipedia_page_extracts_language_and_title() {
        assert_eq!(
            wikipedia_page("https://en.wikipedia.org/wiki/Jules_Verne"),
            Some(("en".to_string(), "Jules_Verne".to_string()))
        );
        assert_eq!(
            wikipedia_page("https://de.wikipedia.org/wiki/Erika_Mann"),
            Some(("de".to_string(), "Erika_Mann".to_string()))
        );
        assert_eq!(
            wikipedia_page("fr.wikipedia.org/wiki/Joseph_Kervyn_de_Lettenhove"),
            Some(("fr".to_string(), "Joseph_Kervyn_de_Lettenhove".to_string()))
        );
    }

    /// Mobile / `www` host labels are ignored; unknown languages default to
    /// English, while multi-part language codes are preserved.
    #[test]
    fn wikipedia_page_normalizes_host_labels() {
        assert_eq!(
            wikipedia_page("en.m.wikipedia.org/wiki/Robert_Thurston_Hopkins"),
            Some(("en".to_string(), "Robert_Thurston_Hopkins".to_string()))
        );
        assert_eq!(
            wikipedia_page("https://www.wikipedia.org/wiki/Potato"),
            Some(("en".to_string(), "Potato".to_string()))
        );
        assert_eq!(
            wikipedia_page("https://mobile.fr.wikipedia.org/wiki/Jules_Verne").map(|(language, _)| language),
            Some("fr".to_string())
        );
        assert_eq!(
            wikipedia_page("https://zh-min-nan.wikipedia.org/wiki/Bun-hak").map(|(language, _)| language),
            Some("zh-min-nan".to_string())
        );
    }

    /// Titles are decoded before encoding, so existing escapes are not encoded
    /// twice, and query strings / fragments are dropped.
    #[test]
    fn wikipedia_page_decodes_and_encodes_title() {
        assert_eq!(
            wikipedia_page("https://fr.wikipedia.org/wiki/%C3%89mile_Zola?action=raw"),
            Some(("fr".to_string(), "%C3%89mile_Zola".to_string()))
        );
        assert_eq!(
            wikipedia_page("https://en.wikipedia.org/wiki/Hello%20World#History"),
            Some(("en".to_string(), "Hello_World".to_string()))
        );
        assert_eq!(
            wikipedia_page("https://en.wikipedia.org/wiki/Café_(film)"),
            Some(("en".to_string(), "Caf%C3%A9_%28film%29".to_string()))
        );
        assert_eq!(
            wikipedia_page("https://en.wikipedia.org/wiki/A%2FB"),
            Some(("en".to_string(), "A%2FB".to_string()))
        );
        assert_eq!(
            wikipedia_page("https://en.wikipedia.org/wiki/100%Good"),
            Some(("en".to_string(), "100%25Good".to_string()))
        );
    }

    /// Non-Wikipedia hosts and empty titles are rejected.
    #[test]
    fn wikipedia_page_rejects_non_articles() {
        assert_eq!(wikipedia_page("https://example.com/wiki/Jules_Verne"), None);
        assert_eq!(wikipedia_page("https://en.wikipedia.org/wiki/"), None);
        assert_eq!(wikipedia_page("https://en.wikipedia.org/wiki/?printable=yes"), None);
        assert_eq!(wikipedia_page("https://en.wikipedia.org/?next=/wiki/Jules_Verne"), None);
        assert_eq!(wikipedia_page(""), None);
    }

    /// The summary endpoint uses the article's language subdomain.
    #[test]
    fn wikipedia_summary_url_targets_language_rest_endpoint() {
        assert_eq!(
            wikipedia_summary_url("en", "Jules_Verne"),
            "https://en.wikipedia.org/api/rest_v1/page/summary/Jules_Verne"
        );
        assert_eq!(
            wikipedia_summary_url("fr", "Erika_Mann"),
            "https://fr.wikipedia.org/api/rest_v1/page/summary/Erika_Mann"
        );
        assert_eq!(
            wikipedia_summary_url("de", "Caf%C3%A9"),
            "https://de.wikipedia.org/api/rest_v1/page/summary/Caf%C3%A9"
        );
    }

    /// Only Wikipedia article URLs qualify for enrichment; personal sites and
    /// Gutenberg pages must be ignored. Scheme-less and extra subdomain forms
    /// found in the feeds are still accepted.
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

    /// Redirects are handled explicitly so every hop can be throttled.
    #[test]
    fn wikipedia_agent_does_not_follow_redirects_automatically() {
        assert_eq!(WIKIPEDIA_AGENT.config().max_redirects(), 0);
    }

    /// Redirect targets are restricted to Wikipedia hosts; relative and
    /// protocol-relative locations are resolved against the article language.
    #[test]
    fn wikipedia_redirect_locations_are_resolved_safely() {
        assert_eq!(
            wikipedia_redirect_url("fr", "/api/rest_v1/page/summary/Zola"),
            Some("https://fr.wikipedia.org/api/rest_v1/page/summary/Zola".to_string())
        );
        assert_eq!(
            wikipedia_redirect_url("en", "//de.wikipedia.org/wiki/Erika_Mann"),
            Some("https://de.wikipedia.org/wiki/Erika_Mann".to_string())
        );
        assert_eq!(
            wikipedia_redirect_url("en", "https://en.wikipedia.org/wiki/Jules_Verne"),
            Some("https://en.wikipedia.org/wiki/Jules_Verne".to_string())
        );
        assert_eq!(wikipedia_redirect_url("en", "https://example.com/evil"), None);
        assert_eq!(
            wikipedia_redirect_url("en", "/wiki/Local_only"),
            Some("https://en.wikipedia.org/wiki/Local_only".to_string())
        );
        assert_eq!(wikipedia_redirect_url("en", "not a url"), None);
    }

    fn summary(value: serde_json::Value) -> WikipediaSummaryResponse {
        serde_json::from_value(value).unwrap()
    }

    /// A standard summary supplies both the lead image and the plain-text bio.
    #[test]
    fn wikipedia_summary_yields_image_and_bio() {
        let enrichment = parse_wikipedia_summary(summary(serde_json::json!({
            "type": "standard",
            "extract": "  Jules Verne was a French novelist.  ",
            "thumbnail": { "source": "https://thumb.wikimedia.org/verne.jpg" }
        })))
        .unwrap();
        assert_eq!(enrichment.bio.as_deref(), Some("Jules Verne was a French novelist."));
        assert_eq!(
            enrichment.image.as_deref(),
            Some("https://thumb.wikimedia.org/verne.jpg")
        );
    }

    /// Disambiguation pages are skipped entirely.
    #[test]
    fn wikipedia_summary_skips_disambiguation_pages() {
        let response = summary(serde_json::json!({
            "type": "Disambiguation",
            "extract": "Several people share this name.",
            "thumbnail": { "source": "https://thumb.wikimedia.org/name.jpg" }
        }));
        assert!(parse_wikipedia_summary(response).is_none());
    }

    /// Missing extract and thumbnail do not invalidate each other.
    #[test]
    fn wikipedia_summary_keeps_available_field_independently() {
        let bio_only = parse_wikipedia_summary(summary(serde_json::json!({ "extract": "A short bio." }))).unwrap();
        assert_eq!(bio_only.bio.as_deref(), Some("A short bio."));
        assert!(bio_only.image.is_none());

        let image_only = parse_wikipedia_summary(summary(serde_json::json!({
            "thumbnail": { "source": "https://thumb.wikimedia.org/portrait.jpg" }
        })))
        .unwrap();
        assert!(image_only.bio.is_none());
        assert_eq!(
            image_only.image.as_deref(),
            Some("https://thumb.wikimedia.org/portrait.jpg")
        );
    }

    /// Blank or whitespace-only values become absent; long extracts are kept
    /// verbatim apart from surrounding whitespace.
    #[test]
    fn wikipedia_summary_ignores_blanks_and_preserves_long_extract() {
        let blank = parse_wikipedia_summary(summary(serde_json::json!({
            "extract": "   ",
            "thumbnail": { "source": "" }
        })))
        .unwrap();
        assert!(blank.image.is_none());
        assert!(blank.bio.is_none());

        let whitespace = parse_wikipedia_summary(summary(serde_json::json!({
            "extract": "\tSpaced bio.\n",
            "thumbnail": { "source": "  \n " }
        })))
        .unwrap();
        assert!(whitespace.image.is_none());
        assert_eq!(whitespace.bio.as_deref(), Some("Spaced bio."));

        let long = "A".repeat(5000);
        let parsed = parse_wikipedia_summary(summary(serde_json::json!({ "extract": long.clone() }))).unwrap();
        assert_eq!(parsed.bio.as_deref(), Some(long.as_str()));
    }

    /// Invalid JSON and wrong field types fail the lookup instead of panicking
    /// or leaking partial fields.
    #[test]
    fn wikipedia_summary_body_rejects_malformed_json() {
        assert!(parse_wikipedia_summary_body(&b"not json"[..]).is_none());
        assert!(parse_wikipedia_summary_body(&b"{\"extract\": 42}"[..]).is_none());
        assert!(parse_wikipedia_summary_body(&b"{\"thumbnail\": {\"source\": 5}}"[..]).is_none());
        assert!(parse_wikipedia_summary_body(&b"[]"[..]).is_none());

        let parsed = parse_wikipedia_summary_body(
            &b"{\"extract\":\" Short bio. \",\"thumbnail\":{\"source\":\"https://img.test/a.jpg\"}}"[..],
        )
        .unwrap();
        assert_eq!(parsed.bio.as_deref(), Some("Short bio."));
        assert_eq!(parsed.image.as_deref(), Some("https://img.test/a.jpg"));
    }

    /// Rate-limit delays honor integer headers and fall back to bounded
    /// exponential backoff for dates or missing headers.
    #[test]
    fn wikipedia_retry_delays_are_bounded() {
        assert_eq!(wikipedia_retry_delay(Some("2"), 0), Duration::from_secs(2));
        assert_eq!(wikipedia_retry_delay(Some(" 7 "), 0), Duration::from_secs(7));
        assert_eq!(wikipedia_retry_delay(Some("0"), 0), Duration::ZERO);
        assert_eq!(wikipedia_retry_delay(Some("120"), 0), Duration::from_secs(60));
        let huge = u64::MAX.to_string();
        assert_eq!(wikipedia_retry_delay(Some(&huge), 0), Duration::from_secs(60));
        assert_eq!(
            wikipedia_retry_delay(Some("Wed, 21 Oct 2015 07:28:00 GMT"), 0),
            Duration::from_secs(5)
        );
        assert_eq!(wikipedia_retry_delay(None, 0), Duration::from_secs(5));
        assert_eq!(wikipedia_retry_delay(None, 1), Duration::from_secs(10));
        assert_eq!(wikipedia_retry_delay(None, 12), Duration::from_secs(60));
        assert_eq!(wikipedia_transport_backoff(0), Duration::from_millis(500));
        assert_eq!(wikipedia_transport_backoff(1), Duration::from_secs(1));
        assert_eq!(wikipedia_transport_backoff(12), Duration::from_secs(8));
    }

    /// The limiter keeps the minimum gap below the cap and waits for the
    /// oldest entry once a full rolling window has elapsed.
    #[test]
    fn wikipedia_request_wait_respects_gap_and_rolling_cap() {
        let now = Instant::now();

        let mut recent = VecDeque::new();
        assert_eq!(wikipedia_request_wait(&recent, now), None);

        recent.push_back(now - Duration::from_millis(100));
        assert_eq!(wikipedia_request_wait(&recent, now), Some(Duration::from_millis(200)));

        recent[0] = now - WIKIPEDIA_MIN_INTERVAL;
        assert_eq!(wikipedia_request_wait(&recent, now), None);

        let mut below_cap: VecDeque<_> = (0..WIKIPEDIA_MAX_PER_MINUTE - 2)
            .map(|_| now - Duration::from_secs(5))
            .collect();
        below_cap.push_back(now - Duration::from_millis(100));
        assert_eq!(
            wikipedia_request_wait(&below_cap, now),
            Some(Duration::from_millis(200))
        );

        let mut full_window: VecDeque<_> = (0..WIKIPEDIA_MAX_PER_MINUTE - 1)
            .map(|_| now - Duration::from_secs(5))
            .collect();
        full_window.push_front(now - Duration::from_secs(59));
        assert_eq!(wikipedia_request_wait(&full_window, now), Some(Duration::from_secs(1)));
    }

    /// Agents without a Wikipedia page resolve to no enrichment.
    #[test]
    fn agent_enrichment_absent_without_wikipedia_page() {
        let webpages = vec![
            "https://example.com/alice".to_string(),
            "https://www.gutenberg.org/ebooks/agents/42".to_string(),
        ];
        assert!(agent_wikipedia_enrichment(&webpages).is_none());
        assert!(agent_wikipedia_enrichment(&webpages).is_none());
        assert!(agent_wikipedia_enrichment(&[]).is_none());
    }

    /// A seeded negative result is returned from the cache without a request.
    #[test]
    fn negative_wikipedia_result_is_cached() {
        let wiki_url = "https://en.wikipedia.org/wiki/No_Such_Agent_Page_For_Tests".to_string();
        wikipedia_cache().insert(wiki_url.clone(), None);
        let webpages = vec![wiki_url.clone()];
        assert!(agent_wikipedia_enrichment(&webpages).is_none());
        assert_eq!(wikipedia_cache().get(&wiki_url), Some(&None));
    }

    /// Only the first matching Wikipedia URL is used; later matches are
    /// ignored even when they also have cached enrichment.
    #[test]
    fn first_wikipedia_url_is_used() {
        let first = "https://en.wikipedia.org/wiki/First_Url_For_Tests".to_string();
        let second = "https://fr.wikipedia.org/wiki/Second_Url_For_Tests".to_string();
        wikipedia_cache().insert(
            first.clone(),
            Some(WikipediaEnrichment {
                image: Some("https://img.test/first.jpg".to_string()),
                bio: Some("First article".to_string()),
            }),
        );
        wikipedia_cache().insert(
            second.clone(),
            Some(WikipediaEnrichment {
                image: Some("https://img.test/second.jpg".to_string()),
                bio: Some("Second article".to_string()),
            }),
        );

        let webpages = vec!["https://example.com/profile".to_string(), first, second];
        let enrichment = agent_wikipedia_enrichment(&webpages).unwrap();
        assert_eq!(enrichment.image.as_deref(), Some("https://img.test/first.jpg"));
        assert_eq!(enrichment.bio.as_deref(), Some("First article"));
    }
}
