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
            // Broad domain from LC classification.
            lc_domains.insert(dom);
            if !gen.is_empty() {
                if is_genre_keyword(gen) {
                    genres.insert(gen);
                } else {
                    // Particular subject/time/place info preserved as topic.
                    raw_topics.push(vec![dom.to_string(), gen.to_string()]);
                }
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

    // ------------------------------------------------------------------
    // is_genre_keyword (private helper)
    // ------------------------------------------------------------------

    #[test]
    fn genre_keyword_lcsh_map_hit() {
        assert!(is_genre_keyword("fiction"));
        assert!(is_genre_keyword("biography"));
        assert!(is_genre_keyword("autobiography"));
        assert!(is_genre_keyword("drama"));
        assert!(is_genre_keyword("poetry"));
    }

    #[test]
    fn genre_keyword_indicator_array_hit() {
        assert!(is_genre_keyword("Biography"));
        assert!(is_genre_keyword("Fiction"));
        assert!(is_genre_keyword("Short stories"));
        assert!(is_genre_keyword("Memoirs"));
        assert!(is_genre_keyword("Essays"));
        assert!(is_genre_keyword("Science fiction"));
        assert!(is_genre_keyword("Fantasy"));
        assert!(is_genre_keyword("Horror"));
    }

    #[test]
    fn genre_keyword_normalized_punctuation() {
        assert!(is_genre_keyword("fiction."));
        assert!(is_genre_keyword("memoirs,"));
        assert!(is_genre_keyword("biography - essays"));
    }

    #[test]
    fn genre_keyword_false_for_non_genre() {
        assert!(!is_genre_keyword("World War I"));
        assert!(!is_genre_keyword("Mathematics"));
        assert!(!is_genre_keyword("Geography"));
        assert!(!is_genre_keyword(""));
    }

    // ------------------------------------------------------------------
    // extract_taxonomy — core behavior
    // ------------------------------------------------------------------

    #[test]
    fn taxonomy_extracts_domain() {
        let tax = extract_taxonomy(&["Science -- Mathematics".to_string()], &[]);
        assert!(!tax.domain.is_empty());
    }

    #[test]
    fn empty_subjects_and_bookshelves_fallback_domain() {
        let tax = extract_taxonomy(&[], &[]);
        assert_eq!(tax.domain, "General & Uncategorized");
        assert!(tax.genres.is_empty());
        assert!(tax.topics.is_empty());
    }

    #[test]
    fn lc_code_with_genre_keyword() {
        // CT = History, "Biography" -> matches genre indicator, inserted as genre
        let tax = extract_taxonomy(&["CT".to_string()], &[]);
        assert_eq!(tax.domain, "History");
        assert!(tax.genres.contains(&"Biography".to_string()));
        assert!(tax.topics.is_empty());
    }

    #[test]
    fn lc_code_with_non_genre_sub_description_becomes_topic() {
        // D501 = History, "World War I" -> non-genre, kept as topic
        let tax = extract_taxonomy(&["D501".to_string()], &[]);
        assert_eq!(tax.domain, "History");
        assert!(tax.genres.is_empty());
        assert_eq!(tax.topics.len(), 1);
        assert_eq!(tax.topics[0].heading, "History");
        assert_eq!(tax.topics[0].subtopics, vec!["World War I"]);
    }

    #[test]
    fn lc_code_empty_sub_description_no_topic() {
        // B = Philosophy & Religion, empty sub-description
        let tax = extract_taxonomy(&["B".to_string()], &[]);
        assert_eq!(tax.domain, "Philosophy & Religion");
        assert!(tax.genres.is_empty());
        assert!(tax.topics.is_empty());
    }

    #[test]
    fn subject_heading_split_basic() {
        // "Science -- Mathematics" -> heading not genre, sub kept
        let tax = extract_taxonomy(&["Science -- Mathematics".to_string()], &[]);
        assert!(!tax.domain.is_empty());
        assert!(tax.genres.is_empty());
        assert_eq!(tax.topics.len(), 1);
        assert_eq!(tax.topics[0].heading, "Science");
        assert_eq!(tax.topics[0].subtopics, vec!["Mathematics"]);
    }

    #[test]
    fn subject_heading_genre_keyword_keeps_unmapped_subtopic() {
        // "Fiction -- Novel" -> "Fiction" matches LCSH map; "Novel" is not in LCSH map
        // and is kept as subtopic (is_genre_keyword not used for subtopics here)
        let tax = extract_taxonomy(&["Fiction -- Novel".to_string()], &[]);
        assert!(tax.genres.contains(&"Fiction & Novels".to_string()));
        assert_eq!(tax.topics.len(), 1);
        assert_eq!(tax.topics[0].heading, "Fiction");
        assert_eq!(tax.topics[0].subtopics, vec!["Novel"]);
    }

    #[test]
    fn subject_heading_subtopic_genre_keyword() {
        // "History -- Biography" -> "biography" is genre keyword
        let tax = extract_taxonomy(&["History -- Biography".to_string()], &[]);
        assert!(tax.genres.contains(&"Biography & Memoir".to_string()));
        assert_eq!(tax.topics.len(), 1);
        assert_eq!(tax.topics[0].heading, "History");
        assert!(tax.topics[0].subtopics.is_empty());
    }

    #[test]
    fn whitespace_trimmed_subject_ignored_when_empty() {
        let tax = extract_taxonomy(&["   ".to_string()], &[]);
        assert_eq!(tax.domain, "General & Uncategorized");
        assert!(tax.topics.is_empty());
    }

    // ------------------------------------------------------------------
    // Bookshelf inference
    // ------------------------------------------------------------------

    #[test]
    fn bookshelf_science_fiction_inferred() {
        let tax = extract_taxonomy(&[], &["Category: Science Fiction".to_string()]);
        assert!(tax.genres.contains(&"Science Fiction & Fantasy".to_string()));
        assert_eq!(tax.domain, "Language & Literature");
    }

    #[test]
    fn bookshelf_crime_inferred() {
        let tax = extract_taxonomy(&[], &["Category: Crime".to_string()]);
        assert!(tax.genres.contains(&"Mystery & Crime".to_string()));
    }

    #[test]
    fn bookshelf_skip_generic_novels() {
        let tax = extract_taxonomy(&[], &["Category: Novels".to_string()]);
        assert!(tax.genres.is_empty());
    }

    #[test]
    fn bookshelf_skip_best_books_ever() {
        let tax = extract_taxonomy(&[], &["Category: Best Books Ever Listings".to_string()]);
        assert!(tax.genres.is_empty());
    }

    #[test]
    fn bookshelf_skip_general() {
        let tax = extract_taxonomy(&[], &["Category: General".to_string()]);
        assert!(tax.genres.is_empty());
    }

    #[test]
    fn bookshelf_strips_category_prefix() {
        let tax = extract_taxonomy(&[], &["Category: Horror".to_string()]);
        assert!(tax.genres.contains(&"Horror & Gothic".to_string()));
    }

    // ------------------------------------------------------------------
    // Domain resolution priority & sorting
    // ------------------------------------------------------------------

    #[test]
    fn lc_domain_takes_priority_over_inferred() {
        // LC domain "History" should win even if genres infer something else
        let tax = extract_taxonomy(&["D501".to_string()], &["Category: Fiction".to_string()]);
        assert_eq!(tax.domain, "History");
    }

    #[test]
    fn multiple_lc_domains_sorted_lexicographically() {
        // "DA" = History (Great Britain...), "B" = Philosophy & Religion
        // Sorted lexicographically: "History" < "Philosophy & Religion" -> History wins
        let tax = extract_taxonomy(&["DA".to_string(), "B".to_string()], &[]);
        assert_eq!(tax.domain, "History");
    }

    #[test]
    fn inferred_domain_sorting_lexicographically() {
        // "fiction" -> Language & Literature; "biography" -> History
        // Sorted lexicographically -> "History" first
        let tax = extract_taxonomy(&["Fiction".to_string(), "Biography".to_string()], &[]);
        assert_eq!(tax.domain, "History");
    }

    // ------------------------------------------------------------------
    // Deduplication & sorting
    // ------------------------------------------------------------------

    #[test]
    fn duplicate_topics_deduplicated() {
        let tax = extract_taxonomy(
            &[
                "Science -- Mathematics".to_string(),
                "Science -- Mathematics".to_string(),
            ],
            &[],
        );
        assert_eq!(tax.topics.len(), 1);
        assert_eq!(tax.topics[0].heading, "Science");
    }

    #[test]
    fn genres_sorted_lexicographically() {
        let tax = extract_taxonomy(
            &["Fiction".to_string(), "Biography".to_string()],
            &["Category: Horror".to_string()],
        );
        // Expected sorted: Biography & Memoir, Fiction & Novels, Horror & Gothic
        assert!(tax.genres.windows(2).all(|w| w[0] <= w[1]));
    }

    #[test]
    fn topic_subtopics_preserved_and_sorted_not_applicable() {
        let tax = extract_taxonomy(&["History -- World War I -- Europe".to_string()], &[]);
        assert_eq!(tax.topics.len(), 1);
        assert_eq!(tax.topics[0].subtopics, vec!["World War I", "Europe"]);
    }

    // ------------------------------------------------------------------
    // Additional edge cases
    // ------------------------------------------------------------------

    // --- Edge inputs ---

    #[test]
    fn very_long_lc_code_f1461() {
        let tax = extract_taxonomy(&["F1461".to_string()], &[]);
        assert_eq!(tax.domain, "History");
    }

    #[test]
    fn invalid_lc_code_ignored_as_heading() {
        // XYZ123 fails direct LC_MAP lookup; falls through to heading split (no separator)
        // It is treated as heading "XYZ123", which does not match LCSH map, so domain falls back
        let tax = extract_taxonomy(&["XYZ123".to_string()], &[]);
        assert_eq!(tax.domain, "General & Uncategorized");
        assert_eq!(tax.topics.len(), 1);
        assert_eq!(tax.topics[0].heading, "XYZ123");
    }

    #[test]
    fn invalid_numeric_only_lc_code_ignored() {
        let tax = extract_taxonomy(&["123".to_string()], &[]);
        assert_eq!(tax.domain, "General & Uncategorized");
    }

    #[test]
    fn empty_string_subject_ignored() {
        let tax = extract_taxonomy(&["".to_string()], &[]);
        assert_eq!(tax.domain, "General & Uncategorized");
    }

    #[test]
    fn multiple_bookshelves_different_regex_matches() {
        let tax = extract_taxonomy(&[], &["Category: Horror".to_string(), "Category: Poetry".to_string()]);
        assert!(tax.genres.contains(&"Horror & Gothic".to_string()));
        assert!(tax.genres.contains(&"Poetry".to_string()));
    }

    // --- Empty / punctuation / whitespace ---

    #[test]
    fn heading_with_extra_separator_spaces() {
        // Multiple ` -- ` and whitespace around separator
        let tax = extract_taxonomy(&["Science  --  Mathematics".to_string()], &[]);
        assert_eq!(tax.topics.len(), 1);
        assert_eq!(tax.topics[0].heading, "Science");
        assert_eq!(tax.topics[0].subtopics, vec!["Mathematics"]);
    }

    #[test]
    fn empty_subtopic_parts_filtered() {
        // Parts containing only whitespace after trim are filtered
        let tax = extract_taxonomy(&["History --  -- Biography".to_string()], &[]);
        assert!(tax.genres.contains(&"Biography & Memoir".to_string()));
        assert_eq!(tax.topics[0].subtopics.len(), 0);
    }

    #[test]
    fn subtopic_with_punctuation_preserved() {
        let tax = extract_taxonomy(&["History -- World War I.".to_string()], &[]);
        assert_eq!(tax.topics[0].subtopics, vec!["World War I."]);
    }

    #[test]
    fn lc_description_non_standard_punctuation_normalized() {
        // Non-standard punctuation in description should normalize for genre match
        assert!(is_genre_keyword("Memoirs."));
        assert!(is_genre_keyword("Memoirs,"));
        assert!(is_genre_keyword("Memoirs - Essays"));
    }

    // --- Overlap / dedup conflicts ---

    #[test]
    fn genre_from_lc_and_bookshelf_deduplicated() {
        // "CT" -> Biography (LC genre); bookshelf "Category: Biography" doesn't match regex,
        // but "Category: Biograph" matches regex giving "Biography & Memoir".
        // When both sources produce overlapping labels, dedup should keep one entry.
        let tax = extract_taxonomy(&["CT".to_string()], &["Category: Biograph".to_string()]);
        // Should contain genre from LC ("Biography") and from shelf ("Biography & Memoir")
        assert!(tax.genres.contains(&"Biography".to_string()));
        assert!(tax.genres.contains(&"Biography & Memoir".to_string()));
    }

    #[test]
    fn same_domain_from_lc_and_inferred_uses_lc_sorted_first() {
        let tax = extract_taxonomy(&["B".to_string(), "biography".to_string()], &[]);
        // LC domain "Philosophy & Religion" should win over inferred "History"
        assert_eq!(tax.domain, "Philosophy & Religion");
    }

    #[test]
    fn duplicate_bookshelf_entries_deduplicated() {
        let tax = extract_taxonomy(&[], &["Category: Horror".to_string(), "Category: Horror".to_string()]);
        assert_eq!(tax.genres.len(), 1);
        assert_eq!(tax.genres[0], "Horror & Gothic");
    }

    #[test]
    fn same_heading_different_subtopics_kept_separate() {
        let tax = extract_taxonomy(
            &["Science -- Mathematics".to_string(), "Science -- Physics".to_string()],
            &[],
        );
        assert_eq!(tax.topics.len(), 2);
        // Topics are ordered by appearance (not sorted)
        assert_eq!(tax.topics[0].heading, "Science");
        assert_eq!(tax.topics[0].subtopics, vec!["Mathematics"]);
        assert_eq!(tax.topics[1].heading, "Science");
        assert_eq!(tax.topics[1].subtopics, vec!["Physics"]);
    }

    // --- Missing coverage ---

    #[test]
    fn decimal_lc_code_f350_5() {
        let tax = extract_taxonomy(&["F350.5".to_string()], &[]);
        assert_eq!(tax.domain, "History");
        assert!(tax.genres.is_empty());
        assert_eq!(tax.topics.len(), 1);
        assert_eq!(tax.topics[0].subtopics, vec!["Mississippi Valley, Middle West"]);
    }

    #[test]
    fn decimal_lc_code_f590_3() {
        let tax = extract_taxonomy(&["F590.3".to_string()], &[]);
        assert_eq!(tax.domain, "History");
    }

    #[test]
    fn case_insensitive_category_prefix() {
        let tax = extract_taxonomy(&[], &["category: mystery".to_string()]);
        assert!(tax.genres.contains(&"Mystery & Crime".to_string()));
    }

    #[test]
    fn combined_lc_and_heading_subjects() {
        let tax = extract_taxonomy(&["D501".to_string(), "Fiction -- Novel".to_string()], &[]);
        assert_eq!(tax.domain, "History"); // LC domain takes priority
        assert!(tax.genres.contains(&"Fiction & Novels".to_string()));
        assert!(tax.topics.iter().any(|t| t.heading == "History"));
    }
}
