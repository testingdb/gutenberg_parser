//! Data Models for Gutenberg Archive Serialization
//! ---------------------------------------------------------
//! Defines the core `Ebook` record structure and all nested types
//! (`Agent`, `Format`, `Topic`, `Taxonomy`). Each field uses `serde`
//! annotations (`rename`, `skip_serializing_if`, `default`) to control
//! JSON output and handle optional fields cleanly.
//!
//! ## Bridge Schema (`BridgeEbook`)
//! When the CLI `--bridge` flag is enabled, each `Ebook` is mapped to a
//! `BridgeEbook` with renamed fields (`lang_code`, `pg_download_count`,
//! `external_cover_url`, etc.) that match the target database schema.
//!
//! ## Serialization Pruning
//! Optional fields (`agent_id`, `birth_date`, `death_date`, `image`,
//! `description`, `issued_date`) are omitted from JSON when `None` or empty,
//! reducing payload size.

use serde::Serialize;

// ---------------------------------------------------------------------------
// Agent
// ---------------------------------------------------------------------------

/// Represents a contributor (author, translator, illustrator, etc.).
///
/// The `agent_type` string identifies the role (`author`, `translator`,
/// `illustrator`, `editor`, etc.). `agent_id` links to the Gutenberg agent
/// database when present. `image` carries a Wikipedia thumbnail when the
/// `--wiki-images` flag resolved one.
#[derive(Serialize, Debug, Clone)]
pub struct Agent {
    /// Role of the agent (`author`, `translator`, etc.).
    #[serde(rename = "type")]
    pub agent_type: String,

    /// Numeric agent identifier from the RDF resource URI.
    /// Omitted from JSON when `None`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_id: Option<u64>,

    /// Full display name of the agent.
    pub name: String,

    /// Alternative names / pseudonyms.
    #[serde(default)]
    pub aliases: Vec<String>,

    /// External webpages of the agent (personal sites, Wikipedia, etc.),
    /// read from the `webpage` RDF child nodes.
    #[serde(default)]
    pub webpages: Vec<String>,

    /// Birth year / date, when known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub birth_date: Option<String>,

    /// Death year / date, when known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub death_date: Option<String>,

    /// Thumbnail image URL of the agent, resolved from Wikipedia when
    /// `--wiki-images` is enabled and the agent has a Wikipedia page.
    /// Omitted from JSON when `None`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
}

// ---------------------------------------------------------------------------
// Format
// ---------------------------------------------------------------------------

/// A downloadable file format for an ebook.
///
/// `mime_type` is normalized to `text/html` or `application/epub+zip`.
/// The `url` points to the transformed mirror URL.
#[derive(Serialize, Debug, Clone)]
pub struct Format {
    /// MIME type of the format file.
    #[serde(rename = "type")]
    pub mime_type: String,

    /// Transformed download URL.
    pub url: String,
}

// ---------------------------------------------------------------------------
// Topic & Taxonomy
// ---------------------------------------------------------------------------

/// A single taxonomy topic (subject heading) with optional subtopics.
#[derive(Serialize, Debug, Clone)]
pub struct Topic {
    /// Main heading (e.g. `"Fiction"`, `"Biography"`).
    pub heading: String,

    /// Finer-grained sub-headings.
    #[serde(default)]
    pub subtopics: Vec<String>,
}

/// Taxonomy classification for an ebook.
///
/// `domain` is the broad category (`History`, `Science`, etc.).
/// `genres` are inferred from bookshelf labels and LC form keywords.
/// `topics` are constructed from `subject` RDF nodes.
#[derive(Serialize, Debug, Clone)]
pub struct Taxonomy {
    /// Broad domain category.
    pub domain: String,

    /// Inferred genre labels.
    #[serde(default)]
    pub genres: Vec<String>,

    /// Structured topic headings.
    #[serde(default)]
    pub topics: Vec<Topic>,
}

// ---------------------------------------------------------------------------
// Ebook (Core Record)
// ---------------------------------------------------------------------------

/// Complete metadata record for a Project Gutenberg ebook.
///
/// All fields are serialized in JSON arrays by `cli::write_chunk`. The
/// `taxomomy` field (note: field name `taxonomy` in JSON matches `Taxonomy`
/// struct) holds the domain/genre/topic classification.
#[derive(Serialize, Debug, Clone)]
pub struct Ebook {
    /// Cleaned title (MARC subfield codes removed).
    pub title: String,

    /// Alternative titles (translated, variant spellings).
    #[serde(default)]
    pub alternative_titles: Vec<String>,

    /// Publication / release date.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub issued_date: Option<String>,

    /// Contributors (authors, translators, illustrators, editors).
    #[serde(default)]
    pub agents: Vec<Agent>,

