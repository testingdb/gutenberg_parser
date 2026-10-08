---
type: "Reference"
title: "RDF/XML Record Extraction and Acceptance Filters"
openwiki_generated: true
verified:
  - by: openwiki/0.7.1
    at: 2026-10-08T08:38:59.748Z
sources:
  - id: openwiki-source-c38906bbfa9e9c69417b11b5
    resource: repo://src/cli.rs
  - id: openwiki-source-fdbf7e7124c50fb2a5a041fb
    resource: repo://src/xml_parser.rs
generated: { by: "opencode", at: "2026-10-08T08:38:59.748Z" }
---


# RDF/XML Record Extraction and Acceptance Filters

`process_rdf_xml` is the unit of work in this crate: one archive entry in, one `Ebook` out, or one of ten `&'static str` rejection codes. It is a pure function — no I/O, no shared state except the Wikipedia cache — which is what makes it safe to call concurrently from every worker thread (repo://src/xml_parser.rs#L157-L162).

The signature carries three configuration values down from the CLI: `mirror_base`, `include_licensed`, and `wiki_images`. Rejection is expressed as an error code rather than a partially built record, so the caller can skip an entry without inspecting it.

## The filter sequence

Filters run in a fixed order, and the first failure wins. That order matters for diagnosis: a non-text record with no title reports `filter_type`, never `filter_title` (repo://src/xml_parser.rs#L157-L377).

| Step | Check | Rejection code |
| :--- | :--- | :--- |
| 1 | Byte buffer decodes as UTF-8 | `utf8_error` |
| 2 | `roxmltree` parses the document | `xml_parse_error` |
| 3 | A child element named `ebook` exists | `no_ebook_element` |
| 4 | First `type` descendant `value` is `text` (lowercased) | `filter_type` |
| 5 | `title` non-empty after `clean_marc_subfields` | `filter_title` |
| 6 | `language` child `value` non-empty | `filter_language` |
| 7 | `rights` contains "public domain" unless `include_licensed` | `filter_license` |
| 8 | Both `text/html` and `application/epub+zip` present | `filter_required_formats` |
| 9 | At least one agent was extracted | `filter_creator` |

Two filters do more than reject. `type` must equal exactly `text`, which drops `audio` and `video` releases (repo://src/xml_parser.rs#L178-L189). The license filter defaults an absent `rights` node to `"Public domain in the USA."`, so a missing license is treated as permissive rather than disqualifying (repo://src/xml_parser.rs#L211-L220).

The format requirement is stricter than it may appear: an ebook is only accepted if it offers **both** a `text/html` and an `application/epub+zip` download. Records with only one of the two are dropped, which is the single largest source of rejections alongside the license filter.

## Ebook identity

The ebook ID is derived from the `rdf:about` attribute by stripping everything up to and including `ebooks/` — so `ebooks/1342` yields the string `"1342"`. It is kept as a `String`, not parsed to an integer, and is later re-parsed (defaulting to `0`) only in bridge mode (repo://src/xml_parser.rs#L173-L175, repo://src/models.rs#L301-L319).

## Agent extraction and role mapping

Agents are collected by a `push_agents` closure invoked once per RDF role tag, which maps the feed's abbreviations onto the role strings that appear in output (repo://src/xml_parser.rs#L275-L300):

| RDF tag | `type` in JSON |
| :--- | :--- |
| `creator` | `author` |
| `trl` | `translator` |
| `aui` | `introduction_author` |
| `ill` | `illustrator` |
| `edt` | `editor` |
| `aut` | `author` (fallback only) |

`aut` is consulted only when no `creator` produced an author, so a record carrying both keeps its `creator` and drops the `aut` duplicate rather than listing the same person twice (repo://src/xml_parser.rs#L293-L296). Order of appearance in the document is preserved.

`parse_agent` resolves the agent node in two steps: it uses the parent node itself when that node *is* a `pgterms:agent`, otherwise it takes the first child element named `agent`. This lets the same function serve both a bare `agent` fixture and a `creator`-wrapped one (repo://src/xml_parser.rs#L54-L63). It then reads:

- `agent_id` — parsed from the `rdf:about` URI with `RE_AGENT_ID` (`agents/(\d+)`), optional.
- `name` — required; an agent with an empty name is dropped silently and never becomes a rejection.
- `alias` children — all non-empty aliases.
- `webpage` children — each `rdf:resource` run through `transform_url`, empties discarded.
- `birthdate` / `deathdate` — optional raw text, kept as strings.

Only when `wiki_images` is set does the agent get an `image`, resolved from the first Wikipedia entry in its `webpages` — see [Wikipedia Agent Image Enrichment](../integrations/wikipedia-enrichment.md).

## Formats, cover, and remaining fields

Formats come from `hasFormat` → `file` nodes. The MIME type is inferred from the file's `value` text when possible and otherwise from the transformed URL's extension, and a `HashSet` of seen types ensures only the first URL for each MIME type is kept (repo://src/xml_parser.rs#L226-L268). The same `seen_mime` set is what the mandatory-formats check consults.

The cover image is not read from the feed at all: it is constructed as `https://www.gutenberg.org/cache/epub/{id}/pg{id}.cover.medium.jpg` and then passed through `transform_url`, so it follows the selected mirror (repo://src/xml_parser.rs#L302-L311).

The remaining optional fields each have a specific source, and none of their absence is an error:

- `description` — the first of `marc520`, `marc500`, or `description` that appears.
- `alternative_titles` — all `alternative` children, MARC-cleaned.
- `issued_date` — `issued` text, unparsed.
- `downloads` — `downloads` parsed as `u64`, defaulting to `0`.
- `subjects` / `bookshelves` — collected as raw `value` strings and handed to the taxonomy engine, which is the only consumer (repo://src/xml_parser.rs#L344-L361).

Related pages: [Pipeline Orchestration and Threading Model](../pipeline/pipeline.md), [Taxonomy Inference Engine](../taxonomy/taxonomy-inference.md), [Record Model and Bridge Schema Mapping](../records/data-models.md), [Mirror Selection and URL Transformation](mirror-urls.md).

Key resources: `repo://src/xml_parser.rs`.
