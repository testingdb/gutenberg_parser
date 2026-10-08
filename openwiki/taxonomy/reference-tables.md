---
type: reference-tables
title: Reference Tables, Regexes, and Namespace Constants
description: The static lookup data and compiled regexes in config.rs that every other module reads, what consumes each one, and the matching semantics they encode.
tags: [configuration, tables, regex, taxonomy, reference]
verified:
  - by: openwiki/0.7.1
    at: 2026-10-08T08:38:59.748Z
sources:
  - id: openwiki-source-2a737474d86fc75cc9d9694f
    resource: repo://src/config.rs
  - id: openwiki-source-b55a21a31ede1b56cd31a6a6
    resource: repo://src/main.rs
  - id: openwiki-source-1da28519a35b3d9fde64420c
    resource: repo://src/taxonomy.rs
  - id: openwiki-source-0993c2c0112ac99cf6999828
    resource: repo://src/utils.rs
generated: { by: "opencode", at: "2026-10-08T08:38:59.748Z" }
---

# Reference Tables, Regexes, and Namespace Constants

`config.rs` holds no logic — only immutable data and compiled patterns that the rest of the crate reads. It is the file to change when output vocabulary needs to change, and the file to read when asking "what domain names can this tool possibly emit?" (repo://src/config.rs#L1-L27).

Every table is a `static LazyLock<...>` rather than a `const`. The lazy initialization means a run that never touches taxonomy never pays to build the 266-entry LC map, which matters because these maps are large and the crate is built for throughput on a single archive pass.

## Lookup tables

**`NAMESPACES`** maps prefix to URI for `rdf`, `dcterms`, `pgterms`, `marcrel`, `cc`, and `rdfs` (repo://src/config.rs#L37-L46). It is what lets `xml_parser` address attributes namespace-correctly, e.g. `(NAMESPACES["rdf"], "about")`. Only `rdf` and `pgterms` are actually read today; the rest are declared for completeness.

**`GUTENBERG_MIRRORS`** maps the six user-facing mirror keys to base URLs (repo://src/config.rs#L52-L64). Consumed by `cli::run` with a verbatim fallback, so this table is an optional convenience rather than a closed set — see [Mirror Selection and URL Transformation](../ingestion/mirror-urls.md).

**`LC_MAP`** is the largest table, roughly 266 entries from `AC` (General Works) through `Z`, each mapping a classification code to a `(Domain, Sub-description)` pair (repo://src/config.rs#L95-L390). The sub-description is what turns a bare code like `D501` into `("History", "World War I")`. It carries both letter codes and numeric sub-codes (`E186`, `D731`, `F350.5`), and the 18 distinct domains it can produce are the complete vocabulary of `Taxonomy.domain` when LC evidence wins:

Agriculture, Bibliography & Library Science, Education, Fine Arts, General Works, Geography & Anthropology, History, Language & Literature, Law & Jurisprudence, Medicine, Military science, Music, Naval science, Philosophy & Religion, Political science, Science, Social sciences, Technology.

**`BOOKSHELF_MAP`** is an ordered `Vec<(Regex, &'static str)>` of about 34 patterns mapping Gutenberg shelf labels to normalized genre labels such as "Science Fiction & Fantasy" or "Biography & Memoir" (repo://src/config.rs#L400-L459). Despite the ordering, **every matching regex applies** — `extract_taxonomy` iterates the whole vector and inserts each match into a `HashSet`, so a label can contribute several genres and no entry is ever shadowed (repo://src/taxonomy.rs#L221-L226).

**`LCSH_FORM_GENRE_MAP`** holds 50 entries mapping lowercase subject-heading keywords to `(Broad Domain, Narrow Genre)` — `fiction` → `("Language & Literature", "Fiction & Novels")` and so on. Only five domains appear here: Bibliography & Library Science, General Works, History, Language & Literature, Social sciences (repo://src/config.rs#L465-L535).

**`GENRE_TO_DOMAIN_MAP`** is the inverse direction, mapping each of 55 normalized genre labels back to a broad domain, and it is what lets a genre inferred from a bookshelf still contribute a domain when no LC code was present (repo://src/config.rs#L540-L598).

## Compiled regexes

Seven patterns carry all the matching logic (repo://src/config.rs#L604-L629):

| Constant | Pattern | Semantics that matter |
| :--- | :--- | :--- |
| `RE_LC_CODE_VALID` | `^[A-Z]{1,3}\d*(\.\d+)?$` | Accepts one to three capitals, optional digits, optional `.N`; anything else cannot be an LC code |
| `RE_PREFIX` | `^([A-Z]{1,3})` | Captures the alphabetic part for the 3→2→1 letter fallback chain |
| `RE_AGENT_ID` | `agents/(\d+)` | Extracts a numeric agent ID from an RDF `about` URI |
| `RE_FILES_DIRS` | `(?:files\|dirs)/([^/]+)/(.+)` | Splits a legacy path into ebook directory and filename |
| `RE_SHELF_CAT` | `(?i)^Category:\s*` | Strips the shelf prefix, case-insensitively |
| `RE_MARC_SUBFIELD` | `\$[a-zA-Z]\b` | The `\b` is what preserves `$100` and `$billions`; a title like `$aThe Title` is also left alone because no word boundary follows `a` |
| `RE_WIKIPEDIA_URL` | `(?i)^(?:https?://)?(?:[a-z-]+\.)*wikipedia\.org/wiki/` | Optional scheme and any number of subdomains; the `/wiki/` requirement is what excludes bare hosts and non-article paths |

`RE_WIKIPEDIA_URL`'s tolerance is deliberate: the RDF feeds contain `http://`, `https://`, and scheme-less article URLs across `en.`, `m.`, and other subdomains, and all must be recognized while `https://en.wikipedia.org/wiki` (no article) and `https://fr.wikipedia` (no path) must not.

## Constants outside the tables

Three plain `const` values complete the configuration surface: `RDF_FEED_URL` pointing at the daily `rdf-files.tar.bz2` feed (repo://src/config.rs#L67-L67), and `WIKIPEDIA_SUMMARY_API` plus `WIKIPEDIA_USER_AGENT`, the latter assembled at compile time with `concat!` and `env!("CARGO_PKG_VERSION")` so the identity sent to Wikimedia always matches the crate version (repo://src/config.rs#L74-L85).

Related pages: [Taxonomy Inference Engine](taxonomy-inference.md), [RDF/XML Record Extraction and Acceptance Filters](../ingestion/rdfextraction.md), [Mirror Selection and URL Transformation](../ingestion/mirror-urls.md), [Wikipedia Agent Image Enrichment](../integrations/wikipedia-enrichment.md).

Key resources: `repo://src/config.rs`, `repo://src/taxonomy.rs`.
