---
type: architecture-models
title: Record Model and Bridge Schema Mapping
description: The serialized Ebook record and its nested types, the three serde presence classes that stabilize the JSON contract, and how bridge mode renames fields for the target database.
tags: [models, serde, json, schema, bridge]
verified:
  - by: openwiki/0.7.1
    at: 2026-10-08T08:38:59.748Z
sources:
  - id: openwiki-source-a5d2f77c02ea33206b391c3a
    resource: repo://src/models.rs
generated: { by: "opencode", at: "2026-10-08T08:38:59.748Z" }
---

# Record Model and Bridge Schema Mapping

`models.rs` defines the output contract and nothing else — no parsing, no inference. Every field's presence in JSON is decided here by serde attributes, which is why this module is the authority on the shape consumers can rely on (repo://src/models.rs#L1-L17).

## Record types

Four parser-mode types make up a record (repo://src/models.rs#L30-L170):

- **`Ebook`** — the top-level record: `title`, `alternative_titles`, `issued_date`, `agents`, `description`, `language`, `formats`, `taxonomy`, `downloads`, `ebook_id`, `cover_image`, `license`.
- **`Agent`** — a contributor: `agent_type` (serialized as `type`), `agent_id`, `name`, `aliases`, `webpages`, `birth_date`, `death_date`, `image`.
- **`Format`** — a download: `mime_type` (serialized as `type`) and `url`.
- **`Taxonomy`** / **`Topic`** — `domain`, `genres`, and `topics`, where each topic is a `heading` plus `subtopics`.

Two fields carry non-obvious types. `ebook_id` is a `String` end to end in parser mode, even though it is always numeric in practice; `downloads` is a `u64` that defaults to `0` when the RDF node is missing or unparseable.

## The three presence classes

The JSON contract rests on a strict division, and every optional field follows exactly one of these rules:

**Always present, possibly empty.** Fields marked `#[serde(default)]` on a `Vec` are always serialized as `[]` when empty — never omitted, never `null`. That covers `alternative_titles`, `agents`, `aliases`, `webpages`, `taxonomy.genres`, `taxonomy.topics`, and `topics[].subtopics`. A consumer can index into them unconditionally (repo://src/models.rs#L44-L51, repo://src/models.rs#L112-L117).

**Present only when a value exists.** Fields marked `#[serde(skip_serializing_if = "Option::is_none")]` are dropped from JSON entirely when `None`: `issued_date`, `description`, `agent_id`, `birth_date`, `death_date`, and `image`. Their absence means "unknown", and it is indistinguishable from never having been looked up — which is why `image` cannot signal that enrichment was switched off (repo://src/models.rs#L138-L141, repo://src/models.rs#L60-L65).

**Always present and non-null.** `title`, `language`, `formats`, `taxonomy.domain`, `downloads`, `ebook_id`, `cover_image`, and `license` have no skipping attribute. This is guaranteed by construction: the parser rejects any record that would leave them empty (see [RDF/XML Record Extraction and Acceptance Filters](../ingestion/rdfextraction.md)).

Note the asymmetry in `Ebook`: `agents` has `#[serde(default)]` yet is also required to be non-empty by the parser, so the attribute is a type-level convenience that the parser guarantees is never exercised as an empty array.

## The `type` rename

`Agent.agent_type` and `Format.mime_type` are both serialized as the JSON key `type` (repo://src/models.rs#L33-L34, repo://src/models.rs#L77-L78). The Rust field names exist only because `type` is a keyword. Bridge mode does *not* keep this rename for agents: `BridgeAgent.role` serializes as `role`, while `BridgeFormat.mime_type` keeps the name `mime_type`. So the two modes disagree on that one key — a real asymmetry for any consumer reading both.

## Bridge mode

`BridgeEbook` is a parallel struct with renamed fields for the target database schema, produced by `From<&Ebook>` (repo://src/models.rs#L223-L320):

| Parser mode | Bridge mode | Note |
| :--- | :--- | :--- |
| `language` | `lang_code` | |
| `downloads` | `pg_download_count` | |
| `ebook_id` (String) | `pg_id` (u64) | re-parsed, **defaults to `0`** on failure |
| `cover_image` | `md_cover_image_url` | |
| `license` | `license_statement` | |
| `agents[].type` | `agents[].role` | |
| `agents[].agent_id` | `agents[].pg_id` | |
| `agents[].webpages` | `agents[].external_urls` | |
| `formats[].type` | `formats[].mime_type` | rename reversed |
| `formats[].url` | `formats[].file_url` | |
| `title`, `alternative_titles`, `issued_date`, `description`, `formats`, `agents` | same | unchanged |
| `taxonomy` | `taxonomy` | passed through by clone, identical shape |

The `pg_id` conversion is the only lossy one: a non-numeric `ebook_id` silently becomes `0` rather than failing, so bridge output can contain a placeholder ID that collides across records. Everything else is a mechanical rename with no loss.

Presence semantics are preserved exactly — `BridgeAgent` repeats the same `skip_serializing_if` and `default` attributes, so a bridge consumer sees the same three classes, including `image` carrying through under its original name (repo://src/models.rs#L181-L211).

Related pages: [RDF/XML Record Extraction and Acceptance Filters](../ingestion/rdfextraction.md), [Chunked JSON Output and Serialization Guarantees](../pipeline/json-output.md), [Taxonomy Inference Engine](../taxonomy/taxonomy-inference.md).

Key resources: `repo://src/models.rs`, `repo://src/xml_parser.rs`, `repo://README.md`.
