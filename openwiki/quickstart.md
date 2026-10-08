---
type: "Reference"
title: "Quickstart"
sources:
  - id: openwiki-source-23775c3de52f3ab95a13cb8b
    resource: repo://README.md
generated: { by: "opencode", at: "2026-10-08T08:57:55.085Z" }
verified:
  - by: openwiki/0.7.1
    at: 2026-10-08T08:57:55.085Z
---

This repository (`gutenberg_parser`) is a multi-threaded Rust CLI for parsing Project Gutenberg RDF/XML archives.

## Systems

- **[Module Dependency Graph and Change Seams](architecture/module-graph.md)** — the seven-module chain, what each layer owns, which files a change crosses.
- **[Pipeline Orchestration and Threading Model](pipeline/pipeline.md)** — producer, worker pool, and consumer wired through two bounded channels; how a run terminates.
- **[RDF/XML Record Extraction and Acceptance Filters](ingestion/rdfextraction.md)** — how one archive entry becomes an `Ebook`, and every rejection code.
- **[Mirror Selection and URL Transformation](ingestion/mirror-urls.md)** — rewriting canonical Gutenberg URLs for the chosen mirror.
- **[Taxonomy Inference Engine](taxonomy/taxonomy-inference.md)** — subject and bookshelf strings to domain, genres, and topics.
- **[Reference Tables, Regexes, and Namespace Constants](taxonomy/reference-tables.md)** — the static data every other module reads.
- **[Record Model and Bridge Schema Mapping](records/data-models.md)** — the serialized record and its three presence classes.
- **[Chunked JSON Output and Serialization Guarantees](pipeline/json-output.md)** — chunk naming, gzip by suffix, per-chunk ordering.
- **[Wikipedia Agent Image Enrichment](integrations/wikipedia-enrichment.md)** — the only outbound network path during parsing.
- **[Record Selection Policy and Its Consequences](features/record-selection.md)** — which ebooks survive the filters and what that guarantees.
- **[Project Gutenberg RDF Feed Contract](integration/gutenberg-feed.md)** — the upstream feed, its element vocabulary, and how silently it could drift.

## Operations and Development

- **[Running the Parser: Inputs, Options, and Runtime Cost](operations/running-the-parser.md)** — provisioning, flags, and what each option costs.
- **[CI Quality Gates, Dependency Audit, and Release](operations/delivery-pipeline.md)** — what gates a change and what publishes binaries.
- **[Test Suite Layout and Local Verification](development/verification.md)** — where tests live and the commands that mirror CI.

## Task Routes

| Question or change | Start here |
| :--- | :--- |
| Why was my ebook dropped? | [Acceptance filters](ingestion/rdfextraction.md) — nine ordered checks, each with its own error code |
| Understand module coupling before editing | [Module graph](architecture/module-graph.md) — the change-seam table says which files a given edit crosses |
| Change what gets selected | [Selection policy](features/record-selection.md) for the consequences, then [filters](ingestion/rdfextraction.md) for the mechanics |
| React to an upstream feed change | [Feed contract](integration/gutenberg-feed.md) — which elements are read and why drift fails silently |
| Change an output field name | [Data models](records/data-models.md), then [JSON output](pipeline/json-output.md) for bridge mode |
| Add a rejection rule | [RDF extraction](ingestion/rdfextraction.md); add a case per filter branch as [tests](development/verification.md) |
| Support a new mirror | [URL transformation](ingestion/mirror-urls.md); the key works without a code change, the table is optional |
| Change the domain or genre vocabulary | [Reference tables](taxonomy/reference-tables.md), consumed by the [taxonomy engine](taxonomy/taxonomy-inference.md) |
| Wire in a new external data source | [Wikipedia enrichment](integrations/wikipedia-enrichment.md) as the pattern for throttled, cached, best-effort lookups |
| Runs are slow or hang | [Pipeline](pipeline/pipeline.md) for threading and termination, [usage](operations/running-the-parser.md) for option costs |
| Verify before pushing | [Local verification](development/verification.md) |
