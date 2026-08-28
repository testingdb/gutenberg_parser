//! RDF / XML Parser for Project Gutenberg Feeds
//! ---------------------------------------------------------
//! Parses individual XML files from the `rdf-files.tar.bz2` archive.
//! Each file contains one or more `ebook` elements describing a Gutenberg
//! release. The parser filters for `type == "text"`, validates required
//! fields (`title`, `language`, `creator`, `formats`), and builds structured
//! `Ebook` objects.
//!
//! ## Filtering Rules
//! - `type` must be `"text"` (audio / video excluded).
//! - `title` must be non-empty after MARC cleaning.
//! - `language` must be non-empty.
//! - At least one `creator` / `aut` agent must exist.
//! - Both `text/html` and `application/epub+zip` formats must be present.
//! - `license` must contain `"public domain"` unless `include_licensed`
//!   is enabled.
//!
//! ## Agent Extraction
//! `parse_agent` handles nested `agent` nodes inside `creator`, `trl`,
//! `aui`, `ill`, `edt`, and `aut` tags. It reads `rdf:about`, `name`,
//! `alias`, `birthdate`, `deathdate`, and `webpage` children.

use crate::config::*;
use crate::models::*;
use crate::taxonomy::extract_taxonomy;
use crate::utils::*;
use roxmltree::{Document, Node};
use std::collections::HashSet;

/// Parses an `agent` XML node into an `Agent` model.
///
/// # Arguments
/// * `parent_node` — Parent XML node (`creator`, `trl`, etc.); may be `None`.
/// * `agent_type` — Role string (`author`, `translator`, etc.).
/// * `ebook_id` — Numeric ebook identifier (for URL transformation).
/// * `mirror_base` — Mirror base URL (passed to `transform_url`).
///
/// # Returns
/// `Some(Agent)` if the agent has a non-empty `name`; `None` otherwise.
pub fn parse_agent(parent_node: Option<Node>, agent_type: &str, ebook_id: &str, mirror_base: &str) -> Option<Agent> {
    let parent = parent_node?;

    // Resolve the `agent` node: either the parent itself (if it is an
    // `pgterms:agent`) or the first child `agent` element.
    let agent_node =
        if parent.tag_name().name() == "agent" && parent.tag_name().namespace() == NAMESPACES.get("pgterms").copied() {
            parent
        } else {
            parent
                .children()
                .find(|n| n.is_element() && n.tag_name().name() == "agent")?
        };

    // Extract agent ID from the RDF `about` attribute.
    let about = agent_node.attribute((NAMESPACES["rdf"], "about")).unwrap_or("");
    let agent_id = RE_AGENT_ID
        .captures(about)
        .and_then(|c| c.get(1))
        .and_then(|m| m.as_str().parse::<u64>().ok());

    // Read the `name` child node.
    let name = agent_node
        .children()
        .find(|n| n.is_element() && n.tag_name().name() == "name")
        .and_then(|n| n.text())
        .unwrap_or("")
        .trim();

    if name.is_empty() {
        return None;
    }

    // Read optional aliases.
    let aliases: Vec<String> = agent_node
        .children()
        .filter(|n| n.is_element() && n.tag_name().name() == "alias")
        .filter_map(|n| n.text())
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
        .collect();

    // Read optional web pages (transformed to mirror URLs).
    let webpages: Vec<String> = agent_node
        .children()
        .filter(|n| n.is_element() && n.tag_name().name() == "webpage")
        .filter_map(|n| n.attribute((NAMESPACES["rdf"], "resource")))
        .filter_map(|url| transform_url(Some(url), ebook_id, mirror_base))
        .filter(|url| !url.is_empty())
        .collect();

    // Read optional birth and death dates.
    let birth_date = agent_node
        .children()
        .find(|n| n.is_element() && n.tag_name().name() == "birthdate")
        .and_then(|n| n.text())
        .map(|t| t.trim().to_string());

    let death_date = agent_node
        .children()
        .find(|n| n.is_element() && n.tag_name().name() == "deathdate")
        .and_then(|n| n.text())
        .map(|t| t.trim().to_string());

    Some(Agent {
        agent_type: agent_type.to_string(),
        agent_id,
        name: name.to_string(),
        aliases,
        webpages,
        birth_date,
        death_date,
    })
}

