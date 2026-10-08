---
type: features-selection
title: Record Selection Policy and Its Consequences
description: Which ebooks end up in the output as a dataset policy, what that guarantees downstream, and why adding --include-licensed changes the population.
tags: [features, filtering, policy, quality, dataset]
verified:
  - by: openwiki/0.7.1
    at: 2026-10-08T08:57:55.085Z
sources:
  - id: openwiki-source-23775c3de52f3ab95a13cb8b
    resource: repo://README.md
  - id: openwiki-source-c38906bbfa9e9c69417b11b5
    resource: repo://src/cli.rs
  - id: openwiki-source-a5d2f77c02ea33206b391c3a
    resource: repo://src/models.rs
  - id: openwiki-source-1da28519a35b3d9fde64420c
    resource: repo://src/taxonomy.rs
  - id: openwiki-source-fdbf7e7124c50fb2a5a041fb
    resource: repo://src/xml_parser.rs
generated: { by: "opencode", at: "2026-10-08T08:57:55.085Z" }
---

# Record Selection Policy and Its Consequences

The parser is not a lossless reader of the feed. Roughly nine checks decide whether an ebook becomes a JSON record or is dropped, and the set of survivors is a deliberately curated corpus: text works, public domain, downloadable in two formats, and attributable to at least one named contributor.

This page describes that policy as a product decision. The mechanics — the order of the checks and the code each returns — are in [RDF/XML Record Extraction and Acceptance Filters](../ingestion/rdfextraction.md).

## What the default corpus contains

| Criterion | Rule | Why it is there |
| :--- | :--- | :--- |
| Medium | `type` is exactly `text` | Drops `audio` and `video` releases, which have no text record to describe |
| Rights | License text contains "public domain" | Keeps the corpus redistributable by construction |
| Formats | Both `text/html` **and** `application/epub+zip` present | Guarantees every record is usable in two independent readers |
| Attribution | At least one agent with a non-empty name | No anonymous, unattributable records |
| Identity | Non-empty title after MARC cleaning, non-empty language | A record must be identifiable and have a language |

Two of these are quality bars rather than exclusions. The format requirement is the strictest: a work offered only as plain text, or as EPUB without HTML, is dropped even though it is perfectly valid. That is the single largest source of exclusions and it exists so that no consumer has to handle a record with a missing format.

Attribution has a subtlety worth knowing: an agent whose `name` is empty is discarded *before* the count is taken, so a record with only nameless agents is rejected by `filter_creator` rather than emitted with an empty contributor list (repo://src/xml_parser.rs#L80-L82).

## What this guarantees downstream

Because acceptance is a precondition rather than a post-hoc cleanup, several things are true of every emitted record — properties a consumer can rely on without defensive checks:

- **`agents` is never empty.** The at-least-one-agent rule holds before the record is built, so the model's `#[serde(default)]` on that vector is never exercised as `[]` in practice (repo://src/xml_parser.rs#L298-L300).
- **`taxonomy.domain` is never empty.** Domain resolution always terminates in a real domain or the literal `"General & Uncategorized"`, so an empty domain string cannot appear (repo://src/taxonomy.rs#L239-L255).
- **`cover_image` is a constructed URL, not feed data.** No RDF node provides it; the medium cover path is synthesized from the ebook ID and passed through mirror rewriting. It is therefore always present and always well-formed, but its existence says nothing about whether the image actually exists on the mirror (repo://src/xml_parser.rs#L302-L311).
- **`license` is always meaningful.** Either the feed's rights text or the explicit public-domain default; it is never absent (repo://src/xml_parser.rs#L211-L216).
- **`downloads` may legitimately be `0`.** A missing or unparseable `downloads` node defaults to zero rather than rejecting the record, so `0` means "unknown", not "never downloaded" (repo://src/xml_parser.rs#L337-L342).
- **`alternative_titles`, `genres`, and `topics` may be `[]`.** Nothing in the policy requires a subtitle, a genre, or a subject heading, so an accepted record can legitimately have none (repo://src/models.rs#L134-L137).

## Changing the population

`--include-licensed` relaxes exactly one criterion — the rights check — while leaving the other eight untouched (repo://src/xml_parser.rs#L218-L220). The result is strictly a superset: every record a default run would have emitted is still emitted, plus the licensed ones. It does not, however, change the guarantee set, because the other criteria are unchanged.

The tool defaults to the narrower population, which means the default output is a redistributable corpus. Widening it is a deliberate act by the operator, and the `--bridge` mode exists to load such a corpus into a target database.

**The practical caveat:** because selection is a filter, changing it changes the population. A corpus built with `--include-licensed` is not comparable line-for-line with a default one, and a corpus built before a filter change is not comparable with one built after. There is no version stamp in the output recording which filters were active, so the flags used must be tracked outside the data.

Related pages: [RDF/XML Record Extraction and Acceptance Filters](../ingestion/rdfextraction.md), [Record Model and Bridge Schema Mapping](../records/data-models.md), [Taxonomy Inference Engine](../taxonomy/taxonomy-inference.md), [Running the Parser: Inputs, Options, and Runtime Cost](../operations/running-the-parser.md).

Key resources: `repo://src/xml_parser.rs`, `repo://src/models.rs`, `repo://src/taxonomy.rs`, `repo://README.md`.