    /// Short description or abstract.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Language code (e.g. `en`, `fr`, `de`).
    pub language: String,

    /// Available formats (`text/html` and `application/epub+zip` required).
    pub formats: Vec<Format>,

    /// Taxonomy classification.
    pub taxonomy: Taxonomy,

    /// Download count from Gutenberg statistics.
    pub downloads: u64,

    /// Gutenberg ebook ID (numeric string, e.g. `"1342"`).
    pub ebook_id: String,

    /// URL of the medium-sized cover image.
    pub cover_image: String,

    /// License text (`"Public domain in the USA."` when public domain).
    pub license: String,
}

// ---------------------------------------------------------------------------
// Bridge Schema Models (Target Schema Alignment)
// ---------------------------------------------------------------------------

/// Bridge representation of an agent with renamed fields.
///
/// `agent_type` is mapped to `role`; `agent_id` to `pg_id`; `webpages` to
/// `external_urls`.
#[derive(Serialize, Debug, Clone)]
pub struct BridgeAgent {
    /// Agent role (`author`, `translator`, etc.).
    pub role: String,

    /// Numeric Gutenberg agent ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pg_id: Option<u64>,

    /// Full display name.
    pub name: String,

    /// Alternative names.
    #[serde(default)]
    pub aliases: Vec<String>,

    /// External URLs (renamed from `webpages`).
    #[serde(default)]
    pub external_urls: Vec<String>,

    /// Birth date.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub birth_date: Option<String>,

    /// Death date.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub death_date: Option<String>,

    /// Wikipedia thumbnail image URL (same name as in parser mode).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
}

/// Bridge representation of a format with renamed fields.
#[derive(Serialize, Debug, Clone)]
pub struct BridgeFormat {
    /// MIME type.
    pub mime_type: String,

    /// File URL (renamed from `url`).
    pub file_url: String,
}

/// Bridge representation of the complete ebook record.
///
/// Field names (`lang_code`, `pg_download_count`, `external_cover_url`,
/// `license_statement`) match the external database target schema.
#[derive(Serialize, Debug, Clone)]
pub struct BridgeEbook {
    /// Cleaned title.
    pub title: String,

    /// Alternative titles.
    #[serde(default)]
    pub alternative_titles: Vec<String>,

    /// Publication date.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub issued_date: Option<String>,

    /// Contributors mapped to `BridgeAgent`.
    #[serde(default)]
    pub agents: Vec<BridgeAgent>,

    /// Description / abstract.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Language code (renamed from `language`).
    pub lang_code: String,

    /// Formats mapped to `BridgeFormat`.
    pub formats: Vec<BridgeFormat>,

    /// Taxonomy classification (unchanged).
    pub taxonomy: Taxonomy,

    /// Download count (renamed from `downloads`).
    pub pg_download_count: u64,

    /// Numeric ebook ID (renamed from `ebook_id`).
    pub pg_id: u64,

    /// Cover image URL (renamed from `cover_image` in bridge mode only).
    /// Always emitted, as an empty string when no cover URL was resolved.
    pub external_cover_url: String,

    /// License text (renamed from `license`).
    pub license_statement: String,
}

// ---------------------------------------------------------------------------
// Conversion Implementations
// ---------------------------------------------------------------------------

impl From<&Agent> for BridgeAgent {
    /// Converts an `Agent` to `BridgeAgent`, mapping `agent_type` → `role`,
    /// `agent_id` → `pg_id`, `webpages` → `external_urls`.
    fn from(agent: &Agent) -> Self {
        BridgeAgent {
            role: agent.agent_type.clone(),
            pg_id: agent.agent_id,
            name: agent.name.clone(),
            aliases: agent.aliases.clone(),
            external_urls: agent.webpages.clone(),
            birth_date: agent.birth_date.clone(),
            death_date: agent.death_date.clone(),
            image: agent.image.clone(),
        }
    }
}

impl From<&Format> for BridgeFormat {
    /// Converts `Format` to `BridgeFormat`, mapping `url` → `file_url`.
    fn from(format: &Format) -> Self {
        BridgeFormat {
            mime_type: format.mime_type.clone(),
            file_url: format.url.clone(),
        }
    }
}