/// Parses a raw RDF XML byte buffer into a structured `Ebook`.
///
/// # Filtering
/// Returns an error string for any of the following conditions:
/// - `"utf8_error"`: The byte buffer is not valid UTF-8.
/// - `"xml_parse_error"`: The XML is malformed.
/// - `"no_ebook_element"`: No `<ebook>` root found.
/// - `"filter_type"`: `type` is not `"text"`.
/// - `"filter_title"`: Title is empty after cleaning.
/// - `"filter_language"`: Language is empty.
/// - `"filter_license"`: Non-public-domain and `include_licensed` is `false`.
/// - `"filter_required_formats"`: Missing `text/html` or `application/epub+zip`.
/// - `"filter_creator"`: No agent entries exist.
///
/// # Arguments
/// * `xml_data` — Raw XML bytes from the archive entry.
/// * `mirror_base` — Mirror base URL for URL transformation.
/// * `include_licensed` — If `false`, excludes non-public-domain ebooks.
///
/// # Returns
/// `Result<Ebook, &'static str>` — Structured ebook or error code.
pub fn process_rdf_xml(xml_data: &[u8], mirror_base: &str, include_licensed: bool) -> Result<Ebook, &'static str> {
    // Decode UTF-8 and parse XML document.
    let xml_str = std::str::from_utf8(xml_data).map_err(|_| "utf8_error")?;
    let doc = Document::parse(xml_str).map_err(|_| "xml_parse_error")?;

    let root = doc.root_element();
    let ebook = root
        .children()
        .find(|n| n.is_element() && n.tag_name().name() == "ebook")
        .ok_or("no_ebook_element")?;

    // Extract ebook identifier from RDF `about` attribute.
    let ebook_about = ebook.attribute((NAMESPACES["rdf"], "about")).unwrap_or("");
    let ebook_id = ebook_about.replace("ebooks/", "").trim().to_string();

    // Verify that the resource type is a text ebook.
    let type_val = ebook
        .descendants()
        .find(|n| n.is_element() && n.tag_name().name() == "type")
        .and_then(|n| n.descendants().find(|c| c.tag_name().name() == "value"))
        .and_then(|n| n.text())
        .unwrap_or("")
        .trim()
        .to_lowercase();

    if type_val != "text" {
        return Err("filter_type");
    }

    // Clean and validate title.
    let title = ebook
        .children()
        .find(|n| n.is_element() && n.tag_name().name() == "title")
        .and_then(|n| n.text())
        .map(clean_marc_subfields)
        .filter(|t| !t.is_empty())
        .ok_or("filter_title")?;

    // Read and validate language.
    let language = ebook
        .children()
        .find(|n| n.is_element() && n.tag_name().name() == "language")
        .and_then(|n| n.descendants().find(|c| c.tag_name().name() == "value"))
        .and_then(|n| n.text())
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
        .ok_or("filter_language")?;

    // Read license and apply public-domain filter.
    let license = ebook
        .children()
        .find(|n| n.is_element() && n.tag_name().name() == "rights")
        .and_then(|n| n.text())
        .map(|t| t.trim().to_string())
        .unwrap_or_else(|| "Public domain in the USA.".to_string());

    if !include_licensed && !is_public_domain_license(&license) {
        return Err("filter_license");
    }

    // Extract formats (`hasFormat` → `file` nodes).
    let mut seen_mime = HashSet::new();
    let mut format_objects = Vec::new();

    for has_fmt in ebook
        .children()
        .filter(|n| n.is_element() && n.tag_name().name() == "hasFormat")
    {
        if let Some(file_node) = has_fmt
            .children()
            .find(|n| n.is_element() && n.tag_name().name() == "file")
        {
            let file_url = file_node
                .attribute((NAMESPACES["rdf"], "about"))
                .or_else(|| file_node.attribute((NAMESPACES["rdf"], "resource")))
                .unwrap_or("");

            let fmt_val = file_node
                .descendants()
                .find(|n| n.is_element() && n.tag_name().name() == "value")
                .and_then(|n| n.text())
                .unwrap_or("")
                .trim();

            // Transform URL and infer MIME type from value or extension.
            if let Some(transformed_url) = transform_url(Some(file_url), &ebook_id, mirror_base) {
                let url_lower = transformed_url.to_lowercase();
                let mime_key =
                    if fmt_val.contains("text/html") || url_lower.ends_with(".htm") || url_lower.ends_with(".html") {
                        Some("text/html")
                    } else if fmt_val.contains("epub") || url_lower.ends_with(".epub") {
                        Some("application/epub+zip")
                    } else {
                        None
                    };

                if let Some(key) = mime_key {
                    if seen_mime.insert(key) {
                        format_objects.push(Format {
                            mime_type: key.to_string(),
                            url: transformed_url,
                        });
                    }
                }
            }
        }
    }

    // Both text/html and epub formats are mandatory.
    if !seen_mime.contains("text/html") || !seen_mime.contains("application/epub+zip") {
        return Err("filter_required_formats");
    }

    // Extract agents using `push_agents` closure.
    let mut agents = Vec::new();
    let push_agents = |tag: &str, agent_type: &str, agents: &mut Vec<Agent>| {
        for node in ebook
            .children()
            .filter(|n| n.is_element() && n.tag_name().name() == tag)
        {
            if let Some(agent) = parse_agent(Some(node), agent_type, &ebook_id, mirror_base) {
                agents.push(agent);
            }
        }
    };

    push_agents("creator", "author", &mut agents);
    push_agents("trl", "translator", &mut agents);
    push_agents("aui", "introduction_author", &mut agents);
    push_agents("ill", "illustrator", &mut agents);
    push_agents("edt", "editor", &mut agents);
    // Fallback to `aut` tag if no `author` found.
    if !agents.iter().any(|a| a.agent_type == "author") {
        push_agents("aut", "author", &mut agents);
    }

    if agents.is_empty() {
        return Err("filter_creator");
    }

    // Build cover image URL (medium size) using mirror transformation.
    let cover_image = transform_url(
        Some(&format!(
            "https://www.gutenberg.org/cache/epub/{}/pg{}.cover.medium.jpg",
            ebook_id, ebook_id
        )),
        &ebook_id,
        mirror_base,
    )
    .unwrap();

    // Read optional description from `marc520`, `marc500`, or `description`.
    let description = ebook
        .children()
        .find(|n| ["marc520", "marc500", "description"].contains(&n.tag_name().name()))
        .and_then(|n| n.text())
        .map(|t| t.trim().to_string());

    // Read alternative titles.
    let alternative_titles: Vec<String> = ebook
        .children()
        .filter(|n| n.tag_name().name() == "alternative")
        .filter_map(|n| n.text())
        .map(clean_marc_subfields)
        .filter(|t| !t.is_empty())
        .collect();

    // Read issued date.
    let issued_date = ebook
        .children()
        .find(|n| n.tag_name().name() == "issued")
        .and_then(|n| n.text())
        .map(|t| t.trim().to_string());

    // Read download count.
    let downloads = ebook
        .children()
        .find(|n| n.tag_name().name() == "downloads")
        .and_then(|n| n.text())
        .and_then(|t| t.trim().parse::<u64>().ok())
        .unwrap_or(0);

    // Read raw subjects and bookshelves for taxonomy inference.
    let subjects_raw: Vec<String> = ebook
        .children()
        .filter(|n| n.tag_name().name() == "subject")
        .filter_map(|n| n.descendants().find(|c| c.tag_name().name() == "value"))
        .filter_map(|n| n.text())
        .map(|t| t.trim().to_string())
        .collect();

    let bookshelves_raw: Vec<String> = ebook
        .children()
        .filter(|n| n.tag_name().name() == "bookshelf")
        .filter_map(|n| n.descendants().find(|c| c.tag_name().name() == "value"))
        .filter_map(|n| n.text())
        .map(|t| t.trim().to_string())
        .collect();

    let taxonomy = extract_taxonomy(&subjects_raw, &bookshelves_raw);

    Ok(Ebook {
        title,
        alternative_titles,
        issued_date,
        agents,
        description,
        language,
        formats: format_objects,
        taxonomy,
        downloads,
        ebook_id,
        cover_image,
        license,
    })
}

