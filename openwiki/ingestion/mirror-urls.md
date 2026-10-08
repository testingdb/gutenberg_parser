---
type: architecture-urls
title: Mirror Selection and URL Transformation
description: How canonical Project Gutenberg URLs are rewritten for the selected mirror, including the digit-path layout, the files/dirs rebuild, and the pglaf epub special case.
tags: [urls, mirrors, ingestion, rust]
verified:
  - by: openwiki/0.7.1
    at: 2026-10-08T08:38:59.748Z
sources:
  - id: openwiki-source-c38906bbfa9e9c69417b11b5
    resource: repo://src/cli.rs
  - id: openwiki-source-2a737474d86fc75cc9d9694f
    resource: repo://src/config.rs
  - id: openwiki-source-0993c2c0112ac99cf6999828
    resource: repo://src/utils.rs
  - id: openwiki-source-fdbf7e7124c50fb2a5a041fb
    resource: repo://src/xml_parser.rs
generated: { by: "opencode", at: "2026-10-08T08:38:59.748Z" }
---

# Mirror Selection and URL Transformation

Project Gutenberg RDF feeds always reference `https://www.gutenberg.org/...`, but operators frequently want links that resolve against a mirror. `transform_url` is the single rewriting function that converts those canonical references for whatever `--mirror` was chosen; every URL that reaches the output passes through it.

## Mirror resolution

`--mirror` is first resolved against the `GUTENBERG_MIRRORS` table in `config.rs`, which maps `gutenberg`, `pglaf`, `odu`, `waterloo`, `uk`, and `xmission` to base URLs (repo://src/config.rs#L52-L64). The lookup is `GUTENBERG_MIRRORS.get(key).unwrap_or(&key)`, so an unrecognized key is used verbatim as a base URL — a custom mirror works without touching the table, at the cost of no validation (repo://src/cli.rs#L277-L280).

## The rewriting rules

`transform_url(url, ebook_id, mirror_base)` is a pure function with no I/O, and its branches are ordered so that each later branch only sees URLs the earlier ones declined (repo://src/utils.rs#L79-L132):

1. **Empty inputs pass through.** An empty URL or an empty mirror base returns the input unchanged. Note the asymmetry: passing an empty mirror base disables rewriting entirely rather than producing an error.
2. **The canonical base is an identity function.** If the normalized mirror is `https://www.gutenberg.org/` (or the `http` variant), the original URL is returned untouched. This is what makes the default run emit unmodified Gutenberg links.
3. **The digit path is precomputed.** For an all-numeric `ebook_id` longer than one character, every leading digit becomes its own path segment: `1342` becomes `1/3/4/1342`. This reproduces the directory layout mirrors use, where a file for ebook 1342 lives under `1342/` but is reached via `1/3/4/`. Non-numeric IDs and single-digit IDs are used as-is.
4. **`files/` and `dirs/` URLs are rebuilt from the digit path.** `RE_FILES_DIRS` captures `(?:files|dirs)/([^/]+)/(.+)`. The captured ebook directory must equal the record's `ebook_id` — the guard is deliberate, since a mismatched directory means the file belongs to a different book and the URL is left alone rather than silently relabeled. On a match, the result is `{mirror}/{digit_path}/{filename}`.
5. **Known Gutenberg prefixes are stripped and remapped.** After removing a `www.gutenberg.org` / `gutenberg.org` prefix (http or https), an `ebooks/...` remainder becomes `{mirror}/cache/epub/{ebook_id}/pg{remainder}`; any other relative path is simply re-hosted at the mirror.
6. **The pglaf epub exception.** When the mirror string contains `pglaf` and the remainder mentions epub (or ends in `.epub3.images`), the URL is replaced wholesale by `resolve_pglaf_epub_url`, which yields `{mirror}cache/epub/{id}/pg{id}-images.epub` (repo://src/utils.rs#L56-L59). pglaf publishes only the `-images` flavor, so the canonical `pg{id}.epub` and `pg{id}.epub3.images` names would 404 there.
7. **Unrecognized URLs pass through.** Anything that matched no branch — a non-Gutenberg external link, for example — is returned unchanged.

## Which URLs go through it

Three call sites in `xml_parser.rs` depend on this function, and all three pass the record's `ebook_id` so the digit-path and pglaf logic can apply (repo://src/xml_parser.rs#L93-L100, repo://src/xml_parser.rs#L247, repo://src/xml_parser.rs#L303-L311):

- `hasFormat` file URLs, which become the emitted `formats` entries.
- Agent `webpage` resources, which become each agent's `webpages` list.
- The synthesized medium cover image URL, built as `https://www.gutenberg.org/cache/epub/{id}/pg{id}.cover.medium.jpg` and then transformed.

The cover call site uses `.unwrap()` on the result. That is safe only because the synthesized URL is never empty and the function returns `Some` for every non-`None` input; it is the one place in the URL path where a `None` would panic the worker thread.

Note that agent webpage URLs are transformed by the *ebook's* ID, not the agent's own identifier. An external personal site is therefore unaffected by the rewrite, while a `gutenberg.org/ebooks/...` agent link is re-hosted against the ebook that referenced the agent.

Related pages: [RDF/XML Record Extraction and Acceptance Filters](rdfextraction.md), [Reference Tables, Regexes, and Namespace Constants](../taxonomy/reference-tables.md), [Running the Parser: Inputs, Options, and Runtime Cost](../operations/running-the-parser.md).

Key resources: `repo://src/utils.rs`, `repo://src/config.rs`, `repo://src/xml_parser.rs`.
