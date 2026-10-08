---
type: data-model
title: Data Models and Schema Bridge
description: Core serializable structures (Ebook, Agent, Taxonomy, Format) and the Bridge schema conversion for external database alignment.
tags: [models, serialization, bridge-schema, rust]
verified:
  - by: openwiki/0.7.0
    at: 2026-10-08T08:19:49.366Z
sources:
  - id: openwiki-source-a5d2f77c02ea33206b391c3a
    resource: repo://src/models.rs
generated: { by: "pi", at: "2026-10-08T08:19:49.366Z" }
---

The `models` module (`src/models.rs`) defines the core `Ebook` record and nested types (`Agent`, `Format`, `Topic`, `Taxonomy`). All fields use `serde` annotations (`rename`, `skip_serializing_if`, `default`) to control JSON output and omit optional empty fields.

## Agent

`Agent` represents a contributor (`author`, `translator`, `illustrator`, etc.). `agent_type` is serialized as `"type"`. `agent_id` links to the Gutenberg agent database when present (`skip_serializing_if = "Option::is_none"`). `aliases`, `webpages`, `birth_date`, `death_date`, and `image` are optional; `image` carries the Wikipedia thumbnail when `--wiki-images` is enabled (repo://src/models.rs#L13-L53).

## Format and Topic

`Format` holds `mime_type` (`text/html` or `application/epub+zip`) and the transformed `url`. `Topic` carries a `heading` and optional `subtopics` (repo://src/models.rs#L56-L79).

## Taxonomy

`Taxonomy` contains `domain` (broad category), `genres` (inferred from bookshelf/LC keywords), and structured `topics` from `subject` RDF nodes (repo://src/models.rs#L82-L94).

## Ebook

`Ebook` is the complete record: `title`, `alternative_titles`, `issued_date`, `agents`, `description`, `language`, `formats`, `taxonomy`, `downloads`, `ebook_id`, `cover_image`, and `license`. Optional fields are omitted from JSON when `None` or empty (repo://src/models.rs#L97-L137).

## Bridge Schema

When `--bridge` is set, `BridgeEbook` renames fields (`lang_code`, `pg_download_count`, `md_cover_image_url`, `license_statement`) and converts nested agents (`BridgeAgent`: `role`, `pg_id`, `external_urls`) and formats (`BridgeFormat`: `file_url`). `BridgeEbook` parses `ebook_id` into `pg_id` (`u64`, defaulting to `0` on failure) (repo://src/models.rs#L140-L266).

## Serialization Invariants

- Optional fields are omitted from JSON when `None` or empty.
- `BridgeAgent::image` survives conversion and is omitted when `None`.
- `BridgeEbook::from` clones all strings; no mutation of source `Ebook` occurs.

Related: [Parsing](parsing.md), [Taxonomy](taxonomy.md), [Pipeline](../features/pipeline.md), [Filtering](../features/filtering.md), [Wikipedia Integration](../integration/wikipedia.md).

Key evidence resources: `repo://src/models.rs`.
