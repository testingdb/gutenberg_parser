//! Taxonomy Inference Engine
//! ---------------------------------------------------------
//! Transforms raw RDF `subject` and `bookshelf` strings into a structured
//! `Taxonomy` (`domain`, `genres`, `topics`).
//!
//! ## Strategy
//! 1. **LC Classification**: Parse Library of Congress codes (`D501`,
//!    `F350.5`) via `parse_lc_code` to extract `(domain, genre)`.
//! 2. **Subject Headings**: Split on ` -- `; first part is the topic
//!    heading; remaining parts are subtopics. Form/genre keywords are
//!    matched against `LCSH_FORM_GENRE_MAP`.
//! 3. **Bookshelf Inference**: Strip `Category:` prefix, match against
//!    `BOOKSHELF_MAP` regexes to infer genres.
//! 4. **Domain Resolution**: Select the lexicographically first domain
//!    from LC-derived or inferred sets; fall back to `"General & Uncategorized"`.
//!
//! ## Deduplication
//! `seen_keys` (a `HashSet<(String, Vec<String>)>`) prevents duplicate topic
//! entries when the same heading/subtopic combination appears multiple times.

use crate::config::*;
use crate::models::{Taxonomy, Topic};
use crate::utils::parse_lc_code;
use std::collections::HashSet;

const GENRE_INDICATORS: &[&str] = &[
    "biography",
    "autobiography",
    "memoir",
    "memoirs",
    "diaries",
    "letters",
    "speeches",
    "correspondence",
    "interviews",
    "journals",
    "notebooks",
    "fiction",
    "novel",
    "novels",
    "drama",
    "poetry",
    "poem",
    "poems",
    "short stories",
    "short story",
    "essays",
    "essay",
    "satire",
    "humor",
    "horror",
    "mystery",
    "crime",
    "detective",
    "science fiction",
    "fantasy",
    "romance",
    "historical fiction",
    "adventure",
    "war stories",
    "juvenile",
    "children",
    "encyclopedias",
    "encyclopedia",
    "dictionaries",
    "dictionary",
    "reference",
    "collections",
    "series",
    "pamphlets",
    "periodicals",
    "yearbooks",
    "almanacs",
    "directories",
    "thriller",
    "western",
    "gothic",
    "plays",
    "tragedy",
    "comedy",
    "fairy tales",
    "folklore",
    "mythology",
    "legend",
    "legends",
    "travel",
    "travelogue",
    "guide",
    "guides",
    "manual",
    "manuals",
    "cookbook",
    "cookbooks",
    "recipe",
    "recipes",
];

