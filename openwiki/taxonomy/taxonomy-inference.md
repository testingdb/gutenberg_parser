---
type: architecture-taxonomy
title: Taxonomy Inference Engine
description: How raw RDF subject and bookshelf strings become a domain, a genre set, and structured topics, including the LC-versus-heading branch split and domain resolution priority.
tags: [taxonomy, classification, inference, lc-classification, genres]
verified:
  - by: openwiki/0.7.1
    at: 2026-10-08T08:38:59.748Z
sources:
  - id: openwiki-source-1da28519a35b3d9fde64420c
    resource: repo://src/taxonomy.rs
  - id: openwiki-source-0993c2c0112ac99cf6999828
    resource: repo://src/utils.rs
generated: { by: "opencode", at: "2026-10-08T08:38:59.748Z" }
---

# Taxonomy Inference Engine

Gutenberg metadata has no domain field. `extract_taxonomy` synthesizes one from two untrusted string sources — LC subject headings and bookshelf labels — and is the sole producer of `Taxonomy` (repo://src/taxonomy.rs#L147-L284). It is pure and thread-safe: it reads only the static tables, keeps all intermediate state in locals, and touches no shared mutable structure, so every worker calls it concurrently without coordination.

Its signature is deliberately narrow: `&[String]` subjects and `&[String]` bookshelves in, one `Taxonomy` out.

## Two kinds of subject line

Each subject string is routed one of two ways, and the routing decision is what determines whether a string becomes a domain, a genre, or a topic (repo://src/taxonomy.rs#L154-L209).

**The LC-code branch.** If `parse_lc_code` recognizes the string as a Library of Congress code, the code resolves to a `(domain, sub-description)` pair. The domain is recorded as *LC evidence*, which is the stronger kind. The sub-description is then routed by `is_genre_keyword`: if it reads as a form or genre term it becomes a genre, and otherwise it becomes a topic under the domain heading. So `D501` yields domain `History` plus topic `History → World War I`, while `CT` yields domain `History` plus genre `Biography` with no topic at all.

`is_genre_keyword` first tries a direct lowercase hit in `LCSH_FORM_GENRE_MAP`, then falls back to a normalized substring scan against `GENRE_INDICATORS` — LCSH keys plus about 70 supplementary form terms built once into a `LazyLock<Vec<String>>` (repo://src/taxonomy.rs#L29-L136). Normalization replaces every non-alphanumeric, non-space character with a space, so `"Memoirs,"` and `"Memoirs - Essays"` still match. This is why trailing punctuation in an LC sub-description does not suppress the genre.

**The subject-heading branch.** Anything `parse_lc_code` rejects — and that includes the majority of real headings — is split on the literal `" -- "` separator. The first part is the heading; the rest are subtopics, with empty segments dropped. Any part, heading or subtopic, that hits `LCSH_FORM_GENRE_MAP` is diverted into genres *and* contributes inferred domain evidence; it is excluded from the topic's subtopic list (repo://src/taxonomy.rs#L178-L206).

So `"Fiction -- Novel"` produces genre `Fiction & Novels` plus topic `Fiction → [Novel]`: `Fiction` was consumed as a genre keyword, `Novel` was not in the LCSH map and was kept as a subtopic. The distinction between the two branches matters most for headings that resemble codes — `"123"` or `"XYZ123"` fail LC validation, fall to the heading branch, and become topics with an unresolved domain rather than being discarded.

## Bookshelf inference

Shelf labels are lowercased after `RE_SHELF_CAT` strips a `Category:` prefix, then every regex in `BOOKSHELF_MAP` is tested against the result (repo://src/taxonomy.rs#L212-L226). Three labels are explicitly skipped because they classify nothing: `best books ever listings`, `novels`, and `general`. Shelves contribute genres only, never topics.

## Domain resolution

Two candidate pools accumulate during the pass — LC-derived and inferred — and the resolver prefers the first whenever it is non-empty (repo://src/taxonomy.rs#L236-L255):

1. If any LC evidence exists, choose from it.
2. Otherwise choose from inferred evidence, which includes domains contributed both by LCSH hits and, after genres are known, by `GENRE_TO_DOMAIN_MAP` lookups.
3. If both are empty, emit `"General & Uncategorized"`.

Within a pool, domains are counted and the winner is the most frequent, with ties broken lexicographically. LC evidence therefore beats inferred evidence outright rather than competing by count: a record with LC `History` and a bookshelf genre implying `Language & Literature` stays `History` (repo://src/taxonomy.rs#L239-L246).

The lexicographic tie-break has a visible consequence worth knowing: among equal-frequency inferred domains, `"History"` sorts before `"Language & Literature"` and wins, which is why several tests assert `History` for inputs that could plausibly resolve either way.

## Topics and genres, finalized

Topics are deduplicated by a normalized key — lowercase heading joined with the subtopics **sorted** and comma-separated — so `"Science -- Mathematics"` appearing twice yields one topic, while `"Science -- Mathematics"` and `"Science -- Physics"` both survive as separate topics (repo://src/taxonomy.rs#L258-L273). Topic order follows first appearance, not sorting.

Genres live in a `HashSet` for the whole pass and are sorted lexicographically at the end, which is what makes the serialized output byte-identical across runs for identical input (repo://src/taxonomy.rs#L275-L277). A record can carry several genres from different sources and nothing reconciles them: `CT` plus a `Biograph` bookshelf yields both `Biography` and `Biography & Memoir` as separate genres (repo://src/taxonomy.rs#L599-L607).

The result is always a complete `Taxonomy` — `domain` is never empty and `genres`/`topics` are always vectors, which is what lets the model guarantee a non-null `taxonomy.domain` for every accepted record.

Related pages: [Reference Tables, Regexes, and Namespace Constants](reference-tables.md), [RDF/XML Record Extraction and Acceptance Filters](../ingestion/rdfextraction.md), [Record Model and Bridge Schema Mapping](../records/data-models.md).

Key resources: `repo://src/taxonomy.rs`, `repo://src/config.rs`, `repo://src/utils.rs`.
