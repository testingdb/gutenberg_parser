//! Gutenberg Archive Extractor — Application Entry Point
//! ---------------------------------------------------------
//! This module serves as the binary entry point for the multi-threaded
//! Gutenberg archive parser. It declares the sub-module hierarchy and
//! delegates execution to the CLI driver (`cli::run`).
//!
//! ## Sub-module Architecture
//! - `config`: Static lookup tables (namespaces, mirror URLs, LC codes,
//!   genre mappings, regex patterns).
//! - `models`: Serializable data structures (`Ebook`, `Agent`, `Taxonomy`)
//!   and bridge variants for schema alignment with external targets.
//! - `utils`: Helper functions for URL transformation, MARC subfield
//!   cleaning, public-domain license detection, and LC classification.
//! - `taxonomy`: Taxonomy inference logic that maps Library of Congress
//!   subject headings and Gutenberg bookshelf labels to domain/genre/topic.
//! - `xml_parser`: RDF/XML stream parser that extracts ebook metadata,
//!   agents, formats, and descriptors from Project Gutenberg feeds.
//! - `cli`: Command-line argument parsing, archive download, worker-pool
//!   orchestration, and chunked JSON output.

mod cli;
mod config;
mod models;
mod taxonomy;
mod utils;
mod xml_parser;

/// Main execution entry point.
///
/// Delegates to `cli::run()`, which parses arguments, configures the
/// pipeline, and manages the multi-threaded worker pool.
fn main() {
    cli::run();
}

#[cfg(test)]
mod tests {
    use super::utils::clean_marc_subfields;

    /// Strips an embedded MARC subfield code (`$b`) from the middle of text.
    /// Ensures that punctuation and spacing are preserved correctly after
    /// removal.
    #[test]
    fn strips_subfield_code_in_middle() {
        assert_eq!(
            clean_marc_subfields("Eloisa : $b or, A series of original letters"),
            "Eloisa : or, A series of original letters"
        );
    }

    /// Removes leading and trailing MARC subfield codes (`$a`, `$b`).
    /// Validates trimming of whitespace around the cleaned result.
    #[test]
    fn strips_leading_and_trailing_codes() {
        assert_eq!(clean_marc_subfields("$a The Title"), "The Title");
        assert_eq!(clean_marc_subfields("The Title $b"), "The Title");
    }

    /// Handles subfield codes followed by a colon (`$b:`).
    /// Verifies colon preservation after code removal.
    #[test]
    fn strips_code_followed_by_colon() {
        assert_eq!(
            clean_marc_subfields("Dress design $b: an account of costume"),
            "Dress design : an account of costume"
        );
    }

    /// Validates punctuation preservation (`$b,` and `$b.`) after subfield
    /// stripping.
    #[test]
    fn strips_code_followed_by_punctuation() {
        assert_eq!(clean_marc_subfields("Title $b, subtitle"), "Title , subtitle");
        assert_eq!(clean_marc_subfields("Title $b. subtitle"), "Title . subtitle");
    }

    /// Confirms consecutive codes (`$a ... $b`) collapse into a single
    /// whitespace and that double-spacing / tabs are normalized.
    #[test]
    fn strips_consecutive_codes_and_normalizes_spacing() {
        assert_eq!(clean_marc_subfields("$a Title $b  subtitle"), "Title subtitle");
        assert_eq!(
            clean_marc_subfields("Title  with\t double  spacing"),
            "Title with double spacing"
        );
    }

    /// Protects monetary amounts (`$100`, `$5`) from being misinterpreted
    /// as MARC subfield markers.
    #[test]
    fn preserves_dollar_amounts() {
        assert_eq!(clean_marc_subfields("The $100 Startup"), "The $100 Startup");
        assert_eq!(clean_marc_subfields("The $5 Man"), "The $5 Man");
    }

    /// Ensures words that begin with a dollar fragment (`$billions`)
    /// or codes without a space (`$aThe Title`) are untouched.
    #[test]
    fn preserves_words_starting_with_dollar_fragment() {
        assert_eq!(clean_marc_subfields("How to Make $billions"), "How to Make $billions");
        assert_eq!(clean_marc_subfields("$aThe Title"), "$aThe Title");
    }

    /// Trims leading and trailing whitespace from cleaned strings.
    #[test]
    fn trims_whitespace() {
        assert_eq!(clean_marc_subfields("  Title  "), "Title");
    }

    /// Empty inputs and pure subfield codes yield empty outputs.
    #[test]
    fn empty_input_stays_empty() {
        assert_eq!(clean_marc_subfields(""), "");
        assert_eq!(clean_marc_subfields("  "), "");
        assert_eq!(clean_marc_subfields("$b"), "");
    }
}