// ---------------------------------------------------------------------------
// Unit Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use roxmltree::Document;

    // =======================================================================
    // parse_agent tests
    // =======================================================================

    fn agent_xml_self(
        name: &str,
        aliases: &[&str],
        birth: Option<&str>,
        death: Option<&str>,
        pages: &[&str],
        about: &str,
    ) -> String {
        let mut aliases_xml = String::new();
        for a in aliases {
            aliases_xml.push_str(&format!("<alias>{}</alias>", a));
        }
        let birth_xml = birth
            .map(|b| format!("<birthdate>{}</birthdate>", b))
            .unwrap_or_default();
        let death_xml = death
            .map(|d| format!("<deathdate>{}</deathdate>", d))
            .unwrap_or_default();
        let pages_xml = pages
            .iter()
            .map(|p| format!("<webpage rdf:resource=\"{}\"/>", p))
            .collect::<String>();
        format!(
            r#"<agent xmlns="http://www.gutenberg.org/2009/pgterms/" xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#" rdf:about="{}">
<name>{}</name>
{}{}{}{}
</agent>"#,
            about, name, aliases_xml, birth_xml, death_xml, pages_xml
        )
    }

    fn agent_xml_child(parent_tag: &str, _agent_type_for_parse: &str, name: &str, about: &str) -> String {
        format!(
            r#"<{} xmlns="http://www.gutenberg.org/2009/pgterms/" xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
<agent rdf:about="{}">
<name>{}</name>
</agent>
</{}>"#,
            parent_tag, about, name, parent_tag
        )
    }

    #[test]
    fn parse_agent_none_parent() {
        assert!(parse_agent(None, "author", "1", "").is_none());
    }

    #[test]
    fn parse_agent_self_agent_node() {
        let xml = agent_xml_self(
            "Alice",
            &["Alicia"],
            Some("1900"),
            Some("1950"),
            &["https://example.com/alice"],
            "http://www.gutenberg.org/ebooks/agents/42",
        );
        let doc = Document::parse(&xml).unwrap();
        let agent = parse_agent(Some(doc.root_element()), "author", "123", "");
        let a = agent.unwrap();
        assert_eq!(a.agent_type, "author");
        assert_eq!(a.name, "Alice");
        assert_eq!(a.agent_id, Some(42));
        assert_eq!(a.aliases, vec!["Alicia"]);
        assert_eq!(a.birth_date, Some("1900".to_string()));
        assert_eq!(a.death_date, Some("1950".to_string()));
        assert!(a.webpages.contains(&"https://example.com/alice".to_string()));
    }

    #[test]
    fn parse_agent_child_agent_node() {
        let xml = agent_xml_child("creator", "author", "Bob", "http://www.gutenberg.org/ebooks/agents/99");
        let doc = Document::parse(&xml).unwrap();
        let parent = doc.root_element();
        let agent = parse_agent(Some(parent), "author", "777", "");
        let a = agent.unwrap();
        assert_eq!(a.name, "Bob");
        assert_eq!(a.agent_id, Some(99));
    }

    #[test]
    fn parse_agent_empty_name_returns_none() {
        let xml = agent_xml_self("", &[], None, None, &[], "");
        let doc = Document::parse(&xml).unwrap();
        assert!(parse_agent(Some(doc.root_element()), "author", "1", "").is_none());
    }

    #[test]
    fn parse_agent_empty_name_child_agent() {
        let xml = agent_xml_child("aut", "author", "", "");
        let doc = Document::parse(&xml).unwrap();
        assert!(parse_agent(Some(doc.root_element()), "author", "1", "").is_none());
    }

    // =======================================================================
    // process_rdf_xml error branch fixtures (embedded XML)
    // =======================================================================

    #[test]
    fn process_rdf_utf8_error() {
        assert_eq!(process_rdf_xml(b"\xff\xfe", "", false).unwrap_err(), "utf8_error");
    }

    #[test]
    fn process_rdf_xml_parse_error() {
        assert_eq!(process_rdf_xml(b"<bad", "", false).unwrap_err(), "xml_parse_error");
    }

    #[test]
    fn process_rdf_no_ebook_element() {
        let xml = r#"<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"><other/></rdf:RDF>"#;
        assert_eq!(
            process_rdf_xml(xml.as_bytes(), "", false).unwrap_err(),
            "no_ebook_element"
        );
    }

    fn base_ebook_xml(
        type_val: &str,
        title_text: &str,
        language_text: &str,
        rights_text: &str,
        _include_licensed: bool,
    ) -> Vec<u8> {
        format!(
            r#"<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#" xmlns:dcterms="http://purl.org/dc/terms/" xmlns:pgterms="http://www.gutenberg.org/2009/pgterms/">
<ebook rdf:about="ebooks/1">
<type>
<value>{}</value>
</type>
<title>{}</title>
<language>
<value>{}</value>
</language>
<rights>{}</rights>
</ebook>
</rdf:RDF>"#,
            type_val, title_text, language_text, rights_text
        ).into_bytes()
    }

    #[test]
    fn process_rdf_filter_type_not_text() {
        let xml = base_ebook_xml("audio", "Title", "en", "Public domain", false);
        assert_eq!(process_rdf_xml(&xml, "", false).unwrap_err(), "filter_type");
    }

    #[test]
    fn process_rdf_filter_title_empty() {
        let xml = base_ebook_xml("text", "", "en", "Public domain", false);
        assert_eq!(process_rdf_xml(&xml, "", false).unwrap_err(), "filter_title");
    }

    #[test]
    fn process_rdf_filter_title_only_subfields() {
        let xml = base_ebook_xml("text", "$b", "en", "Public domain", false);
        assert_eq!(process_rdf_xml(&xml, "", false).unwrap_err(), "filter_title");
    }

    #[test]
    fn process_rdf_filter_language_empty() {
        let xml = base_ebook_xml("text", "Title", "", "Public domain", false);
        assert_eq!(process_rdf_xml(&xml, "", false).unwrap_err(), "filter_language");
    }

    #[test]
    fn process_rdf_filter_license_non_public_domain() {
        let xml = base_ebook_xml("text", "Title", "en", "Copyrighted material.", false);
        assert_eq!(process_rdf_xml(&xml, "", false).unwrap_err(), "filter_license");
    }

    #[test]
    fn process_rdf_filter_license_public_domain_passes() {
        let xml = base_ebook_xml("text", "Title", "en", "Public domain in the USA.", false);
        let res = process_rdf_xml(&xml, "", false);
        assert!(!matches!(res, Err("filter_license")));
    }

    #[test]
    fn process_rdf_filter_license_include_licensed_allows_non_pd() {
        let xml = base_ebook_xml("text", "Title", "en", "Copyrighted.", true);
        let res = process_rdf_xml(&xml, "", true);
        assert!(!matches!(res, Err("filter_license")));
    }

    fn full_ebook_xml_with_formats(html_url: &str, epub_url: &str, include_formats: bool) -> String {
        let formats = if include_formats {
            format!(
                r#"<hasFormat><file rdf:about="{}"><value>text/html</value></file></hasFormat>
<hasFormat><file rdf:about="{}"><value>application/epub+zip</value></file></hasFormat>"#,
                html_url, epub_url
            )
        } else {
            String::new()
        };
        format!(
            r#"<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#" xmlns:dcterms="http://purl.org/dc/terms/" xmlns:pgterms="http://www.gutenberg.org/2009/pgterms/">
<ebook rdf:about="ebooks/42">
<type><value>text</value></type>
<title>The Test Book</title>
<language><value>en</value></language>
<rights>Public domain in the USA.</rights>
{}
</ebook>
</rdf:RDF>"#,
            formats
        )
    }

    #[test]
    fn process_rdf_filter_required_formats_missing_epub() {
        let xml = format!(
            r#"<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#" xmlns:pgterms="http://www.gutenberg.org/2009/pgterms/">
<ebook rdf:about="ebooks/42" xmlns="http://www.gutenberg.org/2009/pgterms/">
<type><value>text</value></type>
<title>The Test Book</title>
<language><value>en</value></language>
<rights>Public domain in the USA.</rights>
<creator><agent rdf:about="http://www.gutenberg.org/ebooks/agents/1"><name>Author</name></agent></creator>
<hasFormat><file rdf:about="https://www.gutenberg.org/files/42/42-0.html"><value>text/html</value></file></hasFormat>
</ebook>
</rdf:RDF>"#
        );
        assert_eq!(
            process_rdf_xml(xml.as_bytes(), "", false).unwrap_err(),
            "filter_required_formats"
        );
    }

    #[test]
    fn process_rdf_filter_required_formats_missing_html() {
        let xml = format!(
            r#"<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
<ebook rdf:about="ebooks/42">
<type><value>text</value></type>
<title>T</title>
<language><value>en</value></language>
<rights>Public domain</rights>
<hasFormat><file rdf:about="https://example.org/1.epub"><value>application/epub+zip</value></file></hasFormat>
</ebook>
</rdf:RDF>"#
        );
        assert_eq!(
            process_rdf_xml(xml.as_bytes(), "", false).unwrap_err(),
            "filter_required_formats"
        );
    }

    #[test]
    fn process_rdf_filter_creator_no_agents() {
        let xml = full_ebook_xml_with_formats(
            "https://www.gutenberg.org/files/42/42-0.html",
            "https://www.gutenberg.org/files/42/42.epub",
            true,
        );
        assert_eq!(
            process_rdf_xml(xml.as_bytes(), "", false).unwrap_err(),
            "filter_creator"
        );
    }

    // =======================================================================
    // process_rdf_xml success path (full fixture covering all branches)
    // =======================================================================

    fn complete_ebook_xml() -> String {
        format!(
            r#"<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
         xmlns:dcterms="http://purl.org/dc/terms/"
         xmlns:pgterms="http://www.gutenberg.org/2009/pgterms/"
         xmlns:marcrel="http://id.loc.gov/vocabulary/relators/">
         <ebook rdf:about="ebooks/99" xmlns="http://www.gutenberg.org/2009/pgterms/">
           <type><value>text</value></type>
           <title>$a The Complete $b Book</title>
           <language><value>en</value></language>
           <rights>Public domain in the USA.</rights>
           <description>A description text here.</description>
           <alternative>The Alt Title</alternative>
           <issued>1923-04-01</issued>
           <downloads>150</downloads>
           <subject><value>Fiction -- Novel</value></subject>
           <bookshelf><value>Category: Fiction</value></bookshelf>
           <creator>
             <agent rdf:about="http://www.gutenberg.org/ebooks/agents/77">
               <name>John Writer</name>
               <alias>J.W.</alias>
               <birthdate>1870</birthdate>
               <deathdate>1930</deathdate>
               <webpage rdf:resource="https://example.org/john"/>
             </agent>
           </creator>
           <aut>
             <agent rdf:about="http://www.gutenberg.org/ebooks/agents/88">
               <name>Jane Author</name>
             </agent>
           </aut>
           <trl>
             <agent rdf:about="http://www.gutenberg.org/ebooks/agents/55">
               <name>Trans Translator</name>
             </agent>
           </trl>
           <aui>
             <agent rdf:about="http://www.gutenberg.org/ebooks/agents/66">
               <name>Intro Writer</name>
             </agent>
           </aui>
           <ill>
             <agent rdf:about="http://www.gutenberg.org/ebooks/agents/44">
               <name>Illus Artist</name>
             </agent>
           </ill>
           <edt>
             <agent rdf:about="http://www.gutenberg.org/ebooks/agents/33">
               <name>Editor Pro</name>
             </agent>
           </edt>
           <hasFormat>
             <file rdf:about="https://www.gutenberg.org/files/99/99-0.txt">
               <value>text/html</value>
             </file>
           </hasFormat>
           <hasFormat>
             <file rdf:about="https://www.gutenberg.org/files/99/99.epub">
               <value>application/epub+zip</value>
             </file>
           </hasFormat>
         </ebook>
         </rdf:RDF>"#
        )
    }

    #[test]
    fn process_rdf_full_success_path() {
        let xml = complete_ebook_xml();
        let res = process_rdf_xml(xml.as_bytes(), "https://mirror.example.org/", false);
        assert!(res.is_ok(), "Expected Ok, got {:?}", res);
        let ebook = res.unwrap();
        assert_eq!(ebook.ebook_id, "99");
        assert_eq!(ebook.title, "The Complete Book");
        assert_eq!(ebook.language, "en");
        assert!(ebook.license.contains("Public domain"));
        assert_eq!(ebook.alternative_titles, vec!["The Alt Title"]);
        assert!(ebook.description.is_some());
        assert_eq!(ebook.downloads, 150);
        assert!(!ebook.agents.is_empty());
        let author = ebook.agents.iter().find(|a| a.agent_type == "author");
        assert!(author.is_some());
        assert_eq!(author.unwrap().name, "John Writer");
        assert!(ebook.formats.iter().any(|f| f.mime_type == "text/html"));
        assert!(ebook.formats.iter().any(|f| f.mime_type == "application/epub+zip"));
        assert!(ebook.cover_image.contains("cover.medium.jpg"));
        assert!(!ebook.taxonomy.domain.is_empty());
    }

    #[test]
    fn process_rdf_full_success_aut_fallback_when_no_creator() {
        let xml = format!(
            r#"<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#" xmlns:pgterms="http://www.gutenberg.org/2009/pgterms/">
            <ebook rdf:about="ebooks/123" xmlns="http://www.gutenberg.org/2009/pgterms/">
              <type><value>text</value></type>
              <title>Aut Only</title>
              <language><value>en</value></language>
              <rights>Public domain in the USA.</rights>
              <aut>
                <agent rdf:about="http://www.gutenberg.org/ebooks/agents/10">
                  <name>Aut Writer</name>
                </agent>
              </aut>
              <hasFormat>
                <file rdf:about="https://www.gutenberg.org/files/123/123-0.html"><value>text/html</value></file>
              </hasFormat>
              <hasFormat>
                <file rdf:about="https://www.gutenberg.org/files/123/123.epub"><value>application/epub+zip</value></file>
              </hasFormat>
            </ebook>
            </rdf:RDF>"#
        );
        let res = process_rdf_xml(xml.as_bytes(), "", false);
        assert!(res.is_ok(), "Expected Ok for aut fallback, got {:?}", res);
        let ebook = res.unwrap();
        assert!(ebook
            .agents
            .iter()
            .any(|a| a.agent_type == "author" && a.name == "Aut Writer"));
    }

    // =======================================================================
    // Additional branch / line coverage helpers
    // =======================================================================

    #[test]
    fn process_rdf_format_inference_by_extension_only() {
        let xml = format!(
            r#"<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#" xmlns:pgterms="http://www.gutenberg.org/2009/pgterms/">
            <ebook rdf:about="ebooks/1" xmlns="http://www.gutenberg.org/2009/pgterms/">
              <type><value>text</value></type>
              <title>T</title>
              <language><value>en</value></language>
              <rights>Public domain</rights>
              <creator><agent rdf:about="http://www.gutenberg.org/ebooks/agents/1"><name>W</name></agent></creator>
              <hasFormat><file rdf:about="https://www.gutenberg.org/files/1/1.html"><value>unknown</value></file></hasFormat>
              <hasFormat><file rdf:about="https://www.gutenberg.org/files/1/1.epub"><value>unknown</value></file></hasFormat>
            </ebook>
            </rdf:RDF>"#
        );
        let res = process_rdf_xml(xml.as_bytes(), "", false);
        assert!(res.is_ok());
        let ebook = res.unwrap();
        assert!(ebook.formats.iter().any(|f| f.mime_type == "application/epub+zip"));
        assert!(ebook.formats.iter().any(|f| f.mime_type == "text/html"));
    }
}
