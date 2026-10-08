---
type: "Reference"
title: "Project Gutenberg RDF Feed Contract"
openwiki_generated: true
verified:
  - by: openwiki/0.7.1
    at: 2026-10-08T08:57:55.085Z
sources:
  - id: openwiki-source-23775c3de52f3ab95a13cb8b
    resource: repo://README.md
  - id: openwiki-source-c38906bbfa9e9c69417b11b5
    resource: repo://src/cli.rs
  - id: openwiki-source-2a737474d86fc75cc9d9694f
    resource: repo://src/config.rs
  - id: openwiki-source-fdbf7e7124c50fb2a5a041fb
    resource: repo://src/xml_parser.rs
generated: { by: "opencode", at: "2026-10-08T08:57:55.085Z" }
---


# Project Gutenberg RDF Feed Contract

Everything this tool knows about ebooks comes from one upstream artifact: `rdf-files.tar.bz2`, a bzip2-compressed tar of per-ebook RDF/XML files, republished by Project Gutenberg roughly daily (repo://src/config.rs#L67-L67, repo://README.md#L64-L72). There is no version negotiation, no schema declaration, and no pinned revision on the parser side. The feed is an unpinned external contract, and this page is the inventory of what the code assumes about it.

## The coupling surface

The parser declares six RDF namespaces (repo://src/config.rs#L37-L46):

| Prefix | URI | Actually used for |
| :--- | :--- | :--- |
| `rdf` | `http://www.w3.org/1999/02/22-rdf-syntax-ns#` | `about` / `resource` attributes |
| `pgterms` | `http://www.gutenberg.org/2009/pgterms/` | Identifying a node as an `agent` |
| `dcterms` | `http://purl.org/dc/terms/` | Declared only |
| `marcrel` | `http://id.loc.gov/vocabulary/relators/` | Declared only |
| `cc` | `http://web.resource.org/cc/` | Declared only |
| `rdfs` | `http://www.w3.org/2000/01/rdf-schema#` | Declared only |

Within an `ebook` element, the elements the code actually reads are:

- **Identity and gating** — `type`, `title`, `language`, `rights`, `issued`, `downloads`.
- **Files** — `hasFormat` → `file`, whose `value` carries the declared MIME type.
- **Contributors** — `creator`, `trl`, `aui`, `ill`, `edt`, `aut`, each wrapping an `agent` with `name`, `alias`, `webpage`, `birthdate`, `deathdate`.
- **Descriptive** — `marc520`, `marc500`, or `description`; `alternative`; `subject`; `bookshelf`.

Anything else in the file is ignored (repo://src/xml_parser.rs#L167-L359).

## How matching actually works, and why that matters

Almost every lookup matches on the **local tag name** via `tag_name().name()` — `"title"`, `"subject"`, `"hasFormat"` — with the namespace ignored entirely (repo://src/xml_parser.rs#L192-L208). Only two places are namespace-aware: the `rdf:about` and `rdf:resource` attributes, and the check that a node is a `pgterms:agent` (repo://src/xml_parser.rs#L57-L66).

The result is that the parser is namespace-agnostic but tag-name-dependent. A feed-side rename — `marc520` becoming something else, or a `value` child being dropped — would **silently yield a missing optional field** rather than an error. This is a deliberate robustness trade (it is why the parser copes with the namespace variation across entries), but it means feed drift degrades output quietly. The compile-time `unwrap()`s on the regexes in `config` are the only loud failures in this layer, and they fire on a bad pattern, not on bad data.

## Tolerant of absence, strict about structure

The contract's two halves behave differently:

**Optional and defaulted.** A missing `description`, `issued`, `alternative`, `birthdate`, `deathdate`, or `webpage` node produces `None` or an empty vector, never a rejection. A missing `downloads` becomes `0`. A missing `rights` becomes the public-domain default (repo://src/xml_parser.rs#L211-L216).

**Required and rejecting.** A missing or wrong-typed `type`, `title`, `language`, one of the two required formats, or any agent drops the entire record — the record is skipped, not partially emitted (repo://src/xml_parser.rs#L178-L300).

There is also a structural assumption: each entry is expected to hold exactly one `ebook` element, and the parser looks for it among the root's direct children. An entry whose `ebook` is nested deeper is rejected with `no_ebook_element` (repo://src/xml_parser.rs#L167-L171).

The archive is consumed entry by entry and streamed — `.xml` and `.rdf` entries only — so a single malformed file rejects one record and never affects the rest of the run (repo://src/cli.rs#L322-L333).

## Offline by default

The feed is a *build-time input*, not a runtime dependency. A normal run reads a local archive and never contacts `www.gutenberg.org`; the only outbound requests in a normal run come from `--wiki-images`. Passing `--download` is the single flag that turns the feed itself into a live dependency, and even then it resolves to the one hardcoded URL — the `--mirror` setting affects link rewriting in the output, not where the feed is fetched from (repo://src/cli.rs#L187-L258).

Related pages: [RDF/XML Record Extraction and Acceptance Filters](../ingestion/rdfextraction.md), [Mirror Selection and URL Transformation](../ingestion/mirror-urls.md), [Reference Tables, Regexes, and Namespace Constants](../taxonomy/reference-tables.md), [Running the Parser: Inputs, Options, and Runtime Cost](../operations/running-the-parser.md).

Key resources: `repo://src/config.rs`, `repo://src/xml_parser.rs`, `repo://src/cli.rs`, `repo://README.md`.