impl From<&Ebook> for BridgeEbook {
    /// Converts a full `Ebook` record to the `BridgeEbook` target schema.
    /// Parses `ebook_id` into `pg_id` (defaults to `0` on parse failure).
    fn from(ebook: &Ebook) -> Self {
        BridgeEbook {
            title: ebook.title.clone(),
            alternative_titles: ebook.alternative_titles.clone(),
            issued_date: ebook.issued_date.clone(),
            agents: ebook.agents.iter().map(BridgeAgent::from).collect(),
            description: ebook.description.clone(),
            lang_code: ebook.language.clone(),
            formats: ebook.formats.iter().map(BridgeFormat::from).collect(),
            taxonomy: ebook.taxonomy.clone(),
            pg_download_count: ebook.downloads,
            pg_id: ebook.ebook_id.parse::<u64>().unwrap_or(0),
            external_cover_url: ebook.cover_image.clone(),
            license_statement: ebook.license.clone(),
        }
    }
}

// ---------------------------------------------------------------------------
// Unit Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Minimal valid `Ebook` used to exercise both serialization modes.
    fn sample_ebook(cover_image: &str) -> Ebook {
        Ebook {
            title: "Test".to_string(),
            alternative_titles: vec![],
            issued_date: None,
            agents: vec![],
            description: None,
            language: "en".to_string(),
            formats: vec![],
            taxonomy: Taxonomy {
                domain: "General".to_string(),
                genres: vec![],
                topics: vec![],
            },
            downloads: 0,
            ebook_id: "42".to_string(),
            cover_image: cover_image.to_string(),
            license: "Public domain".to_string(),
        }
    }

    /// Basic agent creation and property verification.
    #[test]
    fn agent_creation() {
        let agent = Agent {
            agent_type: "author".to_string(),
            agent_id: Some(123),
            name: "Alice".to_string(),
            aliases: vec![],
            webpages: vec![],
            birth_date: None,
            death_date: None,
            image: None,
        };
        assert_eq!(agent.name, "Alice");
    }

    /// Validates `BridgeEbook` conversion, including numeric `pg_id`
    /// parsing from the `ebook_id` string.
    #[test]
    fn bridge_ebook_conversion() {
        let ebook = sample_ebook("");
        let bridge: BridgeEbook = BridgeEbook::from(&ebook);
        assert_eq!(bridge.title, "Test");
        assert_eq!(bridge.pg_id, 42);
        assert_eq!(bridge.external_cover_url, "");

        // An unresolved cover is still emitted as an empty string, not omitted.
        let json = serde_json::to_string(&bridge).unwrap();
        assert!(
            json.contains("\"external_cover_url\":\"\""),
            "empty cover URL must be emitted: {}",
            json
        );
        assert!(
            !json.contains("md_cover_image_url"),
            "legacy cover key must be absent: {}",
            json
        );
    }

    /// Only the cover key changes in bridge mode; the parser-mode record keeps
    /// `cover_image`, and every other bridge key keeps its established name.
    #[test]
    fn bridge_renames_only_the_cover_key() {
        let cover_url = "https://gutenberg.pglaf.org/cache/epub/1342/pg1342.med.jpg";
        let ebook = sample_ebook(cover_url);
        let parser_json = serde_json::to_value(&ebook).unwrap();
        let bridge_json = serde_json::to_value(BridgeEbook::from(&ebook)).unwrap();

        assert_eq!(parser_json["cover_image"].as_str(), Some(cover_url));
        assert!(parser_json.get("external_cover_url").is_none());
        assert!(parser_json.get("md_cover_image_url").is_none());

        assert_eq!(bridge_json["external_cover_url"].as_str(), Some(cover_url));
        assert!(bridge_json.get("cover_image").is_none());
        assert!(bridge_json.get("md_cover_image_url").is_none());

        let mut keys: Vec<&str> = bridge_json.as_object().unwrap().keys().map(String::as_str).collect();
        keys.sort_unstable();
        assert_eq!(
            keys,
            vec![
                "agents",
                "alternative_titles",
                "external_cover_url",
                "formats",
                "lang_code",
                "license_statement",
                "pg_download_count",
                "pg_id",
                "taxonomy",
                "title",
            ]
        );
    }

    /// A resolved Wikipedia thumbnail survives the bridge conversion, and is
    /// omitted from JSON when no image was resolved.
    #[test]
    fn bridge_agent_carries_optional_image() {
        let mut agent = Agent {
            agent_type: "author".to_string(),
            agent_id: Some(31),
            name: "Verne, Jules".to_string(),
            aliases: vec![],
            webpages: vec!["https://en.wikipedia.org/wiki/Jules_Verne".to_string()],
            birth_date: None,
            death_date: None,
            image: Some("https://thumb.wikimedia.org/jules.jpg".to_string()),
        };
        assert_eq!(
            BridgeAgent::from(&agent).image.as_deref(),
            Some("https://thumb.wikimedia.org/jules.jpg")
        );

        agent.image = None;
        let json = serde_json::to_string(&BridgeAgent::from(&agent)).unwrap();
        assert!(!json.contains("\"image\""), "image must be omitted: {}", json);
    }
}
