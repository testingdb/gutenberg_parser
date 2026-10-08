---
type: architecture-taxonomy
title: Taxonomy Inference and Classification Mapping
description: LC classification parsing, genre/domain/topic inference from subjects and bookshelves, and deterministic taxonomy construction.
tags: [taxonomy, lc-classification, genre, domain]
verified:
  - by: openwiki/0.7.0
    at: 2026-10-08T08:19:49.366Z
sources:
  - id: openwiki-source-2a737474d86fc75cc9d9694f
    resource: repo://src/config.rs
  - id: openwiki-source-1da28519a35b3d9fde64420c
    resource: repo://src/taxonomy.rs
generated: { by: "pi", at: "2026-10-08T08:19:49.366Z" }
---

`taxonomy` (`src/taxonomy.rs`) transforms raw RDF `subject` and `bookshelf` strings into a `Taxonomy` (`domain`, `genres`, `topics`) (repo://src/taxonomy.rs#L70-L340).

## LC Classification

`parse_lc_code` (from `utils`) maps Library of Congress codes (`D501`, `F350.5`) to `(domain, genre)`. The broad domain (`History`, `Science`, etc.) is added to `lc_domains`. The sub-description is checked with `is_genre_keyword`: if it matches `LCSH_FORM_GENRE_MAP` or the `GENRE_INDICATORS` array, it becomes a genre; otherwise it is preserved as a topic sub-heading (repo://src/taxonomy.rs#L80-L130).

## Subject Headings

Subjects split on ` -- `. The first part is the topic heading. Subsequent parts are subtopics, but genre keywords among them are extracted to `genres` rather than kept as subtopics. This prevents duplicate genre classification from both LC and subject forms (repo://src/taxonomy.rs#L132-L190).

## Bookshelf Inference

Bookshelf labels (`Category: ...`) are stripped with `RE_SHELF_CAT`, matched against `BOOKSHELF_MAP` regexes to infer genres (`Science Fiction & Fantasy`, `Mystery & Crime`, etc.), and skipped for generic labels (`best books ever listings`, `novels`, `general`) (repo://src/taxonomy.rs#L190-L220).

## Domain Resolution

`primary_domain` prefers LC-derived domains (highest frequency, lexicographic tie-break), falls back to the most frequent inferred domain from genres, and defaults to `"General & Uncategorized"`. Topics are deduplicated via normalized lowercase keys (`heading|subtopics`) (repo://src/taxonomy.rs#L220-L260).

Key resources: `repo://src/taxonomy.rs`, `repo://src/config.rs`. Related: [Parsing](parsing.md), [Data Models](models.md), [Pipeline](../features/pipeline.md), [Filtering](../features/filtering.md), [Wikipedia Integration](../integration/wikipedia.md).
