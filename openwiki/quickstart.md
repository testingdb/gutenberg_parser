---
type: "Reference"
title: "Quickstart"
openwiki_generated: true
verified:
  - by: openwiki/0.7.0
    at: 2026-10-08T08:19:49.366Z
sources:
  - id: openwiki-source-23775c3de52f3ab95a13cb8b
    resource: repo://README.md
generated: { by: "pi", at: "2026-10-08T08:19:49.366Z" }
---


This repository (`gutenberg_parser`) is a multi-threaded Rust CLI for parsing Project Gutenberg RDF/XML archives.

## Key Systems

- **Pipeline & CLI** (`openwiki/features/pipeline.md`): Download, multi-threaded parsing, chunked output.
- **Parsing** (`openwiki/architecture/parsing.md`): RDF/XML extraction, filtering rules.
- **Taxonomy** (`openwiki/architecture/taxonomy.md`): LC classification and genre/domain inference.
- **Models** (`openwiki/architecture/models.md`): Data structures and bridge schema.
- **Filtering** (`openwiki/features/filtering.md`): Quality/licensing rules.
- **Wikipedia Integration** (`openwiki/integration/wikipedia.md`): Thumbnail resolution.

## How to Navigate

Start with `pipeline.md` for the end-to-end flow, then follow links into `parsing.md`, `models.md`, `taxonomy.md`, and `filtering.md`. Integration details are in `wikipedia.md`.
