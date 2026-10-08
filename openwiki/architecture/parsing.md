---
type: architecture-parsing
title: RDF/XML Stream Parser and Metadata Extraction
description: Stream parsing of Gutenberg RDF/XML from tar.bz2, agent/format/descriptor extraction, filtering rules, and error codes.
tags: [parser, rdf-xml, filtering, rust]
verified:
  - by: openwiki/0.7.0
    at: 2026-10-08T08:19:49.366Z
sources:
  - id: openwiki-source-fdbf7e7124c50fb2a5a041fb
    resource: repo://src/xml_parser.rs
generated: { by: "pi", at: "2026-10-08T08:19:49.366Z" }
---

The `xml_parser` module (`src/xml_parser.rs`) parses raw XML buffers from `rdf-files.tar.bz2`. `process_rdf_xml` validates UTF-8, builds a `roxmltree` document, and applies strict filtering before returning an `Ebook` (repo://src/xml_parser.rs#L84-L352).

## Filtering Rules

- `filter_type`: `type` must equal `"text"`.
- `filter_title`: Title must be non-empty after `clean_marc_subfields`.
- `filter_language`: Language must be non-empty.
- `filter_license`: Non-public-domain excluded unless `include_licensed` is `true`.
- `filter_required_formats`: Both `text/html` and `application/epub+zip` must be present.
- `filter_creator`: At least one agent entry (`author`, `translator`, etc.) must exist.

## Agent Extraction

`parse_agent` resolves agent nodes (either the parent itself if `agent` tag or first child `agent` element), reads `rdf:about`, `name`, `alias`, `birthdate`, `deathdate`, and `webpage`. `agent_id` is parsed from the `about` URL via regex. When `wiki_images` is enabled, agents whose `webpages` include Wikipedia are enriched with a thumbnail URL (repo://src/xml_parser.rs#L34-L130).

## Format and Metadata Extraction

Formats are read from `hasFormat` → `file` nodes, transformed via `transform_url`, and mapped to MIME types (`text/html`, `application/epub+zip`). Cover image URLs and download counts are derived directly from XML nodes. Description is read from `marc520`, `marc500`, or `description`. Alternative titles come from `alternative` nodes after MARC cleaning (repo://src/xml_parser.rs#L200-L350).

## Failure Behavior

Errors are returned as static `&'static str` codes (`utf8_error`, `xml_parse_error`, `filter_type`, etc.) rather than exceptions, allowing the pipeline to skip invalid entries without terminating the worker pool.

Key resources: `repo://src/xml_parser.rs`.
