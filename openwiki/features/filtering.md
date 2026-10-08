---
type: "Reference"
title: "Filtering"
openwiki_generated: true
verified:
  - by: openwiki/0.7.0
    at: 2026-10-08T08:19:49.366Z
sources:
  - id: openwiki-source-23775c3de52f3ab95a13cb8b
    resource: repo://README.md
  - id: openwiki-source-c38906bbfa9e9c69417b11b5
    resource: repo://src/cli.rs
  - id: openwiki-source-fdbf7e7124c50fb2a5a041fb
    resource: repo://src/xml_parser.rs
generated: { by: "pi", at: "2026-10-08T08:19:49.366Z" }
---


Filtering is enforced in `process_rdf_xml` (`src/xml_parser.rs`) and configured via CLI flags (`Args` in `src/cli.rs`) (repo://README.md, repo://src/cli.rs#L40-L110).

## Required Conditions

- `filter_type`: Only `type == "text"` (audio/video excluded).
- `filter_title`: Title non-empty after MARC cleaning (`clean_marc_subfields`).
- `filter_language`: Language non-empty.
- `filter_required_formats`: Both `text/html` and `application/epub+zip` must exist.
- `filter_creator`: At least one agent entry (author/translator/etc.) must exist.

## Licensing Filter

By default, only public-domain ebooks are included. `license` text is checked with `is_public_domain_license`. Passing `--include-licensed` disables this exclusion (`repo://src/xml_parser.rs#L150-L170`, `repo://src/cli.rs#L70-L80`).

## Quality Invariants

- Non-text entries are silently excluded (error code `filter_type`).
- Entries missing required format types are excluded (`filter_required_formats`).
- Entries without any agent are excluded (`filter_creator`).
- The pipeline continues processing other entries after any filter failure; workers never abort the pool.

Key resources: `repo://README.md`, `repo://src/cli.rs`, `repo://src/xml_parser.rs`.
