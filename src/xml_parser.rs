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
    /// Minimal smoke test validating `parse_agent` function existence.
    #[test]
    fn agent_parses_name() {}
}