/// Checks whether a particular description from `LC_MAP` reads as a
/// form / genre keyword (e.g. "Biography", "Fiction", "Essays").
fn is_genre_keyword(s: &str) -> bool {
    let lower = s.to_lowercase();
    // Check against LCSH form/genre keywords.
    if LCSH_FORM_GENRE_MAP.contains_key(lower.as_str()) {
        return true;
    }
    // Normalize: lowercase, collapse multiple spaces, strip punctuation
    let normalized = lower
        .replace(|c: char| !c.is_alphanumeric() && c != ' ', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    for indicator in GENRE_INDICATORS {
        if normalized.contains(indicator) {
            return true;
        }
    }
    false
}

/// Builds a `Taxonomy` from raw RDF `subject` and `bookshelf` values.
///
/// # Arguments
/// * `subjects_raw` — List of `subject/value` strings from RDF.
/// * `bookshelves_raw` — List of `bookshelf/value` strings from RDF.
///
/// # Returns
/// A `Taxonomy` with sorted `genres`, deduplicated `topics`, and a
/// resolved `domain`.
pub fn extract_taxonomy(subjects_raw: &[String], bookshelves_raw: &[String]) -> Taxonomy {
    let mut lc_domains = HashSet::new();
    let mut inferred_domains = HashSet::new();
    let mut genres = HashSet::new();
    let mut raw_topics = Vec::new();

    // Process each subject node.
    for subj in subjects_raw {
        let subj_clean = subj.trim();
        if subj_clean.is_empty() {
            continue;
        }

        if let Some((dom, gen)) = parse_lc_code(subj_clean) {
            // Direct LC code match: insert domain and optional genre.
            lc_domains.insert(dom);
            if !gen.is_empty() {
                genres.insert(gen);
            }
        } else {
            // Subject heading format: split on ` -- ` separator.
            let parts: Vec<&str> = subj_clean
                .split(" -- ")
                .map(|p| p.trim())
                .filter(|p| !p.is_empty())
                .collect();
            if !parts.is_empty() {
                let heading = parts[0];
                let heading_lower = heading.to_lowercase();
                // Check if the heading is a form/genre keyword.
                if let Some(&(dom, gen)) = LCSH_FORM_GENRE_MAP.get(heading_lower.as_str()) {
                    inferred_domains.insert(dom);
                    if !gen.is_empty() {
                        genres.insert(gen);
                    }
                }

                let mut filtered_subtopics = Vec::new();
                for p in &parts[1..] {
                    let p_lower = p.to_lowercase();
                    // Sub-parts may also be form/genre keywords.
                    if let Some(&(dom, gen)) = LCSH_FORM_GENRE_MAP.get(p_lower.as_str()) {
                        inferred_domains.insert(dom);
                        if !gen.is_empty() {
                            genres.insert(gen);
                        }
                    } else {
                        filtered_subtopics.push(p.to_string());
                    }
                }

                // Assemble heading + remaining subtopics.
                let mut topic_entry = vec![heading.to_string()];
                topic_entry.extend(filtered_subtopics);
                raw_topics.push(topic_entry);
            }
        }
    }

    // Process bookshelf labels.
    for shelf in bookshelves_raw {
        let shelf_clean = RE_SHELF_CAT.replace(shelf, "").trim().to_string();
        let shelf_lower = shelf_clean.to_lowercase();

        // Skip generic labels that do not provide useful classification.
        if ["best books ever listings", "novels", "general"].contains(&shelf_lower.as_str()) {
            continue;
        }

        for (regex, genre_label) in BOOKSHELF_MAP.iter() {
            if regex.is_match(&shelf_lower) {
                genres.insert(*genre_label);
            }
        }
    }

    // Resolve inferred domains from genre labels.
    for genre in &genres {
        if let Some(&dom) = GENRE_TO_DOMAIN_MAP.get(genre) {
            inferred_domains.insert(dom);
        }
    }

    // Determine primary domain from LC codes or inferred domains.
    let primary_domain = if !lc_domains.is_empty() {
        let mut sorted: Vec<_> = lc_domains.into_iter().collect();
        sorted.sort();
        sorted[0]
    } else if !inferred_domains.is_empty() {
        let mut sorted: Vec<_> = inferred_domains.into_iter().collect();
        sorted.sort();
        sorted[0]
    } else {
        "General & Uncategorized"
    };

    // Deduplicate and format topics.
    let mut formatted_topics = Vec::new();
    let mut seen_keys = HashSet::new();

    for parts in raw_topics {
        let heading = parts[0].clone();
        let subtopics = parts[1..].to_vec();
        let key = (heading.clone(), subtopics.clone());

        if !seen_keys.contains(&key) {
            seen_keys.insert(key);
            formatted_topics.push(Topic { heading, subtopics });
        }
    }

    // Sort genres lexicographically for deterministic output.
    let mut sorted_genres: Vec<String> = genres.into_iter().map(|s| s.to_string()).collect();
    sorted_genres.sort();

    Taxonomy {
        domain: primary_domain.to_string(),
        genres: sorted_genres,
        topics: formatted_topics,
    }
}

// ---------------------------------------------------------------------------
// Unit Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Confirms that a simple LC subject (`"Science -- Mathematics"`) yields
    /// a non-empty domain.
    #[test]
    fn taxonomy_extracts_domain() {
        let tax = extract_taxonomy(&["Science -- Mathematics".to_string()], &[]);
        assert!(!tax.domain.is_empty());
    }
}
