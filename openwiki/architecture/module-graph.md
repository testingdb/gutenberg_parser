---
type: architecture-structure
title: Module Dependency Graph and Change Seams
description: The seven-module dependency chain in src/, what each layer owns, and which seams a typical change has to cross.
tags: [architecture, modules, dependencies, structure, rust]
verified:
  - by: openwiki/0.7.1
    at: 2026-10-08T08:57:55.085Z
sources:
  - id: openwiki-source-c38906bbfa9e9c69417b11b5
    resource: repo://src/cli.rs
  - id: openwiki-source-b55a21a31ede1b56cd31a6a6
    resource: repo://src/main.rs
  - id: openwiki-source-a5d2f77c02ea33206b391c3a
    resource: repo://src/models.rs
  - id: openwiki-source-1da28519a35b3d9fde64420c
    resource: repo://src/taxonomy.rs
  - id: openwiki-source-0993c2c0112ac99cf6999828
    resource: repo://src/utils.rs
  - id: openwiki-source-fdbf7e7124c50fb2a5a041fb
    resource: repo://src/xml_parser.rs
generated: { by: "opencode", at: "2026-10-08T08:57:55.085Z" }
---

# Module Dependency Graph and Change Seams

The crate is seven modules in one binary with no library target and no integration tests. Its dependency structure is a strict chain with two leaves, which makes the codebase easy to hold in your head — and means there is exactly one direction in which a change can propagate (repo://src/main.rs#L21-L26).

## The chain

Each module imports only the ones listed, and `main` imports none of them by name; it declares them and calls `cli::run` (repo://src/main.rs#L21-L34).

| Module | Depends on | Owns |
| :--- | :--- | :--- |
| `config` | — | Static tables, regexes, endpoint constants |
| `models` | — | Record types, serde presence rules, bridge conversion |
| `utils` | `config` | MARC cleaning, license check, URL rewriting, LC parsing, Wikipedia lookups |
| `taxonomy` | `config`, `models`, `utils` | Subject/bookshelf to domain, genres, topics |
| `xml_parser` | `config`, `models`, `taxonomy`, `utils` | One RDF/XML buffer to one `Ebook`, or a rejection code |
| `cli` | `config`, `models`, `xml_parser` | Arguments, archive acquisition, threading, output writing |
| `main` | declares all | Nothing beyond delegating to `cli::run` |

Two properties are worth naming explicitly:

- **No cycles.** The graph is acyclic, so any function can be reasoned about in isolation and unit-tested without constructing its callers.
- **Coarse coupling by glob.** Every cross-module import is `use crate::x::*` (repo://src/cli.rs#L25-L39). Any `pub` item in `config` or `models` is therefore in scope in every consumer, so the table and model modules function as de facto shared libraries without being declared as one.

## Two leaf layers with no shared owner

`config` and `models` sit at the bottom and depend on nothing internal — `config` on `regex` alone, `models` on `serde` alone. Everything above them reads from these two.

That split has a consequence worth planning around: **no single module owns the JSON contract.** The guarantees a consumer relies on — arrays always present, optionals omitted, required fields non-null — come from `models`' serde attributes *and* from `xml_parser`'s rejection filters, and the two are maintained in different files by the same reasoning but not by any shared assertion. See [Record Model and Bridge Schema Mapping](../records/data-models.md) and [RDF/XML Record Extraction and Acceptance Filters](../ingestion/rdfextraction.md).

## Change seams

The chain predicts which tasks are one-file and which are multi-file:

| Change | Files it crosses |
| :--- | :--- |
| Add a lookup table or regex | `config`, plus the one consumer (`utils`, `taxonomy`, or `xml_parser`) |
| Add a rejection rule | `xml_parser`, plus a test case per branch in the same file |
| Add a record field | `models`, `xml_parser` (populate it), `models` (`BridgeEbook` rename or pass-through), and the output contract |
| Add a taxonomy source | `taxonomy`, plus a table in `config` |
| Add an enrichment source (like Wikipedia) | `utils`, plus the `wiki_images` flag threaded from `cli` → `xml_parser` → `utils` |
| Change output writing | `cli` only |

Two patterns recur in that table. New enrichment is the expensive kind: the boolean is threaded through three modules as a plain argument rather than through a config struct, so adding a third source means a fourth parameter on `process_rdf_xml` and every test that calls it. New record fields are the other kind: a field must be added to both the parser and bridge structs and populated in one place.

## No abstraction over the stages

There is no trait for "a pipeline stage" and no pluggable source or sink. The producer, worker, and consumer are inline closures and blocks in `cli::run`, wired with concrete `crossbeam` channels (repo://src/cli.rs#L310-L356). Swapping the input format or the output format means editing `run`, not implementing a trait. This is a reasonable trade for a single-purpose binary, and it is why [Pipeline Orchestration and Threading Model](../pipeline/pipeline.md) reads as one function rather than as components.

Related pages: [Pipeline Orchestration and Threading Model](../pipeline/pipeline.md), [Record Model and Bridge Schema Mapping](../records/data-models.md), [RDF/XML Record Extraction and Acceptance Filters](../ingestion/rdfextraction.md), [Record Selection Policy and Its Consequences](../features/record-selection.md).

Key resources: `repo://src/main.rs`, `repo://src/cli.rs`, `repo://src/xml_parser.rs`, `repo://src/models.rs`.
