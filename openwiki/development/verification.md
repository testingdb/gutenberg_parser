---
type: reference-verification
title: Test Suite Layout and Local Verification
description: Where verification lives in this crate (inline per-module test modules), what the fixtures pin down, and which local commands reproduce the CI gates.
tags: [testing, quality, ci, rust, coverage]
verified:
  - by: openwiki/0.7.1
    at: 2026-10-08T08:38:59.748Z
sources:
  - id: openwiki-source-c1a55afad9a61e65437aa93e
    resource: repo://.github/workflows/pr-gate.yml
  - id: openwiki-source-8b5771729573840865fcd275
    resource: repo://.rustfmt.toml
  - id: openwiki-source-c38906bbfa9e9c69417b11b5
    resource: repo://src/cli.rs
  - id: openwiki-source-1da28519a35b3d9fde64420c
    resource: repo://src/taxonomy.rs
  - id: openwiki-source-0993c2c0112ac99cf6999828
    resource: repo://src/utils.rs
  - id: openwiki-source-fdbf7e7124c50fb2a5a041fb
    resource: repo://src/xml_parser.rs
generated: { by: "opencode", at: "2026-10-08T08:38:59.748Z" }
---

# Test Suite Layout and Local Verification

There is no `tests/` directory and no `benches/` directory. Every test lives in a `#[cfg(test)] mod tests` block at the bottom of the module it verifies, so a test is always next to the code it constrains (repo://src/main.rs#L36-L117, repo://src/utils.rs#L327-L420). Verification of this crate is therefore a source-reading exercise first: to learn what behavior is guaranteed, read the test module of the module that owns the behavior.

## Coverage by module

Test counts differ sharply, and the distribution is itself a fact about the code (repo://src/taxonomy.rs#L290-L667, repo://src/xml_parser.rs#L383-L838):

| Module | Tests | Focus |
| :--- | ---: | :--- |
| `taxonomy` | 42 | Domain priority, genre inference, dedupe, LC code edges |
| `xml_parser` | 23 | One case per filter branch plus the full success fixture |
| `main` | 9 | `clean_marc_subfields` edge cases |
| `utils` | 9 | MARC cleaning, LC parsing, Wikipedia URL/agent helpers |
| `config` | 3 | Table initialization and regex sanity |
| `models` | 3 | Record construction and bridge conversion |
| `cli` | 1 | Chunk file naming |

`taxonomy` and `xml_parser` carry the weight because they hold the branching logic; `cli.rs` has almost none, since orchestration is thin glue around `process_rdf_xml` and `write_chunk` (repo://src/cli.rs#L417-L427).

## How the tests are written

XML fixtures are embedded `String` literals assembled by small helper functions, not files on disk. `base_ebook_xml` builds a minimal valid ebook and `full_ebook_xml_with_formats` adds `hasFormat` nodes, so each rejection test varies one field and asserts the exact error code returned by `process_rdf_xml` — `filter_type`, `filter_title`, `filter_language`, `filter_license`, `filter_required_formats`, `filter_creator` — rather than merely asserting `is_err` (repo://src/xml_parser.rs#L552-L691). Pinning the code string means a rename of a rejection reason becomes a deliberate, visible test change.

`complete_ebook_xml` is the single success-path fixture: it exercises MARC cleaning, alternative titles, description, issued date, downloads, five agent roles, both required formats, and the `aut` fallback, then asserts on the assembled `Ebook` (repo://src/xml_parser.rs#L697-L813).

String-level helpers are tested against the exact failure modes they were written for. `clean_marc_subfields` has dedicated cases for monetary amounts (`$100`, `$5`), dollar-prefixed words (`$billions`), and codes glued to text (`$aThe Title`), because the `\$[a-zA-Z]\b` pattern must not eat those (repo://src/main.rs#L88-L101). `wikipedia_url_pattern_selects_articles_only` asserts both positives (scheme-less and `m.` subdomain forms) and negatives (`https://en.wikipedia.org/wiki` with no trailing segment, a bare host) so the matcher cannot silently widen (repo://src/utils.rs#L389-L399).

## The network test

Exactly one test is marked `#[ignore = "requires network access to the Wikipedia REST API"]`: `parse_agent_image_resolved_from_wikipedia`, which asserts that a resolved thumbnail starts with `http` and is hosted on `wikimedia.org` (repo://src/xml_parser.rs#L503-L521). The companion `parse_agent_image_absent_when_wiki_images_disabled` covers the flag-off path without network. The ignored test runs explicitly:

```bash
cargo test -- --ignored
```

## Local commands that mirror CI

The PR gate runs, per operating system, formatting, linting, dependency hygiene, a locked release build, the test suite, and docs (repo://.github/workflows/pr-gate.yml#L64-L86):

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features --locked -- -D warnings
cargo machete
cargo build --release --locked
cargo test --all-features --locked
cargo doc --no-deps --all-features
```

Warnings are errors under `-D warnings`, so an unused import fails the build. Formatting is governed by a single setting, `max_width = 120` (repo://.rustfmt.toml#L1). Coverage for SonarCloud is produced separately with `cargo llvm-cov --all-features --tests --lcov --output-path lcov.info`, which requires the `llvm-tools-preview` component (repo://.github/workflows/pr-gate.yml#L116-L122).

Dependency hygiene is split: `cargo machete` runs in the PR gate to catch unused crates, while `cargo-audit` and `cargo-deny` (bans, licenses, sources) run on a weekly schedule rather than on every push — see [CI Quality Gates, Dependency Audit, and Release](../operations/delivery-pipeline.md).

Related pages: [RDF/XML Record Extraction and Acceptance Filters](../ingestion/rdfextraction.md), [Taxonomy Inference Engine](../taxonomy/taxonomy-inference.md), [Wikipedia Agent Image Enrichment](../integrations/wikipedia-enrichment.md), [CI Quality Gates, Dependency Audit, and Release](../operations/delivery-pipeline.md).

Key resources: `repo://src/taxonomy.rs`, `repo://src/xml_parser.rs`, `repo://src/utils.rs`, `repo://.github/workflows/pr-gate.yml`.
